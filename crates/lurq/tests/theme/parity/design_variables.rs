//! The neutral theme has the values of the design's variables.
//!
//! `design/tokens/variables.tsv` lists the variables of
//! `pencil/lurq-design-system.pen`. Every one of them is either a value of
//! [`ThemeDefinition::neutral`], checked here, or named below as not the
//! theme's, so a variable added to the board cannot be missed and a value
//! changed on one side cannot drift from the other.

use std::{path::Path, time::Duration};

use lurq::{
  animation::Easing,
  app::theme::{
    BlurSize, BorderSize, ControlSize, Density, IconSize, MotionDuration, MotionEasing, OpacityLevel, RadiusSize,
    SpacingSize, ThemeDefinition, ThemePalette,
  },
  node::{color::Color, dimension::Dimension},
};

/// Variables of the board that the theme does not hold.
const NOT_THE_THEMES: &[(&str, &str)] = &[
  ("sheet", "the backdrop of the board's own sheets"),
  (
    "font-sans",
    "the design is drawn in Inter; lurq bundles no font and uses the system's (D4)",
  ),
  ("font-mono", "as font-sans"),
  (
    "z-sticky",
    "lurq has no z-index: what is painted later is on top, and layers are the overlay host's",
  ),
  ("z-dropdown", "as z-sticky"),
  ("z-popover", "as z-sticky"),
  ("z-overlay", "as z-sticky"),
  ("z-modal", "as z-sticky"),
  ("z-toast", "as z-sticky"),
  ("z-tooltip", "as z-sticky"),
];

/// The names of the board that differ from the role's.
const RENAMED: &[(&str, &str)] = &[
  // D3: in lurq `Accent` was the brand color; the design's `accent` is a subtle hover fill.
  ("accent", "highlight"),
  ("accent-foreground", "highlight_foreground"),
];

struct Variable {
  kind: String,
  name: String,
  values: Vec<String>,
}

/// The variables, or `None` where the design is not there: the published
/// crate carries its tests without the repository around them.
fn variables() -> Option<Vec<Variable>> {
  let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/tokens/variables.tsv");
  let text = std::fs::read_to_string(path).ok()?;
  let variables = text
    .lines()
    .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
    .map(|line| {
      let mut fields = line.split('\t').map(str::to_owned);
      Variable {
        kind: fields.next().expect("a type"),
        name: fields.next().expect("a name"),
        values: fields.collect(),
      }
    })
    .collect();
  Some(variables)
}

fn role_name(variable: &str) -> String {
  RENAMED
    .iter()
    .find(|(board, _)| *board == variable)
    .map(|(_, role)| (*role).to_owned())
    .unwrap_or_else(|| variable.replace('-', "_"))
}

fn palette_color(palette: &ThemePalette, name: &str) -> Option<Color> {
  ThemePalette::roles()
    .into_iter()
    .find(|role| role.as_str() == name)
    .map(|role| palette.get(role))
}

fn number(variable: &Variable) -> f32 {
  variable.values[0].parse().expect("a number")
}

fn bezier(text: &str) -> Easing {
  let inner = text
    .strip_prefix("cubic-bezier(")
    .and_then(|rest| rest.strip_suffix(')'))
    .expect("a cubic-bezier");
  let points: Vec<f64> = inner
    .split(',')
    .map(|part| part.trim().parse().expect("a number"))
    .collect();
  Easing::CubicBezier {
    x1: points[0],
    y1: points[1],
    x2: points[2],
    y2: points[3],
  }
}

/// The value the neutral theme has for a number of the board, by the scale
/// its name begins with.
fn theme_number(theme: &ThemeDefinition, name: &str) -> Option<f32> {
  let (scale, role) = name.split_once('-')?;
  let px = |dimension: Dimension| match dimension {
    Dimension::Px(px) => px,
    other => panic!("spacing is in pixels, not {other:?}"),
  };
  Some(match (scale, role) {
    ("radius", "sm") => theme.radii.get(RadiusSize::Sm),
    ("radius", "md") => theme.radii.get(RadiusSize::Md),
    ("radius", "lg") => theme.radii.get(RadiusSize::Lg),
    ("radius", "xl") => theme.radii.get(RadiusSize::Xl),
    ("radius", "full") => theme.radii.get(RadiusSize::Full),
    ("spacing", "xs") => px(theme.spacing.get(SpacingSize::Xs)),
    ("spacing", "sm") => px(theme.spacing.get(SpacingSize::Sm)),
    ("spacing", "md") => px(theme.spacing.get(SpacingSize::Md)),
    ("spacing", "lg") => px(theme.spacing.get(SpacingSize::Lg)),
    ("spacing", "xl") => px(theme.spacing.get(SpacingSize::Xl)),
    // The largest step is `Section` in lurq.
    ("spacing", "2xl") => px(theme.spacing.get(SpacingSize::Section)),
    ("control", "sm") => theme.control_sizes.get(ControlSize::Sm),
    ("control", "md") => theme.control_sizes.get(ControlSize::Md),
    ("control", "lg") => theme.control_sizes.get(ControlSize::Lg),
    ("border", "sm") => theme.border_sizes.get(BorderSize::Sm),
    ("border", "md") => theme.border_sizes.get(BorderSize::Md),
    ("border", "lg") => theme.border_sizes.get(BorderSize::Lg),
    ("icon", "xs") => theme.icon_sizes.get(IconSize::Xs),
    ("icon", "sm") => theme.icon_sizes.get(IconSize::Sm),
    ("icon", "md") => theme.icon_sizes.get(IconSize::Md),
    ("icon", "lg") => theme.icon_sizes.get(IconSize::Lg),
    ("icon", "xl") => theme.icon_sizes.get(IconSize::Xl),
    ("blur", "thin") => theme.blurs.get(BlurSize::Thin),
    ("blur", "regular") => theme.blurs.get(BlurSize::Regular),
    ("blur", "thick") => theme.blurs.get(BlurSize::Thick),
    ("density", "compact") => theme.densities.get(Density::Compact),
    ("density", "default") => theme.densities.get(Density::Default),
    ("density", "comfortable") => theme.densities.get(Density::Comfortable),
    ("density", "touch") => theme.densities.get(Density::Touch),
    ("opacity", "disabled") => theme.opacities.get(OpacityLevel::Disabled),
    ("opacity", "muted") => theme.opacities.get(OpacityLevel::Muted),
    ("duration", role) => {
      let role = match role {
        "fast" => MotionDuration::Fast,
        "base" => MotionDuration::Base,
        "slow" => MotionDuration::Slow,
        "slower" => MotionDuration::Slower,
        _ => return None,
      };
      theme.durations.get(role).as_secs_f32() * 1000.0
    }
    ("focus", "ring-width") => theme.focus_ring.width,
    ("focus", "ring-offset") => theme.focus_ring.offset,
    _ => return None,
  })
}

#[test]
fn the_neutral_theme_has_the_value_of_every_variable_of_the_design() {
  let Some(variables) = variables() else {
    return;
  };
  assert!(variables.len() > 80, "the file was read: {} variables", variables.len());
  let theme = ThemeDefinition::neutral();
  let dark = theme.dark.clone().expect("the neutral theme has a dark palette");

  for variable in &variables {
    let name = variable.name.as_str();
    if NOT_THE_THEMES.iter().any(|(skipped, _)| *skipped == name) {
      continue;
    }
    match variable.kind.as_str() {
      "color" => {
        let role = role_name(name);
        let light = palette_color(&theme.light, &role).unwrap_or_else(|| panic!("no palette role for `{name}`"));
        assert_eq!(light, Color::from_hex(&variable.values[0]), "{name}, light");
        assert_eq!(
          palette_color(&dark, &role).unwrap(),
          Color::from_hex(&variable.values[1]),
          "{name}, dark"
        );
      }
      "number" => {
        let value = theme_number(&theme, name).unwrap_or_else(|| panic!("no theme value for `{name}`"));
        assert_eq!(value, number(variable), "{name}");
      }
      "string" => {
        let role = match name {
          "ease-standard" => MotionEasing::Standard,
          "ease-enter" => MotionEasing::Enter,
          "ease-exit" => MotionEasing::Exit,
          _ => panic!("no theme value for `{name}`"),
        };
        assert_eq!(theme.easings.get(role), bezier(&variable.values[0]), "{name}");
      }
      kind => panic!("`{name}` is a {kind}"),
    }
  }
}

#[test]
fn every_role_of_the_palette_is_a_variable_of_the_design() {
  let Some(variables) = variables() else {
    return;
  };
  let on_the_board: Vec<String> = variables
    .iter()
    .filter(|variable| variable.kind == "color")
    .map(|variable| role_name(&variable.name))
    .collect();
  for role in ThemePalette::roles() {
    // A deprecated role is one of lurq 0.61, not of the design.
    if role.replacement().is_some() {
      continue;
    }
    assert!(
      on_the_board.iter().any(|name| name == role.as_str()),
      "`{}` is not a variable of the design",
      role.as_str()
    );
  }
}

#[test]
fn a_duration_of_the_design_is_in_milliseconds() {
  assert_eq!(
    ThemeDefinition::neutral().durations.get(MotionDuration::Fast),
    Duration::from_millis(100)
  );
}
