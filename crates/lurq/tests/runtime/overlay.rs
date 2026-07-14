use lurq::{
  animation::Transition,
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{
    CollisionStrategy, Column, Modal, Overlay, Parent, Placement, Popup, Rect, Root, ScrollVertical, Select, Stack,
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

struct PopupSelectRoot {
  anchor: ElementRef,
  value: Signal<String>,
}

impl Component for PopupSelectRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
      value: ctx.signal("studio".to_owned()),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(640.0, 400.0)
      .child(
        Rect::new(120.0, 32.0)
          .absolute_position(440.0, 36.0)
          .background("#22c55e")
          .ref_element(self.anchor.clone()),
      )
      .child(
        Popup::new(
          self.anchor.clone(),
          Column::new().key("popup-content").child(
            Select::new(self.value.clone())
              .options([
                ("game".to_owned(), "Game"),
                ("studio".to_owned(), "Studio"),
                ("warm".to_owned(), "Warm"),
              ])
              .width(180.0)
              .height(36.0),
          ),
        )
        .placement(Placement::BottomEnd)
        .offset(0.0, 8.0),
      )
  }
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

struct HoverOverlayRoot {
  anchor: ElementRef,
}

impl Component for HoverOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(240.0, 180.0)
      .child(
        Rect::new(100.0, 40.0)
          .background("#22c55e")
          .absolute_position(20.0, 20.0)
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(
          Rect::new(100.0, 40.0)
            .background("#ef4444")
            .hovered(|style| style.background("#38bdf8")),
        )
        .anchor(self.anchor.clone())
        .placement(Placement::BottomStart)
        .collision(CollisionStrategy::None),
      )
  }
}

struct TransitionOverlayRoot {
  anchor: ElementRef,
}

impl Component for TransitionOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Stack::new()
      .size(240.0, 180.0)
      .child(
        Rect::new(100.0, 40.0)
          .background("#22c55e")
          .absolute_position(20.0, 20.0)
          .ref_element(self.anchor.clone()),
      )
      .child(
        Overlay::new(
          Rect::new(100.0, 40.0)
            .background("#ef4444")
            .transition(Transition::background_color().duration_ms(1000).linear())
            .hovered(|style| style.background("#38bdf8")),
        )
        .anchor(self.anchor.clone())
        .placement(Placement::BottomStart)
        .collision(CollisionStrategy::None),
      )
  }
}

#[derive(Default)]
struct KeyedOverlaySignals {
  flipped: Option<Signal<bool>>,
}

struct KeyedOverlayRoot {
  first_anchor: ElementRef,
  second_anchor: ElementRef,
  flipped: Signal<bool>,
}

impl Component for KeyedOverlayRoot {
  type Props = Shared<std::sync::Mutex<KeyedOverlaySignals>>;

  fn create(ctx: &mut Ctx) -> Self {
    let flipped = ctx.signal(false);
    ctx.props::<Self::Props>().0.lock().unwrap().flipped = Some(flipped.clone());
    Self {
      first_anchor: ElementRef::new(),
      second_anchor: ElementRef::new(),
      flipped,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let red = Overlay::new(Rect::new(80.0, 30.0).background("#ef4444").key("red"))
      .anchor(self.first_anchor.clone())
      .placement(Placement::BottomStart)
      .collision(CollisionStrategy::None);
    let blue = Overlay::new(Rect::new(80.0, 30.0).background("#38bdf8").key("blue"))
      .anchor(self.second_anchor.clone())
      .placement(Placement::BottomStart)
      .collision(CollisionStrategy::None);

    let base = Stack::new()
      .size(300.0, 140.0)
      .child(
        Rect::new(80.0, 30.0)
          .background("#22c55e")
          .absolute_position(20.0, 20.0)
          .ref_element(self.first_anchor.clone()),
      )
      .child(
        Rect::new(80.0, 30.0)
          .background("#a855f7")
          .absolute_position(140.0, 20.0)
          .ref_element(self.second_anchor.clone()),
      );

    if self.flipped.get() {
      base.child(blue).child(red)
    } else {
      base.child(red).child(blue)
    }
  }
}

struct DeepOverlayRoot {
  first_anchor: ElementRef,
  second_anchor: ElementRef,
  third_anchor: ElementRef,
}

impl Component for DeepOverlayRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      first_anchor: ElementRef::new(),
      second_anchor: ElementRef::new(),
      third_anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let deepest = Overlay::new(
      Rect::new(80.0, 30.0)
        .background("#ef4444")
        .hovered(|style| style.background("#38bdf8")),
    )
    .anchor(self.third_anchor.clone())
    .placement(Placement::BottomStart)
    .collision(CollisionStrategy::None);

    let middle = Overlay::new(
      Stack::new()
        .size(140.0, 90.0)
        .child(
          Rect::new(80.0, 24.0)
            .background("#a855f7")
            .absolute_position(10.0, 10.0)
            .ref_element(self.third_anchor.clone()),
        )
        .child(deepest),
    )
    .anchor(self.second_anchor.clone())
    .placement(Placement::BottomStart)
    .collision(CollisionStrategy::None);

    let first = Overlay::new(
      Stack::new()
        .size(160.0, 110.0)
        .child(
          Rect::new(90.0, 26.0)
            .background("#f97316")
            .absolute_position(10.0, 10.0)
            .ref_element(self.second_anchor.clone()),
        )
        .child(middle),
    )
    .anchor(self.first_anchor.clone())
    .placement(Placement::BottomStart)
    .collision(CollisionStrategy::None);

    Stack::new()
      .size(320.0, 260.0)
      .child(
        Rect::new(100.0, 30.0)
          .background("#22c55e")
          .absolute_position(20.0, 20.0)
          .ref_element(self.first_anchor.clone()),
      )
      .child(first)
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
fn select_menu_inside_positioned_popup_uses_global_trigger_bounds() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PopupSelectRoot>(&mut app, ());
  run_pass(&mut tree);

  let trigger = tree
    .find_element(|element| element.tag_name() == "Select")
    .expect("select trigger should render inside popup")
    .bounds();
  let (x, y) = trigger.center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);

  let option = tree
    .find_element(|element| element.text_content() == Some("Warm"))
    .expect("select menu should open")
    .bounds();
  assert!(
    option.x >= trigger.x && option.x < trigger.x + trigger.width,
    "nested select option must align to its global trigger x: trigger={trigger:?}, option={option:?}"
  );
  assert!(
    option.y >= trigger.y + trigger.height,
    "nested select menu must open below its global trigger: trigger={trigger:?}, option={option:?}"
  );
}

#[test]
fn popup_dismisses_only_on_left_mouse_down_outside() {
  let open = Signal::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PopupRoot>(&mut app, Shared(std::sync::Arc::new(open.clone())));
  run_pass(&mut tree);

  let popup = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  let popup_center = (popup.x + popup.width * 0.5, popup.y + popup.height * 0.5);

  tree.mouse_down(popup_center.0, popup_center.1, MouseButton::Left);
  tree.mouse_move(300.0, 300.0);
  tree.mouse_up(300.0, 300.0, MouseButton::Left);
  run_pass(&mut tree);
  assert!(open.get(), "releasing a drag outside must keep the popup open");

  tree.mouse_down(300.0, 300.0, MouseButton::Right);
  run_pass(&mut tree);
  assert!(open.get(), "non-left presses must keep the popup open");

  tree.mouse_down(300.0, 300.0, MouseButton::Left);
  run_pass(&mut tree);
  assert!(!open.get(), "a left press outside must dismiss the popup");
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
fn overlay_hover_style_survives_host_rebuild() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<HoverOverlayRoot>(&mut app, ());
  run_pass(&mut tree);

  let overlay = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  tree.mouse_move(overlay.x + overlay.width * 0.5, overlay.y + overlay.height * 0.5);

  let snapshot = render_pass(&mut tree);
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#38bdf8")),
    "overlay hover style should be visible after the overlay host is rebuilt"
  );
}

#[test]
fn overlay_transition_survives_host_rebuild() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TransitionOverlayRoot>(&mut app, ());
  run_pass(&mut tree);

  let overlay = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  tree.mouse_move(overlay.x + overlay.width * 0.5, overlay.y + overlay.height * 0.5);
  render_pass(&mut tree);

  std::thread::sleep(std::time::Duration::from_millis(50));
  let snapshot = render_pass(&mut tree);
  let color = snapshot
    .rects
    .iter()
    .find(|rect| {
      rect.width == overlay.width && rect.height == overlay.height && rect.color != Color::from_hex("#22c55e")
    })
    .map(|rect| rect.color)
    .expect("transitioning overlay rect should render");

  assert_ne!(color, Color::from_hex("#ef4444"), "overlay transition should progress");
  assert_ne!(
    color,
    Color::from_hex("#38bdf8"),
    "overlay transition should not jump directly to the hovered target"
  );
}

#[test]
fn keyed_overlays_preserve_ids_when_declaration_order_changes() {
  let signals = std::sync::Arc::new(std::sync::Mutex::new(KeyedOverlaySignals::default()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<KeyedOverlayRoot>(&mut app, Shared(signals.clone()));
  run_pass(&mut tree);

  let red_before =
    node_id_for_color(tree.root().unwrap(), Color::from_hex("#ef4444")).expect("red overlay should render");
  let blue_before =
    node_id_for_color(tree.root().unwrap(), Color::from_hex("#38bdf8")).expect("blue overlay should render");

  signals.lock().unwrap().flipped.as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let red_after =
    node_id_for_color(tree.root().unwrap(), Color::from_hex("#ef4444")).expect("red overlay should still render");
  let blue_after =
    node_id_for_color(tree.root().unwrap(), Color::from_hex("#38bdf8")).expect("blue overlay should still render");

  assert_eq!(red_after, red_before);
  assert_eq!(blue_after, blue_before);
}

fn node_id_for_color(root: lurq::node::ElementRef<'_>, color: Color) -> Option<lurq::core::NodeId> {
  if root.color() == Some(color) {
    return Some(root.node_id());
  }
  for child in root.children() {
    if let Some(id) = node_id_for_color(child, color) {
      return Some(id);
    }
  }
  None
}

#[test]
fn overlays_expand_and_preserve_hover_at_arbitrary_depth() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<DeepOverlayRoot>(&mut app, ());
  run_pass(&mut tree);

  let deepest = tree
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .expect("deeply nested overlay should render")
    .bounds();
  let (x, y) = deepest.center();
  tree.mouse_move(x, y);

  let snapshot = render_pass(&mut tree);
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#38bdf8")),
    "deeply nested overlay hover should survive overlay host rebuild"
  );
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
