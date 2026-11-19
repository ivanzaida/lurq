//! A pagination: the pages of a list, one of them the current one, and a press that goes to another.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonVariant},
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{AnySelect, Dress, SelectPart},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{ControlSize, IconSize, PaletteColor, Theme, TypographyStyle},
  },
  components::Row,
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, FocusGroup, NodeUpdate, dimension::Dimension},
};

/// The parts of a pagination a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaginationPart {
  /// The whole: what is before the buttons and the buttons, side by side.
  Root,
  /// The row of buttons, previous, the pages and next, that the arrow keys move along.
  Nav,
  /// What the pagination says of each of its buttons: laid over the theme's recipe for a button's root. The
  /// button of the current page is in [`PaginationState::Current`].
  Item,
  /// The box that stands where pages are left out.
  Ellipsis,
  /// The glyph in it: its side and its color.
  EllipsisIcon,
  /// The text of where the list is, "1–20 of 284" or "Page 3 of 12", in the compact form.
  Summary,
  /// The text before the select of how many to a page: "Rows per page".
  PageSizeLabel,
  /// What the pagination says of that select: laid over the theme's recipe for a select's trigger.
  PageSize,
}

names!(PaginationPart {
  Root => "root",
  Nav => "nav",
  Item => "item",
  Ellipsis => "ellipsis",
  EllipsisIcon => "ellipsis_icon",
  Summary => "summary",
  PageSizeLabel => "page_size_label",
  PageSize => "page_size",
});

/// The states of a pagination a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaginationState {
  /// It is previous and next with where the list is, and no pages.
  Compact,
  /// It has a select of how many to a page, and is as wide as what holds it.
  PageSize,
  /// It takes no input.
  Disabled,
  /// Of an item: it is the button of the current page.
  Current,
}

names!(PaginationState {
  Compact => "compact",
  PageSize => "page_size",
  Disabled => "disabled",
  Current => "current",
});

/// One of what a pagination draws between previous and next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageItem {
  /// The page of this number, counted from 1.
  Page(usize),
  /// Pages left out.
  Ellipsis,
}

/// What builds a pagination's nodes: a theme's in place of the default one.
pub type PaginationRenderer = Arc<dyn Fn(PaginationView) -> Element + Send + Sync>;

type Go = Arc<dyn Fn(usize) + Send + Sync>;

/// The pages of a list, one of them the current one: the results of a
/// search, the rows of a table, the posts of a feed.
///
/// ```
/// use lurq::{core::Signal, ui::{Pagination, Select}};
///
/// # fn pages(page: Signal<usize>, size: Signal<usize>) {
/// // Pages count from 1. The signal has the current one, and a press sets it.
/// let results = Pagination::new(page.clone(), 12);
/// // The pages of 284 rows, 20 to a page, as the footer of a table has them.
/// let footer = Pagination::for_items(page, 284, 20)
///   .compact(true)
///   .page_size("Rows per page", Select::new(size, [(10, "10"), (20, "20"), (50, "50")]));
/// # }
/// ```
///
/// Which pages are drawn is the pagination's: the first and the last always,
/// the current one with [`Pagination::siblings`] on each side of it, and an
/// ellipsis where pages are left out. As long as there are more pages than
/// fit, it draws the same number of things wherever the current page is,
/// `2 × siblings + 5`, so that the row does not change its length as a
/// person goes through it: near an end the pages run on to that end, and the
/// room of the ellipsis that is not needed there goes to a page. An ellipsis
/// always stands for two pages or more. Previous and next are disabled at
/// the ends.
///
/// Each page is a button: Tab reaches each, Left and Right move between them,
/// and Enter or Space goes to the page. To what reads the tree the whole is a
/// navigation called "Pagination", each page is called "Page 3", and the
/// current one says it is.
#[derive(Clone)]
pub struct Pagination {
  current: usize,
  pages: usize,
  items: Option<(usize, usize)>,
  go: Vec<Go>,
  siblings: usize,
  compact: bool,
  disabled: bool,
  label: Arc<str>,
  summary_text: Option<Arc<str>>,
  previous_label: Arc<str>,
  next_label: Arc<str>,
  page_label: Arc<str>,
  /// The label of the select of how many to a page, and the select, which is built once.
  page_size: Option<(Arc<str>, Arc<Mutex<Option<AnySelect>>>)>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// What a mounted pagination draws in the room it has: fewer pages than the app says, or where it is.
  fit: Fit,
  /// Where a mounted pagination is measured, and how wide it is held while it draws less than it would.
  held: Option<Held>,
}

/// What a mounted pagination draws of its pages in the room it has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Fit {
  /// What the app says.
  #[default]
  All,
  /// Fewer pages on each side of the current one than the app says: this many.
  Siblings(usize),
  /// No pages: where the list is, between previous and next.
  Summary,
}

/// A signal's value is something DevTools can inspect: this one has nothing to show.
impl crate::app::component::DevtoolsInspectable for Fit {}

/// What a mounted pagination's host gives the pagination it builds: what holds the summary and the buttons, what
/// they are in, and the width the first is held at.
#[derive(Clone)]
struct Held {
  room: ElementRef,
  content: ElementRef,
  width: Option<f32>,
}

impl Pagination {
  fn of(current: usize, pages: usize, go: Go) -> Self {
    Self {
      current,
      pages,
      items: None,
      go: vec![go],
      siblings: 1,
      compact: false,
      disabled: false,
      label: super::current_locale().navigation.pagination.clone(),
      summary_text: None,
      previous_label: super::current_locale().navigation.previous_page.clone(),
      next_label: super::current_locale().navigation.next_page.clone(),
      page_label: super::current_locale().navigation.page.clone(),
      page_size: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      fit: Fit::All,
      held: None,
    }
  }

  /// `pages` pages, of which the current one is the signal's: counted from
  /// 1, as a person reads them. A press of another page sets the signal.
  pub fn new(page: Signal<usize>, pages: usize) -> Self {
    let current = page.get();
    Self::of(
      current,
      pages,
      Arc::new(move |to| {
        if page.get_untracked() != to {
          page.set(to);
        }
      }),
    )
  }

  /// The pages of `total` items, `page_size` to a page, of which the current
  /// one is the signal's. The compact form says which of them are shown:
  /// "1–20 of 284".
  pub fn for_items(page: Signal<usize>, total: usize, page_size: usize) -> Self {
    Self::new(page, 0).items(total, page_size)
  }

  /// A pagination whose page the app holds: `current` is the current one,
  /// counted from 1, and `on_change` is told the page each press is for.
  pub fn controlled(current: usize, pages: usize, on_change: impl Fn(usize) + Send + Sync + 'static) -> Self {
    Self::of(current, pages, Arc::new(on_change))
  }

  /// The pages are those of `total` items, `page_size` to a page: in place of
  /// a number of pages, and with what the compact form says of them.
  pub fn items(mut self, total: usize, page_size: usize) -> Self {
    self.pages = if page_size == 0 { 0 } else { total.div_ceil(page_size) };
    self.items = Some((total, page_size));
    self
  }

  /// How many pages are drawn on each side of the current one, between the
  /// first and the last: 1 unless it is said.
  pub fn siblings(mut self, siblings: usize) -> Self {
    self.siblings = siblings;
    self
  }

  /// Whether it is previous and next with where the list is, and no pages:
  /// "1–20 of 284 ‹ ›", for the footer of a table.
  pub fn compact(mut self, compact: bool) -> Self {
    self.compact = compact;
    self
  }

  /// Whether no button of it takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the pagination is called to what reads the tree: "Pagination"
  /// unless it is said.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What it says of where the list is, in place of its own "1–20 of 284"
  /// or "Page 3 of 12": in another language, or of other things. Said in the
  /// compact form, and in the full form too once it is given.
  pub fn summary_text(mut self, text: &str) -> Self {
    self.summary_text = Some(Arc::from(text));
    self
  }

  /// What the button of the page before is called: "Previous page" unless told otherwise.
  pub fn previous_label(mut self, label: &str) -> Self {
    self.previous_label = Arc::from(label);
    self
  }

  /// What the button of the page after is called: "Next page" unless told otherwise.
  pub fn next_label(mut self, label: &str) -> Self {
    self.next_label = Arc::from(label);
    self
  }

  /// The word before the number in the name of a page's button: "Page", of "Page 3", unless told otherwise.
  pub fn page_label(mut self, label: &str) -> Self {
    self.page_label = Arc::from(label);
    self
  }

  /// A select of how many to a page, after `label`, at the start: the
  /// pagination is then as wide as what holds it, with the select at one end
  /// and the pages at the other. The select is the app's, bound to its own
  /// signal; the pagination sizes its trigger.
  pub fn page_size(mut self, label: &str, select: impl Into<AnySelect>) -> Self {
    self.page_size = Some((Arc::from(label), Arc::new(Mutex::new(Some(select.into())))));
    self
  }

  /// Called with the page of each press, after the signal is set.
  pub fn on_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.go.push(Arc::new(handler));
    self
  }

  /// Puts the pagination in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this pagination over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this pagination.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the button Tab would stop on first that takes input (previous, or the first page that is not the
  /// current one) as it is under the pointer, held down or focused from the keyboard, with nothing pointing at it:
  /// for a gallery or a test. The other buttons are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the whole. Its buttons have ids after it: `{id}-previous`,
  /// `{id}-next` and `{id}-page-3`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What a pagination of `pages` pages draws between previous and next
  /// while `current` is the current page, with `siblings` pages on each side
  /// of it: every page while they fit in `2 × siblings + 5`, and that many
  /// things, pages and ellipses, once they do not. A current page past the
  /// ends is taken as the end it is past.
  pub fn pages_shown(current: usize, pages: usize, siblings: usize) -> Vec<PageItem> {
    use PageItem::{Ellipsis, Page};
    // The first, the last, the current with its siblings, and an ellipsis on each side of those.
    let slots = siblings.saturating_mul(2).saturating_add(5);
    if pages <= slots {
      return (1..=pages).map(Page).collect();
    }
    let current = current.clamp(1, pages);
    // Near an end the pages run on to it: all but the ellipsis and the page at the other end.
    let run = slots - 2;
    let mut shown = Vec::with_capacity(slots);
    if current <= siblings + 3 {
      shown.extend((1..=run).map(Page));
      shown.extend([Ellipsis, Page(pages)]);
    } else if current + siblings + 2 >= pages {
      shown.extend([Page(1), Ellipsis]);
      shown.extend((pages + 1 - run..=pages).map(Page));
    } else {
      shown.extend([Page(1), Ellipsis]);
      shown.extend((current - siblings..=current + siblings).map(Page));
      shown.extend([Ellipsis, Page(pages)]);
    }
    shown
  }

  /// The pagination mounted where it is declared: it reads the room it has, before the frame is painted, and
  /// draws what that has room for. Narrower than its buttons, it draws fewer pages on each side of the current
  /// one, down to the first, the current and the last between two ellipses; narrower than those, it says where
  /// the list is between previous and next, as the compact form does. With room again it draws what the app
  /// says. A pagination that is only built draws what the app says in any room.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let theme = current_theme();
    // A pagination the app keeps compact, or one a theme builds its own way, is built as it is.
    if self.compact || theme.renderer::<Pagination>().is_some() {
      return self.build(&theme);
    }
    // The summary and the buttons are the host's, which builds them again as their room changes; the select of
    // how many to a page is built here, once, with the root that holds both.
    let mut pages = self.clone();
    pages.page_size = None;
    let pages = match self.key.as_ref() {
      Some(key) => ctx.mount_keyed::<PaginationHost>(&format!("{key}-pages"), PaginationProps { pagination: pages }),
      None => ctx.mount::<PaginationHost>(PaginationProps { pagination: pages }),
    };
    self.view(&theme).root(pages)
  }

  /// How many things the row of pages has with `siblings` pages on each side of the current one.
  fn slots(&self, siblings: usize) -> usize {
    Self::pages_shown(self.current, self.pages, siblings).len()
  }

  /// Builds the pagination in `theme`, with nothing kept: a still one, which
  /// cannot measure its room, draws the pages the app says in any room, and
  /// its root wraps, so what has no room beside the select of how many to a
  /// page goes on to a line under it. [`mount`](Self::mount) is the one that
  /// measures, and draws fewer pages as its room narrows. Converting a
  /// pagination into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Pagination>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The pagination as a renderer sees it, in `theme`.
  fn view(self, theme: &Theme) -> PaginationView {
    let (siblings, compact) = match self.fit {
      Fit::All => (self.siblings, self.compact),
      Fit::Siblings(siblings) => (siblings.min(self.siblings), self.compact),
      Fit::Summary => (self.siblings, true),
    };
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PaginationState::Compact, compact)
    .state(PaginationState::PageSize, self.page_size.is_some())
    .state(PaginationState::Disabled, self.disabled);
    let current = if self.pages == 0 {
      0
    } else {
      self.current.clamp(1, self.pages)
    };
    let summary = self.summary_text.clone().unwrap_or_else(|| {
      let locale = super::current_locale();
      let fill = crate::core::fill;
      Arc::from(match self.items {
        Some((total, page_size)) if current > 0 => {
          let first = (current - 1) * page_size + 1;
          let last = (current * page_size).min(total);
          fill(
            &locale.navigation.range_of,
            &[
              ("first", &first.to_string()),
              ("last", &last.to_string()),
              ("total", &total.to_string()),
            ],
          )
        }
        Some((total, _)) => fill(&locale.common.place_of, &[("at", "0"), ("total", &total.to_string())]),
        None => fill(
          &locale.navigation.page_of,
          &[("at", &current.to_string()), ("total", &self.pages.to_string())],
        ),
      })
    });
    PaginationView {
      current,
      pages: self.pages,
      shown: Self::pages_shown(current, self.pages, siblings),
      compact,
      disabled: self.disabled,
      label: self.label,
      summary,
      summarized: compact || self.summary_text.is_some(),
      recipe: theme.resolve::<Pagination>(&case, Some(&self.rules)),
      current_recipe: theme
        .resolve::<Pagination>(&case.clone().state(PaginationState::Current, true), Some(&self.rules)),
      previous_label: self.previous_label,
      next_label: self.next_label,
      page_label: self.page_label,
      forced: self.forced,
      page_size: self
        .page_size
        .and_then(|(label, select)| Some((label, select.lock().take()?))),
      go: self.go,
      held: self.held,
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Pagination> for Element {
  fn from(pagination: Pagination) -> Self {
    pagination.build(&current_theme())
  }
}

#[derive(Clone)]
struct PaginationProps {
  pagination: Pagination,
}

/// A pagination is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for PaginationProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for PaginationProps {}

/// What a mounted pagination has measured, and what it chooses by.
#[derive(Default)]
struct Measured {
  /// How wide the room of the summary and the buttons was last laid out.
  room: Option<f32>,
  /// How wide the summary and the buttons are with everything the app says drawn.
  full: Option<f32>,
  /// What each thing of the row of pages takes: a button and the room after it.
  step: f32,
  /// What it can draw in place of all of it, the widest first: so many siblings, with so many things fewer in
  /// the row.
  fewer: Vec<(usize, usize)>,
}

impl Measured {
  /// What the room has room for.
  fn fit(&self) -> Option<Fit> {
    let (room, full) = (self.room?, self.full?);
    let fits = |fewer: usize| full - fewer as f32 * self.step <= room + 0.5;
    Some(if fits(0) {
      Fit::All
    } else {
      self
        .fewer
        .iter()
        .find(|(_, fewer)| fits(*fewer))
        .map_or(Fit::Summary, |(siblings, _)| Fit::Siblings(*siblings))
    })
  }
}

/// The summary and the buttons of a mounted pagination: it measures the room they have, before the frame is
/// painted, and builds them with what that has room for.
struct PaginationHost {
  room: ElementRef,
  content: ElementRef,
  fit: Signal<Fit>,
  measured: Arc<Mutex<Measured>>,
}

impl Component for PaginationHost {
  type Props = PaginationProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (room, content, fit) = (ElementRef::new(), ElementRef::new(), ctx.signal(Fit::All));
    let measured = Arc::new(Mutex::new(Measured::default()));
    // Written only where the room crosses what a form needs: a panel dragged within one renders nothing.
    let settle = {
      let watch = fit.clone();
      move |measured: &Measured| {
        if let Some(now) = measured.fit()
          && watch.get_untracked() != now
        {
          watch.set(now);
        }
      }
    };
    {
      let (held, settle) = (measured.clone(), settle.clone());
      room.observe_rect_before_paint(move |rect| {
        let mut measured = held.lock();
        measured.room = Some(rect.width);
        settle(&measured);
      });
    }
    {
      let (held, watch) = (measured.clone(), fit.clone());
      content.observe_rect_before_paint(move |rect| {
        let mut measured = held.lock();
        // With all of it drawn, what is in the room says how wide all of it is.
        if watch.get_untracked() == Fit::All {
          measured.full = Some(rect.width);
        }
        settle(&measured);
      });
    }
    Self {
      room,
      content,
      fit,
      measured,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut pagination = ctx.props::<PaginationProps>().pagination.clone();
    let theme = current_theme();
    let fit = self.fit.get();
    let full = {
      let mut measured = self.measured.lock();
      let recipe = theme.resolve::<Pagination>(&Case::new(), Some(&pagination.rules));
      let side = match recipe
        .part(PaginationPart::Ellipsis)
        .rest()
        .width
        .and_then(|width| width.resolve(&theme))
      {
        Some(Dimension::Px(px)) => px,
        _ => 0.0,
      };
      let gap = recipe
        .part(PaginationPart::Nav)
        .rest()
        .gap
        .map(|gap| gap.resolve(&theme.spacing(), 0.0))
        .unwrap_or(0.0);
      measured.step = side + gap;
      let all = pagination.slots(pagination.siblings);
      measured.fewer = (0..pagination.siblings)
        .rev()
        .map(|siblings| (siblings, all - pagination.slots(siblings)))
        .filter(|(_, fewer)| *fewer > 0)
        .collect();
      measured.full
    };
    pagination.fit = fit;
    pagination.held = Some(Held {
      room: self.room.clone(),
      content: self.content.clone(),
      // Drawing less than all of it, it is held as wide as all of it where there is room, so that the room says
      // when all of it fits again.
      width: full.filter(|_| fit != Fit::All),
    });
    pagination.view(&theme).pages()
  }
}

/// A pagination as a renderer sees it: where the list is, what is drawn of
/// its pages, the style of its parts, and its buttons ready to be laid out.
pub struct PaginationView {
  /// The current page, counted from 1; 0 where there are no pages.
  pub current: usize,
  pub pages: usize,
  /// What is drawn between previous and next, in order.
  pub shown: Vec<PageItem>,
  pub compact: bool,
  pub disabled: bool,
  /// What the whole is called.
  pub label: Arc<str>,
  /// What is said of where the list is.
  pub summary: Arc<str>,
  /// Whether the summary is drawn.
  pub summarized: bool,
  /// The style of each part of this pagination.
  pub recipe: ResolvedRecipe,
  /// The style of the parts of the current page's button.
  current_recipe: ResolvedRecipe,
  /// What the buttons of the page before and after are called, and the word before a page's number.
  pub previous_label: Arc<str>,
  pub next_label: Arc<str>,
  pub page_label: Arc<str>,
  forced: Option<Interaction>,
  page_size: Option<(Arc<str>, AnySelect)>,
  go: Vec<Go>,
  held: Option<Held>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PaginationView {
  /// The style of a part, as this pagination is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A button of the pagination: in the style the pagination gives it, its
  /// id after the pagination's, and a press that goes to `to`.
  fn item(&self, button: Button, name: &str, to: Option<usize>, current: bool) -> Button {
    let recipe = if current { &self.current_recipe } else { &self.recipe };
    let rules = recipe.rules_for(PaginationPart::Item, ButtonPart::Root);
    let forced = self.forced.filter(|_| self.entry().as_deref() == Some(name));
    let mut button = button
      .rules(&rules)
      .disabled(self.disabled || (to.is_none() && !current))
      .force(forced)
      .key(name.to_owned());
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-{name}"));
    }
    if let Some(to) = to {
      let go = self.go.clone();
      button = button.on_click(move |_| {
        for go in &go {
          go(to);
        }
      });
    }
    button
  }

  /// The name, in the view's ids and keys, of the button Tab would stop on first that takes input: `previous`, or
  /// `page-3` for the first page shown that is not the current one, or `next`. None while it is disabled.
  pub fn entry(&self) -> Option<String> {
    if self.disabled {
      return None;
    }
    if self.current > 1 {
      return Some("previous".to_owned());
    }
    let page = (!self.compact)
      .then(|| {
        self.shown.iter().find_map(|item| match item {
          PageItem::Page(page) if *page != self.current => Some(format!("page-{page}")),
          _ => None,
        })
      })
      .flatten();
    page.or_else(|| (self.current < self.pages).then(|| "next".to_owned()))
  }

  /// The button that goes to the page before, disabled on the first.
  pub fn previous(&self) -> Element {
    self.step(
      icons::CHEVRON_LEFT,
      &self.previous_label,
      "previous",
      self.current.checked_sub(1).filter(|to| *to >= 1),
    )
  }

  /// The button that goes to the page after, disabled on the last.
  pub fn next(&self) -> Element {
    self.step(
      icons::CHEVRON_RIGHT,
      &self.next_label,
      "next",
      Some(self.current + 1).filter(|to| *to <= self.pages),
    )
  }

  fn step(&self, icon: &str, label: &str, name: &str, to: Option<usize>) -> Element {
    // Quiet beside the pages; with no pages beside them, the compact form draws them with an edge.
    let variant = if self.compact {
      ButtonVariant::Outline
    } else {
      ButtonVariant::Ghost
    };
    let button = Button::new(label).variant(variant).icon(icon).icon_only(true);
    self.item(button, name, to, false).build(self.theme())
  }

  /// The button of the page `page`, counted from 1: with an edge where it is the current one, which a press
  /// goes nowhere from. It is called "Page 3" to what reads the tree, after its [`page_label`](Pagination::page_label).
  pub fn page(&self, page: usize) -> Element {
    let current = page == self.current;
    let variant = if current {
      ButtonVariant::Outline
    } else {
      ButtonVariant::Ghost
    };
    let button = Button::new(&page.to_string()).variant(variant);
    let to = (!current).then_some(page);
    let name: Arc<str> = Arc::from(format!("{} {page}", self.page_label));
    self
      .item(button, &format!("page-{page}"), to, current)
      .build(self.theme())
      .map_node(move |mut node| {
        node.describe(Arc::from("a11y_name"), name);
        if current {
          node.describe(Arc::from("a11y_current"), Arc::from("page"));
        }
        node
      })
  }

  /// What stands where pages are left out: the `index`th of the row's, for its key.
  pub fn ellipsis(&self, index: usize) -> Element {
    Row::new()
      .part(&self.part(PaginationPart::Ellipsis))
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .key(format!("ellipsis-{index}"))
      .child(glyph(
        &self.part(PaginationPart::EllipsisIcon),
        Name::new(icons::MORE_HORIZONTAL),
        false,
      ))
      .into()
  }

  /// The text of where the list is.
  pub fn summary(&self) -> Element {
    let part = self.part(PaginationPart::Summary);
    Row::new().part(&part).child(part.text(&self.summary).nowrap()).into()
  }

  /// The row of buttons: previous, the pages unless the pagination is compact, and next. Each is its own stop
  /// of Tab, and the arrow keys move between them.
  pub fn nav(&self) -> Row {
    let mut nav = Row::new()
      .part(&self.part(PaginationPart::Nav))
      .align_items(Alignment::Center)
      // The ring of the focus is outside the button it is on.
      .overflow_visible()
      .focus_group(FocusGroup::horizontal())
      .child(self.previous());
    if !self.compact {
      let mut ellipses = 0;
      for item in &self.shown {
        nav = nav.child(match *item {
          PageItem::Page(page) => self.page(page),
          PageItem::Ellipsis => {
            ellipses += 1;
            self.ellipsis(ellipses)
          }
        });
      }
    }
    nav.child(self.next())
  }

  /// The label of the select of how many to a page, and the select called by it, if the pagination has one.
  pub fn page_size(&mut self) -> Option<[Element; 2]> {
    let (label, select) = self.page_size.take()?;
    let text = self.part(PaginationPart::PageSizeLabel);
    let rules = self.recipe.rules_for(PaginationPart::PageSize, SelectPart::Trigger);
    Some([
      Row::new().part(&text).child(text.text(&label).nowrap()).into(),
      select.build(
        self.theme(),
        Dress {
          label: Some(&label),
          rules: Some(&rules),
          ..Dress::default()
        },
      ),
    ])
  }

  /// Where the list is, if the pagination says so, and the row of buttons, the root's gap apart: one thing of the
  /// root, no wider than the root has room for. A mounted pagination measures its room.
  pub fn pages(&self) -> Row {
    let gap = self
      .part(PaginationPart::Root)
      .rest()
      .gap
      .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    let mut content = Row::new()
      .spacing(gap)
      .align_items(Alignment::Center)
      .overflow_visible()
      .flex_shrink(0.0);
    if self.summarized {
      content = content.child(self.summary());
    }
    content = content.child(self.nav());
    let mut room = Row::new().overflow_visible().max_width(Dimension::Pct(100.0));
    if let Some(held) = &self.held {
      content = content.ref_element(held.content.clone());
      room = room
        .ref_element(held.room.clone())
        .flex_shrink(1.0)
        .min_width(Dimension::Px(0.0));
      if let Some(width) = held.width {
        room = room.width(Dimension::Px(width));
      }
    }
    room.child(content)
  }

  /// How the default renderer builds a pagination: the select of how many to a page, if it has one, and then at
  /// the other end where the list is, if it says so, and the row of buttons.
  pub fn render(self) -> Element {
    let pages = self.pages().into();
    self.root(pages)
  }

  /// The root with `pages` in it, after the select of how many to a page if the pagination has one. What has no
  /// room beside the select goes on to a line under it, and the whole is no wider than its room.
  fn root(mut self, pages: Element) -> Element {
    let mut root = Row::new()
      .part(&self.part(PaginationPart::Root))
      .wrap()
      .max_width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .overflow_visible()
      .describe("a11y_role", "navigation")
      .describe("a11y_name", self.label.clone());
    if let Some([label, select]) = self.page_size() {
      // What is left of the width is between the select and the rest.
      let gap = self
        .part(PaginationPart::Root)
        .rest()
        .gap
        .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
        .unwrap_or(0.0);
      root = root.justify(Justify::SpaceBetween).child(
        Row::new()
          .spacing(gap)
          .align_items(Alignment::Center)
          .overflow_visible()
          .child(label)
          .child(select),
      );
    }
    root = root.child(pages);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for Pagination {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Pagination {
  const NAME: &'static str = "pagination";
  type Renderer = PaginationRenderer;

  fn parts() -> crate::ui::Declared {
    PaginationPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PaginationState::declared()
  }

  /// The design's pagination (`pencil/lurq-design-system.pen`, Pagination
  /// and Pagination/Compact): buttons of 36, 4 apart, quiet, with the
  /// current page's edged; the ellipsis a box of 36 with its glyph of 16 in
  /// the muted color. The compact form has edged buttons of 30 for previous
  /// and next, with where the list is 12 before them in the body's type at
  /// 13, and a select of 72 by 30 after its label, muted, at the other end.
  fn recipe() -> Recipe {
    use PaginationPart::{Ellipsis, EllipsisIcon, Item, Nav, PageSize, PageSizeLabel, Root, Summary};
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| s.gap(12.0).foreground(Foreground))
      .base(Nav, |s| s.gap(4.0))
      // As wide as it is high for a number of a few digits, and wider for a longer one.
      .base(Item, |s| s.min_width(ControlSize::Md).padding_x(6.0))
      .base(Ellipsis, |s| s.square(ControlSize::Md))
      .base(EllipsisIcon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Summary, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .base(PageSizeLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(PageSize, |s| s.width(72.0).height(30.0).font_size(13.0))
      .state(PaginationState::Compact, Item, |s| s.min_width(30.0).square(30.0))
      .state(PaginationState::PageSize, Root, |s| s.width(Length::Fill))
  }
}
