//! Link triggers of a rich text editor: a string (`[[`, `KON-`) that, typed, opens a list of what can be linked,
//! narrowed by what is typed after it, whose choice puts a link in place of the trigger and what was typed.

use std::sync::Arc;

use super::recipe::Name;
use crate::components::{SpanStyle, StyledSpan, StyledText, WikiLink};

/// What a [`LinkTrigger`] offers: a page, a task, anything an app links to.
///
/// ```
/// use lurq::ui::{LinkOffer, icons};
///
/// let page = LinkOffer::new("Roadmap 2027", "/roadmap").icon(icons::FILE).description("Updated today");
/// // The words the link shows, in place of the label.
/// let task = LinkOffer::new("KON-12 Fix the login", "KON-12").text("KON-12");
/// ```
#[derive(Clone)]
pub struct LinkOffer {
  label: Arc<str>,
  target: Arc<str>,
  text: Option<Arc<str>>,
  icon: Option<Name>,
  description: Option<Arc<str>>,
  wiki: Option<bool>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl LinkOffer {
  /// An offer called `label` of a link to `target`, which shows the label.
  pub fn new(label: &str, target: &str) -> Self {
    Self {
      label: Arc::from(label),
      target: Arc::from(target),
      text: None,
      icon: None,
      description: None,
      wiki: None,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// The words the link shows, in place of the label.
  pub fn text(mut self, text: &str) -> Self {
    self.text = Some(Arc::from(text));
    self
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// A line under the label: where the page is, the state of a task.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Whether the link is a wiki link ([`WikiLink`]), in place of what its trigger says
  /// ([`LinkTrigger::wiki`]).
  pub fn wiki(mut self, wiki: bool) -> Self {
    self.wiki = Some(wiki);
    self
  }

  /// Whether the offer is shown dimmed and cannot be chosen: a click does nothing, and Up and Down pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the offer's row in the list.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the offer's row in the list.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What the offer is called.
  pub fn label(&self) -> &str {
    &self.label
  }

  /// What the link goes to.
  pub fn target(&self) -> &str {
    &self.target
  }

  pub(crate) fn icon_name(&self) -> Option<&Name> {
    self.icon.as_ref()
  }

  pub(crate) fn description_text(&self) -> Option<&str> {
    self.description.as_deref()
  }

  pub(crate) fn is_disabled(&self) -> bool {
    self.disabled
  }

  pub(crate) fn row_id(&self) -> Option<&Arc<str>> {
    self.id.as_ref()
  }

  pub(crate) fn row_key(&self) -> Option<&Arc<str>> {
    self.key.as_ref()
  }

  /// The link the offer puts in the text: its words with the link over them, a wiki link where the offer, else
  /// its trigger (`wiki`), says so. A wiki link with the label for its words is titled by it, and written
  /// `[[target]]`; one with words of the offer's own is written `[[target|words]]`.
  pub fn link(&self, wiki: bool) -> StyledText {
    let words = self.text.as_ref().unwrap_or(&self.label);
    let style = if self.wiki.unwrap_or(wiki) {
      let form = match &self.text {
        Some(_) => WikiLink::Worded,
        None => WikiLink::Titled(self.label.clone()),
      };
      SpanStyle::new().wiki_link(self.target.clone(), form)
    } else {
      SpanStyle::new().link(self.target.clone())
    };
    StyledText::new(words.to_string(), vec![StyledSpan::new(0..words.len(), style)])
  }

  /// Whether a list narrowed by `query`, in lower case, keeps the offer: its label, its words or its target has it.
  fn matches(&self, query: &str) -> bool {
    [Some(&self.label), self.text.as_ref(), Some(&self.target)]
      .into_iter()
      .flatten()
      .any(|held| held.to_lowercase().contains(query))
  }
}

/// What a trigger offers: a list, which the editor narrows to the offers whose label, words or target has what was
/// typed after the trigger, or a function of what was typed.
#[derive(Clone)]
pub enum LinkSource {
  List(Arc<[LinkOffer]>),
  Find(Arc<dyn Fn(&str) -> Vec<LinkOffer> + Send + Sync>),
}

impl From<Vec<LinkOffer>> for LinkSource {
  fn from(list: Vec<LinkOffer>) -> Self {
    Self::List(list.into())
  }
}

impl<F: Fn(&str) -> Vec<LinkOffer> + Send + Sync + 'static> From<F> for LinkSource {
  fn from(find: F) -> Self {
    Self::Find(Arc::new(find))
  }
}

/// The most offers a list shows.
pub const LINK_OFFERS_LIMIT: usize = 8;

/// The longest text after a trigger that is still looked up: past it the trigger is taken for text.
const LONGEST_QUERY: usize = 64;

impl LinkSource {
  /// The offers for `query`, typed after the trigger, at most [`LINK_OFFERS_LIMIT`].
  pub fn offers(&self, query: &str) -> Vec<LinkOffer> {
    let mut offers = match self {
      Self::Find(find) => find(query),
      Self::List(list) => {
        let query = query.to_lowercase();
        list.iter().filter(|offer| offer.matches(&query)).cloned().collect()
      }
    };
    offers.truncate(LINK_OFFERS_LIMIT);
    offers
  }
}

/// A string that opens a list of links as it is typed in a [`RichTextEditor`](super::RichTextEditor), and what the
/// list offers.
///
/// ```
/// use lurq::ui::{LinkOffer, LinkTrigger};
///
/// let pages = vec![LinkOffer::new("Roadmap 2027", "/roadmap"), LinkOffer::new("Onboarding", "/onboarding")];
/// // `[[` links a page as a wiki link, and a title has spaces in it.
/// let page = LinkTrigger::new("[[", pages).wiki(true).spaces(true);
/// // `KON-` links a task the app finds by what follows it.
/// let task = LinkTrigger::new("KON-", |typed: &str| {
///   vec![LinkOffer::new(&format!("KON-{typed}"), &format!("https://tracker.example/KON-{typed}"))]
/// });
/// ```
#[derive(Clone)]
pub struct LinkTrigger {
  text: Arc<str>,
  source: LinkSource,
  wiki: bool,
  spaces: bool,
}

impl LinkTrigger {
  /// A trigger `text` of any length that offers `offers`: a list of [`LinkOffer`]s, narrowed to what is typed after
  /// the trigger, or a function of what is typed. One that starts with a letter or a digit is a trigger only at the
  /// start of a word; one that starts with a mark (`[[`, `@`) anywhere.
  pub fn new(text: &str, offers: impl Into<LinkSource>) -> Self {
    Self {
      text: Arc::from(text),
      source: offers.into(),
      wiki: false,
      spaces: false,
    }
  }

  /// Whether the links it puts in are wiki links ([`WikiLink`]), written `[[target]]` in Markdown: not unless told.
  /// An offer may say otherwise ([`LinkOffer::wiki`]).
  pub fn wiki(mut self, wiki: bool) -> Self {
    self.wiki = wiki;
    self
  }

  /// Whether what is typed after it may have spaces in it, as a page's title has: not unless told, and a space
  /// then ends it.
  pub fn spaces(mut self, spaces: bool) -> Self {
    self.spaces = spaces;
    self
  }

  /// The string that opens the list.
  pub fn text(&self) -> &str {
    &self.text
  }

  pub(crate) fn source(&self) -> &LinkSource {
    &self.source
  }

  pub(crate) fn is_wiki(&self) -> bool {
    self.wiki
  }
}

/// A trigger the caret is after: where it starts in the text, which of the triggers it is, and what is typed after
/// it up to the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingTrigger {
  pub start: usize,
  pub trigger: usize,
  pub query: String,
}

/// The trigger the caret at `caret` of `text` is after, in its paragraph: the nearest before it that stands where a
/// trigger may, with nothing linked or marked as code from it to the caret, and with what is typed after it short and,
/// unless the trigger allows them, with no spaces. Of two that overlap, the one that starts first: `[[` and not the
/// `[` in it.
pub(crate) fn pending_trigger(
  text: &str,
  caret: usize,
  spans: &[StyledSpan],
  triggers: &[LinkTrigger],
) -> Option<PendingTrigger> {
  let before = text.get(..caret)?;
  let line_start = before.rfind('\n').map_or(0, |at| at + 1);
  let line = &before[line_start..];
  let mut best: Option<(usize, usize)> = None;
  for (index, trigger) in triggers.iter().enumerate() {
    let Some(start) = last_place(text, line_start, line, caret, spans, trigger) else {
      continue;
    };
    let length = trigger.text.len();
    best = match best {
      None => Some((start, index)),
      Some((held, held_index)) => {
        let held_length = triggers[held_index].text.len();
        let inside_held = held < start && start < held + held_length;
        let holds = start < held && held < start + length;
        let wins = holds || (!inside_held && (start > held || start == held && length > held_length));
        Some(if wins { (start, index) } else { (held, held_index) })
      }
    };
  }
  let (start, trigger) = best?;
  let query = text[start + triggers[trigger].text.len()..caret].to_owned();
  Some(PendingTrigger { start, trigger, query })
}

/// Where `trigger` last stands in `line`, the text of a paragraph from `line_start` up to the caret, as a trigger
/// whose query reaches the caret.
fn last_place(
  text: &str,
  line_start: usize,
  line: &str,
  caret: usize,
  spans: &[StyledSpan],
  trigger: &LinkTrigger,
) -> Option<usize> {
  let word = trigger.text.chars().next()?.is_alphanumeric();
  for (at, _) in line.rmatch_indices(trigger.text.as_ref()) {
    let start = line_start + at;
    let query = &text[start + trigger.text.len()..caret];
    // An earlier place has this query in its own: nothing before it can do better.
    if query.chars().count() > LONGEST_QUERY || !trigger.spaces && query.contains(char::is_whitespace) {
      return None;
    }
    let held = spans
      .iter()
      .any(|span| span.range.start < caret && start < span.range.end && (span.style.link.is_some() || span.style.code));
    if held {
      return None;
    }
    let after_word = text[..start]
      .chars()
      .next_back()
      .is_some_and(|before| before.is_alphanumeric() || before == '_');
    if word && after_word {
      continue;
    }
    return Some(start);
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn offers() -> Vec<LinkOffer> {
    vec![
      LinkOffer::new("Roadmap 2027", "/roadmap"),
      LinkOffer::new("Onboarding", "/onboarding"),
      LinkOffer::new("Release notes", "/releases").text("the notes"),
    ]
  }

  fn triggers() -> Vec<LinkTrigger> {
    vec![
      LinkTrigger::new("[[", offers()).wiki(true).spaces(true),
      LinkTrigger::new("KON-", |typed: &str| {
        vec![LinkOffer::new(
          &format!("KON-{typed}"),
          &format!("https://tracker.example/KON-{typed}"),
        )]
      }),
      LinkTrigger::new("[", Vec::new()),
    ]
  }

  /// The trigger the caret at the end of `text` is after: its start, which, and the query.
  fn at_end(text: &str) -> Option<(usize, &'static str, String)> {
    at_end_with(text, &[])
  }

  fn at_end_with(text: &str, spans: &[StyledSpan]) -> Option<(usize, &'static str, String)> {
    let names = ["[[", "KON-", "["];
    pending_trigger(text, text.len(), spans, &triggers()).map(|found| (found.start, names[found.trigger], found.query))
  }

  #[test]
  fn a_trigger_of_several_characters_is_found_with_what_follows_it() {
    assert_eq!(at_end("See [["), Some((4, "[[", String::new())));
    assert_eq!(at_end("See [[road"), Some((4, "[[", "road".into())));
    assert_eq!(at_end("Fixed in KON-12"), Some((9, "KON-", "12".into())));
    assert_eq!(at_end("KON-"), Some((0, "KON-", String::new())));
  }

  #[test]
  fn the_longer_trigger_that_holds_a_shorter_one_wins() {
    assert_eq!(at_end("[[x").map(|found| found.1), Some("[["));
    assert_eq!(at_end("a [x").map(|found| found.1), Some("["));
  }

  #[test]
  fn a_trigger_of_letters_starts_a_word_and_one_of_marks_stands_anywhere() {
    assert_eq!(at_end("XKON-12"), None);
    assert_eq!(at_end("(KON-12").map(|found| found.0), Some(1));
    assert_eq!(at_end("word[[pa").map(|found| found.0), Some(4));
  }

  #[test]
  fn a_space_ends_a_query_unless_the_trigger_allows_it() {
    assert_eq!(at_end("KON-12 and"), None);
    assert_eq!(at_end("[[Release no"), Some((0, "[[", "Release no".into())));
    // A long run of text after a trigger is not looked up.
    assert_eq!(at_end(&format!("[[{}", "a ".repeat(40))), None);
  }

  #[test]
  fn only_the_paragraph_of_the_caret_is_looked_in() {
    assert_eq!(at_end("[[road\nmap"), None);
    assert_eq!(at_end("first\n[[map"), Some((6, "[[", "map".into())));
  }

  #[test]
  fn a_trigger_in_a_link_or_in_code_opens_nothing() {
    let link = StyledSpan::new(0..6, SpanStyle::new().link("KON-12"));
    assert_eq!(at_end_with("KON-12", &[link]), None);
    let code = StyledSpan::new(4..8, SpanStyle::new().code());
    assert_eq!(at_end_with("see [[ab", &[code]), None);
    // A link before the trigger is no matter.
    let before = StyledSpan::new(0..3, SpanStyle::new().link("/a"));
    assert_eq!(at_end_with("abc [[x", &[before]).map(|found| found.0), Some(4));
  }

  #[test]
  fn a_list_is_narrowed_by_label_words_or_target_and_a_function_is_asked() {
    let list = LinkSource::from(offers());
    let labels = |query: &str| {
      list
        .offers(query)
        .iter()
        .map(|offer| offer.label().to_owned())
        .collect::<Vec<_>>()
    };
    assert_eq!(labels(""), ["Roadmap 2027", "Onboarding", "Release notes"]);
    assert_eq!(labels("ROAD"), ["Roadmap 2027"]);
    assert_eq!(labels("notes"), ["Release notes"]);
    assert_eq!(labels("/onb"), ["Onboarding"]);
    assert!(labels("nothing").is_empty());
    let many = LinkSource::from((0..20).map(|n| LinkOffer::new(&n.to_string(), "/")).collect::<Vec<_>>());
    assert_eq!(many.offers("").len(), LINK_OFFERS_LIMIT);
    let find = LinkSource::from(|typed: &str| vec![LinkOffer::new(typed, typed)]);
    assert_eq!(find.offers("42")[0].target(), "42");
  }

  #[test]
  fn an_offer_puts_in_a_link_or_a_wiki_link_of_its_words() {
    let plain = LinkOffer::new("Roadmap 2027", "/roadmap").link(false);
    assert_eq!(plain.text, "Roadmap 2027");
    assert_eq!(plain.spans, [StyledSpan::new(0..12, SpanStyle::new().link("/roadmap"))]);
    let titled = LinkOffer::new("Roadmap 2027", "/roadmap").link(true);
    assert_eq!(
      titled.spans[0].style.wiki,
      Some(WikiLink::Titled("Roadmap 2027".into()))
    );
    let worded = LinkOffer::new("Roadmap 2027", "/roadmap").text("the plan").link(true);
    assert_eq!(worded.text, "the plan");
    assert_eq!(worded.spans[0].style.wiki, Some(WikiLink::Worded));
    let refused = LinkOffer::new("Roadmap 2027", "/roadmap").wiki(false).link(true);
    assert_eq!(refused.spans[0].style.wiki, None);
  }
}
