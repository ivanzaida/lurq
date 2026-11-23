//! Issue 1: monospace text wraps inside its own measured width in the GPU window.
//!
//! Look at the labels in the middle column (Geist Mono 12) and the right column (lurq's own `TypographyStyle::Mono`):
//! a label such as `background` should sit on one line, and the swatch value `#0D0D0C` under it should not be
//! covered. The left column is the same texts in Geist (sans), which lay out on one line.
//!
//! cargo run --example mono_wrap     (set REPRO_DEFAULT_MONO=1 to leave lurq's default mono family alone)

use lurq::{
  app::{
    App, Tree, component::Component, ctx::Ctx, theme::TypographyStyle, wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{Column, Grid, Text},
  layout::{Alignment, grid::GridTrack, text_style::TextStyle},
  mcp::McpConfig,
  node::{Element, color::Color, dimension::Dimension},
};
use std::sync::Arc;

const GEIST_MONO: &[u8] = include_bytes!(
  "../fonts/GeistMono-Regular.ttf"
);
const GEIST: &[u8] = include_bytes!(
  "../fonts/Geist-Regular.ttf"
);

const NAMES: [&str; 4] = ["background", "chevron-up", "more-horizontal", "card-foreground"];

struct Page;

/// One tile: a swatch, a label in `style`, and a value under it. Nothing gives the label a width.
fn tile(name: &str, style: TypographyStyle) -> Column {
  Column::new()
    .padding(12.0)
    .spacing(8.0)
    .align_items(Alignment::Start)
    .child(
      Column::new()
        .width(Dimension::Pct(100.0))
        .height(Dimension::Px(48.0))
        .background(Color::from_hex("#d0d0d0")),
    )
    .child(Text::new(name).variant(style))
    .child(Text::new("#0D0D0C").variant(TypographyStyle::Caption))
}

impl Component for Page {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let sans = TypographyStyle::extra("repro.sans12");
    let geist_mono = TypographyStyle::extra("repro.geistmono12");
    let default_mono = TypographyStyle::Mono;
    let mut grid = Grid::new()
      .columns([GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fr(1.0)])
      .gap(8.0);
    for name in NAMES {
      grid = grid
        .child(tile(name, sans.clone()))
        .child(tile(name, geist_mono.clone()))
        .child(tile(name, default_mono.clone()));
    }
    Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .padding(16.0)
      .child(grid)
  }
}

fn main() {
  let mut app = App::new();
  app.install_fonts([GEIST.to_vec(), GEIST_MONO.to_vec()], [] as [(&str, &str); 0]);
  let theme = app.theme();
  if std::env::var_os("REPRO_DEFAULT_MONO").is_none() {
    theme.set_mono_family("Geist Mono");
  }
  let body = theme.typography_style(TypographyStyle::Body);
  theme.set_typography_style(
    TypographyStyle::extra("repro.sans12"),
    TextStyle { font_family: Arc::from("Geist"), font_size: 12.0, ..body.clone() },
  );
  theme.set_typography_style(
    TypographyStyle::extra("repro.geistmono12"),
    TextStyle { font_family: Arc::from("Geist Mono"), font_size: 12.0, ..body },
  );
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-repro-mono-wrap"));
  println!("pid {} port {}", std::process::id(), mcp.port());
  tree.mount_root::<Page>(&mut app, ());
  WinitWindow::new(app, tree).with_title("mono wrap").with_size(1000, 640).run();
}
