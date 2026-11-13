//! A breadcrumb: the way to where one is, a step for each place on it.

use std::{ops::Range, sync::Arc};

use super::{
  icons,
  menu::{Menu, MenuItem},
  names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Placement,
    theme::{IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element},
};

/// The parts of a breadcrumb a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BreadcrumbPart {
  /// The row of the steps and what stands between them.
  Root,
  /// A step that is a link: what is pressed to go there, with its icon and its text.
  Link,
  /// The last step: the page one is on, which is text and not a link.
  Page,
  /// The icon of a step, before its text.
  Icon,
  /// What stands between two steps: an icon, or a text such as a slash.
  Separator,
  /// The icon of a separator.
  SeparatorIcon,
  /// What stands for the steps that are hidden: what is pressed to open the menu of them.
  Ellipsis,
  /// The icon of the ellipsis.
  EllipsisIcon,
}

names!(BreadcrumbPart {
  Root => "root",
  Link => "link",
  Page => "page",
  Icon => "icon",
  Separator => "separator",
  SeparatorIcon => "separator_icon",
  Ellipsis => "ellipsis",
  EllipsisIcon => "ellipsis_icon",
});

/// The states of a breadcrumb a recipe styles, besides what the pointer and
/// the keyboard do to its steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BreadcrumbState {
  /// Some steps are hidden behind the ellipsis.
  Collapsed,
  /// It takes no input.
  Disabled,
}

names!(BreadcrumbState {
  Collapsed => "collapsed",
  Disabled => "disabled",
});

/// What builds a breadcrumb's nodes: a theme's in place of the default one.
pub type BreadcrumbRenderer = Arc<dyn Fn(BreadcrumbView) -> Element + Send + Sync>;

/// What builds one part of a breadcrumb: a theme's, for
/// [`BreadcrumbPart::Separator`], built once for each place between two steps.
pub type BreadcrumbPartRenderer = Arc<dyn Fn(&BreadcrumbView) -> Element + Send + Sync>;

/// What stands between two steps of a breadcrumb.
#[derive(Clone, Debug, PartialEq)]
pub enum BreadcrumbSeparator {
  /// An icon of the theme's set, by its name: a chevron to the right, the design's.
  Icon(Name),
  /// A text: a slash.
  Text(Arc<str>),
}

impl BreadcrumbSeparator {
  /// A chevron to the right: the design's.
  pub fn chevron() -> Self {
    Self::Icon(Name::new(icons::CHEVRON_RIGHT))
  }

  /// A slash, in the type of the steps.
  pub fn slash() -> Self {
    Self::Text(Arc::from("/"))
  }
}

impl Default for BreadcrumbSeparator {
  fn default() -> Self {
    Self::chevron()
  }
}

type Handler = Arc<dyn Fn() + Send + Sync>;
type ValueHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A step of a breadcrumb: a place on the way to where one is.
///
/// ```
/// use lurq::ui::{BreadcrumbStep, icons};
///
/// let home = BreadcrumbStep::new("Home").icon(icons::HOME).value("/").on_select(|| println!("home"));
/// let plain: BreadcrumbStep = "Settings".into();
/// ```
#[derive(Clone)]
pub struct BreadcrumbStep {
  label: Arc<str>,
  value: Arc<str>,
  icon: Option<Name>,
  on_select: Vec<Handler>,
  id: Option<Arc<str>>,
}

impl BreadcrumbStep {
  /// A step called `label`, whose value is its label until it is given another.
  pub fn new(label: &str) -> Self {
    let label: Arc<str> = Arc::from(label);
    Self {
      value: label.clone(),
      label,
      icon: None,
      on_select: Vec::new(),
      id: None,
    }
  }

  /// What the step stands for, which the breadcrumb's handler is told when it
  /// is followed: a route, a path, a key.
  pub fn value(mut self, value: &str) -> Self {
    self.value = Arc::from(value);
    self
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Called when the step is followed: a click of it, Enter or Space while it
  /// is focused, or choosing it in the menu of the hidden steps. The last step
  /// is the page one is on, and is not followed.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// The id of what is pressed to follow the step, or of the page for the
  /// last one. A hidden step's item in the menu has it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn label(&self) -> &str {
    &self.label
  }

  pub fn value_of(&self) -> &str {
    &self.value
  }

  pub fn icon_name(&self) -> Option<&Name> {
    self.icon.as_ref()
  }
}

impl From<&str> for BreadcrumbStep {
  fn from(label: &str) -> Self {
    Self::new(label)
  }
}

/// A breadcrumb: the way to where one is, a step for each place on it in
/// order. Each step but the last is a link that a press follows; the last is
/// the page one is on.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{Breadcrumb, BreadcrumbStep, icons},
/// };
///
/// # fn build(anchor: ElementRef, open: Signal<bool>) {
/// let way = Breadcrumb::new()
///   .step(BreadcrumbStep::new("Home").icon(icons::HOME).value("/"))
///   .step(BreadcrumbStep::new("Settings").value("/settings"))
///   .step(BreadcrumbStep::new("Appearance").value("/settings/appearance"))
///   .step("Theme")
///   .on_select(|route| println!("go to {route}"));
///
/// // With more than three steps, the first, an ellipsis and the last two; the ellipsis opens a menu of the
/// // others, hung from `anchor` while `open` is.
/// let short = Breadcrumb::new()
///   .steps(["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
///   .max_items(3)
///   .menu(anchor, open);
/// # }
/// ```
///
/// A step is followed by a click, and by Enter or Space while it is focused:
/// the links are stops of Tab in their order, and the page is not. Following
/// a step calls its own handler and the breadcrumb's, told its value; where
/// it goes is the app's. The whole is a navigation called "Breadcrumb" to
/// what reads the tree, and the last step is its current page.
///
/// With more steps than [`max_items`](Breadcrumb::max_items), the breadcrumb
/// shows the first, an ellipsis and the last ones, and the ellipsis opens a
/// [`Menu`] of the steps it hides. What the menu hangs from and whether it is
/// open are the app's ([`menu`](Breadcrumb::menu)), as for any menu.
#[derive(Clone)]
pub struct Breadcrumb {
  steps: Vec<BreadcrumbStep>,
  max_items: Option<usize>,
  separator: BreadcrumbSeparator,
  menu: Option<(ElementRef, Signal<bool>)>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_select: Vec<ValueHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Breadcrumb {
  fn default() -> Self {
    Self::new()
  }
}

impl Breadcrumb {
  /// A breadcrumb with no steps yet.
  pub fn new() -> Self {
    Self {
      steps: Vec::new(),
      max_items: None,
      separator: BreadcrumbSeparator::default(),
      menu: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A step, after the steps the breadcrumb has already. The last is the page one is on.
  pub fn step(mut self, step: impl Into<BreadcrumbStep>) -> Self {
    self.steps.push(step.into());
    self
  }

  /// Steps, in order, after the steps the breadcrumb has already.
  pub fn steps<S: Into<BreadcrumbStep>>(mut self, steps: impl IntoIterator<Item = S>) -> Self {
    self.steps.extend(steps.into_iter().map(Into::into));
    self
  }

  /// The most steps shown: with more, the first, an ellipsis and the last
  /// `max - 1`. Never fewer than two, the first and the last; all of them
  /// without it.
  pub fn max_items(mut self, max: usize) -> Self {
    self.max_items = Some(max);
    self
  }

  /// What stands between two steps: a chevron to the right unless it is given another.
  pub fn separator(mut self, separator: BreadcrumbSeparator) -> Self {
    self.separator = separator;
    self
  }

  /// What the menu of the hidden steps hangs from, and whether it is open:
  /// the app's `Signal<bool>`, which a press of the ellipsis turns, and
  /// choosing a step, a press outside and Escape set to `false`. Without it
  /// the ellipsis opens nothing. The anchor is the ellipsis's own: the app
  /// keeps it so that the menu stays the same menu while it is open.
  pub fn menu(mut self, anchor: ElementRef, open: Signal<bool>) -> Self {
    self.menu = Some((anchor, open));
    self
  }

  /// Whether the breadcrumb takes no input: no step is followed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the breadcrumb in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called when a step is followed, with its value, after the step's own handlers.
  pub fn on_select(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Styles one part of this breadcrumb over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this breadcrumb.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the links and the ellipsis as they are under the pointer, held
  /// down or focused from the keyboard, with nothing pointing at them: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. The ellipsis has it with `-ellipsis` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the breadcrumb in `theme`. Converting a breadcrumb into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let hidden = hidden(self.steps.len(), self.max_items);
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(BreadcrumbState::Collapsed, hidden.is_some())
    .state(BreadcrumbState::Disabled, self.disabled);
    let view = BreadcrumbView {
      steps: self.steps,
      hidden,
      separator: self.separator,
      disabled: self.disabled,
      recipe: theme.resolve::<Breadcrumb>(&case, Some(&self.rules)),
      forced: self.forced,
      menu: self.menu,
      on_select: self.on_select,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Breadcrumb>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Breadcrumb> for Element {
  fn from(breadcrumb: Breadcrumb) -> Self {
    breadcrumb.build(&current_theme())
  }
}

/// The steps the ellipsis stands for among `count`, where no more than `max`
/// are shown: all but the first and the last `max - 1`.
fn hidden(count: usize, max: Option<usize>) -> Option<Range<usize>> {
  let max = max?.max(2);
  (count > max).then(|| 1..count - (max - 1))
}

/// A breadcrumb as a renderer sees it: its steps, which of them are hidden,
/// the style of its parts, and what following a step does.
pub struct BreadcrumbView {
  /// Every step, in order, the hidden ones too.
  pub steps: Vec<BreadcrumbStep>,
  /// The steps the ellipsis stands for, if some are hidden.
  pub hidden: Option<Range<usize>>,
  pub separator: BreadcrumbSeparator,
  pub disabled: bool,
  /// The style of each part of this breadcrumb.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  menu: Option<(ElementRef, Signal<bool>)>,
  on_select: Vec<ValueHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BreadcrumbView {
  /// The style of a part, as this breadcrumb is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the step at `index` is the page one is on: the last.
  pub fn is_current(&self, index: usize) -> bool {
    index + 1 == self.steps.len()
  }

  /// The steps shown, in order, by their index: all of them but the hidden ones.
  pub fn shown(&self) -> Vec<usize> {
    (0..self.steps.len())
      .filter(|index| !self.hidden.as_ref().is_some_and(|hidden| hidden.contains(index)))
      .collect()
  }

  /// What following the step at `index` does: its own handlers, then the
  /// breadcrumb's with its value. `None` for the page, and while disabled.
  pub fn follow(&self, index: usize) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let step = self.steps.get(index)?;
    if self.disabled || self.is_current(index) {
      return None;
    }
    let (own, told, value) = (step.on_select.clone(), self.on_select.clone(), step.value.clone());
    Some(move || {
      for handler in &own {
        handler();
      }
      for handler in &told {
        handler(&value);
      }
    })
  }

  /// The root in the style of its part, with nothing in it yet: a row that
  /// is a navigation called "Breadcrumb" to what reads the tree.
  pub fn root(&self) -> Row {
    // The ring of a focused link stands out of its box: the row does not clip it.
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(BreadcrumbPart::Root))
      .describe("a11y_role", "navigation")
      .describe("a11y_name", "Breadcrumb");
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon of the step at `index`, if it has one.
  fn icon(&self, index: usize) -> Option<Element> {
    let name = self.steps.get(index)?.icon.clone()?;
    Some(glyph(&self.part(BreadcrumbPart::Icon), name, false))
  }

  /// The step at `index`: a link that a press follows, or the page one is on
  /// for the last, in the style of its part, with its icon and its text.
  pub fn step(&self, index: usize) -> Element {
    let step = &self.steps[index];
    if self.is_current(index) {
      let part = self.part(BreadcrumbPart::Page);
      let mut page = Row::new()
        .align_items(Alignment::Center)
        .part(&part)
        .describe("a11y_name", step.label.clone())
        .describe("a11y_current", "page");
      if let Some(icon) = self.icon(index) {
        page = page.child(icon);
      }
      page = page.child(part.text(&step.label).nowrap());
      if let Some(id) = &step.id {
        page = page.id(id.clone());
      }
      return page.into();
    }
    let part = self.part(BreadcrumbPart::Link);
    let mut link = pressable(&part, self.disabled)
      .describe("a11y_role", "link")
      .describe("a11y_name", step.label.clone());
    if let Some(follow) = self.follow(index) {
      link = link.on_click(move |_| follow());
    }
    if let Some(icon) = self.icon(index) {
      link = link.child(icon);
    }
    link = link.child(part.text(&step.label).nowrap());
    if let Some(id) = &step.id {
      link = link.id(id.clone());
    }
    link.into()
  }

  /// What stands between two steps. A theme's renderer for
  /// [`BreadcrumbPart::Separator`] builds it in place of the default one.
  pub fn separator(&self) -> Element {
    if let Some(renderer) = self
      .theme()
      .part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator)
    {
      return renderer(self);
    }
    let part = self.part(BreadcrumbPart::Separator);
    let room = Row::new().align_items(Alignment::Center).part(&part);
    match &self.separator {
      BreadcrumbSeparator::Icon(name) => {
        room.child(glyph(&self.part(BreadcrumbPart::SeparatorIcon), name.clone(), false))
      }
      BreadcrumbSeparator::Text(text) => room.child(part.text(text).nowrap()),
    }
    .into()
  }

  /// The menu of the hidden steps: an item for each, which follows it.
  fn hidden_menu(&self, anchor: ElementRef, open: Signal<bool>) -> Menu {
    let mut menu = Menu::new(anchor, open).placement(Placement::BottomStart);
    for index in self.hidden.clone().unwrap_or(0..0) {
      let step = &self.steps[index];
      let mut item = MenuItem::new(&step.label).disabled(self.disabled);
      if let Some(icon) = &step.icon {
        item = item.icon(icon.clone());
      }
      if let Some(follow) = self.follow(index) {
        item = item.on_select(follow);
      }
      if let Some(id) = &step.id {
        item = item.id(id.clone());
      }
      menu = menu.item(item);
    }
    menu
  }

  /// The ellipsis, with the menu of the hidden steps it opens, if some are
  /// hidden: a button that turns the menu's open signal.
  pub fn ellipsis(&self) -> Option<Element> {
    self.hidden.as_ref()?;
    let part = self.part(BreadcrumbPart::Ellipsis);
    let (anchor, open) = match &self.menu {
      Some((anchor, open)) => (anchor.clone(), Some(open.clone())),
      None => (ElementRef::new(), None),
    };
    let expanded = open.as_ref().is_some_and(|open| open.get());
    let mut button = pressable(&part, self.disabled)
      .ref_element(anchor.clone())
      .describe("a11y_name", "Show the hidden steps")
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if expanded { "true" } else { "false" })
      .child(glyph(
        &self.part(BreadcrumbPart::EllipsisIcon),
        Name::new(icons::MORE_HORIZONTAL),
        false,
      ));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-ellipsis"));
    }
    // The menu is an overlay: a child of a row, beside what it hangs from.
    let mut holder = Row::new().align_items(Alignment::Center).overflow_visible();
    match open {
      Some(open) => {
        if !self.disabled {
          let toggle = open.clone();
          button = button.on_click(move |_| toggle.update(|open| *open = !*open));
        }
        holder = holder.child(button).child(self.hidden_menu(anchor, open));
      }
      None => holder = holder.child(button),
    }
    Some(holder.into())
  }

  /// How the default renderer builds a breadcrumb: the steps shown, a
  /// separator between each two, and the ellipsis in place of the hidden ones.
  pub fn render(self) -> Element {
    let mut root = self.root();
    let shown = self.shown();
    for (place, index) in shown.iter().enumerate() {
      if place > 0 {
        root = root.child(self.separator());
      }
      root = root.child(self.step(*index));
      // The ellipsis stands after the first step, where the hidden ones would be.
      if place == 0
        && let Some(ellipsis) = self.ellipsis()
      {
        root = root.child(self.separator()).child(ellipsis);
      }
    }
    root.into()
  }
}

impl Themed for Breadcrumb {
  const NAME: &'static str = "breadcrumb";
  type Renderer = BreadcrumbRenderer;

  /// The design's breadcrumb (`pencil/lurq-design-system.pen`, Breadcrumb and
  /// Breadcrumb/Collapsed): steps 8 apart in the body's type, the links in
  /// the muted foreground and the page in the foreground at 500, a chevron of
  /// 14 in the muted foreground between each two. The ellipsis is a square of
  /// 24 filled with the highlight (the board's `$accent`), with the small
  /// radius and an icon of 16 in it. Under the pointer a link is in the
  /// foreground.
  fn recipe() -> Recipe {
    use BreadcrumbPart::*;
    use PaletteColor::{Foreground, Highlight, MutedForeground, StateHover, StatePressed};
    Recipe::new()
      .base(Root, |s| s.gap(8.0))
      .base(Link, |s| {
        fading(
          s.gap(4.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .typography(TypographyStyle::Body)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Link, |s| s.foreground(Foreground))
      .focus_visible(Link, |s| s.focus_ring())
      .base(Page, |s| {
        s.gap(4.0)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
          .weight(FontWeight::Medium)
      })
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Separator, |s| {
        s.foreground(MutedForeground).typography(TypographyStyle::Body)
      })
      .base(SeparatorIcon, |s| s.square(IconSize::Sm))
      .base(Ellipsis, |s| {
        fading(
          s.square(24.0)
            .radius(RadiusSize::Sm)
            .background(Highlight)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Ellipsis, |s| s.veil(StateHover))
      .pressed(Ellipsis, |s| s.veil(StatePressed))
      .focus_visible(Ellipsis, |s| s.focus_ring())
      .base(EllipsisIcon, |s| s.square(IconSize::Md))
      .state(BreadcrumbState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(BreadcrumbState::Disabled, Link, |s| s.cursor(CursorIcon::Default))
      .state(BreadcrumbState::Disabled, Ellipsis, |s| s.cursor(CursorIcon::Default))
  }
}
