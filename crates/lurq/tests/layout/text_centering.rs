use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx},
  core::Signal,
  layout::{
    Alignment, Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

use super::PassLayoutExt;
use crate::support::{GlyphSnapshot, render_pass};

fn rt() -> Tree {
  Tree::new()
}

#[derive(lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

struct TextCounterHost;

impl Component for TextCounterHost {
  type Props = Shared<Mutex<Option<Signal<i32>>>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Row::new()
      .align_items(Alignment::Center)
      .child(ctx.mount::<TextCounter>(ctx.props::<Self::Props>().clone()))
  }
}

struct TextCounter {
  count: Signal<i32>,
}

impl Component for TextCounter {
  type Props = Shared<Mutex<Option<Signal<i32>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let count = ctx.signal(9);
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(count.clone());
    Self { count }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::styled(
      &self.count.get().to_string(),
      TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    )
  }
}

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct ErrorLabelProps {
  error: Signal<bool>,
}

impl PartialEq for ErrorLabelProps {
  fn eq(&self, other: &Self) -> bool {
    self.error.id() == other.error.id()
  }
}

struct ErrorLabelHost;

impl Component for ErrorLabelHost {
  type Props = ErrorLabelProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let label = if ctx.props::<Self::Props>().error.get() {
      "ERROR!!!!!!!!!!!!!"
    } else {
      "HEX PRIVATE KEY"
    };

    lurq::components::Column::new().child(lurq::components::Text::styled(
      label,
      TextStyle {
        font_size: 10.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#f05d5e"),
        ..TextStyle::default()
      },
    ))
  }
}

struct KeyedErrorLabelHost;

impl Component for KeyedErrorLabelHost {
  type Props = ErrorLabelProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let key = if props.error.get() {
      "private_key-invalid"
    } else {
      "private_key-valid"
    };

    ctx.mount_keyed::<ErrorLabelHost>(key, props)
  }
}

struct NestedKeyedErrorLabelRoot;

impl Component for NestedKeyedErrorLabelRoot {
  type Props = ErrorLabelProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new().child(ctx.mount::<KeyedErrorLabelHost>(ctx.props::<Self::Props>().clone()))
  }
}

fn measured_height(style: TextStyle, max: Size) -> f32 {
  let mut rt = rt();
  rt.set_root(lurq::components::Text::styled("Ag", style));
  rt.pass_layout(Constraints::loose(max)).unwrap().size.height
}

#[test]
fn trim_line_box_collapses_single_line_to_em_box() {
  let font_size = 20.0;
  let base = TextStyle {
    font_size,
    line_height: 1.0,
    ..TextStyle::default()
  };
  let tall = TextStyle {
    line_height: 1.5,
    ..base.clone()
  };
  let trimmed = TextStyle {
    line_height: 1.5,
    trim_line_box: true,
    ..base.clone()
  };

  let base_h = measured_height(base, Size::new(400.0, 400.0));
  let tall_h = measured_height(tall, Size::new(400.0, 400.0));
  let trimmed_h = measured_height(trimmed, Size::new(400.0, 400.0));

  assert!(
    (tall_h - font_size * 1.5).abs() < 1.0,
    "un-trimmed 1.5 line height = {tall_h}"
  );
  // Trimmed single line collapses to the em box — same as line-height 1.0.
  assert!(
    (trimmed_h - base_h).abs() < 1.0,
    "trimmed 1.5 single line ({trimmed_h}) should match line-height 1.0 ({base_h})"
  );
  assert!(
    (trimmed_h - font_size).abs() < 1.0,
    "trimmed single line = em box {font_size}, got {trimmed_h}"
  );
}

#[test]
fn trim_line_box_keeps_leading_between_wrapped_lines() {
  let font_size = 18.0;
  let line_height = 1.5;
  let style = TextStyle {
    font_size,
    line_height,
    weight: FontWeight::Bold,
    trim_line_box: true,
    ..TextStyle::default()
  };
  // Narrow box forces a wrap to two lines.
  let mut rt = rt();
  rt.set_root(lurq::components::Text::styled("alpha bravo", style).width(60.0));
  let two_line = rt
    .pass_layout(Constraints::loose(Size::new(60.0, 400.0)))
    .unwrap()
    .size
    .height;

  // Two trimmed lines = one inter-line advance (line_height) + one em box.
  let expected = font_size * line_height + font_size;
  assert!(
    (two_line - expected).abs() < 2.0,
    "two trimmed lines should keep full leading between them: got {two_line}, expected ~{expected}"
  );
}

#[test]
fn trim_line_box_does_not_move_single_line_glyphs() {
  // With trim + a tall line height, a single line must render exactly where the
  // classic line-height 1.0 box renders it (the render path re-centers into the
  // trimmed box), so switching studio text to trim is a visual no-op for labels.
  fn center(line_height: f32, trim: bool) -> f32 {
    let mut rt = rt();
    rt.set_root(
      lurq::components::Text::styled(
        "Open",
        TextStyle {
          font_size: 18.0,
          line_height,
          trim_line_box: trim,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      )
      .height(54.0),
    );
    glyph_bounds_center_y(&render_pass(&mut rt).glyphs)
  }

  let classic = center(1.0, false);
  let trimmed = center(1.6, true);
  assert!(
    (classic - trimmed).abs() <= 1.0,
    "trimmed tall-line-height glyphs must sit where line-height 1.0 puts them: classic={classic}, trimmed={trimmed}"
  );
}

#[test]
fn trim_line_box_keeps_wrapped_glyphs_inside_the_measured_box() {
  let tint = Color::from_hex("#ff00ff");
  let mut rt = rt();
  rt.set_root(
    lurq::components::Column::new().width(286.0).clip().child(
      lurq::components::Text::styled(
        "Exact splat terrain inside this radius; the lightweight overview remains visible beyond it.",
        TextStyle {
          font_size: 10.0,
          line_height: 1.4,
          trim_line_box: true,
          color: tint,
          ..TextStyle::default()
        },
      )
      .width(286.0),
    ),
  );

  let snapshot = render_pass(&mut rt);
  let bounds = rt
    .find_element(|element| {
      element.text_content()
        == Some("Exact splat terrain inside this radius; the lightweight overview remains visible beyond it.")
    })
    .expect("wrapped text should render")
    .bounds();
  let color = tint.to_linear_f32_array();
  let glyphs = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.color == color)
    .collect::<Vec<_>>();
  let glyph_bottom = glyphs
    .iter()
    .map(|glyph| glyph.y + glyph.height)
    .fold(f32::NEG_INFINITY, f32::max);
  assert!(!glyphs.is_empty(), "wrapped text should emit glyphs");
  let measured_bottom = bounds.y + bounds.height;

  assert!(
    glyph_bottom <= measured_bottom + 4.0,
    "wrapped text must center the full multiline run inside its measured box \
     (allowing atlas padding): glyph_bottom={glyph_bottom}, measured_bottom={measured_bottom}, bounds={bounds:?}"
  );
}

#[test]
fn text_height_equals_line_height() {
  let mut rt = rt();
  let style = TextStyle {
    font_size: 24.0,
    ..TextStyle::default()
  };
  let node = lurq::components::Text::styled("0", style.clone());
  rt.set_root(node);
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let expected_height = style.font_size * style.line_height;
  assert!(
    (r.size.height - expected_height).abs() < 1.0,
    "text height should be ~{} (font_size * line_height), got {}",
    expected_height,
    r.size.height
  );
}

#[test]
fn signal_dirty_component_requests_redraw() {
  let error = Signal::new(false);
  let mut rt = rt();
  rt.mount_root::<ErrorLabelHost>(&mut lurq::app::App::new(), ErrorLabelProps { error: error.clone() });
  let _ = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  rt.clear_needs_redraw();
  assert!(!rt.needs_redraw());

  error.set(true);

  assert!(rt.needs_redraw());
}

#[test]
fn mounted_child_text_content_updates_after_signal_rerender() {
  let error = Signal::new(false);
  let mut rt = rt();
  rt.mount_root::<ErrorLabelHost>(&mut lurq::app::App::new(), ErrorLabelProps { error: error.clone() });

  let initial = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let initial_quads = rt.resolve_quads(&initial);
  assert!(text_quads_contain(&initial_quads, "HEX PRIVATE KEY"));

  error.set(true);

  let updated = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let updated_quads = rt.resolve_quads(&updated);
  assert!(text_quads_contain(&updated_quads, "ERROR!!!!!!!!!!!!!"));
  assert!(!text_quads_contain(&updated_quads, "HEX PRIVATE KEY"));
}

#[test]
fn keyed_mounted_child_updates_after_signal_rerender() {
  let error = Signal::new(false);
  let mut rt = rt();
  rt.mount_root::<KeyedErrorLabelHost>(&mut lurq::app::App::new(), ErrorLabelProps { error: error.clone() });

  let initial = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let initial_quads = rt.resolve_quads(&initial);
  assert!(text_quads_contain(&initial_quads, "HEX PRIVATE KEY"));
  #[cfg(feature = "devtools")]
  assert_eq!(snapshot_child_key(&rt).as_deref(), Some("private_key-valid"));

  error.set(true);

  let updated = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let updated_quads = rt.resolve_quads(&updated);
  assert!(text_quads_contain(&updated_quads, "ERROR!!!!!!!!!!!!!"));
  assert!(!text_quads_contain(&updated_quads, "HEX PRIVATE KEY"));
  #[cfg(feature = "devtools")]
  assert_eq!(snapshot_child_key(&rt).as_deref(), Some("private_key-invalid"));
}

#[test]
fn nested_dirty_component_updates_keyed_child_after_signal_rerender() {
  let error = Signal::new(false);
  let mut rt = rt();
  rt.mount_root::<NestedKeyedErrorLabelRoot>(&mut lurq::app::App::new(), ErrorLabelProps { error: error.clone() });

  let initial = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let initial_quads = rt.resolve_quads(&initial);
  assert!(text_quads_contain(&initial_quads, "HEX PRIVATE KEY"));
  #[cfg(feature = "devtools")]
  assert_eq!(snapshot_child_key(&rt).as_deref(), Some("private_key-valid"));

  error.set(true);

  let updated = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let updated_quads = rt.resolve_quads(&updated);
  assert!(text_quads_contain(&updated_quads, "ERROR!!!!!!!!!!!!!"));
  assert!(!text_quads_contain(&updated_quads, "HEX PRIVATE KEY"));
  #[cfg(feature = "devtools")]
  assert_eq!(snapshot_child_key(&rt).as_deref(), Some("private_key-invalid"));
}

fn text_quads_contain(quads: &[lurq::layout::quad::Quad], expected: &str) -> bool {
  quads.iter().any(|quad| match &quad.content {
    QuadContent::Text { text, .. } => text == expected,
    _ => false,
  })
}

#[cfg(feature = "devtools")]
fn snapshot_child_key(rt: &Tree) -> Option<String> {
  let snapshot = lurq::app::devtools::DevToolsSnapshot::from_tree(rt);
  find_snapshot_key(snapshot.root.as_ref()?).map(str::to_owned)
}

#[cfg(feature = "devtools")]
fn find_snapshot_key(node: &lurq::app::devtools::DevToolsNode) -> Option<&str> {
  if matches!(node.key.as_deref(), Some("private_key-valid" | "private_key-invalid")) {
    return node.key.as_deref();
  }

  node.children.iter().find_map(find_snapshot_key)
}

#[test]
fn nowrap_text_keeps_intrinsic_single_line_width() {
  let mut rt = rt();
  let style = TextStyle {
    font_size: 14.0,
    ..TextStyle::default()
  };
  rt.set_root(lurq::components::Text::styled("x: 1279  y: 450  |  entered: true", style).nowrap());
  let r = rt.pass_layout(Constraints::loose(Size::new(120.0, 80.0))).unwrap();

  assert!(
    r.size.width > 120.0,
    "nowrap text should keep its intrinsic width instead of wrapping to the parent constraint"
  );

  let quads = rt.resolve_quads(&r);
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("nowrap text should emit a text quad");
  assert_eq!(text_quad.width, r.size.width);
}

#[test]
fn changed_text_content_invalidates_ancestor_layout_cache() {
  let count = Arc::new(Mutex::new(None));
  let mut rt = rt();
  rt.mount_root::<TextCounterHost>(&mut lurq::app::App::new(), Shared(count.clone()));

  let one_digit = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let one_digit_width = one_digit.children[0].result.size.width;
  let one_digit_height = one_digit.children[0].result.size.height;

  count.lock().unwrap().as_ref().unwrap().set(10);

  let two_digit = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let two_digit_width = two_digit.children[0].result.size.width;
  let two_digit_height = two_digit.children[0].result.size.height;

  assert!(
    two_digit_width > one_digit_width,
    "updated text should be remeasured wider than the previous single digit: {} <= {}",
    two_digit_width,
    one_digit_width
  );
  assert!(
    (two_digit_height - one_digit_height).abs() < 1.0,
    "updated text should stay single-line height: {} vs {}",
    two_digit_height,
    one_digit_height
  );
}

#[test]
fn text_vertically_centered_in_row_with_rects() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(lurq::components::Rect::new(36.0, 36.0))
    .child(lurq::components::Text::styled(
      "0",
      TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    ))
    .child(lurq::components::Rect::new(36.0, 36.0));
  rt.set_root(node);
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

  let row_height = r.size.height;
  assert!(
    (row_height - 36.0).abs() < 1.0,
    "row height should be 36 (max child), got {}",
    row_height
  );

  let text_child = &r.children[1];
  let text_height = text_child.result.size.height;
  let text_center_y = text_child.offset.y + text_height / 2.0;
  let row_center_y = row_height / 2.0;

  assert!(
    (text_center_y - row_center_y).abs() < 1.0,
    "text center ({}) should match row center ({}), text_y={}, text_h={}",
    text_center_y,
    row_center_y,
    text_child.offset.y,
    text_height
  );
}

#[test]
fn text_vertically_centered_in_fixed_height_row() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(lurq::components::Text::styled(
      "Layout",
      TextStyle {
        font_size: 11.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#f8fafc"),
        ..TextStyle::default()
      },
    ))
    .width(200.0)
    .height(38.0);
  rt.set_root(node);
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

  let row = &r;
  let text_child = &row.children[0];
  let text_height = text_child.result.size.height;
  let text_center_y = text_child.offset.y + text_height / 2.0;

  assert!(
    text_height < row.size.height,
    "text child should keep intrinsic height inside fixed-height row, text_h={}, row_h={}",
    text_height,
    row.size.height
  );
  assert!(
    (text_center_y - row.size.height / 2.0).abs() < 1.0,
    "text center ({}) should match row center ({})",
    text_center_y,
    row.size.height / 2.0
  );
}

#[test]
fn centered_text_glyphs_do_not_shift_when_line_height_changes() {
  let compact_center = centered_text_glyph_center_y(1.0);
  let tall_center = centered_text_glyph_center_y(1.8);
  let row_center = 32.0;

  assert!(
    (compact_center - tall_center).abs() <= 1.0,
    "centered glyphs should stay visually centered when line-height changes: compact={compact_center}, tall={tall_center}",
  );
  assert!(
    (compact_center - row_center).abs() <= 2.0,
    "compact glyph center should match row center: compact={compact_center}, row={row_center}",
  );
  assert!(
    (tall_center - row_center).abs() <= 2.0,
    "tall glyph center should match row center: tall={tall_center}, row={row_center}",
  );
}

#[test]
fn fixed_height_text_centers_rendered_glyphs() {
  let mut rt = rt();
  rt.set_root(
    lurq::components::Text::styled(
      "Open",
      TextStyle {
        font_size: 18.0,
        line_height: 1.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    )
    .height(54.0),
  );

  let snapshot = render_pass(&mut rt);
  let center = glyph_bounds_center_y(&snapshot.glyphs);
  let text_center = 27.0;

  assert!(
    (center - text_center).abs() <= 3.0,
    "fixed-height text glyph center should match text box center: glyph={center}, text={text_center}",
  );
}

fn centered_text_glyph_center_y(line_height: f32) -> f32 {
  let mut rt = rt();
  rt.set_root(
    lurq::components::Row::new()
      .align_items(Alignment::Center)
      .height(64.0)
      .child(lurq::components::Text::styled(
        "Text",
        TextStyle {
          font_size: 20.0,
          line_height,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      )),
  );

  let snapshot = render_pass(&mut rt);
  glyph_bounds_center_y(&snapshot.glyphs)
}

fn glyph_bounds_center_y(glyphs: &[GlyphSnapshot]) -> f32 {
  assert!(!glyphs.is_empty(), "text should render glyphs");
  let top = glyphs.iter().map(|glyph| glyph.y).fold(f32::INFINITY, f32::min);
  let bottom = glyphs
    .iter()
    .map(|glyph| glyph.y + glyph.height)
    .fold(f32::NEG_INFINITY, f32::max);
  (top + bottom) * 0.5
}
