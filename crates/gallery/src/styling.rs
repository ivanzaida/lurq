//! The styling section of a playground: the parts and the states the component declares
//! ([`Themed::parts`], [`Themed::states`]), a part restyled live, and the Rust of it in both forms: `.style(..)`
//! on the one on the stage, and `theme.patch_recipe::<X>(..)` for every one of them in the stage's own theme. The
//! gallery's own chrome is never in that theme.
//!
//! A page gives the playground its component with [`Playground::styled`](crate::playground::Playground::styled),
//! which says what the component is and lays on it what the section asks for; nothing else of a page knows of it.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    ctx::Ctx,
    theme::{
      BorderSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, SpacingSize, Theme, ThemeDefinition,
      TypographyStyle,
    },
    window::Appearance,
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::text_style::FontWeight,
  node::{Element, border::Border, color::Color},
  ui::{Button, ButtonSize, ButtonVariant, Declared, Name, PartStyle, Recipe, Styled, Themed},
};

use crate::{
  kit::{FILL, code_block, quiet, segmented_fill, select, text},
  playground::{divider, prop},
};

/// What a component is to the section: its name as code writes it, and what it declares.
#[derive(Clone)]
pub struct Anatomy {
  /// The component's type as code writes it: `Switch`.
  pub type_name: String,
  pub parts: Declared,
  pub states: Declared,
  /// Lays rules over the component's recipe in a theme: what `theme.patch_recipe::<X>(..)` does.
  patch: Arc<dyn Fn(&Theme, Recipe) + Send + Sync>,
}

impl Anatomy {
  pub fn of<C: Themed>() -> Self {
    let full = std::any::type_name::<C>();
    let bare = full.split('<').next().unwrap_or(full);
    Self {
      type_name: bare.rsplit("::").next().unwrap_or(bare).to_owned(),
      parts: C::parts(),
      states: C::states(),
      patch: Arc::new(|theme, rules| theme.patch_recipe::<C>(move |so_far| so_far.extend(&rules))),
    }
  }

  /// What the one on the stage is called in the code: `switch`, `tabs`, `date_picker`.
  fn binding(&self) -> String {
    let mut binding = String::new();
    for (index, letter) in self.type_name.chars().enumerate() {
      if letter.is_uppercase() {
        if index > 0 {
          binding.push('_');
        }
        binding.extend(letter.to_lowercase());
      } else {
        binding.push(letter);
      }
    }
    binding
  }
}

/// A color a part is painted in: a role of the palette, or a color of its own.
#[derive(Clone)]
enum Paint {
  Role(PaletteColor, &'static str),
  Hex(&'static str),
}

impl Paint {
  fn code(&self) -> String {
    match self {
      Self::Role(_, name) => format!("PaletteColor::{name}"),
      Self::Hex(hex) => format!("Color::from_hex({hex:?})"),
    }
  }

  fn label(&self) -> String {
    match self {
      Self::Role(_, name) => (*name).to_owned(),
      Self::Hex(hex) => (*hex).to_owned(),
    }
  }
}

fn paints() -> Vec<Paint> {
  use PaletteColor as P;
  vec![
    Paint::Role(P::Primary, "Primary"),
    Paint::Role(P::PrimaryForeground, "PrimaryForeground"),
    Paint::Role(P::Secondary, "Secondary"),
    Paint::Role(P::SecondaryForeground, "SecondaryForeground"),
    Paint::Role(P::Muted, "Muted"),
    Paint::Role(P::MutedForeground, "MutedForeground"),
    Paint::Role(P::Highlight, "Highlight"),
    Paint::Role(P::Destructive, "Destructive"),
    Paint::Role(P::DestructiveForeground, "DestructiveForeground"),
    Paint::Role(P::Success, "Success"),
    Paint::Role(P::Warning, "Warning"),
    Paint::Role(P::Info, "Info"),
    Paint::Role(P::Background, "Background"),
    Paint::Role(P::Foreground, "Foreground"),
    Paint::Role(P::Card, "Card"),
    Paint::Role(P::Border, "Border"),
    Paint::Role(P::Ring, "Ring"),
    Paint::Role(P::Chart1, "Chart1"),
    Paint::Role(P::Chart2, "Chart2"),
    Paint::Hex("#7c3aed"),
    Paint::Hex("#0ea5e9"),
    Paint::Hex("#f97316"),
  ]
}

/// The edges a part can be given: a width of the theme and a role.
fn borders() -> Vec<(&'static str, BorderSize, PaletteColor, &'static str)> {
  vec![
    ("Sm", BorderSize::Sm, PaletteColor::Border, "Border"),
    ("Sm", BorderSize::Sm, PaletteColor::Input, "Input"),
    ("Md", BorderSize::Md, PaletteColor::Primary, "Primary"),
    ("Md", BorderSize::Md, PaletteColor::Destructive, "Destructive"),
    ("Lg", BorderSize::Lg, PaletteColor::Ring, "Ring"),
  ]
}

const RADII: [(&str, RadiusSize); 5] = [
  ("Sm", RadiusSize::Sm),
  ("Md", RadiusSize::Md),
  ("Lg", RadiusSize::Lg),
  ("Xl", RadiusSize::Xl),
  ("Full", RadiusSize::Full),
];

const SPACINGS: [(&str, SpacingSize); 5] = [
  ("Xs", SpacingSize::Xs),
  ("Sm", SpacingSize::Sm),
  ("Md", SpacingSize::Md),
  ("Lg", SpacingSize::Lg),
  ("Xl", SpacingSize::Xl),
];

const SHADOWS: [(&str, ShadowStyle); 3] = [
  ("Sm", ShadowStyle::Sm),
  ("Md", ShadowStyle::Md),
  ("Lg", ShadowStyle::Lg),
];

const OPACITIES: [(&str, OpacityLevel); 2] = [("Muted", OpacityLevel::Muted), ("Disabled", OpacityLevel::Disabled)];

const TYPOGRAPHY: [(&str, TypographyStyle); 9] = [
  ("Heading", TypographyStyle::Heading),
  ("Title", TypographyStyle::Title),
  ("Body", TypographyStyle::Body),
  ("Description", TypographyStyle::Description),
  ("Caption", TypographyStyle::Caption),
  ("Label", TypographyStyle::Label),
  ("Button", TypographyStyle::Button),
  ("Link", TypographyStyle::Link),
  ("Mono", TypographyStyle::Mono),
];

/// The cases a style is for besides the component's own states: always, and what the pointer and the keyboard
/// do to it, with the method of a recipe that writes each.
const INTERACTIONS: [(&str, &str); 4] = [
  ("always", "base"),
  ("hovered", "hovered"),
  ("pressed", "pressed"),
  ("focus_visible", "focus_visible"),
];

/// What one part is restyled with in one case: each property the index of its choice, 0 for none.
#[derive(Clone, Default, PartialEq, lurq::DevtoolsInspectable)]
pub struct Look {
  part: usize,
  /// The case: an index of [`INTERACTIONS`], then of the component's states.
  when: usize,
  background: usize,
  foreground: usize,
  border: usize,
  radius: usize,
  padding: usize,
  gap: usize,
  shadow: usize,
  opacity: usize,
  typography: usize,
}

impl Look {
  fn is_empty(&self) -> bool {
    [
      self.background,
      self.foreground,
      self.border,
      self.radius,
      self.padding,
      self.gap,
      self.shadow,
      self.opacity,
      self.typography,
    ]
    .iter()
    .all(|chosen| *chosen == 0)
  }

  /// The style, laid over `style`.
  fn apply(&self, mut s: PartStyle) -> PartStyle {
    let paints = paints();
    let paint = |index: usize| -> Option<Paint> { index.checked_sub(1).and_then(|at| paints.get(at).cloned()) };
    if let Some(paint) = paint(self.background) {
      s = match paint {
        Paint::Role(role, _) => s.background(role),
        Paint::Hex(hex) => s.background(Color::from_hex(hex)),
      };
    }
    if let Some(paint) = paint(self.foreground) {
      s = match paint {
        Paint::Role(role, _) => s.foreground(role),
        Paint::Hex(hex) => s.foreground(Color::from_hex(hex)),
      };
    }
    if let Some((_, size, role, _)) = self.border.checked_sub(1).and_then(|at| borders().get(at).cloned()) {
      s = s.border(Border::inside(size, role));
    }
    if let Some((_, radius)) = self.radius.checked_sub(1).and_then(|at| RADII.get(at).cloned()) {
      s = s.radius(radius);
    }
    if let Some((_, padding)) = self.padding.checked_sub(1).and_then(|at| SPACINGS.get(at).cloned()) {
      s = s.padding_x(padding.clone()).padding_y(padding);
    }
    if let Some((_, gap)) = self.gap.checked_sub(1).and_then(|at| SPACINGS.get(at).cloned()) {
      s = s.gap(gap);
    }
    if let Some((_, shadow)) = self.shadow.checked_sub(1).and_then(|at| SHADOWS.get(at).cloned()) {
      s = s.shadow(shadow);
    }
    if let Some((_, opacity)) = self.opacity.checked_sub(1).and_then(|at| OPACITIES.get(at).cloned()) {
      s = s.opacity(opacity);
    }
    if let Some((_, typography)) = self
      .typography
      .checked_sub(1)
      .and_then(|at| TYPOGRAPHY.get(at).cloned())
    {
      s = s.typography(typography);
    }
    s
  }

  /// The calls of a `PartStyle` that write it: `.background(PaletteColor::Primary)`, one a line.
  fn calls(&self) -> Vec<String> {
    let paints = paints();
    let pick = |index: usize, of: &dyn Fn(usize) -> Option<String>| index.checked_sub(1).and_then(of);
    let mut calls = Vec::new();
    if let Some(code) = pick(self.background, &|at| paints.get(at).map(Paint::code)) {
      calls.push(format!(".background({code})"));
    }
    if let Some(code) = pick(self.foreground, &|at| paints.get(at).map(Paint::code)) {
      calls.push(format!(".foreground({code})"));
    }
    if let Some(code) = pick(self.border, &|at| {
      borders()
        .get(at)
        .map(|(size, _, _, role)| format!("Border::inside(BorderSize::{size}, PaletteColor::{role})"))
    }) {
      calls.push(format!(".border({code})"));
    }
    if let Some(name) = pick(self.radius, &|at| RADII.get(at).map(|(name, _)| (*name).to_owned())) {
      calls.push(format!(".radius(RadiusSize::{name})"));
    }
    if let Some(name) = pick(self.padding, &|at| SPACINGS.get(at).map(|(name, _)| (*name).to_owned())) {
      calls.push(format!(".padding_x(SpacingSize::{name})"));
      calls.push(format!(".padding_y(SpacingSize::{name})"));
    }
    if let Some(name) = pick(self.gap, &|at| SPACINGS.get(at).map(|(name, _)| (*name).to_owned())) {
      calls.push(format!(".gap(SpacingSize::{name})"));
    }
    if let Some(name) = pick(self.shadow, &|at| SHADOWS.get(at).map(|(name, _)| (*name).to_owned())) {
      calls.push(format!(".shadow(ShadowStyle::{name})"));
    }
    if let Some(name) = pick(self.opacity, &|at| {
      OPACITIES.get(at).map(|(name, _)| (*name).to_owned())
    }) {
      calls.push(format!(".opacity(OpacityLevel::{name})"));
    }
    if let Some(name) = pick(self.typography, &|at| {
      TYPOGRAPHY.get(at).map(|(name, _)| (*name).to_owned())
    }) {
      calls.push(format!(".typography(TypographyStyle::{name})"));
    }
    calls
  }
}

/// How many selects the section has: the case, and one for each property.
const SELECTS: usize = 10;

/// What a playground keeps for its styling section.
pub struct Styling {
  /// The component of the page, as [`Playground::styled`](crate::playground::Playground::styled) last said it.
  anatomy: Arc<Mutex<Option<Anatomy>>>,
  /// The part being restyled, an index of the component's parts.
  part: Signal<usize>,
  /// The case it is restyled in: see [`Look::when`].
  when: Signal<usize>,
  /// Whether the style goes to every one of the component in the stage's theme, or to the one on the stage.
  all: Signal<bool>,
  /// What each part is restyled with, in each case: one look each.
  looks: Signal<Vec<Look>>,
  /// The component's states it is drawn in, as bits of their indices: `.state(name, true)`.
  previewed: Signal<u64>,
  selects: Vec<(ElementRef, Signal<bool>)>,
  /// The stage's own theme while the style is for every one: the page's look as it was, the rules over it, and
  /// the Rust they were laid as, so that they are laid again only when they change.
  scoped: Mutex<Option<(Theme, ThemeDefinition, String)>>,
}

impl Styling {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      anatomy: Arc::default(),
      part: ctx.signal(0),
      when: ctx.signal(0),
      all: ctx.signal(false),
      looks: ctx.signal(Vec::new()),
      previewed: ctx.signal(0),
      selects: (0..SELECTS).map(|_| (ElementRef::new(), ctx.signal(false))).collect(),
      scoped: Mutex::new(None),
    }
  }

  fn anatomy(&self) -> Option<Anatomy> {
    self
      .anatomy
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .clone()
  }

  /// The component on the stage, said to be a `C`, with what the section asks of the one: its rules, and the states
  /// it is drawn in.
  pub fn styled<C: Styled>(&self, component: C) -> C {
    let anatomy = Anatomy::of::<C::Component>();
    let mut component = component;
    for (index, state) in anatomy.states.names().enumerate() {
      if index < 64 && self.previewed.get() & (1 << index) != 0 {
        component = Styled::state(component, state, true);
      }
    }
    if !self.all.get() {
      let rules = self.recipe(&anatomy);
      if !rules.is_empty() {
        component = Styled::rules(component, &rules);
      }
    }
    *self.anatomy.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(anatomy);
    component
  }

  /// The rules of every look that styles something.
  fn recipe(&self, anatomy: &Anatomy) -> Recipe {
    let parts: Vec<Name> = anatomy.parts.names().collect();
    let states: Vec<Name> = anatomy.states.names().collect();
    let mut recipe = Recipe::new();
    for look in self.looks.get().into_iter().filter(|look| !look.is_empty()) {
      let Some(part) = parts.get(look.part).copied() else {
        continue;
      };
      let when = look.when;
      let style = move |s: PartStyle| look.apply(s);
      recipe = match when {
        0 => recipe.base(part, style),
        1 => recipe.hovered(part, style),
        2 => recipe.pressed(part, style),
        3 => recipe.focus_visible(part, style),
        state => match states.get(state - INTERACTIONS.len()) {
          Some(state) => recipe.state(*state, part, style),
          None => recipe,
        },
      };
    }
    recipe
  }

  /// How many looks style something, and how many states the component is drawn in.
  /// The Rust of what the section lays now, in the form it is applied in, for the Code tab under the stage: nothing
  /// while nothing is restyled or drawn in a state.
  pub fn applied_code(&self) -> Vec<String> {
    let Some(anatomy) = self.anatomy() else {
      return Vec::new();
    };
    if self.changed() == 0 {
      return Vec::new();
    }
    let mut code = vec![String::new()];
    if self.all.get() {
      code.push("// Styling: every one in the theme".to_owned());
      code.extend(self.theme_code(&anatomy));
      if self.previewed.get() != 0 {
        code.push("// and the one on the stage drawn in its states".to_owned());
        code.extend(self.states_code(&anatomy));
      }
    } else {
      code.push("// Styling: the one on the stage".to_owned());
      code.extend(self.instance_code(&anatomy));
    }
    code
  }

  /// The one on the stage in the states it is drawn in.
  fn states_code(&self, anatomy: &Anatomy) -> Vec<String> {
    let mut code = vec![anatomy.binding()];
    for (index, state) in anatomy.states.names().enumerate() {
      if index < 64 && self.previewed.get() & (1 << index) != 0 {
        code.push(format!("    .state({}, true)", anatomy.states.code(state)));
      }
    }
    code
  }

  pub fn changed(&self) -> usize {
    self.looks.get().iter().filter(|look| !look.is_empty()).count() + self.previewed.get().count_ones() as usize
  }

  /// What puts everything back: no look, no state, the first part.
  pub fn reset_handle(&self) -> impl Fn() + Send + Sync + 'static {
    let (looks, previewed, part, when) = (
      self.looks.clone(),
      self.previewed.clone(),
      self.part.clone(),
      self.when.clone(),
    );
    move || {
      looks.set(Vec::new());
      previewed.set(0);
      part.set(0);
      when.set(0);
    }
  }

  /// The stage's own theme while the style is for every one of the component: the page's look with the rules over
  /// the component's recipe, in `appearance`. `None` while there is nothing to lay there.
  pub fn stage_theme(&self, ctx: &mut Ctx, appearance: Appearance) -> Option<Theme> {
    if !self.all.get() {
      return None;
    }
    let anatomy = self.anatomy()?;
    let rules = self.recipe(&anatomy);
    if rules.is_empty() {
      return None;
    }
    let key = self.theme_code(&anatomy).join("\n");
    let mut scoped = self.scoped.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (theme, base, laid) = scoped.get_or_insert_with(|| {
      let base = ctx.theme().definition();
      (Theme::from_definition(base.clone()), base, String::new())
    });
    if *laid != key {
      theme.apply(base.clone());
      (anatomy.patch)(theme, rules);
      *laid = key;
    }
    theme.set_preferred_appearance(Some(appearance));
    Some(theme.clone())
  }

  /// The case of a look, as the Rust of a recipe writes it: `.hovered(`, `.state(SwitchState::On, `.
  fn opener(anatomy: &Anatomy, when: usize) -> String {
    match INTERACTIONS.get(when) {
      Some((_, method)) => format!(".{method}("),
      None => {
        let state = anatomy.states.names().nth(when - INTERACTIONS.len());
        format!(
          ".state({}, ",
          state.map(|state| anatomy.states.code(state)).unwrap_or_default()
        )
      }
    }
  }

  /// A rule as code: `{opener}Part::X, |s| ..)` over as many lines as its calls, each line after the first
  /// indented by `indent`.
  fn rule(anatomy: &Anatomy, opener: &str, look: &Look, indent: &str) -> Vec<String> {
    let part = anatomy
      .parts
      .names()
      .nth(look.part)
      .map(|part| anatomy.parts.code(part))
      .unwrap_or_default();
    let calls = look.calls();
    if calls.len() == 1 {
      return vec![format!("{opener}{part}, |s| s{})", calls[0])];
    }
    let mut lines = vec![format!("{opener}{part}, |s| {{")];
    lines.push(format!("{indent}    s{}", calls[0]));
    for call in &calls[1..] {
      lines.push(format!("{indent}        {call}"));
    }
    lines.push(format!("{indent}}})"));
    lines
  }

  /// The one on the stage, styled: `.state(..)`, `.style(..)`, and `.rules(&Recipe::new()..)` for a case.
  fn instance_code(&self, anatomy: &Anatomy) -> Vec<String> {
    let mut code = vec![anatomy.binding()];
    for (index, state) in anatomy.states.names().enumerate() {
      if index < 64 && self.previewed.get() & (1 << index) != 0 {
        code.push(format!("    .state({}, true)", anatomy.states.code(state)));
      }
    }
    let looks: Vec<Look> = self.looks.get().into_iter().filter(|look| !look.is_empty()).collect();
    for look in looks.iter().filter(|look| look.when == 0) {
      let rule = Self::rule(anatomy, ".style(", look, "    ");
      code.extend(rule.into_iter().map(|line| format!("    {line}")));
    }
    let cased: Vec<&Look> = looks.iter().filter(|look| look.when != 0).collect();
    if !cased.is_empty() {
      code.push("    .rules(&Recipe::new()".to_owned());
      for look in cased {
        let rule = Self::rule(anatomy, &Self::opener(anatomy, look.when), look, "        ");
        code.extend(rule.into_iter().map(|line| format!("        {line}")));
      }
      code.push("    )".to_owned());
    }
    if code.len() == 1 {
      code.push("    // nothing restyled yet: pick a part and a property".to_owned());
    }
    code
  }

  /// Every one of the component in a theme: `theme.patch_recipe::<X>(..)`.
  fn theme_code(&self, anatomy: &Anatomy) -> Vec<String> {
    let looks: Vec<Look> = self.looks.get().into_iter().filter(|look| !look.is_empty()).collect();
    let mut code = vec![format!("theme.patch_recipe::<{}>(|r| {{", anatomy.type_name)];
    if looks.is_empty() {
      code.push("    r // nothing restyled yet".to_owned());
    } else {
      code.push("    r".to_owned());
      for look in &looks {
        let rule = Self::rule(anatomy, &Self::opener(anatomy, look.when), look, "        ");
        code.extend(rule.into_iter().map(|line| format!("        {line}")));
      }
    }
    code.push("});".to_owned());
    code
  }

  /// The section: what the component declares, the part and the case, the properties, the states, and the code.
  /// `force` sets the state the stage draws the component in, an index of the switch over it: a style for the
  /// pointer is shown under it.
  pub fn section(&self, forceable: bool, force: impl Fn(usize) + Send + Sync + 'static) -> Element {
    let Some(anatomy) = self.anatomy() else {
      return Column::new()
        .width(FILL)
        .padding(16.0)
        .spacing(8.0)
        .child(text("No parts to style", 14.0, FontWeight::SemiBold))
        .child(
          quiet(
            "What this page shows is the engine's, not a component of the design system: it has no recipe and no \
             parts. It is styled by its own builder's methods. Getting started › Styling tells how a component of \
             the design system is styled.",
            12.0,
          )
          .width(FILL),
        )
        .into();
    };
    let mut column = Column::new().width(FILL).padding(16.0).spacing(16.0);

    // This one, or all of them.
    let all = self.all.clone();
    column = column.child(prop(
      "applies to",
      if self.all.get() {
        "theme.patch_recipe"
      } else {
        ".style / .rules"
      },
      segmented_fill(
        &[("this one", false), ("all of them", true)],
        &self.all.get(),
        move |every| all.set(every),
      ),
    ));
    column = column.child(
      quiet(
        &if self.all.get() {
          format!(
            "Laid over the recipe of {} in the stage's own theme: every one in that theme follows; the gallery around \
             it does not.",
            anatomy.type_name
          )
        } else {
          format!(
            "Laid on the one {} on the stage, over what the theme gives it.",
            anatomy.type_name
          )
        },
        11.0,
      )
      .width(FILL),
    );

    if anatomy.parts.is_empty() {
      column = column.child(
        quiet(
          &format!("{} declares no parts a recipe styles.", anatomy.type_name),
          12.0,
        )
        .width(FILL),
      );
      return column.into();
    }

    // The parts, each a button: the one chosen is restyled.
    let looks = self.looks.get();
    let chosen_part = self.part.get().min(anatomy.parts.len() - 1);
    let mut parts = Row::new().width(FILL).spacing(4.0).wrap();
    for (index, part) in anatomy.parts.names().enumerate() {
      let styled = looks.iter().any(|look| look.part == index && !look.is_empty());
      let set = self.part.clone();
      let label = if styled {
        format!("{} •", part.as_str())
      } else {
        part.as_str().to_owned()
      };
      parts = parts.child(
        Button::new(&label)
          .size(ButtonSize::Xs)
          .variant(if index == chosen_part {
            ButtonVariant::Primary
          } else {
            ButtonVariant::Outline
          })
          .on_click(move |_| set.set(index)),
      );
    }
    let part_type = anatomy.parts.enum_name.unwrap_or("&str");
    column = column.child(prop(
      "part",
      part_type,
      Column::new().width(FILL).spacing(6.0).child(parts),
    ));

    // The case.
    let states: Vec<Name> = anatomy.states.names().collect();
    let mut cases: Vec<(String, usize)> = INTERACTIONS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| ((*name).to_owned(), index))
      .collect();
    for (index, state) in states.iter().enumerate() {
      cases.push((format!("state: {}", state.as_str()), INTERACTIONS.len() + index));
    }
    let case_options: Vec<(&str, usize)> = cases.iter().map(|(name, index)| (name.as_str(), *index)).collect();
    let when = self.when.get().min(cases.len() - 1);
    let (set_when, previewed) = (self.when.clone(), self.previewed.clone());
    let (anchor, open) = &self.selects[0];
    column = column.child(prop(
      "when",
      "Selector",
      select(anchor, open, &case_options, &when, move |case| {
        set_when.set(case);
        // What the style is for is shown: the pointer's case by the switch over the stage, a state by drawing
        // the component in it.
        if case < INTERACTIONS.len() {
          if forceable {
            force(case);
          }
        } else if case - INTERACTIONS.len() < 64 {
          previewed.update(|bits| *bits |= 1 << (case - INTERACTIONS.len()));
        }
      })
      .width(FILL),
    ));
    column = column.child(divider());

    // The properties of the look of the part in the case.
    let current = looks
      .iter()
      .find(|look| look.part == chosen_part && look.when == when)
      .cloned()
      .unwrap_or(Look {
        part: chosen_part,
        when,
        ..Look::default()
      });
    let edit = |change: fn(&mut Look, usize)| {
      let (looks, current) = (self.looks.clone(), current.clone());
      move |chosen: usize| {
        looks.update(|looks| {
          let at = match looks
            .iter()
            .position(|look| look.part == current.part && look.when == current.when)
          {
            Some(at) => at,
            None => {
              looks.push(Look {
                part: current.part,
                when: current.when,
                ..Look::default()
              });
              looks.len() - 1
            }
          };
          change(&mut looks[at], chosen);
          if looks[at].is_empty() {
            looks.remove(at);
          }
        })
      }
    };
    let paint_names: Vec<String> = paints().iter().map(Paint::label).collect();
    let paint_options: Vec<(&str, usize)> = std::iter::once(("—", 0))
      .chain(
        paint_names
          .iter()
          .enumerate()
          .map(|(index, name)| (name.as_str(), index + 1)),
      )
      .collect();
    let border_names: Vec<String> = borders()
      .iter()
      .map(|(size, _, _, role)| format!("{size} · {role}"))
      .collect();
    let border_options: Vec<(&str, usize)> = std::iter::once(("—", 0))
      .chain(
        border_names
          .iter()
          .enumerate()
          .map(|(index, name)| (name.as_str(), index + 1)),
      )
      .collect();
    fn options<T>(table: &[(&'static str, T)]) -> Vec<(&'static str, usize)> {
      std::iter::once(("—", 0))
        .chain(table.iter().enumerate().map(|(index, (name, _))| (*name, index + 1)))
        .collect()
    }
    let properties: [(&str, &str, Vec<(&str, usize)>, usize, fn(&mut Look, usize)); 9] = [
      (
        "background",
        "PaletteColor | Color",
        paint_options.clone(),
        current.background,
        |look, chosen| look.background = chosen,
      ),
      (
        "foreground",
        "PaletteColor | Color",
        paint_options,
        current.foreground,
        |look, chosen| look.foreground = chosen,
      ),
      ("border", "Border", border_options, current.border, |look, chosen| {
        look.border = chosen
      }),
      (
        "radius",
        "RadiusSize",
        options(&RADII),
        current.radius,
        |look, chosen| look.radius = chosen,
      ),
      (
        "padding",
        "SpacingSize",
        options(&SPACINGS),
        current.padding,
        |look, chosen| look.padding = chosen,
      ),
      ("gap", "SpacingSize", options(&SPACINGS), current.gap, |look, chosen| {
        look.gap = chosen
      }),
      (
        "shadow",
        "ShadowStyle",
        options(&SHADOWS),
        current.shadow,
        |look, chosen| look.shadow = chosen,
      ),
      (
        "opacity",
        "OpacityLevel",
        options(&OPACITIES),
        current.opacity,
        |look, chosen| look.opacity = chosen,
      ),
      (
        "typography",
        "TypographyStyle",
        options(&TYPOGRAPHY),
        current.typography,
        |look, chosen| look.typography = chosen,
      ),
    ];
    for (index, (label, kind, choices, chosen, change)) in properties.into_iter().enumerate() {
      let (anchor, open) = &self.selects[index + 1];
      column = column.child(prop(
        label,
        kind,
        select(anchor, open, &choices, &chosen, edit(change)).width(FILL),
      ));
    }

    // The component's own states, to draw it in.
    if !states.is_empty() {
      column = column.child(divider());
      column = column.child(
        quiet(
          "Draw it in a state of its own (.state(name, true)); the pointer's are the switch over the stage.",
          11.0,
        )
        .width(FILL),
      );
      let mut chips = Row::new().width(FILL).spacing(4.0).wrap();
      for (index, state) in states.iter().enumerate().take(64) {
        let on = self.previewed.get() & (1 << index) != 0;
        let previewed = self.previewed.clone();
        chips = chips.child(
          Button::new(state.as_str())
            .size(ButtonSize::Xs)
            .variant(if on {
              ButtonVariant::Primary
            } else {
              ButtonVariant::Outline
            })
            .on_click(move |_| previewed.update(|bits| *bits ^= 1 << index)),
        );
      }
      column = column.child(prop("states", anatomy.states.enum_name.unwrap_or("&str"), chips));
    }

    // The code, in both forms; the one applied first.
    column = column.child(divider());
    let instance = self.instance_code(&anatomy);
    let theme = self.theme_code(&anatomy);
    let (first, second) = if self.all.get() {
      (("all of them · applied", theme), ("this one", instance))
    } else {
      (("this one · applied", instance), ("all of them", theme))
    };
    for (index, (label, code)) in [first, second].into_iter().enumerate() {
      column = column.child(
        Column::new().width(FILL).spacing(6.0).child(quiet(label, 11.0)).child(
          Column::new()
            .width(FILL)
            .rounded(lurq::app::theme::RadiusSize::Md)
            .border(crate::kit::line())
            .clip()
            .child(code_block(&code).key(format!("styling-code-{index}"))),
        ),
      );
    }
    column.into()
  }
}
