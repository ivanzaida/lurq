//! What a theme holds for components: their recipes and their renderers, and
//! the theme a component being built is in.

use std::{
  any::Any,
  cell::RefCell,
  collections::HashMap,
  sync::{Arc, Mutex, OnceLock},
};

use super::{
  part_style::PartStyle,
  recipe::{Case, Name, Recipe, ResolvedRecipe, ResolvedStyles, resolve_styles},
};
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

  /// The part the component puts its id on, which [`ResolvedRecipe::owned_by`] names no other node after: `"root"`
  /// unless the component says another (an overlay's `"panel"`).
  const ROOT: &'static str = super::recipe::ROOT;

  /// What builds the component's nodes: usually
  /// `Arc<dyn Fn(&View) -> Element + Send + Sync>` for the component's view.
  type Renderer: Clone + Send + Sync + 'static;

  /// The recipe of the design: what the component looks like in the neutral
  /// theme, and in any theme that says nothing about it.
  fn recipe() -> Recipe;

  /// The parts a recipe styles, in the order the component declares them: its `*Part` enum
  /// (`SwitchPart::declared()`). What a gallery lists and a tool offers to restyle. A component that declares none
  /// has the parts its design's recipe styles, as names.
  fn parts() -> Declared {
    Declared::parts_of(&Self::recipe())
  }

  /// The states a recipe styles the component in besides what the pointer and the keyboard do to it, in the order
  /// the component declares them: its `*State` enum. A component that declares none has the states its design's
  /// recipe names.
  fn states() -> Declared {
    Declared::states_of(&Self::recipe())
  }
}

/// A builder of a themed component, styled as one instance: every component of `lurq::ui` is one, so that what
/// styles any of them (a gallery's playground, an app's helper over several kinds) is written once. Its rules are
/// laid over the recipe of [`Component`](Styled::Component): the builder's own, or that of the component it
/// dresses (a data grid's table, a tour's teaching tip).
///
/// ```
/// use lurq::{
///   app::theme::RadiusSize,
///   ui::{Button, ButtonPart, Name, Styled},
/// };
///
/// // A pill, whatever the component is.
/// fn pill<C: Styled>(component: C, part: impl Into<Name>) -> C {
///   Styled::style(component, part, |s| s.radius(RadiusSize::Full))
/// }
/// let save = pill(Button::new("Save"), ButtonPart::Root);
/// ```
pub trait Styled: Sized {
  /// The component whose recipe the rules are laid over, whose parts and states they name.
  type Component: Themed;

  /// Lays rules over the theme's recipe for this instance, the later over the earlier: what its own `rules` does.
  fn rules(self, rules: &Recipe) -> Self;

  /// Puts this instance in a state its recipe names, or takes it out: what its own `state` does.
  fn state(self, state: impl Into<Name>, on: bool) -> Self;

  /// Styles one part of this instance over what the theme gives it: one rule that always holds, laid as
  /// [`rules`](Styled::rules).
  fn style(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    Styled::rules(self, &Recipe::new().base(part, style))
  }

  /// Says `value` of this instance's root under `name` to tooling that reads the tree, the DevTools and the MCP
  /// tree, as an element's own `describe` does: the state an app's tests and an agent read of it.
  ///
  /// ```
  /// use lurq::ui::{Callout, Progress, Styled};
  ///
  /// let quota = Callout::new("The plan's quota is near.").describe("tone", "warning");
  /// let upload = Progress::new(0.4).describe("share", "40");
  /// ```
  ///
  /// It is said on the node of the root part, laid as [`rules`](Styled::rules) ([`Recipe::describe`]), before what
  /// the component says of itself there: a reader that takes the first value of a name takes the app's. It says
  /// nothing unless a tooling feature (`devtools`, `mcp`) is on.
  fn describe(self, name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
    Styled::rules(self, &Recipe::new().describe(name, value))
  }
}

/// What a component declares of itself for a recipe, its parts or its states: each as a name, and as the Rust
/// that writes it. [`Themed::parts`] and [`Themed::states`] give it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Declared {
  /// The enum that writes them, `SwitchPart`; `None` where they are names only, written as strings.
  pub enum_name: Option<&'static str>,
  /// Each name, with the variant of the enum that writes it: `("thumb", "Thumb")`. A name only has itself.
  pub names: Vec<(Name, &'static str)>,
}

impl Declared {
  /// Names an enum writes: what `names!` gives each enum of parts and states.
  pub fn of(enum_name: &'static str, names: Vec<(Name, &'static str)>) -> Self {
    Self {
      enum_name: Some(enum_name),
      names,
    }
  }

  /// Names alone, written as strings, each once.
  pub fn names_only(names: impl IntoIterator<Item = Name>) -> Self {
    let mut declared = Self::default();
    for name in names {
      if !declared.contains(name) {
        declared.names.push((name, name.as_str()));
      }
    }
    declared
  }

  /// The parts `recipe` styles, in the order it first styles them.
  pub fn parts_of(recipe: &Recipe) -> Self {
    Self::names_only(recipe.rules().iter().map(|rule| rule.part))
  }

  /// The states `recipe` names in its rules, in the order it first names them.
  pub fn states_of(recipe: &Recipe) -> Self {
    Self::names_only(
      recipe
        .rules()
        .iter()
        .flat_map(|rule| rule.selector.held_states().iter().copied()),
    )
  }

  pub fn is_empty(&self) -> bool {
    self.names.is_empty()
  }

  pub fn len(&self) -> usize {
    self.names.len()
  }

  /// Whether `name` is one of them.
  pub fn contains(&self, name: Name) -> bool {
    self.names.iter().any(|(held, _)| *held == name)
  }

  /// The names, in order.
  pub fn names(&self) -> impl Iterator<Item = Name> + '_ {
    self.names.iter().map(|(name, _)| *name)
  }

  /// The Rust that writes `name`: `SwitchPart::Thumb`, or `"thumb"` where there is no enum or the enum does not
  /// have it (a name an app adds).
  pub fn code(&self, name: Name) -> String {
    match (self.enum_name, self.names.iter().find(|(held, _)| *held == name)) {
      (Some(enum_name), Some((_, variant))) => format!("{enum_name}::{variant}"),
      _ => format!("{:?}", name.as_str()),
    }
  }
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

  /// The definition of [`ThemeDefinition::legacy`](crate::app::theme::ThemeDefinition::legacy): the design's
  /// recipes for the components of `lurq::ui`, and the colors of 0.62 for the engine's own styled pieces, which
  /// take theirs from a recipe too: the phone's sheet, the host around it, its tab bar, navigation bar, pull to
  /// refresh and swipe row, the stack's shades, and the engine's select, checkbox and slider.
  pub fn legacy() -> Self {
    let mut ui = Self::new();
    ui.set_recipe::<crate::node::SelectStyle>(crate::node::SelectStyle::legacy_recipe());
    #[allow(deprecated)]
    ui.set_recipe::<crate::components::Checkbox>(crate::components::Checkbox::legacy_recipe());
    ui.set_recipe::<crate::components::Slider>(crate::components::Slider::legacy_recipe());
    ui.set_recipe::<crate::components::SheetStyle>(crate::components::SheetStyle::legacy_recipe());
    ui.set_recipe::<crate::components::SheetHostStyle>(crate::components::SheetHostStyle::legacy_recipe());
    ui.set_recipe::<crate::components::PullToRefreshStyle>(crate::components::PullToRefreshStyle::legacy_recipe());
    ui.set_recipe::<crate::components::SwipeRowStyle>(crate::components::SwipeRowStyle::legacy_recipe());
    #[cfg(feature = "router")]
    {
      ui.set_recipe::<crate::components::TabBar>(crate::components::TabBar::legacy_recipe());
      ui.set_recipe::<crate::components::NavigationBar>(crate::components::NavigationBar::legacy_recipe());
      ui.set_recipe::<crate::components::StackMotion>(crate::components::StackMotion::legacy_recipe());
    }
    ui
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

  /// Takes back the renderer the definition gave one part of a component.
  pub fn reset_part_renderer<C: Themed>(&mut self, part: impl Into<Name>) -> &mut Self {
    self.part_renderers.remove(&(Name::new(C::NAME), part.into()));
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
    let case = &case.in_theme(self);
    let mut recipes: Vec<&Recipe> = layers.iter().map(|layer| &**layer).collect();
    let lent = instance.and_then(|rules| rules.lent()).cloned();
    if let Some(instance) = instance.filter(|rules| !rules.is_empty()) {
      recipes.push(instance);
      return ResolvedRecipe::new(self, Arc::new(resolve_styles(&recipes, case)), C::ROOT, lent).described(&recipes);
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
    // An instance with no rules may still describe its root.
    recipes.extend(instance);
    ResolvedRecipe::new(self, styles, C::ROOT, lent).described(&recipes)
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

  /// Takes back what the theme said of a component's recipe: see [`UiDefinition::reset_recipe`].
  pub fn reset_recipe<C: Themed>(&self) {
    self.update_ui(|ui| {
      ui.reset_recipe::<C>();
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

  /// Takes back the renderer the theme gave a component: its default one builds it again.
  pub fn reset_renderer<C: Themed>(&self) {
    self.update_ui(|ui| {
      ui.reset_renderer::<C>();
    });
  }

  /// Takes back the renderer the theme gave one part of a component.
  pub fn reset_part_renderer<C: Themed>(&self, part: impl Into<Name>) {
    let part = part.into();
    self.update_ui(|ui| {
      ui.reset_part_renderer::<C>(part);
    });
  }
}

/// What a component being rendered is in: its theme, and the breakpoint of its window.
struct Surroundings {
  theme: Theme,
  breakpoint: Option<Memo<Option<Breakpoint>>>,
  /// How the app writes dates: what a component's render was given of its app.
  dates: Option<crate::core::AppDateText>,
  /// The app's locale: every word lurq writes by itself.
  locale: Option<crate::core::AppLocale>,
  /// What the person points with, the app's.
  pointer: Option<crate::app::pointer::AppPointer>,
  /// The window what is being built is in.
  window: Option<crate::app::window::Window>,
  /// Which render of a component on this thread it is, one number for each: what the ids of parts are counted in
  /// (`ui::part_ids`).
  render: u64,
}

thread_local! {
  /// The surroundings of the components being rendered on this thread, innermost last.
  static RENDERING_IN: RefCell<Vec<Surroundings>> = const { RefCell::new(Vec::new()) };
  /// The number of the last render of a component on this thread.
  static RENDERS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Which render of a component on this thread is going on, a number no other render has had: 0 outside one. A
/// subtree in a theme of its own is in the render around it.
pub(crate) fn current_render() -> u64 {
  RENDERING_IN.with(|rendering| rendering.borrow().last().map_or(0, |surroundings| surroundings.render))
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

/// What the person points with, in the app of what is being built
/// ([`App::pointer`](crate::app::App::pointer)): a finger or a mouse, a trackpad, a pen. What reads it is rendered
/// again when it changes. Outside a render, what the platform starts with
/// ([`Pointer::platform_default`](crate::app::Pointer::platform_default)).
pub fn current_pointer() -> crate::app::Pointer {
  RENDERING_IN
    .with(|rendering| {
      rendering
        .borrow()
        .last()
        .and_then(|surroundings| surroundings.pointer.clone())
    })
    .map_or_else(crate::app::Pointer::platform_default, |pointer| pointer.get())
}

/// Whether what is being built takes its touch form, dressed for a finger: swipes in place of buttons over its
/// sides, larger targets, dots in place of arrows, a phone's chrome. It is the person's pointer and nothing else:
/// a coarse one ([`Pointer::Coarse`](crate::app::Pointer::Coarse), [`current_pointer`]), which an app on iOS and
/// Android starts with and any app has once a finger touches it. Neither the window's width nor the theme's density
/// is asked: a narrow window of a desktop is a narrow room, laid out narrow, and a density is how large things are.
/// Every component with a touch form reads it, and a component's own prop for its form (`variant(..)`,
/// `touch(bool)`) wins over it. What reads it is rendered again when the pointer changes.
pub fn touch_form() -> bool {
  current_pointer() == crate::app::Pointer::Coarse
}

/// How many times the app or a test set the pointer rather than input moving it: what an open component that holds
/// its form against a finger landing on it gives way to. Read so that what reads it is rendered again.
pub(crate) fn pointer_told() -> u64 {
  RENDERING_IN
    .with(|rendering| {
      rendering
        .borrow()
        .last()
        .and_then(|surroundings| surroundings.pointer.clone())
    })
    .map_or(0, |pointer| pointer.told())
}

pub(crate) fn enter_theme(
  theme: Theme,
  breakpoint: Option<Memo<Option<Breakpoint>>>,
  dates: Option<crate::core::AppDateText>,
  locale: Option<crate::core::AppLocale>,
  pointer: Option<crate::app::pointer::AppPointer>,
  window: Option<crate::app::window::Window>,
) {
  let render = RENDERS.with(|renders| {
    renders.set(renders.get() + 1);
    renders.get()
  });
  RENDERING_IN.with(|rendering| {
    rendering.borrow_mut().push(Surroundings {
      theme,
      breakpoint,
      dates,
      locale,
      pointer,
      window,
      render,
    })
  });
}

/// The window what is being built is in, in a component's render; `None` outside one. What reads the window
/// through it is rendered again as the window changes.
pub(crate) fn current_window() -> Option<crate::app::window::Window> {
  RENDERING_IN.with(|rendering| {
    rendering
      .borrow()
      .last()
      .and_then(|surroundings| surroundings.window.clone())
  })
}

/// How the app of what is being built writes and reads dates: what
/// [`App::set_date_locale`](crate::app::App::set_date_locale) set, read so that what reads it is rendered again when
/// it changes. Outside a render, and in an app that set none, English. This is where a calendar or a date picker with
/// no `.locale(..)` of its own takes its text, mounted or built in a render.
pub fn current_date_locale() -> std::sync::Arc<dyn crate::core::DateText> {
  RENDERING_IN
    .with(|rendering| {
      rendering
        .borrow()
        .last()
        .and_then(|surroundings| surroundings.dates.clone())
    })
    .map_or_else(crate::core::english_date_text, |dates| dates.get())
}

/// The locale of the app of what is being built: every word lurq writes by itself, as
/// [`App::set_locale`](crate::app::App::set_locale) set it, read so that what reads it is rendered again when it
/// changes. Outside a render, and in an app that set none, English. This is where a component takes the words it
/// was not told itself, mounted or built in a render.
pub fn current_locale() -> std::sync::Arc<crate::core::Locale> {
  RENDERING_IN
    .with(|rendering| {
      rendering
        .borrow()
        .last()
        .and_then(|surroundings| surroundings.locale.clone())
    })
    .map_or_else(crate::core::english_locale, |locale| locale.get())
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
  // The window and the app are the ones around: only the theme changes.
  let (breakpoint, dates, locale, pointer, window) = RENDERING_IN.with(|rendering| {
    rendering
      .borrow()
      .last()
      .map_or((None, None, None, None, None), |surroundings| {
        (
          surroundings.breakpoint.clone(),
          surroundings.dates.clone(),
          surroundings.locale.clone(),
          surroundings.pointer.clone(),
          surroundings.window.clone(),
        )
      })
  });
  let render = current_render();
  enter_theme(theme.clone(), breakpoint, dates, locale, pointer, window);
  // The subtree is built in the render around it.
  if render != 0 {
    RENDERING_IN.with(|rendering| {
      if let Some(surroundings) = rendering.borrow_mut().last_mut() {
        surroundings.render = render;
      }
    });
  }
  let _leave = Leave;
  build()
}
