//! A highlight: a text with marks on ranges of it, the matches of a search,
//! one of them the current one.
//!
//! The ranges are the app's: [`find_matches`] finds those of a query under a
//! find bar's options, for an app that searches a plain text.

use std::{
  ops::Range,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use parking_lot::Mutex;

use super::{
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, Theme, TypographyStyle},
  },
  components::TextHighlight,
  core::ElementRef,
  layout::{
    quad::RichTextSpan,
    text_style::{FontWeight, TextStyle},
  },
  node::{BackgroundColor, Element, Node, NodeUpdate, TextColor, border::Border},
};

/// The parts of a highlight a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HighlightPart {
  /// The text: its type and its color.
  Root,
  /// A mark on a range: its fill, its radius, how far it reaches past its
  /// words, and the weight of its words.
  Mark,
  /// The current mark, in place of `Mark`: its fill, its edge, its words' weight.
  Current,
}

names!(HighlightPart {
  Root => "root",
  Mark => "mark",
  Current => "current",
});

/// What builds a highlight's nodes: a theme's in place of the default one.
pub type HighlightRenderer = Arc<dyn Fn(HighlightView) -> Element + Send + Sync>;

/// A highlight: a text with marks on ranges of it, as a search marks its
/// matches, one of them the current one.
///
/// ```
/// use lurq::ui::{FindOptions, Highlight, find_matches};
///
/// let text = "the cat and the hat";
/// let found = find_matches(text, "the", FindOptions::default()).unwrap_or_default();
/// let shown = Highlight::new(text).marks(found).current(Some(1));
/// ```
///
/// The ranges are in bytes of the text, and the current one is an index into
/// them; the marks follow their words as the text wraps, and a mark over the
/// end of a line goes on at the start of the next. A mark's words are of the
/// weight the board gives them, 500, so a mark in a sentence stands out as
/// the board draws it ("Results for **backdrop blur** in docs").
///
/// A find in a text keeps the text's weight ([`keep_weight`](Self::keep_weight)):
/// heavier words are wider, so the text would wrap again under the reader's
/// eyes as each letter of the query is typed and the marks move. With it the
/// text is shaped and laid out as without the marks.
///
/// Mounted ([`mount`](Self::mount)), the highlight brings its current mark
/// into view each time it changes: as a find bar goes to the next match and
/// as what is searched for changes.
#[derive(Clone)]
pub struct Highlight {
  text: Arc<str>,
  marks: Vec<Range<usize>>,
  current: Option<usize>,
  keep_weight: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Highlight {
  /// `text`, with no marks yet.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      marks: Vec::new(),
      current: None,
      keep_weight: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A mark on bytes `range` of the text, after the ones it has.
  pub fn mark(mut self, range: Range<usize>) -> Self {
    self.marks.push(range);
    self
  }

  /// Marks on ranges of the text, after the ones it has: the matches of a search.
  pub fn marks(mut self, ranges: impl IntoIterator<Item = Range<usize>>) -> Self {
    self.marks.extend(ranges);
    self
  }

  /// Which of the marks is the current one, by its index: drawn stronger,
  /// and brought into view where the highlight is mounted. `None` for none.
  pub fn current(mut self, current: Option<usize>) -> Self {
    self.current = current;
    self
  }

  /// Whether the marks' words keep the text's weight, whatever the recipe
  /// gives a mark: the find in a text, which must not wrap again as the
  /// query is typed. The board's weight, 500, by default.
  pub fn keep_weight(mut self, keep: bool) -> Self {
    self.keep_weight = keep;
    self
  }

  /// Puts the highlight in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this highlight over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this highlight.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the text.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The current mark's range, where there is one.
  fn current_range(&self) -> Option<Range<usize>> {
    self.current.and_then(|index| self.marks.get(index).cloned())
  }

  /// Puts the highlight in the tree of `ctx`'s component, in the theme of
  /// what is being rendered: as [`build`](Self::build) does, and it brings
  /// the current mark into view each time it changes, not as it is first shown.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    // Keyed, it keeps its host where what is before it comes and goes.
    let props = HighlightProps { highlight: self };
    match key {
      Some(key) => ctx.mount_keyed::<HighlightHost>(&key, props).key(key),
      None => ctx.mount::<HighlightHost>(props),
    }
  }

  /// Builds the highlight in `theme`. Converting a highlight into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.view(theme, None)
  }

  /// The fills of the marks in `theme`, the current one in its own style, for a text drawn in a style of its own
  /// (a cell of a data grid): the find in it keeps the text's weight.
  pub(crate) fn fills_in(self, theme: &Theme) -> Vec<TextHighlight> {
    let view = HighlightView {
      text: self.text,
      marks: self.marks,
      current: self.current,
      keep_weight: true,
      recipe: theme.resolve::<Highlight>(&Case::new().breakpoint(super::current_breakpoint()), Some(&self.rules)),
      text_ref: None,
      id: None,
      key: None,
    };
    view.fills()
  }

  fn view(self, theme: &Theme, text_ref: Option<ElementRef>) -> Element {
    let mut case = Case::new().breakpoint(super::current_breakpoint());
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = HighlightView {
      text: self.text,
      marks: self.marks,
      current: self.current,
      keep_weight: self.keep_weight,
      recipe: theme
        .resolve::<Highlight>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      text_ref,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Highlight>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Highlight> for Element {
  fn from(highlight: Highlight) -> Self {
    highlight.build(&current_theme())
  }
}

#[derive(Clone)]
struct HighlightProps {
  highlight: Highlight,
}

/// The highlight is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for HighlightProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for HighlightProps {}

/// What a mounted highlight keeps: the current mark it last brought into
/// view, whether it was rendered, and its text, which it scrolls in.
struct HighlightHost {
  shown: Mutex<Option<Range<usize>>>,
  rendered: AtomicBool,
  text: ElementRef,
}

impl Component for HighlightHost {
  type Props = HighlightProps;

  fn create(_: &mut Ctx) -> Self {
    Self {
      shown: Mutex::new(None),
      rendered: AtomicBool::new(false),
      text: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let HighlightProps { highlight } = ctx.props::<HighlightProps>().clone();
    let current = highlight.current_range();
    let first = !self.rendered.swap(true, Ordering::Relaxed);
    let changed = {
      let mut shown = self.shown.lock();
      let changed = *shown != current;
      shown.clone_from(&current);
      changed
    };
    // The current mark changed: it is brought into view. As the text is first shown, the page stays where it is.
    if changed
      && !first
      && let Some(range) = current
    {
      ctx.scroll_range_into_view(&self.text, range);
    }
    highlight.view(&current_theme(), Some(self.text.clone()))
  }
}

/// A highlight as a renderer sees it: its text, its marks, which is the
/// current one, and the style of its parts.
pub struct HighlightView {
  pub text: Arc<str>,
  pub marks: Vec<Range<usize>>,
  pub current: Option<usize>,
  /// Whether the marks' words keep the text's weight.
  pub keep_weight: bool,
  /// The style of each part of this highlight.
  pub recipe: ResolvedRecipe,
  text_ref: Option<ElementRef>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl HighlightView {
  /// The style of a part, as this highlight is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The fill behind `range` in the style of `part`: its color, its radius,
  /// how far it reaches past its words, and its edge.
  pub fn fill(&self, range: Range<usize>, part: impl Into<Name>) -> Option<TextHighlight> {
    fill_of(&self.part(part), range)
  }

  /// Every mark's fill, the current one in its own style.
  pub fn fills(&self) -> Vec<TextHighlight> {
    self
      .marks
      .iter()
      .enumerate()
      .filter_map(|(index, range)| {
        let part = if Some(index) == self.current {
          HighlightPart::Current
        } else {
          HighlightPart::Mark
        };
        self.fill(range.clone(), part)
      })
      .collect()
  }

  /// The weight of the words of the mark at `index`, where it is another
  /// than the text's: the part's, unless the marks keep the text's weight.
  fn weight_of(&self, index: usize, text: FontWeight) -> Option<FontWeight> {
    if self.keep_weight {
      return None;
    }
    let part = if Some(index) == self.current {
      HighlightPart::Current
    } else {
      HighlightPart::Mark
    };
    self.part(part).rest().text.weight.filter(|weight| *weight != text)
  }

  /// The text in runs: the words of each mark in its weight, the rest in the
  /// text's style. `None` where no mark's weight is another than the text's.
  pub fn runs(&self, style: &TextStyle) -> Option<Vec<RichTextSpan>> {
    let mut marks: Vec<(Range<usize>, FontWeight)> = self
      .marks
      .iter()
      .enumerate()
      .filter_map(|(index, range)| Some((range.clone(), self.weight_of(index, style.weight)?)))
      .filter(|(range, _)| {
        range.start < range.end
          && range.end <= self.text.len()
          && self.text.is_char_boundary(range.start)
          && self.text.is_char_boundary(range.end)
      })
      .collect();
    if marks.is_empty() {
      return None;
    }
    marks.sort_by_key(|(range, _)| range.start);
    let run = |text: &str, weight: FontWeight| RichTextSpan {
      text: text.to_owned(),
      style: TextStyle {
        weight,
        ..style.clone()
      },
    };
    let mut runs = Vec::new();
    let mut at = 0;
    for (range, weight) in marks {
      // Marks that overlap: the later one starts where the earlier ended.
      let start = range.start.max(at);
      if start >= range.end {
        continue;
      }
      if at < start {
        runs.push(run(&self.text[at..start], style.weight));
      }
      runs.push(run(&self.text[start..range.end], weight));
      at = range.end;
    }
    if at < self.text.len() {
      runs.push(run(&self.text[at..], style.weight));
    }
    Some(runs)
  }

  /// How the default renderer builds a highlight: the text, in the style of
  /// its root, with the marks' fills behind it; where a mark's words are of
  /// another weight, the text in runs, the fills behind the runs as they are
  /// laid out.
  pub fn render(self) -> Element {
    let part = self.part(HighlightPart::Root);
    let style = part.rest().text.resolve(self.theme());
    if let Some(runs) = self.runs(&style) {
      let mut node = Node::rich_text(runs);
      part.apply(&mut node);
      node.set_text_highlights(self.fills());
      if let Some(reference) = &self.text_ref {
        NodeUpdate::ref_element(&mut node, reference.clone());
      }
      if let Some(id) = &self.id {
        node = node.id(id.clone());
      }
      let mut element = Element::from_node(node);
      if let Some(key) = &self.key {
        element = element.key(key.clone());
      }
      return element;
    }
    let mut text = part.text(&self.text).part(&part).highlights(self.fills());
    if let Some(reference) = &self.text_ref {
      text = text.ref_element(reference.clone());
    }
    if let Some(id) = &self.id {
      text = text.id(id.clone());
    }
    let mut element: Element = text.into();
    if let Some(key) = &self.key {
      element = element.key(key.clone());
    }
    element
  }
}

/// The fill behind `range` of a text in the style of a part: its fill, its
/// radius, how far it reaches past its words (its padding), and its edge.
/// `None` for a part with no fill. What a highlight's mark is, and a
/// document's code and keys in a sentence.
pub(crate) fn fill_of(part: &ResolvedPart, range: Range<usize>) -> Option<TextHighlight> {
  let theme = part.theme();
  let style = part.rest();
  let as_text = |color: BackgroundColor| match color {
    BackgroundColor::Color(color) => TextColor::Color(color),
    BackgroundColor::Palette(role) => TextColor::Palette(role),
    BackgroundColor::CurrentForeground => TextColor::Palette(PaletteColor::Foreground),
  };
  let color = as_text(style.background.clone()?);
  let radius = style
    .radius
    .map(|radius| radius.resolve(&theme.radii()).top_left)
    .unwrap_or(0.0);
  let spacing = theme.spacing();
  let [top, end, _, start] = style.paddings();
  let pad = |side: Option<crate::node::SpacingValue>| side.map_or(0.0, |side| side.resolve(&spacing, 0.0));
  let mut fill = TextHighlight::new(range, color)
    .radius(radius)
    .padding(pad(start).max(pad(end)), pad(top));
  if let Some(edge) = &style.border {
    let width = edge.width.resolve(&theme.border_sizes());
    fill = fill.border(width, as_text(edge.color.clone()));
  }
  Some(fill)
}

impl crate::ui::Styled for Highlight {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Highlight {
  const NAME: &'static str = "highlight";
  type Renderer = HighlightRenderer;

  fn parts() -> crate::ui::Declared {
    HighlightPart::declared()
  }

  /// The design's highlight (`pencil/lurq-design-system.pen`, Highlight):
  /// a text of 14 in the foreground, a mark on the muted warning fill of the
  /// small radius 2, reaching 3 past its words and 1 above and below, its
  /// words 500. The board draws no current mark: it is the mark with an edge
  /// of 2 in the warning color, which reads in both appearances where a fill
  /// of the warning color would not under the text's own color.
  fn recipe() -> Recipe {
    use HighlightPart::{Current, Mark, Root};
    use PaletteColor::{Foreground, Warning, WarningMuted};
    let mark = |s: super::PartStyle| {
      s.background(WarningMuted)
        .radius(2.0)
        .padding_x(3.0)
        .padding_y(1.0)
        .weight(FontWeight::Medium)
    };
    Recipe::new()
      .base(Root, |s| s.typography(TypographyStyle::Body).foreground(Foreground))
      .base(Mark, mark)
      .base(Current, |s| mark(s).border(Border::inside(BorderSize::Md, Warning)))
  }
}

/// How a search matches its query: a find bar's options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FindOptions {
  /// The case of the query's letters counts: "The" does not match "the".
  pub match_case: bool,
  /// A match is a whole word: "the" does not match in "other".
  pub whole_word: bool,
  /// The query is a regular expression (with the `regex` feature).
  pub regex: bool,
}

/// Why [`find_matches`] found nothing to search for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindError {
  /// The query is not a regular expression: what is wrong with it.
  InvalidPattern(String),
  /// A regular expression was asked for, and lurq was built without its `regex` feature.
  RegexUnavailable,
}

impl std::fmt::Display for FindError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidPattern(why) => write!(formatter, "not a regular expression: {why}"),
      Self::RegexUnavailable => formatter.write_str("lurq was built without the `regex` feature"),
    }
  }
}

impl std::error::Error for FindError {}

/// The ranges of `text` that match `query` under `options`, in bytes, in
/// order and apart: what a find bar's app marks with a [`Highlight`] and
/// counts. An empty query matches nothing. Without `match_case` letters
/// match whatever their case; with `whole_word` a match has no letter, digit
/// or `_` right before or after it; with `regex` the query is a regular
/// expression, and a match of no length is passed over.
///
/// ```
/// use lurq::ui::{FindOptions, find_matches};
///
/// let text = "The other theme";
/// assert_eq!(find_matches(text, "the", FindOptions::default()).unwrap(), [0..3, 5..8, 10..13]);
/// let whole = FindOptions { whole_word: true, ..FindOptions::default() };
/// assert_eq!(find_matches(text, "the", whole).unwrap(), [0..3]);
/// let case = FindOptions { match_case: true, ..FindOptions::default() };
/// assert_eq!(find_matches(text, "the", case).unwrap(), [5..8, 10..13]);
/// ```
pub fn find_matches(text: &str, query: &str, options: FindOptions) -> Result<Vec<Range<usize>>, FindError> {
  if query.is_empty() {
    return Ok(Vec::new());
  }
  if options.regex {
    return find_pattern(text, query, options);
  }
  let mut found = Vec::new();
  let mut at = 0;
  while at < text.len() {
    match match_at(text, at, query, options.match_case) {
      Some(end) if !options.whole_word || whole_word(text, at, end) => {
        found.push(at..end);
        at = end;
      }
      _ => at += text[at..].chars().next().map_or(1, char::len_utf8),
    }
  }
  Ok(found)
}

/// Where a match of `query` that starts at byte `at` of `text` ends: each of
/// its characters matched, with its case or without.
fn match_at(text: &str, at: usize, query: &str, match_case: bool) -> Option<usize> {
  let mut rest = text[at..].char_indices();
  let mut end = at;
  for wanted in query.chars() {
    let (offset, found) = rest.next()?;
    let same = if match_case {
      found == wanted
    } else {
      found == wanted || found.to_lowercase().eq(wanted.to_lowercase())
    };
    if !same {
      return None;
    }
    end = at + offset + found.len_utf8();
  }
  Some(end)
}

/// Whether nothing of a word stands right before `start` or right after `end`.
fn whole_word(text: &str, start: usize, end: usize) -> bool {
  let wordy = |c: char| c.is_alphanumeric() || c == '_';
  !text[..start].chars().next_back().is_some_and(wordy) && !text[end..].chars().next().is_some_and(wordy)
}

#[cfg(feature = "regex")]
fn find_pattern(text: &str, query: &str, options: FindOptions) -> Result<Vec<Range<usize>>, FindError> {
  let pattern = regex::RegexBuilder::new(query)
    .case_insensitive(!options.match_case)
    .build()
    .map_err(|error| FindError::InvalidPattern(error.to_string()))?;
  Ok(
    pattern
      .find_iter(text)
      .filter(|found| !found.is_empty())
      .filter(|found| !options.whole_word || whole_word(text, found.start(), found.end()))
      .map(|found| found.range())
      .collect(),
  )
}

#[cfg(not(feature = "regex"))]
fn find_pattern(_: &str, _: &str, _: FindOptions) -> Result<Vec<Range<usize>>, FindError> {
  Err(FindError::RegexUnavailable)
}
