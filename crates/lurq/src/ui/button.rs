//! A button: an action a press starts.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{
    BorderSize, ControlSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle,
  },
  components::{self, Row},
  layout::layout_kind::Justify,
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

/// How tall a button is: the steps of the theme's control sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonSize {
  Sm,
  #[default]
  Md,
  Lg,
}

names!(ButtonSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The parts of a button a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonPart {
  /// The button itself: what is pressed.
  Root,
  /// Its text.
  Label,
  /// What is before the text: an icon.
  Leading,
  /// What is after the text.
  Trailing,
}

names!(ButtonPart {
  Root => "root",
  Label => "label",
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
}

names!(ButtonState {
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// What builds a button's nodes: a theme's in place of the default one.
pub type ButtonRenderer = Arc<dyn Fn(ButtonView) -> Element + Send + Sync>;

/// What builds one part of a button: a theme's, for [`ButtonPart::Label`].
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
pub struct Button {
  label: Arc<str>,
  variant: Name,
  size: Name,
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
      variant: ButtonVariant::default().into(),
      size: ButtonSize::default().into(),
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

  /// An element before the label: an icon.
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

  /// Builds the button in `theme`. Converting a button into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ButtonState::Disabled, self.disabled)
    .state(ButtonState::FullWidth, self.full_width);
    let recipe = theme.resolve::<Button>(&case, Some(&self.rules));
    let view = ButtonView {
      label: self.label,
      variant: self.variant,
      size: self.size,
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
  pub variant: Name,
  pub size: Name,
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
    let mut root = components::Button::empty()
      .justify(Justify::Center)
      .disabled(self.disabled)
      .part(&self.part(ButtonPart::Root));
    #[cfg(feature = "form")]
    if self.submit {
      root = root.submit();
    }
    for handler in &self.on_click {
      root = root.on_click(handler.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
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

  /// How the default renderer builds a button: the root, with what is before
  /// the label, the label, and what is after it.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(leading) = self.leading.take() {
      root = root.child(Row::new().part(&self.part(ButtonPart::Leading)).child(leading));
    }
    root = root.child(self.label());
    if let Some(trailing) = self.trailing.take() {
      root = root.child(Row::new().part(&self.part(ButtonPart::Trailing)).child(trailing));
    }
    root.into()
  }
}

impl Themed for Button {
  const NAME: &'static str = "button";
  type Renderer = ButtonRenderer;

  /// The design's button (`pencil/lurq-design-system.pen`, Button): 36 tall,
  /// padded 16, its corners `radius-md`, its label 13/500, and a fill for
  /// each variant with its fill under the pointer.
  fn recipe() -> Recipe {
    use ButtonPart::{Label, Root};
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
      .size(ButtonSize::Sm, Root, |s| s.height(ControlSize::Sm).padding_x(12.0))
      .size(ButtonSize::Lg, Root, |s| s.height(ControlSize::Lg).padding_x(20.0))
      .state(ButtonState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(ButtonState::FullWidth, Root, |s| s.width(Length::Fill));
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
