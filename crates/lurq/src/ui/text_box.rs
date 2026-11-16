//! What a box that is typed in or chosen from has, whatever is in it: an input, a textarea, and a select when it
//! is built. Its edge and fill, the focus as a line inside it, the color of its placeholder, the judgment of what
//! it holds, read only, its sizes and their type.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Case, Name, Recipe, Selector},
  shared::fading,
};
use crate::{
  animation::Transition,
  app::{
    events::TextInputEvent,
    theme::{BorderSize, ControlSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::TextInput,
  layout::text_style::TextStyle,
  node::{CursorIcon, Outline, TextColor, border::Border},
};

/// The sizes of a box that is typed in: the heights of the theme's controls, and the type of each. An input is
/// as high as its size; a textarea has the type of its size and is as high as its rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextBoxSize {
  /// 32 high, its text 13.
  Sm,
  /// 36 high, its text 14.
  #[default]
  Md,
  /// 40 high, its text 15.
  Lg,
}

names!(TextBoxSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// What the app says of what a control holds: its edge, and a field's description, are in the judgment's color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Judgment {
  /// It is wrong: the destructive color.
  Invalid,
  /// It is allowed, with a caveat: the warning color.
  Warning,
  /// It was checked and is right: the success color.
  Valid,
}

names!(Judgment {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
});

impl Judgment {
  /// The one judgment shown of the three an app may set: what is wrong over what is doubtful over what is right.
  pub fn of(invalid: bool, warning: bool, valid: bool) -> Option<Self> {
    if invalid {
      Some(Self::Invalid)
    } else if warning {
      Some(Self::Warning)
    } else if valid {
      Some(Self::Valid)
    } else {
      None
    }
  }

  /// The role of the color the judgment is drawn in.
  pub fn color(self) -> PaletteColor {
    match self {
      Self::Invalid => PaletteColor::Destructive,
      Self::Warning => PaletteColor::Warning,
      Self::Valid => PaletteColor::Success,
    }
  }
}

/// The states every box that is typed in or chosen from has, by the names its own enum gives them too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextBoxState {
  /// What it holds is wrong.
  Invalid,
  /// What it holds is allowed, with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It can be read, selected and copied, and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is in a fluid field: the field's label is inside it, over what it holds.
  Fluid,
}

names!(TextBoxState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  Fluid => "fluid",
});

impl From<Judgment> for TextBoxState {
  fn from(judgment: Judgment) -> Self {
    match judgment {
      Judgment::Invalid => Self::Invalid,
      Judgment::Warning => Self::Warning,
      Judgment::Valid => Self::Valid,
    }
  }
}

/// What a builder of a box keeps of its states: its size, the judgments the app set, read only and disabled.
#[derive(Clone, Debug)]
pub(crate) struct BoxStates {
  pub size: Name,
  pub invalid: bool,
  pub warning: bool,
  pub valid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether it is in a fluid field, which draws its label inside the box.
  pub fluid: bool,
  /// What the box is called to what reads the tree. A field gives it its label.
  pub label: Option<Arc<str>>,
  /// What the box is to what reads the tree, in place of a `textbox`: a search field's `searchbox`.
  pub role: Option<&'static str>,
}

impl Default for BoxStates {
  fn default() -> Self {
    Self {
      size: TextBoxSize::default().into(),
      invalid: false,
      warning: false,
      valid: false,
      read_only: false,
      disabled: false,
      fluid: false,
      label: None,
      role: None,
    }
  }
}

impl BoxStates {
  /// The judgment shown.
  pub fn judgment(&self) -> Option<Judgment> {
    Judgment::of(self.invalid, self.warning, self.valid)
  }

  /// The case the box is resolved in: its size, the app's own states, and the judgment, read only and disabled.
  pub fn case(&self, states: Vec<Name>) -> Case {
    let judgment = self.judgment();
    Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states,
    }
    .state(TextBoxState::Invalid, judgment == Some(Judgment::Invalid))
    .state(TextBoxState::Warning, judgment == Some(Judgment::Warning))
    .state(TextBoxState::Valid, judgment == Some(Judgment::Valid))
    .state(TextBoxState::ReadOnly, self.read_only)
    .state(TextBoxState::Disabled, self.disabled)
    .state(TextBoxState::Fluid, self.fluid)
  }

  /// Takes what the field it is in says: the field's label where the box has no name of its own, the field's
  /// judgment where the app judged the box itself in none, and the field's being disabled.
  pub fn in_field(&mut self, field: &super::InField) {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    if self.judgment().is_none() {
      self.invalid = field.judgment == Some(Judgment::Invalid);
      self.warning = field.judgment == Some(Judgment::Warning);
      self.valid = field.judgment == Some(Judgment::Valid);
    }
    self.disabled |= field.disabled;
    self.fluid |= field.fluid;
  }
}

/// The edge of a box: a line of 1 inside it.
pub(crate) fn edge(color: PaletteColor) -> Border {
  Border::inside(BorderSize::Sm, color)
}

/// The focus of a box: a line of 2 inside it, over its edge. An outline, so that the edge under the pointer, which
/// the engine lays over what the focus sets, does not take it away.
pub(crate) fn ring(color: PaletteColor) -> Outline {
  Outline::new(2.0, color).offset(-2.0)
}

/// The box at rest: as wide as its room, the background's fill in an edge of 1 in the color of an input's, the
/// medium radius, 12 from its edge to what is in it, the cursor of text. Its fill, ink, outline and edge fade.
pub(crate) fn box_style(style: PartStyle) -> PartStyle {
  fading(
    style
      .width(Length::Fill)
      .padding_x(12.0)
      .radius(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(edge(PaletteColor::Input))
      .cursor(CursorIcon::Text),
  )
  .transition(PartTransition::new(Transition::border_color(), MotionDuration::Fast))
}

/// The rules every box has: on `control`, the box, the ring's color under the pointer and a line of 2 inside it in
/// the focus; the muted fill and the color of a border when it is read only; the muted fill when it is disabled,
/// and `whole` at half its opacity; each judgment's color in its edge, under the pointer and in the focus. The
/// placeholder is in the muted foreground.
pub(crate) fn box_rules(
  recipe: Recipe,
  whole: impl Into<Name>,
  control: impl Into<Name>,
  placeholder: impl Into<Name>,
) -> Recipe {
  let (whole, control) = (whole.into(), control.into());
  let recipe = recipe
    .base(placeholder, |s| s.foreground(PaletteColor::MutedForeground))
    .hovered(control, |s| s.border(edge(PaletteColor::Ring)))
    .focus_visible(control, |s| s.outline(ring(PaletteColor::Ring)))
    .state(TextBoxState::ReadOnly, control, |s| {
      s.background(PaletteColor::Muted).border(edge(PaletteColor::Border))
    })
    .state(TextBoxState::Disabled, whole, |s| s.opacity(OpacityLevel::Disabled))
    .state(TextBoxState::Disabled, control, |s| {
      s.background(PaletteColor::Muted).cursor(CursorIcon::Default)
    });
  // A judgment keeps its color under the pointer and in the focus, where the focus is thicker.
  [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
    .into_iter()
    .fold(recipe, |recipe, judgment| {
      let (state, color) = (TextBoxState::from(judgment), judgment.color());
      recipe
        .state(state, control, |s| s.border(edge(color.clone())))
        .when(Selector::new().state(state).hovered(), control, |s| {
          s.border(edge(color.clone()))
        })
        .when(Selector::new().state(state).focus_visible(), control, |s| {
          s.outline(ring(color))
        })
    })
}

/// How far the top of a fluid box is from what it holds: the 10 over the field's label, the label's line of 15 and
/// the 4 under it.
pub(crate) const FLUID_TOP: f32 = 29.0;
/// The padding of a fluid box under what it holds, and over the field's label.
pub(crate) const FLUID_PAD: f32 = 10.0;
/// How high a fluid box of one line is: the board's 56.
pub(crate) const FLUID_HEIGHT: f32 = 56.0;

/// A line of 1 along the bottom of a fluid box, and no edge on its other sides.
pub(crate) fn fluid_line(style: PartStyle, color: PaletteColor) -> PartStyle {
  style
    .border(Border::inside(0.0, color.clone()))
    .underline(Border::inside(BorderSize::Sm, color))
}

/// The box of a fluid field (`pencil/lurq-design-system.pen`, Fluid Field `imRnW`), over the box at rest: on the
/// muted fill, round at its top and square at its bottom, with a line of 1 in the foreground along its bottom and no
/// edge, what it holds 29 under its top, where the field's label stands, and 10 over its bottom.
pub(crate) fn fluid_box(style: PartStyle) -> PartStyle {
  fluid_line(style, PaletteColor::Foreground)
    .padding_top(FLUID_TOP)
    .padding_bottom(FLUID_PAD)
    .corners(RadiusSize::Md, RadiusSize::Md, 0.0, 0.0)
    .background(PaletteColor::Muted)
}

/// The rules of a box in a fluid field, laid over [`box_rules`]: `whole` as wide as the field, and `control`, the
/// box, the fluid one ([`fluid_box`]), as high as what it holds and at least 56. Where the board is silent the line
/// says what the edge of a box says: the ring's color under the pointer, the border's when it is read only, the
/// judgment's color; the focus is the box's line of 2 inside it.
pub(crate) fn fluid_rules(recipe: Recipe, whole: impl Into<Name>, control: impl Into<Name>) -> Recipe {
  let (whole, control) = (whole.into(), control.into());
  let fluid = || Selector::new().state(TextBoxState::Fluid);
  let recipe = recipe
    .state(TextBoxState::Fluid, whole, |s| s.width(Length::Fill))
    .state(TextBoxState::Fluid, control, |s| {
      fluid_box(s).height(Length::Auto).min_height(FLUID_HEIGHT)
    })
    .when(fluid().hovered(), control, |s| fluid_line(s, PaletteColor::Ring))
    .when(fluid().state(TextBoxState::ReadOnly), control, |s| {
      fluid_line(s, PaletteColor::Border)
    })
    .when(fluid().state(TextBoxState::ReadOnly).hovered(), control, |s| {
      fluid_line(s, PaletteColor::Border)
    });
  [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
    .into_iter()
    .fold(recipe, |recipe, judgment| {
      let (state, color) = (TextBoxState::from(judgment), judgment.color());
      recipe
        .when(fluid().state(state), control, |s| fluid_line(s, color.clone()))
        .when(fluid().state(state).hovered(), control, |s| fluid_line(s, color))
    })
}

/// The type of the sizes on `part`: 13 small and 15 large, the body's 14 being the medium one's. Where `heights`,
/// the part is as high as a control of its size.
pub(crate) fn box_sizes(recipe: Recipe, part: impl Into<Name>, heights: bool) -> Recipe {
  let part = part.into();
  [
    (TextBoxSize::Sm, ControlSize::Sm, 13.0),
    (TextBoxSize::Lg, ControlSize::Lg, 15.0),
  ]
  .into_iter()
  .fold(recipe, |recipe, (size, height, font_size)| {
    recipe.size(size, part, |s| {
      let s = s.font_size(font_size);
      if heights { s.height(height) } else { s }
    })
  })
}

/// The text of a part as a text input paints it: the part's type, in the color of its foreground. A text input does
/// not take the foreground of what it is in, so the color is named; a part with no foreground keeps the type's.
pub(crate) fn ink(theme: &Theme, style: &PartStyle) -> TextStyle {
  let mut text = style.text.resolve(theme);
  match &style.foreground {
    Some(TextColor::Color(color)) => text.color = *color,
    Some(TextColor::Palette(role)) => text.color = theme.palette_color(role.clone()),
    None => {}
  }
  text
}

/// What is called with an edit before it is made.
pub(crate) type EditHandler = Arc<dyn Fn(&TextInputEvent) + Send + Sync>;

/// The engine's text input as a box of the design system: a stop of Tab wherever it is, a `textbox` to what reads
/// the tree, called by its label and saying whether it is invalid and read only. A read-only one refuses every
/// edit, and its handlers are not called; the others are called with each edit before it is made.
pub(crate) fn text_box(control: TextInput, states: &BoxStates, handlers: Vec<EditHandler>) -> TextInput {
  text_box_as(control, states, handlers, "textbox")
}

/// [`text_box`], saying to what reads the tree that it is a `role` in place of a `textbox`: a `spinbutton`.
pub(crate) fn text_box_as(
  control: TextInput,
  states: &BoxStates,
  handlers: Vec<EditHandler>,
  role: &'static str,
) -> TextInput {
  let invalid = states.judgment() == Some(Judgment::Invalid);
  let mut control = control
    // A text input is a stop of Tab only in a form or with an index of its own; a control of the design system is
    // one wherever it is.
    .tab_index(0)
    .disabled(states.disabled)
    .describe("a11y_role", states.role.unwrap_or("textbox"))
    .describe("a11y_role", role)
    .describe("a11y_invalid", if invalid { "true" } else { "false" });
  if states.read_only {
    control = control.describe("a11y_readonly", "true");
  }
  if let Some(label) = &states.label {
    control = control.describe("a11y_name", label.clone());
  }
  let read_only = states.read_only;
  if read_only || !handlers.is_empty() {
    control = control.on_input(move |event: TextInputEvent| {
      if read_only {
        event.prevent_default();
        return;
      }
      for handler in &handlers {
        handler(&event);
      }
    });
  }
  control
}
