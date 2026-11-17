//! An editable: text that is read where it stands and edited in place.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons,
  input::{Input, InputPart, InputSize},
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Row, TextInputController},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, NodeUpdate},
};

/// The parts of an editable a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditablePart {
  /// The reading form: the row of the text and the pencil, which is pressed to edit.
  Root,
  /// The text as it is read.
  Label,
  /// What the reading form shows while the text is empty: laid over the label.
  Placeholder,
  /// The pencil that says the text can be edited.
  Icon,
  /// The editing form: the row of the field and its two buttons.
  Editing,
  /// What the editable says of its field: laid over an input's root and its control.
  Field,
  /// What the editable says of the button that keeps the edit: laid over a button's root.
  Confirm,
  /// What the editable says of the button that drops the edit: laid over a button's root.
  Cancel,
}

names!(EditablePart {
  Root => "root",
  Label => "label",
  Placeholder => "placeholder",
  Icon => "icon",
  Editing => "editing",
  Field => "field",
  Confirm => "confirm",
  Cancel => "cancel",
});

/// The states of an editable a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditableState {
  /// It is being edited: the field is shown in place of the reading form.
  Editing,
  /// It holds no text: the reading form shows the placeholder.
  Empty,
  /// What is typed is wrong: the field's edge is in the destructive color.
  Invalid,
  /// It is read and cannot be edited: no pencil, and a press does nothing.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(EditableState {
  Editing => "editing",
  Empty => "empty",
  Invalid => "invalid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// What builds an editable's nodes: a theme's in place of the default one.
pub type EditableRenderer = Arc<dyn Fn(EditableView) -> Element + Send + Sync>;

type Run = Arc<dyn Fn() + Send + Sync>;
type TextHandler = Arc<dyn Fn(&str) + Send + Sync>;
type FlagHandler = Arc<dyn Fn(bool) + Send + Sync>;

/// The text an editable holds: the app's signal, or what the app holds itself.
#[derive(Clone)]
enum Held {
  Bound(Signal<String>),
  Controlled(Arc<str>),
}

/// An editable: a text that is read where it stands (a title, a name, a cell) and edited in place. A press of
/// it, Enter or F2 puts a field where the text was, with the text selected; Enter keeps what was typed, and so
/// does leaving the field; Escape drops it.
///
/// ```
/// use lurq::{app::ctx::Ctx, core::Signal, node::Element, ui::Editable};
///
/// # fn build(ctx: &mut Ctx, title: Signal<String>) -> Element {
/// // Bound to the app's signal, which is set as an edit is kept.
/// Editable::new(title)
///   .placeholder("Untitled")
///   .label("Project name")
///   .on_change(|name| println!("renamed to {name}"))
///   .mount(ctx)
/// # }
/// // Held by the app: it is told what was typed, and the text shown is what the app answers with.
/// let held = Editable::controlled("Project Alpha", |name| println!("{name}"));
/// ```
///
/// The reading form is one stop of Tab, a button to what reads the tree. In the editing form the field has the
/// focus; its two buttons keep the edit and drop it, as Enter and Escape do, and may be left out
/// ([`actions`](Self::actions)). They are pressed by the pointer and are no stops of Tab: Tab in the field keeps
/// the edit and goes on to the control after the editable. When the edit ends by a key or a button the focus goes back
/// to the reading form; when it ends because the focus went elsewhere, the focus stays where it went. An edit that
/// changes nothing tells nothing.
///
/// Mounted ([`mount`](Self::mount)), it keeps whether it is being edited (unless the app's signal does:
/// [`editing`](Self::editing)) and what is typed. Built without, it is the still form of itself: the reading
/// form, or the editing one where the app's signal says so.
#[derive(Clone)]
pub struct Editable {
  value: Held,
  on_change: Vec<TextHandler>,
  on_cancel: Vec<Run>,
  editing: Option<Signal<bool>>,
  default_editing: bool,
  on_editing_change: Vec<FlagHandler>,
  placeholder: Option<Arc<str>>,
  label: Option<Arc<str>>,
  actions: bool,
  submit_on_blur: bool,
  invalid: bool,
  read_only: bool,
  disabled: bool,
  full_width: bool,
  confirm_label: Arc<str>,
  cancel_label: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Editable {
  fn of(value: Held) -> Self {
    Self {
      value,
      on_change: Vec::new(),
      on_cancel: Vec::new(),
      editing: None,
      default_editing: false,
      on_editing_change: Vec::new(),
      placeholder: None,
      label: None,
      actions: true,
      submit_on_blur: true,
      invalid: false,
      read_only: false,
      disabled: false,
      full_width: false,
      confirm_label: super::current_locale().common.save.clone(),
      cancel_label: super::current_locale().common.cancel.clone(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// An editable of the text `value` holds: an edit that is kept is written into the signal.
  pub fn new(value: Signal<String>) -> Self {
    Self::of(Held::Bound(value))
  }

  /// An editable of a text the app holds: an edit that is kept is told to `on_change`, and the text shown is
  /// what the app answers with.
  pub fn controlled(value: &str, on_change: impl Fn(&str) + Send + Sync + 'static) -> Self {
    Self::of(Held::Controlled(Arc::from(value))).on_change(on_change)
  }

  /// Called with the new text when an edit that changed it is kept: by Enter, the button, or leaving the field.
  /// Bound, the signal is set first. Not when the app sets the signal, and not for an edit that changed nothing.
  pub fn on_change(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called when an edit is dropped: by Escape or the button.
  pub fn on_cancel(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_cancel.push(Arc::new(handler));
    self
  }

  /// Whether it is being edited, in the app's signal: the app starts an edit by setting it, and it is set as a
  /// person starts one and as one ends. Without it a mounted editable keeps this itself.
  pub fn editing(mut self, editing: Signal<bool>) -> Self {
    self.editing = Some(editing);
    self
  }

  /// Whether an editable that keeps this itself is being edited as it is first shown: a name just made.
  pub fn default_editing(mut self, editing: bool) -> Self {
    self.default_editing = editing;
    self
  }

  /// Called when a person starts an edit, with `true`, and when one ends, with `false`: after the signal is
  /// set, and after `on_change` or `on_cancel`. Not when the app sets the signal.
  pub fn on_editing_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_editing_change.push(Arc::new(handler));
    self
  }

  /// What the reading form shows while the text is empty, and the field while nothing is typed.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the text is, to what reads the tree: "Project name". It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the editing form has the buttons that keep the edit and drop it: it has unless told. Without
  /// them Enter and Escape do.
  pub fn actions(mut self, actions: bool) -> Self {
    self.actions = actions;
    self
  }

  /// Whether leaving the field keeps what was typed: it does unless told, and then leaving drops it.
  pub fn submit_on_blur(mut self, submit: bool) -> Self {
    self.submit_on_blur = submit;
    self
  }

  /// Whether what is typed is wrong: the field's edge says so.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether the text is read and cannot be edited.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the editable is as wide as what holds it: its text is cut where it is longer, and its field fills
  /// the room its buttons leave.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// What the button that keeps the edit is called: "Save" by default.
  pub fn confirm_label(mut self, label: &str) -> Self {
    self.confirm_label = Arc::from(label);
    self
  }

  /// What the button that drops the edit is called: "Cancel" by default.
  pub fn cancel_label(mut self, label: &str) -> Self {
    self.cancel_label = Arc::from(label);
    self
  }

  /// Puts the editable in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this editable over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this editable.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the reading form as it is under the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the reading form. The field has this id with `-field` after it, and the buttons `-confirm` and
  /// `-cancel`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The text it holds.
  fn text(&self) -> Arc<str> {
    match &self.value {
      Held::Bound(signal) => Arc::from(signal.get()),
      Held::Controlled(value) => value.clone(),
    }
  }

  /// Puts the editable in the tree of `ctx`'s component, in the theme of what is being rendered: it keeps
  /// whether it is being edited and what is typed, puts the focus in its field as an edit starts, and back on
  /// its reading form as one ends by a key or a button.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<EditableHost>(EditableProps { editable: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the editable in `theme` once, with nothing kept: the reading form, or the editing one where the
  /// app's signal says so. Converting an editable into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let kept = Kept {
      editing: self
        .editing
        .clone()
        .unwrap_or_else(|| Signal::new(self.default_editing)),
      draft: Signal::new(self.text().to_string()),
      reading: ElementRef::new(),
      field: TextInputController::new(),
      return_focus: Arc::default(),
      started: Arc::default(),
    };
    self.view(theme, kept)
  }

  fn view(self, theme: &Theme, kept: Kept) -> Element {
    let text = self.text();
    let editing = kept.editing.get() && !self.read_only && !self.disabled;
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(EditableState::Editing, editing)
      .state(EditableState::Empty, text.is_empty())
      .state(EditableState::Invalid, self.invalid)
      .state(EditableState::ReadOnly, self.read_only)
      .state(EditableState::Disabled, self.disabled)
      .state(EditableState::FullWidth, self.full_width);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = EditableView {
      text,
      placeholder: self.placeholder,
      label: self.label,
      editing,
      actions: self.actions,
      invalid: self.invalid,
      read_only: self.read_only,
      disabled: self.disabled,
      full_width: self.full_width,
      recipe: theme.resolve::<Editable>(&case, Some(&self.rules)),
      held: self.value,
      on_change: self.on_change,
      on_cancel: self.on_cancel,
      on_editing_change: self.on_editing_change,
      submit_on_blur: self.submit_on_blur,
      confirm_label: self.confirm_label,
      cancel_label: self.cancel_label,
      forced: self.forced,
      kept,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Editable>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Editable> for Element {
  fn from(editable: Editable) -> Self {
    editable.build(&current_theme())
  }
}

/// What an editable keeps from one render to the next.
#[derive(Clone)]
struct Kept {
  editing: Signal<bool>,
  /// What is typed, until it is kept or dropped.
  draft: Signal<String>,
  reading: ElementRef,
  field: TextInputController,
  /// Set where an edit ends by a key or a button: the focus goes back to the reading form.
  return_focus: Arc<AtomicBool>,
  /// Set where a person starts an edit, which puts the text in the field as it does.
  started: Arc<AtomicBool>,
}

#[derive(Clone)]
struct EditableProps {
  editable: Editable,
}

/// The editable is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for EditableProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for EditableProps {}

struct EditableHost {
  /// Whether it is being edited, where the app keeps no signal of it.
  editing: Signal<bool>,
  draft: Signal<String>,
  reading: ElementRef,
  field: TextInputController,
  return_focus: Arc<AtomicBool>,
  started: Arc<AtomicBool>,
  was_editing: AtomicBool,
  rendered: AtomicBool,
}

impl Component for EditableHost {
  type Props = EditableProps;

  fn create(_: &mut Ctx) -> Self {
    Self {
      editing: Signal::new(false),
      draft: Signal::new(String::new()),
      reading: ElementRef::new(),
      field: TextInputController::new(),
      return_focus: Arc::default(),
      started: Arc::default(),
      was_editing: AtomicBool::new(false),
      rendered: AtomicBool::new(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let EditableProps { editable } = ctx.props::<EditableProps>().clone();
    let first = !self.rendered.swap(true, Ordering::Relaxed);
    if first && editable.editing.is_none() && editable.default_editing {
      self.editing.set(true);
    }
    let editing = editable.editing.clone().unwrap_or_else(|| self.editing.clone());
    let now = editing.get() && !editable.read_only && !editable.disabled;
    let was = self.was_editing.swap(now, Ordering::Relaxed);
    if now && !was {
      // An edit a person started holds the text already. One the app started by its signal, or that the
      // editable starts with, starts from the text as it is.
      if !self.started.swap(false, Ordering::Relaxed) {
        self.draft.set(editable.text().to_string());
      }
      // An editable whose signal says it is edited as it is first shown leaves the focus where it is.
      if !first || editable.editing.is_none() {
        ctx.focus(&self.field.element_ref());
      }
    } else if !now && was && self.return_focus.swap(false, Ordering::Relaxed) {
      ctx.focus(&self.reading);
    }
    let kept = Kept {
      editing,
      draft: self.draft.clone(),
      reading: self.reading.clone(),
      field: self.field.clone(),
      return_focus: self.return_focus.clone(),
      started: self.started.clone(),
    };
    editable.view(&current_theme(), kept)
  }
}

/// An editable as a renderer sees it: what it holds, whether it is being edited, the style of its parts, and
/// its two forms ready to be dressed.
pub struct EditableView {
  /// The text it holds.
  pub text: Arc<str>,
  pub placeholder: Option<Arc<str>>,
  pub label: Option<Arc<str>>,
  /// Whether the field is shown in place of the reading form.
  pub editing: bool,
  /// Whether the editing form has its two buttons.
  pub actions: bool,
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether it is as wide as what holds it.
  pub full_width: bool,
  /// The style of each part of this editable.
  pub recipe: ResolvedRecipe,
  held: Held,
  on_change: Vec<TextHandler>,
  on_cancel: Vec<Run>,
  on_editing_change: Vec<FlagHandler>,
  submit_on_blur: bool,
  confirm_label: Arc<str>,
  cancel_label: Arc<str>,
  forced: Option<Interaction>,
  kept: Kept,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl EditableView {
  /// The style of a part, as this editable is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// What starts an edit: the field holds the text, and the editable says it is being edited.
  fn start(&self) -> Run {
    let (kept, text, handlers) = (self.kept.clone(), self.text.clone(), self.on_editing_change.clone());
    Arc::new(move || {
      if kept.editing.get_untracked() {
        return;
      }
      kept.return_focus.store(false, Ordering::Relaxed);
      kept.started.store(true, Ordering::Relaxed);
      kept.draft.set(text.to_string());
      kept.editing.set(true);
      for handler in &handlers {
        handler(true);
      }
    })
  }

  /// What ends an edit: `keep` says whether what was typed is kept, and `by_person` whether a key or a button
  /// ended it, which gives the focus back to the reading form.
  fn end(&self) -> Arc<dyn Fn(bool, bool) + Send + Sync> {
    let (kept, held, text) = (self.kept.clone(), self.held.clone(), self.text.clone());
    let (changed, cancelled, turned) = (
      self.on_change.clone(),
      self.on_cancel.clone(),
      self.on_editing_change.clone(),
    );
    Arc::new(move |keep, by_person| {
      if !kept.editing.get_untracked() {
        return;
      }
      if by_person {
        kept.return_focus.store(true, Ordering::Relaxed);
      }
      kept.editing.set(false);
      if keep {
        let typed = kept.draft.get_untracked();
        if typed.as_str() != text.as_ref() {
          if let Held::Bound(signal) = &held {
            signal.set(typed.clone());
          }
          for handler in &changed {
            handler(&typed);
          }
        }
      } else {
        for handler in &cancelled {
          handler();
        }
      }
      for handler in &turned {
        handler(false);
      }
    })
  }

  /// The reading form: the text, or the placeholder while there is none, and the pencil, in a row that a press,
  /// Enter, Space or F2 starts an edit by. Read only, it is the text alone and takes no press.
  pub fn reading(&self) -> Element {
    let root = self.part(EditablePart::Root).forced(self.forced);
    let shown = if self.text.is_empty() {
      self.placeholder.clone().unwrap_or_default()
    } else {
      self.text.clone()
    };
    let label = if self.text.is_empty() {
      self.part(EditablePart::Placeholder)
    } else {
      self.part(EditablePart::Label)
    };
    let name = self.label.clone().unwrap_or_else(|| shown.clone());
    if self.read_only {
      let mut row = Row::new()
        .part(&root)
        .align_items(Alignment::Center)
        .describe("a11y_name", name)
        .child(one_line(&label, &shown));
      if let Some(id) = &self.id {
        row = row.id(id.clone());
      }
      if let Some(key) = &self.key {
        row = row.key(key.clone());
      }
      return row.into();
    }
    let (start, on_key) = (self.start(), self.start());
    let mut button = pressable(&root, self.disabled)
      .justify(crate::layout::layout_kind::Justify::Start)
      .align_items(Alignment::Center)
      .ref_element(self.kept.reading.clone())
      .describe("a11y_name", name)
      .describe("a11y_description", self.text.clone())
      .on_click(move |_| start())
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "F2" && !(event.ctrl || event.alt || event.meta || event.shift) {
          event.prevent_default();
          on_key();
        }
      })
      .child(one_line(&label, &shown))
      .child(glyph(&self.part(EditablePart::Icon), Name::from(icons::PENCIL), false));
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    if let Some(key) = &self.key {
      button = button.key(key.clone());
    }
    button.into()
  }

  /// A button of the editing form: its icon alone, which leaves the focus in the field. It is no stop of Tab:
  /// Enter and Escape do what the two do, and Tab in the field keeps the edit and goes on to what follows.
  fn action(
    &self,
    part: EditablePart,
    label: &str,
    icon: &str,
    variant: ButtonVariant,
    suffix: &str,
    run: Run,
  ) -> Element {
    let mut button = Button::new(label)
      .icon(icon)
      .icon_only(true)
      .variant(variant)
      .size(ButtonSize::Sm)
      .rules(&self.recipe.rules_for(part, ButtonPart::Root))
      .focuses(&self.kept.field.element_ref())
      .on_click(move |_| run());
    if let Some(id) = self.sub_id(suffix) {
      button = button.id(id);
    }
    let mut element = button.build(self.theme());
    NodeUpdate::tab_index(&mut *element.node, -1);
    element
  }

  /// The editing form: the field, with the text selected as it takes the focus, and the buttons that keep the
  /// edit and drop it. Enter keeps it and Escape drops it; leaving the field keeps it, unless the editable was
  /// told otherwise.
  pub fn editor(&self) -> Element {
    let kept = &self.kept;
    let rules = self
      .recipe
      .rules_for(EditablePart::Field, InputPart::Root)
      .extend(&self.recipe.rules_for(EditablePart::Field, InputPart::Control));
    let mut input = Input::new(kept.draft.clone())
      .size(InputSize::Sm)
      .invalid(self.invalid)
      .rules(&rules);
    if let Some(placeholder) = &self.placeholder {
      input = input.placeholder(placeholder);
    }
    if let Some(label) = self.label.as_ref().or(self.placeholder.as_ref()) {
      input = input.label(label);
    }
    if let Some(id) = self.sub_id("field") {
      input = input.id(id);
    }
    let view = input.view(self.theme());
    let (on_key, on_blur) = (self.end(), self.end());
    let submit_on_blur = self.submit_on_blur;
    let control = view
      .control()
      .controller(kept.field.clone())
      .select_on_focus(true)
      .on_blur(move || on_blur(submit_on_blur, false))
      .on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.composing || event.ctrl || event.alt || event.meta {
          return;
        }
        match event.key.as_str() {
          "Enter" => {
            event.prevent_default();
            on_key(true, true);
          }
          "Escape" => {
            event.prevent_default();
            on_key(false, true);
          }
          _ => {}
        }
      });
    let mut row = Row::new()
      .part(&self.part(EditablePart::Editing))
      .align_items(Alignment::Center)
      // The field gives way to its buttons in a room narrower than them all, and takes what they leave of a
      // room it fills.
      .child(
        Row::new()
          .flex_full(if self.full_width { 1.0 } else { 0.0 }, 1.0, None)
          .min_width(0.0)
          .child(view.render_with_end(control, None)),
      );
    if self.actions {
      let (confirm, cancel) = (self.end(), self.end());
      row = row
        .child(self.action(
          EditablePart::Confirm,
          &self.confirm_label,
          icons::CHECK,
          ButtonVariant::Outline,
          "confirm",
          Arc::new(move || confirm(true, true)),
        ))
        .child(self.action(
          EditablePart::Cancel,
          &self.cancel_label,
          icons::CLOSE,
          ButtonVariant::Ghost,
          "cancel",
          Arc::new(move || cancel(false, true)),
        ));
    }
    row.into()
  }

  /// How the default renderer builds an editable: the editing form while it is being edited, and the reading
  /// form otherwise.
  pub fn render(self) -> Element {
    if self.editing { self.editor() } else { self.reading() }
  }
}

impl Themed for Editable {
  const NAME: &'static str = "editable";
  type Renderer = EditableRenderer;

  /// The design's editable (`pencil/lurq-design-system.pen`, Editable): a row 32 high, its text 16 semibold in
  /// the foreground, and 8 after it a pencil of 14 in the muted foreground. Its placeholder is in the muted
  /// foreground. Edited, a field 160 wide and 32 high with text of 14, and 6 after it the two buttons, 32
  /// square: the outline one that keeps the edit and the ghost one that drops it, with icons of 16. Disabled,
  /// it is at half its opacity; read only, it has no pencil.
  fn recipe() -> Recipe {
    use EditablePart::*;
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.height(32.0)
            .gap(8.0)
            .max_width(Length::Fill)
            .radius(RadiusSize::Sm)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .state(EditableState::FullWidth, Root, |s| s.width(Length::Fill))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Placeholder, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
      .base(Icon, |s| fading(s.square(14.0).foreground(MutedForeground)))
      .focus_visible(Root, |s| s.focus_ring())
      .state(EditableState::ReadOnly, Root, |s| s.cursor(CursorIcon::Default))
      .state(EditableState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .base(Editing, |s| s.gap(6.0).max_width(Length::Fill))
      .state(EditableState::FullWidth, Editing, |s| s.width(Length::Fill))
      .base(Field, |s| s.width(160.0).height(32.0).font_size(14.0))
      .state(EditableState::FullWidth, Field, |s| s.width(Length::Fill))
      .base(Confirm, |s| s.square(32.0))
      .base(Cancel, |s| s.square(32.0))
  }
}
