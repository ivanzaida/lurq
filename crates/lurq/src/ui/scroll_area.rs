//! A scroll area: content that scrolls in a room of its own, with the design's scroll bar.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::Orientation,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, Theme},
  components::{Column, Row, ScrollBoth, ScrollHorizontal, ScrollVertical},
  layout::{
    layout_kind::ScrollState,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
  node::{Element, dimension::Dimension, outline::Outline},
};

/// The parts of a scroll area a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAreaPart {
  /// The room the content scrolls in: its size, its corners, its ring of focus. Its bar is the theme's
  /// [`ScrollBarStyle`].
  Root,
}

names!(ScrollAreaPart {
  Root => "root",
});

/// The states of a scroll area a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAreaState {
  /// It scrolls down: alone, or with across.
  Vertical,
  /// It scrolls across: alone, or with down.
  Horizontal,
  /// Its bars stand in a lane of their own beside the content, not over it.
  Gutter,
}

names!(ScrollAreaState {
  Vertical => "vertical",
  Horizontal => "horizontal",
  Gutter => "gutter",
});

/// Which way a scroll area scrolls: down, across, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAreaOrientation {
  /// Down, with a bar at its end.
  #[default]
  Vertical,
  /// Across, with a bar at its bottom.
  Horizontal,
  /// Both ways, with both bars: a table, a canvas.
  Both,
}

impl From<Orientation> for ScrollAreaOrientation {
  fn from(orientation: Orientation) -> Self {
    match orientation {
      Orientation::Horizontal => Self::Horizontal,
      Orientation::Vertical => Self::Vertical,
    }
  }
}

/// When the bars of a scroll area show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAreaScrollbars {
  /// While the content is larger than the room: the design's bar.
  #[default]
  Auto,
  /// Whether or not the content is larger than the room.
  Always,
  /// While the pointer is over the area, the content scrolls or a thumb is dragged; then they fade.
  Hover,
  /// While the content scrolls or a thumb is dragged, not for the pointer alone: "show scroll bars only when
  /// scrolling".
  Scrolling,
  /// No bar: the wheel, a finger, the keys and the app still scroll the content.
  Never,
}

impl From<ScrollAreaScrollbars> for ScrollBarVisibility {
  fn from(scrollbars: ScrollAreaScrollbars) -> Self {
    match scrollbars {
      ScrollAreaScrollbars::Auto => Self::Auto,
      ScrollAreaScrollbars::Always => Self::Always,
      ScrollAreaScrollbars::Hover => Self::OnHover,
      ScrollAreaScrollbars::Scrolling => Self::WhileScrolling,
      ScrollAreaScrollbars::Never => Self::Never,
    }
  }
}

/// What builds a scroll area's nodes: a theme's in place of the default one.
pub type ScrollAreaRenderer = Arc<dyn Fn(ScrollAreaView) -> Element + Send + Sync>;

/// Content that scrolls in a room of its own: a dialog's long body, a list in a panel, a table. Its bar is the
/// design's (`pencil/lurq-design-system.pen`, Scrollbar), the theme's [`ScrollBarStyle`].
///
/// ```
/// use lurq::{
///   components::Text,
///   ui::{ScrollArea, ScrollAreaScrollbars},
/// };
///
/// let terms = ScrollArea::new()
///   .height(200.0)
///   .label("Terms of use")
///   .child(Text::new("By installing lurq you agree to these terms."))
///   .scrollbars(ScrollAreaScrollbars::Hover);
/// ```
///
/// It is as large as the room it is given, or as its `width` and `height` say, and its content scrolls in it with
/// the wheel, a finger, a drag of the bar's thumb and a press on its track. A scroll area with a
/// [`label`](Self::label) is a region of that name to what reads the tree, and a stop of Tab, whose keys scroll it:
/// give one to an area whose content has nothing that takes the focus, so that the keyboard reaches it. To what
/// reads the tree its bars are `scrollbar_vertical` and `scrollbar_horizontal`, each with how far it is scrolled,
/// from 0 to 100.
pub struct ScrollArea {
  content: Vec<Element>,
  orientation: ScrollAreaOrientation,
  scrollbars: ScrollAreaScrollbars,
  gutter: bool,
  fill_width: bool,
  scroll_state: Option<ScrollState>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for ScrollArea {
  fn default() -> Self {
    Self::new()
  }
}

impl ScrollArea {
  /// An empty area that scrolls down.
  pub fn new() -> Self {
    Self {
      content: Vec::new(),
      orientation: ScrollAreaOrientation::Vertical,
      scrollbars: ScrollAreaScrollbars::Auto,
      gutter: false,
      fill_width: false,
      scroll_state: None,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What scrolls, after what the area has: one under another, or side by side in an area that scrolls across.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.content.push(child.into());
    self
  }

  /// What scrolls, after what the area has.
  pub fn children<I: Into<Element>>(mut self, children: impl IntoIterator<Item = I>) -> Self {
    self.content.extend(children.into_iter().map(Into::into));
    self
  }

  /// Which way it scrolls: down (the default), across, or [`ScrollAreaOrientation::Both`].
  pub fn orientation(mut self, orientation: impl Into<ScrollAreaOrientation>) -> Self {
    self.orientation = orientation.into();
    self
  }

  /// When its bars show: while the content overflows (the default), always, under the pointer, only while it
  /// scrolls, or never.
  pub fn scrollbars(mut self, scrollbars: ScrollAreaScrollbars) -> Self {
    self.scrollbars = scrollbars;
    self
  }

  /// Whether its bars stand in a lane of their own beside the content, as the board's list has it, and not over
  /// the content's edge. The lane is kept while a bar fades.
  pub fn gutter(mut self, gutter: bool) -> Self {
    self.gutter = gutter;
    self
  }

  /// Whether what scrolls across is laid out at least as wide as the area: items that grow share the room there
  /// is, and scroll sideways only once they need more. Of an area that scrolls across.
  pub fn fill_width(mut self, fill: bool) -> Self {
    self.fill_width = fill;
    self
  }

  /// The app's [`ScrollState`]: where the content is scrolled to, read and set by the app.
  pub fn scroll_state(mut self, state: ScrollState) -> Self {
    self.scroll_state = Some(state);
    self
  }

  /// What the area is called to what reads the tree: it is a region of that name, and a stop of Tab whose keys
  /// scroll it.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// How wide it is: a number, a share of what holds it, or what fills it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(ScrollAreaPart::Root, move |s| s.width(width));
    self
  }

  /// How high it is: a number, a share of what holds it, or what fills it.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    let height = height.into();
    self.rules = self.rules.base(ScrollAreaPart::Root, move |s| s.height(height));
    self
  }

  /// The most it is high: as high as its content up to this, and scrolling past it.
  pub fn max_height(mut self, height: impl Into<Length>) -> Self {
    let height = height.into();
    self.rules = self.rules.base(ScrollAreaPart::Root, move |s| s.max_height(height));
    self
  }

  /// Puts the area in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this area over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this area: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the area as it is focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the area in `theme`. Converting an area into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (down, across) = match self.orientation {
      ScrollAreaOrientation::Vertical => (true, false),
      ScrollAreaOrientation::Horizontal => (false, true),
      ScrollAreaOrientation::Both => (true, true),
    };
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(ScrollAreaState::Vertical, down)
      .state(ScrollAreaState::Horizontal, across)
      .state(ScrollAreaState::Gutter, self.gutter);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    // The theme's bar, shown and placed as the area says.
    let bar = ScrollBarStyle {
      visible: self.scrollbars.into(),
      placement: if self.gutter {
        ScrollBarPlacement::Reserved
      } else {
        ScrollBarPlacement::Overlay
      },
      ..theme.scrollbar().clone()
    };
    let view = ScrollAreaView {
      content: self.content,
      orientation: self.orientation,
      bar,
      fill_width: self.fill_width,
      scroll_state: self.scroll_state,
      label: self.label,
      recipe: theme
        .resolve::<ScrollArea>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ScrollArea>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ScrollArea> for Element {
  fn from(area: ScrollArea) -> Self {
    area.build(&current_theme())
  }
}

/// A scroll area as a renderer sees it: what scrolls, which way, its bar and the style of its parts.
pub struct ScrollAreaView {
  pub content: Vec<Element>,
  pub orientation: ScrollAreaOrientation,
  /// The theme's bar, shown and placed as the area says.
  pub bar: ScrollBarStyle,
  /// Whether what scrolls across is at least as wide as the area.
  pub fill_width: bool,
  scroll_state: Option<ScrollState>,
  pub label: Option<Arc<str>>,
  /// The style of each part of this area.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ScrollAreaView {
  /// The style of a part, as this area is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What scrolls, laid out the way it scrolls: one under another, or side by side across.
  pub fn content(&mut self) -> Element {
    let content = std::mem::take(&mut self.content);
    match self.orientation {
      ScrollAreaOrientation::Horizontal => content.into_iter().fold(Row::new(), Row::child).into(),
      ScrollAreaOrientation::Vertical => content
        .into_iter()
        .fold(Column::new().width(Dimension::Pct(100.0)), Column::child)
        .into(),
      ScrollAreaOrientation::Both => content.into_iter().fold(Column::new(), Column::child).into(),
    }
  }

  /// How the default renderer builds a scroll area: the engine's scroll container of its way, in the style of
  /// its root, with the theme's bar; a region that Tab reaches where it has a label.
  pub fn render(mut self) -> Element {
    let content = self.content();
    let part = self.part(ScrollAreaPart::Root);
    macro_rules! dress {
      ($scroll:expr) => {{
        let mut scroll = $scroll.part(&part).scrollbar(self.bar.clone());
        if let Some(state) = self.scroll_state.clone() {
          scroll = scroll.with_scroll_state(state);
        }
        if let Some(label) = &self.label {
          scroll = scroll
            .tab_index(0)
            .describe("a11y_role", "region")
            .describe("a11y_name", label.clone());
        }
        if let Some(id) = &self.id {
          scroll = scroll.id(id.clone());
        }
        if let Some(key) = &self.key {
          scroll = scroll.key(key.clone());
        }
        Element::from(scroll)
      }};
    }
    match self.orientation {
      ScrollAreaOrientation::Vertical => dress!(ScrollVertical::new(content)),
      ScrollAreaOrientation::Horizontal => dress!(ScrollHorizontal::new(content).fill_width(self.fill_width)),
      ScrollAreaOrientation::Both => dress!(ScrollBoth::new(content).fill_width(self.fill_width)),
    }
  }
}

impl crate::ui::Styled for ScrollArea {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ScrollArea {
  const NAME: &'static str = "scroll_area";
  type Renderer = ScrollAreaRenderer;

  fn parts() -> crate::ui::Declared {
    ScrollAreaPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ScrollAreaState::declared()
  }

  /// The design's scroll area: the room its content scrolls in, with nothing of its own but the ring of the
  /// keyboard's focus, inside it where the area is a stop of Tab. Its bar (`pencil/lurq-design-system.pen`,
  /// Scrollbar) is the theme's [`ScrollBarStyle`]: a lane of 8 with a round thumb of 6 in `Ring`.
  fn recipe() -> Recipe {
    Recipe::new().focus_visible(ScrollAreaPart::Root, |s| {
      s.outline(Outline::new(BorderSize::Md, PaletteColor::Ring).offset(-2.0))
    })
  }
}
