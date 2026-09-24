use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    events::MouseButton,
    theme::{PaletteColor, ShadowStyle},
  },
  components::{Column, Rect, ScrollVertical, Stack},
  layout::{Constraints, Size, render_list::RectShadow},
  node::{BoxShadow, BoxShadowValue, Style, border::Border, color::Color},
};

use super::PassLayoutExt;
use crate::support::{RectSnapshot, pointer_click, render_pass, render_pass_with_app};

fn shadows(rects: &[RectSnapshot]) -> Vec<(RectSnapshot, RectShadow)> {
  rects
    .iter()
    .filter_map(|rect| rect.shadow.map(|shadow| (*rect, shadow)))
    .collect()
}

#[test]
fn box_shadow_does_not_change_layout() {
  let build = |shadow: bool| {
    let card = Rect::new(80.0, 40.0).background("#ffffff");
    let card = if shadow {
      card.box_shadow(BoxShadow::new(0.0, 12.0, 24.0, "#00000080").spread(6.0))
    } else {
      card
    };
    Column::new()
      .child(card)
      .child(Rect::new(80.0, 20.0).background("#ff0000"))
  };
  let mut plain = Tree::new();
  plain.set_root(build(false));
  let plain = plain.pass_layout(Constraints::loose(Size::new(200.0, 200.0))).unwrap();
  let mut shadowed = Tree::new();
  shadowed.set_root(build(true));
  let shadowed = shadowed
    .pass_layout(Constraints::loose(Size::new(200.0, 200.0)))
    .unwrap();
  assert_eq!(plain.size, shadowed.size);
  for (a, b) in plain.children.iter().zip(&shadowed.children) {
    assert_eq!(
      (a.offset.x, a.offset.y, a.result.size),
      (b.offset.x, b.offset.y, b.result.size)
    );
  }
}

#[test]
fn box_shadow_paints_beneath_its_element_first_shadow_on_top() {
  let mut tree = Tree::new();
  tree.set_root(Stack::new().size(200.0, 200.0).child(
    Rect::new(80.0, 40.0).background("#ffffff").rounded(8.0).box_shadow([
      BoxShadow::new(0.0, 2.0, 4.0, "#ff000080"),
      BoxShadow::new(0.0, 10.0, 20.0, "#0000ff80").spread(2.0),
    ]),
  ));
  let snapshot = render_pass(&mut tree);
  let kinds: Vec<_> = snapshot
    .rects
    .iter()
    .map(|rect| rect.shadow.map(|shadow| shadow.sigma))
    .collect();
  // Back to front: the last listed shadow, the first, then the background.
  assert_eq!(kinds, vec![Some(10.0), Some(2.0), None]);
  let (rect, shadow) = shadows(&snapshot.rects)[0];
  assert_eq!((rect.x, rect.y, rect.width, rect.height), (0.0, 0.0, 80.0, 40.0));
  assert_eq!(rect.radii, [8.0; 4]);
  assert_eq!(rect.color, Color::new(0, 0, 255, 128));
  assert_eq!(shadow.offset, [0.0, 10.0]);
  assert_eq!(shadow.spread, 2.0);
  assert!(!shadow.inset);
  // Radii grow with the spread.
  assert_eq!(shadow.shape_radii, [10.0; 4]);
}

#[test]
fn box_shadow_scales_with_the_display() {
  let mut tree = Tree::new();
  tree.set_scale_factor(2.0);
  tree.set_root(
    Stack::new()
      .size(100.0, 100.0)
      .child(Rect::new(20.0, 20.0).box_shadow(BoxShadow::new(1.0, 2.0, 6.0, "#000000").spread(3.0))),
  );
  let snapshot = render_pass(&mut tree);
  let (rect, shadow) = shadows(&snapshot.rects)[0];
  assert_eq!((rect.width, rect.height), (40.0, 40.0));
  assert_eq!(shadow.offset, [2.0, 4.0]);
  assert_eq!(shadow.spread, 6.0);
  // CSS blur radius 6 is a Gaussian of sigma 3, doubled by the scale.
  assert_eq!(shadow.sigma, 6.0);
}

#[test]
fn inset_box_shadow_paints_above_background_below_border() {
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new().size(200.0, 200.0).child(
      Rect::new(80.0, 40.0)
        .background("#ffffff")
        .rounded(10.0)
        .border(Border::inside(2.0, "#333333"))
        .box_shadow(BoxShadow::new(0.0, 2.0, 4.0, "#00000040").spread(1.0).inset()),
    ),
  );
  let snapshot = render_pass(&mut tree);
  let roles: Vec<&str> = snapshot
    .rects
    .iter()
    .filter(|rect| rect.shadow.is_some() || rect.color.a() > 0 || rect.stroke_color.a() > 0)
    .map(|rect| match rect.shadow {
      Some(_) => "shadow",
      None if rect.stroke.iter().any(|width| *width > 0.0) => "border",
      None => "background",
    })
    .collect();
  assert_eq!(roles, vec!["background", "shadow", "border"]);
  let (rect, shadow) = shadows(&snapshot.rects)[0];
  // The padding box: inside the 2px border, corners reduced by it.
  assert_eq!((rect.x, rect.y, rect.width, rect.height), (2.0, 2.0, 76.0, 36.0));
  assert_eq!(rect.radii, [8.0; 4]);
  assert!(shadow.inset);
  // An inset shape shrinks by the spread.
  assert_eq!(shadow.shape_radii, [7.0; 4]);
}

#[test]
fn box_shadow_of_a_child_is_clipped_by_its_clipping_ancestor() {
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new().size(200.0, 200.0).child(
      Stack::new()
        .size(100.0, 100.0)
        .rounded(6.0)
        .clip()
        .child(Rect::new(50.0, 50.0).box_shadow(BoxShadow::new(0.0, 0.0, 30.0, "#000000"))),
    ),
  );
  let snapshot = render_pass(&mut tree);
  let (rect, _) = shadows(&snapshot.rects)[0];
  assert!(rect.clip.active);
  assert_eq!(
    (rect.clip.x, rect.clip.y, rect.clip.width, rect.clip.height),
    (0.0, 0.0, 100.0, 100.0)
  );
}

#[test]
fn box_shadow_keeps_a_scrolled_out_element_whose_shadow_is_visible() {
  let build = |blur: f32| {
    ScrollVertical::new(
      Column::new()
        .child(Rect::new(100.0, 100.0))
        // Starts 4px below the viewport; its shadow reaches 3 * 10 = 30px up.
        // Containers clip by default, so this one lets the shadow out.
        .child(
          Column::new()
            .overflow_visible()
            .padding_top(4.0)
            .child(Rect::new(100.0, 40.0).box_shadow(BoxShadow::new(0.0, 0.0, blur, "#000000"))),
        ),
    )
    .size(100.0, 100.0)
  };
  let mut tree = Tree::new();
  tree.set_root(build(20.0));
  let snapshot = render_pass(&mut tree);
  assert_eq!(shadows(&snapshot.rects).len(), 1, "the visible shadow must be painted");
  let mut tree = Tree::new();
  tree.set_root(build(0.0));
  let snapshot = render_pass(&mut tree);
  assert!(
    shadows(&snapshot.rects).is_empty(),
    "a hard shadow out of view is culled"
  );
}

#[test]
fn box_shadow_takes_the_element_opacity() {
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new()
      .size(100.0, 100.0)
      .opacity(0.5)
      .child(Rect::new(20.0, 20.0).box_shadow(BoxShadow::new(0.0, 0.0, 4.0, "#000000c8"))),
  );
  let snapshot = render_pass(&mut tree);
  let (rect, _) = shadows(&snapshot.rects)[0];
  assert_eq!(rect.color, Color::new(0, 0, 0, 100));
}

#[test]
fn box_shadow_roles_resolve_through_the_theme() {
  let mut app = App::new();
  app.theme().set_shadow_style(
    ShadowStyle::extra("card"),
    vec![BoxShadow::new(0.0, 3.0, 8.0, PaletteColor::Accent)],
  );
  app
    .theme()
    .set_palette_color(PaletteColor::Accent, Color::new(10, 20, 30, 255));
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Rect::new(20.0, 20.0).box_shadow(ShadowStyle::extra("card")))
      .child(Rect::new(20.0, 20.0).box_shadow(ShadowStyle::Lg))
      .child(Rect::new(20.0, 20.0).box_shadow(ShadowStyle::extra("missing"))),
  );
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let found = shadows(&snapshot.rects);
  // One from the extra role, two from the default `lg`, none from a missing role.
  assert_eq!(found.len(), 3);
  assert_eq!(found[0].0.color, Color::new(10, 20, 30, 255));
  assert_eq!(found[0].1.offset, [0.0, 3.0]);
  assert_eq!(found[0].1.sigma, 4.0);

  app.theme().set_shadow_style(ShadowStyle::extra("card"), Vec::new());
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(shadows(&snapshot.rects).len(), 2, "a theme change repaints");
}

#[test]
fn box_shadow_follows_hover_styles() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new().size(200.0, 200.0).child(
      Rect::new(50.0, 50.0)
        .background("#ffffff")
        .box_shadow(ShadowStyle::Sm)
        .hovered(|style: Style| style.box_shadow(BoxShadowValue::none())),
    ),
  );
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(shadows(&snapshot.rects).len(), 1);
  tree.mouse_move(25.0, 25.0);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert!(shadows(&snapshot.rects).is_empty());
}

#[test]
fn box_shadow_is_not_hit_tested() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let counter = clicks.clone();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new().size(200.0, 200.0).child(
      Rect::new(50.0, 50.0)
        .background("#ffffff")
        .box_shadow(BoxShadow::new(40.0, 40.0, 0.0, "#000000"))
        .on_click(move |_| {
          counter.fetch_add(1, Ordering::SeqCst);
        }),
    ),
  );
  render_pass_with_app(&mut tree, &mut app);
  pointer_click(&mut tree, 70.0, 70.0, MouseButton::Left);
  assert_eq!(clicks.load(Ordering::SeqCst), 0, "a click on the shadow misses");
  pointer_click(&mut tree, 25.0, 25.0, MouseButton::Left);
  assert_eq!(clicks.load(Ordering::SeqCst), 1);
}

#[test]
fn box_shadow_can_be_set_on_a_mounted_element() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(
    Stack::new()
      .size(100.0, 100.0)
      .child(Rect::new(20.0, 20.0).id("card").background("#ffffff")),
  );
  assert!(shadows(&render_pass_with_app(&mut tree, &mut app).rects).is_empty());
  tree
    .get_element_by_id_mut("card")
    .unwrap()
    .set_box_shadow(ShadowStyle::Md);
  assert_eq!(shadows(&render_pass_with_app(&mut tree, &mut app).rects).len(), 2);
}
