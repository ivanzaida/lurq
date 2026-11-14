//! An input group: an input joined with what stands before and after it into one box.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart},
  field::{FieldControl, InField},
  icon::Icon,
  input::{Input, InputPart, InputView},
  names,
  part_style::{Length, PartStyle},
  recipe::{Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{Select, SelectPart},
  text_box::{self, Judgment, TextBoxState},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::TextInputEvent,
    theme::{IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::Row,
  layout::Alignment,
  node::{Element, RadiusValue, border::Border, color::Color, dimension::Dimension},
};

/// The parts of an input group a recipe styles. What the group says of the input, a button and a select in it is
/// laid over the theme's recipes for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputGroupPart {
  /// The one box: its width, its edge, its corners, its fill.
  Root,
  /// What the group says of the input's root: it is as wide as the room the addons leave.
  Field,
  /// What the group says of the input's box: no edge or fill of its own, its padding.
  Control,
  /// A text, or an icon away from the input, before or after it: on the muted fill.
  Addon,
  /// The glyph of an addon.
  AddonIcon,
  /// The line between two neighbours.
  Separator,
  /// What the group says of a button's root: no corners but the group's.
  Button,
  /// What the group says of a select's trigger: no edge or fill of its own, as wide as what it shows.
  Select,
}

names!(InputGroupPart {
  Root => "root",
  Field => "field",
  Control => "control",
  Addon => "addon",
  AddonIcon => "addon_icon",
  Separator => "separator",
  Button => "button",
  Select => "select",
});

/// The states of an input group a recipe styles. The group is in the input's judgment, size and read only too, by
/// their names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputGroupState {
  /// Nothing in it takes input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(InputGroupState {
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// What stands before or after the input of a group.
pub enum InputAddon {
  /// A text on the muted fill: `https://`, `.com`, a unit.
  Text(Arc<str>),
  /// A glyph of the theme's set. Beside the input it is in the input's box, as an input's own icon is; away from it,
  /// on the muted fill as a text is.
  Icon(Name),
  /// A button: what the input is sent with.
  Button(Button),
  /// A select: a scheme, a unit, a field to search in.
  Select(Select),
}

impl InputAddon {
  /// A text on the muted fill.
  pub fn text(text: &str) -> Self {
    Self::Text(Arc::from(text))
  }

  /// A glyph of the theme's set.
  pub fn icon(name: impl Into<Name>) -> Self {
    Self::Icon(name.into())
  }
}

impl From<&str> for InputAddon {
  fn from(text: &str) -> Self {
    Self::text(text)
  }
}

impl From<String> for InputAddon {
  fn from(text: String) -> Self {
    Self::Text(Arc::from(text))
  }
}

impl From<Button> for InputAddon {
  fn from(button: Button) -> Self {
    Self::Button(button)
  }
}

impl From<Select> for InputAddon {
  fn from(select: Select) -> Self {
    Self::Select(select)
  }
}

/// An input group: an [`Input`] joined with what stands before and after it
/// into one box. Texts, icons, a [`Button`], a [`Select`].
///
/// ```
/// use lurq::{core::Signal, ui::{Button, ButtonVariant, InputAddon, InputGroup, icons}};
///
/// let site = Signal::new("lurq".to_owned());
/// let url = InputGroup::new(lurq::ui::Input::new(site.clone()))
///   .before("https://")
///   .after(".com");
/// let invite = InputGroup::new(lurq::ui::Input::new(Signal::new(String::new())).placeholder("Email"))
///   .before(InputAddon::icon(icons::MAIL))
///   .after(Button::new("Invite").variant(ButtonVariant::Ghost));
/// ```
///
/// The corners of the box are the group's: those of what is at its ends
/// are round where the box is, and every other corner is square. A line is
/// between two neighbours. The edge of the box says what the input holds,
/// and turns the ring's color under the pointer; the ring of the focus is
/// the focused part's own, drawn over its neighbours. The group gives the
/// input, the buttons and the selects in it its size, and disables them
/// where it is disabled. In a [`Field`](super::Field) the input is called by
/// the field's label and judged as the field is.
pub struct InputGroup {
  input: Input,
  before: Vec<InputAddon>,
  after: Vec<InputAddon>,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InputGroup {
  /// A group around `input`, with nothing before or after it yet.
  pub fn new(input: Input) -> Self {
    Self {
      input,
      before: Vec::new(),
      after: Vec::new(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What stands before the input, after what stands there already: a text with `.before("https://")`.
  pub fn before(mut self, addon: impl Into<InputAddon>) -> Self {
    self.before.push(addon.into());
    self
  }

  /// What stands after the input, after what stands there already.
  pub fn after(mut self, addon: impl Into<InputAddon>) -> Self {
    self.after.push(addon.into());
    self
  }

  /// An [`InputSize`](super::InputSize), or a size the theme's recipe names: the input's, and each button's and
  /// select's in the group.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.input = self.input.size(size);
    self
  }

  /// Whether what the input holds is wrong: the box's edge is in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.input = self.input.invalid(invalid);
    self
  }

  /// Whether what the input holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.input = self.input.warning(warning);
    self
  }

  /// Whether what the input holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.input = self.input.valid(valid);
    self
  }

  /// Whether nothing in the group takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.input = self.input.disabled(disabled);
    self
  }

  /// Whether the group is as wide as what holds it, in place of the design's 280.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the group in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this group.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the group's box. The input keeps its own.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Called before an edit of the input is made, with what it will hold.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_input(handler);
    self
  }

  /// Builds the group in `theme`. Converting a group into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let boxed = self.input.boxed.clone();
    let case = boxed
      .case(self.states.clone())
      .state(InputGroupState::Disabled, boxed.disabled)
      .state(InputGroupState::FullWidth, self.full_width);
    let recipe = theme.resolve::<InputGroup>(&case, Some(&self.rules));

    // An icon beside the input is in its box: before it, the input's own icon; after it, over the end of its box.
    let (mut before, mut after) = (self.before, self.after);
    let mut input = self.input;
    if matches!(before.last(), Some(InputAddon::Icon(_)))
      && let Some(InputAddon::Icon(name)) = before.pop()
    {
      input = input.icon(name);
    }
    let end_icon = match after.first() {
      Some(InputAddon::Icon(_)) => match after.remove(0) {
        InputAddon::Icon(name) => Some(name),
        _ => None,
      },
      _ => None,
    };

    let count = before.len() + 1 + after.len();
    let at_input = before.len();
    let corners = |index: usize| (index == 0, index + 1 == count);
    let of_input = recipe
      .rules_for(InputGroupPart::Field, InputPart::Root)
      .extend(&recipe.rules_for(InputGroupPart::Control, InputPart::Control))
      .extend(&corner_rules(&recipe, InputPart::Control, corners(at_input)));
    let view = InputGroupView {
      size: boxed.size,
      disabled: boxed.disabled,
      before,
      after,
      at_input,
      count,
      end_icon,
      input: input.rules(&of_input).view(theme),
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<InputGroup>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// Rules that give `part` of a component the group's corners where it is at an end of the group (`first`, `last`),
/// and square ones elsewhere.
fn corner_rules(recipe: &ResolvedRecipe, part: impl Into<Name>, (first, last): (bool, bool)) -> Recipe {
  let radius = recipe
    .part(InputGroupPart::Root)
    .rest()
    .radius
    .map(|radius| radius.top_left);
  let Some(round) = radius else {
    return Recipe::new();
  };
  let at = |end: bool| if end { round } else { RadiusValue::Px(0.0) };
  Recipe::new().base(part, |s| s.corners(at(first), at(last), at(last), at(first)))
}

impl FieldControl for InputGroup {
  /// The input is called by the field's label unless it has a name of its own, and judged as the field is unless
  /// the app judged it itself; the group is disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.input.boxed.in_field(field);
    self.into()
  }
}

impl From<InputGroup> for Element {
  fn from(group: InputGroup) -> Self {
    group.build(&current_theme())
  }
}

/// An input group as a renderer sees it: the input, what stands before and after it, and the style of its parts.
pub struct InputGroupView {
  /// The input, with what the group says of it laid over its recipe.
  pub input: InputView,
  pub before: Vec<InputAddon>,
  pub after: Vec<InputAddon>,
  /// The glyph after the input, in its box.
  pub end_icon: Option<Name>,
  /// The size of the group: each button's and select's in it.
  pub size: Name,
  pub disabled: bool,
  /// The style of each part of the group.
  pub recipe: ResolvedRecipe,
  at_input: usize,
  count: usize,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InputGroupView {
  /// The style of a part, as this group is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The line between two neighbours.
  pub fn separator(&self) -> Row {
    Row::new().part(&self.part(InputGroupPart::Separator))
  }

  /// A text, or a glyph, on the muted fill.
  fn cell(&self, child: Element) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(InputGroupPart::Addon))
      .child(child)
  }

  fn glyph(&self, name: Name) -> Element {
    let part = self.part(InputGroupPart::AddonIcon);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .part(&part)
      .child(Icon::new(name).size(side).build(self.theme()))
      .into()
  }

  /// An addon at `index` among the group's parts, with the group's corners where it is at an end.
  pub fn addon(&self, addon: InputAddon, index: usize) -> Element {
    let ends = (index == 0, index + 1 == self.count);
    match addon {
      InputAddon::Text(text) => {
        let part = self.part(InputGroupPart::Addon);
        self.cell(part.text(&text).nowrap().into()).into()
      }
      InputAddon::Icon(name) => self.cell(self.glyph(name)).into(),
      InputAddon::Button(button) => {
        let rules = self
          .recipe
          .rules_for(InputGroupPart::Button, ButtonPart::Root)
          .extend(&corner_rules(&self.recipe, ButtonPart::Root, ends));
        let button = button.size(self.size).rules(&rules);
        let button = if self.disabled { button.disabled(true) } else { button };
        button.build(self.theme())
      }
      InputAddon::Select(select) => {
        let rules = self
          .recipe
          .rules_for(InputGroupPart::Select, SelectPart::Trigger)
          .extend(&corner_rules(&self.recipe, SelectPart::Trigger, ends));
        let select = select.size(self.size).rules(&rules);
        let select = if self.disabled { select.disabled(true) } else { select };
        select.build(self.theme())
      }
    }
  }

  /// The input, and the glyph after it over the end of its box. The pointer passes through the glyph.
  pub fn field(&self) -> Element {
    let mut control = self.input.control();
    let mut end = None;
    if let Some(name) = self.end_icon {
      let side = match self
        .part(InputGroupPart::AddonIcon)
        .rest()
        .width
        .and_then(|width| width.resolve(self.theme()))
      {
        Some(Dimension::Px(px)) => px,
        _ => 0.0,
      };
      control = control.padding_right(self.input.end_room(side));
      end = Some(Row::new().pointer_events_none().child(self.glyph(name)));
    }
    // The input takes the room the addons leave, and its root is as wide as that room.
    Row::new()
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .child(self.input.render_with_end(control, end))
      .into()
  }

  /// How the default renderer builds a group: one box, its parts side by side with a line between two of them.
  pub fn render(mut self) -> Element {
    let before = std::mem::take(&mut self.before);
    let after = std::mem::take(&mut self.after);
    // The parts are as high as the box, which clips them to its round corners. A ring is drawn over it.
    let mut root = Row::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(InputGroupPart::Root))
      .clip();
    let mut index = 0;
    for addon in before {
      if index > 0 {
        root = root.child(self.separator());
      }
      root = root.child(self.addon(addon, index));
      index += 1;
    }
    if index > 0 {
      root = root.child(self.separator());
    }
    root = root.child(self.field());
    index += 1;
    debug_assert_eq!(index, self.at_input + 1);
    for addon in after {
      root = root.child(self.separator()).child(self.addon(addon, index));
      index += 1;
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl Themed for InputGroup {
  const NAME: &'static str = "input_group";
  type Renderer = Arc<dyn Fn(InputGroupView) -> Element + Send + Sync>;

  /// The design's input group (`pencil/lurq-design-system.pen`, Input Group): one box 280 wide, the background's
  /// fill in an edge of 1 in the color of an input's, 6 round. A text addon is on the muted fill, padded 10, in 13
  /// and the muted foreground; a line of 1 in the input's color is between two neighbours. The input in it has no
  /// edge or fill of its own and is padded 10. The edge is the ring's color under the pointer, and the judgment's
  /// color where the input is judged; disabled, the box is half opaque on the muted fill. Small has text of 12 in its
  /// addons, large 14.
  fn recipe() -> Recipe {
    use InputGroupPart::{Addon, AddonIcon, Button, Control, Field, Root, Select, Separator};
    use PaletteColor::{Background, Muted, MutedForeground, Ring};
    let clear = Color::new(0, 0, 0, 0);
    let unedged = move |s: PartStyle| s.background(clear).border(Border::inside(1.0, clear));
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(280.0)
          .radius(RadiusSize::Md)
          .background(Background)
          .border(text_box::edge(PaletteColor::Input))
      })
      .hovered(Root, |s| s.border(text_box::edge(Ring)))
      .state(InputGroupState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(InputGroupState::Disabled, Root, |s| {
        s.background(Muted).opacity(OpacityLevel::Disabled)
      })
      .base(Field, |s| s.width(Length::Fill))
      // The group is dimmed whole: what is in it is not dimmed again.
      .state(InputGroupState::Disabled, Field, |s| s.opacity(1.0))
      .base(Control, move |s| unedged(s).padding_x(10.0))
      .hovered(Control, move |s| s.border(Border::inside(1.0, clear)))
      .base(Addon, |s| {
        s.padding_x(10.0)
          .background(Muted)
          .foreground(MutedForeground)
          .font_size(13.0)
      })
      .size(text_box::TextBoxSize::Sm, Addon, |s| s.font_size(12.0))
      .size(text_box::TextBoxSize::Lg, Addon, |s| s.font_size(14.0))
      .base(AddonIcon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Separator, |s| s.width(1.0).background(PaletteColor::Input))
      .state(InputGroupState::Disabled, Button, |s| s.opacity(1.0))
      .base(Select, move |s| unedged(s).width(Length::Auto))
      .hovered(Select, move |s| s.border(Border::inside(1.0, clear)))
      .state(InputGroupState::Disabled, Select, |s| s.opacity(1.0));
    // The edge says what the input holds, under the pointer too.
    [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
      .into_iter()
      .fold(recipe, |recipe, judgment| {
        let (state, color) = (TextBoxState::from(judgment), judgment.color());
        recipe
          .state(state, Root, |s| s.border(text_box::edge(color.clone())))
          .when(super::recipe::Selector::new().state(state).hovered(), Root, |s| {
            s.border(text_box::edge(color))
          })
      })
  }
}
