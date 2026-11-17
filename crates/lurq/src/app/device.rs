//! A phone or a tablet a desktop app is run as: [`App::emulate`](crate::app::App::emulate), and `LURQ_DEVICE` for
//! any app.

use std::sync::Arc;

use super::window::SafeAreaInsets;

/// A device whose screen and fingers a desktop window emulates: its screen's size, its scale and its safe area.
///
/// While an app emulates one ([`App::emulate`](crate::app::App::emulate), or `LURQ_DEVICE=phone` in its
/// environment) its window takes the device's logical size, the safe area's insets reach what reads them
/// ([`Ctx::safe_area`](crate::app::ctx::Ctx::safe_area)) as a phone's do, the app's pointer is pinned
/// [`Coarse`](crate::app::Pointer::Coarse), and the mouse is fingers: it reaches the app through the same path a
/// touch screen's fingers take ([`Tree::finger`](crate::app::Tree::finger)), and never as a mouse.
///
/// ```
/// use lurq::app::Device;
///
/// let phone = Device::phone();
/// assert_eq!(phone.size, (390.0, 844.0));
/// assert_eq!(Device::named("tablet"), Some(Device::tablet()));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
  /// What it is called: `phone`, `tablet`, or the app's own name.
  pub name: Arc<str>,
  /// Its screen, upright, in logical pixels.
  pub size: (f32, f32),
  /// Its physical pixels to a logical one: 3 on an iPhone. The window is drawn at its monitor's scale, so this
  /// is what the device would have, for an app that reads it.
  pub scale: f32,
  /// What its screen's edges keep for the system, in logical pixels: the status bar or the notch at the top,
  /// the home indicator at the bottom.
  pub safe_area: SafeAreaInsets,
}

impl Device {
  /// An iPhone of today (an iPhone 14 or 15): 390 by 844, at 3x, a status bar of 47 at the top and a home
  /// indicator of 34 at the bottom.
  pub fn phone() -> Self {
    Self {
      name: Arc::from("phone"),
      size: (390.0, 844.0),
      scale: 3.0,
      safe_area: SafeAreaInsets {
        top: 47.0,
        right: 0.0,
        bottom: 34.0,
        left: 0.0,
        keyboard: 0.0,
      },
    }
  }

  /// An iPad of today (an iPad Air): 820 by 1180, at 2x, a status bar of 24 at the top and a home indicator of 20
  /// at the bottom.
  pub fn tablet() -> Self {
    Self {
      name: Arc::from("tablet"),
      size: (820.0, 1180.0),
      scale: 2.0,
      safe_area: SafeAreaInsets {
        top: 24.0,
        right: 0.0,
        bottom: 20.0,
        left: 0.0,
        keyboard: 0.0,
      },
    }
  }

  /// The device a name says: `phone` or `tablet`, in any case; `None` for another.
  pub fn named(name: &str) -> Option<Self> {
    match name.trim().to_ascii_lowercase().as_str() {
      "phone" => Some(Self::phone()),
      "tablet" => Some(Self::tablet()),
      _ => None,
    }
  }

  /// The device `LURQ_DEVICE` names in the app's environment, if it names one. Not in the browser, which has no
  /// environment.
  pub fn from_env() -> Option<Self> {
    #[cfg(target_arch = "wasm32")]
    {
      None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      std::env::var("LURQ_DEVICE").ok().as_deref().and_then(Self::named)
    }
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for Device {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}
