// Stack depth regression (lurq#25, lurq#26).
//
// A debug build of an application runs its input events, and the component
// rebuild those events trigger, on the main thread; on Windows that thread has
// a 1 MiB stack. Every level of component nesting costs the frames of
// `Ctx::mount`, the component's `render` and the builders it calls, and in an
// unoptimized build each `Element`/`Node` temporary in those frames has its own
// stack slot. This test mounts an editor-sized screen (a window chrome with a
// title bar, then nested panels, toolbars and rows of labels), lays it out,
// and then re-renders every component from a key event, all on a thread with a
// 1 MiB stack. A stack overflow aborts the test binary.
//
// `LURQ_STACK_TEST_BYTES` overrides the thread's stack size so the smallest
// stack that passes can be measured, e.g. with `scripts/stack-depth-probe.py`.

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, SpacingSize, TypographyStyle},
  },
  components::{Button, ChromeTitleBar, Column, Rect, Row, Spacer, Stack, Text, WindowChrome, WindowChromeMode},
  core::Signal,
  layout::Alignment,
  node::Element,
};

use crate::support::render_pass_with_app;

/// Nested component levels under the root, like router outlet, route view,
/// screen, workspace, panel and section in an editor.
const DEPTH: u32 = 10;
const DEFAULT_STACK_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct LevelProps {
  depth: u32,
  tick: u32,
}

struct Level {
  hovered: Signal<bool>,
}

impl Component for Level {
  type Props = LevelProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      hovered: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<LevelProps>().clone();
    let hovered = self.hovered.clone();
    let header = Row::new()
      .id(format!("header-{}", props.depth))
      .height(30.0)
      .spacing(SpacingSize::Sm)
      .align_items(Alignment::Center)
      .padding_horizontal(SpacingSize::Md)
      .background(PaletteColor::SurfacePanel)
      .child(Rect::new(12.0, 12.0).corner_radius(RadiusSize::Sm))
      .child(
        Text::new(&format!(
          "Level {} of an editor-sized screen, tick {}",
          props.depth, props.tick
        ))
        .variant(TypographyStyle::Caption)
        .color(PaletteColor::TextMuted)
        .nowrap(),
      )
      .child(Spacer::new().flex(1.0))
      .child(
        Button::new("Action")
          .padding_horizontal(SpacingSize::Sm)
          .corner_radius(RadiusSize::Md)
          .hovered(|style| style.background(PaletteColor::SurfaceInput))
          .on_mouse_enter(move || hovered.set(true)),
      );
    let panels = Row::new()
      .spacing(SpacingSize::Md)
      .with_children((0..4).map(|panel| panel_column(props.depth, panel)));
    let mut body = Column::new()
      .flex(1.0)
      .spacing(SpacingSize::Xs)
      .padding(SpacingSize::Sm)
      .corner_radius(RadiusSize::Lg)
      .child(header)
      .child(panels);
    if props.depth > 0 {
      body = body.child(ctx.mount::<Level>(LevelProps {
        depth: props.depth - 1,
        tick: props.tick,
      }));
    }
    Stack::new().child(body)
  }
}

fn panel_column(depth: u32, panel: u32) -> Element {
  Column::new()
    .spacing(SpacingSize::Xs)
    .padding(SpacingSize::Sm)
    .corner_radius(RadiusSize::Sm)
    .background(PaletteColor::SurfaceRaised)
    .with_children((0..3).map(|row| {
      Row::new()
        .spacing(SpacingSize::Xs)
        .align_items(Alignment::Center)
        .child(Rect::new(8.0, 8.0).corner_radius(RadiusSize::Sm))
        .child(Text::new(&format!("{depth}.{panel}.{row}")).variant(TypographyStyle::Label))
    }))
    .into()
}

struct Screen {
  tick: Signal<u32>,
}

impl Component for Screen {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { tick: ctx.signal(0) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let tick = self.tick.get();
    let update = self.tick.clone();
    let title_bar = ChromeTitleBar::new()
      .leading(
        Row::new()
          .spacing(SpacingSize::Sm)
          .child(Rect::new(16.0, 16.0))
          .child(Button::new("File"))
          .child(Button::new("Edit")),
      )
      .title(Text::new("Stack depth").variant(TypographyStyle::Label))
      .trailing(Row::new().child(Button::new("Share")));
    let content = Column::new()
      .flex(1.0)
      .child(ctx.mount::<Level>(LevelProps { depth: DEPTH, tick }));
    Column::new()
      .on_key_down(move |_| update.update(|tick| *tick += 1))
      .child(
        WindowChrome::new()
          .mode(WindowChromeMode::AlwaysCustom)
          .title_bar(title_bar)
          .content(content)
          .mount(ctx),
      )
  }
}

fn stack_bytes() -> usize {
  std::env::var("LURQ_STACK_TEST_BYTES")
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(DEFAULT_STACK_BYTES)
}

#[test]
fn editor_sized_rebuild_from_an_input_event_fits_a_one_mebibyte_stack() {
  let texts = std::thread::Builder::new()
    .name("stack-depth".into())
    .stack_size(stack_bytes())
    .spawn(|| {
      let mut app = App::new();
      let mut tree = Tree::new();
      tree.resize(1440, 900);
      tree.mount_root::<Screen>(&mut app, ());
      render_pass_with_app(&mut tree, &mut app);
      tree.key_down("a".into(), "KeyA".into(), false, false, false);
      render_pass_with_app(&mut tree, &mut app);
      tree.key_down("a".into(), "KeyA".into(), false, false, false);
      render_pass_with_app(&mut tree, &mut app);
      tree.get_element_by_id("header-0").and_then(|header| {
        header
          .children()
          .iter()
          .find_map(|child| child.text_content().map(str::to_owned))
      })
    })
    .expect("spawn the stack-depth thread")
    .join()
    .expect("the stack-depth thread finished");
  assert_eq!(texts.as_deref(), Some("Level 0 of an editor-sized screen, tick 2"));
}
