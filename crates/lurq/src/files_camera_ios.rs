//! On iOS, the camera of a file dialog that takes several files: lurq's own, on AVFoundation, as Telegram's and the
//! Camera app's are. It stays open until the person is done.
//!
//! - **The picture**, 3 by 4 under the top bar: a pinch zooms it, a tap focuses and exposes where it is.
//! - **The lenses**, as the Camera app has them (0.5×, 1×, 2× or the telephoto's), from the device's own switch-over
//!   factors (`virtualDeviceSwitchOverVideoZoomFactors`); the one in use says how far it is zoomed.
//! - **The flash** (auto, on, off), **the other camera**, front or back, and **the shutter**, which takes a photo at
//!   once, with the system's sound, in the orientation the phone is held.
//! - **The photos taken**: the last beside the shutter, with how many there are. A tap opens them in a viewer as a
//!   gallery's: full screen, swiped one to the next, pinched or tapped twice to zoom, "3 of 5", and a button that takes
//!   the one shown out.
//! - **Done** gives them all, once those still being taken are kept; **Cancel** gives none.
//!
//! The words are the locale's, as the dialog was given them. Each photo is a JPEG of the temporary folder. Where the
//! app may not use the camera, or there is none to use, the system's camera is shown, which says why.

use std::{ffi::c_void, path::PathBuf, sync::Mutex};

use objc2::{
  ClassType, Encode, Encoding, MainThreadMarker, msg_send,
  rc::{Allocated, Retained},
  runtime::{AnyClass, AnyObject, AnyProtocol, Bool, ClassBuilder, Sel},
  sel,
};
use objc2_foundation::{NSArray, NSObject, NSString};

use super::{
  Answering, DroppedFile, SourceWords,
  native::{Point, Rect, Size, photo_path, system_camera, top_controller},
};

type Obj = Retained<AnyObject>;

#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {
  static AVMediaTypeVideo: *const AnyObject;
  static AVCaptureSessionPresetPhoto: *const AnyObject;
  static AVLayerVideoGravityResizeAspectFill: *const AnyObject;
  static AVCaptureDeviceTypeBuiltInTripleCamera: *const AnyObject;
  static AVCaptureDeviceTypeBuiltInDualWideCamera: *const AnyObject;
  static AVCaptureDeviceTypeBuiltInDualCamera: *const AnyObject;
  static AVCaptureDeviceTypeBuiltInWideAngleCamera: *const AnyObject;
}

/// `AVCaptureDevicePosition`.
const BACK: isize = 1;
const FRONT: isize = 2;
/// `AVCaptureFlashMode`.
const FLASH_OFF: isize = 0;
const FLASH_ON: isize = 1;
const FLASH_AUTO: isize = 2;
/// `UIControlEventTouchUpInside`.
const TOUCH_UP_INSIDE: usize = 1 << 6;
/// `UIGestureRecognizerState`.
const BEGAN: isize = 1;
const CHANGED: isize = 2;
/// The tag of the viewer's row of pages; a page is `PAGE` and its place.
const PAGER: isize = 1;
const PAGE: isize = 100;
/// The side of the last photo beside the shutter, and of what it is drawn from.
const STACK: f64 = 54.0;

/// The camera that is open: its session, what it shows, and the photos taken.
struct Camera {
  answering: Answering,
  words: SourceWords,
  delegate: Obj,
  controller: Obj,
  session: Obj,
  output: Obj,
  input: Obj,
  device: Obj,
  back: bool,
  /// What the zoom factor is where the lens of 1× is, and the lenses, in the factors shown (0.5, 1, 2).
  base: f64,
  lenses: Vec<f64>,
  pinched_from: f64,
  flash: isize,
  /// The last orientation the phone was held in, face up or down being none.
  held: isize,
  taken: Vec<(DroppedFile, Option<Obj>)>,
  /// Photos asked for and not yet kept; Done waits for them.
  pending: usize,
  finishing: bool,
  views: Views,
  viewer: Option<Viewer>,
}

/// The views of the camera that change.
struct Views {
  preview: Obj,
  layer: Obj,
  pill: Obj,
  lens_buttons: Vec<Obj>,
  flash: Obj,
  last: Obj,
  count: Obj,
  done: Obj,
  blink: Obj,
  focus: Obj,
}

/// The photos taken, full screen.
struct Viewer {
  controller: Obj,
  pager: Obj,
  title: Obj,
  /// Each page, its picture, and whether the picture is the photo's whole (near the one shown) or its small one.
  pages: Vec<(Obj, Obj, bool)>,
  at: usize,
  width: f64,
}

// SAFETY: touched on the main thread alone.
unsafe impl Send for Camera {}

static CAMERA: Mutex<Option<Camera>> = Mutex::new(None);

/// What is handed to another thread: an object of Objective-C's whose count is kept atomically, and which is used
/// there as its class allows on any thread.
struct Sending<T>(T);
// SAFETY: see above.
unsafe impl<T> Send for Sending<T> {}

impl<T> Sending<T> {
  /// What is held, taken by the whole: a closure that calls this captures the whole, which is `Send`, and not
  /// its field.
  fn into_inner(self) -> T {
    self.0
  }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Insets {
  top: f64,
  left: f64,
  bottom: f64,
  right: f64,
}

// SAFETY: UIKit's `UIEdgeInsets`, four `CGFloat`s.
unsafe impl Encode for Insets {
  const ENCODING: Encoding = Encoding::Struct(
    "UIEdgeInsets",
    &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING],
  );
}

/// A `CGColorRef`, as a layer's border takes it.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct CgColor(*mut c_void);

// SAFETY: a pointer to CoreGraphics' opaque color.
unsafe impl Encode for CgColor {
  const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
  Rect {
    origin: Point { x, y },
    size: Size { width, height },
  }
}

fn ns(text: &str) -> Retained<NSString> {
  NSString::from_str(text)
}

/// Opens the camera, asking for it first where the person has not been asked.
pub(super) fn shoot(_main: MainThreadMarker, answering: Answering, words: SourceWords) {
  // SAFETY: AVFoundation's class methods, with its name of video.
  let status: isize =
    unsafe { msg_send![objc2::class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeVideo] };
  match status {
    // Authorized.
    3 => open(answering, words),
    // Not asked yet: asked, and the answer comes on another thread.
    0 => {
      let held = Mutex::new(Some(Sending((answering, words))));
      let answered = block2::RcBlock::new(move |granted: Bool| {
        if let Some(Sending((answering, words))) = held.lock().unwrap().take() {
          let granted = granted.as_bool();
          let sent = Sending((answering, words));
          super::on_main(move |_| {
            let (answering, words) = sent.into_inner();
            if granted {
              open(answering, words);
            } else {
              system_camera(answering);
            }
          });
        }
      });
      // SAFETY: the block is called once, on a thread of the system's.
      unsafe {
        let _: () = msg_send![
          objc2::class!(AVCaptureDevice),
          requestAccessForMediaType: AVMediaTypeVideo,
          completionHandler: &*answered
        ];
      }
    }
    // Denied or restricted: the system's camera says so.
    _ => system_camera(answering),
  }
}

/// The best camera on a side of the phone: the one that joins its lenses, where it has several.
unsafe fn find_device(position: isize) -> Option<(Obj, usize)> {
  // SAFETY: AVFoundation's names of the kinds of camera, and its class method.
  unsafe {
    let kinds = [
      AVCaptureDeviceTypeBuiltInTripleCamera,
      AVCaptureDeviceTypeBuiltInDualWideCamera,
      AVCaptureDeviceTypeBuiltInDualCamera,
      AVCaptureDeviceTypeBuiltInWideAngleCamera,
    ];
    for (kind, name) in kinds.into_iter().enumerate() {
      if position == FRONT && kind != 3 {
        continue;
      }
      let device: Option<Obj> = msg_send![
        objc2::class!(AVCaptureDevice),
        defaultDeviceWithDeviceType: name,
        mediaType: AVMediaTypeVideo,
        position: position
      ];
      if let Some(device) = device {
        return Some((device, kind));
      }
    }
    None
  }
}

/// The factor of 1×, and the lenses as the Camera app shows them, for a camera of a kind of [`find_device`].
unsafe fn lenses_of(device: &AnyObject, kind: usize, back: bool) -> (f64, Vec<f64>) {
  // SAFETY: the device's switch-over factors, an array of numbers, and its least zoom.
  unsafe {
    let switches: Retained<NSArray<AnyObject>> = msg_send![device, virtualDeviceSwitchOverVideoZoomFactors];
    let switches: Vec<f64> = switches.iter().map(|number| msg_send![&*number, doubleValue]).collect();
    let least: f64 = msg_send![device, minAvailableVideoZoomFactor];
    if !back {
      return (1.0, Vec::new());
    }
    match (kind, switches.as_slice()) {
      (0, [wide, tele, ..]) => (*wide, vec![least / wide, 1.0, tele / wide]),
      (1, [wide, ..]) => (*wide, vec![least / wide, 1.0, 2.0]),
      (2, [tele, ..]) => (1.0, vec![1.0, *tele]),
      _ => (1.0, vec![1.0, 2.0]),
    }
  }
}

/// Builds the camera and shows it. Where there is no camera, or it cannot be read, the system's is shown.
fn open(answering: Answering, words: SourceWords) {
  let mut slot = CAMERA.lock().unwrap();
  let over = top_controller();
  let (Some(over), None) = (over, slot.as_ref()) else {
    answering.give(Vec::new());
    return;
  };
  // SAFETY: on the main thread, with AVFoundation's and UIKit's classes and the selectors they answer.
  unsafe {
    let Some((device, kind)) = find_device(BACK).or_else(|| find_device(FRONT)) else {
      drop(slot);
      system_camera(answering);
      return;
    };
    let position: isize = msg_send![&*device, position];
    let back = position == BACK;
    let session: Obj = msg_send![objc2::class!(AVCaptureSession), new];
    let _: () = msg_send![&*session, beginConfiguration];
    let photo: Bool = msg_send![&*session, canSetSessionPreset: AVCaptureSessionPresetPhoto];
    if photo.as_bool() {
      let _: () = msg_send![&*session, setSessionPreset: AVCaptureSessionPresetPhoto];
    }
    let input: Option<Obj> = msg_send![
      objc2::class!(AVCaptureDeviceInput),
      deviceInputWithDevice: &*device,
      error: std::ptr::null_mut::<*mut AnyObject>()
    ];
    let output: Obj = msg_send![objc2::class!(AVCapturePhotoOutput), new];
    let joined = input.as_ref().is_some_and(|input| {
      let takes: Bool = msg_send![&*session, canAddInput: &**input];
      let gives: Bool = msg_send![&*session, canAddOutput: &*output];
      takes.as_bool() && gives.as_bool()
    });
    let Some(input) = input.filter(|_| joined) else {
      let _: () = msg_send![&*session, commitConfiguration];
      drop(slot);
      system_camera(answering);
      return;
    };
    let _: () = msg_send![&*session, addInput: &*input];
    let _: () = msg_send![&*session, addOutput: &*output];
    let _: () = msg_send![&*session, commitConfiguration];
    let (base, lenses) = lenses_of(&device, kind, back);
    zoom_to(&device, base, false);

    let delegate: Obj = msg_send![delegate_class(), new];
    let controller: Obj = msg_send![controller_class(), new];
    let _: () = msg_send![&*controller, setModalPresentationStyle: 0isize];
    let root: Obj = msg_send![&*controller, view];
    let _: () = msg_send![&*root, setBackgroundColor: &*color(0.0, 1.0)];

    let shown: Obj = msg_send![&*over, view];
    let bounds: Rect = msg_send![&*shown, bounds];
    let insets: Insets = msg_send![&*shown, safeAreaInsets];
    let _: () = msg_send![&*root, setFrame: bounds];
    let (width, height) = (bounds.size.width, bounds.size.height);
    let top = insets.top.max(20.0);
    let (left, right) = (insets.left + 20.0, width - insets.right - 20.0);
    let bar = top + 2.0;
    let picture_top = bar + 50.0;
    let picture = width * 4.0 / 3.0;
    let row = (height - insets.bottom.max(12.0) - 62.0).max(picture_top + picture * 0.6);

    // The picture, and what it shows on it.
    let preview = view(rect(0.0, picture_top, width, picture));
    let _: () = msg_send![&*preview, setClipsToBounds: Bool::YES];
    let layer: Obj = msg_send![objc2::class!(AVCaptureVideoPreviewLayer), layerWithSession: &*session];
    let _: () = msg_send![&*layer, setVideoGravity: AVLayerVideoGravityResizeAspectFill];
    let _: () = msg_send![&*layer, setFrame: rect(0.0, 0.0, width, picture)];
    let base_layer: Obj = msg_send![&*preview, layer];
    let _: () = msg_send![&*base_layer, addSublayer: &*layer];
    let pinch: Allocated<AnyObject> = msg_send![objc2::class!(UIPinchGestureRecognizer), alloc];
    let pinch: Obj = msg_send![pinch, initWithTarget: &*delegate, action: sel!(lurqPinch:)];
    let _: () = msg_send![&*preview, addGestureRecognizer: &*pinch];
    let tap: Allocated<AnyObject> = msg_send![objc2::class!(UITapGestureRecognizer), alloc];
    let tap: Obj = msg_send![tap, initWithTarget: &*delegate, action: sel!(lurqFocus:)];
    let _: () = msg_send![&*preview, addGestureRecognizer: &*tap];
    let _: () = msg_send![&*root, addSubview: &*preview];

    let blink = view(rect(0.0, 0.0, width, picture));
    let _: () = msg_send![&*blink, setBackgroundColor: &*color(0.0, 1.0)];
    let _: () = msg_send![&*blink, setAlpha: 0.0f64];
    let _: () = msg_send![&*blink, setUserInteractionEnabled: Bool::NO];
    let _: () = msg_send![&*preview, addSubview: &*blink];

    let yellow: Obj = msg_send![objc2::class!(UIColor), systemYellowColor];
    let focus = view(rect(0.0, 0.0, 76.0, 76.0));
    let clear: Obj = msg_send![objc2::class!(UIColor), clearColor];
    let _: () = msg_send![&*focus, setBackgroundColor: &*clear];
    let _: () = msg_send![&*focus, setUserInteractionEnabled: Bool::NO];
    let _: () = msg_send![&*focus, setAlpha: 0.0f64];
    border(&focus, &yellow, 1.5);
    let _: () = msg_send![&*preview, addSubview: &*focus];

    // The lenses, over the bottom of the picture.
    let pill = view(rect(0.0, 0.0, 0.0, 0.0));
    let _: () = msg_send![&*pill, setBackgroundColor: &*color(0.0, 0.35)];
    let mut lens_buttons = Vec::new();
    let span = lenses.len() as f64 * 42.0 + 6.0;
    let _: () = msg_send![&*pill, setFrame: rect((width - span) / 2.0, picture - 60.0, span, 46.0)];
    round(&pill, 23.0);
    let _: () = msg_send![&*pill, setAccessibilityLabel: &*ns(&words.zoom)];
    for at in 0..lenses.len() {
      let lens: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 0isize];
      let _: () = msg_send![&*lens, setFrame: rect(5.0 + at as f64 * 42.0, 4.0, 38.0, 38.0)];
      let _: () = msg_send![&*lens, setBackgroundColor: &*color(0.0, 0.45)];
      let _: () = msg_send![&*lens, setTag: at as isize];
      round(&lens, 19.0);
      target(&lens, &delegate, sel!(lurqLens:));
      let _: () = msg_send![&*pill, addSubview: &*lens];
      lens_buttons.push(lens);
    }
    let _: () = msg_send![&*pill, setHidden: Bool::new(lenses.len() < 2)];
    let _: () = msg_send![&*preview, addSubview: &*pill];

    // The top bar: Cancel, the flash, Done.
    let cancel = text_button(&words.cancel, false);
    let _: () = msg_send![&*cancel, setFrame: rect(left - 4.0, bar, 120.0, 44.0)];
    let _: () = msg_send![&*cancel, setContentHorizontalAlignment: 1isize];
    target(&cancel, &delegate, sel!(lurqCancel:));
    let _: () = msg_send![&*root, addSubview: &*cancel];

    let flash: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 1isize];
    let _: () = msg_send![&*flash, setFrame: rect(width / 2.0 - 22.0, bar, 44.0, 44.0)];
    let _: () = msg_send![&*flash, setAccessibilityLabel: &*ns(&words.flash)];
    target(&flash, &delegate, sel!(lurqFlash:));
    let _: () = msg_send![&*root, addSubview: &*flash];

    let done = text_button(&words.done, true);
    let _: () = msg_send![&*done, setFrame: rect(right - 116.0, bar, 120.0, 44.0)];
    let _: () = msg_send![&*done, setContentHorizontalAlignment: 2isize];
    let _: () = msg_send![&*done, setEnabled: Bool::NO];
    target(&done, &delegate, sel!(lurqDone:));
    let _: () = msg_send![&*root, addSubview: &*done];

    // The bottom row: the photos taken, the shutter, the other camera.
    let stack: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 0isize];
    let _: () = msg_send![&*stack, setFrame: rect(left, row - STACK / 2.0, STACK, STACK)];
    target(&stack, &delegate, sel!(lurqStack:));
    let last: Obj = msg_send![objc2::class!(UIImageView), new];
    let _: () = msg_send![&*last, setFrame: rect(0.0, 0.0, STACK, STACK)];
    let _: () = msg_send![&*last, setContentMode: 2isize];
    let _: () = msg_send![&*last, setUserInteractionEnabled: Bool::NO];
    round(&last, 10.0);
    border(&last, &color(1.0, 1.0), 2.0);
    let _: () = msg_send![&*last, setHidden: Bool::YES];
    let _: () = msg_send![&*stack, addSubview: &*last];
    let count: Obj = msg_send![objc2::class!(UILabel), new];
    let _: () = msg_send![&*count, setFrame: rect(STACK - 14.0, -10.0, 24.0, 24.0)];
    let blue: Obj = msg_send![objc2::class!(UIColor), systemBlueColor];
    let _: () = msg_send![&*count, setBackgroundColor: &*blue];
    let _: () = msg_send![&*count, setTextColor: &*color(1.0, 1.0)];
    let _: () = msg_send![&*count, setTextAlignment: 1isize];
    let font: Obj = msg_send![objc2::class!(UIFont), boldSystemFontOfSize: 14.0f64];
    let _: () = msg_send![&*count, setFont: &*font];
    let _: () = msg_send![&*count, setAdjustsFontSizeToFitWidth: Bool::YES];
    round(&count, 12.0);
    let _: () = msg_send![&*count, setHidden: Bool::YES];
    let _: () = msg_send![&*stack, addSubview: &*count];
    let _: () = msg_send![&*root, addSubview: &*stack];

    let ring = view(rect(width / 2.0 - 39.0, row - 39.0, 78.0, 78.0));
    let _: () = msg_send![&*ring, setBackgroundColor: &*clear];
    let _: () = msg_send![&*ring, setUserInteractionEnabled: Bool::NO];
    round(&ring, 39.0);
    border(&ring, &color(1.0, 1.0), 4.0);
    let _: () = msg_send![&*root, addSubview: &*ring];
    let shutter: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 0isize];
    let _: () = msg_send![&*shutter, setFrame: rect(width / 2.0 - 32.0, row - 32.0, 64.0, 64.0)];
    let _: () = msg_send![&*shutter, setBackgroundColor: &*color(1.0, 1.0)];
    round(&shutter, 32.0);
    let _: () = msg_send![&*shutter, setAccessibilityLabel: &*ns(&words.take_photo)];
    target(&shutter, &delegate, sel!(lurqShutter:));
    let _: () = msg_send![&*root, addSubview: &*shutter];

    if find_device(if back { FRONT } else { BACK }).is_some() {
      let flip = symbol_button("arrow.triangle.2.circlepath", 20.0);
      let _: () = msg_send![&*flip, setFrame: rect(right - 48.0, row - 24.0, 48.0, 48.0)];
      let _: () = msg_send![&*flip, setBackgroundColor: &*color(1.0, 0.16)];
      round(&flip, 24.0);
      let _: () = msg_send![&*flip, setAccessibilityLabel: &*ns(&words.switch_camera)];
      target(&flip, &delegate, sel!(lurqSwitch:));
      let _: () = msg_send![&*root, addSubview: &*flip];
    }

    let phone: Obj = msg_send![objc2::class!(UIDevice), currentDevice];
    let _: () = msg_send![&*phone, beginGeneratingDeviceOrientationNotifications];

    let camera = Camera {
      answering,
      words,
      delegate,
      controller,
      session,
      output,
      input,
      device,
      back,
      base,
      lenses,
      pinched_from: base,
      flash: FLASH_AUTO,
      held: 1,
      taken: Vec::new(),
      pending: 0,
      finishing: false,
      views: Views {
        preview,
        layer,
        pill,
        lens_buttons,
        flash,
        last,
        count,
        done,
        blink,
        focus,
      },
      viewer: None,
    };
    show_flash(&camera);
    show_zoom(&camera, base);
    run(&camera.session, true);
    let _: () = msg_send![
      &*over,
      presentViewController: &*camera.controller,
      animated: Bool::YES,
      completion: std::ptr::null::<c_void>()
    ];
    *slot = Some(camera);
  }
}

/// Starts or stops a session, which blocks while it does: on a thread of its own.
fn run(session: &Obj, start: bool) {
  let session = Sending(session.clone());
  std::thread::spawn(move || {
    let session = session.into_inner();
    // SAFETY: a session may be started and stopped on any thread.
    unsafe {
      if start {
        let _: () = msg_send![&*session, startRunning];
      } else {
        let _: () = msg_send![&*session, stopRunning];
      }
    }
  });
}

/// Zooms a camera to a factor of its own, at once or as the Camera app goes from lens to lens.
unsafe fn zoom_to(device: &AnyObject, factor: f64, ramp: bool) {
  // SAFETY: the device, locked for the change and unlocked after it.
  unsafe {
    let locked: Bool = msg_send![device, lockForConfiguration: std::ptr::null_mut::<*mut AnyObject>()];
    if !locked.as_bool() {
      return;
    }
    let least: f64 = msg_send![device, minAvailableVideoZoomFactor];
    let most: f64 = msg_send![device, maxAvailableVideoZoomFactor];
    let factor = factor.clamp(least, most);
    if ramp {
      let _: () = msg_send![device, rampToVideoZoomFactor: factor, withRate: 16.0f32];
    } else {
      let _: () = msg_send![device, cancelVideoZoomRamp];
      let _: () = msg_send![device, setVideoZoomFactor: factor];
    }
    let _: () = msg_send![device, unlockForConfiguration];
  }
}

/// How far the camera can zoom, in its own factors: up to ten times the lens of 1×, or five times the longest.
fn most_zoom(camera: &Camera) -> f64 {
  let longest = camera.lenses.iter().copied().fold(1.0, f64::max);
  // SAFETY: the device's own limit.
  let most: f64 = unsafe { msg_send![&*camera.device, maxAvailableVideoZoomFactor] };
  most.min(camera.base * (longest * 5.0).max(10.0))
}

/// A zoom as the Camera app writes it: "2", "0.5", "1.4".
fn zoom_words(shown: f64) -> String {
  let tenths = (shown * 10.0).round();
  if tenths % 10.0 == 0.0 {
    format!("{}", tenths as i64 / 10)
  } else {
    format!("{:.1}", tenths / 10.0)
  }
}

/// The lenses' buttons for a zoom of the camera's own factor: the one in use says how far, in yellow.
fn show_zoom(camera: &Camera, factor: f64) {
  let shown = factor / camera.base;
  let using = camera
    .lenses
    .iter()
    .rposition(|lens| *lens <= shown + 0.05)
    .unwrap_or(0);
  // SAFETY: on the main thread, the camera's views.
  unsafe {
    let yellow: Obj = msg_send![objc2::class!(UIColor), systemYellowColor];
    let white = color(1.0, 1.0);
    for (at, (button, lens)) in camera.views.lens_buttons.iter().zip(&camera.lenses).enumerate() {
      let (words, tint, size) = if at == using {
        (format!("{}×", zoom_words(shown)), &yellow, 13.0f64)
      } else {
        (zoom_words(*lens), &white, 12.0f64)
      };
      let _: () = msg_send![&**button, setTitle: &*ns(&words), forState: 0usize];
      let _: () = msg_send![&**button, setTitleColor: &**tint, forState: 0usize];
      let font: Obj = msg_send![objc2::class!(UIFont), boldSystemFontOfSize: size];
      let label: Option<Obj> = msg_send![&**button, titleLabel];
      if let Some(label) = label {
        let _: () = msg_send![&*label, setFont: &*font];
      }
    }
    let _: () = msg_send![&*camera.views.pill, setAccessibilityValue: &*ns(&format!("{}×", zoom_words(shown)))];
  }
}

/// The flash's button for what it is set to; hidden where the camera has none.
fn show_flash(camera: &Camera) {
  let (symbol, words) = match camera.flash {
    FLASH_ON => ("bolt.fill", &camera.words.flash_on),
    FLASH_OFF => ("bolt.slash.fill", &camera.words.flash_off),
    _ => ("bolt.badge.automatic.fill", &camera.words.flash_auto),
  };
  // SAFETY: on the main thread, the button, and the device's flash.
  unsafe {
    let has: Bool = msg_send![&*camera.device, hasFlash];
    let button = &camera.views.flash;
    let _: () = msg_send![&**button, setHidden: Bool::new(!has.as_bool())];
    if let Some(image) = symbol_image(symbol, 18.0).or_else(|| symbol_image("bolt.badge.a.fill", 18.0)) {
      let _: () = msg_send![&**button, setImage: &*image, forState: 0usize];
    }
    let tint: Obj = if camera.flash == FLASH_ON {
      msg_send![objc2::class!(UIColor), systemYellowColor]
    } else {
      color(1.0, 1.0)
    };
    let _: () = msg_send![&**button, setTintColor: &*tint];
    let _: () = msg_send![&**button, setAccessibilityValue: &*ns(words)];
  }
}

/// The last photo beside the shutter, how many there are, and Done.
fn show_count(camera: &Camera) {
  let count = camera.taken.len();
  let none = Bool::new(count == 0);
  // SAFETY: on the main thread, the camera's views.
  unsafe {
    let last = camera.taken.last().and_then(|(_, small)| small.clone());
    let picture = last
      .as_deref()
      .map_or(std::ptr::null(), |last| last as *const AnyObject);
    let _: () = msg_send![&*camera.views.last, setImage: picture];
    let _: () = msg_send![&*camera.views.last, setHidden: none];
    let _: () = msg_send![&*camera.views.count, setText: &*ns(&count.to_string())];
    let _: () = msg_send![&*camera.views.count, setHidden: none];
    let _: () = msg_send![&*camera.views.done, setEnabled: Bool::new(count > 0 && !camera.finishing)];
  }
}

/// A view of UIKit's, with its frame.
unsafe fn view(frame: Rect) -> Obj {
  // SAFETY: on the main thread, a new view.
  unsafe {
    let view: Allocated<AnyObject> = msg_send![objc2::class!(UIView), alloc];
    msg_send![view, initWithFrame: frame]
  }
}

/// A gray of UIKit's, from black (0) to white (1).
unsafe fn color(white: f64, alpha: f64) -> Obj {
  // SAFETY: a color, which any thread may make.
  unsafe { msg_send![objc2::class!(UIColor), colorWithWhite: white, alpha: alpha] }
}

/// Rounds a view's corners, and clips what is in it to them.
unsafe fn round(view: &AnyObject, radius: f64) {
  // SAFETY: on the main thread, a view's layer.
  unsafe {
    let layer: Obj = msg_send![view, layer];
    let _: () = msg_send![&*layer, setCornerRadius: radius];
    let _: () = msg_send![&*layer, setMasksToBounds: Bool::YES];
  }
}

/// A line around a view.
unsafe fn border(view: &AnyObject, tint: &AnyObject, width: f64) {
  // SAFETY: on the main thread, a view's layer, and the color's own CGColor, which the layer keeps.
  unsafe {
    let layer: Obj = msg_send![view, layer];
    let line: CgColor = msg_send![tint, CGColor];
    let _: () = msg_send![&*layer, setBorderColor: line];
    let _: () = msg_send![&*layer, setBorderWidth: width];
  }
}

/// One of the system's symbols, at a size.
unsafe fn symbol_image(name: &str, size: f64) -> Option<Obj> {
  // SAFETY: UIKit's symbols, by name.
  unsafe {
    let configuration: Obj =
      msg_send![objc2::class!(UIImageSymbolConfiguration), configurationWithPointSize: size, weight: 6isize];
    msg_send![objc2::class!(UIImage), systemImageNamed: &*ns(name), withConfiguration: &*configuration]
  }
}

/// A button of a symbol, white.
unsafe fn symbol_button(name: &str, size: f64) -> Obj {
  // SAFETY: on the main thread, a system button.
  unsafe {
    let button: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 1isize];
    if let Some(image) = symbol_image(name, size) {
      let _: () = msg_send![&*button, setImage: &*image, forState: 0usize];
    }
    let _: () = msg_send![&*button, setTintColor: &*color(1.0, 1.0)];
    button
  }
}

/// A button of words, white.
unsafe fn text_button(words: &str, bold: bool) -> Obj {
  // SAFETY: on the main thread, a system button.
  unsafe {
    let button: Obj = msg_send![objc2::class!(UIButton), buttonWithType: 1isize];
    let _: () = msg_send![&*button, setTitle: &*ns(words), forState: 0usize];
    let _: () = msg_send![&*button, setTintColor: &*color(1.0, 1.0)];
    let font: Obj = if bold {
      msg_send![objc2::class!(UIFont), boldSystemFontOfSize: 17.0f64]
    } else {
      msg_send![objc2::class!(UIFont), systemFontOfSize: 17.0f64]
    };
    let label: Option<Obj> = msg_send![&*button, titleLabel];
    if let Some(label) = label {
      let _: () = msg_send![&*label, setFont: &*font];
    }
    button
  }
}

/// Has a press of `button` call `action` of the camera's delegate.
unsafe fn target(button: &AnyObject, delegate: &AnyObject, action: Sel) {
  // SAFETY: on the main thread; the delegate is held by the camera as long as the button is.
  unsafe {
    let _: () = msg_send![button, addTarget: delegate, action: action, forControlEvents: TOUCH_UP_INSIDE];
  }
}

/// Puts the camera away and gives the photos taken, or none.
fn finish(give: bool) {
  let Some(camera) = CAMERA.lock().unwrap().take() else {
    return;
  };
  run(&camera.session, false);
  // SAFETY: on the main thread, the camera shown, and what it presents with it.
  unsafe {
    let phone: Obj = msg_send![objc2::class!(UIDevice), currentDevice];
    let _: () = msg_send![&*phone, endGeneratingDeviceOrientationNotifications];
    let presenter: Option<Obj> = msg_send![&*camera.controller, presentingViewController];
    let presenter = presenter.unwrap_or_else(|| camera.controller.clone());
    let _: () =
      msg_send![&*presenter, dismissViewControllerAnimated: Bool::YES, completion: std::ptr::null::<c_void>()];
  }
  let taken: Vec<DroppedFile> = camera.taken.into_iter().map(|(file, _)| file).collect();
  if !give {
    for file in &taken {
      if let Some(path) = file.path() {
        let _ = std::fs::remove_file(path);
      }
    }
  }
  camera.answering.give(if give { taken } else { Vec::new() });
}

/// The camera's state, where it is open.
fn with_camera(work: impl FnOnce(&mut Camera)) {
  if let Some(camera) = CAMERA.lock().unwrap().as_mut() {
    work(camera);
  }
}

/// The shutter: a photo is asked for, kept when it comes ([`photo_taken`]).
extern "C-unwind" fn press_shutter(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    if camera.finishing {
      return;
    }
    // SAFETY: on the main thread, the session's output, and settings of its own for each photo.
    unsafe {
      let settings: Obj = msg_send![objc2::class!(AVCapturePhotoSettings), photoSettings];
      let modes: Retained<NSArray<AnyObject>> = msg_send![&*camera.output, supportedFlashModes];
      let can = modes.iter().any(|mode| {
        let mode: isize = msg_send![&*mode, integerValue];
        mode == camera.flash
      });
      if can {
        let _: () = msg_send![&*settings, setFlashMode: camera.flash];
      }
      // The photo is turned as the phone is held: its orientation, or the last it had where it lies flat.
      let phone: Obj = msg_send![objc2::class!(UIDevice), currentDevice];
      let held: isize = msg_send![&*phone, orientation];
      if (1..=4).contains(&held) {
        camera.held = held;
      }
      let angle: f64 = match camera.held {
        2 => 270.0,
        3 => 0.0,
        4 => 180.0,
        _ => 90.0,
      };
      let connection: Option<Obj> = msg_send![&*camera.output, connectionWithMediaType: AVMediaTypeVideo];
      if let Some(connection) = connection {
        let turns: Bool = msg_send![&*connection, isVideoRotationAngleSupported: angle];
        if turns.as_bool() {
          let _: () = msg_send![&*connection, setVideoRotationAngle: angle];
        }
      }
      let _: () = msg_send![&*camera.output, capturePhotoWithSettings: &*settings, delegate: &*camera.delegate];
      camera.pending += 1;

      let blink = camera.views.blink.clone();
      let _: () = msg_send![&*blink, setAlpha: 0.85f64];
      let fade = block2::RcBlock::new(move || {
        let _: () = msg_send![&*blink, setAlpha: 0.0f64];
      });
      let _: () = msg_send![objc2::class!(UIView), animateWithDuration: 0.2f64, animations: &*fade];
    }
  });
}

/// A photo the camera took, on a thread of its own: written as a JPEG, with a small picture of it, and kept on the
/// main thread.
extern "C-unwind" fn photo_taken(
  _this: &AnyObject,
  _cmd: Sel,
  _output: &AnyObject,
  photo: &AnyObject,
  error: *mut AnyObject,
) {
  // SAFETY: the photo AVFoundation gives, its data, and UIKit's pictures, which any thread may make.
  let kept: Option<(PathBuf, Option<Obj>)> = unsafe {
    if error.is_null() {
      let data: Option<Obj> = msg_send![photo, fileDataRepresentation];
      data.and_then(|data| {
        let path = photo_path()?;
        let written: Bool = msg_send![&*data, writeToFile: &*ns(path.to_str()?), atomically: Bool::YES];
        if !written.as_bool() {
          return None;
        }
        let image: Option<Obj> = msg_send![objc2::class!(UIImage), imageWithData: &*data];
        let side = STACK * 3.0;
        let small: Option<Obj> = image
          .and_then(|image| msg_send![&*image, imageByPreparingThumbnailOfSize: Size { width: side, height: side }]);
        Some((path, small))
      })
    } else {
      None
    }
  };
  let kept = Sending(kept);
  super::on_main(move |_| {
    let kept = kept.into_inner();
    let mut slot = CAMERA.lock().unwrap();
    let Some(camera) = slot.as_mut() else {
      if let Some((path, _)) = kept {
        let _ = std::fs::remove_file(path);
      }
      return;
    };
    camera.pending = camera.pending.saturating_sub(1);
    if let Some((path, small)) = kept {
      camera.taken.push((DroppedFile::from_path(path), small));
    }
    show_count(camera);
    if camera.finishing && camera.pending == 0 {
      drop(slot);
      finish(true);
    }
  });
}

/// Done: the photos are given, once those still being taken are kept.
extern "C-unwind" fn press_done(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  let mut slot = CAMERA.lock().unwrap();
  let Some(camera) = slot.as_mut() else {
    return;
  };
  if camera.pending > 0 {
    camera.finishing = true;
    show_count(camera);
    return;
  }
  drop(slot);
  finish(true);
}

extern "C-unwind" fn press_cancel(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  finish(false);
}

/// The flash: auto, on, off, and around.
extern "C-unwind" fn press_flash(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    camera.flash = match camera.flash {
      FLASH_AUTO => FLASH_ON,
      FLASH_ON => FLASH_OFF,
      _ => FLASH_AUTO,
    };
    show_flash(camera);
  });
}

/// A lens: the camera zooms to it, as the Camera app does.
extern "C-unwind" fn press_lens(_this: &AnyObject, _cmd: Sel, sender: &AnyObject) {
  with_camera(|camera| {
    // SAFETY: a lens's button, whose tag is its place.
    let at: isize = unsafe { msg_send![sender, tag] };
    let Some(lens) = usize::try_from(at).ok().and_then(|at| camera.lenses.get(at)) else {
      return;
    };
    let factor = (lens * camera.base).min(most_zoom(camera));
    // SAFETY: the camera's device.
    unsafe { zoom_to(&camera.device, factor, true) };
    show_zoom(camera, factor);
  });
}

/// A pinch on the picture: the zoom follows it.
extern "C-unwind" fn pinch(_this: &AnyObject, _cmd: Sel, sender: &AnyObject) {
  with_camera(|camera| {
    // SAFETY: the pinch's state and scale, and the camera's device.
    unsafe {
      let state: isize = msg_send![sender, state];
      let now: f64 = msg_send![&*camera.device, videoZoomFactor];
      if state == BEGAN {
        camera.pinched_from = now;
      }
      if state == BEGAN || state == CHANGED {
        let scale: f64 = msg_send![sender, scale];
        let factor = (camera.pinched_from * scale).min(most_zoom(camera));
        zoom_to(&camera.device, factor, false);
        let now: f64 = msg_send![&*camera.device, videoZoomFactor];
        show_zoom(camera, now);
      }
    }
  });
}

/// A tap on the picture: the camera focuses and exposes there, and a square shows where.
extern "C-unwind" fn focus(_this: &AnyObject, _cmd: Sel, sender: &AnyObject) {
  with_camera(|camera| {
    // SAFETY: on the main thread, the tap's place in the picture, the preview's own point of the camera's for it,
    // and the device, locked for the change.
    unsafe {
      let at: Point = msg_send![sender, locationInView: &*camera.views.preview];
      let point: Point = msg_send![&*camera.views.layer, captureDevicePointOfInterestForPoint: at];
      let device = &*camera.device;
      let locked: Bool = msg_send![device, lockForConfiguration: std::ptr::null_mut::<*mut AnyObject>()];
      if locked.as_bool() {
        let focuses: Bool = msg_send![device, isFocusPointOfInterestSupported];
        if focuses.as_bool() {
          let _: () = msg_send![device, setFocusPointOfInterest: point];
          let _: () = msg_send![device, setFocusMode: 2isize];
        }
        let exposes: Bool = msg_send![device, isExposurePointOfInterestSupported];
        if exposes.as_bool() {
          let _: () = msg_send![device, setExposurePointOfInterest: point];
          let _: () = msg_send![device, setExposureMode: 2isize];
        }
        let _: () = msg_send![device, unlockForConfiguration];
      }
      let square = camera.views.focus.clone();
      let _: () = msg_send![&*square, setCenter: at];
      let _: () = msg_send![&*square, setAlpha: 1.0f64];
      let fade = block2::RcBlock::new(move || {
        let _: () = msg_send![&*square, setAlpha: 0.0f64];
      });
      let _: () = msg_send![
        objc2::class!(UIView),
        animateWithDuration: 0.3f64,
        delay: 0.9f64,
        options: 0usize,
        animations: &*fade,
        completion: std::ptr::null::<c_void>()
      ];
    }
  });
}

/// The other camera, front or back: its lenses, its flash.
extern "C-unwind" fn press_switch(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    // SAFETY: on the main thread, the session changed inside a configuration of its own.
    unsafe {
      let Some((device, kind)) = find_device(if camera.back { FRONT } else { BACK }) else {
        return;
      };
      let input: Option<Obj> = msg_send![
        objc2::class!(AVCaptureDeviceInput),
        deviceInputWithDevice: &*device,
        error: std::ptr::null_mut::<*mut AnyObject>()
      ];
      let Some(input) = input else {
        return;
      };
      let session = &*camera.session;
      let _: () = msg_send![session, beginConfiguration];
      let _: () = msg_send![session, removeInput: &*camera.input];
      let takes: Bool = msg_send![session, canAddInput: &*input];
      if takes.as_bool() {
        let _: () = msg_send![session, addInput: &*input];
        camera.input = input;
        camera.device = device;
        camera.back = !camera.back;
      } else {
        let _: () = msg_send![session, addInput: &*camera.input];
      }
      let _: () = msg_send![session, commitConfiguration];
      let (base, lenses) = lenses_of(&camera.device, kind, camera.back);
      camera.base = base;
      zoom_to(&camera.device, base, false);
      let shown = lenses.len();
      for (at, button) in camera.views.lens_buttons.iter().enumerate() {
        let _: () = msg_send![&**button, setHidden: Bool::new(at >= shown)];
      }
      if shown >= 2 {
        camera.lenses = lenses;
      }
      let _: () = msg_send![&*camera.views.pill, setHidden: Bool::new(shown < 2)];
      show_zoom(camera, base);
      show_flash(camera);
    }
  });
}

/// The last photo tapped: the photos taken, in the viewer, at the last.
extern "C-unwind" fn press_stack(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    if camera.taken.is_empty() || camera.viewer.is_some() {
      return;
    }
    // SAFETY: on the main thread, with UIKit's classes.
    unsafe {
      let controller: Obj = msg_send![controller_class(), new];
      let _: () = msg_send![&*controller, setModalPresentationStyle: 0isize];
      let _: () = msg_send![&*controller, setModalTransitionStyle: 2isize];
      let root: Obj = msg_send![&*controller, view];
      let _: () = msg_send![&*root, setBackgroundColor: &*color(0.0, 1.0)];
      let camera_view: Obj = msg_send![&*camera.controller, view];
      let bounds: Rect = msg_send![&*camera_view, bounds];
      let insets: Insets = msg_send![&*camera_view, safeAreaInsets];
      let _: () = msg_send![&*root, setFrame: bounds];
      let (width, height) = (bounds.size.width, bounds.size.height);

      let pager: Obj = msg_send![objc2::class!(UIScrollView), new];
      let _: () = msg_send![&*pager, setFrame: rect(0.0, 0.0, width, height)];
      let _: () = msg_send![&*pager, setPagingEnabled: Bool::YES];
      let _: () = msg_send![&*pager, setShowsHorizontalScrollIndicator: Bool::NO];
      let _: () = msg_send![&*pager, setContentInsetAdjustmentBehavior: 2isize];
      let _: () = msg_send![&*pager, setTag: PAGER];
      let _: () = msg_send![&*pager, setDelegate: &*camera.delegate];
      let _: () = msg_send![&*root, addSubview: &*pager];

      // The bar over the photo: close, which one of how many, and take it out.
      let top = insets.top.max(20.0);
      let bar = view(rect(0.0, 0.0, width, top + 48.0));
      let _: () = msg_send![&*bar, setBackgroundColor: &*color(0.0, 0.45)];
      let _: () = msg_send![&*root, addSubview: &*bar];
      let close = symbol_button("xmark", 18.0);
      let _: () = msg_send![&*close, setFrame: rect(insets.left + 8.0, top + 2.0, 44.0, 44.0)];
      let _: () = msg_send![&*close, setAccessibilityLabel: &*ns(&camera.words.close)];
      target(&close, &camera.delegate, sel!(lurqViewerClose:));
      let _: () = msg_send![&*bar, addSubview: &*close];
      let title: Obj = msg_send![objc2::class!(UILabel), new];
      let _: () = msg_send![&*title, setFrame: rect(60.0, top + 2.0, width - 120.0, 44.0)];
      let _: () = msg_send![&*title, setTextAlignment: 1isize];
      let _: () = msg_send![&*title, setTextColor: &*color(1.0, 1.0)];
      let font: Obj = msg_send![objc2::class!(UIFont), boldSystemFontOfSize: 17.0f64];
      let _: () = msg_send![&*title, setFont: &*font];
      let _: () = msg_send![&*bar, addSubview: &*title];
      let remove = symbol_button("trash", 18.0);
      let _: () = msg_send![&*remove, setFrame: rect(width - insets.right - 52.0, top + 2.0, 44.0, 44.0)];
      let _: () = msg_send![&*remove, setAccessibilityLabel: &*ns(&camera.words.remove)];
      target(&remove, &camera.delegate, sel!(lurqViewerRemove:));
      let _: () = msg_send![&*bar, addSubview: &*remove];

      camera.viewer = Some(Viewer {
        controller: controller.clone(),
        pager,
        title,
        pages: Vec::new(),
        at: camera.taken.len() - 1,
        width,
      });
      lay_pages(camera);
      let _: () = msg_send![
        &*camera.controller,
        presentViewController: &*controller,
        animated: Bool::YES,
        completion: std::ptr::null::<c_void>()
      ];
    }
  });
}

/// The viewer's pages, one for each photo, at the one it is at.
fn lay_pages(camera: &mut Camera) {
  let delegate = camera.delegate.clone();
  let Some(viewer) = camera.viewer.as_mut() else {
    return;
  };
  // SAFETY: on the main thread, the viewer's views and new ones.
  unsafe {
    for (page, ..) in viewer.pages.drain(..) {
      let _: () = msg_send![&*page, removeFromSuperview];
    }
    let frame: Rect = msg_send![&*viewer.pager, frame];
    let (width, height) = (frame.size.width, frame.size.height);
    for (at, (_, small)) in camera.taken.iter().enumerate() {
      let page: Obj = msg_send![objc2::class!(UIScrollView), new];
      let _: () = msg_send![&*page, setFrame: rect(at as f64 * width, 0.0, width, height)];
      let _: () = msg_send![&*page, setMinimumZoomScale: 1.0f64];
      let _: () = msg_send![&*page, setMaximumZoomScale: 5.0f64];
      let _: () = msg_send![&*page, setShowsHorizontalScrollIndicator: Bool::NO];
      let _: () = msg_send![&*page, setShowsVerticalScrollIndicator: Bool::NO];
      let _: () = msg_send![&*page, setContentInsetAdjustmentBehavior: 2isize];
      let _: () = msg_send![&*page, setTag: PAGE + at as isize];
      let _: () = msg_send![&*page, setDelegate: &*delegate];
      let picture: Obj = msg_send![objc2::class!(UIImageView), new];
      let _: () = msg_send![&*picture, setFrame: rect(0.0, 0.0, width, height)];
      let _: () = msg_send![&*picture, setContentMode: 1isize];
      if let Some(small) = small {
        let _: () = msg_send![&*picture, setImage: &**small];
      }
      let _: () = msg_send![&*page, addSubview: &*picture];
      let twice: Allocated<AnyObject> = msg_send![objc2::class!(UITapGestureRecognizer), alloc];
      let twice: Obj = msg_send![twice, initWithTarget: &*delegate, action: sel!(lurqViewerTwice:)];
      let _: () = msg_send![&*twice, setNumberOfTapsRequired: 2usize];
      let _: () = msg_send![&*page, addGestureRecognizer: &*twice];
      let _: () = msg_send![&*viewer.pager, addSubview: &*page];
      viewer.pages.push((page, picture, false));
    }
    let total = camera.taken.len();
    viewer.at = viewer.at.min(total.saturating_sub(1));
    let _: () = msg_send![&*viewer.pager, setContentSize: Size { width: width * total as f64, height }];
    let _: () =
      msg_send![&*viewer.pager, setContentOffset: Point { x: viewer.at as f64 * width, y: 0.0 }, animated: Bool::NO];
  }
  show_page(camera);
}

/// The viewer's title for the page it is at, and the photos near it drawn whole, the others small.
fn show_page(camera: &mut Camera) {
  let Some(viewer) = camera.viewer.as_mut() else {
    return;
  };
  let words = crate::core::fill(
    &camera.words.place_of,
    &[
      ("at", &(viewer.at + 1).to_string()),
      ("total", &camera.taken.len().to_string()),
    ],
  );
  // SAFETY: on the main thread, the viewer's views, and pictures read from the photos' files.
  unsafe {
    let _: () = msg_send![&*viewer.title, setText: &*ns(&words)];
    for (at, ((page, picture, whole), (file, small))) in viewer.pages.iter_mut().zip(&camera.taken).enumerate() {
      let near = at.abs_diff(viewer.at) <= 1;
      if near && !*whole {
        let image: Option<Obj> = match file.path().and_then(|path| path.to_str()) {
          Some(path) => msg_send![objc2::class!(UIImage), imageWithContentsOfFile: &*ns(path)],
          None => None,
        };
        if let Some(image) = image {
          let _: () = msg_send![&**picture, setImage: &*image];
          *whole = true;
        }
      } else if !near && *whole {
        let small = small
          .as_deref()
          .map_or(std::ptr::null(), |small| small as *const AnyObject);
        let _: () = msg_send![&**picture, setImage: small];
        let _: () = msg_send![&**page, setZoomScale: 1.0f64];
        *whole = false;
      }
    }
  }
}

extern "C-unwind" fn viewer_close(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    if let Some(viewer) = camera.viewer.take() {
      // SAFETY: on the main thread, the viewer shown.
      unsafe {
        let _: () = msg_send![
          &*viewer.controller,
          dismissViewControllerAnimated: Bool::YES,
          completion: std::ptr::null::<c_void>()
        ];
      }
    }
  });
}

/// The photo shown is taken out, and its file deleted; the viewer closes after the last.
extern "C-unwind" fn viewer_remove(_this: &AnyObject, _cmd: Sel, _sender: &AnyObject) {
  with_camera(|camera| {
    let Some(at) = camera.viewer.as_ref().map(|viewer| viewer.at) else {
      return;
    };
    if at >= camera.taken.len() {
      return;
    }
    let (file, _) = camera.taken.remove(at);
    if let Some(path) = file.path() {
      let _ = std::fs::remove_file(path);
    }
    show_count(camera);
    if camera.taken.is_empty() {
      if let Some(viewer) = camera.viewer.take() {
        // SAFETY: on the main thread, the viewer shown.
        unsafe {
          let _: () = msg_send![
            &*viewer.controller,
            dismissViewControllerAnimated: Bool::YES,
            completion: std::ptr::null::<c_void>()
          ];
        }
      }
      return;
    }
    lay_pages(camera);
  });
}

/// Two taps on a page: zoomed in where they were, or out again.
extern "C-unwind" fn viewer_twice(_this: &AnyObject, _cmd: Sel, sender: &AnyObject) {
  // SAFETY: on the main thread, the page the taps were on and its picture. The page asks for what it zooms
  // ([`zoomed`]), which takes no lock: none is taken here.
  unsafe {
    let page: Option<Obj> = msg_send![sender, view];
    let Some(page) = page else {
      return;
    };
    let scale: f64 = msg_send![&*page, zoomScale];
    if scale > 1.01 {
      let _: () = msg_send![&*page, setZoomScale: 1.0f64, animated: Bool::YES];
      return;
    }
    let subviews: Retained<NSArray<AnyObject>> = msg_send![&*page, subviews];
    let Some(picture) = subviews.firstObject() else {
      return;
    };
    let at: Point = msg_send![sender, locationInView: &*picture];
    let bounds: Rect = msg_send![&*page, bounds];
    let (width, height) = (bounds.size.width / 2.5, bounds.size.height / 2.5);
    let _: () = msg_send![
      &*page,
      zoomToRect: rect(at.x - width / 2.0, at.y - height / 2.0, width, height),
      animated: Bool::YES
    ];
  }
}

/// What a page of the viewer zooms: its picture. It takes no lock, as UIKit asks for it while a zoom is set.
extern "C-unwind" fn zoomed(_this: &AnyObject, _cmd: Sel, scroll: &AnyObject) -> *mut AnyObject {
  // SAFETY: on the main thread, the scroll view's own subviews; the picture is held by the page.
  unsafe {
    let tag: isize = msg_send![scroll, tag];
    if tag < PAGE {
      return std::ptr::null_mut();
    }
    let subviews: Retained<NSArray<AnyObject>> = msg_send![scroll, subviews];
    subviews
      .firstObject()
      .map_or(std::ptr::null_mut(), |picture| Retained::as_ptr(&picture).cast_mut())
  }
}

/// The viewer came to rest on a page: which one it is.
extern "C-unwind" fn paged(_this: &AnyObject, _cmd: Sel, scroll: &AnyObject) {
  // SAFETY: the scroll view's tag and where it is.
  let (tag, offset): (isize, Point) = unsafe { (msg_send![scroll, tag], msg_send![scroll, contentOffset]) };
  if tag != PAGER {
    return;
  }
  with_camera(|camera| {
    let total = camera.taken.len();
    if let Some(viewer) = camera.viewer.as_mut() {
      let at = (offset.x / viewer.width.max(1.0)).round().max(0.0) as usize;
      viewer.at = at.min(total.saturating_sub(1));
    }
    show_page(camera);
  });
}

/// The controller the camera and its viewer are shown in: upright, without the status bar.
fn controller_class() -> &'static AnyClass {
  if let Some(class) = AnyClass::get(c"LurqCameraController") {
    return class;
  }
  let mut builder =
    ClassBuilder::new(c"LurqCameraController", objc2::class!(UIViewController)).expect("a class name of its own");
  extern "C-unwind" fn hidden(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::YES
  }
  extern "C-unwind" fn upright(_this: &AnyObject, _cmd: Sel) -> usize {
    // UIInterfaceOrientationMaskPortrait.
    1 << 1
  }
  // SAFETY: the methods have the selectors' types.
  unsafe {
    builder.add_method(sel!(prefersStatusBarHidden), hidden as extern "C-unwind" fn(_, _) -> _);
    builder.add_method(
      sel!(supportedInterfaceOrientations),
      upright as extern "C-unwind" fn(_, _) -> _,
    );
  }
  builder.register()
}

/// The class of what hears the camera, its photos, its buttons and its viewer, registered once.
fn delegate_class() -> &'static AnyClass {
  if let Some(class) = AnyClass::get(c"LurqCameraKitDelegate") {
    return class;
  }
  let mut builder = ClassBuilder::new(c"LurqCameraKitDelegate", NSObject::class()).expect("a class name of its own");
  for protocol in [c"AVCapturePhotoCaptureDelegate", c"UIScrollViewDelegate"] {
    if let Some(protocol) = AnyProtocol::get(protocol) {
      builder.add_protocol(protocol);
    }
  }
  // SAFETY: the methods have the selectors' types.
  unsafe {
    builder.add_method(
      sel!(captureOutput:didFinishProcessingPhoto:error:),
      photo_taken as extern "C-unwind" fn(_, _, _, _, _),
    );
    builder.add_method(
      sel!(viewForZoomingInScrollView:),
      zoomed as extern "C-unwind" fn(_, _, _) -> _,
    );
    builder.add_method(
      sel!(scrollViewDidEndDecelerating:),
      paged as extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(sel!(lurqShutter:), press_shutter as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqDone:), press_done as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqCancel:), press_cancel as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqFlash:), press_flash as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqLens:), press_lens as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqPinch:), pinch as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqFocus:), focus as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqSwitch:), press_switch as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqStack:), press_stack as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqViewerClose:), viewer_close as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqViewerRemove:), viewer_remove as extern "C-unwind" fn(_, _, _));
    builder.add_method(sel!(lurqViewerTwice:), viewer_twice as extern "C-unwind" fn(_, _, _));
  }
  builder.register()
}
