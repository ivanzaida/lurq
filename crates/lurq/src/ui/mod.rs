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

mod accordion;
mod avatar;
mod badge;
mod breadcrumb;
mod button;
mod button_group;
mod card;
mod checkbox;
mod chip;
mod collapsible;
mod combobox;
mod dialog;
mod fab;
mod field;
mod icon;
pub mod icons;
mod input;
mod link;
mod menu;
mod multi_select;
mod pagination;
mod part_style;
mod popover;
mod progress;
mod radio;
mod recipe;
mod segmented;
mod select;
mod separator;
mod shared;
mod skeleton;
mod slider;
mod spinner;
mod steps;
mod switch;
mod tabs;
mod tag;
mod text_box;
mod textarea;
pub(crate) mod theme;
mod toast;
mod toggle;
mod toggle_group;
mod tooltip;

pub use accordion::{Accordion, AccordionItem, AccordionPart, AccordionRenderer, AccordionState, AccordionView};
pub use avatar::{
  Avatar, AvatarGroup, AvatarGroupPart, AvatarGroupRenderer, AvatarGroupView, AvatarPart, AvatarPresence,
  AvatarRenderer, AvatarShape, AvatarSize, AvatarState, AvatarView,
};
pub use badge::{Badge, BadgePart, BadgeRenderer, BadgeShape, BadgeSize, BadgeState, BadgeVariant, BadgeView};
pub use breadcrumb::{
  Breadcrumb, BreadcrumbPart, BreadcrumbPartRenderer, BreadcrumbRenderer, BreadcrumbSeparator, BreadcrumbState,
  BreadcrumbStep, BreadcrumbView,
};
pub use button::{
  Button, ButtonPart, ButtonPartRenderer, ButtonRenderer, ButtonShape, ButtonSize, ButtonState, ButtonVariant,
  ButtonView,
};
pub use button_group::{ButtonGroup, ButtonGroupPart, ButtonGroupVariant};
pub use card::{Card, CardPart, CardRenderer, CardSize, CardState, CardVariant, CardView};
pub use checkbox::{
  Checkbox, CheckboxGroup, CheckboxGroupPart, CheckboxGroupState, CheckboxPart, CheckboxRenderer, CheckboxState,
  CheckboxView,
};
pub use chip::{Chip, ChipGroup, ChipGroupPart, ChipPart, ChipRenderer, ChipState, ChipView};
pub use collapsible::{Collapsible, CollapsiblePart, CollapsibleRenderer, CollapsibleState, CollapsibleView};
pub use combobox::{Combobox, ComboboxPart, ComboboxRenderer, ComboboxSize, ComboboxState, ComboboxView};
pub use dialog::{Dialog, DialogPart, DialogRenderer, DialogSize, DialogState, DialogView};
pub use fab::{Fab, FabPart, FabState};
pub use field::{Field, FieldControl, FieldPart, FieldRenderer, FieldState, FieldView, InField};
pub use icon::{Icon, SPIN, SPIN_PERIOD};
pub use input::{Input, InputPart, InputRenderer, InputSize, InputState, InputView};
pub use link::{Link, LinkPart, LinkRenderer, LinkState, LinkView};
pub use menu::{
  Menu, MenuEntry, MenuItem, MenuItemPart, MenuItemRenderer, MenuItemState, MenuItemView, MenuPart, MenuRenderer,
  MenuState, MenuView,
};
pub use multi_select::{MultiSelect, MultiSelectPart, MultiSelectRenderer, MultiSelectView};
pub use pagination::{PageItem, Pagination, PaginationPart, PaginationRenderer, PaginationState, PaginationView};
pub use part_style::{
  FontFamily, Length, Opacity, PartAnimation, PartStyle, PartTransition, TextPatch, TransitionDuration,
  TransitionEasing,
};
pub use popover::{Popover, PopoverPart, PopoverRenderer, PopoverView};
pub(crate) use progress::keyframes as progress_keyframes;
pub use progress::{
  PROGRESS_LEAD, PROGRESS_PERIOD, PROGRESS_SEGMENT, Progress, ProgressFormat, ProgressPart, ProgressRenderer,
  ProgressSize, ProgressState, ProgressVariant, ProgressView, RingProgress, RingProgressPart, RingProgressRenderer,
  RingProgressView,
};
pub use radio::{Radio, RadioGroup, RadioGroupPart, RadioGroupState, RadioPart, RadioRenderer, RadioState, RadioView};
pub use recipe::{Case, Interaction, Name, PartStyles, Recipe, ResolvedPart, ResolvedRecipe, Rule, Selector};
pub use segmented::{SegmentedControl, SegmentedControlPart, SegmentedControlState};
pub use select::{Select, SelectPart, SelectRenderer, SelectSize, SelectState, SelectView};
pub use separator::{Separator, SeparatorOrientation, SeparatorPart, SeparatorRenderer, SeparatorView};
pub use shared::{Choice, FADE_IN, FADE_OUT};
pub(crate) use skeleton::keyframes as skeleton_keyframes;
pub use skeleton::{
  SKELETON_PULSE, SKELETON_PULSE_PERIOD, SKELETON_SHIMMER, SKELETON_SHIMMER_PERIOD, Skeleton, SkeletonAnimation,
  SkeletonPart, SkeletonRenderer, SkeletonShape, SkeletonView,
};
pub use slider::{RangeSlider, Slider, SliderOrientation, SliderPart, SliderRenderer, SliderState, SliderView};
pub(crate) use spinner::keyframes as spinner_keyframes;
pub use spinner::{
  LoadingOverlay, LoadingOverlayPart, LoadingOverlayRenderer, LoadingOverlayState, LoadingOverlayView, SPINNER_BARS,
  SPINNER_BARS_PERIOD, SPINNER_DOTS, SPINNER_DOTS_PERIOD, Spinner, SpinnerKind, SpinnerPart, SpinnerRenderer,
  SpinnerSize, SpinnerState, SpinnerVariant, SpinnerView,
};
pub use steps::{
  CurrentStep, Step, StepView, Steps, StepsOrientation, StepsPart, StepsRenderer, StepsState, StepsView,
};
pub use switch::{Switch, SwitchPart, SwitchRenderer, SwitchState, SwitchView};
pub use tabs::{
  Tab, TabEntry, Tabs, TabsOrientation, TabsOverflow, TabsPart, TabsRenderer, TabsState, TabsVariant, TabsView,
};
pub use tag::{Tag, TagPart, TagRenderer, TagSize, TagState, TagVariant, TagView};
pub use text_box::{Judgment, TextBoxSize, TextBoxState};
pub use textarea::{
  TEXTAREA_MAX_ROWS, TEXTAREA_MIN_ROWS, Textarea, TextareaPart, TextareaRenderer, TextareaSize, TextareaState,
  TextareaView,
};
pub use theme::{Themed, UiDefinition, current_breakpoint, current_theme, in_theme};
pub(crate) use toast::keyframes as toast_keyframes;
pub use toast::{
  TOAST_DURATION, TOASTS_SHOWN, Toast, ToastAction, ToastId, ToastPart, ToastPosition, ToastRenderer, ToastState,
  ToastVariant, ToastView, Toasts,
};
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
