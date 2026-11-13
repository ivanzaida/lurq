//! The design's scales that are not a color, a radius, a spacing, a border
//! size or a shadow: control and icon sizes, densities, durations and
//! easings, opacities, blur radii and the focus ring.
//!
//! Each scale is a role enum with an `Extra` variant for roles an app names,
//! and a table with a public field for each built-in role, as the scales
//! beside it are. The values are the design's
//! (`pencil/lurq-design-system.pen`).

use std::{collections::HashMap, sync::Arc, time::Duration};

use super::{PaletteColor, role_name::RoleName};
use crate::{animation::Easing, node::Outline};

/// Declares a scale: its roles, the table that holds a value for each, and
/// the reading and writing of one by the other.
macro_rules! token_table {
  (
    $(#[$role_doc:meta])* $role:ident,
    $(#[$table_doc:meta])* $table:ident,
    $value:ty,
    $what:literal,
    { $( $(#[$doc:meta])* $variant:ident, $field:ident, $name:literal, $default:expr; )* }
  ) => {
    $(#[$role_doc])*
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum $role {
      $( $(#[$doc])* $variant, )*
      /// A role an app names, stored in the table's `extra`.
      Extra(RoleName),
    }

    impl $role {
      /// A role an app names. The name is interned (see [`RoleName`]).
      pub fn extra(name: impl AsRef<str>) -> Self {
        Self::Extra(RoleName::new(name))
      }

      pub fn as_str(self) -> &'static str {
        match self {
          $( Self::$variant => $name, )*
          Self::Extra(name) => name.as_str(),
        }
      }

      /// The built-in roles, in the order of the scale.
      pub fn all() -> &'static [$role] {
        &[ $( Self::$variant, )* ]
      }
    }

    impl From<&str> for $role {
      fn from(name: &str) -> Self {
        Self::extra(name)
      }
    }

    impl From<Arc<str>> for $role {
      fn from(name: Arc<str>) -> Self {
        Self::extra(name)
      }
    }

    $(#[$table_doc])*
    #[derive(Clone, Debug, PartialEq)]
    pub struct $table {
      $( $(#[$doc])* pub $field: $value, )*
      pub extra: HashMap<Arc<str>, $value>,
    }

    impl $table {
      pub fn new() -> Self {
        Self::default()
      }

      /// Panics when an `Extra` name is not in [`Self::extra`]; see [`Self::try_get`].
      pub fn get(&self, role: impl Into<$role>) -> $value {
        let role = role.into();
        self
          .try_get(role)
          .unwrap_or_else(|| panic!(concat!($what, " not found: {}"), role.as_str()))
      }

      pub fn try_get(&self, role: impl Into<$role>) -> Option<$value> {
        match role.into() {
          $( $role::$variant => Some(self.$field), )*
          $role::Extra(name) => self.extra.get(name.as_str()).copied(),
        }
      }

      pub fn set(&mut self, role: impl Into<$role>, value: $value) {
        match role.into() {
          $( $role::$variant => self.$field = value, )*
          $role::Extra(name) => {
            self.extra.insert(Arc::from(name.as_str()), value);
          }
        }
      }
    }

    impl Default for $table {
      fn default() -> Self {
        Self {
          $( $field: $default, )*
          extra: HashMap::new(),
        }
      }
    }
  };
}

token_table! {
  /// How tall a control is: a button, a field, a select.
  ControlSize,
  /// The height of a control of each size, in logical pixels.
  ThemeControlSizes,
  f32,
  "control size",
  {
    Sm, sm, "sm", 32.0;
    Md, md, "md", 36.0;
    Lg, lg, "lg", 40.0;
  }
}

token_table! {
  /// How large an icon is.
  IconSize,
  /// The side of an icon of each size, in logical pixels.
  ThemeIconSizes,
  f32,
  "icon size",
  {
    Xs, xs, "xs", 12.0;
    Sm, sm, "sm", 14.0;
    Md, md, "md", 16.0;
    Lg, lg, "lg", 20.0;
    Xl, xl, "xl", 24.0;
  }
}

token_table! {
  /// How much room a row of a list, a menu or a table takes: tight for a
  /// tool with much to show, loose for a finger.
  Density,
  /// The height of a row at each density, in logical pixels.
  ThemeDensities,
  f32,
  "density",
  {
    Compact, compact, "compact", 28.0;
    /// The density of a desktop app.
    Default, default, "default", 36.0;
    Comfortable, comfortable, "comfortable", 44.0;
    /// A row a finger can press.
    Touch, touch, "touch", 48.0;
  }
}

token_table! {
  /// How long a change takes to show.
  MotionDuration,
  /// The length of each duration.
  ThemeDurations,
  Duration,
  "duration",
  {
    /// A hover, a press: a change under the pointer.
    Fast, fast, "fast", Duration::from_millis(100);
    /// Most changes: a menu opening, a switch moving.
    Base, base, "base", Duration::from_millis(160);
    /// Something larger coming in: a dialog, a sheet.
    Slow, slow, "slow", Duration::from_millis(240);
    /// A whole view changing.
    Slower, slower, "slower", Duration::from_millis(400);
  }
}

token_table! {
  /// How a change moves from its start to its end.
  MotionEasing,
  /// The curve of each easing.
  ThemeEasings,
  Easing,
  "easing",
  {
    /// What stays on screen and changes: quick at first, settling.
    Standard, standard, "standard", Easing::CubicBezier { x1: 0.2, y1: 0.0, x2: 0.0, y2: 1.0 };
    /// What comes in: at full speed from the first frame, slowing to rest.
    Enter, enter, "enter", Easing::CubicBezier { x1: 0.0, y1: 0.0, x2: 0.0, y2: 1.0 };
    /// What leaves: from rest, gathering speed.
    Exit, exit, "exit", Easing::CubicBezier { x1: 0.3, y1: 0.0, x2: 1.0, y2: 1.0 };
  }
}

token_table! {
  /// How much of something shows when it is held back.
  OpacityLevel,
  /// The opacity of each level, from 0 to 1.
  ThemeOpacities,
  f32,
  "opacity level",
  {
    /// A control that takes no input.
    Disabled, disabled, "disabled", 0.5;
    /// Something that says less than what is beside it.
    Muted, muted, "muted", 0.7;
  }
}

token_table! {
  /// How much what is behind glass is blurred.
  BlurSize,
  /// The radius of each blur, in logical pixels.
  ThemeBlurs,
  f32,
  "blur size",
  {
    Thin, thin, "thin", 8.0;
    Regular, regular, "regular", 20.0;
    Thick, thick, "thick", 40.0;
  }
}

/// The ring around the control the keyboard is on.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeFocusRing {
  /// How thick the line is, in logical pixels.
  pub width: f32,
  /// How far the line is from the control's box.
  pub offset: f32,
  pub color: PaletteColor,
}

impl ThemeFocusRing {
  /// The ring as an outline, for `.focus_visible(|style| style.outline(...))`.
  pub fn outline(&self) -> Outline {
    Outline::new(self.width, self.color.clone()).offset(self.offset)
  }
}

impl Default for ThemeFocusRing {
  fn default() -> Self {
    Self {
      width: 2.0,
      offset: 2.0,
      color: PaletteColor::Ring,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{ControlSize, MotionDuration, ThemeControlSizes, ThemeDurations};

  #[test]
  fn a_table_holds_a_value_for_each_role_and_for_the_roles_an_app_names() {
    let mut sizes = ThemeControlSizes::default();
    assert_eq!(sizes.get(ControlSize::Md), 36.0);
    sizes.set(ControlSize::Md, 44.0);
    assert_eq!(sizes.md, 44.0);

    assert_eq!(sizes.try_get("hero"), None);
    sizes.set("hero", 56.0);
    assert_eq!(sizes.get(ControlSize::extra("hero")), 56.0);
    assert_eq!(ControlSize::extra("hero").as_str(), "hero");
  }

  #[test]
  fn the_roles_of_a_scale_are_listed_in_its_order() {
    let durations = ThemeDurations::default();
    let lengths: Vec<_> = MotionDuration::all()
      .iter()
      .map(|role| durations.get(*role).as_millis())
      .collect();
    assert_eq!(lengths, [100, 160, 240, 400]);
  }
}
