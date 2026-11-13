//! A pagination: the pages of a list, one of them the current one, and a press that goes to another.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonVariant},
  icons, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{Select, SelectPart},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{ControlSize, IconSize, PaletteColor, Theme, TypographyStyle},
  components::Row,
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, FocusGroup, NodeUpdate},
};

/// The parts of a pagination a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaginationPart {
  /// The whole: what is before the buttons and the buttons, side by side.
  Root,
  /// The row of buttons, previous, the pages and next, that the arrow keys move along.
  Nav,
  /// What the pagination says of each of its buttons: laid over the theme's recipe for a button's root.
  Item,
  /// What it says of the button of the current page, over [`PaginationPart::Item`].
  Current,
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
  Current => "current",
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
}

names!(PaginationState {
  Compact => "compact",
  PageSize => "page_size",
  Disabled => "disabled",
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
pub struct Pagination {
  current: usize,
  pages: usize,
  items: Option<(usize, usize)>,
  go: Vec<Go>,
  siblings: usize,
  compact: bool,
  disabled: bool,
  label: Arc<str>,
  summary: Option<Arc<str>>,
  page_size: Option<(Arc<str>, Select)>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
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
      label: Arc::from("Pagination"),
      summary: None,
      page_size: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
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
  /// counted from 1, and `go` is told the page each press is for.
  pub fn controlled(current: usize, pages: usize, go: impl Fn(usize) + Send + Sync + 'static) -> Self {
    Self::of(current, pages, Arc::new(go))
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
  pub fn summary(mut self, summary: &str) -> Self {
    self.summary = Some(Arc::from(summary));
    self
  }

  /// A select of how many to a page, after `label`, at the start: the
  /// pagination is then as wide as what holds it, with the select at one end
  /// and the pages at the other. The select is the app's, bound to its own
  /// signal; the pagination sizes its trigger.
  pub fn page_size(mut self, label: &str, select: Select) -> Self {
    self.page_size = Some((Arc::from(label), select));
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

  /// Builds the pagination in `theme`. Converting a pagination into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PaginationState::Compact, self.compact)
    .state(PaginationState::PageSize, self.page_size.is_some())
    .state(PaginationState::Disabled, self.disabled);
    let current = if self.pages == 0 {
      0
    } else {
      self.current.clamp(1, self.pages)
    };
    let summary = self.summary.clone().unwrap_or_else(|| {
      Arc::from(match self.items {
        Some((total, page_size)) if current > 0 => {
          let first = (current - 1) * page_size + 1;
          format!("{first}–{} of {total}", (current * page_size).min(total))
        }
        Some((total, _)) => format!("0 of {total}"),
        None => format!("Page {current} of {}", self.pages),
      })
    });
    let view = PaginationView {
      current,
      pages: self.pages,
      shown: Self::pages_shown(current, self.pages, self.siblings),
      compact: self.compact,
      disabled: self.disabled,
      label: self.label,
      summary,
      summarized: self.compact || self.summary.is_some(),
      recipe: theme.resolve::<Pagination>(&case, Some(&self.rules)),
      page_size: self.page_size,
      go: self.go,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Pagination>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Pagination> for Element {
  fn from(pagination: Pagination) -> Self {
    pagination.build(&current_theme())
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
  page_size: Option<(Arc<str>, Select)>,
  go: Vec<Go>,
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
    let mut rules = self.recipe.rules_for(PaginationPart::Item, ButtonPart::Root);
    if current {
      rules = rules.extend(&self.recipe.rules_for(PaginationPart::Current, ButtonPart::Root));
    }
    let mut button = button
      .rules(&rules)
      .disabled(self.disabled || (to.is_none() && !current))
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

  /// The button that goes to the page before, disabled on the first.
  pub fn previous(&self) -> Element {
    self.step(
      icons::CHEVRON_LEFT,
      "Previous page",
      "previous",
      self.current.checked_sub(1).filter(|to| *to >= 1),
    )
  }

  /// The button that goes to the page after, disabled on the last.
  pub fn next(&self) -> Element {
    self.step(
      icons::CHEVRON_RIGHT,
      "Next page",
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
  /// goes nowhere from. It is called "Page 3" to what reads the tree.
  pub fn page(&self, page: usize) -> Element {
    let current = page == self.current;
    let variant = if current {
      ButtonVariant::Outline
    } else {
      ButtonVariant::Ghost
    };
    let button = Button::new(&page.to_string()).variant(variant);
    let to = (!current).then_some(page);
    let name: Arc<str> = Arc::from(format!("Page {page}"));
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
      select.label(&label).rules(&rules).build(self.theme()),
    ])
  }

  /// How the default renderer builds a pagination: the select of how many to a page, if it has one, and then at
  /// the other end where the list is, if it says so, and the row of buttons.
  pub fn render(mut self) -> Element {
    let mut root = Row::new()
      .part(&self.part(PaginationPart::Root))
      .align_items(Alignment::Center)
      .overflow_visible()
      .describe("a11y_role", "navigation")
      .describe("a11y_name", self.label.clone());
    if let Some([label, select]) = self.page_size() {
      // What is left of the width is between the select and the rest.
      root = root.child(label).child(select).child(Row::new().flex(1.0));
    }
    if self.summarized {
      root = root.child(self.summary());
    }
    root = root.child(self.nav());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Pagination {
  const NAME: &'static str = "pagination";
  type Renderer = PaginationRenderer;

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
