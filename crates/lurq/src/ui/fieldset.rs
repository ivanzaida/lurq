//! A fieldset: fields under a legend, which it disables together. And an error summary: what is wrong in a form,
//! at its top, with a link to each field.

use std::sync::Arc;

use super::{
  Field, icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  core::ElementRef,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

/// The parts of a fieldset a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldsetPart {
  /// The fieldset: its edge, its corners, and the legend and the fields one under another in it.
  Root,
  /// The legend's title over its description.
  Legend,
  /// What the fields are, together: "Profile". What the fieldset is called.
  Title,
  /// More of it, under the title, in the muted color.
  Description,
}

names!(FieldsetPart {
  Root => "root",
  Legend => "legend",
  Title => "title",
  Description => "description",
});

/// The states of a fieldset a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldsetState {
  /// It takes no input: every control in it is disabled.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(FieldsetState {
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// What builds a fieldset's nodes: a theme's in place of the default one.
pub type FieldsetRenderer = Arc<dyn Fn(FieldsetView) -> Element + Send + Sync>;

/// A fieldset: fields that go together, under a legend that says what they are. The profile of an account, the
/// address of an order, the notifications a person is sent.
///
/// ```
/// use lurq::{core::Signal, ui::{Button, ButtonVariant, Field, Fieldset, Input}};
///
/// let name = Signal::new(String::new());
/// let profile = Fieldset::new("Profile")
///   .description("Shown to people you share a workspace with.")
///   .field(Field::new("Display name", Input::new(name).full_width(true)).required(true))
///   .child(Button::new("Save profile"));
/// // Every control in it takes no input, and the whole fieldset is dimmed.
/// let locked = Fieldset::new("Billing").disabled(true);
/// ```
///
/// What it holds is one under another, as wide as the fieldset: a [`Field`] given with [`Fieldset::field`] is made
/// as wide as it ([`Field::full_width`]), any other element is given with [`Fieldset::child`]. A disabled fieldset
/// takes everything in it out of input, as an element that is disabled does whatever it holds, and is dimmed.
pub struct Fieldset {
  legend: Arc<str>,
  description: Option<Arc<str>>,
  children: Vec<Element>,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Fieldset {
  /// A fieldset whose legend is `legend`, with nothing in it yet.
  pub fn new(legend: &str) -> Self {
    Self {
      legend: Arc::from(legend),
      description: None,
      children: Vec::new(),
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// More of what the fields are, under the legend, in the muted color.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Adds a field, as wide as the fieldset.
  pub fn field(mut self, field: Field) -> Self {
    self.children.push(field.full_width(true).into());
    self
  }

  /// Adds an element under those already there: a field, a row of buttons, a group of cards.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Adds elements under those already there.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Whether the fieldset takes no input: no control in it is pressed, focused or typed in, and it is dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the fieldset is as wide as what holds it, in place of the 400 the board gives it.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the fieldset in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this fieldset over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this fieldset.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the fieldset's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the fieldset in `theme`. Converting a fieldset into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(FieldsetState::Disabled, self.disabled)
    .state(FieldsetState::FullWidth, self.full_width);
    let view = FieldsetView {
      legend: self.legend,
      description: self.description,
      children: self.children,
      disabled: self.disabled,
      recipe: theme.resolve::<Fieldset>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Fieldset>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Fieldset> for Element {
  fn from(fieldset: Fieldset) -> Self {
    fieldset.build(&current_theme())
  }
}

/// A fieldset as a renderer sees it: what it was given and the style of its parts.
pub struct FieldsetView {
  pub legend: Arc<str>,
  pub description: Option<Arc<str>>,
  /// What the app put in it.
  pub children: Vec<Element>,
  /// Whether it takes no input: the default renderer disables its root, and with it everything in it.
  pub disabled: bool,
  /// The style of each part of this fieldset.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FieldsetView {
  /// The style of a part, as this fieldset is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The legend: its title over its description, each as wide as the fieldset.
  pub fn legend(&self) -> Column {
    let mut legend = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(FieldsetPart::Legend))
      .child(wrapping(&self.part(FieldsetPart::Title), &self.legend));
    if let Some(description) = &self.description {
      legend = legend.child(wrapping(&self.part(FieldsetPart::Description), description));
    }
    legend
  }

  /// The root with nothing in it yet: a group to what reads the tree, called by the legend and described by its
  /// description, and disabled with everything in it where the fieldset is.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(FieldsetPart::Root))
      .describe("a11y_role", "group")
      .describe("a11y_name", self.legend.clone());
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if self.disabled {
      root = root.disabled(true);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a fieldset: the legend and what it holds, one under another.
  pub fn render(mut self) -> Element {
    let children = std::mem::take(&mut self.children);
    self.root().child(self.legend()).with_children(children).into()
  }
}

/// A text in the style of `part`, as wide as the room it has: a longer one goes on to the next line.
fn wrapping(part: &ResolvedPart, content: &str) -> Column {
  let fill = Dimension::Pct(100.0);
  Column::new()
    .width(fill)
    .min_width(Dimension::Px(0.0))
    .part(part)
    .child(part.text(content).width(fill))
}

impl Themed for Fieldset {
  const NAME: &'static str = "fieldset";
  type Renderer = FieldsetRenderer;

  /// The design's fieldset (`pencil/lurq-design-system.pen`, Fieldset): 400 wide, padded 20, with an edge of 1 of
  /// the border's color inside it and corners of `radius-lg`; the legend and the fields one under another, 16
  /// apart. The legend's title is 15 and semibold in the foreground, its description 4 under it in 13 of the
  /// muted color. A disabled one is dimmed.
  fn recipe() -> Recipe {
    use FieldsetPart::{Description, Legend, Root, Title};
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| {
        s.width(400.0)
          .padding_x(20.0)
          .padding_y(20.0)
          .gap(16.0)
          .radius(RadiusSize::Lg)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .foreground(Foreground)
      })
      .base(Legend, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .state(FieldsetState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(FieldsetState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}

/// What is wrong with one field of a form: a line of an [`ErrorSummary`].
///
/// ```
/// use lurq::{core::ElementRef, ui::FieldError};
///
/// let name = ElementRef::new();
/// let error = FieldError::new("Display name", "Display name is required").focuses(&name);
/// ```
#[derive(Clone)]
pub struct FieldError {
  /// What the field is called: its label.
  pub label: Arc<str>,
  /// What is wrong with what it holds, as its field says it.
  pub message: Arc<str>,
  /// What a press of the line focuses: the first element that takes the focus under the one the ref is on.
  pub focus: Option<ElementRef>,
}

impl FieldError {
  /// The error `message` of the field called `label`.
  pub fn new(label: &str, message: &str) -> Self {
    Self {
      label: Arc::from(label),
      message: Arc::from(message),
      focus: None,
    }
  }

  /// A press of the line focuses `field`: the first element that takes the focus under the one the ref is on. A
  /// [`Field`]'s control is under the field's root, so a ref on a row around the field will do.
  pub fn focuses(mut self, field: &ElementRef) -> Self {
    self.focus = Some(field.clone());
    self
  }
}

/// The parts of an error summary a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorSummaryPart {
  /// The box: its fill, its edge, its corners, with the icon beside the body.
  Root,
  /// The icon at its start.
  Icon,
  /// The title over the lines.
  Body,
  /// How many fields are wrong.
  Title,
  /// One line: the bullet and the message. What is pressed, where it focuses its field.
  Item,
  /// The dot before a message.
  Bullet,
  /// What is wrong.
  Message,
}

names!(ErrorSummaryPart {
  Root => "root",
  Icon => "icon",
  Body => "body",
  Title => "title",
  Item => "item",
  Bullet => "bullet",
  Message => "message",
});

/// The states of an error summary a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorSummaryState {
  /// It is as wide as what holds it.
  FullWidth,
}

names!(ErrorSummaryState {
  FullWidth => "full_width",
});

/// What builds an error summary's nodes: a theme's in place of the default one.
pub type ErrorSummaryRenderer = Arc<dyn Fn(ErrorSummaryView) -> Element + Send + Sync>;

/// An error summary: what is wrong in a form, at its top, a line for each field with a link to it. What a person
/// reads when a form they sent was not accepted.
///
/// ```
/// use lurq::{core::ElementRef, ui::{ErrorSummary, FieldError}};
///
/// let (name, email) = (ElementRef::new(), ElementRef::new());
/// let summary = ErrorSummary::new([
///   FieldError::new("Display name", "Display name is required").focuses(&name),
///   FieldError::new("Email", "Email must include a domain").focuses(&email),
/// ]);
/// ```
///
/// Its title counts the fields ("2 fields need attention") unless it is given one. A press of a line, or Enter or
/// Space while it has the focus, focuses its field; each line that focuses a field is a stop of Tab. A summary with
/// no errors draws nothing. It is an alert to what reads the tree, called by its title; each line is a link, called
/// by the field's label and its message.
pub struct ErrorSummary {
  errors: Vec<FieldError>,
  title: Option<Arc<str>>,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ErrorSummary {
  /// A summary of `errors`, in the order of the form.
  pub fn new(errors: impl IntoIterator<Item = FieldError>) -> Self {
    Self {
      errors: errors.into_iter().collect(),
      title: None,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A summary of what `form` says is wrong, once a submit of it was attempted: a line for each of `fields`, given
  /// as its name in the form, its label and what holds it, that has an error, in their order, then a line for each
  /// other field with an error, called by its name and focusing nothing. It reads the form's errors as signals: what
  /// renders it renders again when they change.
  ///
  /// ```
  /// use lurq::{
  ///   components::{FormHandle, FormOptions},
  ///   core::ElementRef,
  ///   ui::ErrorSummary,
  /// };
  ///
  /// let form = FormHandle::new(FormOptions::new());
  /// let (name, email) = (ElementRef::new(), ElementRef::new());
  /// let summary = ErrorSummary::form(&form, [("name", "Display name", &name), ("email", "Email", &email)]);
  /// ```
  #[cfg(feature = "form")]
  pub fn form<'a>(
    form: &crate::components::FormHandle,
    fields: impl IntoIterator<Item = (&'a str, &'a str, &'a ElementRef)>,
  ) -> Self {
    if !form.submit_attempted().get() {
      return Self::new([]);
    }
    let fields: Vec<(&str, &str, &ElementRef)> = fields.into_iter().collect();
    let mut errors: Vec<FieldError> = fields
      .iter()
      .filter_map(|(name, label, field)| {
        let message = form.error(*name).get()?;
        Some(FieldError::new(label, &message).focuses(field))
      })
      .collect();
    let all = form.errors();
    for (name, messages) in all.entries() {
      if fields.iter().any(|(listed, ..)| *listed == name) {
        continue;
      }
      if let Some(message) = messages.first() {
        errors.push(FieldError::new(name, message));
      }
    }
    Self::new(errors)
  }

  /// The title in words of the app's own, in place of the count: in another language.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// Whether the summary is as wide as what holds it, in place of the 400 the board gives it.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the summary in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this summary over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this summary.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws every line as it is under the pointer, held down or focused from the keyboard, with nothing pointing at
  /// it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the summary's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the summary in `theme`. Converting a summary into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ErrorSummaryState::FullWidth, self.full_width);
    let title = self.title.unwrap_or_else(|| match self.errors.len() {
      1 => Arc::from("1 field needs attention"),
      count => Arc::from(format!("{count} fields need attention").as_str()),
    });
    let view = ErrorSummaryView {
      title,
      errors: self.errors,
      recipe: theme.resolve::<ErrorSummary>(&case, Some(&self.rules)),
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ErrorSummary>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ErrorSummary> for Element {
  fn from(summary: ErrorSummary) -> Self {
    summary.build(&current_theme())
  }
}

/// An error summary as a renderer sees it: what it was given and the style of its parts.
pub struct ErrorSummaryView {
  /// The title: the count of the fields, or the app's own words.
  pub title: Arc<str>,
  pub errors: Vec<FieldError>,
  /// The style of each part of this summary.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ErrorSummaryView {
  /// The style of a part, as this summary is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The icon at the start, in the style of its part.
  pub fn icon(&self) -> Element {
    glyph(
      &self.part(ErrorSummaryPart::Icon),
      Name::new(icons::ALERT_CIRCLE),
      false,
    )
  }

  /// The line of one error: the bullet and the message. Where the error focuses a field, the line is pressed and
  /// focused, and is a link to what reads the tree, called by the field's label and the message.
  pub fn item(&self, error: &FieldError) -> Element {
    let part = self.part(ErrorSummaryPart::Item);
    let bullet = Row::new().part(&self.part(ErrorSummaryPart::Bullet));
    let message = self.part(ErrorSummaryPart::Message);
    match &error.focus {
      Some(field) => pressable(&part, false)
        .justify(Justify::Start)
        .align_items(Alignment::Center)
        .describe("a11y_role", "link")
        .describe("a11y_name", format!("{}: {}", error.label, error.message))
        .focuses(field)
        .child(bullet)
        .child(wrapping(&message, &error.message).flex(1.0))
        .into(),
      // A line that goes nowhere is read: it is not underlined under the pointer, nor shows it can be pressed.
      None => {
        let fill = Dimension::Pct(100.0);
        let mut text = Column::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .child(message.text(&error.message).width(fill));
        if let Some(color) = message.rest().foreground {
          text = text.foreground(color);
        }
        Row::new()
          .align_items(Alignment::Center)
          .part(&part)
          .cursor(CursorIcon::Default)
          .child(bullet)
          .child(text)
          .into()
      }
    }
  }

  /// The title over the lines.
  pub fn body(&self) -> Column {
    let mut body = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Stretch)
      // The ring of the focus of a line reaches past it.
      .overflow_visible()
      .part(&self.part(ErrorSummaryPart::Body))
      .child(wrapping(&self.part(ErrorSummaryPart::Title), &self.title));
    for error in &self.errors {
      body = body.child(self.item(error));
    }
    body
  }

  /// How the default renderer builds a summary: the icon beside the body, in the box; nothing where there are no
  /// errors.
  pub fn render(self) -> Element {
    if self.errors.is_empty() {
      return Row::new().into();
    }
    let mut root = Row::new()
      .align_items(Alignment::Start)
      .overflow_visible()
      .part(&self.part(ErrorSummaryPart::Root))
      .describe("a11y_role", "alert")
      .describe("a11y_name", self.title.clone())
      .child(self.icon())
      .child(self.body());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for ErrorSummary {
  const NAME: &'static str = "error_summary";
  type Renderer = ErrorSummaryRenderer;

  /// The design's error summary (`pencil/lurq-design-system.pen`, Error Summary): 400 wide, padded 12 and 14, in
  /// the muted destructive color with an edge of 1 of the destructive color inside it and corners of `radius-md`.
  /// The alert icon of 16 in the destructive color is 10 before the body: the title in 13 and semibold in the
  /// destructive color, and under it, 6 apart, a line for each error, a dot of 4 in the destructive color 8 before
  /// its message in 13 of the foreground. A line is underlined under the pointer.
  fn recipe() -> Recipe {
    use ErrorSummaryPart::{Body, Bullet, Icon, Item, Message, Root, Title};
    use PaletteColor::{Destructive, DestructiveMuted, Foreground};
    Recipe::new()
      .base(Root, |s| {
        s.width(400.0)
          .padding_x(14.0)
          .padding_y(12.0)
          .gap(10.0)
          .background(DestructiveMuted)
          .border(Border::inside(BorderSize::Sm, Destructive))
          .radius(RadiusSize::Md)
          .foreground(Foreground)
      })
      .base(Icon, |s| s.square(IconSize::Md).foreground(Destructive))
      .base(Body, |s| s.gap(6.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Label)
          .weight(FontWeight::SemiBold)
          .foreground(Destructive)
      })
      .base(Item, |s| {
        fading(s.gap(8.0).radius(RadiusSize::Sm).cursor(CursorIcon::Pointer))
      })
      .base(Bullet, |s| {
        s.square(4.0).radius(RadiusSize::Full).background(Destructive)
      })
      .base(Message, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .hovered(Message, |s| s.underline(Border::inside(1.0, Foreground)))
      .focus_visible(Item, |s| s.focus_ring())
      .state(ErrorSummaryState::FullWidth, Root, |s| s.width(Length::Fill))
  }
}
