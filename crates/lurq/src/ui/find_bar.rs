//! A find bar: the search of what a view shows, hung from the top of the
//! view over its content.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons,
  input::{Input, InputPart, InputSize},
  menu::{Menu, MenuItem},
  names,
  part_style::{Length, PartAnimation},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::KeyframesId,
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{KeyboardEvent, TextInputEvent},
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
    },
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, HitTestBehavior, NodeUpdate, border::Border, dimension::Dimension, node::Node},
};

/// The keyframes a toast comes in and leaves with from above, which every
/// window has (`ui/toast.rs`): a fade, and a move of 16 from the top.
const ENTER_FROM_ABOVE: KeyframesId = KeyframesId::new(0xFF12);
const LEAVE_UPWARD: KeyframesId = KeyframesId::new(0xFF14);

/// The parts of a find bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FindBarPart {
  /// The room over the view the bar searches, which it hangs in: as large as that, and taking no press itself.
  Root,
  /// The bar: its fill, its edge, its shadow, its width, how it comes in and leaves.
  Bar,
  /// The row of the field, the count and the buttons.
  FindRow,
  /// The row of the replace field and its buttons.
  ReplaceRow,
  /// What the bar says of its fields: laid over an input's box.
  Field,
  /// The row of options over the end of the field.
  Options,
  /// An option of the search: match case, whole word, regular expression.
  Option,
  /// An option that is on, after `Option`.
  OptionOn,
  /// How many matches there are, and which is current: "3 of 12".
  Count,
  /// What the bar says of its buttons: laid over a button's root.
  Button,
  /// What the bar says of a button's icon: laid over a button's icon.
  ButtonIcon,
  /// The room at the end of the replace row, under the find row's buttons.
  Pad,
}

names!(FindBarPart {
  Root => "root",
  Bar => "bar",
  FindRow => "find_row",
  ReplaceRow => "replace_row",
  Field => "field",
  Options => "options",
  Option => "option",
  OptionOn => "option_on",
  Count => "count",
  Button => "button",
  ButtonIcon => "button_icon",
  Pad => "pad",
});

/// The states of a find bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FindBarState {
  /// The bar is closed: gone, or leaving.
  Closed,
  /// Something is searched for and nothing matches it.
  NoMatches,
  /// The bar has the button that shows the replace row, and the row is indented under the field.
  Expandable,
  /// The replace row is shown.
  Replacing,
  /// Its controls take no input.
  Disabled,
}

names!(FindBarState {
  Closed => "closed",
  NoMatches => "no_matches",
  Expandable => "expandable",
  Replacing => "replacing",
  Disabled => "disabled",
});

/// What builds a find bar's nodes: a theme's in place of the default one.
pub type FindBarRenderer = Arc<dyn Fn(FindBarView) -> Element + Send + Sync>;

type Run = Arc<dyn Fn() + Send + Sync>;
type TextHandler = Arc<dyn Fn(&str) + Send + Sync>;
type FlagHandler = Arc<dyn Fn(bool) + Send + Sync>;
type CountText = Arc<dyn Fn(usize, usize) -> String + Send + Sync>;

/// An option of the search, as the app gave it: whether it is on, the signal
/// it is bound to, and what hears it change.
#[derive(Clone, Default)]
struct Flag {
  given: bool,
  on: bool,
  signal: Option<Signal<bool>>,
  handlers: Vec<FlagHandler>,
}

impl Flag {
  fn bound(signal: Signal<bool>) -> Self {
    Self {
      given: true,
      on: signal.get(),
      signal: Some(signal),
      handlers: Vec::new(),
    }
  }

  fn held(on: bool, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    Self {
      given: true,
      on,
      signal: None,
      handlers: vec![Arc::new(handler)],
    }
  }

  fn listen(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.handlers.push(Arc::new(handler));
    self
  }

  /// What a press of it does: the signal set to the other value first, then its handlers told it.
  fn press(&self) -> Run {
    let (signal, handlers, next) = (self.signal.clone(), self.handlers.clone(), !self.on);
    Arc::new(move || {
      if let Some(signal) = &signal {
        signal.set(next);
      }
      for handler in &handlers {
        handler(next);
      }
    })
  }
}

/// The words of the bar: the names of its controls and what it says.
#[derive(Clone)]
struct Words {
  label: Arc<str>,
  placeholder: Arc<str>,
  replace_placeholder: Arc<str>,
  previous: Arc<str>,
  next: Arc<str>,
  close: Arc<str>,
  match_case: Arc<str>,
  whole_word: Arc<str>,
  regex: Arc<str>,
  replace: Arc<str>,
  replace_all: Arc<str>,
  toggle_replace: Arc<str>,
  options: Arc<str>,
  match_case_text: Arc<str>,
  whole_word_text: Arc<str>,
  regex_text: Arc<str>,
}

/// A find bar: what a person types to find in the view in front of them, how
/// many matches there are, and the buttons that go to the next and the one
/// before. With a replace text, the row that replaces them.
///
/// ```
/// use lurq::{
///   app::ctx::Ctx,
///   components::Stack,
///   core::Signal,
///   node::Element,
///   ui::FindBar,
/// };
///
/// # fn build(ctx: &mut Ctx, text: Element, open: Signal<bool>, query: Signal<String>, case: Signal<bool>) -> Element {
/// let bar = FindBar::new(query)
///   .open(open)
///   .matches(3, 12)
///   .match_case(case)
///   .on_next(|| println!("next"))
///   .on_previous(|| println!("previous"))
///   .mount(ctx);
/// Stack::new().child(text).child(bar).into()
/// # }
/// ```
///
/// It is as large as the room it is given and takes no press itself: put it
/// over the view it searches, in a [`Stack`](crate::components::Stack). The
/// bar hangs at the top end of that room, comes in as it opens and leaves as
/// it closes. Enter in the field goes to the next match and Shift+Enter to
/// the one before, as F3 and Shift+F3 do anywhere in the bar; Enter in the
/// replace field replaces the current one. Escape and the close button
/// dismiss it. It is a search to what reads the tree.
///
/// The app binds the keys that open it (Ctrl+F) and F3 outside it.
/// Mounted ([`mount`](Self::mount)), it puts the focus in its field as it
/// opens (as its `open` signal turns true, or as it is first rendered where
/// it has none) and, as it closes with the focus in it, gives the focus to what
/// [`return_focus`](Self::return_focus) names: the view it searched.
#[derive(Clone)]
pub struct FindBar {
  query: Signal<String>,
  on_query: Vec<TextHandler>,
  on_next: Vec<Run>,
  on_previous: Vec<Run>,
  matches: Option<(usize, usize)>,
  count_text: Option<CountText>,
  match_case: Flag,
  whole_word: Flag,
  regex: Flag,
  replace: Option<Signal<String>>,
  replace_open: Option<Signal<bool>>,
  on_replace: Vec<TextHandler>,
  on_replace_all: Vec<TextHandler>,
  open: Option<Signal<bool>>,
  on_dismiss: Vec<Run>,
  on_open_change: Vec<FlagHandler>,
  return_focus: Option<ElementRef>,
  disabled: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FindBar {
  /// The bar of what is typed in `query`: the app finds it, and gives the bar the count.
  pub fn new(query: Signal<String>) -> Self {
    let locale = super::current_locale();
    Self {
      query,
      on_query: Vec::new(),
      on_next: Vec::new(),
      on_previous: Vec::new(),
      matches: None,
      count_text: None,
      match_case: Flag::default(),
      whole_word: Flag::default(),
      regex: Flag::default(),
      replace: None,
      replace_open: None,
      on_replace: Vec::new(),
      on_replace_all: Vec::new(),
      open: None,
      on_dismiss: Vec::new(),
      on_open_change: Vec::new(),
      return_focus: None,
      disabled: false,
      words: Words {
        label: locale.editing.find.clone(),
        placeholder: locale.editing.find.clone(),
        replace_placeholder: locale.editing.replace.clone(),
        previous: locale.editing.previous_match.clone(),
        next: locale.editing.next_match.clone(),
        close: locale.common.close.clone(),
        match_case: locale.editing.match_case.clone(),
        whole_word: locale.editing.match_whole_word.clone(),
        regex: locale.editing.use_regex.clone(),
        replace: locale.editing.replace.clone(),
        replace_all: locale.editing.replace_all.clone(),
        toggle_replace: locale.editing.toggle_replace.clone(),
        options: locale.editing.search_options.clone(),
        match_case_text: Arc::from("Aa"),
        whole_word_text: Arc::from("ab"),
        regex_text: Arc::from(".*"),
      },
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called after each edit of the field, with what it holds then: where the app finds it again.
  pub fn on_query(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_query.push(Arc::new(handler));
    self
  }

  /// What going to the next match does: Enter in the field, F3, the down arrow.
  pub fn on_next(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_next.push(Arc::new(handler));
    self
  }

  /// What going to the match before does: Shift+Enter in the field, Shift+F3, the up arrow.
  pub fn on_previous(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_previous.push(Arc::new(handler));
    self
  }

  /// How many matches there are, and which of them is current, counted from
  /// 1 (0 where none is): the bar says "3 of 12", or "No results" for none.
  /// Without it the bar says nothing of the count.
  pub fn matches(mut self, current: usize, total: usize) -> Self {
    self.matches = Some((current, total));
    self
  }

  /// What the bar says of the count, in place of "3 of 12" and "No results":
  /// told the current match and how many there are.
  pub fn count_text(mut self, text: impl Fn(usize, usize) -> String + Send + Sync + 'static) -> Self {
    self.count_text = Some(Arc::new(text));
    self
  }

  /// Whether the search matches the case of what is typed, bound to the
  /// app's signal: with it the bar has the option "Aa" in its field.
  pub fn match_case(mut self, on: Signal<bool>) -> Self {
    self.match_case = Flag::bound(on);
    self
  }

  /// Whether the search matches the case, held by the app: each press tells `on_change` the other value.
  pub fn match_case_controlled(mut self, on: bool, on_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.match_case = Flag::held(on, on_change);
    self
  }

  /// Called when a person turns matching the case on or off, after the signal is set.
  pub fn on_match_case_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.match_case = self.match_case.listen(handler);
    self
  }

  /// Whether the search matches whole words only, bound to the app's
  /// signal: with it the bar has the option "ab" in its field.
  pub fn whole_word(mut self, on: Signal<bool>) -> Self {
    self.whole_word = Flag::bound(on);
    self
  }

  /// Whether the search matches whole words only, held by the app.
  pub fn whole_word_controlled(mut self, on: bool, on_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.whole_word = Flag::held(on, on_change);
    self
  }

  /// Called when a person turns matching whole words on or off, after the signal is set.
  pub fn on_whole_word_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.whole_word = self.whole_word.listen(handler);
    self
  }

  /// Whether what is typed is a regular expression, bound to the app's
  /// signal: with it the bar has the option ".*" in its field.
  pub fn regex(mut self, on: Signal<bool>) -> Self {
    self.regex = Flag::bound(on);
    self
  }

  /// Whether what is typed is a regular expression, held by the app.
  pub fn regex_controlled(mut self, on: bool, on_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.regex = Flag::held(on, on_change);
    self
  }

  /// Called when a person turns the regular expression on or off, after the signal is set.
  pub fn on_regex_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.regex = self.regex.listen(handler);
    self
  }

  /// What the matches are replaced with: with it the bar has the replace row
  /// under its field, with the buttons that replace the current match and all of them.
  pub fn replace(mut self, text: Signal<String>) -> Self {
    self.replace = Some(text);
    self
  }

  /// Whether the replace row is shown, bound to the app's signal: with it the
  /// bar has the chevron before its field that shows the row and hides it.
  /// Without it the row is always shown where there is a replace text.
  pub fn replace_open(mut self, open: Signal<bool>) -> Self {
    self.replace_open = Some(open);
    self
  }

  /// What replacing the current match does, with the replace text: its
  /// button, and Enter in the replace field.
  pub fn on_replace(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_replace.push(Arc::new(handler));
    self
  }

  /// What replacing every match does, with the replace text.
  pub fn on_replace_all(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_replace_all.push(Arc::new(handler));
    self
  }

  /// Whether the bar is open, bound to the app's signal, which Escape and the
  /// close button set to false. Without it the bar is always shown.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Called when a person dismisses the bar, by Escape or its close button: after the signal is set.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Called with false when a person closes the bar, after `on_dismiss`.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Where the focus goes when the bar closes with the focus in it: the view
  /// it searched. Without it the focus leaves with the bar. A bar does this
  /// where it is mounted ([`mount`](Self::mount)).
  pub fn return_focus(mut self, element: ElementRef) -> Self {
    self.return_focus = Some(element);
    self
  }

  /// Whether the bar's controls take no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the bar and its field are called to what reads the tree: "Find" by default.
  pub fn label(mut self, label: &str) -> Self {
    self.words.label = Arc::from(label);
    self
  }

  /// What the field shows while it is empty: "Find" by default.
  pub fn placeholder(mut self, text: &str) -> Self {
    self.words.placeholder = Arc::from(text);
    self
  }

  /// What the replace field shows while it is empty, and what it is called: "Replace" by default.
  pub fn replace_placeholder(mut self, text: &str) -> Self {
    self.words.replace_placeholder = Arc::from(text);
    self
  }

  /// What the button to the match before is called: "Previous match" by default.
  pub fn previous_label(mut self, label: &str) -> Self {
    self.words.previous = Arc::from(label);
    self
  }

  /// What the button to the next match is called: "Next match" by default.
  pub fn next_label(mut self, label: &str) -> Self {
    self.words.next = Arc::from(label);
    self
  }

  /// What the close button is called: "Close" by default.
  pub fn close_label(mut self, label: &str) -> Self {
    self.words.close = Arc::from(label);
    self
  }

  /// What the option that matches the case is called: "Match case" by default.
  pub fn match_case_label(mut self, label: &str) -> Self {
    self.words.match_case = Arc::from(label);
    self
  }

  /// What the option that matches whole words is called: "Match whole word" by default.
  pub fn whole_word_label(mut self, label: &str) -> Self {
    self.words.whole_word = Arc::from(label);
    self
  }

  /// What the option of a regular expression is called: "Use regular expression" by default.
  pub fn regex_label(mut self, label: &str) -> Self {
    self.words.regex = Arc::from(label);
    self
  }

  /// What the option that matches the case shows: "Aa" by default.
  pub fn match_case_text(mut self, text: &str) -> Self {
    self.words.match_case_text = Arc::from(text);
    self
  }

  /// What the option that matches whole words shows: "ab" by default.
  pub fn whole_word_text(mut self, text: &str) -> Self {
    self.words.whole_word_text = Arc::from(text);
    self
  }

  /// What the option of a regular expression shows: ".*" by default.
  pub fn regex_text(mut self, text: &str) -> Self {
    self.words.regex_text = Arc::from(text);
    self
  }

  /// What the button that opens the menu of the options is called, where the field has no room for them in it:
  /// "Search options" by default.
  pub fn options_label(mut self, label: &str) -> Self {
    self.words.options = Arc::from(label);
    self
  }

  /// What the button that replaces the current match is called: "Replace" by default.
  pub fn replace_label(mut self, label: &str) -> Self {
    self.words.replace = Arc::from(label);
    self
  }

  /// What the button that replaces every match is called: "Replace all" by default.
  pub fn replace_all_label(mut self, label: &str) -> Self {
    self.words.replace_all = Arc::from(label);
    self
  }

  /// What the chevron that shows the replace row is called: "Toggle replace" by default.
  pub fn toggle_replace_label(mut self, label: &str) -> Self {
    self.words.toggle_replace = Arc::from(label);
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

  /// Puts the bar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Draws its buttons as they are under the pointer, held down or focused
  /// from the keyboard, with nothing pointing at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its field has this id with `-field` after it; its
  /// buttons `-previous`, `-next`, `-close`, `-toggle-replace`, `-replace`
  /// and `-replace-all`; its options `-match-case`, `-whole-word` and
  /// `-regex`; the replace field `-replace-field`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Whether the bar is shown.
  fn is_open(&self) -> bool {
    self.open.as_ref().is_none_or(|open| open.get())
  }

  /// Puts the bar in the tree of `ctx`'s component, in the theme of what is
  /// being rendered: as [`build`](Self::build) does, and it puts the focus in
  /// its field as it opens, and where [`return_focus`](Self::return_focus)
  /// says as it closes with the focus in it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<FindBarHost>(FindBarProps { bar: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the bar in `theme`. Converting a bar into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.view(theme, ElementRef::new(), None)
  }

  fn view(self, theme: &Theme, field: ElementRef, room: Option<Room>) -> Element {
    let open = self.is_open();
    let replacing = self.replace.is_some() && self.replace_open.as_ref().is_none_or(|open| open.get());
    let no_matches = self.matches.is_some_and(|(_, total)| total == 0) && !self.query.get().is_empty();
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(FindBarState::Closed, !open)
      .state(FindBarState::NoMatches, no_matches)
      .state(
        FindBarState::Expandable,
        self.replace.is_some() && self.replace_open.is_some(),
      )
      .state(FindBarState::Replacing, replacing)
      .state(FindBarState::Disabled, self.disabled);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let count_text = self.matches.map(|(current, total)| match &self.count_text {
      Some(text) => Arc::from(text(current, total)),
      None if total == 0 => super::current_locale().common.no_results.clone(),
      None => Arc::from(crate::core::fill(
        &super::current_locale().common.place_of,
        &[("at", &current.to_string()), ("total", &total.to_string())],
      )),
    });
    let dismiss = {
      let (open, handlers, changes) = (self.open.clone(), self.on_dismiss.clone(), self.on_open_change.clone());
      Arc::new(move || {
        if let Some(open) = &open {
          open.set(false);
        }
        for handler in &handlers {
          handler();
        }
        for handler in &changes {
          handler(false);
        }
      }) as Run
    };
    let view = FindBarView {
      open,
      replacing,
      count_text,
      recipe: theme
        .resolve::<FindBar>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      next: all(self.on_next),
      previous: all(self.on_previous),
      dismiss,
      field,
      room,
      bar: Bar {
        query: self.query,
        on_query: self.on_query,
        match_case: self.match_case,
        whole_word: self.whole_word,
        regex: self.regex,
        replace: self.replace,
        replace_open: self.replace_open,
        on_replace: self.on_replace,
        on_replace_all: self.on_replace_all,
      },
      words: self.words,
      disabled: self.disabled,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<FindBar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// Every handler of a list, in order, as one.
fn all(handlers: Vec<Run>) -> Run {
  Arc::new(move || {
    for handler in &handlers {
      handler();
    }
  })
}

impl From<FindBar> for Element {
  fn from(bar: FindBar) -> Self {
    bar.build(&current_theme())
  }
}

#[derive(Clone)]
struct FindBarProps {
  bar: FindBar,
}

/// The bar is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for FindBarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for FindBarProps {}

/// The room to read what is typed that a field keeps before the options in it, and the least it keeps before the
/// one button they are behind: a few characters of 13.
const TEXT_ROOM: f32 = 64.0;
const TEXT_LEAST: f32 = 40.0;

/// What a mounted bar keeps from one render to the next: whether the focus is
/// in it, whether it was open, and its field, which it focuses as it opens.
struct FindBarHost {
  focused: Arc<AtomicBool>,
  was_open: AtomicBool,
  /// Whether it was rendered before: a bar bound to the app's signal that is open from the start is not opening.
  rendered: AtomicBool,
  field: ElementRef,
  room: Room,
}

/// The room of a mounted bar's field: what says whether the options stand in the field or behind one button.
#[derive(Clone)]
struct Room {
  /// Whether the field has too little room for the options in it: they are in a menu.
  narrow: Signal<bool>,
  /// The box of the field, measured before the frame is painted, and the least it is wide with the options in it.
  field: ElementRef,
  least: Arc<Mutex<f32>>,
  /// The button of the options, which their menu hangs from, and whether the menu is open.
  anchor: ElementRef,
  menu: Signal<bool>,
}

/// Has each stop of Tab in `node` say whether it has the focus in `focused`.
fn watch_focus(node: &mut Node, focused: &Arc<AtomicBool>) {
  if node.tab_index_value().is_some_and(|index| index >= 0) && node.is_focusable() {
    let (gained, lost) = (focused.clone(), focused.clone());
    NodeUpdate::on_focus(node, move || gained.store(true, Ordering::Relaxed));
    NodeUpdate::on_blur(node, move || lost.store(false, Ordering::Relaxed));
  }
  for child in &mut node.children {
    watch_focus(child, focused);
  }
}

impl Component for FindBarHost {
  type Props = FindBarProps;

  fn create(ctx: &mut Ctx) -> Self {
    let room = Room {
      narrow: ctx.signal(false),
      field: ElementRef::new(),
      least: Arc::new(Mutex::new(0.0)),
      anchor: ElementRef::new(),
      menu: ctx.signal(false),
    };
    // Written only where the field crosses the least width: a view dragged within one side of it renders
    // nothing. The field is as wide in either form, so the form does not change what is measured.
    let (narrow, least) = (room.narrow.clone(), room.least.clone());
    room.field.observe_rect_before_paint(move |rect| {
      let now = rect.width < *least.lock();
      if narrow.get_untracked() != now {
        narrow.set(now);
      }
    });
    Self {
      focused: Arc::new(AtomicBool::new(false)),
      was_open: AtomicBool::new(false),
      rendered: AtomicBool::new(false),
      field: ElementRef::new(),
      room,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let FindBarProps { bar } = ctx.props::<FindBarProps>().clone();
    let open = bar.is_open();
    let was_open = self.was_open.swap(open, Ordering::Relaxed);
    let first = !self.rendered.swap(true, Ordering::Relaxed);
    // A bar the app mounts as it shows it opens with its first render; one bound to the app's signal opens as the
    // signal turns true, and one open from the start leaves the focus where it is.
    if open && !was_open && !(first && bar.open.is_some()) {
      // It opens: what is typed goes into its field.
      ctx.focus(&self.field);
    } else if !open
      && self.focused.swap(false, Ordering::Relaxed)
      && let Some(element) = &bar.return_focus
    {
      // It closes with the focus in it: the focus goes where the app said, and is not lost with the bar.
      ctx.focus(element);
    }
    // Reading it renders the bar again when the field crosses its least width.
    let _ = self.room.narrow.get();
    let mut element = bar.view(&current_theme(), self.field.clone(), Some(self.room.clone()));
    watch_focus(&mut element.node, &self.focused);
    element
  }
}

/// What the controls of the bar are bound to.
struct Bar {
  query: Signal<String>,
  on_query: Vec<TextHandler>,
  match_case: Flag,
  whole_word: Flag,
  regex: Flag,
  replace: Option<Signal<String>>,
  replace_open: Option<Signal<bool>>,
  on_replace: Vec<TextHandler>,
  on_replace_all: Vec<TextHandler>,
}

/// A find bar as a renderer sees it: whether it is open, what it says, the
/// style of its parts, and its controls ready to be dressed.
pub struct FindBarView {
  pub open: bool,
  /// Whether the replace row is shown.
  pub replacing: bool,
  /// What it says of the count, where the app gave one: "3 of 12".
  pub count_text: Option<Arc<str>>,
  /// The style of each part of this bar.
  pub recipe: ResolvedRecipe,
  next: Run,
  previous: Run,
  dismiss: Run,
  field: ElementRef,
  /// The room of a mounted bar's field. A bar that is only built has none, and its options are in its field.
  room: Option<Room>,
  bar: Bar,
  words: Words,
  disabled: bool,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FindBarView {
  /// The style of a part, as this bar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// A button of the bar: its icon alone, called `label`, ghost and small, in the bar's dress.
  pub fn button(&self, label: &str, icon: &str, suffix: &str, press: Run) -> Element {
    let rules = self
      .recipe
      .rules_for(FindBarPart::Button, ButtonPart::Root)
      .extend(&self.recipe.rules_for(FindBarPart::ButtonIcon, ButtonPart::Icon));
    let mut button = Button::new(label)
      .icon(icon)
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .disabled(self.disabled)
      .force(self.forced)
      .rules(&rules)
      .on_click(move |_| press());
    if let Some(id) = self.sub_id(suffix) {
      button = button.id(id);
    }
    button.build(self.theme())
  }

  /// An option of the search in the field: its glyph, called `label`, on or off.
  fn option(&self, flag: &Flag, text: &str, label: &str, suffix: &str) -> Option<Element> {
    if !flag.given {
      return None;
    }
    let part = if flag.on {
      FindBarPart::OptionOn
    } else {
      FindBarPart::Option
    };
    let part = self.part(part).forced(self.forced);
    let press = flag.press();
    let mut root = pressable(&part, self.disabled)
      .describe("a11y_name", Arc::<str>::from(label))
      .describe("a11y_pressed", if flag.on { "true" } else { "false" })
      .on_click(move |_| press())
      .align_items(Alignment::Center)
      // The glyph takes the type of the option, and its color from it: the box is the option's alone.
      .child(part.text(text).nowrap());
    if let Some(id) = self.sub_id(suffix) {
      root = root.id(id);
    }
    Some(root.into())
  }

  /// Whether the options are behind one button: the field of a mounted bar has too little room for them in it.
  pub fn options_in_menu(&self) -> bool {
    self.room.as_ref().is_some_and(|room| room.narrow.get_untracked()) && self.options_width() > 0.0
  }

  /// The one button the options are behind where the field has no room for them: it shows that one is on, and
  /// opens a menu of them, each checked while it is on.
  fn options_button(&self, room: &Room) -> Row {
    let flags = [
      (&self.bar.match_case, &self.words.match_case, "match-case"),
      (&self.bar.whole_word, &self.words.whole_word, "whole-word"),
      (&self.bar.regex, &self.words.regex, "regex"),
    ];
    let on = flags.iter().any(|(flag, ..)| flag.given && flag.on);
    let part = self
      .part(if on { FindBarPart::OptionOn } else { FindBarPart::Option })
      .forced(self.forced);
    let open = room.menu.clone();
    let mut button = pressable(&part, self.disabled)
      .describe("a11y_name", self.words.options.clone())
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if room.menu.get() { "true" } else { "false" })
      .on_click(move |_| open.update(|open| *open = !*open))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .ref_element(room.anchor.clone())
      .child(super::Icon::new(icons::SLIDERS).size(12.0).build(self.theme()));
    if let Some(id) = self.sub_id("options") {
      button = button.id(id);
    }
    let mut menu = Menu::new(room.menu.clone(), room.anchor.clone())
      .label(&self.words.options)
      .placement(Placement::BottomEnd);
    if let Some(id) = self.sub_id("options-menu") {
      menu = menu.id(id);
    }
    for (flag, label, suffix) in flags {
      if !flag.given {
        continue;
      }
      let press = flag.press();
      let mut item = MenuItem::new(label)
        .checked(flag.on)
        .disabled(self.disabled)
        .on_select(move || press());
      if let Some(id) = self.sub_id(suffix) {
        item = item.id(id);
      }
      menu = menu.item(item);
    }
    // The button is `x-options`: the row around it is not named apart.
    Row::new()
      .part(&self.part(FindBarPart::Options).unnamed())
      .child(button)
      .child(menu.build(self.theme()))
  }

  /// The options the app gave, in a row: match case, whole word, regular expression. Where the field of a mounted
  /// bar has no room for them they are behind one button, in its menu.
  pub fn options(&self) -> Option<Row> {
    if let Some(room) = self.room.as_ref().filter(|_| self.options_in_menu()) {
      return Some(self.options_button(room));
    }
    let options: Vec<Element> = [
      self.option(
        &self.bar.match_case,
        &self.words.match_case_text,
        &self.words.match_case,
        "match-case",
      ),
      self.option(
        &self.bar.whole_word,
        &self.words.whole_word_text,
        &self.words.whole_word,
        "whole-word",
      ),
      self.option(&self.bar.regex, &self.words.regex_text, &self.words.regex, "regex"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if options.is_empty() {
      return None;
    }
    Some(options
        .into_iter()
        // `x-options` is the button the options are behind where the field has no room: the row is not named.
        .fold(Row::new().part(&self.part(FindBarPart::Options).unnamed()), |row, option| {
          row.child(option)
        }))
  }

  /// The width the options take over the end of the field.
  fn options_width(&self) -> f32 {
    let count = [&self.bar.match_case, &self.bar.whole_word, &self.bar.regex]
      .iter()
      .filter(|flag| flag.given)
      .count() as f32;
    if count == 0.0 {
      return 0.0;
    }
    let spacing = self.theme().spacing();
    let rest = self.part(FindBarPart::Option).rest();
    let side = rest
      .width
      .and_then(|width| width.resolve(self.theme()))
      .map(|width| match width {
        Dimension::Px(px) => px,
        _ => 0.0,
      })
      .unwrap_or(0.0);
    let gap = self
      .part(FindBarPart::Options)
      .rest()
      .gap
      .map(|gap| gap.resolve(&spacing, 0.0))
      .unwrap_or(0.0);
    side * count + gap * (count - 1.0)
  }

  /// The field of what is found, with its options over its end. Enter goes
  /// to the next match and Shift+Enter to the one before.
  pub fn field(&self) -> Element {
    let handlers = self.bar.on_query.clone();
    let mut input = Input::new(self.bar.query.clone())
      .size(InputSize::Sm)
      .full_width(true)
      .placeholder(&self.words.placeholder)
      .label(&self.words.label)
      .disabled(self.disabled)
      .rules(&self.recipe.rules_for(FindBarPart::Field, InputPart::Control));
    if !handlers.is_empty() {
      input = input.on_input(move |event: &TextInputEvent| {
        let value = event.new_value();
        for handler in &handlers {
          handler(&value);
        }
      });
    }
    if let Some(id) = self.sub_id("field") {
      input = input.id(id);
    }
    let view = input.view(self.theme());
    let (next, previous) = (self.next.clone(), self.previous.clone());
    let mut control = view
      .control()
      .ref_element(self.field.clone())
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Enter" && !event.composing && !(event.ctrl || event.alt || event.meta) {
          event.prevent_default();
          if event.shift { previous() } else { next() }
        }
      });
    let options = self.options();
    // What the field is wide at least: its padding, room to read what is typed, and what stands at its end. With
    // the options in it, more room to read, and the three of them; behind one button, the one.
    let side = self.option_side();
    let start = view.start_padding();
    let with_options = start + TEXT_ROOM + view.end_room(self.options_width());
    let with_button = start + TEXT_LEAST + view.end_room(side);
    if options.is_some() {
      control = control.padding_right(view.end_room(if self.options_in_menu() {
        side
      } else {
        self.options_width()
      }));
    }
    // A mounted bar's field gives way down to the least it has with one button, and under the least it has with
    // the options in it they go behind that button; a bar that is only built keeps the options in the field, and
    // room for them. Under its least the field has a line of its own, the count and the buttons under it.
    let least = match (&self.room, options.is_some()) {
      (Some(room), true) => {
        *room.least.lock() = with_options;
        with_button
      }
      (None, true) => with_options,
      (_, false) => start + TEXT_LEAST + view.end_room(0.0),
    };
    let mut field = Row::new()
      .flex_full(1.0, 1.0, Some(least))
      .min_width(Dimension::Px(least))
      .align_items(Alignment::Stretch)
      .child(view.render_with_end(control, options));
    if let Some(room) = &self.room {
      field = field.ref_element(room.field.clone());
    }
    field.into()
  }

  /// The side of an option, in pixels.
  fn option_side(&self) -> f32 {
    match self
      .part(FindBarPart::Option)
      .rest()
      .width
      .and_then(|width| width.resolve(self.theme()))
    {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  /// What the bar says of the count, where the app gave one.
  pub fn count(&self) -> Option<Element> {
    let text = self.count_text.as_ref()?;
    let part = self.part(FindBarPart::Count);
    Some(
      Row::new()
        .part(&part)
        .align_items(Alignment::Center)
        .describe("a11y_live", "polite")
        .child(part.text(text).nowrap().part(&part))
        .into(),
    )
  }

  /// The chevron before the field that shows the replace row and hides it, where the app bound it.
  pub fn toggle_replace(&self) -> Option<Element> {
    let open = self.bar.replace_open.clone()?;
    self.bar.replace.as_ref()?;
    let icon = if self.replacing {
      icons::CHEVRON_DOWN
    } else {
      icons::CHEVRON_RIGHT
    };
    let mut element = self.button(
      &self.words.toggle_replace,
      icon,
      "toggle-replace",
      Arc::new(move || open.update(|open| *open = !*open)),
    );
    let expanded = if self.replacing { "true" } else { "false" };
    NodeUpdate::describe(&mut *element.node, Arc::from("a11y_expanded"), Arc::from(expanded));
    Some(element)
  }

  /// The row of the field: the chevron of the replace row, the field, the
  /// count, the buttons to the match before and the next, and the close button.
  pub fn find_row(&self) -> Row {
    // What has no room beside the field at its least goes on to a line under it.
    let mut row = Row::new()
      .part(&self.part(FindBarPart::FindRow))
      .wrap()
      .align_items(Alignment::Center);
    if let Some(toggle) = self.toggle_replace() {
      row = row.child(toggle);
    }
    row = row.child(self.field());
    if let Some(count) = self.count() {
      row = row.child(count);
    }
    row
      .child(self.button(&self.words.previous, icons::ARROW_UP, "previous", self.previous.clone()))
      .child(self.button(&self.words.next, icons::ARROW_DOWN, "next", self.next.clone()))
      .child(self.button(&self.words.close, icons::CLOSE, "close", self.dismiss.clone()))
  }

  /// The row that replaces: its field, and the buttons that replace the
  /// current match and every one. Enter in the field replaces the current one.
  pub fn replace_row(&self) -> Option<Row> {
    let text = self.bar.replace.clone().filter(|_| self.replacing)?;
    let replace = {
      let (text, handlers) = (text.clone(), self.bar.on_replace.clone());
      Arc::new(move || {
        let value = text.get_untracked();
        for handler in &handlers {
          handler(&value);
        }
      }) as Run
    };
    let replace_all = {
      let (text, handlers) = (text.clone(), self.bar.on_replace_all.clone());
      Arc::new(move || {
        let value = text.get_untracked();
        for handler in &handlers {
          handler(&value);
        }
      }) as Run
    };
    let mut input = Input::new(text)
      .size(InputSize::Sm)
      .full_width(true)
      .placeholder(&self.words.replace_placeholder)
      .label(&self.words.replace_placeholder)
      .disabled(self.disabled)
      .rules(&self.recipe.rules_for(FindBarPart::Field, InputPart::Control));
    if let Some(id) = self.sub_id("replace-field") {
      input = input.id(id);
    }
    let view = input.view(self.theme());
    let on_enter = replace.clone();
    let control = view.control().on_key_down(move |event: KeyboardEvent| {
      if event.focus_within
        && event.key == "Enter"
        && !event.composing
        && !(event.ctrl || event.alt || event.meta || event.shift)
      {
        event.prevent_default();
        on_enter();
      }
    });
    let least = view.start_padding() + TEXT_LEAST + view.end_room(0.0);
    let field: Element = Row::new()
      .flex_full(1.0, 1.0, Some(least))
      .min_width(Dimension::Px(least))
      .align_items(Alignment::Stretch)
      .child(view.render_with_end(control, None))
      .into();
    Some(
      Row::new()
        .part(&self.part(FindBarPart::ReplaceRow))
        .wrap()
        .align_items(Alignment::Center)
        .child(field)
        .child(self.button(&self.words.replace, icons::REPLACE, "replace", replace))
        .child(self.button(&self.words.replace_all, icons::REPLACE_ALL, "replace-all", replace_all))
        .child(Row::new().part(&self.part(FindBarPart::Pad))),
    )
  }

  /// The bar: the find row, and the replace row under it where it is shown.
  /// It is a search to what reads the tree, called by its label; Escape
  /// dismisses it, and F3 and Shift+F3 go to the next match and the one before.
  pub fn bar(&self) -> Column {
    let (next, previous, dismiss) = (self.next.clone(), self.previous.clone(), self.dismiss.clone());
    let mut bar = Column::new()
      .part(&self.part(FindBarPart::Bar))
      .key("find-bar")
      .describe("a11y_role", "search")
      .describe("a11y_name", self.words.label.clone())
      .on_key_down(move |event: KeyboardEvent| {
        // Only where the focus is in the bar: the app binds what the keys do elsewhere.
        if !event.focus_within || event.default_prevented() || event.composing || event.ctrl || event.alt || event.meta
        {
          return;
        }
        match event.key.as_str() {
          "Escape" if !event.shift => {
            event.prevent_default();
            dismiss();
          }
          "F3" => {
            event.prevent_default();
            if event.shift { previous() } else { next() }
          }
          _ => {}
        }
      })
      .child(self.find_row());
    if let Some(row) = self.replace_row() {
      bar = bar.child(row);
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    bar
  }

  /// How the default renderer builds a find bar: the room over the view it
  /// searches, and in it, at its top end, the bar while it is open. The room
  /// keeps the bar while it leaves.
  pub fn render(self) -> Element {
    let mut root = Column::new()
      .part(&self.part(FindBarPart::Root))
      .presence()
      .justify(Justify::Start)
      .align_items(Alignment::End)
      .overflow_visible()
      .hit_test(HitTestBehavior::ContentOnly);
    if self.open {
      root = root.child(self.bar());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for FindBar {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for FindBar {
  const NAME: &'static str = "find_bar";
  const ROOT: &'static str = "bar";
  type Renderer = FindBarRenderer;

  fn parts() -> crate::ui::Declared {
    FindBarPart::declared()
  }

  fn states() -> crate::ui::Declared {
    FindBarState::declared()
  }

  /// The design's find bar (`pencil/lurq-design-system.pen`, Find Bar): a
  /// panel 440 wide of the popover's color, of the large radius, with an edge
  /// of 1 in the border's color and the large shadow, padded 6, its rows 6
  /// apart and their controls 4 apart. The field is 28 high, of the middle
  /// radius, padded 8 before its text of 13 and 3 after its options, which
  /// are 22 square, of the small radius, their glyph of 11 in the muted
  /// foreground; one that is on has the fill of what is picked out, an edge of
  /// 1 in an input's color, and its glyph in the foreground. The count is 11
  /// in the muted foreground, padded 6. The buttons are 24 square, ghost, with
  /// icons of 14. The replace row is indented 28 under a bar that has the
  /// chevron, and ends 52 before the end. The bar hangs 8 from the top and
  /// the end of the view, and comes down into place as it opens.
  fn recipe() -> Recipe {
    use FindBarPart::*;
    use FindBarState::{Expandable, NoMatches};
    use PaletteColor::{Foreground, Highlight, Input as Edge, MutedForeground, Popover, PopoverForeground};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill).height(Length::Fill).padding_x(8.0).padding_y(8.0)
      })
      .base(Bar, |s| {
        s.width(440.0)
          .max_width(Length::Fill)
          .padding_x(6.0)
          .padding_y(6.0)
          .gap(6.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(edge(PaletteColor::Border))
          .shadow(ShadowStyle::Lg)
          .enter(PartAnimation::new(
            ENTER_FROM_ABOVE,
            MotionDuration::Base,
            MotionEasing::Enter,
          ))
          .exit(PartAnimation::new(
            LEAVE_UPWARD,
            MotionDuration::Fast,
            MotionEasing::Exit,
          ))
      })
      .base(FindRow, |s| s.gap(4.0).min_height(28.0))
      .base(ReplaceRow, |s| s.gap(4.0).min_height(28.0))
      .state(Expandable, ReplaceRow, |s| s.padding_start(28.0))
      .base(Field, |s| {
        s.height(28.0)
          .padding_start(8.0)
          .padding_end(3.0)
          .radius(RadiusSize::Md)
          .font_size(13.0)
      })
      .base(Options, |s| s.gap(2.0))
      .base(Option, |s| {
        fading(
          s.width(22.0)
            .height(22.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .typography(TypographyStyle::Caption)
            .font_size(11.0),
        )
      })
      .hovered(Option, |s| s.background(Highlight).foreground(Foreground))
      .focus_visible(Option, |s| s.focus_ring())
      .base(OptionOn, |s| {
        fading(
          s.width(22.0)
            .height(22.0)
            .radius(RadiusSize::Sm)
            .background(Highlight)
            .border(edge(Edge))
            .foreground(Foreground)
            .typography(TypographyStyle::Caption)
            .font_size(11.0),
        )
      })
      .focus_visible(OptionOn, |s| s.focus_ring())
      .base(Count, |s| {
        s.padding_x(6.0)
          .typography(TypographyStyle::Caption)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .state(NoMatches, Count, |s| s.foreground(PaletteColor::Destructive))
      .base(Button, |s| s.square(24.0).padding_x(0.0).radius(RadiusSize::Sm))
      .base(ButtonIcon, |s| s.square(IconSize::Sm))
      .base(Pad, |s| s.width(52.0))
  }
}
