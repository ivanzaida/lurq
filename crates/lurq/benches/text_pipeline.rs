use std::num::NonZeroIsize;

use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, render_engine::RenderEngine},
  components::{Markdown, MarkdownProps},
  layout::render_list::RenderList,
  markdown::parse_markdown,
  node::Element,
};
use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

const README: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"));
const VIEWPORT_WIDTH: f32 = 1200.0;
const VIEWPORT_HEIGHT: f32 = 20_000.0;
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

  fn render(&mut self, list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    black_box(list.glyphs.len());
    black_box(list.atlas.version);
  }
}

fn tree() -> Tree {
  let mut tree = Tree::new();
  tree.resize(VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32);
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

#[cfg(feature = "perf_profile")]
fn print_text_profile_once() {
  if std::env::var_os("LURQ_TEXT_PROFILE").is_none() {
    return;
  }

  let mut app = App::new();
  let mut tree = tree();
  tree.mount_root::<MarkdownRoot>(&mut app, readme_props(usize::MAX));
  run_pass(&mut tree, &mut app);
  eprintln!("[text_pipeline_profile] {}", tree.profile());
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

  group.finish();
}

criterion_group!(benches, bench_text_pipeline);
criterion_main!(benches);
