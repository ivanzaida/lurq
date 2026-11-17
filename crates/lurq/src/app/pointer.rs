//! What the person points with: a fine pointer (a mouse, a trackpad, a pen) or a coarse one (a finger). It is a
//! fact about the device in the person's hand, not a choice of the theme: the theme says how large things are
//! (its density), the pointer says whether a component is dressed for a finger (`ui::touch_form`).

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};

use crate::core::Signal;

/// What the person points with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Pointer {
  /// A mouse, a trackpad or a pen: a point the size of a pixel, which can hover. A component takes its desktop
  /// form.
  #[default]
  Fine,
  /// A finger: a point the size of a fingertip, which cannot hover. A component that has a touch form takes it.
  Coarse,
}

impl Pointer {
  /// What the person points with when the app starts, before any input: [`Coarse`](Self::Coarse) on iOS and
  /// Android, [`Fine`](Self::Fine) on Windows, macOS and Linux, and in the browser what the page's
  /// `(pointer: coarse)` media query says of the device's primary pointer.
  pub fn platform_default() -> Self {
    Self::starting_on(PHONE, browser_says_coarse())
  }

  /// What the person points with at the start on a phone's platform (`phone`), or on another where the browser
  /// may say what its primary pointer is (`browser_coarse`).
  pub(crate) fn starting_on(phone: bool, browser_coarse: Option<bool>) -> Self {
    match (phone, browser_coarse) {
      (true, _) | (false, Some(true)) => Self::Coarse,
      _ => Self::Fine,
    }
  }

  fn code(self) -> u8 {
    match self {
      Self::Fine => 0,
      Self::Coarse => 1,
    }
  }

  fn of_code(code: u8) -> Self {
    if code == 1 { Self::Coarse } else { Self::Fine }
  }
}

/// Whether the platform is a phone's: iOS and Android.
const PHONE: bool = cfg!(any(target_os = "ios", target_os = "android"));

/// What the browser's `(pointer: coarse)` media query says, in the browser; `None` elsewhere.
#[cfg(target_arch = "wasm32")]
fn browser_says_coarse() -> Option<bool> {
  web_sys::window()
    .and_then(|window| window.match_media("(pointer: coarse)").ok().flatten())
    .map(|query| query.matches())
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_says_coarse() -> Option<bool> {
  None
}

/// The app's pointer: what [`App::set_pointer`](crate::app::App::set_pointer) pins, and what the last input moved
/// it to while it follows. Read so that what reads it in a render is rendered again when it changes.
#[derive(Clone)]
pub(crate) struct AppPointer {
  shared: Arc<PointerShared>,
  version: Signal<u64>,
}

struct PointerShared {
  /// The pointer, as `Pointer::code`.
  now: AtomicU8,
  /// The app set it: input does not move it.
  pinned: AtomicBool,
  /// Input moves it, while it is not pinned.
  follows: AtomicBool,
  /// How many times it was set by the app or a test rather than by input: what an open component that holds its
  /// form against a change of input gives way to.
  told: AtomicU64,
}

impl AppPointer {
  pub fn new() -> Self {
    Self::starting_at(Pointer::platform_default())
  }

  pub fn starting_at(pointer: Pointer) -> Self {
    Self {
      shared: Arc::new(PointerShared {
        now: AtomicU8::new(pointer.code()),
        pinned: AtomicBool::new(false),
        follows: AtomicBool::new(true),
        told: AtomicU64::new(0),
      }),
      version: Signal::new(0),
    }
  }

  /// The pointer, read so that the render reading it is run again when it changes.
  pub fn get(&self) -> Pointer {
    let _ = self.version.get();
    self.now()
  }

  /// The pointer, not read by the render.
  pub fn now(&self) -> Pointer {
    Pointer::of_code(self.shared.now.load(Ordering::Relaxed))
  }

  /// How many times the pointer was told rather than followed, read so that the render reading it is run again.
  pub fn told(&self) -> u64 {
    let _ = self.version.get();
    self.shared.told.load(Ordering::Relaxed)
  }

  /// The pointer, whether the app pinned it, and whether input moves it: for tooling.
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub fn state(&self) -> (Pointer, bool, bool) {
    (
      self.now(),
      self.shared.pinned.load(Ordering::Relaxed),
      self.shared.follows.load(Ordering::Relaxed),
    )
  }

  /// Sets it, as the app or a test does: at once for what is open too. `pin` keeps input from moving it.
  pub fn tell(&self, pointer: Pointer, pin: bool) {
    self.shared.pinned.store(pin, Ordering::Relaxed);
    self.shared.told.fetch_add(1, Ordering::Relaxed);
    self.shared.now.store(pointer.code(), Ordering::Relaxed);
    self.bump();
  }

  /// Whether input moves it while it is not pinned. Following again lets go of a pin.
  pub fn follow(&self, follows: bool) {
    self.shared.follows.store(follows, Ordering::Relaxed);
    if follows {
      self.shared.pinned.store(false, Ordering::Relaxed);
    }
  }

  /// Input of a kind came: a finger is `Coarse`, a mouse or a trackpad `Fine`. Only the first input of the other
  /// kind changes anything; one of the kind it is at costs two loads.
  pub fn input(&self, pointer: Pointer) {
    let shared = &self.shared;
    if shared.now.load(Ordering::Relaxed) == pointer.code()
      || shared.pinned.load(Ordering::Relaxed)
      || !shared.follows.load(Ordering::Relaxed)
    {
      return;
    }
    shared.now.store(pointer.code(), Ordering::Relaxed);
    self.bump();
  }

  fn bump(&self) {
    self.version.update(|version| *version = version.wrapping_add(1));
  }
}

#[cfg(test)]
mod tests {
  use super::{AppPointer, Pointer};

  #[test]
  fn a_phone_starts_coarse_a_desktop_fine_and_the_browser_as_its_media_query_says() {
    assert_eq!(Pointer::starting_on(true, None), Pointer::Coarse, "iOS and Android");
    assert_eq!(
      Pointer::starting_on(false, None),
      Pointer::Fine,
      "Windows, macOS, Linux"
    );
    assert_eq!(
      Pointer::starting_on(false, Some(true)),
      Pointer::Coarse,
      "a browser on a phone"
    );
    assert_eq!(
      Pointer::starting_on(false, Some(false)),
      Pointer::Fine,
      "a browser on a desktop"
    );
    let phone = cfg!(any(target_os = "ios", target_os = "android"));
    assert_eq!(Pointer::platform_default(), Pointer::starting_on(phone, None));
  }

  #[test]
  fn input_moves_it_unless_it_is_pinned_or_does_not_follow() {
    let pointer = AppPointer::starting_at(Pointer::Fine);
    pointer.input(Pointer::Coarse);
    assert_eq!(pointer.now(), Pointer::Coarse, "a finger");
    pointer.input(Pointer::Fine);
    assert_eq!(pointer.now(), Pointer::Fine, "a mouse");

    pointer.tell(Pointer::Coarse, true);
    pointer.input(Pointer::Fine);
    assert_eq!(pointer.now(), Pointer::Coarse, "pinned");

    pointer.follow(true);
    pointer.input(Pointer::Fine);
    assert_eq!(pointer.now(), Pointer::Fine, "following again lets go of the pin");

    pointer.follow(false);
    pointer.input(Pointer::Coarse);
    assert_eq!(pointer.now(), Pointer::Fine, "not following");
  }

  #[test]
  fn only_a_telling_counts_as_told() {
    let pointer = AppPointer::starting_at(Pointer::Fine);
    pointer.input(Pointer::Coarse);
    assert_eq!(pointer.told(), 0);
    pointer.tell(Pointer::Fine, false);
    assert_eq!(pointer.told(), 1);
    pointer.input(Pointer::Coarse);
    assert_eq!(
      pointer.now(),
      Pointer::Coarse,
      "a telling that does not pin leaves input to move it"
    );
  }
}
