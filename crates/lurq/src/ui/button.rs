//! A button: an action a press starts.

use std::sync::Arc;

use super::{
  icon::Icon,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::pressable,
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{
    BorderSize, ControlSize, Density, IconSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme,
    TypographyStyle,
  },
  components::{self, Row},
  node::{Element, EventHandler, IntoMouseEventHandler},
  ui::PartTransition,
};

/// How much a button stands out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
  /// The main action of a view.
  #[default]
  Primary,
  /// An alternative beside the primary one.
  Secondary,
  /// A lesser action: dismiss, go back.
  Outline,
  /// An action in a toolbar or in a line of text.
  Ghost,
  /// An action that removes or cannot be undone.
  Destructive,
}

names!(ButtonVariant {
  Primary => "primary",
  Secondary => "secondary",
  Outline => "outline",
  Ghost => "ghost",
  Destructive => "destructive",
});

/// How tall a button is. `Sm`, `Md` and `Lg` are the steps of the theme's
/// control sizes (32, 36, 40); `Xs` is as high as a row of a compact tool
/// (28) and `Xl` as one with room to breathe (44).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonSize {
  Xs,
  Sm,
  #[default]
  Md,
  Lg,
  Xl,
}

names!(ButtonSize {
  Xs => "xs",
  Sm => "sm",
  Md => "md",
  Lg => "lg",
  Xl => "xl",
});

/// The shape of a button's corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonShape {
  /// The theme's radius for a control.
  #[default]
  Default,
  /// Rounded all the way: a pill, and a circle for a button that is an icon alone.
  Pill,
}

/// The parts of a button a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonPart {
  /// The button itself: what is pressed.
  Root,
  /// Its text.
  Label,
  /// An icon the button was given by name, before or after the text: its side, and a color of its own.
  Icon,
  /// What turns while the button is loading, in the place of its icon.
  Spinner,
  /// A second line under the label, which says more of what the button does.
  Description,
  /// An element put before the text.
  Leading,
  /// An element put after the text.
  Trailing,
}

names!(ButtonPart {
  Root => "root",
  Label => "label",
  Icon => "icon",
  Spinner => "spinner",
  Description => "description",
  Leading => "leading",
  Trailing => "trailing",
});

/// The states of a button a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonState {
  /// It takes no input.
  Disabled,
  /// It is as wide as what it is in.
  FullWidth,
  /// What it started is under way: it shows a spinner and is not pressed again.
  Loading,
  /// It is an icon alone, as wide as it is high.
  IconOnly,
  /// Its corners are rounded all the way ([`ButtonShape::Pill`]).
  Pill,
  /// It has a description under its label, and is as high as the two lines are.
  Compound,
}

names!(ButtonState {
  Disabled => "disabled",
  FullWidth => "full_width",
  Loading => "loading",
  IconOnly => "icon_only",
  Pill => "pill",
  Compound => "compound",
});

/// How much of a button shows while it is loading, in the design's recipe.
const LOADING_OPACITY: f32 = 0.75;

/// What builds a button's nodes: a theme's in place of the default one.
pub type ButtonRenderer = Arc<dyn Fn(ButtonView) -> Element + Send + Sync>;

/// What builds one part of a button: a theme's, for [`ButtonPart::Label`],
/// [`ButtonPart::Icon`] or [`ButtonPart::Spinner`].
pub type ButtonPartRenderer = Arc<dyn Fn(&ButtonView) -> Element + Send + Sync>;

/// A button.
///
/// ```
/// use lurq::{components::Row, ui::{Button, ButtonVariant}};
///
/// let actions = Row::new()
///   .spacing(8.0)
///   .child(Button::new("Cancel").variant(ButtonVariant::Outline))
///   .child(Button::new("Save").on_click(|_| println!("saved")));
/// ```
///
/// It is pressed by a click, and by Enter or Space while it is focused. What
/// it looks like is the theme's: its recipe has the parts of [`ButtonPart`],
/// the variants of [`ButtonVariant`], the sizes of [`ButtonSize`] and the
/// states of [`ButtonState`], and a theme may add variants, sizes and states
/// of its own by name.
///
/// An icon is given by its name in the theme's set, and is drawn in the
/// color of the label at the size the button's size has for it:
///
/// ```
/// use lurq::ui::{Button, ButtonShape, ButtonVariant, icons};
///
/// let new_window = Button::new("New window").variant(ButtonVariant::Outline).icon(icons::PLUS);
/// // An icon alone: a square, or a circle. Its label is still what it is called.
/// let settings = Button::new("Settings").variant(ButtonVariant::Ghost).icon(icons::SETTINGS).icon_only(true);
/// let add = Button::new("Add").icon(icons::PLUS).icon_only(true).shape(ButtonShape::Pill);
/// // While what it started is under way.
/// let saving = Button::new("Saving…").loading(true);
/// ```
#[derive(Clone)]
pub struct Button {
  label: Arc<str>,
  description: Option<Arc<str>>,
  variant: Name,
  size: Name,
  shape: ButtonShape,
  icon: Option<Name>,
  trailing_icon: Option<Name>,
  icon_only: bool,
  loading: bool,
  leading: Option<Element>,
  trailing: Option<Element>,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  #[cfg(feature = "form")]
  submit: bool,
}

impl Button {
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      description: None,
      variant: ButtonVariant::default().into(),
      size: ButtonSize::default().into(),
      shape: ButtonShape::default(),
      icon: None,
      trailing_icon: None,
      icon_only: false,
      loading: false,
      leading: None,
      trailing: None,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_click: Vec::new(),
      id: None,
      key: None,
      #[cfg(feature = "form")]
      submit: false,
    }
  }

  /// A [`ButtonVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`ButtonSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// A second line under the label that says more of what the button does:
  /// "Schedule", and under it "Pick a date and time". The button is then as
  /// high as the two lines, with a larger icon beside them.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The shape of the button's corners.
  pub fn shape(mut self, shape: ButtonShape) -> Self {
    self.shape = shape;
    self
  }

  /// An icon before the label, by its name in the theme's set
  /// ([`icons`](super::icons)). It is drawn in the color of the label, at the
  /// size the recipe gives [`ButtonPart::Icon`] for the button's size.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An icon after the label: a chevron, an arrow.
  pub fn trailing_icon(mut self, name: impl Into<Name>) -> Self {
    self.trailing_icon = Some(name.into());
    self
  }

  /// Whether the button is its icon alone: a square as wide as it is high,
  /// or a circle with [`ButtonShape::Pill`]. Its label is not drawn, and
  /// stays what the button is called.
  pub fn icon_only(mut self, icon_only: bool) -> Self {
    self.icon_only = icon_only;
    self
  }

  /// Whether what the button started is under way: a spinner turns in the
  /// place of its icon, or before its label, and a click, Enter and Space do
  /// nothing until it is over. The label is the app's to change: "Saving…".
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// An element before the label, where an icon by name is not enough.
  pub fn leading(mut self, element: impl Into<Element>) -> Self {
    self.leading = Some(element.into());
    self
  }

  /// An element after the label.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing = Some(element.into());
    self
  }

  /// Whether the button takes no input: it is not pressed, hovered or
  /// focused, and Tab passes it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the button is as wide as what it is in.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the button in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a click, and on Enter and Space while the button is focused.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Styles one part of this button over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this button: what the component
  /// it is in says of the button in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the button as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
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

  /// Makes the button the one that submits the form it is in.
  #[cfg(feature = "form")]
  pub fn submit(mut self) -> Self {
    self.submit = true;
    self
  }

  /// What an item of a menu that stands for this button takes from it: a
  /// toolbar moves a button that does not fit into its menu with this. Its
  /// press is the button's, as Enter would make it.
  pub(crate) fn command(&self) -> super::toolbar::ToolbarCommand {
    let handlers = self.on_click.clone();
    super::toolbar::ToolbarCommand {
      label: self.label.clone(),
      icon: self.icon,
      icon_only: self.icon_only,
      disabled: self.disabled || self.loading,
      checked: None,
      run: Arc::new(move || {
        let click = super::toolbar::keyboard_click();
        for handler in &handlers {
          handler.call(&click);
        }
      }),
      id: self.id.clone(),
    }
  }

  /// Builds the button in `theme`. Converting a button into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ButtonState::Disabled, self.disabled)
    .state(ButtonState::FullWidth, self.full_width)
    .state(ButtonState::Loading, self.loading)
    .state(ButtonState::IconOnly, self.icon_only)
    .state(ButtonState::Pill, self.shape == ButtonShape::Pill)
    .state(ButtonState::Compound, self.description.is_some());
    let recipe = theme.resolve::<Button>(&case, Some(&self.rules));
    let view = ButtonView {
      label: self.label,
      description: self.description,
      variant: self.variant,
      size: self.size,
      shape: self.shape,
      icon: self.icon,
      trailing_icon: self.trailing_icon,
      icon_only: self.icon_only,
      loading: self.loading,
      disabled: self.disabled,
      full_width: self.full_width,
      leading: self.leading,
      trailing: self.trailing,
      recipe,
      forced: self.forced,
      on_click: self.on_click,
      id: self.id,
      key: self.key,
      #[cfg(feature = "form")]
      submit: self.submit,
    };
    match theme.renderer::<Button>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Button> for Element {
  fn from(button: Button) -> Self {
    button.build(&current_theme())
  }
}

/// A button as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct ButtonView {
  pub label: Arc<str>,
  /// The line under the label, if the button has one.
  pub description: Option<Arc<str>>,
  pub variant: Name,
  pub size: Name,
  pub shape: ButtonShape,
  /// The name of the icon before the label, if the button has one.
  pub icon: Option<Name>,
  /// The name of the icon after the label, if the button has one.
  pub trailing_icon: Option<Name>,
  /// Whether the button is its icon alone.
  pub icon_only: bool,
  /// Whether what the button started is under way.
  pub loading: bool,
  pub disabled: bool,
  pub full_width: bool,
  /// The element before the label, if the button has one.
  pub leading: Option<Element>,
  /// The element after the label, if the button has one.
  pub trailing: Option<Element>,
  /// The style of each part of this button.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  #[cfg(feature = "form")]
  submit: bool,
}

impl ButtonView {
  /// The style of a part, as this button is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The button's behavior with the style of its root: the element that is
  /// pressed, focused and disabled, with nothing in it yet. A renderer puts
  /// the button's content in it.
  pub fn root(&self) -> components::Button {
    let mut root = pressable(&self.part(ButtonPart::Root), self.disabled);
    // A button that is loading is not pressed again: it is there, and focused if it was, and does nothing.
    #[cfg(feature = "form")]
    if self.submit && !self.loading {
      root = root.submit();
    }
    if !self.loading {
      for handler in &self.on_click {
        root = root.on_click(handler.clone());
      }
    }
    // A button that is an icon alone draws no label, and is still called by it.
    if self.icon_only {
      root = root.describe("a11y_name", self.label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The line under the label, in the typography of the description, if the button has one.
  pub fn description(&self) -> Option<Element> {
    let description = self.description.as_ref()?;
    let part = self.part(ButtonPart::Description);
    Some(part.text(description).nowrap().part(&part).into())
  }

  /// The button's text, in the typography of its label. A theme's renderer
  /// for [`ButtonPart::Label`] builds it in place of the default one.
  pub fn label(&self) -> Element {
    if let Some(renderer) = self
      .theme()
      .part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label)
    {
      return renderer(self);
    }
    let part = self.part(ButtonPart::Label);
    part.text(&self.label).nowrap().part(&part).into()
  }

  /// An icon of the theme's set as this button draws one: in the style of
  /// `part`, at the side the recipe gives that part for the button's size.
  fn glyph(&self, part: ButtonPart, name: Name) -> Element {
    if let Some(renderer) = self.theme().part_renderer::<Button, ButtonPartRenderer>(part) {
      return renderer(self);
    }
    let part = self.part(part);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .part(&part)
      .child(Icon::new(name).size(side).build(self.theme()))
      .into()
  }

  /// The button's icon before its label, if it has one. A theme's renderer
  /// for [`ButtonPart::Icon`] builds it in place of the default one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|name| self.glyph(ButtonPart::Icon, name))
  }

  /// The button's icon after its label, if it has one.
  pub fn trailing_icon(&self) -> Option<Element> {
    self.trailing_icon.map(|name| self.glyph(ButtonPart::Icon, name))
  }

  /// What turns while the button is loading: a [`Spinner`](super::Spinner)
  /// at the side the recipe gives [`ButtonPart::Spinner`], in the color of the
  /// label, and no status of its own, since the label says what is under way.
  /// A theme's renderer for [`ButtonPart::Spinner`] builds it in place of the
  /// default one.
  pub fn spinner(&self) -> Element {
    if let Some(renderer) = self
      .theme()
      .part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Spinner)
    {
      return renderer(self);
    }
    let part = self.part(ButtonPart::Spinner);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let spinner = super::Spinner::new()
      .decorative(true)
      .style(super::SpinnerPart::Indicator, |s| s.square(side));
    Row::new().part(&part).child(spinner.build(self.theme())).into()
  }

  /// How the default renderer builds a button: the root, with what is before
  /// the label, the label, and what is after it. While the button is loading
  /// the spinner stands where its icon was, or before the label; a button
  /// that is an icon alone draws that icon, or the spinner, and nothing else.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    let before = if self.loading {
      Some(self.spinner())
    } else {
      self.icon()
    };
    if let Some(before) = before {
      root = root.child(before);
    }
    if self.icon_only {
      return root.into();
    }
    if let Some(leading) = self.leading.take() {
      root = root.child(Row::new().part(&self.part(ButtonPart::Leading)).child(leading));
    }
    root = match self.description() {
      // The label with its description under it, both at the start of their column.
      Some(description) => root.child(
        crate::components::Column::new()
          .spacing(2.0)
          .child(self.label())
          .child(description),
      ),
      None => root.child(self.label()),
    };
    if let Some(trailing) = self.trailing.take() {
      root = root.child(Row::new().part(&self.part(ButtonPart::Trailing)).child(trailing));
    }
    if let Some(after) = self.trailing_icon() {
      root = root.child(after);
    }
    root.into()
  }
}

impl Themed for Button {
  const NAME: &'static str = "button";
  type Renderer = ButtonRenderer;

  /// The design's button (`pencil/lurq-design-system.pen`, Buttons): 36 tall,
  /// padded 16, its corners `radius-md`, its label 13/500 and its icon 16,
  /// and a fill for each variant with its fill under the pointer. Five
  /// sizes, from 28 to 44, each with its padding, and the smallest and the
  /// largest with a label and an icon of their own size.
  fn recipe() -> Recipe {
    use ButtonPart::{Description, Icon, Label, Root, Spinner};
    use PaletteColor::*;
    let paint = |style: super::PartStyle| {
      [
        Transition::background_color(),
        Transition::foreground(),
        Transition::box_shadow(),
        Transition::outline(),
      ]
      .into_iter()
      .fold(style, |style, what| {
        style.transition(PartTransition::new(what, MotionDuration::Fast))
      })
    };
    let filled =
      |recipe: Recipe, variant: ButtonVariant, fill: PaletteColor, ink: PaletteColor, hover: PaletteColor| {
        recipe
          .variant(variant, Root, |s| s.background(fill).foreground(ink))
          .when(Selector::new().variant(variant).hovered(), Root, |s| {
            s.background(hover)
          })
      };
    let quiet = |recipe: Recipe, variant: ButtonVariant| {
      recipe.variant(variant, Root, |s| s.foreground(Foreground)).when(
        Selector::new().variant(variant).hovered(),
        Root,
        |s| s.background(Highlight).foreground(HighlightForeground),
      )
    };

    let recipe = Recipe::new()
      .base(Root, |s| {
        paint(
          s.height(ControlSize::Md)
            .padding_x(16.0)
            .gap(8.0)
            .radius(RadiusSize::Md),
        )
      })
      .base(Label, |s| s.typography(TypographyStyle::Button))
      .pressed(Root, |s| s.veil(StatePressed))
      .focus_visible(Root, |s| s.focus_ring())
      .base(Icon, |s| s.square(IconSize::Md))
      .base(Spinner, |s| s.square(IconSize::Md))
      .size(ButtonSize::Xs, Root, |s| s.height(Density::Compact).padding_x(10.0))
      .size(ButtonSize::Xs, Label, |s| s.font_size(12.0))
      .size(ButtonSize::Sm, Root, |s| s.height(ControlSize::Sm).padding_x(12.0))
      .size(ButtonSize::Lg, Root, |s| s.height(ControlSize::Lg).padding_x(20.0))
      .size(ButtonSize::Xl, Root, |s| s.height(Density::Comfortable).padding_x(24.0))
      .size(ButtonSize::Xl, Label, |s| s.font_size(15.0))
      .state(ButtonState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(ButtonState::Loading, Root, |s| s.opacity(LOADING_OPACITY))
      .state(ButtonState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(ButtonState::Pill, Root, |s| s.radius(RadiusSize::Full))
      // Two lines: as high as they are, with 10 above and below, and a larger icon 12 before them.
      .base(Description, |s| {
        s.typography(TypographyStyle::Caption).opacity(OpacityLevel::Muted)
      })
      .state(ButtonState::Compound, Root, |s| {
        s.height(Length::Auto).padding_x(16.0).padding_y(10.0).gap(12.0)
      })
      .state(ButtonState::Compound, Label, |s| s.font_size(14.0))
      .state(ButtonState::Compound, Icon, |s| s.square(IconSize::Lg))
      .state(ButtonState::Compound, Spinner, |s| s.square(IconSize::Lg))
      // An icon alone: no padding, and as wide as the button of its size is high.
      .state(ButtonState::IconOnly, Root, |s| s.padding_x(0.0).width(ControlSize::Md));
    // The icon of a size, and the width of a button of that size that is an icon alone.
    let sized = |recipe: Recipe, size: ButtonSize, icon: Length, side: Length| {
      recipe
        .size(size, Icon, |s| s.square(icon))
        .size(size, Spinner, |s| s.square(icon))
        .when(Selector::new().size(size).state(ButtonState::IconOnly), Root, |s| {
          s.width(side)
        })
    };
    let recipe = sized(recipe, ButtonSize::Xs, IconSize::Sm.into(), Density::Compact.into());
    let recipe = sized(recipe, ButtonSize::Sm, IconSize::Md.into(), ControlSize::Sm.into());
    let recipe = sized(recipe, ButtonSize::Lg, Length::Px(18.0), ControlSize::Lg.into());
    let recipe = sized(recipe, ButtonSize::Xl, IconSize::Lg.into(), Density::Comfortable.into());
    let recipe = filled(recipe, ButtonVariant::Primary, Primary, PrimaryForeground, PrimaryHover);
    let recipe = filled(
      recipe,
      ButtonVariant::Secondary,
      Secondary,
      SecondaryForeground,
      SecondaryHover,
    );
    let recipe = filled(
      recipe,
      ButtonVariant::Destructive,
      Destructive,
      DestructiveForeground,
      DestructiveHover,
    );
    let recipe = quiet(recipe, ButtonVariant::Ghost);
    quiet(recipe, ButtonVariant::Outline).variant(ButtonVariant::Outline, Root, |s| {
      s.background(Background)
        .border(crate::node::border::Border::inside(BorderSize::Sm, Input))
    })
  }
}
