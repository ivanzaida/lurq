//! What a theme holds for components: their recipes and their renderers, and
//! the theme a component being built is in.

use std::{
  any::Any,
  cell::RefCell,
  collections::HashMap,
  sync::{Arc, Mutex, OnceLock},
};

use super::recipe::{Case, Name, Recipe, ResolvedRecipe, ResolvedStyles, resolve_styles};
use crate::{
  app::theme::{Breakpoint, Theme},
  core::Memo,
};

/// A component the theme styles: it has a name the theme keeps its recipe
/// under, the recipe of the design, and a type of renderer.
///
/// lurq's own components implement it, and so does a component of an app
/// that wants to be themed the same way.
pub trait Themed: 'static {
  /// The name the theme keeps the component's recipe and renderer under.
  const NAME: &'static str;

  /// What builds the component's nodes: usually
  /// `Arc<dyn Fn(&View) -> Element + Send + Sync>` for the component's view.
  type Renderer: Clone + Send + Sync + 'static;

  /// The recipe of the design: what the component looks like in the neutral
  /// theme, and in any theme that says nothing about it.
  fn recipe() -> Recipe;
}

type Stored = Arc<dyn Any + Send + Sync>;

/// What a theme says about components: a part of a
/// [`ThemeDefinition`](crate::app::theme::ThemeDefinition).
///
/// For each component, by its name: a recipe in place of the design's, rules
/// laid over its recipe, a renderer in place of the default one, and
/// renderers for single parts. A definition that says nothing leaves every
/// component as the design has it; a [bare](Self::bare) one leaves every
/// component with no style at all.
#[derive(Clone, Default)]
pub struct UiDefinition {
  recipes: HashMap<Name, Arc<Recipe>>,
  patches: HashMap<Name, Arc<Recipe>>,
  renderers: HashMap<Name, Stored>,
  part_renderers: HashMap<(Name, Name), Stored>,
  bare: bool,
}

impl UiDefinition {
  pub fn new() -> Self {
    Self::default()
  }

  /// A definition in which no component has a recipe: every part comes out
  /// unstyled, and every component still works. The base for a look that
  /// shares nothing with the design's.
  pub fn bare() -> Self {
    Self {
      bare: true,
      ..Self::default()
    }
  }

  /// Whether components fall back to the design's recipe where the
  /// definition has none for them.
  pub fn is_bare(&self) -> bool {
    self.bare
  }

  /// Gives a component a recipe in place of the design's, all of it: nothing
  /// of the design's recipe is left under it.
  pub fn set_recipe<C: Themed>(&mut self, recipe: Recipe) -> &mut Self {
    self.recipes.insert(Name::new(C::NAME), Arc::new(recipe));
    self
  }

  /// Lays rules over a component's recipe: `patch` is given the rules laid
  /// over it so far, and what it adds wins where both style a case.
  ///
  /// ```
  /// use lurq::{
  ///   app::theme::{RadiusSize, ThemeDefinition},
  ///   ui::{Button, ButtonPart, ButtonVariant},
  /// };
  ///
  /// let mut theme = ThemeDefinition::neutral();
  /// // Every primary button is a pill.
  /// theme.ui.patch::<Button>(|rules| rules.variant(ButtonVariant::Primary, ButtonPart::Root, |s| s.radius(RadiusSize::Full)));
  /// ```
  pub fn patch<C: Themed>(&mut self, patch: impl FnOnce(Recipe) -> Recipe) -> &mut Self {
    let name = Name::new(C::NAME);
    let so_far = self
      .patches
      .get(&name)
      .map(|rules| (**rules).clone())
      .unwrap_or_default();
    self.patches.insert(name, Arc::new(patch(so_far)));
    self
  }

  /// Takes back what the definition said of a component's recipe: it is the
  /// design's again, or none in a bare definition.
  pub fn reset_recipe<C: Themed>(&mut self) -> &mut Self {
    let name = Name::new(C::NAME);
    self.recipes.remove(&name);
    self.patches.remove(&name);
    self
  }

  /// Gives a component a renderer in place of its default one: other nodes
  /// for the same behavior.
  pub fn set_renderer<C: Themed>(&mut self, renderer: C::Renderer) -> &mut Self {
    self.renderers.insert(Name::new(C::NAME), Arc::new(renderer));
    self
  }

  pub fn reset_renderer<C: Themed>(&mut self) -> &mut Self {
    self.renderers.remove(&Name::new(C::NAME));
    self
  }

  /// The renderer the definition gives a component, if it gives one.
  pub fn renderer<C: Themed>(&self) -> Option<C::Renderer> {
    self
      .renderers
      .get(&Name::new(C::NAME))
      .and_then(|stored| stored.downcast_ref::<C::Renderer>())
      .cloned()
  }

  /// Gives one part of a component a renderer of its own: the rest of the
  /// component is still built by the renderer it had. `R` is the type the
  /// component documents for the part.
  pub fn set_part_renderer<C: Themed, R: Clone + Send + Sync + 'static>(
    &mut self,
    part: impl Into<Name>,
    renderer: R,
  ) -> &mut Self {
    self
      .part_renderers
      .insert((Name::new(C::NAME), part.into()), Arc::new(renderer));
    self
  }

  /// The renderer the definition gives one part of a component.
  pub fn part_renderer<C: Themed, R: Clone + Send + Sync + 'static>(&self, part: impl Into<Name>) -> Option<R> {
    self
      .part_renderers
      .get(&(Name::new(C::NAME), part.into()))
      .and_then(|stored| stored.downcast_ref::<R>())
      .cloned()
  }

  /// The recipes a component is styled by, the later over the earlier: its
  /// own or the definition's in its place, then the rules laid over it.
  fn layers<C: Themed>(&self) -> Vec<Arc<Recipe>> {
    let name = Name::new(C::NAME);
    let mut layers = Vec::with_capacity(2);
    match self.recipes.get(&name) {
      Some(recipe) => layers.push(recipe.clone()),
      None if !self.bare => layers.push(design_recipe::<C>()),
      None => {}
    }
    if let Some(patch) = self.patches.get(&name) {
      layers.push(patch.clone());
    }
    layers
  }
}

/// The design's recipe of a component, built once.
fn design_recipe<C: Themed>() -> Arc<Recipe> {
  static RECIPES: OnceLock<Mutex<HashMap<Name, Arc<Recipe>>>> = OnceLock::new();
  let mut recipes = RECIPES.get_or_init(Default::default).lock().unwrap();
  recipes
    .entry(Name::new(C::NAME))
    .or_insert_with(|| Arc::new(C::recipe()))
    .clone()
}

/// The styles a theme has resolved, for the version of the theme they were
/// resolved at.
#[derive(Default)]
pub(crate) struct ResolvedCache {
  version: u64,
  styles: HashMap<(Name, Case), Arc<ResolvedStyles>>,
}

impl Theme {
  /// The style of every part of a component in `case`. `instance` is what one
  /// instance lays over the theme's recipe: the rules of the place it is in,
  /// and its own.
  ///
  /// What the theme resolves it keeps until the theme changes, so a list of
  /// a thousand buttons resolves the recipe once.
  pub fn resolve<C: Themed>(&self, case: &Case, instance: Option<&Recipe>) -> ResolvedRecipe {
    self.track_access();
    let layers = self.with_ui(|ui| ui.layers::<C>());
    let mut recipes: Vec<&Recipe> = layers.iter().map(|layer| &**layer).collect();
    if let Some(instance) = instance.filter(|rules| !rules.is_empty()) {
      recipes.push(instance);
      return ResolvedRecipe::new(self, Arc::new(resolve_styles(&recipes, case)));
    }
    let key = (Name::new(C::NAME), case.clone());
    let version = self.version();
    let mut cache = self.resolved_cache().lock().unwrap();
    if cache.version != version {
      cache.version = version;
      cache.styles.clear();
    }
    let styles = cache
      .styles
      .entry(key)
      .or_insert_with(|| Arc::new(resolve_styles(&recipes, case)))
      .clone();
    ResolvedRecipe::new(self, styles)
  }

  /// The renderer the theme gives a component in place of its default one.
  pub fn renderer<C: Themed>(&self) -> Option<C::Renderer> {
    self.track_access();
    self.with_ui(|ui| ui.renderer::<C>())
  }

  /// The renderer the theme gives one part of a component.
  pub fn part_renderer<C: Themed, R: Clone + Send + Sync + 'static>(&self, part: impl Into<Name>) -> Option<R> {
    self.track_access();
    let part = part.into();
    self.with_ui(|ui| ui.part_renderer::<C, R>(part))
  }

  /// Gives a component a recipe in place of the design's: see [`UiDefinition::set_recipe`].
  pub fn set_recipe<C: Themed>(&self, recipe: Recipe) {
    self.update_ui(|ui| {
      ui.set_recipe::<C>(recipe);
    });
  }

  /// Lays rules over a component's recipe: see [`UiDefinition::patch`].
  pub fn patch_recipe<C: Themed>(&self, patch: impl FnOnce(Recipe) -> Recipe) {
    self.update_ui(|ui| {
      ui.patch::<C>(patch);
    });
  }

  /// Gives a component a renderer in place of its default one: see [`UiDefinition::set_renderer`].
  pub fn set_renderer<C: Themed>(&self, renderer: C::Renderer) {
    self.update_ui(|ui| {
      ui.set_renderer::<C>(renderer);
    });
  }

  /// Gives one part of a component a renderer of its own: see [`UiDefinition::set_part_renderer`].
  pub fn set_part_renderer<C: Themed, R: Clone + Send + Sync + 'static>(&self, part: impl Into<Name>, renderer: R) {
    let part = part.into();
    self.update_ui(|ui| {
      ui.set_part_renderer::<C, R>(part, renderer);
    });
  }
}

/// What a component being rendered is in: its theme, and the breakpoint of its window.
struct Surroundings {
  theme: Theme,
  breakpoint: Option<Memo<Option<Breakpoint>>>,
}

thread_local! {
  /// The surroundings of the components being rendered on this thread, innermost last.
  static RENDERING_IN: RefCell<Vec<Surroundings>> = const { RefCell::new(Vec::new()) };
}

/// The theme of what is being built: that of the component being rendered,
/// or of the scope around it ([`Ctx::with_theme`](crate::app::ctx::Ctx::with_theme)).
///
/// Outside a render there is no such theme, and this is a neutral one of its
/// own that follows nothing: build components where they are rendered.
pub fn current_theme() -> Theme {
  RENDERING_IN
    .with(|rendering| rendering.borrow().last().map(|surroundings| surroundings.theme.clone()))
    .unwrap_or_else(|| {
      static NEUTRAL: OnceLock<Theme> = OnceLock::new();
      NEUTRAL.get_or_init(Theme::new).clone()
    })
}

/// The breakpoint of the window what is being built is in: `None` below the
/// smallest one, and outside a render. What reads it is rendered again when
/// the window's width crosses a breakpoint, not at every resize.
pub fn current_breakpoint() -> Option<Breakpoint> {
  let breakpoint = RENDERING_IN.with(|rendering| {
    rendering
      .borrow()
      .last()
      .and_then(|surroundings| surroundings.breakpoint.clone())
  });
  breakpoint.and_then(|breakpoint| breakpoint.get())
}

pub(crate) fn enter_theme(theme: Theme, breakpoint: Option<Memo<Option<Breakpoint>>>) {
  RENDERING_IN.with(|rendering| rendering.borrow_mut().push(Surroundings { theme, breakpoint }));
}

pub(crate) fn leave_theme() {
  RENDERING_IN.with(|rendering| {
    rendering.borrow_mut().pop();
  });
}

/// Runs `build` with `theme` as the theme of what it builds.
pub fn in_theme<R>(theme: &Theme, build: impl FnOnce() -> R) -> R {
  struct Leave;
  impl Drop for Leave {
    fn drop(&mut self) {
      leave_theme();
    }
  }
  // The window is the one around: only the theme changes.
  let breakpoint = RENDERING_IN.with(|rendering| {
    rendering
      .borrow()
      .last()
      .and_then(|surroundings| surroundings.breakpoint.clone())
  });
  enter_theme(theme.clone(), breakpoint);
  let _leave = Leave;
  build()
}
