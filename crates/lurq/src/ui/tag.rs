//! A tag: a label on a thing, which the app may let a person take away.

use std::sync::Arc;

use super::{
  icon::Icon,
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable, tab_stop},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::KeyboardEvent,
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Row},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border as Line, outline::Outline},
};

/// The parts of a tag a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TagPart {
  /// The tag: its fill, its line, its corners and the color of its text.
  Root,
  /// The icon before its text.
  Icon,
  /// Its text.
  Label,
  /// What takes it away, at its end: a button of its own.
  Remove,
  /// The cross on that button: its side.
  RemoveIcon,
}

names!(TagPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
  Remove => "remove",
  RemoveIcon => "remove_icon",
});

/// The states of a tag a recipe styles, besides what the pointer and the
/// keyboard do to its remove button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TagState {
  /// It has the button that takes it away: the part `Remove`.
  Remove,
  /// It is round at both ends, with no line around it: a pill.
  Pill,
  /// It takes no input: its remove button is not pressed.
  Disabled,
}

names!(TagState {
  Remove => "remove",
  Pill => "pill",
  Disabled => "disabled",
});

/// What a tag says by its color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TagVariant {
  /// A keyword, a category: the fill of what is secondary, with a line around it.
  #[default]
  Neutral,
  /// A state that is told: in review, a filter that is set.
  Info,
  /// Something that went well: passing, done.
  Success,
  /// Something to look at: pending, due soon.
  Warning,
  /// Something wrong: failing, blocked.
  Destructive,
}

names!(TagVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// How tall a tag is: 20, 24 or 28.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TagSize {
  Sm,
  #[default]
  Md,
  Lg,
}

names!(TagSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// What builds a tag's nodes: a theme's in place of the default one.
pub type TagRenderer = Arc<dyn Fn(TagView) -> Element + Send + Sync>;

/// The shape of a tag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TagShape {
  /// The theme's small radius, with a line inside it.
  #[default]
  Default,
  /// Round at both ends, with no line around it: a pill.
  Pill,
}

/// A tag: a label on a thing, a keyword or a category, which the app may let
/// a person take away.
///
/// ```
/// use lurq::ui::{Tag, TagShape, TagVariant, icons};
///
/// let plain = Tag::new("design");
/// let status = Tag::new("in review").variant(TagVariant::Info).icon(icons::CIRCLE);
/// // With a button at its end that takes it away: called "Remove rust" to what reads the tree.
/// let removable = Tag::new("rust").on_remove(|| println!("rust taken away"));
/// // Round, with no line around it: a token in a box that is typed in.
/// let token = Tag::new("React").shape(TagShape::Pill).on_remove(|| println!("React taken away"));
/// ```
///
/// A tag does not keep whether it is there: the app holds its list, and is
/// told by [`Tag::on_remove`] which one a person took away. A tag with a
/// handler has its remove button, which Tab reaches; a click, Enter, Space,
/// Backspace and Delete press it.
pub struct Tag {
  label: Arc<str>,
  variant: Name,
  size: Name,
  icon: Option<Name>,
  shape: TagShape,
  disabled: bool,
  tab_stop: bool,
  remove_label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_remove: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Tag {
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      variant: TagVariant::default().into(),
      size: TagSize::default().into(),
      icon: None,
      shape: TagShape::Default,
      disabled: false,
      tab_stop: true,
      remove_label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_remove: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A [`TagVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`TagSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The shape of the tag: [`TagShape::Pill`] is round at both ends, with no line around it.
  pub fn shape(mut self, shape: TagShape) -> Self {
    self.shape = shape;
    self
  }

  /// What the remove button is called to what reads the tree: "Remove" and
  /// the tag's text by default ("Remove rust").
  pub fn remove_label(mut self, label: &str) -> Self {
    self.remove_label = Some(Arc::from(label));
    self
  }

  /// Whether the tag takes no input: it is dimmed, and its remove button is not pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether Tab stops at the remove button; it does unless told otherwise. `false` is a tag whose remove button
  /// the pointer reaches, and the app's own keys: Tab passes it by, and a press of it takes the tag away and leaves
  /// the focus where it was. Focused by the app, a click, Enter, Space, Backspace and Delete press it as before,
  /// and it is the same button to what reads the tree.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// Puts the tag in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of its remove button, which a tag with a handler has.
  pub fn on_remove(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_remove.push(Arc::new(handler));
    self
  }

  /// Styles one part of this tag over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this tag: what the component it
  /// is in says of the tag in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the remove button as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery or
  /// a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the tag's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the tag in `theme`. Converting a tag into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let removable = !self.on_remove.is_empty();
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TagState::Remove, removable)
    .state(TagState::Pill, self.shape == TagShape::Pill)
    .state(TagState::Disabled, self.disabled);
    let remove_label = self.remove_label.unwrap_or_else(|| {
      Arc::from(crate::core::fill(
        &super::current_locale().common.remove_named,
        &[("name", &self.label)],
      ))
    });
    let view = TagView {
      label: self.label,
      remove_label,
      variant: self.variant,
      size: self.size,
      icon: self.icon,
      shape: self.shape,
      removable,
      disabled: self.disabled,
      tab_stop: self.tab_stop,
      recipe: theme
        .resolve::<Tag>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      on_remove: self.on_remove,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Tag>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Tag> for Element {
  fn from(tag: Tag) -> Self {
    tag.build(&current_theme())
  }
}

/// A tag as a renderer sees it: what it was given, the style of its parts,
/// and its remove button ready to be dressed.
pub struct TagView {
  pub label: Arc<str>,
  /// What the remove button is called to what reads the tree.
  pub remove_label: Arc<str>,
  pub variant: Name,
  pub size: Name,
  /// The name of its icon, if it has one.
  pub icon: Option<Name>,
  pub shape: TagShape,
  /// Whether it has a handler for its remove button, and so the button.
  pub removable: bool,
  pub disabled: bool,
  /// Whether Tab stops at the remove button.
  tab_stop: bool,
  /// The style of each part of this tag.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_remove: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TagView {
  /// The style of a part, as this tag is to be drawn. The remove button is
  /// the part the pointer and the keyboard act on, and what `force` draws.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    let part = part.into();
    let resolved = self.recipe.part(part);
    if part == Name::from(TagPart::Remove) {
      resolved.forced(self.forced)
    } else {
      resolved
    }
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What takes the tag away: a button with the cross in it, which a press,
  /// Enter, Space, Backspace and Delete press while the focus is on it. It is
  /// called by `remove_label` to what reads the tree. `None` for a
  /// tag with no handler.
  pub fn remove(&self) -> Option<components::Button> {
    if !self.removable {
      return None;
    }
    let part = self.part(TagPart::Remove);
    let side = self
      .part(TagPart::RemoveIcon)
      .rest()
      .width
      .unwrap_or(Length::Icon(IconSize::Sm));
    let mut remove = tab_stop(pressable(&part, self.disabled), self.tab_stop)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.remove_label.clone())
      .child(Icon::new(icons::CLOSE).size(side).build(self.theme()));
    if let Some(id) = &self.id {
      remove = remove.id(format!("{id}-remove"));
    }
    if !self.disabled {
      let (clicked, keyed) = (self.on_remove.clone(), self.on_remove.clone());
      remove = remove
        .on_click(move |_| {
          for handler in &clicked {
            handler();
          }
        })
        // A key reaches every node: these two take the tag away only while the focus is on its button.
        .on_key_down(move |event: KeyboardEvent| {
          let plain = !event.ctrl && !event.alt && !event.meta && !event.shift;
          if event.focus_within && plain && matches!(event.key.as_str(), "Backspace" | "Delete") {
            event.prevent_default();
            for handler in &keyed {
              handler();
            }
          }
        });
    }
    Some(remove)
  }

  /// How the default renderer builds a tag: its icon, its text and its remove button, side by side.
  pub fn render(self) -> Element {
    // The ring of the remove button's focus is drawn outside it, at the tag's own edge.
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(TagPart::Root));
    if let Some(name) = self.icon {
      root = root.child(glyph(&self.part(TagPart::Icon), name, false));
    }
    let label = self.part(TagPart::Label);
    root = root.child(label.text(&self.label).nowrap().part(&label));
    if let Some(remove) = self.remove() {
      root = root.child(remove);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for Tag {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Tag {
  const NAME: &'static str = "tag";
  type Renderer = TagRenderer;

  fn parts() -> crate::ui::Declared {
    TagPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TagState::declared()
  }

  /// The design's tag (`pencil/lurq-design-system.pen`, Tag): 24 high, padded
  /// 8, the corners of a small radius, in the fill of what is secondary with a
  /// line of the border's color inside it, its text 12 and medium. A
  /// removable one is padded 4 at its end, where a button of 16 with a cross
  /// of 11 in the muted color takes it away. An informative one is in the
  /// muted fill of its color and its text in the color. The design's pill
  /// (Pill) is a tag round at both ends with no line, padded 10 and 6, and a
  /// cross of 12.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use TagPart::{Icon, Label, Remove, RemoveIcon, Root};
    let line = |color: PaletteColor| Line::inside(BorderSize::Sm, color);
    let colored = |recipe: Recipe, variant: TagVariant, fill: PaletteColor, ink: PaletteColor| {
      recipe.variant(variant, Root, move |s| {
        s.background(fill.clone()).border(line(fill)).foreground(ink)
      })
    };
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.height(24.0)
          .padding_x(8.0)
          .gap(4.0)
          .radius(RadiusSize::Sm)
          .background(Secondary)
          .border(line(Border))
          .foreground(SecondaryForeground)
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption).weight(FontWeight::Medium)
      })
      .base(Icon, |s| s.square(IconSize::Xs))
      .base(Remove, |s| {
        fading(
          s.square(16.0)
            .radius(3.0)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(RemoveIcon, |s| s.square(11.0))
      .hovered(Remove, |s| s.background(StateHover).foreground(Foreground))
      .pressed(Remove, |s| s.background(StatePressed).foreground(Foreground))
      // The ring hugs the button: one at the theme's offset would cover the end of the label, 4 before it.
      .focus_visible(Remove, |s| s.outline(Outline::new(BorderSize::Md, Ring)))
      .size(TagSize::Sm, Root, |s| s.height(20.0).padding_x(6.0))
      .size(TagSize::Sm, Label, |s| s.font_size(11.0))
      .size(TagSize::Sm, Remove, |s| s.square(14.0))
      .size(TagSize::Sm, RemoveIcon, |s| s.square(10.0))
      .size(TagSize::Lg, Root, |s| s.height(28.0).padding_x(10.0).gap(6.0))
      .size(TagSize::Lg, Label, |s| s.font_size(13.0))
      .size(TagSize::Lg, Icon, |s| s.square(IconSize::Sm))
      .size(TagSize::Lg, Remove, |s| s.square(18.0))
      .size(TagSize::Lg, RemoveIcon, |s| s.square(12.0))
      .state(TagState::Remove, Root, |s| s.padding_end(4.0))
      .when(
        Selector::new().size(TagSize::Sm).state(TagState::Remove),
        Root,
        |s| s.padding_end(3.0),
      )
      .when(
        Selector::new().size(TagSize::Lg).state(TagState::Remove),
        Root,
        |s| s.padding_end(5.0),
      )
      .state(TagState::Pill, Root, |s| {
        s.radius(RadiusSize::Full)
          .border(Line::inside(0.0, Border))
          .padding_x(10.0)
      })
      .state(TagState::Pill, Remove, |s| s.square(12.0).radius(RadiusSize::Full))
      .state(TagState::Pill, RemoveIcon, |s| s.square(12.0))
      .when(Selector::new().size(TagSize::Sm).state(TagState::Pill), Root, |s| {
        s.padding_x(8.0)
      })
      .when(Selector::new().size(TagSize::Lg).state(TagState::Pill), Root, |s| {
        s.padding_x(12.0)
      })
      .when(
        Selector::new().state(TagState::Pill).state(TagState::Remove),
        Root,
        |s| s.padding_end(6.0),
      )
      .when(
        Selector::new()
          .size(TagSize::Sm)
          .state(TagState::Pill)
          .state(TagState::Remove),
        Root,
        |s| s.padding_end(4.0),
      )
      .when(
        Selector::new()
          .size(TagSize::Lg)
          .state(TagState::Pill)
          .state(TagState::Remove),
        Root,
        |s| s.padding_end(8.0),
      )
      .state(TagState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(TagState::Disabled, Remove, |s| s.cursor(CursorIcon::Default));
    let recipe = colored(recipe, TagVariant::Info, InfoMuted, Info);
    let recipe = colored(recipe, TagVariant::Success, SuccessMuted, Success);
    let recipe = colored(recipe, TagVariant::Warning, WarningMuted, Warning);
    colored(recipe, TagVariant::Destructive, DestructiveMuted, Destructive)
  }
}
