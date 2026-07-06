use std::num::NonZeroIsize;

use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, render_engine::RenderEngine},
  components::{Column, Markdown, MarkdownProps, Text},
  layout::{render_list::RenderList, text_style::TextStyle},
  markdown::parse_markdown,
  node::Element,
};
use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

const README: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"));
const VIEWPORT_WIDTH: f32 = 1200.0;
const TALL_VIEWPORT_HEIGHT: f32 = 20_000.0;
const REALISTIC_VIEWPORT_HEIGHT: f32 = 800.0;
const MARKDOWN_WIDTH: f32 = 860.0;

struct MarkdownRoot;

impl Component for MarkdownRoot {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Markdown::mount(ctx, ctx.props::<Self::Props>().clone())
  }
}

struct LongTextRoot;

impl Component for LongTextRoot {
  type Props = String;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Text::styled(ctx.props::<Self::Props>(), TextStyle::default()).width(MARKDOWN_WIDTH)
  }
}

struct FlowLongTextRoot;

impl Component for FlowLongTextRoot {
  type Props = String;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Text::styled(ctx.props::<Self::Props>(), TextStyle::default()).width(MARKDOWN_WIDTH))
  }
}

struct BenchSurface;

impl HasWindowHandle for BenchSurface {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let handle = Win32WindowHandle::new(NonZeroIsize::new(1).unwrap());
    Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
  }
}

impl HasDisplayHandle for BenchSurface {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    Ok(unsafe { DisplayHandle::borrow_raw(WindowsDisplayHandle::new().into()) })
  }
}

struct NoopRenderEngine;

impl RenderEngine for NoopRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    black_box(list.glyphs.len());
    black_box(list.atlas.version);
    true
  }
}

fn tree() -> Tree {
  tree_with_viewport_height(TALL_VIEWPORT_HEIGHT)
}

fn realistic_viewport_tree() -> Tree {
  tree_with_viewport_height(REALISTIC_VIEWPORT_HEIGHT)
}

fn tree_with_viewport_height(height: f32) -> Tree {
  let mut tree = Tree::new();
  tree.resize(VIEWPORT_WIDTH as u32, height as u32);
  tree.set_render_engine_factory(|| Box::new(NoopRenderEngine));
  tree
}

fn run_pass(tree: &mut Tree, app: &mut App) {
  tree.request_redraw();
  tree.pass(app, &BenchSurface);
}

fn readme_source(max_lines: usize) -> String {
  if max_lines == usize::MAX {
    return README.to_owned();
  }
  README.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

fn readme_props(max_lines: usize) -> MarkdownProps {
  MarkdownProps::new(readme_source(max_lines)).width(MARKDOWN_WIDTH)
}

fn long_text_source() -> String {
  let mut source = String::with_capacity(README.len() * 24);
  for _ in 0..24 {
    source.push_str(README);
    source.push('\n');
  }
  source
}

#[cfg(feature = "perf_profile")]
fn print_text_profile_once() {
  if std::env::var_os("LURQ_TEXT_PROFILE").is_none() {
    return;
  }

  let mut app = App::new();
  let mut tree = tree();
  tree.mount_root::<MarkdownRoot>(&mut app, readme_props(usize::MAX));
  run_pass(&mut tree, &mut app);
  eprintln!("[text_pipeline_profile tall] {}", tree.profile());

  let mut app = App::new();
  let mut tree = realistic_viewport_tree();
  tree.mount_root::<MarkdownRoot>(&mut app, readme_props(usize::MAX));
  run_pass(&mut tree, &mut app);
  eprintln!("[text_pipeline_profile realistic] {}", tree.profile());

  let mut app = App::new();
  let mut tree = realistic_viewport_tree();
  tree.mount_root::<LongTextRoot>(&mut app, long_text_source());
  run_pass(&mut tree, &mut app);
  eprintln!("[text_pipeline_profile long_text_realistic] {}", tree.profile());

  let mut app = App::new();
  let mut tree = realistic_viewport_tree();
  tree.mount_root::<FlowLongTextRoot>(&mut app, long_text_source());
  run_pass(&mut tree, &mut app);
  eprintln!("[text_pipeline_profile flow_long_text_realistic] {}", tree.profile());
}

#[cfg(not(feature = "perf_profile"))]
fn print_text_profile_once() {}

fn bench_text_pipeline(c: &mut Criterion) {
  print_text_profile_once();
  let mut group = c.benchmark_group("text_pipeline");

  for lines in [32, 128, usize::MAX] {
    let label = if lines == usize::MAX {
      "all".to_owned()
    } else {
      lines.to_string()
    };

    group.bench_with_input(
      BenchmarkId::new("parse_readme_markdown", &label),
      &lines,
      |b, &lines| {
        let source = readme_source(lines);
        b.iter(|| {
          black_box(parse_markdown(black_box(&source)));
        });
      },
    );

    group.bench_with_input(
      BenchmarkId::new("cold_readme_markdown_first_pass", &label),
      &lines,
      |b, &lines| {
        b.iter_batched(
          || {
            let mut app = App::new();
            let mut tree = tree();
            tree.mount_root::<MarkdownRoot>(&mut app, readme_props(lines));
            (app, tree)
          },
          |(mut app, mut tree)| {
            run_pass(&mut tree, &mut app);
          },
          BatchSize::SmallInput,
        );
      },
    );

    group.bench_with_input(
      BenchmarkId::new("warm_readme_markdown_cached_pass", &label),
      &lines,
      |b, &lines| {
        let mut app = App::new();
        let mut tree = tree();
        tree.mount_root::<MarkdownRoot>(&mut app, readme_props(lines));
        run_pass(&mut tree, &mut app);
        b.iter(|| {
          run_pass(&mut tree, &mut app);
        });
      },
    );
  }

  group.bench_function("cold_readme_markdown_realistic_viewport/all", |b| {
    b.iter_batched(
      || {
        let mut app = App::new();
        let mut tree = realistic_viewport_tree();
        tree.mount_root::<MarkdownRoot>(&mut app, readme_props(usize::MAX));
        (app, tree)
      },
      |(mut app, mut tree)| {
        run_pass(&mut tree, &mut app);
      },
      BatchSize::SmallInput,
    );
  });

  group.bench_function("warm_readme_markdown_realistic_viewport/all", |b| {
    let mut app = App::new();
    let mut tree = realistic_viewport_tree();
    tree.mount_root::<MarkdownRoot>(&mut app, readme_props(usize::MAX));
    run_pass(&mut tree, &mut app);
    b.iter(|| {
      run_pass(&mut tree, &mut app);
    });
  });

  group.bench_function("cold_long_text_realistic_viewport/all", |b| {
    b.iter_batched(
      || {
        let mut app = App::new();
        let mut tree = realistic_viewport_tree();
        tree.mount_root::<LongTextRoot>(&mut app, long_text_source());
        (app, tree)
      },
      |(mut app, mut tree)| {
        run_pass(&mut tree, &mut app);
      },
      BatchSize::SmallInput,
    );
  });

  group.bench_function("warm_long_text_realistic_viewport/all", |b| {
    let mut app = App::new();
    let mut tree = realistic_viewport_tree();
    tree.mount_root::<LongTextRoot>(&mut app, long_text_source());
    run_pass(&mut tree, &mut app);
    b.iter(|| {
      run_pass(&mut tree, &mut app);
    });
  });

  group.bench_function("cold_flow_long_text_realistic_viewport/all", |b| {
    b.iter_batched(
      || {
        let mut app = App::new();
        let mut tree = realistic_viewport_tree();
        tree.mount_root::<FlowLongTextRoot>(&mut app, long_text_source());
        (app, tree)
      },
      |(mut app, mut tree)| {
        run_pass(&mut tree, &mut app);
      },
      BatchSize::SmallInput,
    );
  });

  group.bench_function("warm_flow_long_text_realistic_viewport/all", |b| {
    let mut app = App::new();
    let mut tree = realistic_viewport_tree();
    tree.mount_root::<FlowLongTextRoot>(&mut app, long_text_source());
    run_pass(&mut tree, &mut app);
    b.iter(|| {
      run_pass(&mut tree, &mut app);
    });
  });

  group.bench_function("remount_flow_long_text_same_app/all", |b| {
    let mut app = App::new();
    let source = long_text_source();
    b.iter(|| {
      let mut tree = realistic_viewport_tree();
      tree.mount_root::<FlowLongTextRoot>(&mut app, source.clone());
      run_pass(&mut tree, &mut app);
    });
  });

  group.finish();
}

criterion_group!(benches, bench_text_pipeline);
criterion_main!(benches);
