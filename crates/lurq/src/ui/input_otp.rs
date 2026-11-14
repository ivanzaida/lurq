//! A one-time code: a few characters typed one to a cell, as one field.

use std::{
  cell::RefCell,
  collections::HashMap,
  sync::{Arc, Weak},
  time::Duration,
};

use super::{
  field::{FieldControl, InField},
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::glyph,
  text_box::{self, BoxStates, Judgment, TextBoxSize, TextBoxState},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId},
  app::{
    events::{KeyboardEvent, MouseEvent, PasteEvent, TextInputEvent},
    theme::{IconSize, OpacityLevel, PaletteColor, Theme, TypographyStyle},
  },
  components::{self, ContentKind, KeyboardKind, Row, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::TextStyle},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

/// The keyframes of the caret of a code's cell: shown for half of its period, gone for the other half, as a text
/// input's caret is. Every window has them.
pub const INPUT_OTP_CARET_BLINK: KeyframesId = KeyframesId::new(0xFF41);

/// One shown and one hidden half of the caret's blink: a text input's caret blinks at the same pace.
pub const INPUT_OTP_CARET_PERIOD: Duration = Duration::from_millis(1060);

/// The keyframes a window registers for a code's caret.
pub(crate) fn keyframes() -> [Keyframes; 1] {
  let blink = Keyframes::new(INPUT_OTP_CARET_BLINK)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(0.5, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(0.501, |frame| {
      frame.set(AnimatableProperty::Opacity, 0.0);
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 0.0);
    });
  [blink]
}

/// The parts of a code a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputOtpPart {
  /// The row of cells and separators, and the room between them.
  Root,
  /// A box one character is typed in: its size, its edge, its fill. The cell that has the focus has the ring.
  Cell,
  /// The character in a cell, or the mask's dot for it.
  Value,
  /// The line in the empty cell that has the focus, where the next character goes.
  Caret,
  /// What stands between two groups of cells: a dash.
  Separator,
}

names!(InputOtpPart {
  Root => "root",
  Cell => "cell",
  Value => "value",
  Caret => "caret",
  Separator => "separator",
});

/// The sizes of a code: its cells smaller or larger, with their type.
pub type InputOtpSize = TextBoxSize;

/// The states of a code a recipe styles, besides what the pointer and the keyboard do to it. A cell is in
/// [`Active`](Self::Active) and [`Filled`](Self::Filled) on its own; the rest are the whole code's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputOtpState {
  /// What it holds is wrong: the edge of each cell is in the destructive color.
  Invalid,
  /// What it holds is allowed, with a caveat: the edges are in the warning color.
  Warning,
  /// What it holds was checked and is right: the edges are in the success color.
  Valid,
  /// It can be read and copied, and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// Its cells show a dot for each character.
  Masked,
  /// The cell the focus is on: what is typed next goes into it.
  Active,
  /// A cell that holds a character.
  Filled,
}

names!(InputOtpState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  Masked => "masked",
  Active => "active",
  Filled => "filled",
});

/// What a code is made of: what is taken of what is typed or pasted. Anything else is left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputOtpKind {
  /// The digits 0 to 9: the keyboard of a phone shows digits.
  #[default]
  Digits,
  /// The letters A to Z, of either case, and the digits.
  Alphanumeric,
}

impl InputOtpKind {
  /// Whether `character` is one a code of this kind holds.
  pub fn takes(self, character: char) -> bool {
    match self {
      Self::Digits => character.is_ascii_digit(),
      Self::Alphanumeric => character.is_ascii_alphanumeric(),
    }
  }
}

/// What builds a code's nodes: a theme's in place of the default one.
pub type InputOtpRenderer = Arc<dyn Fn(InputOtpView) -> Element + Send + Sync>;

/// What is called with the code once its last cell is filled.
type CompleteHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A one-time code: a few characters typed one to a cell, the code a message or an authenticator app gives to
/// prove who a person is.
///
/// ```
/// use lurq::{core::Signal, ui::{InputOtp, InputOtpKind}};
///
/// let code = Signal::new(String::new());
/// let otp = InputOtp::new(code.clone())
///   .groups([3, 3])
///   .label("Verification code")
///   .on_complete(|code| println!("verify {code}"));
/// let recovery = InputOtp::new(Signal::new(String::new()))
///   .length(8)
///   .groups([4, 4])
///   .kind(InputOtpKind::Alphanumeric)
///   .uppercase(true);
/// ```
///
/// To a person it is one field: typing fills the cell that has the focus and goes on to the next, Backspace
/// empties the cell and goes back, Left and Right move, a paste of a whole code fills every cell from the first,
/// and a click puts the focus on the cell clicked, or on the first empty one past it. To a form and to what reads
/// the tree it is one text input, called by its label, whose value is the code: under the cells it draws is the
/// engine's [`TextInput`](crate::components::TextInput), which takes what is typed, pasted or filled in by the
/// platform (it says it holds a [`ContentKind::OneTimeCode`]), and is the one stop of Tab.
pub struct InputOtp {
  value: Signal<String>,
  length: usize,
  groups: Vec<usize>,
  kind: InputOtpKind,
  uppercase: bool,
  mask: bool,
  on_complete: Option<CompleteHandler>,
  boxed: BoxStates,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InputOtp {
  /// A code of six digits bound to `value`: its cells show what the signal holds, and what is typed is set on it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      value,
      length: 6,
      groups: Vec::new(),
      kind: InputOtpKind::default(),
      uppercase: false,
      mask: false,
      on_complete: None,
      boxed: BoxStates::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// How many characters the code has: a cell for each. At least one.
  pub fn length(mut self, length: usize) -> Self {
    self.length = length.max(1);
    self
  }

  /// How the cells are grouped, from the first: `[3, 3]` is three cells, a separator, and three more. Cells past
  /// the groups are a group of their own; groups past the length are left out.
  pub fn groups(mut self, groups: impl IntoIterator<Item = usize>) -> Self {
    self.groups = groups.into_iter().filter(|group| *group > 0).collect();
    self
  }

  /// What the code is made of: digits, or letters and digits.
  pub fn kind(mut self, kind: InputOtpKind) -> Self {
    self.kind = kind;
    self
  }

  /// Whether a letter typed or pasted is taken as a capital, so that a code is the same however it was typed.
  pub fn uppercase(mut self, uppercase: bool) -> Self {
    self.uppercase = uppercase;
    self
  }

  /// Whether the cells show a dot for each character in place of it, as a PIN is shown.
  pub fn mask(mut self, mask: bool) -> Self {
    self.mask = mask;
    self
  }

  /// Called with the code when an edit fills its last cell: what verifies it, or moves on.
  pub fn on_complete(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_complete = Some(Arc::new(handler));
    self
  }

  /// An [`InputOtpSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether the code is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether the code is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether the code was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether the code can be read and copied, and not changed. It is still a stop of Tab, and Left and Right
  /// still move.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the code takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// What the code is called, to what reads the tree. It draws nothing: the label a person reads is the field's,
  /// above it, which gives the code this name unless it has its own.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// Puts the code in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the code has in the form it is in: its value is the whole code.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this code over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this code: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the code as it is focused, with nothing focusing it: for a gallery or a test. The focus is on the
  /// first empty cell.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the input under the cells: what is typed in and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the code in `theme`. Converting a code into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = self
      .boxed
      .case(self.states.clone())
      .state(InputOtpState::Masked, self.mask);
    let cell_case = |active: bool, filled: bool| {
      case
        .clone()
        .state(InputOtpState::Active, active)
        .state(InputOtpState::Filled, filled)
    };
    let resolve = |case: &Case| theme.resolve::<InputOtp>(case, Some(&self.rules));
    let cells = [
      resolve(&cell_case(false, false)),
      resolve(&cell_case(false, true)),
      resolve(&cell_case(true, false)),
      resolve(&cell_case(true, true)),
    ];
    let cursor = cursor(&self.value);
    let characters: Vec<char> = self.value.get().chars().take(self.length).collect();
    let last = last_cell(characters.len(), self.length);
    let focused = cursor.focused.get() || self.forced == Some(Interaction::FocusVisible);
    let active = focused.then(|| {
      if cursor.focused.get_untracked() {
        cursor.at.get().min(last)
      } else {
        last
      }
    });
    let editor = Editor {
      value: self.value.clone(),
      cursor,
      length: self.length,
      kind: self.kind,
      uppercase: self.uppercase,
      on_complete: self.on_complete,
    };
    let view = InputOtpView {
      value: self.value,
      length: self.length,
      characters,
      active,
      separators: separators(&self.groups, self.length),
      kind: self.kind,
      mask: self.mask,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      recipe: resolve(&case),
      cells,
      cell_refs: (0..self.length).map(|_| ElementRef::new()).collect(),
      forced: self.forced,
      boxed: self.boxed,
      editor,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<InputOtp>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl FieldControl for InputOtp {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// code itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }
}

impl From<InputOtp> for Element {
  fn from(otp: InputOtp) -> Self {
    otp.build(&current_theme())
  }
}

/// The last cell the focus may be on: the first empty one, or the last cell of a full code.
fn last_cell(filled: usize, length: usize) -> usize {
  filled.min(length.saturating_sub(1))
}

/// The cells a separator follows: the ends of the groups, short of the last cell.
fn separators(groups: &[usize], length: usize) -> Vec<usize> {
  let mut after = Vec::new();
  let mut end = 0;
  for group in groups {
    end += group;
    if end >= length {
      break;
    }
    after.push(end);
  }
  after
}

/// Where a person is in a code: whether its input has the focus, and the cell the focus is on. A builder keeps
/// nothing from one render to the next, so this is kept by the code's signal for as long as the input bound to it
/// is: the input's handlers hold it, and it goes when they do.
struct CursorState {
  focused: Signal<bool>,
  at: Signal<usize>,
}

type Cursor = Arc<CursorState>;

thread_local! {
  /// The cursor of each code that is shown, by the id of its signal. A window's codes are all on its thread.
  static CURSORS: RefCell<HashMap<usize, Weak<CursorState>>> = RefCell::new(HashMap::new());
}

/// The cursor of the code bound to `value`: the one its input holds, or a new one, out of the focus, where there is
/// none.
fn cursor(value: &Signal<String>) -> Cursor {
  CURSORS.with(|cursors| {
    let mut cursors = cursors.borrow_mut();
    if let Some(cursor) = cursors.get(&value.id()).and_then(Weak::upgrade) {
      return cursor;
    }
    cursors.retain(|_, cursor| cursor.strong_count() > 0);
    let cursor = Arc::new(CursorState {
      focused: Signal::new(false),
      at: Signal::new(0),
    });
    cursors.insert(value.id(), Arc::downgrade(&cursor));
    cursor
  })
}

/// What edits a code: the handlers of its input put what is typed, pasted and deleted into its cells.
#[derive(Clone)]
struct Editor {
  value: Signal<String>,
  cursor: Cursor,
  length: usize,
  kind: InputOtpKind,
  uppercase: bool,
  on_complete: Option<CompleteHandler>,
}

impl Editor {
  /// What the code holds, a character for each filled cell.
  fn characters(&self) -> Vec<char> {
    self.value.get_untracked().chars().take(self.length).collect()
  }

  /// The cell the focus is on, of a code with `filled` cells filled.
  fn at(&self, filled: usize) -> usize {
    self.cursor.at.get_untracked().min(last_cell(filled, self.length))
  }

  /// What of `text` the code takes: its characters of the code's kind, as capitals where it says.
  fn taken(&self, text: &str) -> Vec<char> {
    text
      .chars()
      .filter(|character| self.kind.takes(*character))
      .map(|character| {
        if self.uppercase {
          character.to_ascii_uppercase()
        } else {
          character
        }
      })
      .collect()
  }

  /// Sets the code and the cell the focus is on, and tells the app when the edit filled the last cell.
  fn set(&self, before: &[char], characters: Vec<char>, at: usize) {
    let full = characters.len() == self.length && characters != before;
    let code: String = characters.iter().collect();
    self.put_at(last_cell(characters.len(), self.length).min(at));
    if self.value.get_untracked() != code {
      self.value.set(code.clone());
    }
    if full && let Some(complete) = &self.on_complete {
      complete(&code);
    }
  }

  /// Puts the focus on cell `at`, telling what reads the cursor only of a change.
  fn put_at(&self, at: usize) {
    if self.cursor.at.get_untracked() != at {
      self.cursor.at.set(at);
    }
  }

  /// Puts `text` into the cells: a whole code from the first cell, anything shorter over the cells from the one
  /// that has the focus. The focus goes on past what was put.
  fn write(&self, text: &str) {
    let taken = self.taken(text);
    if taken.is_empty() {
      return;
    }
    let before = self.characters();
    if taken.len() >= self.length {
      let code: Vec<char> = taken.into_iter().take(self.length).collect();
      let at = code.len();
      self.set(&before, code, at);
      return;
    }
    let start = self.at(before.len());
    let mut code = before.clone();
    for (offset, character) in taken.iter().enumerate() {
      let cell = start + offset;
      if cell >= self.length {
        break;
      }
      if cell < code.len() {
        code[cell] = *character;
      } else {
        code.push(*character);
      }
    }
    self.set(&before, code, start + taken.len());
  }

  /// Backspace: empties the cell that has the focus where it is filled, else goes back and empties the one before.
  /// The cells after an emptied one move up, so that a code has no gaps.
  fn backspace(&self) {
    let before = self.characters();
    let at = self.at(before.len());
    let mut code = before.clone();
    if at < code.len() {
      code.remove(at);
      self.set(&before, code, at);
    } else if at > 0 {
      code.remove(at - 1);
      self.set(&before, code, at - 1);
    }
  }

  /// Delete: empties the cell that has the focus, and the focus stays.
  fn delete(&self) {
    let before = self.characters();
    let at = self.at(before.len());
    if at < before.len() {
      let mut code = before.clone();
      code.remove(at);
      self.set(&before, code, at);
    }
  }

  /// Empties every cell, and puts the focus on the first.
  fn clear(&self) {
    let before = self.characters();
    self.set(&before, Vec::new(), 0);
  }

  /// Moves the focus by `by` cells, no further than the first empty one.
  fn step(&self, by: isize) {
    let filled = self.characters().len();
    let at = self.at(filled).saturating_add_signed(by);
    self.put_at(at.min(last_cell(filled, self.length)));
  }

  /// Puts the focus on the cell that was clicked, or on the first empty one where that is before it.
  fn click(&self, cell: usize) {
    self.put_at(cell.min(last_cell(self.characters().len(), self.length)));
  }
}

/// A code as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be dressed.
pub struct InputOtpView {
  /// The code, which the input under the cells is bound to.
  pub value: Signal<String>,
  /// How many cells it has.
  pub length: usize,
  /// What the cells hold, from the first: no more than `length` of what the value holds.
  pub characters: Vec<char>,
  /// The cell the focus is on, while the input has the focus.
  pub active: Option<usize>,
  /// The cells a separator follows.
  pub separators: Vec<usize>,
  pub kind: InputOtpKind,
  pub mask: bool,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  /// Whether the code is wrong, as it is shown.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of the code, as a cell that is neither active nor filled has it.
  pub recipe: ResolvedRecipe,
  /// The recipe of a cell: neither active nor filled, filled, active, active and filled.
  cells: [ResolvedRecipe; 4],
  /// What is put on each cell, so that a click on the input is known to be on a cell.
  cell_refs: Vec<ElementRef>,
  forced: Option<Interaction>,
  boxed: BoxStates,
  editor: Editor,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InputOtpView {
  /// The style of a part, as the code is drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  /// The style of a part of the cell at `index`: the cell itself, its value or its caret.
  pub fn cell_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let active = self.active == Some(index);
    let filled = index < self.characters.len();
    self.cells[usize::from(active) * 2 + usize::from(filled)]
      .part(part)
      .forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the cell at `index` shows: its character, a dot where the code is masked, or nothing.
  pub fn shown(&self, index: usize) -> Option<char> {
    let character = *self.characters.get(index)?;
    Some(if self.mask { '\u{2022}' } else { character })
  }

  /// The width and the height of the row of cells, where the recipe gives them in pixels: what the input under the
  /// cells takes, so that a click anywhere on the code is the input's.
  pub fn size(&self) -> Option<(f32, f32)> {
    let theme = self.theme();
    let px = |length: Option<Length>| match length?.resolve(theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    };
    let cell = self.part(InputOtpPart::Cell).rest();
    let (width, height) = (px(cell.width)?, px(cell.height)?);
    let separator = px(
      self
        .part(InputOtpPart::Separator)
        .rest()
        .width
        .or(Some(Length::Icon(IconSize::Md))),
    )?;
    let gap = self
      .part(InputOtpPart::Root)
      .rest()
      .gap
      .map(|gap| gap.resolve(&theme.spacing(), 0.0))
      .unwrap_or(0.0);
    let separators = self.separators.len();
    let items = self.length + separators;
    let row = width * self.length as f32 + separator * separators as f32 + gap * items.saturating_sub(1) as f32;
    Some((row, height.max(separator)))
  }

  /// The input the code is typed into, under the cells: the engine's text input bound to the code, which draws
  /// nothing of its own. It is the one stop of Tab and the one `textbox` to what reads the tree; its handlers put
  /// what is typed, pasted and deleted into the cells, and a click on it puts the focus on the cell under the
  /// pointer.
  pub fn input(&self) -> components::TextInput {
    // The input is there to be typed in and read by tooling: whatever it would paint, the cells paint.
    let clear = Color::new(0, 0, 0, 0);
    let mut control = components::TextInput::new(self.value.clone())
      .single_line()
      .text_style(TextStyle {
        color: clear,
        ..TextStyle::default()
      })
      .caret_color(clear)
      .selection_color(clear)
      .keyboard(match self.kind {
        InputOtpKind::Digits => KeyboardKind::Number,
        InputOtpKind::Alphanumeric => KeyboardKind::Text,
      })
      .content(ContentKind::OneTimeCode)
      .cursor(CursorIcon::Text);
    if self.mask {
      control = control.mask();
    }
    control = match self.size() {
      Some((width, height)) => control.width(width).height(height),
      None => control.width(Dimension::Pct(100.0)).height(Dimension::Pct(100.0)),
    };
    let mut control = text_box::text_box(control, &self.boxed, Vec::new());
    let (read_only, disabled) = (self.boxed.read_only, self.boxed.disabled);
    let editor = self.editor.clone();
    control = control.on_input(move |event: TextInputEvent| {
      // The code is the editor's to change: the input's own edit, at its own caret, is never made.
      event.prevent_default();
      if read_only || disabled {
        return;
      }
      let keyboard = &event.keyboard;
      if event.new_value.is_empty() {
        // Everything selected, and cut or typed over with nothing.
        editor.clear();
      } else if !keyboard.ctrl && !keyboard.meta && event.new_value.contains(keyboard.key.as_str()) {
        // A key typed, or what an input method or the platform put in: the event's key is that text.
        editor.write(&keyboard.key);
      }
    });
    let editor = self.editor.clone();
    control = control.on_paste(move |event: PasteEvent| {
      event.prevent_default();
      if read_only || disabled {
        return;
      }
      if let Some(text) = event.text() {
        editor.write(text);
      }
    });
    let editor = self.editor.clone();
    control = control.on_key_down(move |event: KeyboardEvent| {
      if !event.focus_within || disabled || event.composing {
        return;
      }
      let edits = !read_only;
      match event.key.as_str() {
        "Backspace" if edits => editor.backspace(),
        "Delete" if edits => editor.delete(),
        "Backspace" | "Delete" => {}
        "ArrowLeft" => editor.step(-1),
        "ArrowRight" => editor.step(1),
        "Home" | "ArrowUp" => editor.put_at(0),
        "End" | "ArrowDown" => editor.step(isize::MAX),
        _ => return,
      }
      event.prevent_default();
    });
    let cursor = self.editor.cursor.clone();
    let editor = self.editor.clone();
    control = control
      .on_focus(move || {
        // Tab, or a click: the focus goes on the first empty cell, and a click then moves it to its cell.
        editor.step(isize::MAX);
        if !cursor.focused.get_untracked() {
          cursor.focused.set(true);
        }
      })
      .on_blur({
        let cursor = self.editor.cursor.clone();
        move || {
          if cursor.focused.get_untracked() {
            cursor.focused.set(false);
          }
        }
      });
    let (editor, cells) = (self.editor.clone(), self.cell_refs.clone());
    control = control.on_click(move |event: MouseEvent| {
      if disabled || event.from_keyboard {
        return;
      }
      let under = cells.iter().position(|cell| {
        let bounds = cell.bounds();
        cell.is_attached() && event.x >= bounds.x && event.x < bounds.x + bounds.width
      });
      if let Some(cell) = under {
        editor.click(cell);
      }
    });
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      control = control.name(name.clone());
    }
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    if let Some(key) = &self.key {
      control = control.key(key.clone());
    }
    control
  }

  /// The cell at `index`, in the style of its part, with its character, or with the caret where it is the empty
  /// cell the focus is on. The caret blinks unless the theme reduces motion.
  pub fn cell(&self, index: usize) -> Row {
    let mut cell = Row::new()
      .part(&self.cell_part(index, InputOtpPart::Cell))
      .justify(Justify::Center)
      .align_items(Alignment::Center);
    if let Some(cell_ref) = self.cell_refs.get(index) {
      cell = cell.ref_element(cell_ref.clone());
    }
    if let Some(shown) = self.shown(index) {
      let value = self.cell_part(index, InputOtpPart::Value);
      cell = cell.child(value.text(&shown.to_string()).nowrap());
    } else if self.active == Some(index) {
      let mut caret = Row::new().part(&self.cell_part(index, InputOtpPart::Caret));
      if !self.theme().reduced_motion() {
        caret = caret.animation(
          Animation::new(INPUT_OTP_CARET_BLINK)
            .duration(INPUT_OTP_CARET_PERIOD)
            .easing(Easing::Linear)
            .infinite(),
        );
      }
      cell = cell.child(caret);
    }
    cell
  }

  /// What stands between two groups of cells: the theme's dash, in the style of its part.
  pub fn separator(&self) -> Element {
    glyph(&self.part(InputOtpPart::Separator), icons::MINUS.into(), false)
  }

  /// The cells, with a separator after each group but the last: what the default renderer lays over the input.
  pub fn cells(&self) -> Row {
    let mut row = Row::new()
      .part(&self.part(InputOtpPart::Root))
      .align_items(Alignment::Center)
      // The pointer is the input's, under the cells: a click on any of them focuses it.
      .pointer_events_none();
    for index in 0..self.length {
      row = row.child(self.cell(index));
      if self.separators.contains(&(index + 1)) {
        row = row.child(self.separator());
      }
    }
    row
  }

  /// How the default renderer builds a code: the input, and its cells over it.
  pub fn render(self) -> Element {
    Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .child(self.input())
      .child(self.cells())
      .into()
  }
}

impl Themed for InputOtp {
  const NAME: &'static str = "input_otp";
  type Renderer = InputOtpRenderer;

  /// The design's code (`pencil/lurq-design-system.pen`, Input OTP): cells 36 wide and 40 high, 6 apart, each the
  /// background's fill in an edge of 1 in the color of an input's, `radius-md` round, its character in the mono
  /// type at 16 in the foreground. The cell the focus is on has a line of 2 inside it in the ring's color, and
  /// while it is empty a caret 1.5 wide and 18 high in the foreground. Groups are parted by a dash 16 across in
  /// the muted foreground. Small cells are 32 by 36 with type of 14, large ones 40 by 44 with type of 18. The edges
  /// say a judgment as an input's do; a disabled code is half opaque on the muted fill, a read-only one on the
  /// muted fill in the color of a border.
  fn recipe() -> Recipe {
    use InputOtpPart::{Caret, Cell, Root, Separator, Value};
    use PaletteColor::{Border, Foreground, Muted, MutedForeground, Ring};
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(6.0))
      .base(Cell, |s| text_box::box_style(s).width(36.0).height(40.0).padding_x(0.0))
      .base(Value, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(16.0)
          .foreground(Foreground)
      })
      .base(Caret, |s| s.width(1.5).height(18.0).background(Foreground))
      .base(Separator, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .state(InputOtpState::Active, Cell, |s| s.outline(text_box::ring(Ring)))
      .state(TextBoxState::ReadOnly, Cell, |s| {
        s.background(Muted).border(text_box::edge(Border))
      })
      .state(TextBoxState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(TextBoxState::Disabled, Cell, |s| {
        s.background(Muted).cursor(CursorIcon::Default)
      });
    let recipe = [
      (TextBoxSize::Sm, 32.0, 36.0, 14.0, 16.0),
      (TextBoxSize::Lg, 40.0, 44.0, 18.0, 20.0),
    ]
    .into_iter()
    .fold(recipe, |recipe, (size, width, height, font_size, caret)| {
      recipe
        .size(size, Cell, |s| s.width(width).height(height))
        .size(size, Value, |s| s.font_size(font_size))
        .size(size, Caret, |s| s.height(caret))
    });
    // A judgment colors every edge, and the ring of the cell the focus is on.
    [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
      .into_iter()
      .fold(recipe, |recipe, judgment| {
        let (state, color) = (TextBoxState::from(judgment), judgment.color());
        recipe
          .state(state, Cell, |s| s.border(text_box::edge(color.clone())))
          .when(Selector::new().state(state).state(InputOtpState::Active), Cell, |s| {
            s.outline(text_box::ring(color))
          })
      })
  }
}
