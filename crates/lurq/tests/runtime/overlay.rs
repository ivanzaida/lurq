use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{
    CollisionStrategy, Column, Modal, Overlay, Parent, Placement, Popup, Rect, Root, ScrollVertical, Stack,
  },
  core::{ElementRef, Signal},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RectSnapshot, pointer_click, render_pass, run_pass};

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Shared<T>(std::sync::Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    std::sync::Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared")
      .field(&(std::sync::Arc::as_ptr(&self.0) as usize))
      .finish()
  }
}

struct AnchoredOverlayRoot {
  anchor: ElementRef,
  open: Signal<bool>,
}

impl Component for AnchoredOverlayRoot {
  type Props = Shared<Signal<bool>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .spacing(10.0)
      .child(
        Rect::new(120.0, 40.0)
          .background("#22c55e")
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(Rect::new(50.0, 20.0).background("#ef4444"))
          .anchor(self.anchor.clone())
          .open(self.open.clone())
          .placement(Placement::BottomStart)
          .offset(5.0, 7.0)
          .match_anchor_width(true),
      )
      .child(Rect::new(200.0, 30.0).background("#111827"))
  }
}

struct TopOverlayRoot {
  anchor: ElementRef,
}

impl Component for TopOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(240.0, 160.0)
      .child(
        Rect::new(80.0, 30.0)
          .background("#22c55e")
          .absolute_position(40.0, 70.0)
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(Rect::new(60.0, 20.0).background("#ef4444"))
          .anchor(self.anchor.clone())
          .placement(Placement::TopStart)
          .offset(3.0, 4.0)
          .collision(CollisionStrategy::None),
      )
  }
}

struct FlipOverlayRoot {
  anchor: ElementRef,
}

impl Component for FlipOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(180.0, 100.0)
      .child(
        Rect::new(80.0, 30.0)
          .background("#22c55e")
          .absolute_position(10.0, 5.0)
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(Rect::new(60.0, 20.0).background("#ef4444"))
          .anchor(self.anchor.clone())
          .placement(Placement::TopStart)
          .offset(0.0, 4.0),
      )
  }
}

struct PopupRoot {
  anchor: ElementRef,
  open: Signal<bool>,
}

impl Component for PopupRoot {
  type Props = Shared<Signal<bool>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .spacing(10.0)
      .child(
        Rect::new(120.0, 40.0)
          .background("#22c55e")
          .ref_element(self.anchor.clone()),
      )
      .child(
        Popup::new(self.anchor.clone(), Rect::new(70.0, 20.0).background("#ef4444"))
          .open(self.open.clone())
          .placement(Placement::BottomStart)
          .offset(0.0, 4.0),
      )
      .child(Rect::new(120.0, 40.0).background("#111827"))
  }
}

struct StaticPopupRoot {
  anchor: ElementRef,
}

impl Component for StaticPopupRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(
        Rect::new(80.0, 30.0)
          .background("#22c55e")
          .ref_element(self.anchor.clone()),
      )
      .child(Popup::new(self.anchor.clone(), Rect::new(50.0, 20.0).background("#ef4444")).open_when(true))
  }
}

struct RoundedPanelOverlayRoot {
  anchor: ElementRef,
}

impl Component for RoundedPanelOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let list = Column::new()
      .child(Rect::new(Dimension::full(), 34.0).background("#1f2937"))
      .child(Rect::new(Dimension::full(), 34.0).background("#111827"))
      .child(Rect::new(Dimension::full(), 34.0).background("#111827"));

    let panel = Column::new()
      .width(Dimension::full())
      .background("#101419")
      .rounded(8.0)
      .clip()
      .border_inside(1.0, "#30343a")
      .child(Rect::new(Dimension::full(), 28.0).background("#151a20"))
      .child(ScrollVertical::new(list).height(70.0));

    Stack::new()
      .size(260.0, 220.0)
      .child(
        Rect::new(180.0, 40.0)
          .background("#22c55e")
          .absolute_position(30.0, 150.0)
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(
          Column::new()
            .width(180.0)
            .height(118.0)
            .padding_bottom(14.0)
            .child(panel),
        )
        .anchor(self.anchor.clone())
        .placement(Placement::TopStart)
        .collision(CollisionStrategy::None),
      )
  }
}

struct ParentModalRoot {
  open: Signal<bool>,
}

impl Component for ParentModalRoot {
  type Props = Shared<Signal<bool>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(
        Stack::new()
          .size(220.0, 120.0)
          .child(Rect::new(220.0, 120.0).background("#22c55e"))
          .child(
            Modal::new(Rect::new(Dimension::full(), Dimension::full()).background("#ef4444"))
              .open(self.open.clone())
              .target(Parent),
          ),
      )
      .child(Rect::new(220.0, 40.0).background("#111827"))
  }
}

struct RootModalRoot {
  open: Signal<bool>,
}

impl Component for RootModalRoot {
  type Props = Shared<Signal<bool>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Rect::new(120.0, 40.0).background("#22c55e")).child(
      Modal::new(Rect::new(Dimension::full(), Dimension::full()).background("#ef4444"))
        .open(self.open.clone())
        .target(Root),
    )
  }
}

struct ElementTargetModalRoot {
  target: ElementRef,
  open: Signal<bool>,
}

impl Component for ElementTargetModalRoot {
  type Props = Shared<Signal<bool>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      target: ElementRef::new(),
      open: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .spacing(10.0)
      .child(
        Rect::new(160.0, 50.0)
          .background("#22c55e")
          .ref_element(self.target.clone()),
      )
      .child(Rect::new(160.0, 30.0).background("#111827"))
      .child(
        Modal::new(Rect::new(Dimension::full(), Dimension::full()).background("#ef4444"))
          .open(self.open.clone())
          .target(self.target.clone()),
      )
  }
}

#[test]
fn overlay_element_renders_native_host_above_root() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<AnchoredOverlayRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_eq!(root.tag_name(), "OverlayHost");
  assert_eq!(root.children().len(), 2);
  assert_eq!(root.children().iter().next().unwrap().tag_name(), "AnchoredOverlayRoot");
}

#[test]
fn bottom_start_overlay_uses_anchor_rect_offset_and_matched_width() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<AnchoredOverlayRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let anchor = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let overlay = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(overlay.x, anchor.x + 5.0);
  assert_eq!(overlay.y, anchor.y + anchor.height + 7.0);
  assert_eq!(overlay.width, anchor.width);
}

#[test]
fn overlay_declaration_does_not_affect_column_layout_or_spacing() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<AnchoredOverlayRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let second = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#111827")))
    .unwrap()
    .bounds();

  assert_eq!(second.y, 50.0);
}

#[test]
fn closed_overlay_is_not_hosted_or_laid_out() {
  let open = Signal::new(false);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<AnchoredOverlayRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  assert_ne!(tree.root().unwrap().tag_name(), "OverlayHost");
  assert!(
    tree
      .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
      .is_none()
  );
}

#[test]
fn top_start_overlay_uses_measured_height() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TopOverlayRoot>(&mut app, ());
  run_pass(&mut tree);

  let anchor = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let overlay = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(overlay.x, anchor.x + 3.0);
  assert_eq!(overlay.y, anchor.y - overlay.height - 4.0);
}

#[test]
fn top_start_overlay_flips_to_bottom_when_it_would_leave_viewport() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<FlipOverlayRoot>(&mut app, ());
  run_pass(&mut tree);

  let anchor = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let overlay = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(overlay.x, anchor.x);
  assert_eq!(overlay.y, anchor.y + anchor.height + 4.0);
}

#[test]
fn popup_renders_through_overlay_host_with_popup_defaults() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PopupRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_eq!(root.tag_name(), "OverlayHost");

  let anchor = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let popup = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(popup.x, anchor.x);
  assert_eq!(popup.y, anchor.y + anchor.height + 4.0);
}

#[test]
fn popup_does_not_close_when_clicking_anchor_or_content_but_closes_outside() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PopupRoot>(&mut app, Shared(std::sync::Arc::new(open.clone())));
  run_pass(&mut tree);

  let anchor = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  pointer_click(
    &mut tree,
    anchor.x + anchor.width * 0.5,
    anchor.y + anchor.height * 0.5,
    MouseButton::Left,
  );
  run_pass(&mut tree);
  assert!(open.get());

  let popup = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  pointer_click(
    &mut tree,
    popup.x + popup.width * 0.5,
    popup.y + popup.height * 0.5,
    MouseButton::Left,
  );
  run_pass(&mut tree);
  assert!(open.get());

  pointer_click(&mut tree, 300.0, 300.0, MouseButton::Left);
  run_pass(&mut tree);
  assert!(!open.get());
  assert!(
    tree
      .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
      .is_none()
  );
}

#[test]
fn popup_closes_on_escape_when_open_is_signal() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PopupRoot>(&mut app, Shared(std::sync::Arc::new(open.clone())));
  run_pass(&mut tree);

  tree.key_down("Escape".into(), "Escape".into(), false, false, false);
  run_pass(&mut tree);

  assert!(!open.get());
}

#[test]
fn static_popup_dismiss_options_do_not_close_without_signal() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<StaticPopupRoot>(&mut app, ());
  run_pass(&mut tree);

  pointer_click(&mut tree, 300.0, 300.0, MouseButton::Left);
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
      .is_some()
  );
}

#[test]
fn rounded_overlay_panel_geometry_is_stable_on_first_render() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RoundedPanelOverlayRoot>(&mut app, ());

  let first = render_pass(&mut tree);
  let second = render_pass(&mut tree);
  let first_panel = rounded_overlay_panel(&first.rects);
  let second_panel = rounded_overlay_panel(&second.rects);

  assert_eq!(first_panel.x, second_panel.x);
  assert_eq!(first_panel.y, second_panel.y);
  assert_eq!(first_panel.width, second_panel.width);
  assert_eq!(first_panel.height, second_panel.height);
  assert_eq!(first_panel.radii, [8.0, 8.0, 8.0, 8.0]);
  assert_eq!(second_panel.radii, [8.0, 8.0, 8.0, 8.0]);
}

fn rounded_overlay_panel(rects: &[RectSnapshot]) -> RectSnapshot {
  *rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#101419") && rect.radii == [8.0, 8.0, 8.0, 8.0])
    .expect("rounded overlay panel should render with its radius on the first pass")
}

#[test]
fn modal_target_parent_covers_declaring_parent_bounds() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ParentModalRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let modal = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(modal.x, 0.0);
  assert_eq!(modal.y, 0.0);
  assert_eq!(modal.width, 220.0);
  assert_eq!(modal.height, 120.0);
}

#[test]
fn modal_target_root_covers_viewport_bounds() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootModalRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let modal = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(modal.x, 0.0);
  assert_eq!(modal.y, 0.0);
  assert_eq!(modal.width, 800.0);
  assert_eq!(modal.height, 600.0);
}

#[test]
fn modal_target_element_ref_covers_target_bounds() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ElementTargetModalRoot>(&mut app, Shared(std::sync::Arc::new(open)));
  run_pass(&mut tree);

  let target = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let modal = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();

  assert_eq!(modal, target);
}

#[test]
fn modal_declaration_closes_on_escape_when_open_is_signal() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RootModalRoot>(&mut app, Shared(std::sync::Arc::new(open.clone())));
  run_pass(&mut tree);

  tree.key_down("Escape".into(), "Escape".into(), false, false, false);
  run_pass(&mut tree);

  assert!(!open.get());
}
