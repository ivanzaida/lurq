//! The design system's components, and what a theme styles them with.
//!
//! A component here is built like any other in lurq, by a builder that goes
//! where an element goes: `Button::new("Save").variant(ButtonVariant::Primary)`.
//! What it looks like is the theme's to say, in three layers:
//!
//! - **Tokens**: the palette and the scales of `lurq::app::theme`.
//! - **Recipes** ([`Recipe`]): for each component, the style of each of its parts ([`PartStyle`]) in each case: always,
//!   for a variant, for a size, in a state. The design's recipes are the default; a theme lays rules over one, replaces
//!   one whole, or starts with none ([`ThemeDefinition::bare`](crate::app::theme::ThemeDefinition::bare)).
//! - **Renderers**: what builds a component's nodes from its recipe. A theme may give a component, or one part of it,
//!   another.
//!
//! A component being built takes the theme of the component being rendered
//! around it ([`current_theme`]), so it needs no context passed to it, and
//! what renders it renders again when the theme changes.

mod button;
mod button_group;
mod checkbox;
mod fab;
mod field;
mod icon;
pub mod icons;
mod input;
mod link;
mod part_style;
mod radio;
mod recipe;
mod segmented;
mod shared;
mod slider;
mod switch;
mod textarea;
pub(crate) mod theme;
mod toggle;
mod toggle_group;
mod tooltip;

pub use button::{
  Button, ButtonPart, ButtonPartRenderer, ButtonRenderer, ButtonShape, ButtonSize, ButtonState, ButtonVariant,
  ButtonView,
};
pub use button_group::{ButtonGroup, ButtonGroupPart, ButtonGroupVariant};
pub use checkbox::{Checkbox, CheckboxPart, CheckboxRenderer, CheckboxState, CheckboxView};
pub use fab::{Fab, FabPart, FabState};
pub use field::{Field, FieldPart, FieldRenderer, FieldState, FieldView};
pub use icon::{Icon, SPIN, SPIN_PERIOD};
pub use input::{Input, InputPart, InputRenderer, InputSize, InputState, InputView};
pub use link::{Link, LinkPart, LinkRenderer, LinkState, LinkView};
pub use part_style::{
  FontFamily, Length, Opacity, PartStyle, PartTransition, TextPatch, TransitionDuration, TransitionEasing,
};
pub use radio::{Radio, RadioGroup, RadioGroupPart, RadioPart, RadioRenderer, RadioState, RadioView};
pub use recipe::{Case, Interaction, Name, PartStyles, Recipe, ResolvedPart, ResolvedRecipe, Rule, Selector};
pub use segmented::{SegmentedControl, SegmentedControlPart, SegmentedControlState};
pub use shared::Choice;
pub use slider::{Slider, SliderPart, SliderRenderer, SliderState, SliderView};
pub use switch::{Switch, SwitchPart, SwitchRenderer, SwitchState, SwitchView};
pub use textarea::{
  TEXTAREA_MAX_ROWS, TEXTAREA_MIN_ROWS, Textarea, TextareaPart, TextareaRenderer, TextareaState, TextareaView,
};
pub use theme::{Themed, UiDefinition, current_breakpoint, current_theme, in_theme};
pub use toggle::{Toggle, TogglePart, ToggleRenderer, ToggleState, ToggleView};
pub use toggle_group::{ToggleGroup, ToggleGroupPart};
pub use tooltip::{
  TOOLTIP_DELAY, TOOLTIP_SKIP_DELAY, Tooltip, TooltipPart, TooltipRenderer, TooltipSide, TooltipState, TooltipView,
};

/// Gives an enum of names its names: `as_str`, and a conversion into
/// [`Name`], so that a variant, a size, a state or a part of a component is
/// accepted where a name is.
macro_rules! names {
  ($type:ident { $($variant:ident => $name:literal),* $(,)? }) => {
    impl $type {
      pub fn as_str(self) -> &'static str {
        match self {
          $( Self::$variant => $name, )*
        }
      }

      /// Every one of them, in order.
      pub fn all() -> &'static [$type] {
        &[ $( Self::$variant, )* ]
      }
    }

    impl From<$type> for $crate::ui::Name {
      fn from(name: $type) -> Self {
        $crate::ui::Name::new(name.as_str())
      }
    }
  };
}
pub(crate) use names;
