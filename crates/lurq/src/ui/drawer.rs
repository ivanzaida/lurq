//! A drawer: a panel at one side of a layout that opens beside the page and
//! pushes it aside, or lies over it on a narrow window.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartAnimation, PartStyle, PartTransition},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT},
  sheet::{Sheet, SheetPart, SheetSide},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{BorderSize, MotionDuration, MotionEasing, PaletteColor, Theme},
  components::{Column, Row, ScrollVertical},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, border::Border as Edge, dimension::Dimension},
};

/// The parts of a drawer a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawerPart {
  /// The room the drawer takes in the layout: none while it is closed, and
  /// the panel's while it is open, with a transition between the two.
  Root,
  /// The drawer itself: its color, and the line along the edge it shows the
  /// page. It keeps its size while the room around it opens and closes.
  Panel,
  /// What the app puts in the drawer. It scrolls where the drawer is too
  /// small for it.
  Body,
}

names!(DrawerPart {
  Root => "root",
  Panel => "panel",
  Body => "body",
});

/// The states of a drawer a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawerState {
  /// It is open: its room is the panel's.
  Open,
}

names!(DrawerState {
  Open => "open",
});

/// What builds a drawer's nodes: a theme's in place of the default one.
pub type DrawerRenderer = Arc<dyn Fn(DrawerView) -> Element + Send + Sync>;

/// A drawer: a panel at one side of a layout, beside the page rather than
/// over it, that opens and closes. The navigation of an app, a pane of
/// details or of filters.
///
/// ```
/// use lurq::{
///   components::{Row, Text},
///   core::Signal,
///   ui::{Drawer, SheetSide},
/// };
///
/// let open = Signal::new(true);
/// let layout = Row::new()
///   .child(Drawer::new(open.clone()).side(SheetSide::Start).label("Navigation").child(Text::new("Inbox")))
///   .child(Text::new("The page"));
/// ```
///
/// A drawer does not keep whether it is open: the app's signal does. It is
/// declared in the layout where it stands, at the start or the end of a row
/// or the top or the bottom of a column. As it opens its room grows from
/// nothing to the panel's and pushes the page aside, and the panel fades
/// in; as it closes the room closes over the panel as it fades out, and
/// what it held is gone, so that Tab does not reach it. With motion reduced
/// it opens and closes at once. To tooling that reads the tree it is a
/// complementary region, called by its label.
///
/// A drawer that lies over the page with a veil, on a window too narrow to
/// push the page aside, is a [`Sheet`] at the same edge: [`Drawer::overlay`].
pub struct Drawer {
  open: Signal<bool>,
  side: SheetSide,
  label: Option<Arc<str>>,
  body: Vec<Element>,
  overlay: bool,
  scroll: bool,
  dismiss_on_veil: bool,
  dismiss_on_escape: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Drawer {
  /// A drawer open while `open` holds `true`, at the start of its layout.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      side: SheetSide::Start,
      label: None,
      body: Vec::new(),
      overlay: false,
      scroll: true,
      dismiss_on_veil: true,
      dismiss_on_escape: true,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The side of the layout the drawer stands at. At the start or the end it
  /// opens with its width, at the top or the bottom with its height.
  pub fn side(mut self, side: SheetSide) -> Self {
    self.side = side;
    self
  }

  /// What the drawer is called to tooling that reads the tree: "Navigation".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Adds an element to the drawer.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements to the drawer.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// A width of its own at the start or the end, in logical pixels, over the
  /// board's 300.
  pub fn width(mut self, width: f32) -> Self {
    self.rules =
      self
        .rules
        .base(DrawerPart::Panel, |s| s.width(width))
        .state(DrawerState::Open, DrawerPart::Root, |s| s.width(width));
    self
  }

  /// A height of its own at the top or the bottom, in logical pixels, over
  /// the height of what it holds.
  pub fn height(mut self, height: f32) -> Self {
    self.rules =
      self
        .rules
        .base(DrawerPart::Panel, |s| s.height(height))
        .state(DrawerState::Open, DrawerPart::Root, |s| s.height(height));
    self
  }

  /// Whether the drawer lies over the page with a veil, as a [`Sheet`] at its
  /// edge, rather than beside it: for a window too narrow to push the page
  /// aside. It then takes no room in the layout, Escape and a press on the
  /// veil close it, and it is a dialog to tooling.
  pub fn overlay(mut self, overlay: bool) -> Self {
    self.overlay = overlay;
    self
  }

  /// Whether what the drawer holds scrolls where the drawer is too small for
  /// it. It does unless told otherwise; what does not is laid out at the
  /// panel's height, for what holds a scroll of its own with something that
  /// stays beside it: a sidebar with its header and its footer.
  pub fn scroll(mut self, scroll: bool) -> Self {
    self.scroll = scroll;
    self
  }

  /// Whether a press on the veil of a drawer that lies over the page closes
  /// it. On by default.
  pub fn dismiss_on_veil(mut self, dismiss: bool) -> Self {
    self.dismiss_on_veil = dismiss;
    self
  }

  /// Whether Escape closes a drawer that lies over the page. On by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Puts the drawer in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this drawer over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this drawer.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the drawer's room in the layout, or of what a drawer that
  /// lies over the page lays over the window.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the drawer in `theme`. Converting a drawer into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let open = self.open.get();
    let case = Case {
      variant: Some(self.side.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(DrawerState::Open, open);
    let recipe = theme.resolve::<Drawer>(&case, Some(&self.rules));
    if self.overlay {
      return over_the_page(self, &recipe);
    }
    let view = DrawerView {
      side: self.side,
      label: self.label,
      body: self.body,
      scroll: self.scroll,
      is_open: open,
      recipe,
      open: self.open,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Drawer>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Drawer> for Element {
  fn from(drawer: Drawer) -> Self {
    drawer.build(&current_theme())
  }
}

/// A drawer that lies over the page: a sheet at its edge, as large across as
/// the drawer's panel, holding what the drawer holds as the drawer does.
fn over_the_page(drawer: Drawer, recipe: &ResolvedRecipe) -> Element {
  let panel = recipe.part(DrawerPart::Panel).rest();
  let body = recipe.part(DrawerPart::Body).rest();
  let mut sheet = Sheet::new(drawer.open)
    .side(drawer.side)
    .dismiss_on_veil(drawer.dismiss_on_veil)
    .dismiss_on_escape(drawer.dismiss_on_escape)
    .body_scrolls(drawer.scroll)
    .children(drawer.body)
    // What the drawer holds is laid out as in the drawer: the body's spacing is the drawer's.
    .style(SheetPart::Body, |s| {
      s.padding_x(0.0)
        .padding_y(0.0)
        .gap(0.0)
        .merged(&PartStyle {
          padding_x: body.padding_x,
          padding_y: body.padding_y,
          padding_top: body.padding_top,
          padding_bottom: body.padding_bottom,
          padding_start: body.padding_start,
          padding_end: body.padding_end,
          gap: body.gap,
          ..PartStyle::default()
        })
    });
  if drawer.side.is_side() {
    if let Some(width) = panel.width {
      sheet = sheet.style(SheetPart::Panel, |s| s.width(width));
    }
  } else if let Some(height) = panel.height {
    sheet = sheet.style(SheetPart::Panel, |s| s.height(height));
  }
  if let Some(label) = &drawer.label {
    sheet = sheet.label(label);
  }
  if let Some(id) = drawer.id {
    sheet = sheet.id(id);
  }
  if let Some(key) = drawer.key {
    sheet = sheet.key(key);
  }
  sheet.build(recipe.theme())
}

/// A drawer as a renderer sees it: what it was given, the style of its parts,
/// and whether it is open.
pub struct DrawerView {
  pub side: SheetSide,
  pub label: Option<Arc<str>>,
  /// What the app put in the drawer.
  pub body: Vec<Element>,
  /// Whether what the app put in the drawer scrolls.
  pub scroll: bool,
  /// Whether the drawer is open now.
  pub is_open: bool,
  /// The style of each part of this drawer.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl DrawerView {
  /// The style of a part, as this drawer is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The signal that holds whether the drawer is open.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  /// The body: a column with what the app put there, which scrolls where the
  /// panel is too small for it, or is laid out at the panel's height where it
  /// does not scroll.
  pub fn body(&mut self) -> Element {
    let body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(DrawerPart::Body))
      .with_children(std::mem::take(&mut self.body));
    let grow = if self.side.is_side() { 1.0 } else { 0.0 };
    if !self.scroll {
      return body.align_items(Alignment::Stretch).flex_full(grow, 1.0, None).into();
    }
    ScrollVertical::new(body)
      .width(Dimension::Pct(100.0))
      .flex_full(grow, 1.0, None)
      .into()
  }

  /// The panel with the body in it, in the style of its part: keyed, so that
  /// it is kept, only painted, while the room closes over it. It keeps its
  /// size however much room it is given.
  pub fn panel(&mut self) -> Column {
    let body = self.body();
    let mut panel = Column::new()
      .key("drawer-panel")
      .flex_full(0.0, 0.0, None)
      .part(&self.part(DrawerPart::Panel))
      .child(body);
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    panel
  }

  /// The room the drawer takes in the layout, in the style of its part, with
  /// `panel` in it, or nothing while the drawer is closed: a complementary
  /// region to tooling, called by its label. The panel stands against the
  /// page in it, so that as the room opens the panel comes out from the
  /// layout's edge.
  pub fn root(&self, panel: Option<Column>) -> Element {
    let part = self.part(DrawerPart::Root);
    // The panel's edge toward the page shows first.
    let toward_page = match self.side {
      SheetSide::Start | SheetSide::Top => Justify::End,
      SheetSide::End | SheetSide::Bottom => Justify::Start,
    };
    macro_rules! region {
      ($root:expr) => {{
        let mut root = $root
          .align_items(Alignment::Stretch)
          .justify(toward_page)
          .clip()
          .presence()
          .part(&part)
          .describe("a11y_role", "complementary");
        if let Some(label) = &self.label {
          root = root.describe("a11y_name", label.clone());
        }
        if let Some(key) = &self.key {
          root = root.key(key.clone());
        }
        if let Some(panel) = panel {
          root = root.child(panel);
        }
        root.into()
      }};
    }
    if self.side.is_side() {
      region!(Row::new())
    } else {
      region!(Column::new())
    }
  }

  /// How the default renderer builds a drawer: the room, with the panel in
  /// it while the drawer is open.
  pub fn render(mut self) -> Element {
    let panel = self.is_open.then(|| self.panel());
    self.root(panel)
  }
}

impl Themed for Drawer {
  const NAME: &'static str = "drawer";
  type Renderer = DrawerRenderer;

  /// The design's drawer (`pencil/lurq-design-system.pen`, Mobile/Navigation
  /// Drawer): a panel 300 wide in the background's colors with a line of the
  /// border's color along the edge it shows the page. Its room opens and
  /// closes over the theme's slow duration, and the panel fades in over the
  /// slow duration and out over the base one (Foundations, Motion: a drawer
  /// takes the slow duration, and exits are quicker than entrances).
  fn recipe() -> Recipe {
    use DrawerPart::{Panel, Root};
    use DrawerState::Open;
    use PaletteColor::*;
    use SheetSide::{Bottom, End, Start, Top};
    let line = || Edge::inside(BorderSize::Sm, PaletteColor::Border);
    let opening = |what: Transition| PartTransition::new(what, MotionDuration::Slow).easing(MotionEasing::Standard);
    let open = |side: SheetSide| Selector::new().variant(side).state(Open);
    Recipe::new()
      .base(Panel, |s| {
        s.background(Background)
          .foreground(Foreground)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Slow, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Base, MotionEasing::Exit))
      })
      // At the start or the end: as high as the layout, opening with its width.
      .variant(Start, Root, |s| s.width(0.0).height(Length::Fill).transition(opening(Transition::width())))
      .variant(End, Root, |s| s.width(0.0).height(Length::Fill).transition(opening(Transition::width())))
      .variant(Start, Panel, |s| s.width(300.0).height(Length::Fill).end_line(line()))
      .variant(End, Panel, |s| s.width(300.0).height(Length::Fill).start_line(line()))
      // At the top or the bottom: as wide as the layout, opening with the height of what it holds.
      .variant(Top, Root, |s| s.width(Length::Fill).height(0.0).transition(opening(Transition::height())))
      .variant(Bottom, Root, |s| s.width(Length::Fill).height(0.0).transition(opening(Transition::height())))
      .variant(Top, Panel, |s| s.width(Length::Fill).underline(line()))
      .variant(Bottom, Panel, |s| s.width(Length::Fill).top_line(line()))
      // Open, its room is the panel's.
      .when(open(Start), Root, |s| s.width(300.0))
      .when(open(End), Root, |s| s.width(300.0))
      .when(open(Top), Root, |s| s.height(Length::Auto))
      .when(open(Bottom), Root, |s| s.height(Length::Auto))
  }
}
