//! A status bar: the bar along the bottom of a window, which says what state the app is in.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
  tooltip::Tooltip,
};
use crate::{
  app::{
    ctx::{Ctx, Placement},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  layout::{Alignment, layout_kind::ShrinkLimit, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension, outline::Outline},
};

/// The tones of a status bar, and of an item of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusBarVariant {
  /// The bar on the bar's fill, its items in the foreground.
  #[default]
  Neutral,
  /// In the primary fill: the branch an editor is on, the item that says most.
  Primary,
  /// What is going on: a build that runs.
  Info,
  /// What went well: tests that pass.
  Success,
  /// What wants a look: warnings.
  Warning,
  /// What went wrong: errors, a connection lost.
  Destructive,
}

names!(StatusBarVariant {
  Neutral => "neutral",
  Primary => "primary",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// The tones of an item of a status bar: a bar's.
pub type StatusItemVariant = StatusBarVariant;

/// The parts of a status bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusBarPart {
  /// The bar: its fill and the line along its top.
  Root,
  /// One item: what it says, and what is pressed where it does something.
  Item,
  /// The icon of an item, before its text.
  Icon,
  /// The text of an item.
  Label,
  /// The rail of an item's progress.
  Track,
  /// The part of the rail done.
  Fill,
}

names!(StatusBarPart {
  Root => "root",
  Item => "item",
  Icon => "icon",
  Label => "label",
  Track => "track",
  Fill => "fill",
});

/// The states of a status bar's items a recipe styles, besides what the pointer and the keyboard do to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusBarState {
  /// The item does something when it is pressed.
  Pressable,
  /// The item shows how far something has gone, a rail after its text.
  Progress,
  /// The item takes no input.
  Disabled,
}

names!(StatusBarState {
  Pressable => "pressable",
  Progress => "progress",
  Disabled => "disabled",
});

/// Which end of a status bar gives way first where the bar is too narrow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusRegion {
  /// The items at the start.
  Leading,
  /// The items at the end.
  #[default]
  Trailing,
}

/// What builds a status bar's nodes: a theme's in place of the default one.
pub type StatusBarRenderer = Arc<dyn Fn(StatusBarView) -> Element + Send + Sync>;

/// What is called with nothing to tell.
type Call = Arc<dyn Fn() + Send + Sync>;

/// One thing a status bar says: a text, an icon, or both, which may do something when it is pressed.
///
/// ```
/// use lurq::ui::{StatusItem, StatusItemVariant, icons};
///
/// let branch = StatusItem::new("feat/design-system").icon(icons::BRANCH).variant(StatusItemVariant::Primary);
/// let errors = StatusItem::new("0").icon(icons::CLOSE_CIRCLE).tooltip("No errors").on_select(|| {});
/// let indexing = StatusItem::new("Indexing").progress(0.55);
/// let bell = StatusItem::new("Notifications").icon(icons::BELL).icon_only(true);
/// ```
#[derive(Clone)]
pub struct StatusItem {
  label: Arc<str>,
  icon: Option<Name>,
  icon_only: bool,
  variant: Option<Name>,
  on_select: Vec<Call>,
  tooltip: Option<Arc<str>>,
  progress: Option<f64>,
  priority: i32,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatusItem {
  /// An item that says `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      icon_only: false,
      variant: None,
      on_select: Vec::new(),
      tooltip: None,
      progress: None,
      priority: 0,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the item shows its icon alone: its text is its name to what reads the tree.
  pub fn icon_only(mut self, icon_only: bool) -> Self {
    self.icon_only = icon_only;
    self
  }

  /// The tone of the item, a [`StatusItemVariant`]: the bar's unless told.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = Some(variant.into());
    self
  }

  /// Called when the item is pressed: by a click, or by Space or Enter while it is focused. An item with a handler
  /// is pressed; one without says what it says. A second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// What a tooltip over the item says, of a mounted bar: "3 warnings, 0 errors".
  pub fn tooltip(mut self, text: &str) -> Self {
    self.tooltip = Some(Arc::from(text));
    self
  }

  /// How far what the item says has gone, from 0 to 1: a rail after its text.
  pub fn progress(mut self, done: impl Into<f64>) -> Self {
    self.progress = Some(done.into().clamp(0.0, 1.0));
    self
  }

  /// How long the item stays where the bar is too narrow: the lower leave first, 0 unless told.
  pub fn priority(mut self, priority: i32) -> Self {
    self.priority = priority;
    self
  }

  /// Whether a pressed item takes no input: it is dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
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
}

impl From<&str> for StatusItem {
  fn from(label: &str) -> Self {
    Self::new(label)
  }
}

/// The bar along the bottom of a window: at its start what it says of the work (the branch, what is being done),
/// at its end of the place in it (the line, the encoding, the language).
///
/// ```
/// use lurq::ui::{StatusBar, StatusItem, StatusItemVariant, icons};
///
/// let bar = StatusBar::new()
///   .label("Editor status")
///   .leading([
///     StatusItem::new("feat/design-system").icon(icons::BRANCH).variant(StatusItemVariant::Primary),
///     StatusItem::new("3").icon(icons::WARNING).on_select(|| println!("problems")),
///   ])
///   .trailing(["Ln 42, Col 7", "UTF-8", "Rust"]);
/// ```
///
/// The items that are pressed are one stop of Tab, whose arrow keys move between them. Where the bar is too narrow
/// its items leave it, those of the lowest [`priority`](StatusItem::priority) first, the end
/// [`drops_first`](Self::drops_first) says before the other, each from its end. It is a status to what reads the
/// tree.
pub struct StatusBar {
  leading: Vec<StatusItem>,
  trailing: Vec<StatusItem>,
  variant: Name,
  drops_first: StatusRegion,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for StatusBar {
  fn default() -> Self {
    Self::new()
  }
}

impl StatusBar {
  /// An empty bar, in the neutral tone.
  pub fn new() -> Self {
    Self {
      leading: Vec::new(),
      trailing: Vec::new(),
      variant: StatusBarVariant::default().into(),
      drops_first: StatusRegion::default(),
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Items at the start of the bar, after those it has there.
  pub fn leading(mut self, items: impl IntoIterator<Item = impl Into<StatusItem>>) -> Self {
    self.leading.extend(items.into_iter().map(Into::into));
    self
  }

  /// Items at the end of the bar, after those it has there.
  pub fn trailing(mut self, items: impl IntoIterator<Item = impl Into<StatusItem>>) -> Self {
    self.trailing.extend(items.into_iter().map(Into::into));
    self
  }

  /// The tone of the whole bar, a [`StatusBarVariant`]: an app that is debugging, offline. Each item is in it unless
  /// it has its own.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Which end gives way first where the bar is too narrow: the trailing unless told. Items of a higher priority
  /// stay longer, wherever they are.
  pub fn drops_first(mut self, region: StatusRegion) -> Self {
    self.drops_first = region;
    self
  }

  /// What the bar is called to what reads the tree: "Editor status".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the bar and each of its items in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this bar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this bar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the first pressed item, where Tab enters the bar, as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
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

  /// The bar as a renderer sees it, in `theme`.
  fn view(self, theme: &Theme, ctx: Option<&mut Ctx>) -> StatusBarView {
    let case = Case {
      variant: Some(self.variant),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
      ..Case::new()
    };
    let recipe = theme
      .resolve::<StatusBar>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let bar_variant = self.variant;
    let leading_first = self.drops_first == StatusRegion::Leading;
    let items: Vec<(StatusItem, bool)> = self
      .leading
      .into_iter()
      .map(|item| (item, true))
      .chain(self.trailing.into_iter().map(|item| (item, false)))
      .collect();
    let entry = items
      .iter()
      .position(|(item, _)| !item.on_select.is_empty() && !item.disabled);
    let mut ctx = ctx;
    let entries = items
      .into_iter()
      .enumerate()
      .map(|(index, (item, leading))| {
        let pressable = !item.on_select.is_empty();
        let item_case = Case {
          variant: Some(item.variant.unwrap_or(bar_variant)),
          ..case.clone()
        }
        .state(StatusBarState::Pressable, pressable)
        .state(StatusBarState::Progress, item.progress.is_some())
        .state(StatusBarState::Disabled, item.disabled);
        let recipe = theme
          .resolve::<StatusBar>(&item_case, Some(&self.rules))
          .owned_like(&recipe);
        // An item names its parts after its id, else after its key, where the app gave it one.
        let recipe = match &item.key {
          Some(key) if item.id.is_none() => recipe.keyed_item(key, StatusBarPart::Item),
          _ => recipe.item(item.id.as_ref(), StatusBarPart::Item),
        };
        // The end that gives way first is in the lower order; the lower priority gives way before the higher.
        let first = leading == leading_first;
        let order = item.priority.saturating_mul(2).saturating_add(i32::from(!first));
        let forced = self.forced.filter(|_| entry == Some(index));
        let element = StatusBarView::item_element(&item, &recipe, forced);
        let element = match (&item.tooltip, ctx.as_deref_mut()) {
          (Some(text), Some(ctx)) => Tooltip::new(text).placement(Placement::Top).mount(ctx, element),
          _ => element,
        };
        StatusEntry {
          label: item.label,
          leading,
          pressable,
          order,
          element: Some(element),
        }
      })
      .collect();
    StatusBarView {
      entries,
      label: self.label,
      recipe,
      id: self.id,
      key: self.key,
    }
  }

  /// Puts the bar where it goes: with the tooltips of its items, which keep their timers.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let theme = current_theme();
    let view = self.view(&theme, Some(ctx));
    render(&theme, view)
  }

  /// Builds the bar in `theme`, with no tooltips. Converting a bar into an element does this in the theme of what
  /// is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme, None);
    render(theme, view)
  }
}

fn render(theme: &Theme, view: StatusBarView) -> Element {
  match theme.renderer::<StatusBar>() {
    Some(renderer) => renderer(view),
    None => view.render(),
  }
}

impl From<StatusBar> for Element {
  fn from(bar: StatusBar) -> Self {
    bar.build(&current_theme())
  }
}

/// An item of a status bar as a renderer sees it: what it says, where it is, and its element, built.
pub struct StatusEntry {
  pub label: Arc<str>,
  /// Whether it is at the start of the bar.
  pub leading: bool,
  /// Whether it does something when it is pressed.
  pub pressable: bool,
  /// When it leaves the bar where the bar is too narrow: the lower first.
  pub order: i32,
  element: Option<Element>,
}

/// A status bar as a renderer sees it: its items, built, and the style of its parts.
pub struct StatusBarView {
  pub entries: Vec<StatusEntry>,
  pub label: Option<Arc<str>>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatusBarView {
  /// The style of a part of the bar.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// An item: its icon, its text and its rail, pressed where it does something.
  fn item_element(item: &StatusItem, recipe: &ResolvedRecipe, forced: Option<Interaction>) -> Element {
    let part = |part: StatusBarPart| recipe.part(part).forced(forced);
    let mut children: Vec<Element> = Vec::new();
    if let Some(icon) = item.icon {
      children.push(glyph(&part(StatusBarPart::Icon), icon, false));
    }
    if !item.icon_only {
      children.push(one_line(&part(StatusBarPart::Label), &item.label));
    }
    if let Some(done) = item.progress {
      let fill = Row::new()
        .part(&part(StatusBarPart::Fill))
        .width(Dimension::Pct((done * 100.0) as f32));
      children.push(Row::new().part(&part(StatusBarPart::Track)).child(fill).into());
    }
    if item.on_select.is_empty() {
      let mut row = Row::new()
        .part(&part(StatusBarPart::Item))
        .align_items(Alignment::Center)
        .describe("a11y_name", item.label.clone());
      for child in children {
        row = row.child(child);
      }
      if let Some(id) = &item.id {
        row = row.id(id.clone());
      }
      if let Some(key) = &item.key {
        row = row.key(key.clone());
      }
      return row.into();
    }
    let handlers = item.on_select.clone();
    let mut button = pressable(&part(StatusBarPart::Item), item.disabled)
      .focus_entry(forced.is_some())
      .describe("a11y_name", item.label.clone())
      .on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    if let Some(progress) = item.progress {
      button = button.describe("a11y_valuenow", format!("{}", (progress * 100.0).round()));
    }
    for child in children {
      button = button.child(child);
    }
    if let Some(id) = &item.id {
      button = button.id(id.clone());
    }
    if let Some(key) = &item.key {
      button = button.key(key.clone());
    }
    button.into()
  }

  /// The item at `index` as the bar lays it out: it leaves the bar whole where the bar is too narrow, in its order.
  pub fn item(&mut self, index: usize) -> Element {
    let entry = &mut self.entries[index];
    let element = entry.element.take().unwrap_or_else(Element::new);
    Row::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .flex_shrink(1.0)
      .shrink_order(entry.order)
      .shrink_limit(ShrinkLimit::Drop)
      .child(element)
      .into()
  }

  /// How the default renderer builds the bar: its leading items, room, and its trailing items, one stop of Tab for
  /// those that are pressed. It is a status to what reads the tree.
  pub fn render(mut self) -> Element {
    let mut root = Row::new()
      .part(&self.part(StatusBarPart::Root))
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .describe("a11y_role", "status")
      .describe("a11y_name", self.label.clone().unwrap_or_default());
    let split = self
      .entries
      .iter()
      .position(|entry| !entry.leading)
      .unwrap_or(self.entries.len());
    for index in 0..split {
      root = root.child(self.item(index));
    }
    root = root.child(Row::new().flex(1.0));
    for index in split..self.entries.len() {
      root = root.child(self.item(index));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for StatusBar {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for StatusBar {
  const NAME: &'static str = "status_bar";
  type Renderer = StatusBarRenderer;

  fn parts() -> crate::ui::Declared {
    StatusBarPart::declared()
  }

  fn states() -> crate::ui::Declared {
    StatusBarState::declared()
  }

  /// The design's status bar (`pencil/lurq-design-system.pen`, Window Status Bar): a bar 26 high in the bar's fill
  /// under a line of the border's color. An item is padded 8, its icon of 13 and its text of 12 in the foreground 5
  /// apart; a pressed one under the pointer is under the hover veil (the board's accent, which is the bar's own fill in
  /// the light appearance). The primary item is in the primary fill and its foreground. An item that shows progress
  /// has its text in the muted foreground and, 8 after it, a rail 72 by 4 of the input's color, rounded, its part
  /// done in the foreground. A tone is its muted fill, its text in its color.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Destructive, DestructiveMuted, Foreground, Info, InfoMuted, Input, MutedForeground, Primary, PrimaryForeground,
      Ring, StateHover, StatePressed, Success, SuccessMuted, Warning, WarningMuted,
    };
    use StatusBarPart::{Fill, Icon, Item, Label, Root, Track};
    use StatusBarState::{Disabled, Pressable, Progress};
    let mut recipe = Recipe::new()
      .base(Root, |s| {
        s.height(26.0)
          .width(Length::Fill)
          .background(PaletteColor::Bar)
          .top_line(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(Item, |s| fading(s.padding_x(8.0).gap(5.0).foreground(Foreground)))
      .base(Icon, |s| s.square(13.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption)
          .weight(FontWeight::Normal)
          .max_width(240.0)
      })
      .base(Track, |s| {
        s.width(72.0).height(4.0).radius(RadiusSize::Full).background(Input)
      })
      .base(Fill, |s| s.height(4.0).background(Foreground))
      .state(Pressable, Item, |s| s.cursor(CursorIcon::Pointer))
      .state(Progress, Item, |s| s.gap(8.0))
      .state(Progress, Label, |s| s.foreground(MutedForeground))
      .state(Disabled, Item, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .when(Selector::new().state(Pressable).hovered(), Item, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Item, |s| {
        s.veil(StatePressed)
      })
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .variant(StatusBarVariant::Primary, Root, |s| s.background(Primary))
      .variant(StatusBarVariant::Primary, Item, |s| {
        s.background(Primary).foreground(PrimaryForeground)
      });
    for (variant, color, muted) in [
      (StatusBarVariant::Info, Info, InfoMuted),
      (StatusBarVariant::Success, Success, SuccessMuted),
      (StatusBarVariant::Warning, Warning, WarningMuted),
      (StatusBarVariant::Destructive, Destructive, DestructiveMuted),
    ] {
      let (fill, ink) = (muted.clone(), color.clone());
      let (label, done) = (color.clone(), color);
      recipe = recipe
        .variant(variant, Root, |s| s.background(muted))
        .variant(variant, Item, |s| s.background(fill).foreground(ink))
        .variant(variant, Label, |s| s.foreground(label))
        .variant(variant, Fill, |s| s.background(done));
    }
    recipe
  }
}
