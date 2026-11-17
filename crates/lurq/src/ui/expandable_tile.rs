//! An expandable tile: a tile that is pressed to show more of itself.

use std::sync::Arc;

use super::{
  collapsible::{Content, opening},
  icon::Icon,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{IconChoice, fading, pressable, shown_icon},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension},
};

/// The parts of an expandable tile a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpandableTilePart {
  /// The tile: what is pressed and focused, its fill, its edge and its padding.
  Root,
  /// The row at the top: the title, and the chevron at its end.
  Header,
  /// What the tile is called.
  Title,
  /// The icon at the end of the header that says the tile opens.
  Chevron,
  /// What the tile always shows under its header: the app's elements.
  Body,
  /// What opens and closes: as high as what it holds while the tile is open, of no height while it is closed, and
  /// clipped on the way between them.
  Panel,
  /// What the panel holds, in its padding: the app's content, or a text in this part's type.
  Detail,
}

names!(ExpandableTilePart {
  Root => "root",
  Header => "header",
  Title => "title",
  Chevron => "chevron",
  Body => "body",
  Panel => "panel",
  Detail => "detail",
});

/// The states of an expandable tile a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpandableTileState {
  /// It is open.
  Open,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// It shows elements under its header whether it is open or not.
  Body,
}

names!(ExpandableTileState {
  Open => "open",
  Disabled => "disabled",
  FullWidth => "full_width",
  Body => "body",
});

/// What builds an expandable tile's nodes: a theme's in place of the default one.
pub type ExpandableTileRenderer = Arc<dyn Fn(ExpandableTileView) -> Element + Send + Sync>;

/// A tile that is pressed to show more of itself: a fact with its details, a setting with what it means, a plan
/// with what it includes.
///
/// ```
/// use lurq::{components::Text, ui::ExpandableTile};
///
/// let renderer = ExpandableTile::text("Renderer", "wgpu 26 on DirectX 12. Frame time 6.9 ms median.")
///   .open(true)
///   .on_open_change(|open| println!("open: {open}"));
/// // What it always shows, over what it opens.
/// let plan = ExpandableTile::new("Team", Text::new("Unlimited seats, SSO, audit log."))
///   .child(Text::new("$12 a seat"));
/// # fn bound(details: lurq::core::Signal<bool>) {
/// // Open while the signal is true, which a press turns.
/// let bound = ExpandableTile::text("Details", "What it holds.").bind(details);
/// # }
/// ```
///
/// The whole tile is pressed, by the pointer and by Enter and Space. It does not keep whether it is open: the app
/// says so with [`ExpandableTile::open`], or binds it to a signal with [`ExpandableTile::bind`], and is told of a
/// press with [`ExpandableTile::on_open_change`]. What it opens opens and closes with its height over the theme's
/// duration, as a [`Collapsible`](super::Collapsible)'s does, at once where motion is reduced; while it is closed
/// it is not there, and the controls in it are not stops of Tab. A press of an element the app put in the tile is
/// that element's alone.
pub struct ExpandableTile {
  title: Arc<str>,
  detail: Content,
  body: Vec<Element>,
  open: bool,
  disabled: bool,
  full_width: bool,
  icon: Option<IconChoice>,
  open_icon: Option<IconChoice>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  bound: Option<Signal<bool>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ExpandableTile {
  /// A tile called `title`, which shows `detail` while it is open.
  pub fn new(title: &str, detail: impl Into<Element>) -> Self {
    Self::of(title, Content::Element(detail.into()))
  }

  /// A tile called `title`, which shows the text `detail` while it is open.
  pub fn text(title: &str, detail: &str) -> Self {
    Self::of(title, Content::Text(Arc::from(detail)))
  }

  fn of(title: &str, detail: Content) -> Self {
    Self {
      title: Arc::from(title),
      detail,
      body: Vec::new(),
      open: false,
      disabled: false,
      full_width: false,
      icon: None,
      open_icon: None,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_open_change: Vec::new(),
      bound: None,
      id: None,
      key: None,
    }
  }

  /// Adds an element the tile always shows, under its header: a figure, a line of text, a chart.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements the tile always shows.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// Whether what it opens is shown.
  pub fn open(mut self, open: bool) -> Self {
    self.open = open;
    self
  }

  /// Open while `open` is true, which a press of the tile turns. The handlers of
  /// [`on_open_change`](Self::on_open_change) are told of a press after the signal is set; a value the app sets is
  /// told to none of them.
  pub fn bind(mut self, open: Signal<bool>) -> Self {
    self.open = open.get();
    self.bound = Some(open);
    self
  }

  /// Whether the tile takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the tile is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// The icon of the chevron, by its name in the theme's set, in place of `chevron-down`; or none, `icon(None)`,
  /// and the header has no chevron.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// The icon of the chevron while the tile is open, in place of `chevron-up`, or of its [`icon`](Self::icon) where
  /// it was given one; or none, `open_icon(None)`.
  pub fn open_icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.open_icon = Some(icon.into());
    self
  }

  /// What the tile is called to what reads the tree, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the tile in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of the tile, with what it is to be: open if it was closed. A second handler is called after
  /// the first.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this tile over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this tile.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the tile as it is under the pointer, held down or focused from the keyboard, with nothing pointing at
  /// it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the tile: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the tile in `theme`. Converting a tile into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(current_breakpoint())
      .state(ExpandableTileState::Open, self.open)
      .state(ExpandableTileState::Disabled, self.disabled)
      .state(ExpandableTileState::FullWidth, self.full_width)
      .state(ExpandableTileState::Body, !self.body.is_empty());
    for state in &self.states {
      case = case.state(*state, true);
    }
    // Closed it has its icon, or `chevron-down`; open, the icon of the open form, else the one it was given for
    // both, else `chevron-up`.
    let icon = if self.open {
      match (self.open_icon, self.icon) {
        (Some(IconChoice(open)), _) => open,
        (None, Some(IconChoice(given))) => given,
        (None, None) => Some(Name::from(icons::CHEVRON_UP)),
      }
    } else {
      shown_icon(self.icon, icons::CHEVRON_DOWN)
    };
    // A bound tile sets its signal first: its handlers are told of a press once it is so.
    let mut on_open_change = self.on_open_change;
    if let Some(bound) = self.bound {
      on_open_change.insert(0, Arc::new(move |next| bound.set(next)));
    }
    let view = ExpandableTileView {
      title: self.title,
      open: self.open,
      disabled: self.disabled,
      icon,
      label: self.label,
      recipe: theme.resolve::<ExpandableTile>(&case, Some(&self.rules)),
      detail: Some(self.detail),
      body: self.body,
      forced: self.forced,
      on_open_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ExpandableTile>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ExpandableTile> for Element {
  fn from(tile: ExpandableTile) -> Self {
    tile.build(&current_theme())
  }
}

/// An expandable tile as a renderer sees it: what it was given, the style of its parts, and its behavior ready to
/// be dressed.
pub struct ExpandableTileView {
  pub title: Arc<str>,
  pub open: bool,
  pub disabled: bool,
  /// The icon of the chevron as the tile is now, open or closed: none where it has no chevron.
  pub icon: Option<Name>,
  /// What it is called to what reads the tree, in place of its title.
  pub label: Option<Arc<str>>,
  /// The style of each part of this tile.
  pub recipe: ResolvedRecipe,
  detail: Option<Content>,
  body: Vec<Element>,
  forced: Option<Interaction>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ExpandableTileView {
  /// The style of a part, as this tile is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press does: every handler is told what the tile is to be.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let (handlers, next) = (self.on_open_change.clone(), !self.open);
    (!handlers.is_empty() && !self.disabled).then(|| {
      move || {
        for handler in &handlers {
          handler(next);
        }
      }
    })
  }

  /// The tile's behavior with the style of its root: the element that is pressed, focused and disabled, with
  /// nothing in it yet. It is a button to what reads the tree, called by its label or its title, and says whether
  /// what it opens is open. The controls the app put in it are stops of Tab of their own, after it.
  pub fn root(&self) -> components::Button {
    let mut root = pressable(&self.part(ExpandableTilePart::Root), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .stops_inside(true)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone().unwrap_or_else(|| self.title.clone()))
      .describe("a11y_expanded", if self.open { "true" } else { "false" });
    if let Some(press) = self.press() {
      root = root.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The title in the style of its part, taking the room of the header the chevron does not: a longer one goes on
  /// to the next line.
  pub fn title(&self) -> Column {
    let part = self.part(ExpandableTilePart::Title);
    Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(&self.title).width(Dimension::Pct(100.0)))
  }

  /// The chevron: the icon of the tile as it is now, at the side its part gives it, in the middle of the title's
  /// first line. None where the tile has no icon.
  pub fn chevron(&self) -> Option<Row> {
    let part = self.part(ExpandableTilePart::Chevron);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let icon = Icon::new(self.icon?).size(side).build(self.theme());
    let title = self.part(ExpandableTilePart::Title).rest().text.resolve(self.theme());
    Some(
      Row::new()
        .height(title.font_size * title.line_height)
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .overflow_visible()
        .child(
          Row::new()
            .justify(Justify::Center)
            .align_items(Alignment::Center)
            .part(&part)
            .child(icon),
        ),
    )
  }

  /// The header: the title, and the chevron at its end.
  pub fn header(&self) -> Row {
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(ExpandableTilePart::Header))
      .child(self.title());
    if let Some(chevron) = self.chevron() {
      header = header.child(chevron);
    }
    header
  }

  /// What the tile always shows under its header: a column of what the app put there, whose presses are its own.
  /// `None` for a tile with nothing there.
  pub fn body(&mut self) -> Option<Column> {
    if self.body.is_empty() {
      return None;
    }
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Stretch)
        .focus_group(FocusGroup::none())
        .part(&self.part(ExpandableTilePart::Body))
        .on_click(|event: MouseEvent| event.stop_propagation())
        .with_children(std::mem::take(&mut self.body)),
    )
  }

  /// What opens and closes, with what it holds while the tile is open. Its height is the recipe's: of none while
  /// it is closed and its content's while it is open, with a transition between the two. What it held stays while
  /// the panel closes over it, only painted, and is gone with the detail's exit animation. A press of a text it
  /// shows is the tile's; a press of an element of the app's is that element's.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      .part(&self.part(ExpandableTilePart::Panel));
    if self.open
      && let Some(detail) = self.detail.take()
    {
      let part = self.part(ExpandableTilePart::Detail);
      // Keyed, so that what the panel held is told apart from what it holds next, and kept as it leaves.
      let held = Column::new()
        .key("content")
        .focus_group(FocusGroup::none())
        .align_items(Alignment::Stretch)
        .part(&part);
      panel = panel.child(match detail {
        Content::Element(element) => held
          .on_click(|event: MouseEvent| event.stop_propagation())
          .child(element),
        Content::Text(text) => held.child(part.text(&text).width(Dimension::Pct(100.0))),
      });
    }
    panel
  }

  /// How the default renderer builds a tile: the header over what it always shows and the panel, in the root.
  pub fn render(mut self) -> Element {
    let header = self.header();
    let body = self.body();
    let panel = self.panel();
    // What is pressed is a row: what it holds is in a column as wide as it.
    let column = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Stretch)
      .child(header)
      .with_children(body.map(Element::from))
      .child(panel);
    self.root().child(column).into()
  }
}

impl crate::ui::Styled for ExpandableTile {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ExpandableTile {
  const NAME: &'static str = "expandable_tile";
  type Renderer = ExpandableTileRenderer;

  fn parts() -> crate::ui::Declared {
    ExpandableTilePart::declared()
  }

  fn states() -> crate::ui::Declared {
    ExpandableTileState::declared()
  }

  /// The design's tile (`pencil/lurq-design-system.pen`, Expandable Tile `slE1t`): a box 260 wide on `Background`
  /// in an edge of 1 in `Border` with the large radius, padded 16; its title 14 at medium weight in `Foreground`,
  /// 8 before a chevron of 16 in `MutedForeground` (`chevron-up` while it is open, as the board draws it,
  /// `chevron-down` closed); what it opens 10 under the header, a text of 13 on lines of 1.5 in `MutedForeground`.
  /// Where the board is silent: what it always shows is 10 under the header too; under the pointer its edge is in
  /// the ring's color, as a pressed card's is, and the focus is the theme's ring around it; a disabled one is at
  /// the theme's disabled opacity. What it opens moves as a collapsible's does.
  fn recipe() -> Recipe {
    use ExpandableTilePart::{Body, Chevron, Detail, Header, Panel, Root, Title};
    use ExpandableTileState::{Disabled, FullWidth, Open};
    use PaletteColor::{Background, Foreground, MutedForeground, Ring};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let recipe = Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(260.0)
            .max_width(Length::Fill)
            .padding_x(16.0)
            .padding_y(16.0)
            .radius(RadiusSize::Lg)
            .background(Background)
            .foreground(Foreground)
            .border(edge(PaletteColor::Border))
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Header, |s| s.gap(8.0))
      .base(Title, |s| {
        // The board's line of 17 for the text of 14, where the font's own is 16.8.
        s.typography(TypographyStyle::Body)
          .line_height(17.0 / 14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Chevron, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Body, |s| s.padding_top(10.0).gap(10.0).cursor(CursorIcon::Default))
      .base(Detail, |s| {
        s.padding_top(10.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          // The board's lines of 20 (its 1.5 of 13, to the pixel above).
          .line_height(20.0 / 13.0)
          .foreground(MutedForeground)
      })
      .state(FullWidth, Root, |s| s.width(Length::Fill))
      .hovered(Root, |s| s.border(edge(Ring)))
      .focus_visible(Root, |s| s.focus_ring())
      .state(Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      // A disabled one keeps its edge under the pointer.
      .when(Selector::new().state(Disabled).hovered(), Root, |s| {
        s.border(edge(PaletteColor::Border))
      });
    opening(recipe, Open.into(), Panel.into(), Detail.into())
  }
}
