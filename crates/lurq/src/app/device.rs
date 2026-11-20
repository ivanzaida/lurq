//! A phone or a tablet a desktop app is run as: [`App::emulate`](crate::app::App::emulate), and `LURQ_DEVICE` for
//! any app.

use std::sync::Arc;

use super::window::SafeAreaInsets;

/// A device whose screen and fingers a desktop window emulates: its screen's size, its scale and its safe area.
///
/// While an app emulates one ([`App::emulate`](crate::app::App::emulate), or `LURQ_DEVICE=phone` in its
/// environment) its window takes the device's logical size, the app's pointer is pinned
/// [`Coarse`](crate::app::Pointer::Coarse), and the mouse is fingers: it reaches the app through the same path a
/// touch screen's fingers take ([`Tree::finger`](crate::app::Tree::finger)), and never as a mouse.
///
/// A desktop's window has no status bar and no home indicator, so its safe area is none, as the presets have it.
/// An app that wants to see where a phone's would be gives the device its insets
/// ([`with_safe_area`](Self::with_safe_area), or `LURQ_DEVICE_INSETS=1` beside `LURQ_DEVICE`): they then reach
/// what reads them ([`Ctx::safe_area`](crate::app::ctx::Ctx::safe_area)) as a phone's do.
///
/// ```
/// use lurq::app::Device;
///
/// let phone = Device::phone();
/// assert_eq!(phone.size, (390.0, 844.0));
/// assert_eq!(phone.safe_area.top, 0.0);
/// let with_bars = Device::phone().with_safe_area(Device::PHONE_SAFE_AREA);
/// assert_eq!((with_bars.safe_area.top, with_bars.safe_area.bottom), (47.0, 34.0));
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
  /// What the window keeps for the system, in logical pixels: none unless the app gives the device's
  /// ([`with_safe_area`](Self::with_safe_area)).
  pub safe_area: SafeAreaInsets,
}

const NO_INSETS: SafeAreaInsets = SafeAreaInsets {
  top: 0.0,
  right: 0.0,
  bottom: 0.0,
  left: 0.0,
  keyboard: 0.0,
};

impl Device {
  /// What the screen of an iPhone of today keeps for the system: a status bar of 47 at the top and a home
  /// indicator of 34 at the bottom.
  pub const PHONE_SAFE_AREA: SafeAreaInsets = SafeAreaInsets {
    top: 47.0,
    right: 0.0,
    bottom: 34.0,
    left: 0.0,
    keyboard: 0.0,
  };

  /// What the screen of an iPad of today keeps for the system: a status bar of 24 at the top and a home
  /// indicator of 20 at the bottom.
  pub const TABLET_SAFE_AREA: SafeAreaInsets = SafeAreaInsets {
    top: 24.0,
    right: 0.0,
    bottom: 20.0,
    left: 0.0,
    keyboard: 0.0,
  };

  /// An iPhone of today (an iPhone 14 or 15): 390 by 844, at 3x. No safe area: see
  /// [`PHONE_SAFE_AREA`](Self::PHONE_SAFE_AREA).
  pub fn phone() -> Self {
    Self {
      name: Arc::from("phone"),
      size: (390.0, 844.0),
      scale: 3.0,
      safe_area: NO_INSETS,
    }
  }

  /// An iPad of today (an iPad Air): 820 by 1180, at 2x. No safe area: see
  /// [`TABLET_SAFE_AREA`](Self::TABLET_SAFE_AREA).
  pub fn tablet() -> Self {
    Self {
      name: Arc::from("tablet"),
      size: (820.0, 1180.0),
      scale: 2.0,
      safe_area: NO_INSETS,
    }
  }

  /// This device with `safe_area` kept for the system, as its screen keeps it: to see on a desktop where a
  /// phone's status bar and home indicator would be.
  pub fn with_safe_area(mut self, safe_area: SafeAreaInsets) -> Self {
    self.safe_area = safe_area;
    self
  }

  /// The device a name says: `phone` or `tablet`, in any case; `None` for another.
  pub fn named(name: &str) -> Option<Self> {
    match name.trim().to_ascii_lowercase().as_str() {
      "phone" => Some(Self::phone()),
      "tablet" => Some(Self::tablet()),
      _ => None,
    }
  }

  /// The insets of the screen of the device a name says: [`PHONE_SAFE_AREA`](Self::PHONE_SAFE_AREA) or
  /// [`TABLET_SAFE_AREA`](Self::TABLET_SAFE_AREA).
  fn screen_insets(name: &str) -> Option<SafeAreaInsets> {
    match name.trim().to_ascii_lowercase().as_str() {
      "phone" => Some(Self::PHONE_SAFE_AREA),
      "tablet" => Some(Self::TABLET_SAFE_AREA),
      _ => None,
    }
  }

  /// The device `LURQ_DEVICE` names in the app's environment, if it names one, with its screen's safe area where
  /// `LURQ_DEVICE_INSETS` is `1`. Not in the browser, which has no environment.
  pub fn from_env() -> Option<Self> {
    #[cfg(target_arch = "wasm32")]
    {
      None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      let name = std::env::var("LURQ_DEVICE").ok()?;
      let insets = std::env::var("LURQ_DEVICE_INSETS").is_ok_and(|on| on.trim() == "1");
      Self::named_with(&name, insets)
    }
  }

  /// The device `name` says, with its screen's safe area where `insets`.
  fn named_with(name: &str, insets: bool) -> Option<Self> {
    let device = Self::named(name)?;
    Some(match Self::screen_insets(name).filter(|_| insets) {
      Some(safe_area) => device.with_safe_area(safe_area),
      None => device,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_device_on_a_desktop_keeps_nothing_for_the_system_unless_told() {
    assert_eq!(Device::phone().safe_area, NO_INSETS);
    assert_eq!(Device::named_with("phone", false), Some(Device::phone()));
    assert_eq!(
      Device::named_with("Phone", true).map(|device| device.safe_area),
      Some(Device::PHONE_SAFE_AREA)
    );
    assert_eq!(
      Device::named_with("tablet", true).map(|device| device.safe_area),
      Some(Device::TABLET_SAFE_AREA)
    );
    assert_eq!(Device::named_with("watch", true), None);
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for Device {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}
