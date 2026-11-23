//! A list item, one row of a list, and the list that holds them: its lines, the inset they start at, and, as a
//! contained list, a box with a header. A row of a settings card is a list item too, in its `Setting` state.

use std::sync::Arc;

use super::{
  checkbox::{CheckboxPart, CheckboxState},
  icons,
  kbd::{Kbd, KeyPlatform},
  names,
  part_style::{FontFamily, Length, PartStyle},
  radio::{RadioPart, RadioState},
  recipe::{Case, Interaction, Name, ROW_STEPS, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  separator::Separator,
  shared::{fading, glyph, opens_aside, pressable, tab_stop},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row, TextOverflow},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, EventHandler, FocusGroup, IntoMouseEventHandler, border::Border, dimension::Dimension,
    outline::Outline,
  },
};

/// The parts of a list item a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListItemPart {
  /// The row: what is pressed, selected and focused, with what is before, in and after it.
  Root,
  /// What stands before the text: an avatar, an icon, a checkbox, a radio, or what the app puts there.
  Leading,
  /// The icon before the text, where the row has one: its side and its color.
  Icon,
  /// The title over the description, as wide as the room the row leaves them.
  Content,
  /// What the row says.
  Title,
  /// An element of the app's on a line of its own under the title's line and over the description: a path.
  Subtitle,
  /// The second line under the title, where the row has one.
  Description,
  /// What stands after the title, on its line: the app's tags.
  Tags,
  /// The mark after the title (and its tags) that explains the row, its words in a tooltip.
  Info,
  /// An element of the app's under the description, in the text's column: a link, a path.
  Detail,
  /// What the row says under itself, across it, under its line: a problem, a code to copy.
  Below,
  /// What stands after the text: its value, its time, what the app puts there, its keys, its chevron.
  Trailing,
  /// A value at the end, as a setting has one: "Home".
  Value,
  /// A note at the end, smaller and set in the mono type: a time, "2m".
  Meta,
  /// The chevron at the very end of a row that leads somewhere, or that opens what is under it.
  Chevron,
  /// The bone in the title's place while the title is read: its width and its height.
  TitleSkeleton,
  /// The bone in the description's place while the description is read: its width and its height.
  DescriptionSkeleton,
  /// The row and what it says under itself, where it says something below it: what holds both.
  Item,
}

names!(ListItemPart {
  Root => "root",
  Leading => "leading",
  Icon => "icon",
  Content => "content",
  Title => "title",
  Subtitle => "subtitle",
  Description => "description",
  Tags => "tags",
  Info => "info",
  Detail => "detail",
  Below => "below",
  Trailing => "trailing",
  Value => "value",
  Meta => "meta",
  Chevron => "chevron",
  TitleSkeleton => "title_skeleton",
  DescriptionSkeleton => "description_skeleton",
  Item => "item",
});

/// The states of a list item a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListItemState {
  /// The row is the one selected, or one of them.
  Selected,
  /// The row takes no input.
  Disabled,
  /// The row is pressed: it does something, or goes somewhere. A row that is not answers no pointer.
  Pressable,
  /// The row has a description under its title.
  Description,
  /// What is before the text is the row's icon.
  Icon,
  /// The row is in a contained list: rows that touch, in a box.
  Contained,
  /// The row is a setting, in a [`SettingsCard`](super::SettingsCard): its title and its description, with the
  /// control that sets it at its end.
  Setting,
}

names!(ListItemState {
  Selected => "selected",
  Disabled => "disabled",
  Pressable => "pressable",
  Description => "description",
  Icon => "icon",
  Contained => "contained",
  Setting => "setting",
});

/// What builds a list item's nodes: a theme's in place of the default one.
pub type ListItemRenderer = Arc<dyn Fn(ListItemView) -> Element + Send + Sync>;

/// What stands before a row's text.
#[derive(Clone)]
enum Lead {
  Element(Element),
  Icon(Name),
  Checkbox(bool),
  Radio(bool),
}

/// What a press of a row does, given the press.
type Press = Arc<dyn Fn(&MouseEvent) + Send + Sync>;

/// One row of a list: a title, a description under it, what stands before it (an avatar, an icon, a checkbox) and
/// what stands after it (a value, a time, a switch, a chevron). A setting, a person, a file, a notification.
///
/// ```
/// use lurq::ui::{Avatar, ListItem, Switch, icons};
///
/// let wifi = ListItem::new("Wi-Fi").icon(icons::WIFI).value("Home").chevron(true).on_click(|_| println!("Wi-Fi"));
/// let ivan = ListItem::new("Ivan Zaida")
///   .description("Pushed 3 commits to feat/node-blur")
///   .leading(Avatar::new("Ivan Zaida"))
///   .meta("2m");
/// let dark = ListItem::new("Dark mode").trailing(Switch::new("Dark mode").label_hidden(true));
/// ```
///
/// A row that is given [`on_click`](Self::on_click) or [`href`](Self::href) is pressed: by the pointer, and by
/// Enter and Space while it is focused, and Tab stops at it. It is a button to what reads the tree, or a link with
/// an `href`, and a checkbox or a radio where it has one before its text. A row that is not pressed is not a stop
/// of Tab and answers no pointer: what it holds (a switch at its end) is pressed on its own. In a row that is
/// pressed, what the app put at its end ([`trailing`](Self::trailing)) keeps its own presses, which are not the
/// row's, and is a stop of Tab after the row.
///
/// While the app reads its words, a row shows a skeleton's bone in place of its title or its description
/// ([`title_pending`](Self::title_pending), [`description_pending`](Self::description_pending)). Its id is on the
/// row, what is pressed; a row that says something [below](Self::below) is held with it in a part `item`, `x-item`.
#[derive(Clone)]
pub struct ListItem {
  title: Arc<str>,
  title_pending: bool,
  title_tooltip_when_cut: bool,
  description: Option<Arc<str>>,
  description_pending: bool,
  tags: Vec<Element>,
  info: Option<Arc<str>>,
  subtitles: Vec<Element>,
  details: Vec<Element>,
  below: Vec<Element>,
  lead: Option<Lead>,
  value: Option<Arc<str>>,
  meta: Option<Arc<str>>,
  trailing: Option<Element>,
  shortcut: Option<Arc<str>>,
  chevron: bool,
  /// Whether what the row opens is open, where it is the header of something that opens under it.
  expands: Option<bool>,
  selected: bool,
  disabled: bool,
  on_click: Vec<EventHandler<MouseEvent>>,
  on_open_aside: Vec<EventHandler<MouseEvent>>,
  tab_stop: bool,
  href: Option<Arc<str>>,
  density: Option<Density>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ListItem {
  /// A row that says `title`, with nothing before or after it.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      title_pending: false,
      title_tooltip_when_cut: false,
      description: None,
      description_pending: false,
      tags: Vec::new(),
      info: None,
      subtitles: Vec::new(),
      details: Vec::new(),
      below: Vec::new(),
      lead: None,
      value: None,
      meta: None,
      trailing: None,
      shortcut: None,
      chevron: false,
      expands: None,
      selected: false,
      disabled: false,
      on_click: Vec::new(),
      on_open_aside: Vec::new(),
      tab_stop: true,
      href: None,
      density: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A second line under the title, in the muted color: who, what, when.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Whether the title is still read: a bone stands in place of its words, as wide and as high as the recipe's
  /// part `title_skeleton` says, on a line of the title's height, so that the words that come move nothing. The row
  /// is still called by its title (what the app has, or a word that says what it is) and tells tooling it is busy.
  pub fn title_pending(mut self, pending: bool) -> Self {
    self.title_pending = pending;
    self
  }

  /// Whether the description is still read: a bone stands in its place, as wide and as high as the recipe's part
  /// `description_skeleton` says, on a line of the description's height, whether the row has a description yet or
  /// not. The row is drawn as one of two lines, and tells tooling it is busy.
  pub fn description_pending(mut self, pending: bool) -> Self {
    self.description_pending = pending;
    self
  }

  /// An element after the title, on its line, after those given before: a [`Tag`](super::Tag) that says
  /// *Default* or *Changed*, a [`Badge`](super::Badge). The title gives way before them.
  pub fn tag(mut self, tag: impl Into<Element>) -> Self {
    self.tags.push(tag.into());
    self
  }

  /// Whether the whole title is said in a tooltip while it is cut with an ellipsis, as
  /// [`Tooltip::when_cut`](super::Tooltip::when_cut) says: shown after the theme's delay, and only while the last
  /// layout cut the title. Off unless told.
  pub fn title_tooltip_when_cut(mut self, tooltip: bool) -> Self {
    self.title_tooltip_when_cut = tooltip;
    self
  }

  /// What the row needs explained, in a few words: the mark of an [`InfoLabel`](super::InfoLabel) after the title
  /// and its tags, an "i" in a circle, with the words in a tooltip above it wherever the row is rendered. A press of
  /// the mark is not the row's. An empty text is no mark.
  pub fn info(mut self, info: &str) -> Self {
    self.info = (!info.is_empty()).then(|| Arc::from(info));
    self
  }

  /// An element of the app's on a line of its own under the title's line and over the description, after those
  /// given before: the value a setting names, such as a program's path, with the description that says more under
  /// it. Its presses are its own, not the row's. Its part is `subtitle`, `x-subtitle`.
  pub fn subtitle(mut self, subtitle: impl Into<Element>) -> Self {
    self.subtitles.push(subtitle.into());
    self
  }

  /// An element of the app's under the description, in the text's column, after those given before: a
  /// [`Link`](super::Link) that says more, a path on a line of its own. Its presses are its own, not the row's.
  pub fn detail(mut self, detail: impl Into<Element>) -> Self {
    self.details.push(detail.into());
    self
  }

  /// What the row says under itself, after what was given before: a problem, a question, a code to copy. It stands
  /// across the row, under the line of its title and what is at its end, as part of the row (a list's line is
  /// under it), and a press of it is not a press of the row.
  pub fn below(mut self, element: impl Into<Element>) -> Self {
    self.below.push(element.into());
    self
  }

  /// What stands before the text: an [`Avatar`](super::Avatar), a picture, a mark of the app's.
  pub fn leading(mut self, element: impl Into<Element>) -> Self {
    self.lead = Some(Lead::Element(element.into()));
    self
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.lead = Some(Lead::Icon(name.into()));
    self
  }

  /// A checkbox before the text, checked or not: the row is the checkbox, which its press checks and unchecks
  /// through [`on_click`](Self::on_click).
  pub fn checkbox(mut self, checked: bool) -> Self {
    self.lead = Some(Lead::Checkbox(checked));
    self
  }

  /// A radio before the text, chosen or not: the row is the radio, which its press chooses through
  /// [`on_click`](Self::on_click).
  pub fn radio(mut self, chosen: bool) -> Self {
    self.lead = Some(Lead::Radio(chosen));
    self
  }

  /// A value at the end, in the muted color: what a setting is set to.
  pub fn value(mut self, value: &str) -> Self {
    self.value = Some(Arc::from(value));
    self
  }

  /// A note at the end, small and in the mono type: a time, a size.
  pub fn meta(mut self, meta: &str) -> Self {
    self.meta = Some(Arc::from(meta));
    self
  }

  /// What stands at the end, after the value: a [`Switch`](super::Switch), a [`Badge`](super::Badge), a button.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing = Some(element.into());
    self
  }

  /// The keys that do what the row does, at its end, as [`Kbd::shortcut`] writes them: "Mod+N".
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = Some(Arc::from(keys));
    self
  }

  /// Whether a chevron stands at the very end: the row leads somewhere.
  pub fn chevron(mut self, chevron: bool) -> Self {
    self.chevron = chevron;
    self
  }

  /// The row as the header of what opens under it, open or not: its chevron points down while that is closed and
  /// up while it is open, and it says so to what reads the tree. What a settings card makes of its header.
  pub(crate) fn expands(mut self, open: bool) -> Self {
    self.expands = Some(open);
    self
  }

  /// The row at `density` and drawn in `forced`, where it says neither itself: what the component that holds it
  /// says of its rows.
  pub(crate) fn inherit(mut self, density: Option<Density>, forced: Option<Interaction>) -> Self {
    self.density = self.density.or(density);
    self.forced = self.forced.or(forced);
    self
  }

  /// What the row says.
  pub(crate) fn title_text(&self) -> Arc<str> {
    self.title.clone()
  }

  /// Whether the row has a description under its title.
  pub(crate) fn has_description(&self) -> bool {
    self.description.is_some()
  }

  /// Whether the row is the one selected, or one of them.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Whether the row takes no input: it is dimmed, and is not pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What a press of the row does, given the press: the row is pressed, a stop of Tab, and a button to what reads
  /// the tree. A second handler is called after the first.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Called when a person asks for what the row opens beside what is open, in place of pressing it: a press of the
  /// middle button, or a press with Ctrl held (Cmd on macOS), and Enter with it while the row is focused. An app
  /// with tabs opens the place in a new one. Such a press is the handler's alone: the row's
  /// [`on_click`](Self::on_click) is not called, its [`href`](Self::href) is not gone to, it takes no focus and does
  /// not reach what is around it. It is given the press, whose `shift` asks for a tab that is switched to. A row
  /// with only this handler is pressed too, and a plain press of it does nothing. A second handler is called after
  /// the first.
  pub fn on_open_aside(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_open_aside.push(handler.into_event_handler());
    self
  }

  /// Whether Tab stops at a row that is pressed; it does unless told otherwise. `false` is a row the pointer and
  /// the app's own shortcuts reach: Tab passes it by (what stands at its end is a stop of its own as before), and
  /// a press of it leaves the focus where it was. Focused by the app or by the arrows of the list it is in, it is
  /// pressed by Enter and Space as before, and it is the same row to what reads the tree.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// Where a press of the row goes, in the app's router: the row is pressed, and a link to what reads the tree.
  /// Outside a router it goes nowhere, and its [`on_click`](Self::on_click) is still called.
  pub fn href(mut self, path: &str) -> Self {
    self.href = Some(Arc::from(path));
    self
  }

  /// The density the row is at, whatever its theme is at: how high it is.
  pub fn density(mut self, density: Density) -> Self {
    self.density = Some(density);
    self
  }

  /// Puts the row in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this row over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this row: what the list it is in says of it.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the row as it is under the pointer, held down or focused from the keyboard, with nothing pointing at
  /// it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the row's element: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the row in `theme`. Converting a row into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressable = !self.on_click.is_empty() || !self.on_open_aside.is_empty() || self.href.is_some();
    let case = Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ListItemState::Selected, self.selected)
    .state(ListItemState::Disabled, self.disabled)
    .state(ListItemState::Pressable, pressable)
    .state(
      ListItemState::Description,
      self.description.is_some() || self.description_pending,
    )
    .state(ListItemState::Icon, matches!(self.lead, Some(Lead::Icon(_))));
    let recipe = theme
      .resolve::<ListItem>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let indicator = match &self.lead {
      Some(Lead::Checkbox(checked)) => Some(
        theme.resolve::<super::Checkbox>(
          &Case::new()
            .state(CheckboxState::Checked, *checked)
            .state(CheckboxState::Disabled, self.disabled)
            .breakpoint(super::current_breakpoint()),
          None,
        ),
      ),
      Some(Lead::Radio(chosen)) => Some(
        theme.resolve::<super::Radio>(
          &Case::new()
            .state(RadioState::Selected, *chosen)
            .state(RadioState::Disabled, self.disabled)
            .breakpoint(super::current_breakpoint()),
          None,
        ),
      ),
      _ => None,
    }
    // The box's parts are the row's, named after its id: `x-box`.
    .map(|indicator| indicator.owned_like(&recipe));
    let press = self.press();
    let setting = case.states.contains(&Name::from(ListItemState::Setting));
    let info = self
      .info
      .as_deref()
      .map(|info| super::InfoLabel::mark_declared(info, theme));
    let view = ListItemView {
      title: self.title,
      title_pending: self.title_pending,
      title_tooltip_when_cut: self.title_tooltip_when_cut,
      description: self.description,
      description_pending: self.description_pending,
      tags: self.tags,
      info,
      subtitles: self.subtitles,
      details: self.details,
      below: self.below,
      value: self.value,
      meta: self.meta,
      shortcut: self.shortcut,
      chevron: self.chevron,
      expands: self.expands,
      selected: self.selected,
      disabled: self.disabled,
      link: self.href.is_some(),
      recipe,
      indicator,
      toggle: match &self.lead {
        Some(Lead::Checkbox(on)) => Some(("checkbox", *on)),
        Some(Lead::Radio(on)) => Some(("radio", *on)),
        _ => None,
      },
      lead: self.lead,
      trailing: self.trailing,
      forced: self.forced,
      press,
      aside: self.on_open_aside,
      tab_stop: self.tab_stop,
      id: self.id,
      key: self.key,
      setting,
      wraps: false,
    };
    match theme.renderer::<ListItem>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// What a press of the row does: its handlers, then its path in the router around it. `None` for a row that is
  /// not pressed.
  fn press(&self) -> Option<Press> {
    if self.on_click.is_empty() && self.on_open_aside.is_empty() && self.href.is_none() {
      return None;
    }
    let handlers = self.on_click.clone();
    #[cfg(feature = "router")]
    let go = self.href.clone().zip(crate::router::current_navigator());
    Some(Arc::new(move |event: &MouseEvent| {
      for handler in &handlers {
        handler.call(event);
      }
      #[cfg(feature = "router")]
      if let Some((path, navigator)) = &go {
        navigator.push(path.to_string());
      }
    }))
  }
}

impl From<ListItem> for Element {
  fn from(item: ListItem) -> Self {
    item.build(&current_theme())
  }
}

/// A list item as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct ListItemView {
  pub title: Arc<str>,
  /// Whether the title is still read: a bone stands in its place.
  pub title_pending: bool,
  /// Whether the whole title is said in a tooltip while it is cut.
  pub title_tooltip_when_cut: bool,
  pub description: Option<Arc<str>>,
  /// Whether the description is still read: a bone stands in its place.
  pub description_pending: bool,
  /// The app's elements after the title.
  pub tags: Vec<Element>,
  /// The mark after the title that explains the row, with its tooltip.
  pub info: Option<Element>,
  /// The app's elements between the title's line and the description.
  pub subtitles: Vec<Element>,
  /// The app's elements under the description.
  pub details: Vec<Element>,
  /// What the row says under itself.
  pub below: Vec<Element>,
  pub value: Option<Arc<str>>,
  pub meta: Option<Arc<str>>,
  /// The keys at its end, as [`Kbd::shortcut`] writes them.
  pub shortcut: Option<Arc<str>>,
  pub chevron: bool,
  /// Whether what the row opens is open, where it is the header of something that opens under it: its chevron
  /// points down or up in place of right.
  pub expands: Option<bool>,
  pub selected: bool,
  pub disabled: bool,
  /// Whether a press goes somewhere: the row is a link to what reads the tree.
  pub link: bool,
  /// The style of each part of this row, in its case.
  pub recipe: ResolvedRecipe,
  /// The style of the checkbox or the radio before the text, where the row has one: their own recipe's parts.
  pub indicator: Option<ResolvedRecipe>,
  /// What the row is to tooling where it is a checkbox or a radio, and whether it is checked: kept apart from what
  /// stands before the text, which `leading` takes.
  toggle: Option<(&'static str, bool)>,
  lead: Option<Lead>,
  trailing: Option<Element>,
  forced: Option<Interaction>,
  press: Option<Press>,
  /// What a press that asks for the row beside what is open calls, in place of `press`.
  aside: Vec<EventHandler<MouseEvent>>,
  tab_stop: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// Whether the row is a setting: its text gives way only down to [`SETTING_TEXT_LEAST`], and then what is at
  /// its end goes under it.
  setting: bool,
  /// Whether the root wraps what it holds onto a second line: a setting with something at its end.
  wraps: bool,
}

/// The least a setting's title and description are given beside what is at the row's end. In a narrower room
/// (a card in a window as narrow as a phone) the row is two lines: the icon and the text, and under them the
/// control, so that no description is broken in the middle of its words. A wide room never reaches it.
const SETTING_TEXT_LEAST: f32 = 80.0;

impl ListItemView {
  /// The style of a part, as this row is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the row is pressed: it has a handler or a path.
  pub fn pressable(&self) -> bool {
    self.press.is_some()
  }

  /// The row in the style of its root, holding `children`: a button that is pressed and focused where the row is
  /// pressed (a link with a path, a checkbox or a radio where one is before its text), and a list item that is
  /// neither where it is not. Its title is what typeahead in a list reads.
  pub fn root(&self, children: Vec<Element>) -> Element {
    let part = self.part(ListItemPart::Root);
    let flag = |on: bool| if on { "true" } else { "false" };
    let Some(press) = self.press.clone() else {
      let mut row = Row::new()
        .align_items(Alignment::Center)
        .describe("a11y_role", "listitem")
        .describe("a11y_name", self.title.clone())
        .part(&part);
      if self.pending() {
        row = row.describe("a11y_busy", "true");
      }
      if self.selected {
        row = row.describe("a11y_selected", "true");
      }
      if self.disabled {
        row = row.describe("a11y_disabled", "true");
      }
      if let Some(id) = &self.id {
        row = row.id(id.clone());
      }
      if let Some(key) = &self.key {
        row = row.key(key.clone());
      }
      if self.wraps {
        row = row.wrap();
      }
      return row.with_children(children).into();
    };
    let role = match self.toggle {
      Some((role, _)) => role,
      None if self.link => "link",
      None => "button",
    };
    // Ahead of the press, which a press that asks for the row aside does not reach.
    let mut row = opens_aside(tab_stop(pressable(&part, self.disabled), self.tab_stop), &self.aside)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      // What the app put at the row's end is a stop of Tab of its own, after the row.
      .stops_inside(true)
      .describe("a11y_role", role)
      .describe("a11y_name", self.title.clone())
      .focus_label(self.title.clone())
      .on_click(move |event: MouseEvent| press(&event));
    if let Some(description) = self.description.as_ref().filter(|_| !self.description_pending) {
      row = row.describe("a11y_description", description.clone());
    }
    if self.pending() {
      row = row.describe("a11y_busy", "true");
    }
    match self.toggle {
      Some((_, on)) => row = row.describe("a11y_checked", flag(on)),
      None if self.selected => row = row.describe("a11y_selected", "true"),
      None => {}
    }
    if let Some(open) = self.expands {
      row = row.describe("a11y_expanded", flag(open));
    }
    if self.disabled {
      row = row.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    if let Some(key) = &self.key {
      row = row.key(key.clone());
    }
    if self.wraps {
      row.update_node(crate::node::NodeUpdate::wrap);
    }
    row.with_children(children).into()
  }

  /// What stands before the text, in the style of its part, where the row has anything there.
  pub fn leading(&mut self) -> Option<Row> {
    let lead = self.lead.take()?;
    let part = self.part(ListItemPart::Leading);
    let held = Row::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&part);
    let shown: Element = match lead {
      Lead::Element(element) => element,
      Lead::Icon(name) => glyph(&self.part(ListItemPart::Icon), name, false),
      Lead::Checkbox(checked) => {
        let indicator = self.indicator.as_ref()?;
        let mut square = Row::new()
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .part(&indicator.part(CheckboxPart::Box));
        if checked {
          square = square.child(glyph(
            &indicator.part(CheckboxPart::Indicator),
            Name::new(icons::CHECK),
            false,
          ));
        }
        square.into()
      }
      Lead::Radio(chosen) => {
        let indicator = self.indicator.as_ref()?;
        let mut mark = Row::new()
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .part(&indicator.part(RadioPart::Mark));
        if chosen {
          mark = mark.child(Row::new().part(&indicator.part(RadioPart::Dot)));
        }
        mark.into()
      }
    };
    Some(held.child(shown))
  }

  /// The title over the description, as wide as the room the row leaves them: a longer title is cut with an
  /// ellipsis, a longer description goes on to the next line. The tags and the mark that explains the row stand
  /// after the title on its line, the app's subtitles under that line, and the app's details under the description.
  pub fn content(&self) -> Column {
    let fill = Dimension::Pct(100.0);
    let part = self.part(ListItemPart::Title);
    // The title's node is the part's, and has its id: `x-title`; while it is read, the line that holds its bone.
    let title: Element = if self.title_pending {
      self.bone(ListItemPart::Title, ListItemPart::TitleSkeleton).into()
    } else {
      let text = part
        .text(&self.title)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis)
        .part(&part);
      if self.title_tooltip_when_cut {
        let cut = crate::core::ElementRef::new();
        super::Tooltip::new(&self.title)
          .when_cut(&cut)
          .declared(text.ref_element(cut.clone()))
      } else {
        text.into()
      }
    };
    let line: Element = if self.tags.is_empty() && self.info.is_none() {
      Column::new()
        .width(fill)
        .min_width(Dimension::Px(0.0))
        .child(title)
        .into()
    } else {
      self.title_line(title).into()
    };
    let mut content = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(ListItemPart::Content))
      .child(line);
    for subtitle in &self.subtitles {
      content = content.child(
        self
          .held(Row::new().part(&self.part(ListItemPart::Subtitle)))
          .max_width(fill)
          .child(subtitle.clone()),
      );
    }
    if self.description_pending {
      content = content.child(self.bone(ListItemPart::Description, ListItemPart::DescriptionSkeleton));
    } else if let Some(description) = &self.description {
      let part = self.part(ListItemPart::Description);
      content = content.child(
        Column::new()
          .width(fill)
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(description).width(fill)),
      );
    }
    for detail in &self.details {
      content = content.child(
        self
          .held(Row::new().part(&self.part(ListItemPart::Detail)))
          .max_width(fill)
          .child(detail.clone()),
      );
    }
    content
  }

  /// Whether the title or the description is still read.
  fn pending(&self) -> bool {
    self.title_pending || self.description_pending
  }

  /// The bone in place of the words of `text` (the title or the description) while they are read: a skeleton's line
  /// as wide and as high as the recipe's part `bone` says, which gives way where the room is narrower, on a line of
  /// the text's height, in the part `text`. A space no wider than nothing holds the line's height.
  pub fn bone(&self, text: ListItemPart, bone: ListItemPart) -> Row {
    let part = self.part(text);
    let size = self.part(bone).rest();
    let skeleton = super::Skeleton::line()
      .width(size.width.unwrap_or(Length::Fill))
      .height(size.height.unwrap_or(Length::Px(12.0)))
      .max_width(Length::Fill)
      .min_width(0.0)
      .flex_shrink(1.0)
      .rules(&self.recipe.lend(bone));
    Row::new()
      .width(Dimension::Pct(100.0))
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .part(&part)
      .child(part.text("\u{a0}").nowrap().width(Dimension::Px(0.0)))
      .child(skeleton.build(self.theme()))
  }

  /// The title's line with the tags and the mark that explains the row after the title, which gives way first.
  fn title_line(&self, title: Element) -> Row {
    let mut line = Row::new()
      .width(Dimension::Pct(100.0))
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(Row::new().flex_shrink(1.0).min_width(Dimension::Px(0.0)).child(title));
    if !self.tags.is_empty() {
      line = line.child(
        Row::new()
          .flex_shrink(0.0)
          .align_items(Alignment::Center)
          .overflow_visible()
          .part(&self.part(ListItemPart::Tags))
          .with_children(self.tags.iter().cloned()),
      );
    }
    if let Some(info) = &self.info {
      line = line.child(
        self
          .held(Row::new().flex_shrink(0.0).part(&self.part(ListItemPart::Info)))
          .child(info.clone()),
      );
    }
    line
  }

  /// `row` holding what the app put in the row, whose presses are its own in a row that is pressed.
  fn held(&self, row: Row) -> Row {
    let row = row.align_items(Alignment::Center).overflow_visible();
    if self.pressable() {
      row.on_click(|event: MouseEvent| event.stop_propagation())
    } else {
      row
    }
  }

  /// What the row says under itself, across it, in the style of its part: `None` where it says nothing.
  pub fn below(&mut self) -> Option<Column> {
    if self.below.is_empty() {
      return None;
    }
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Stretch)
        .part(&self.part(ListItemPart::Below))
        .with_children(std::mem::take(&mut self.below)),
    )
  }

  /// What stands after the text, in the style of its part: the value, the note, what the app puts there, the keys
  /// and the chevron, in that order. `None` where the row has none of them.
  pub fn trailing(&mut self) -> Option<Row> {
    let mut ends: Vec<Element> = Vec::new();
    if let Some(value) = &self.value {
      let part = self.part(ListItemPart::Value);
      ends.push(Row::new().part(&part).child(part.text(value).nowrap()).into());
    }
    if let Some(meta) = &self.meta {
      let part = self.part(ListItemPart::Meta);
      ends.push(Row::new().part(&part).child(part.text(meta).nowrap()).into());
    }
    if let Some(element) = self.trailing.take() {
      // In a row that is pressed, a press of the app's control is the control's and not the row's.
      ends.push(if self.pressable() {
        Row::new()
          .align_items(Alignment::Center)
          .overflow_visible()
          .on_click(|event: MouseEvent| event.stop_propagation())
          .child(element)
          .into()
      } else {
        element
      });
    }
    if let Some(keys) = &self.shortcut {
      ends.push(Kbd::shortcut(keys).platform(KeyPlatform::current()).build(self.theme()));
    }
    let chevron = match self.expands {
      Some(true) => Some(icons::CHEVRON_UP),
      Some(false) => Some(icons::CHEVRON_DOWN),
      None => self.chevron.then_some(icons::CHEVRON_RIGHT),
    };
    if let Some(chevron) = chevron {
      ends.push(glyph(&self.part(ListItemPart::Chevron), Name::new(chevron), false));
    }
    if ends.is_empty() {
      return None;
    }
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(ListItemPart::Trailing))
        .with_children(ends),
    )
  }

  /// How the default renderer builds a row: what is before the text, the text, and what is after it, side by side.
  pub fn render(mut self) -> Element {
    let leading = self.leading();
    let mut content = self.content();
    let trailing = self.trailing();
    if self.setting && trailing.is_some() {
      // The text is as wide as the room the end leaves it, down to its least; under that the end wraps. In a
      // wrapping row a growing child is its basis long when the lines are broken, so the least is the basis.
      content = content
        .flex_full(1.0, 0.0, Some(SETTING_TEXT_LEAST))
        .min_width(Dimension::Px(SETTING_TEXT_LEAST));
      self.wraps = true;
    }
    let mut children: Vec<Element> = Vec::new();
    children.extend(leading.map(Into::into));
    children.push(content.into());
    children.extend(trailing.map(Into::into));
    let Some(below) = self.below() else {
      return self.root(children);
    };
    // The row and what it says under itself are one item: the key is on what holds both, the id on the row.
    let key = self.key.take();
    let mut item = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .part(&self.part(ListItemPart::Item))
      .child(self.root(children))
      .child(below);
    if let Some(key) = key {
      item = item.key(key);
    }
    item.into()
  }
}

impl crate::ui::Styled for ListItem {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ListItem {
  const NAME: &'static str = "list_item";
  type Renderer = ListItemRenderer;

  fn parts() -> crate::ui::Declared {
    ListItemPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ListItemState::declared()
  }

  /// The design's list item (`pencil/lurq-design-system.pen`, List Item, with the Contained List, the Settings Row
  /// and the mobile rows beside it): a row of 52 with a title and a description, the avatar of 32 before them and
  /// the time after them, 12 apart, 12 in from its sides, with the medium radius. The title is the body's type at
  /// 14, 500 over a description; the description 12 in the muted color, 2 under it; the time the mono type at 11 in
  /// the muted color. A row of one line is 36, as the contained list's are, and a value, a chevron of 14 and an
  /// icon of 16 are in the muted color. The selected row is in the highlight (the board's `$accent`). At each
  /// density a row is as high as the theme says for it; under a finger it is the mobile row: 56 high, 16 in, its
  /// title at 16, its value at 15, its chevron 18, its icon of 18 on a tile of 32 in the muted fill. The board draws
  /// no row under the pointer, held down, focused or disabled: a pressed row has the veils of those states, the
  /// ring inside it, and a disabled row is faded.
  ///
  /// A setting (Settings Row, and the Settings Card's own row) is 14 in from its sides and 12 from its top and
  /// bottom, 12 between what it holds, square: its title 13 at 500 over a description of 12, the board's 57 of
  /// two lines; an icon of 20 in the foreground before them, the control 12 from a chevron of 16 at its end. The
  /// board draws no setting of one line: it is 44 at the least. Its height and the room over and under it follow
  /// the density, by a row's step and half of it.
  fn recipe() -> Recipe {
    use ListItemPart::{
      Below, Chevron, Content, Description, DescriptionSkeleton, Detail, Icon, Info, Leading, Meta, Root, Tags, Title,
      TitleSkeleton, Trailing, Value,
    };
    use ListItemState::{Contained, Disabled, Pressable, Selected, Setting};
    let (two_lines, with_icon) = (ListItemState::Description, ListItemState::Icon);
    use PaletteColor::{Foreground, Highlight, Muted, MutedForeground, Ring, StateHover, StatePressed};
    let two_line = |density: Density| Selector::new().density(density).state(two_lines);
    let recipe = Recipe::new()
      .base(Root, |s| {
        fading(
          s.min_height(36.0)
            .padding_x(12.0)
            .gap(12.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground),
        )
      })
      .density(Density::Compact, Root, |s| s.min_height(28.0))
      .density(Density::Comfortable, Root, |s| s.min_height(44.0))
      .density(Density::Touch, Root, |s| s.min_height(56.0).padding_x(16.0))
      // Two lines: 52 at the default, 16 more than one line at every density, the text 6 from the edges at least
      // (4 at the compact density).
      .state(two_lines, Root, |s| s.min_height(52.0).padding_y(6.0))
      // At the compact density two lines of text and 6 over and under them are more than 44: the room is 4.
      .when(two_line(Density::Compact), Root, |s| s.min_height(44.0).padding_y(4.0))
      .when(two_line(Density::Comfortable), Root, |s| s.min_height(60.0))
      .when(two_line(Density::Touch), Root, |s| s.min_height(72.0))
      .state(Pressable, Root, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Root, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Root, |s| s.veil(StatePressed))
      // The ring is inside the row: rows touch in a contained list, and the list may clip.
      .focus_visible(Root, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Selected, Root, |s| s.background(Highlight))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default))
      // The contained list's rows: 36, square, touching, their text 8 from what is at their end.
      .state(Contained, Root, |s| s.radius(0.0).gap(8.0))
      .base(Leading, |s| s.gap(0.0))
      .base(Icon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .when(Selector::new().density(Density::Touch).state(with_icon), Leading, |s| {
        s.square(32.0).radius(RadiusSize::Lg).background(Muted)
      })
      .density(Density::Touch, Icon, |s| s.square(18.0).foreground(Foreground))
      .base(Content, |s| s.gap(2.0))
      .base(Title, |s| s.typography(TypographyStyle::Body).font_size(14.0))
      .state(two_lines, Title, |s| s.weight(FontWeight::Medium))
      .state(Contained, Title, |s| s.font_size(13.0))
      .density(Density::Touch, Title, |s| s.font_size(16.0))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .density(Density::Touch, Description, |s| s.font_size(14.0))
      // lurq's, the board has none: tags 6 after the title and 6 apart, the mark that explains the row 6 after
      // them; a detail 2 under the description; what the row says under itself 12 in from its sides, 8 above its
      // bottom, 8 apart.
      .base(Tags, |s| s.padding_start(6.0).gap(6.0))
      .base(Info, |s| s.padding_start(6.0))
      .base(Detail, |s| s.padding_top(2.0))
      // lurq's: a title's bone 140 by 12, a description's 220 by 10, each giving way in a narrower row.
      .base(TitleSkeleton, |s| s.width(140.0).height(12.0))
      .base(DescriptionSkeleton, |s| s.width(220.0).height(10.0))
      .base(Below, |s| s.padding_x(12.0).padding_bottom(8.0).gap(8.0))
      .density(Density::Touch, Below, |s| s.padding_x(16.0))
      .base(Trailing, |s| s.gap(8.0).foreground(MutedForeground))
      .base(Value, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .density(Density::Touch, Value, |s| s.font_size(15.0))
      .base(Meta, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Chevron, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .density(Density::Touch, Chevron, |s| s.square(18.0));
    // A setting: written after the rest, so that among the rules of one state these are the ones that hold.
    let recipe = recipe
      .state(Setting, Root, |s| {
        s.min_height(44.0).padding_x(14.0).padding_y(12.0).gap(12.0).radius(0.0)
      })
      .state(Setting, Title, |s| s.font_size(13.0).weight(FontWeight::Medium))
      .state(Setting, Description, |s| s.font_size(12.0))
      .state(Setting, Icon, |s| s.square(20.0).foreground(Foreground))
      .state(Setting, Trailing, |s| s.gap(12.0))
      .state(Setting, Chevron, |s| s.square(IconSize::Md))
      // Under a setting, 14 in from its sides as its line is, and 12 above its bottom.
      .state(Setting, Below, |s| s.padding_x(14.0).padding_bottom(12.0));
    // Two lines of the theme's type are a pixel under the board's, whose lines are whole pixels: the row is the
    // board's 57 at the least.
    let described = || Selector::new().state(Setting).state(two_lines);
    let recipe = recipe.when(described(), Root, |s| s.min_height(57.0));
    ROW_STEPS.iter().fold(recipe, |recipe, (density, step)| {
      recipe
        .when(Selector::new().density(*density).state(Setting), Root, |s| {
          s.min_height(44.0 + step).padding_y(12.0 + step / 2.0)
        })
        .when(described().density(*density), Root, |s| s.min_height(57.0 + step))
    })
  }
}

/// The parts of a list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListPart {
  /// The list: its rows, and in a contained list the box around them and its header.
  Root,
  /// The bar over the rows of a contained list, with its title and its action.
  Header,
  /// What the header says.
  Title,
  /// What the app puts at the header's end: a button.
  Action,
  /// The rows, one under the other: the room between them.
  Items,
  /// What holds the line between two rows: its padding at the start is the inset the line starts at.
  Separator,
}

names!(ListPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Action => "action",
  Items => "items",
  Separator => "separator",
});

/// The states of a list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListState {
  /// A contained list: its rows in a box, touching, with a line between each two, under a header.
  Contained,
  /// A line between each two rows.
  Separator,
}

names!(ListState {
  Contained => "contained",
  Separator => "separator",
});

/// What builds a list's nodes: a theme's in place of the default one.
pub type ListRenderer = Arc<dyn Fn(ListView) -> Element + Send + Sync>;

/// A list of rows, one under the other: [`ListItem`]s with a line between each two where it has them, the inset
/// the line starts at, and, as a contained list, a box around them with a header.
///
/// ```
/// use lurq::ui::{Button, List, ListItem, icons};
///
/// let recent = List::new()
///   .contained(true)
///   .title("Recent")
///   .action(Button::new("Clear").variant("ghost").size("sm"))
///   .item(ListItem::new("theme.md").chevron(true).on_click(|_| println!("theme.md")))
///   .item(ListItem::new("blur.md").chevron(true).on_click(|_| println!("blur.md")));
/// let settings = List::new()
///   .separators(true)
///   .inset(48.0)
///   .item(ListItem::new("Wi-Fi").icon(icons::WIFI).value("Home").chevron(true))
///   .item(ListItem::new("Bluetooth").icon(icons::BLUETOOTH).value("On").chevron(true));
/// ```
///
/// To what reads the tree it is a list of its rows. The rows that are pressed are stops of Tab each, and Up and
/// Down, Home and End move between them, and a letter to the next whose title starts with it. A list of which
/// one row or many are chosen is a [`Listbox`](super::Listbox).
pub struct List {
  items: Vec<ListItem>,
  title: Option<Arc<str>>,
  action: Option<Element>,
  contained: bool,
  separators: Option<bool>,
  inset: f32,
  label: Option<Arc<str>>,
  density: Option<Density>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for List {
  fn default() -> Self {
    Self::new()
  }
}

impl List {
  /// A list with no rows yet.
  pub fn new() -> Self {
    Self {
      items: Vec::new(),
      title: None,
      action: None,
      contained: false,
      separators: None,
      inset: 0.0,
      label: None,
      density: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A row, after those it holds already.
  pub fn item(mut self, item: ListItem) -> Self {
    self.items.push(item);
    self
  }

  /// Rows, in order, after those it holds already.
  pub fn items(mut self, items: impl IntoIterator<Item = ListItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// Whether it is a contained list: its rows in a box, touching, with a line between each two, under its header.
  pub fn contained(mut self, contained: bool) -> Self {
    self.contained = contained;
    self
  }

  /// What the header over the rows says, in a contained list. It is written in capitals, as the board's is.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// What stands at the end of the header: a button that acts on the list. It is the one action: a second takes
  /// its place.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Whether a line runs between each two rows. A contained list has them unless told otherwise.
  pub fn separators(mut self, separators: bool) -> Self {
    self.separators = Some(separators);
    self
  }

  /// How far from the start the lines between rows start: the room of what is before the rows' text, so that a
  /// line runs under the text alone. 0 unless told.
  pub fn inset(mut self, inset: f32) -> Self {
    self.inset = inset;
    self
  }

  /// What the list is called, to what reads the tree. It draws nothing; a contained list is called by its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The density its rows are at, whatever the theme is at.
  pub fn density(mut self, density: Density) -> Self {
    self.density = Some(density);
    self
  }

  /// Puts the list in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this list over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this list.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws every row that is pressed as it is under the pointer, held down or focused from the keyboard, with
  /// nothing pointing at it, unless the row says otherwise: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the list's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the list in `theme`. Converting a list into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let separators = self.separators.unwrap_or(self.contained);
    let case = Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ListState::Contained, self.contained)
    .state(ListState::Separator, separators);
    let (contained, density, forced) = (self.contained, self.density, self.forced);
    let recipe = theme
      .resolve::<List>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    // A row with no id of its own is named after the list's: `x-item`, `x-item-2`.
    let lent = recipe.lend("item");
    let items = self
      .items
      .into_iter()
      .map(|mut item| {
        item = item.state(ListItemState::Contained, contained).rules(&lent);
        if item.density.is_none() {
          item.density = density;
        }
        if item.forced.is_none() {
          item.forced = forced;
        }
        item.build(theme)
      })
      .collect();
    let view = ListView {
      items,
      title: self.title,
      action: self.action,
      contained,
      separators,
      inset: self.inset,
      label: self.label,
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<List>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<List> for Element {
  fn from(list: List) -> Self {
    list.build(&current_theme())
  }
}

/// A list as a renderer sees it: its rows, built, its header, and the style of its parts.
pub struct ListView {
  /// The rows, built, in order.
  pub items: Vec<Element>,
  /// What the header of a contained list says.
  pub title: Option<Arc<str>>,
  pub contained: bool,
  /// Whether a line runs between each two rows.
  pub separators: bool,
  /// How far from the start the lines start.
  pub inset: f32,
  pub label: Option<Arc<str>>,
  /// The style of each part of this list.
  pub recipe: ResolvedRecipe,
  action: Option<Element>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ListView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The list in the style of its root, with nothing in it yet: a list to what reads the tree, called by its label
  /// or its header.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "list")
      .part(&self.part(ListPart::Root));
    if let Some(name) = self.label.as_ref().or(self.title.as_ref()) {
      root = root.describe("a11y_name", name.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The bar over the rows of a contained list, with its title and its action: `None` for a list with neither.
  pub fn header(&mut self) -> Option<Row> {
    if self.title.is_none() && self.action.is_none() {
      return None;
    }
    let mut header = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(ListPart::Header));
    let title = self.part(ListPart::Title);
    header = header.child(
      Column::new()
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
        .part(&title)
        .child(title.text(&self.title.as_deref().unwrap_or("").to_uppercase()).nowrap()),
    );
    if let Some(action) = self.action.take() {
      header = header.child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(ListPart::Action))
          .child(action),
      );
    }
    Some(header)
  }

  /// The line between two rows, from the list's inset to its end: it takes no room of its own, and is drawn along
  /// the bottom of the row over it, inside it, as the board draws a row's line.
  pub fn separator(&self) -> Column {
    Column::new()
      .align_items(Alignment::Stretch)
      .height(Dimension::Px(0.0))
      .overflow_visible()
      .part(&self.part(ListPart::Separator))
      .padding_left(self.inset)
      .child(
        Column::new()
          .align_items(Alignment::Stretch)
          .overflow_visible()
          .offset_by_size(0.0, -1.0)
          .child(Separator::horizontal().build(self.theme())),
      )
  }

  /// The rows one under the other, with a line between each two where the list has them. The rows that are pressed
  /// are one group to the arrow keys.
  pub fn rows(&mut self) -> Column {
    let items = std::mem::take(&mut self.items);
    let count = items.len();
    let mut column = Column::new()
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::vertical().wrap(false).typeahead(true))
      .part(&self.part(ListPart::Items));
    for (index, item) in items.into_iter().enumerate() {
      column = column.child(item);
      if self.separators && index + 1 < count {
        column = column.child(self.separator());
      }
    }
    column
  }

  /// How the default renderer builds a list: its header over its rows.
  pub fn render(mut self) -> Element {
    let header = self.header();
    let rows = self.rows();
    let mut root = self.root();
    if let Some(header) = header {
      root = root.child(header);
    }
    root.child(rows).into()
  }
}

impl crate::ui::Styled for List {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for List {
  const NAME: &'static str = "list";
  type Renderer = ListRenderer;

  fn parts() -> crate::ui::Declared {
    ListPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ListState::declared()
  }

  /// The design's lists (`pencil/lurq-design-system.pen`, the list of List Items, the Contained List and the mobile
  /// grouped list): the rows of a list 2 apart; with lines between them, touching, a line of 1 in the border's color
  /// from the inset to the end. A contained list is a box in the background with an edge of 1 in the border's color
  /// and the large radius, clipping its rows; its header is 32 high on the bar's fill, 12 in from its sides, with a
  /// line of 1 under it, its title the mono type at 11 in the muted color, 1 apart.
  fn recipe() -> Recipe {
    use ListPart::{Action, Header, Items, Root, Title};
    use ListState::{Contained, Separator};
    use PaletteColor::{Background, Border as Line, MutedForeground};
    Recipe::new()
      .base(Root, |s| s.foreground(PaletteColor::Foreground))
      .state(Contained, Root, |s| {
        s.background(Background)
          .border(Border::inside(BorderSize::Sm, Line))
          .radius(RadiusSize::Lg)
      })
      .base(Items, |s| s.gap(2.0))
      .state(Separator, Items, |s| s.gap(0.0))
      .base(Header, |s| {
        s.height(32.0)
          .padding_x(12.0)
          .gap(8.0)
          .background(PaletteColor::Bar)
          .underline(Border::inside(BorderSize::Sm, Line))
      })
      .base(Title, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .letter_spacing(1.0)
          .foreground(MutedForeground)
      })
      .base(Action, |s| s.gap(4.0))
  }
}
