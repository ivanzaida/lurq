//! A recipe: how the parts of a component look, case by case.
//!
//! A recipe is a list of rules. A rule says "in this case, this part has this
//! style": always, for a variant, for a size, in a window at least so wide,
//! while the component is in a state, under the pointer, or any of these
//! together ("a primary button under the pointer"). For a component as it is being built, the rules whose
//! case holds are laid one over another, the more specific over the less, and
//! what comes out is the style of each part.
//!
//! Variants, sizes, states and parts are names. A component gives its own as
//! enums, which read as names, and an app adds any others it wants: a
//! `"brand"` variant, a `"pinned"` state.

use std::{collections::HashMap, sync::Arc};

use super::part_style::PartStyle;
use crate::{
  app::theme::{Breakpoint, Density, RoleName, Theme},
  components::Text,
  node::{Node, NodeUpdate},
};

/// The name of a variant, a size, a state or a part.
pub type Name = RoleName;

/// What the pointer or the keyboard does to a part, which the part shows by
/// itself: a rule for one of these is no reason to render the component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interaction {
  /// Under the pointer.
  Hovered,
  /// Held down.
  Pressed,
  /// Focused from the keyboard.
  FocusVisible,
}

impl Interaction {
  pub const ALL: [Interaction; 3] = [Self::Hovered, Self::Pressed, Self::FocusVisible];
}

/// The case a rule is for. An empty one holds always.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selector {
  variant: Option<Name>,
  size: Option<Name>,
  /// The density of the theme the rule is for.
  density: Option<Name>,
  /// The narrowest window the rule is for: it holds at this breakpoint and at every wider one.
  breakpoint: Option<Breakpoint>,
  /// States of the component that all have to hold: checked, disabled, loading.
  states: Vec<Name>,
  interaction: Option<Interaction>,
}

impl Selector {
  /// The case that always holds.
  pub fn new() -> Self {
    Self::default()
  }

  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = Some(variant.into());
    self
  }

  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// While the theme is at `density` ([`Theme::density`]).
  pub fn density(mut self, density: impl Into<Density>) -> Self {
    self.density = Some(Name::new(density.into().as_str()));
    self
  }

  /// In a window at `breakpoint` or wider. A rule with no breakpoint is for
  /// every width, the narrowest included, and wider windows are styled over
  /// it: as [`Responsive`](crate::responsive::Responsive) values are.
  pub fn breakpoint(mut self, breakpoint: Breakpoint) -> Self {
    self.breakpoint = Some(breakpoint);
    self
  }

  /// While the component is in `state` too.
  pub fn state(mut self, state: impl Into<Name>) -> Self {
    let state = state.into();
    if !self.states.contains(&state) {
      self.states.push(state);
    }
    self
  }

  pub fn hovered(mut self) -> Self {
    self.interaction = Some(Interaction::Hovered);
    self
  }

  pub fn pressed(mut self) -> Self {
    self.interaction = Some(Interaction::Pressed);
    self
  }

  pub fn focus_visible(mut self) -> Self {
    self.interaction = Some(Interaction::FocusVisible);
    self
  }

  /// How specific the case is: the rules of a more specific case are laid
  /// over those of a less specific one. Always, then a variant, then a size,
  /// then a density, then a breakpoint, a wider one over a narrower, then
  /// states, more of them over fewer.
  fn rank(&self) -> usize {
    if !self.states.is_empty() {
      10 + self.states.len()
    } else if let Some(breakpoint) = self.breakpoint {
      4 + breakpoint as usize
    } else if self.density.is_some() {
      3
    } else if self.size.is_some() {
      2
    } else if self.variant.is_some() {
      1
    } else {
      0
    }
  }

  /// Whether the case holds for a component in `case`, the interaction apart.
  fn holds(&self, case: &Case) -> bool {
    self.variant.is_none_or(|variant| case.variant == Some(variant))
      && self.size.is_none_or(|size| case.size == Some(size))
      && self.density.is_none_or(|density| case.density == Some(density))
      && self
        .breakpoint
        .is_none_or(|from| case.breakpoint.is_some_and(|window| window >= from))
      && self.states.iter().all(|state| case.states.contains(state))
  }
}

/// A rule of a recipe: in a case, a part has a style.
#[derive(Clone, Debug)]
pub struct Rule {
  pub selector: Selector,
  pub part: Name,
  pub style: PartStyle,
}

/// What a component is as it is being built: its variant, its size, the
/// density of its theme, the breakpoint of the window it is in and the
/// states it is in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Case {
  pub variant: Option<Name>,
  pub size: Option<Name>,
  /// The density of the theme. A case that names none takes the density
  /// of the theme it is resolved in.
  pub density: Option<Name>,
  /// The breakpoint of the window: `None` below the smallest one.
  pub breakpoint: Option<Breakpoint>,
  pub states: Vec<Name>,
}

impl Case {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = Some(variant.into());
    self
  }

  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// The density the component is at, whatever its theme is at.
  pub fn density(mut self, density: impl Into<Density>) -> Self {
    self.density = Some(Name::new(density.into().as_str()));
    self
  }

  /// The case with the density of `theme`, unless it names one.
  pub(crate) fn in_theme(&self, theme: &Theme) -> Self {
    let mut case = self.clone();
    if case.density.is_none() {
      case.density = Some(Name::new(theme.density().as_str()));
    }
    case
  }

  /// The breakpoint of the window the component is in.
  pub fn breakpoint(mut self, breakpoint: Option<Breakpoint>) -> Self {
    self.breakpoint = breakpoint;
    self
  }

  /// Adds `state` if `on`.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    if on && !self.states.contains(&state) {
      self.states.push(state);
    }
    self
  }
}

/// How the parts of a component look, case by case.
///
/// ```
/// use lurq::{
///   app::theme::{ControlSize, PaletteColor, RadiusSize},
///   ui::{Recipe, Selector},
/// };
///
/// let recipe = Recipe::new()
///   .base("root", |s| s.height(ControlSize::Md).radius(RadiusSize::Md))
///   .variant("primary", "root", |s| s.background(PaletteColor::Primary).foreground(PaletteColor::PrimaryForeground))
///   .when(Selector::new().variant("primary").hovered(), "root", |s| s.background(PaletteColor::PrimaryHover));
/// assert_eq!(recipe.rules().len(), 3);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Recipe {
  rules: Vec<Rule>,
}

impl Recipe {
  pub fn new() -> Self {
    Self::default()
  }

  /// A rule for the case `selector` describes.
  pub fn when(mut self, selector: Selector, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules.push(Rule {
      selector,
      part: part.into(),
      style: style(PartStyle::new()),
    });
    self
  }

  /// A rule that always holds.
  pub fn base(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new(), part, style)
  }

  /// A rule for a variant.
  pub fn variant(
    self,
    variant: impl Into<Name>,
    part: impl Into<Name>,
    style: impl FnOnce(PartStyle) -> PartStyle,
  ) -> Self {
    self.when(Selector::new().variant(variant), part, style)
  }

  /// A rule for a size.
  pub fn size(self, size: impl Into<Name>, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new().size(size), part, style)
  }

  /// A rule for a density of the theme: a row that is lower in a compact
  /// tool and higher under a finger.
  pub fn density(
    self,
    density: impl Into<Density>,
    part: impl Into<Name>,
    style: impl FnOnce(PartStyle) -> PartStyle,
  ) -> Self {
    self.when(Selector::new().density(density), part, style)
  }

  /// A rule for a window at `breakpoint` or wider: a part that is one size
  /// on a phone and another on a desktop.
  pub fn at(self, breakpoint: Breakpoint, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new().breakpoint(breakpoint), part, style)
  }

  /// A rule for a state of the component: checked, disabled, loading, or a
  /// state an app names.
  pub fn state(
    self,
    state: impl Into<Name>,
    part: impl Into<Name>,
    style: impl FnOnce(PartStyle) -> PartStyle,
  ) -> Self {
    self.when(Selector::new().state(state), part, style)
  }

  /// A rule for a part under the pointer, whatever the variant.
  pub fn hovered(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new().hovered(), part, style)
  }

  /// A rule for a part held down.
  pub fn pressed(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new().pressed(), part, style)
  }

  /// A rule for a part focused from the keyboard.
  pub fn focus_visible(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.when(Selector::new().focus_visible(), part, style)
  }

  /// Adds the rules of `other` after this recipe's: where both style a case,
  /// `other` wins.
  pub fn extend(mut self, other: &Recipe) -> Self {
    self.rules.extend(other.rules.iter().cloned());
    self
  }

  pub fn rules(&self) -> &[Rule] {
    &self.rules
  }

  pub fn is_empty(&self) -> bool {
    self.rules.is_empty()
  }

  /// The style of every part for a component in `case`, in `theme`.
  pub fn resolve(&self, case: &Case, theme: &Theme) -> ResolvedRecipe {
    resolve(&[self], case, theme)
  }
}

/// The styles of every part of a component in one case: the rules of
/// `recipes` that hold, each recipe's over the one before it.
pub(crate) fn resolve(recipes: &[&Recipe], case: &Case, theme: &Theme) -> ResolvedRecipe {
  ResolvedRecipe::new(theme, Arc::new(resolve_styles(recipes, &case.in_theme(theme))))
}

/// The styles of [`resolve`], apart from a theme: they are the same in every
/// theme with the same recipes, so a theme keeps them.
pub(crate) fn resolve_styles(recipes: &[&Recipe], case: &Case) -> ResolvedStyles {
  let mut holding: Vec<(usize, usize, &Rule)> = recipes
    .iter()
    .enumerate()
    .flat_map(|(layer, recipe)| recipe.rules.iter().map(move |rule| (layer, rule)))
    .filter(|(_, rule)| rule.selector.holds(case))
    .map(|(layer, rule)| (layer, rule.selector.rank(), rule))
    .collect();
  // A later recipe over an earlier one, the more specific case over the less; rules of one rank in the order
  // they were written.
  holding.sort_by_key(|(layer, rank, _)| (*layer, *rank));

  let mut parts: HashMap<Name, PartStyles> = HashMap::new();
  for (_, _, rule) in holding {
    let part = parts.entry(rule.part).or_default();
    match rule.selector.interaction {
      None => part.rest.merge(&rule.style),
      Some(Interaction::Hovered) => part.hovered.merge(&rule.style),
      Some(Interaction::Pressed) => part.pressed.merge(&rule.style),
      Some(Interaction::FocusVisible) => part.focus_visible.merge(&rule.style),
    }
  }
  ResolvedStyles {
    parts,
    nothing: PartStyles::default(),
  }
}

/// The styles of one part: at rest, and what each interaction lays over that.
#[derive(Clone, Debug, Default)]
pub struct PartStyles {
  pub rest: PartStyle,
  pub hovered: PartStyle,
  pub pressed: PartStyle,
  pub focus_visible: PartStyle,
}

impl PartStyles {
  /// What an interaction lays over the style at rest.
  pub fn interaction(&self, interaction: Interaction) -> &PartStyle {
    match interaction {
      Interaction::Hovered => &self.hovered,
      Interaction::Pressed => &self.pressed,
      Interaction::FocusVisible => &self.focus_visible,
    }
  }
}

/// The styles of every part of a component in one case.
pub(crate) struct ResolvedStyles {
  parts: HashMap<Name, PartStyles>,
  nothing: PartStyles,
}

/// The style of every part of a component as it is being built: what a
/// renderer builds the component's nodes from.
#[derive(Clone)]
pub struct ResolvedRecipe {
  theme: Theme,
  styles: Arc<ResolvedStyles>,
}

impl ResolvedRecipe {
  pub(crate) fn new(theme: &Theme, styles: Arc<ResolvedStyles>) -> Self {
    Self {
      theme: theme.clone(),
      styles,
    }
  }

  /// The style of a part. A part no rule styles has an empty one.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    ResolvedPart {
      recipe: self.clone(),
      part: part.into(),
      forced: None,
    }
  }

  /// The theme the recipe was resolved in.
  pub fn theme(&self) -> &Theme {
    &self.theme
  }

  fn styles(&self, part: Name) -> &PartStyles {
    self.styles.parts.get(&part).unwrap_or(&self.styles.nothing)
  }

  /// Rules that give the part `inner` of another component what this recipe
  /// has for `part`, at rest and in each interaction: how a component that
  /// holds another hands it its place. A group's recipe has a part for its
  /// items, and gives each button in it these rules over the theme's for a
  /// button (`Button::rules`).
  pub fn rules_for(&self, part: impl Into<Name>, inner: impl Into<Name>) -> Recipe {
    let styles = self.styles(part.into()).clone();
    let inner = inner.into();
    Recipe::new()
      .base(inner, |_| styles.rest.clone())
      .hovered(inner, |_| styles.hovered.clone())
      .pressed(inner, |_| styles.pressed.clone())
      .focus_visible(inner, |_| styles.focus_visible.clone())
  }
}

/// The style of one part of a component as it is being built.
///
/// A renderer gives it to the node that is the part,
/// `Row::new().part(&recipe.part(ButtonPart::Root))`, and makes the part's
/// text with [`ResolvedPart::text`].
#[derive(Clone)]
pub struct ResolvedPart {
  recipe: ResolvedRecipe,
  part: Name,
  forced: Option<Interaction>,
}

impl ResolvedPart {
  /// The part drawn as it is in an interaction, with nothing pointing at it:
  /// for a gallery, a screenshot, a test. `None` leaves it to the pointer.
  pub fn forced(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The styles the rules gave the part.
  pub fn styles(&self) -> &PartStyles {
    self.recipe.styles(self.part)
  }

  /// The style of the part at rest, with a forced interaction laid over it.
  pub fn rest(&self) -> PartStyle {
    let styles = self.styles();
    match self.forced {
      // A part held down is under the pointer too.
      Some(Interaction::Pressed) => styles.rest.clone().merged(&styles.hovered).merged(&styles.pressed),
      Some(interaction) => styles.rest.clone().merged(styles.interaction(interaction)),
      None => styles.rest.clone(),
    }
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text in the part's typography. It names no color: it takes the
  /// foreground of the part it is in.
  pub fn text(&self, content: &str) -> Text {
    Text::styled(content, self.rest().text.resolve(self.theme()))
  }

  /// Gives a node the part's style: what it shows at rest, and what it shows
  /// under the pointer, held down and focused from the keyboard.
  pub(crate) fn apply(&self, node: &mut Node) {
    let theme = self.theme();
    let styles = self.styles();
    self.rest().apply(node, theme);
    if let Some(style) = styles.hovered.state_style(theme) {
      NodeUpdate::hovered_style(node, style);
    }
    if let Some(style) = styles.pressed.state_style(theme) {
      NodeUpdate::active_style(node, style);
    }
    if let Some(style) = styles.focus_visible.state_style(theme) {
      NodeUpdate::focus_visible_style(node, style);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{Case, Interaction, Recipe, Selector};
  use crate::{
    app::theme::{ControlSize, PaletteColor, RadiusSize, Theme},
    node::BackgroundColor,
    ui::Length,
  };

  fn recipe() -> Recipe {
    Recipe::new()
      .base("root", |s| {
        s.height(ControlSize::Md)
          .radius(RadiusSize::Md)
          .background(PaletteColor::Muted)
      })
      .variant("primary", "root", |s| s.background(PaletteColor::Primary))
      .size("lg", "root", |s| s.height(ControlSize::Lg))
      .state("disabled", "root", |s| s.opacity(0.5))
      .when(Selector::new().variant("primary").hovered(), "root", |s| {
        s.background(PaletteColor::PrimaryHover)
      })
      .hovered("root", |s| s.background(PaletteColor::StateHover))
  }

  fn background(color: PaletteColor) -> Option<BackgroundColor> {
    Some(BackgroundColor::Palette(color))
  }

  #[test]
  fn the_rules_that_hold_are_laid_the_more_specific_over_the_less() {
    let theme = Theme::new();
    let resolved = recipe().resolve(&Case::new().variant("primary").size("lg"), &theme);
    let root = resolved.part("root").rest();
    assert_eq!(
      root.background,
      background(PaletteColor::Primary),
      "the variant over the base"
    );
    assert_eq!(
      root.height,
      Some(Length::Control(ControlSize::Lg)),
      "the size over the base"
    );
    assert_eq!(root.radius, Some(RadiusSize::Md.into()), "what nothing replaced");
    assert_eq!(root.opacity, None, "a state it is not in");
  }

  #[test]
  fn a_rule_for_a_state_holds_while_the_component_is_in_it() {
    let theme = Theme::new();
    let case = Case::new().state("disabled", true).state("loading", false);
    assert!(recipe().resolve(&case, &theme).part("root").rest().opacity.is_some());
  }

  #[test]
  fn an_interaction_is_kept_apart_from_the_style_at_rest() {
    let theme = Theme::new();
    let primary = recipe().resolve(&Case::new().variant("primary"), &theme);
    let root = primary.part("root");
    assert_eq!(root.rest().background, background(PaletteColor::Primary));
    assert_eq!(
      root.styles().hovered.background,
      background(PaletteColor::PrimaryHover),
      "the rule for a primary button under the pointer over the one for any"
    );
    let plain = recipe().resolve(&Case::new(), &theme);
    assert_eq!(
      plain.part("root").styles().hovered.background,
      background(PaletteColor::StateHover)
    );
  }

  #[test]
  fn a_forced_interaction_is_drawn_as_the_style_at_rest() {
    let theme = Theme::new();
    let resolved = recipe().resolve(&Case::new().variant("primary"), &theme);
    let hovered = resolved.part("root").forced(Some(Interaction::Hovered)).rest();
    assert_eq!(hovered.background, background(PaletteColor::PrimaryHover));
    assert_eq!(hovered.radius, Some(RadiusSize::Md.into()));
  }

  #[test]
  fn a_later_recipe_is_laid_over_an_earlier_one_whatever_the_case() {
    let theme = Theme::new();
    let instance = Recipe::new().base("root", |s| s.background(PaletteColor::Destructive));
    let resolved = super::resolve(&[&recipe(), &instance], &Case::new().variant("primary"), &theme);
    assert_eq!(
      resolved.part("root").rest().background,
      background(PaletteColor::Destructive),
      "the instance's rule for any case over the theme's rule for the variant"
    );
  }

  #[test]
  fn a_rule_for_a_density_holds_while_the_theme_is_at_it() {
    use crate::app::theme::Density;
    let theme = Theme::new();
    let recipe = Recipe::new()
      .base("root", |s| s.height(36.0))
      .size("lg", "root", |s| s.height(40.0))
      .density(Density::Compact, "root", |s| s.height(28.0))
      .density(Density::Touch, "root", |s| s.height(48.0));
    let height = |case: Case| recipe.resolve(&case, &theme).part("root").rest().height;
    assert_eq!(height(Case::new()), Some(Length::Px(36.0)), "the default density");
    theme.set_density(Density::Compact);
    assert_eq!(height(Case::new()), Some(Length::Px(28.0)), "the theme at compact");
    assert_eq!(
      height(Case::new().size("lg")),
      Some(Length::Px(28.0)),
      "the rule for the density over the rule for the size"
    );
    assert_eq!(
      height(Case::new().density(Density::Touch)),
      Some(Length::Px(48.0)),
      "a case that names a density keeps it"
    );
  }

  #[test]
  fn a_rule_for_a_breakpoint_holds_from_that_width_up() {
    use crate::app::theme::Breakpoint;
    let theme = Theme::new();
    let recipe = Recipe::new()
      .base("root", |s| s.height(48.0))
      .at(Breakpoint::Md, "root", |s| s.height(36.0))
      .at(Breakpoint::Xl, "root", |s| s.height(40.0));
    let height = |breakpoint| {
      recipe
        .resolve(&Case::new().breakpoint(breakpoint), &theme)
        .part("root")
        .rest()
        .height
    };
    assert_eq!(
      height(None),
      Some(Length::Px(48.0)),
      "a phone: the rule for every width"
    );
    assert_eq!(
      height(Some(Breakpoint::Sm)),
      Some(Length::Px(48.0)),
      "narrower than the rule's"
    );
    assert_eq!(height(Some(Breakpoint::Md)), Some(Length::Px(36.0)));
    assert_eq!(
      height(Some(Breakpoint::Lg)),
      Some(Length::Px(36.0)),
      "and at every wider one"
    );
    assert_eq!(
      height(Some(Breakpoint::Xl)),
      Some(Length::Px(40.0)),
      "the widest that holds wins"
    );
  }

  #[test]
  fn a_part_no_rule_styles_is_unstyled() {
    let theme = Theme::new();
    let resolved = recipe().resolve(&Case::new(), &theme);
    assert!(resolved.part("icon").rest().is_empty());
    assert!(
      Recipe::new()
        .resolve(&Case::new(), &theme)
        .part("root")
        .rest()
        .is_empty()
    );
  }

  #[test]
  fn an_app_names_variants_and_states_of_its_own() {
    let theme = Theme::new();
    let recipe = recipe()
      .variant("brand", "root", |s| s.background(PaletteColor::extra("brand")))
      .state("pinned", "root", |s| s.radius(RadiusSize::Full));
    let resolved = recipe.resolve(&Case::new().variant("brand").state("pinned", true), &theme);
    let root = resolved.part("root").rest();
    assert_eq!(root.background, background(PaletteColor::extra("brand")));
    assert_eq!(root.radius, Some(RadiusSize::Full.into()));
  }
}
