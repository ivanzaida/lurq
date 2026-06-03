mod animation_demo;
mod components_demo;
mod context_demo;
mod dnd_demo;
mod events_demo;
mod inputs_demo;
mod layout_demo;
mod positioning_demo;
mod reactivity_demo;
mod scroll_demo;
mod sidebar;
mod sizing_demo;
mod style;
mod text_demo;
mod transform_demo;
mod visual_demo;

#[cfg(target_os = "windows")]
use lurq::app::dx12_render::Dx12RenderEngine;
use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::{Column, Rect, Row, Stack},
  core::Signal,
  layout::{
    Alignment, StackAlignment,
    layout_kind::Justify,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::{
  animation_demo::animation_content,
  dnd_demo::DndDemo,
  layout_demo::layout_content,
  positioning_demo::PositioningDemo,
  scroll_demo::scroll_content,
  sidebar::{DemoTab, sidebar},
  sizing_demo::sizing_content,
  style::DemoTheme,
};

const SIDEBAR_WIDTH: f32 = 200.0;
const DEFAULT_RENDERER: &str = "wgpu";

#[derive(Clone, lurq::DevtoolsInspectable)]
struct DemoProps;

impl PartialEq for DemoProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct DemoApp {
  selected_tab: Signal<DemoTab>,
  theme: Signal<DemoTheme>,
  modal_open: Signal<bool>,
}

impl Component for DemoApp {
  type Props = DemoProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected_tab: ctx.signal(DemoTab::Layout),
      theme: ctx.signal(DemoTheme::Dark),
      modal_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let selected_tab = self.selected_tab.get();
    let theme = self.theme.get();
    let palette = theme.palette();

    ctx.modal(self.modal_open.clone(), |ctx| ctx.mount::<DemoModal>(theme));

    let content = match selected_tab {
      DemoTab::Layout => layout_content(),
      DemoTab::Sizing => sizing_content(),
      DemoTab::Position => ctx.mount::<PositioningDemo>(()),
      DemoTab::Dnd => ctx.mount::<DndDemo>(()),
      DemoTab::Animation => animation_content(),
      DemoTab::Transform => transform_demo::transform_content(),
      DemoTab::Scroll => scroll_content(),
      DemoTab::Visual => visual_demo::visual_content(),
      DemoTab::Text => text_demo::text_content(),
      DemoTab::Inputs => ctx.mount::<inputs_demo::InputsDemo>(theme),
      DemoTab::Events => ctx.mount::<events_demo::EventsDemo>(()),
      DemoTab::Reactivity => ctx.mount::<reactivity_demo::ReactivityDemo>(()),
      DemoTab::Components => ctx.mount::<components_demo::ComponentsDemo>(()),
      DemoTab::Context => ctx.mount::<context_demo::ContextDemo>(context_demo::ContextDemoProps {
        theme: self.theme.clone(),
      }),
    };

    let content = Row::new()
      .align_items(Alignment::Stretch)
      .child(
        lurq::components::ScrollVertical::new(sidebar(selected_tab, self.selected_tab.clone(), theme))
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 6.0,
            thumb_color: Color::from_hex(palette.primary),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .scrollbar_hovered(move |style| style.with_thumb_color(Color::from_hex(palette.accent)))
          .width(SIDEBAR_WIDTH)
          .background(palette.surface_dark),
      )
      .child(
        Column::new()
          .child(demo_toolbar(selected_tab, theme, self.modal_open.clone()))
          .child(
            lurq::components::ScrollVertical::new(content)
              .scrollbar(ScrollBarStyle {
                visible: ScrollBarVisibility::Auto,
                width: 7.0,
                thumb_color: Color::from_hex(palette.primary),
                thumb_radius: 4.0,
                ..ScrollBarStyle::default()
              })
              .scrollbar_hovered(move |style| style.with_thumb_color(Color::from_hex(palette.accent)))
              .background(palette.bg)
              .flex(1.0),
          )
          .background(palette.bg)
          .flex(1.0),
      )
      .background(palette.bg);

    content.background(palette.bg)
  }
}

fn demo_toolbar(selected_tab: DemoTab, theme: DemoTheme, modal_open: Signal<bool>) -> Element {
  let palette = theme.palette();
  Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::End)
    .child(style::text(
      selected_tab.label(),
      13.0,
      FontWeight::Bold,
      palette.text_muted,
    ))
    .child(lurq::components::Spacer::new().flex(1.0))
    .child(demo_button("Open modal", palette.primary, move || modal_open.set(true)))
    .height(54.0)
    .padding_horizontal(18.0)
    .background(palette.surface_dark)
    .border_inside(1.0, Color::from_hex(palette.border))
    .into()
}

struct DemoModal;

impl Component for DemoModal {
  type Props = DemoTheme;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    demo_modal(
      ctx.props::<Self::Props>().to_owned(),
      ctx.modal_context().unwrap().clone(),
    )
  }
}

fn demo_modal(theme: DemoTheme, modal: lurq::app::ctx::ModalContext) -> Element {
  let palette = theme.palette();

  Stack::new()
    .stack_align(StackAlignment::Center)
    .child(
      Rect::new(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .background("#000000")
        .opacity(0.58)
        .on_click({
          let modal = modal.clone();
          move |_| modal.close()
        }),
    )
    .child(
      Column::new()
        .spacing(12.0)
        .child(style::text("Demo modal", 22.0, FontWeight::Bold, palette.text))
        .child(
          style::text(
            "This panel is declared from the demo root with ctx.modal and rendered above the app content.",
            13.0,
            FontWeight::Medium,
            palette.text_muted,
          )
          .width(Dimension::Pct(100.0)),
        )
        .child(
          Row::new()
            .justify(Justify::End)
            .child(demo_button("Close", palette.primary, move || modal.close())),
        )
        .width(420.0)
        .padding(24.0)
        .background(palette.surface)
        .border_inside(1.0, Color::from_hex(palette.border))
        .rounded(10.0),
    )
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .into()
}

fn demo_button(label: &str, fill: &'static str, on_click: impl Fn() + Send + Sync + 'static) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(style::text(label, 12.0, FontWeight::Bold, "#ffffff"))
    .height(34.0)
    .padding_horizontal(14.0)
    .background(fill)
    .rounded(6.0)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.background("#60a5fa"))
    .active(|style| style.background("#2563eb"))
    .on_click(move |_| on_click())
    .into()
}

fn set_selected_render_engine(tree: &mut Tree) -> String {
  let renderer = normalize_renderer_name(&selected_renderer_arg()).to_owned();
  let renderer_for_factory = renderer.clone();
  tree.set_render_engine_factory(move || create_render_engine(&renderer_for_factory));
  renderer
}

fn create_render_engine(renderer: &str) -> Box<dyn lurq::app::render_engine::RenderEngine> {
  match renderer {
    "wgpu" => create_wgpu_render_engine(),
    "dx12" | "d3d12" => create_dx12_render_engine(),
    other => panic!("unknown renderer `{other}`; expected `wgpu` or `dx12`"),
  }
}

fn normalize_renderer_name(renderer: &str) -> &'static str {
  match renderer {
    "wgpu" => "wgpu",
    "dx12" | "d3d12" => "dx12",
    other => panic!("unknown renderer `{other}`; expected `wgpu` or `dx12`"),
  }
}

fn selected_renderer_arg() -> String {
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    if arg == "--renderer" {
      return args
        .next()
        .unwrap_or_else(|| panic!("--renderer requires `wgpu` or `dx12`"))
        .to_ascii_lowercase();
    }

    if let Some(renderer) = arg.strip_prefix("--renderer=") {
      return renderer.to_ascii_lowercase();
    }
  }

  DEFAULT_RENDERER.to_owned()
}

fn create_wgpu_render_engine() -> Box<dyn lurq::app::render_engine::RenderEngine> {
  Box::new(WgpuRenderEngine::new())
}

#[cfg(target_os = "windows")]
fn create_dx12_render_engine() -> Box<dyn lurq::app::render_engine::RenderEngine> {
  Box::new(Dx12RenderEngine::new())
}

#[cfg(not(target_os = "windows"))]
fn create_dx12_render_engine() -> Box<dyn lurq::app::render_engine::RenderEngine> {
  panic!("--renderer dx12 requires Windows");
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  app.set_resource_root(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
  lurq::app::devtools::load_fonts(&mut app);
  app.set_profiling_enabled(true);
  let renderer = set_selected_render_engine(&mut tree);
  animation_demo::register_keyframes(&mut tree);
  tree.mount_root::<DemoApp>(&mut app, DemoProps);
  tree.mount_devtools(&mut app);
  let title = format!("lurq demo ({renderer})");
  let window = WinitWindow::new(app, tree).with_title(&title);
  window.run();
}
