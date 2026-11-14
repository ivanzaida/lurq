//! A key of the keyboard, the keys of a shortcut pressed together, or steps pressed one after another, read and
//! not operated; and a short text of code in a run of text, which is the same kind of thing.

use std::sync::Arc;

use super::{
  names,
  part_style::PartStyle,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    Accelerator,
    theme::{BorderSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  layout::Alignment,
  node::{Element, border::Border as Line},
};

/// How a key is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KbdVariant {
  /// Each key in a box of its own: on a muted fill, with an edge.
  #[default]
  Default,
  /// The keys of a step written as one text, with no box, in the muted color: "Ctrl+K", or "⌘K" on a Mac. What a
  /// menu writes at the end of an item.
  Plain,
}

names!(KbdVariant {
  Default => "default",
  Plain => "plain",
});

/// How tall a key is: 16, 20 or 24.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KbdSize {
  /// 16 high, at 10: a key in a tooltip, or in a line of small text.
  Sm,
  /// 20 high, at 11.
  #[default]
  Md,
  /// 24 high, at 12: a list of the shortcuts of an app.
  Lg,
}

names!(KbdSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// Whose names a key is written with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyPlatform {
  /// In words, as Windows and Linux write them: "Ctrl", "Alt", "Shift", "Enter". `Mod` is Ctrl.
  #[default]
  Standard,
  /// With a Mac's glyphs: ⌃ ⌥ ⇧ ⌘, ↩ ⌫ ⇥, the modifiers of a shortcut first and in that order, as a Mac's menus
  /// write them. `Mod` is ⌘.
  Mac,
}

impl KeyPlatform {
  /// The platform the app is built for: a Mac's glyphs on macOS and iOS, words elsewhere.
  pub fn current() -> Self {
    if cfg!(any(target_os = "macos", target_os = "ios")) {
      Self::Mac
    } else {
      Self::Standard
    }
  }
}

/// The parts of a key, a shortcut or a sequence a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KbdPart {
  /// What holds the keys, in a row.
  Root,
  /// One key: its box, its fill and its edge.
  Key,
  /// The text on a key; in [`KbdVariant::Plain`], the text of a step.
  Label,
  /// What stands between the steps of a sequence: "then".
  Separator,
}

names!(KbdPart {
  Root => "root",
  Key => "key",
  Label => "label",
  Separator => "separator",
});

/// The states of a key a recipe styles. It has nothing the pointer or the keyboard does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KbdState {
  /// It is a sequence of steps pressed one after another.
  Sequence,
  /// It is written with a Mac's glyphs.
  Mac,
}

names!(KbdState {
  Sequence => "sequence",
  Mac => "mac",
});

/// One key as it is drawn: what is on it, and what it is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KbdKey {
  /// What is drawn on the key: "Ctrl", "⌘", "K".
  pub label: Arc<str>,
  /// What it is called in words: "Ctrl", "Cmd", "K".
  pub name: Arc<str>,
  /// Whether it is a modifier: Ctrl, Alt, Shift, Meta.
  pub modifier: bool,
}

/// What builds the nodes of a key: a theme's in place of the default one.
pub type KbdRenderer = Arc<dyn Fn(KbdView) -> Element + Send + Sync>;

/// A key of the keyboard, a shortcut, or a sequence of them: what a person presses to do something.
///
/// ```
/// use lurq::ui::{Kbd, KbdSize, KeyPlatform};
///
/// let escape = Kbd::new("Esc");
/// // A key each: "Ctrl" and "K". `Mod` is Ctrl, and ⌘ on a Mac.
/// let search = Kbd::shortcut("Mod+K").platform(KeyPlatform::current());
/// // ⇧ ⌘ P on a Mac: the modifiers first, in a Mac's order.
/// let palette = Kbd::shortcut("Ctrl+Shift+P").platform(KeyPlatform::Mac);
/// // Pressed one after another: G, then D.
/// let dashboard = Kbd::sequence(["G", "D"]);
/// let chord = Kbd::sequence(["Ctrl+K", "Ctrl+S"]).size(KbdSize::Lg);
/// ```
///
/// Keys are named as lurq names them in a [`KeyboardEvent`](crate::app::events::KeyboardEvent) (`ArrowUp`,
/// `Escape`, `Backspace`), or as a person writes them (`Up` is written as it is, `Esc`, `Del`), in any case. A
/// letter is drawn in capitals. In a shortcut the keys are parted by `+`, and a `+` after another, or alone, is the
/// key of that name: `Ctrl++`.
///
/// A key is read and not operated: it takes no press, no focus and no stop of Tab. To what reads the tree it is one
/// text, called by its keys in words: "Ctrl+K", "G then D".
pub struct Kbd {
  steps: Vec<Vec<Arc<str>>>,
  platform: KeyPlatform,
  variant: Name,
  size: Name,
  separator: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Kbd {
  fn of(steps: Vec<Vec<Arc<str>>>) -> Self {
    Self {
      steps,
      platform: KeyPlatform::default(),
      variant: KbdVariant::default().into(),
      size: KbdSize::default().into(),
      separator: Arc::from("then"),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// One key: "Ctrl", "Esc", "Page Up". A `+` in it is drawn as it is.
  pub fn new(key: &str) -> Self {
    Self::of(vec![vec![Arc::from(key)]])
  }

  /// The keys of a shortcut, pressed together: "Ctrl+K", a key each.
  pub fn shortcut(keys: &str) -> Self {
    Self::of(vec![split(keys)])
  }

  /// Steps pressed one after another, each a key or a shortcut: `["G", "D"]`, `["Ctrl+K", "Ctrl+S"]`.
  pub fn sequence<S: AsRef<str>>(steps: impl IntoIterator<Item = S>) -> Self {
    Self::of(steps.into_iter().map(|step| split(step.as_ref())).collect())
  }

  /// The keys of an [`Accelerator`] of the app's menu: its modifiers, then its key.
  pub fn accelerator(accelerator: &Accelerator) -> Self {
    let modifiers = accelerator.modifiers;
    let mut keys: Vec<Arc<str>> = [
      (modifiers.ctrl, "Ctrl"),
      (modifiers.alt, "Alt"),
      (modifiers.shift, "Shift"),
      (modifiers.meta, "Meta"),
    ]
    .into_iter()
    .filter(|(held, _)| *held)
    .map(|(_, name)| Arc::from(name))
    .collect();
    keys.push(accelerator.key.clone());
    Self::of(vec![keys])
  }

  /// Whose names the keys are written with: words unless told. [`KeyPlatform::current`] is the platform the app
  /// runs on.
  pub fn platform(mut self, platform: KeyPlatform) -> Self {
    self.platform = platform;
    self
  }

  /// A [`KbdVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`KbdSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What stands between the steps of a sequence, and what it is called by between them: "then" unless told.
  pub fn separator(mut self, separator: &str) -> Self {
    self.separator = Arc::from(separator);
    self
  }

  /// Puts the keys in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for these keys: what the component they are in says of them.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the keys in `theme`. Converting them into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(KbdState::Sequence, self.steps.len() > 1)
    .state(KbdState::Mac, self.platform == KeyPlatform::Mac);
    let platform = self.platform;
    let view = KbdView {
      steps: self.steps.iter().map(|step| keys_of(step, platform)).collect(),
      platform,
      variant: self.variant,
      size: self.size,
      separator: self.separator,
      recipe: theme.resolve::<Kbd>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Kbd>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Kbd> for Element {
  fn from(kbd: Kbd) -> Self {
    kbd.build(&current_theme())
  }
}

/// The keys of a shortcut as written, parted by `+`: a `+` where a key is to start is the key of that name.
fn split(keys: &str) -> Vec<Arc<str>> {
  let mut parted = Vec::new();
  let mut key = String::new();
  for character in keys.chars() {
    if character == '+' && !key.trim().is_empty() {
      parted.push(Arc::from(key.trim()));
      key.clear();
    } else if character == '+' {
      key = "+".to_owned();
    } else {
      key.push(character);
    }
  }
  if !key.trim().is_empty() {
    parted.push(Arc::from(key.trim()));
  } else if key == " " {
    parted.push(Arc::from(" "));
  }
  parted
}

/// A modifier: where a Mac puts it among the others, its word, its word on a Mac, and its glyph.
fn modifier(folded: &str, platform: KeyPlatform) -> Option<(u8, &'static str, &'static str, &'static str)> {
  Some(match folded {
    "ctrl" | "control" => (0, "Ctrl", "Ctrl", "⌃"),
    "alt" | "option" | "opt" => (1, "Alt", "Option", "⌥"),
    "shift" => (2, "Shift", "Shift", "⇧"),
    "meta" => (3, "Meta", "Cmd", "⌘"),
    "cmd" | "command" => (3, "Cmd", "Cmd", "⌘"),
    "super" => (3, "Super", "Cmd", "⌘"),
    "win" | "windows" => (3, "Win", "Cmd", "⌘"),
    "mod" if platform == KeyPlatform::Mac => (3, "Ctrl", "Cmd", "⌘"),
    "mod" => (0, "Ctrl", "Cmd", "⌘"),
    _ => return None,
  })
}

/// A key that is not a modifier, by the name lurq gives it or a person writes: its label in words, its label on a
/// Mac, and its word.
fn named(folded: &str) -> Option<(&'static str, &'static str, &'static str)> {
  Some(match folded {
    "enter" | "return" => ("Enter", "↩", "Enter"),
    "escape" | "esc" => ("Esc", "Esc", "Esc"),
    "backspace" => ("Backspace", "⌫", "Backspace"),
    "delete" | "del" => ("Del", "⌦", "Delete"),
    "tab" => ("Tab", "⇥", "Tab"),
    "space" | "spacebar" => ("Space", "Space", "Space"),
    "arrowup" => ("↑", "↑", "Up"),
    "arrowdown" => ("↓", "↓", "Down"),
    "arrowleft" => ("←", "←", "Left"),
    "arrowright" => ("→", "→", "Right"),
    "pageup" => ("Page Up", "⇞", "Page Up"),
    "pagedown" => ("Page Down", "⇟", "Page Down"),
    "home" => ("Home", "↖", "Home"),
    "end" => ("End", "↘", "End"),
    "capslock" => ("Caps Lock", "⇪", "Caps Lock"),
    _ => return None,
  })
}

/// The keys of one step as they are drawn on `platform`: on a Mac its modifiers first, in a Mac's order.
fn keys_of(step: &[Arc<str>], platform: KeyPlatform) -> Vec<KbdKey> {
  let mac = platform == KeyPlatform::Mac;
  let mut keys: Vec<(u8, KbdKey)> = step
    .iter()
    .map(|written| {
      if written.as_ref() == " " {
        let space: Arc<str> = Arc::from("Space");
        return (
          u8::MAX,
          KbdKey {
            label: space.clone(),
            name: space,
            modifier: false,
          },
        );
      }
      let folded: String = written
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
      if let Some((rank, word, mac_word, glyph)) = modifier(&folded, platform) {
        let (label, name) = if mac { (glyph, mac_word) } else { (word, word) };
        return (
          rank,
          KbdKey {
            label: Arc::from(label),
            name: Arc::from(name),
            modifier: true,
          },
        );
      }
      if let Some((word, mac_label, name)) = named(&folded) {
        let label = if mac { mac_label } else { word };
        let name = if mac && name == "Enter" { "Return" } else { name };
        return (
          u8::MAX,
          KbdKey {
            label: Arc::from(label),
            name: Arc::from(name),
            modifier: false,
          },
        );
      }
      // A letter is drawn in capitals; anything else as it is written.
      let label: Arc<str> = if written.chars().count() == 1 {
        Arc::from(written.to_uppercase())
      } else {
        written.clone()
      };
      (
        u8::MAX,
        KbdKey {
          label: label.clone(),
          name: label,
          modifier: false,
        },
      )
    })
    .collect();
  if mac {
    // Stable: what is not a modifier keeps its place after them.
    keys.sort_by_key(|(rank, _)| *rank);
  }
  keys.into_iter().map(|(_, key)| key).collect()
}

/// Keys as a renderer sees them: each step's keys, and the style of the parts.
pub struct KbdView {
  /// The steps pressed one after another, each the keys pressed together, as they are drawn.
  pub steps: Vec<Vec<KbdKey>>,
  pub platform: KeyPlatform,
  pub variant: Name,
  pub size: Name,
  /// What stands between steps.
  pub separator: Arc<str>,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl KbdView {
  /// The style of a part, as these keys are to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the keys are drawn as text, with no box ([`KbdVariant::Plain`]).
  pub fn plain(&self) -> bool {
    self.variant == Name::from(KbdVariant::Plain)
  }

  /// The keys of one step written as one text: "Ctrl+K" in words, "⌘K" on a Mac.
  pub fn written(&self, step: &[KbdKey]) -> String {
    let join = if self.platform == KeyPlatform::Mac { "" } else { "+" };
    step.iter().map(|key| key.label.as_ref()).collect::<Vec<_>>().join(join)
  }

  /// What the keys are called to what reads the tree: their words, parted by `+` within a step and by the
  /// separator between steps. "Ctrl+K", "Cmd+Shift+P", "G then D".
  pub fn name(&self) -> String {
    self
      .steps
      .iter()
      .map(|step| step.iter().map(|key| key.name.as_ref()).collect::<Vec<_>>().join("+"))
      .collect::<Vec<_>>()
      .join(&format!(" {} ", self.separator))
  }

  /// The root in its style, with nothing in it yet: one text to what reads the tree, called by [`name`](Self::name).
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(KbdPart::Root))
      .describe("a11y_role", "text")
      .describe("a11y_name", self.name());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// A text in the style of the label.
  pub fn label(&self, text: &str) -> Row {
    let part = self.part(KbdPart::Label);
    Row::new().part(&part).child(part.text(text).nowrap())
  }

  /// One key in its box.
  pub fn key_cap(&self, key: &KbdKey) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(KbdPart::Key))
      .child(self.label(&key.label))
  }

  /// What stands between two steps.
  pub fn between(&self) -> Row {
    let part = self.part(KbdPart::Separator);
    Row::new().part(&part).child(part.text(&self.separator).nowrap())
  }

  /// How the default renderer builds keys: each key in its box, or each step as one text where they are plain, and
  /// the separator between steps.
  pub fn render(self) -> Element {
    let mut root = self.root();
    for (index, step) in self.steps.iter().enumerate() {
      if index > 0 {
        root = root.child(self.between());
      }
      if self.plain() {
        root = root.child(self.label(&self.written(step)));
      } else {
        for key in step {
          root = root.child(self.key_cap(key));
        }
      }
    }
    root.into()
  }
}

impl Themed for Kbd {
  const NAME: &'static str = "kbd";
  type Renderer = KbdRenderer;

  /// The design's key (`pencil/lurq-design-system.pen`: Kbd, and the keys of the search field, the command
  /// palette, the shortcut recorder and the tablet's list of shortcuts): 20 high, padded 6, on `Muted` with an edge
  /// of `Border` and the radius `Sm`, its text 11 in the family of code in `MutedForeground`. The keys of a shortcut
  /// are 4 apart, with nothing between them. Small is 16, padded 4, at 10, as the tooltip's key; large is 24, padded
  /// 7, at 12 and 6 apart, as the list of shortcuts. Plain is the menu's: the text alone.
  fn recipe() -> Recipe {
    use KbdPart::{Key, Label, Root, Separator};
    use PaletteColor::*;
    let mut recipe = Recipe::new()
      .base(Root, |s| s.gap(4.0))
      .base(Key, |s| {
        s.height(20.0)
          .padding_x(6.0)
          .radius(RadiusSize::Sm)
          .background(Muted)
          .foreground(MutedForeground)
          .border(Line::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .line_height(1.0)
          .foreground(MutedForeground)
      })
      .base(Separator, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(11.0)
          .foreground(MutedForeground)
      });
    for (size, height, padding, font, gap) in [(KbdSize::Sm, 16.0, 4.0, 10.0, 4.0), (KbdSize::Lg, 24.0, 7.0, 12.0, 6.0)]
    {
      recipe = recipe
        .size(size, Root, |s| s.gap(gap))
        .size(size, Key, |s| s.height(height).padding_x(padding))
        .size(size, Label, |s| s.font_size(font))
        .size(size, Separator, |s| s.font_size(font));
    }
    recipe
  }
}

/// The parts of a text of code a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InlineCodePart {
  /// The muted fill around the text.
  Root,
  /// The text, in the family of code.
  Label,
}

names!(InlineCodePart {
  Root => "root",
  Label => "label",
});

/// What builds the nodes of a text of code: a theme's in place of the default one.
pub type InlineCodeRenderer = Arc<dyn Fn(InlineCodeView) -> Element + Send + Sync>;

/// A short text of code in a run of text: a call, a name, a path, a command.
///
/// ```
/// use lurq::ui::InlineCode;
///
/// let call = InlineCode::new("ctx.theme()");
/// ```
///
/// It is read and not operated, and is not cut: it is as wide as its text. To what reads the tree it is `code`,
/// called by its text.
pub struct InlineCode {
  text: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InlineCode {
  /// A text of code that says `text`.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this text.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
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

  /// Builds the text in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    };
    let view = InlineCodeView {
      text: self.text,
      recipe: theme.resolve::<InlineCode>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<InlineCode>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<InlineCode> for Element {
  fn from(code: InlineCode) -> Self {
    code.build(&current_theme())
  }
}

/// A text of code as a renderer sees it.
pub struct InlineCodeView {
  pub text: Arc<str>,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InlineCodeView {
  /// The style of a part, as the text is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The fill in its style, with nothing in it yet: `code` to what reads the tree, called by its text.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(InlineCodePart::Root))
      .describe("a11y_role", "code")
      .describe("a11y_name", self.text.clone());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds it: the text on its fill.
  pub fn render(self) -> Element {
    let part = self.part(InlineCodePart::Label);
    self.root().child(part.text(&self.text).nowrap()).into()
  }
}

impl Themed for InlineCode {
  const NAME: &'static str = "inline_code";
  type Renderer = InlineCodeRenderer;

  /// The design's inline code (`pencil/lurq-design-system.pen`: Inline Code, in the prose of Typography and the
  /// Markdown document): 13 in the family of code in `Foreground`, on `Muted` with the radius `Sm`, padded 5 across
  /// and 1 above and below.
  fn recipe() -> Recipe {
    use InlineCodePart::{Label, Root};
    Recipe::new()
      .base(Root, |s| {
        s.padding_x(5.0)
          .padding_y(1.0)
          .radius(RadiusSize::Sm)
          .background(PaletteColor::Muted)
          .foreground(PaletteColor::Foreground)
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(13.0)
          .foreground(PaletteColor::Foreground)
      })
  }
}
