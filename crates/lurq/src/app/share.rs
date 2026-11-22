//! Sharing an address through the system's share sheet: the list of apps and people the system offers something
//! to (Messages, Mail, Copy, AirDrop), which the person picks from.
//!
//! A [`Share`] is what is shared: an address, with a title and a text beside it where there are any. A window
//! shows the sheet for it ([`WindowHandle::share`](crate::app::WindowHandle::share), which a handler holds, and
//! [`Ctx::share`](crate::app::ctx::Ctx::share)): what the person picks there, and whether they pick anything, is
//! the system's and is not told.
//!
//! ```no_run
//! use lurq::{
//!   app::{Share, ShareError, ctx::Ctx},
//!   core::ElementRef,
//!   ui::Button,
//! };
//!
//! fn share_button(ctx: &mut Ctx, address: String, title: String) -> Button {
//!   let (window, button) = (ctx.window(), ElementRef::new());
//!   Button::new("Share").ref_element(button.clone()).on_click(move |_| {
//!     let share = Share::url(&address).title(&title).at(&button);
//!     if window.share(share) == Err(ShareError::Unsupported) {
//!       // No share sheet here: the address is copied instead.
//!       window.copy_to_clipboard(address.clone());
//!     }
//!   })
//! }
//! ```
//!
//! # What each platform does
//!
//! - **iOS**: `UIActivityViewController`, presented over what the window shows. On an iPad it is a popover that points
//!   at [`Share::at`], or at the middle of the window where none is given. The title is the subject of what takes one
//!   (a mail).
//! - **macOS**: `NSSharingServicePicker`, the menu of services, at [`Share::at`] or the middle of the window. It has no
//!   place for a title.
//! - **Windows**: the system's share window (`DataTransferManager`, shown for the app's window), which the system
//!   places itself. It asks for a title, and is given the address where the share has none; the text is its
//!   description.
//! - **A browser**: `navigator.share`, where the browser has it and the page is a secure one. It is asked within the
//!   press that shares: a browser gives the sheet to nothing else.
//! - **Linux and the rest**: there is none. [`ShareError::Unsupported`], and nothing is done: an app copies the address
//!   then, or shows it.
//!
//! [`supported`] says ahead of a press whether there is a sheet, for an app that shows another button where there
//! is none.
//!
//! Only an address with a scheme is shared (`https:`, `mailto:`, an app's own), never a path or a `file:`
//! address: sharing a file is another thing, which this does not do yet.
//!
//! A test, or a tool that drives the app, puts a stand-in in place of the system with [`stand_in`]: what would
//! have been shared is given to it, and no sheet is shown.

use std::{cell::RefCell, fmt, sync::Arc};

use crate::core::{ElementRect, ElementRef};

/// What is shared through the system's share sheet: an address, with what is said beside it. See the
/// [module](self).
///
/// Its fields are read by what stands in for the system in a test ([`stand_in`]); it is built with
/// [`Share::url`] and the methods after it, and may come to hold more kinds of things than an address.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Share {
  /// The address shared.
  pub url: String,
  /// What the address is called: the subject of a mail, the title of the system's share window.
  pub title: Option<String>,
  /// A text that goes with the address.
  pub text: Option<String>,
  /// Where in the window the sheet comes from, in logical pixels: the element that was pressed.
  pub at: Option<ElementRect>,
}

impl Share {
  /// The address `url`, shared by itself: `https://lurq.dev/docs`.
  pub fn url(url: impl Into<String>) -> Self {
    Self {
      url: url.into(),
      title: None,
      text: None,
      at: None,
    }
  }

  /// What the address is called, where the system has a place for it: the subject of a mail on iOS, the title
  /// of the share window on Windows, the title a browser gives on. Not shown on macOS.
  pub fn title(mut self, title: impl Into<String>) -> Self {
    self.title = Some(title.into());
    self
  }

  /// A text that goes with the address: what a message starts with.
  pub fn text(mut self, text: impl Into<String>) -> Self {
    self.text = Some(text.into());
    self
  }

  /// The element the sheet comes from: the button that was pressed. An iPad's popover points at it and a Mac's
  /// menu opens beside it; an iPhone, Windows and a browser place the sheet themselves. Where it is now is
  /// what is taken, so it is called in the handler of the press, not ahead of it. Without one, and from an
  /// element that is in no tree, the sheet comes from the middle of the window.
  pub fn at(self, element: &ElementRef) -> Self {
    if element.is_attached() {
      self.at_rect(element.bounds())
    } else {
      self
    }
  }

  /// [`at`](Self::at) for a place in the window that is known as a rect, in logical pixels.
  pub fn at_rect(mut self, rect: ElementRect) -> Self {
    self.at = Some(rect);
    self
  }
}

/// Why nothing was shared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShareError {
  /// What is shared is not an address with a scheme, or it is a `file:` one: nothing was asked of the system.
  NotAnAddress,
  /// The platform has no share sheet, or the browser has none: an app copies the address instead.
  Unsupported,
  /// The system did not show the sheet, in its own words: for a log.
  Failed(String),
}

impl fmt::Display for ShareError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotAnAddress => f.write_str("it is not an address the system shares"),
      Self::Unsupported => f.write_str("this platform has no share sheet"),
      Self::Failed(reason) => f.write_str(reason),
    }
  }
}

impl std::error::Error for ShareError {}

type Sharer = Arc<dyn Fn(&Share) + Send + Sync>;

thread_local! {
  /// What is given the shares asked for from this thread in place of the system, where a test put one.
  static STAND_IN: RefCell<Option<Sharer>> = const { RefCell::new(None) };
}

/// Puts `sharer` in place of the system for what is shared from this thread (the UI thread of a test's tree,
/// where every handler runs) until the guard it gives is dropped: a window's `share` gives it the [`Share`] and
/// shows no sheet, and [`supported`] is `true`.
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use lurq::app::{Share, share};
///
/// let shared = Arc::new(Mutex::new(Vec::new()));
/// let log = shared.clone();
/// let _held = share::stand_in(move |share| log.lock().unwrap().push(share.url.clone()));
/// assert!(share::supported());
/// ```
pub fn stand_in(sharer: impl Fn(&Share) + Send + Sync + 'static) -> StandIn {
  let before = STAND_IN.with(|held| held.borrow_mut().replace(Arc::new(sharer)));
  StandIn { before }
}

/// Keeps a [`stand_in`] in place: dropped, the one before it is back.
#[must_use = "the stand-in is taken away when this is dropped"]
pub struct StandIn {
  before: Option<Sharer>,
}

impl Drop for StandIn {
  fn drop(&mut self) {
    let before = self.before.take();
    STAND_IN.with(|held| *held.borrow_mut() = before);
  }
}

/// Whether there is a share sheet to show: the system has one (iOS, macOS, Windows 10 and later), or the
/// browser does (`navigator.share`). `false` on Linux. An app that shows "Copy link" in place of "Share" where
/// there is none asks this; one that shares and copies when it could not reads
/// [`ShareError::Unsupported`] instead.
pub fn supported() -> bool {
  STAND_IN.with(|held| held.borrow().is_some()) || system::supported()
}

/// What a window does with a share it was asked for.
pub(crate) enum Asked {
  /// It was given to a stand-in, or shown already: nothing is left to do.
  Done,
  /// The window's shell shows the sheet for it, on its own thread. Never in a browser, which is asked at once.
  #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
  Shell(Share),
}

/// Takes a share a window was asked for: its address trimmed and checked, then given to the stand-in where
/// there is one. What is left is the shell's to show ([`show`]), where the platform has a sheet.
pub(crate) fn ask(mut share: Share) -> Result<Asked, ShareError> {
  share.url = share.url.trim().to_owned();
  if !super::open_url::has_scheme(&share.url) {
    return Err(ShareError::NotAnAddress);
  }
  // A title or a text of nothing but spaces is none.
  share.title = share.title.filter(|title| !title.trim().is_empty());
  share.text = share.text.filter(|text| !text.trim().is_empty());
  if let Some(sharer) = STAND_IN.with(|held| held.borrow().clone()) {
    sharer(&share);
    return Ok(Asked::Done);
  }
  if !system::supported() {
    return Err(ShareError::Unsupported);
  }
  // A browser gives its sheet only within the press that asks: it is asked here, and not by the shell later.
  #[cfg(target_arch = "wasm32")]
  {
    show(None, &share).map(|()| Asked::Done)
  }
  #[cfg(not(target_arch = "wasm32"))]
  Ok(Asked::Shell(share))
}

/// Shows the system's sheet for `share` over the window of `window`: called by the shell, on the thread of the
/// window. An error is what the system said at once; what happens in the sheet is not told.
#[cfg_attr(not(any(feature = "winit", target_arch = "wasm32")), allow(dead_code))]
pub(crate) fn show(window: Option<raw_window_handle::RawWindowHandle>, share: &Share) -> Result<(), ShareError> {
  system::show(window, share)
}

#[cfg(windows)]
mod system {
  use std::{cell::RefCell, ffi::c_void};

  use raw_window_handle::RawWindowHandle;
  use windows::{
    ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager},
    Foundation::{TypedEventHandler, Uri},
    Win32::{Foundation::HWND, UI::Shell::IDataTransferManagerInterop},
    core::{HSTRING, Ref, factory},
  };

  use super::{Share, ShareError};

  thread_local! {
    /// The window's manager of what it shares, and the handler it has of what was last shared: the handler is
    /// taken away before the next share gives another.
    static ASKED: RefCell<Option<(DataTransferManager, i64)>> = const { RefCell::new(None) };
  }

  /// Windows 10 from its 1703 has the share window for a desktop app.
  pub(super) fn supported() -> bool {
    DataTransferManager::IsSupported().unwrap_or(false)
  }

  pub(super) fn show(window: Option<RawWindowHandle>, share: &Share) -> Result<(), ShareError> {
    let Some(RawWindowHandle::Win32(handle)) = window else {
      return Err(ShareError::Failed("the window has no handle of the system".to_owned()));
    };
    let window = HWND(handle.hwnd.get() as *mut c_void);
    let failed = |error: windows::core::Error| ShareError::Failed(error.to_string());
    let interop = factory::<DataTransferManager, IDataTransferManagerInterop>().map_err(failed)?;
    // SAFETY: the handle is the shell's own window, which is there while its commands are run.
    let manager: DataTransferManager = unsafe { interop.GetForWindow(window) }.map_err(failed)?;
    if let Some((before, token)) = ASKED.with(|asked| asked.borrow_mut().take()) {
      let _ = before.RemoveDataRequested(token);
    }
    // The share window asks what is shared once it is shown.
    let shared = share.clone();
    let token = manager
      .DataRequested(&TypedEventHandler::new(
        move |_: Ref<DataTransferManager>, args: Ref<DataRequestedEventArgs>| {
          let Some(args) = args.as_ref() else {
            return Ok(());
          };
          let data = args.Request()?.Data()?;
          let properties = data.Properties()?;
          // It shows nothing without a title.
          properties.SetTitle(&HSTRING::from(shared.title.as_deref().unwrap_or(&shared.url)))?;
          if let Some(text) = &shared.text {
            properties.SetDescription(&HSTRING::from(text))?;
          }
          let address = HSTRING::from(&shared.url);
          // A link of the web is one; an address of another scheme is shared as its text.
          let web = ["http://", "https://"]
            .iter()
            .any(|scheme| shared.url.to_ascii_lowercase().starts_with(scheme));
          match Uri::CreateUri(&address) {
            Ok(uri) if web => data.SetWebLink(&uri),
            _ => data.SetText(&address),
          }
        },
      ))
      .map_err(failed)?;
    ASKED.with(|asked| *asked.borrow_mut() = Some((manager, token)));
    // SAFETY: as above.
    unsafe { interop.ShowShareUIForWindow(window) }.map_err(failed)
  }
}

/// CoreGraphics' rect, as AppKit and UIKit take one.
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod geometry {
  #[repr(C)]
  #[derive(Clone, Copy)]
  pub(super) struct Rect {
    pub(super) origin: Point,
    pub(super) size: Size,
  }

  #[repr(C)]
  #[derive(Clone, Copy)]
  pub(super) struct Point {
    pub(super) x: f64,
    pub(super) y: f64,
  }

  #[repr(C)]
  #[derive(Clone, Copy)]
  pub(super) struct Size {
    pub(super) width: f64,
    pub(super) height: f64,
  }

  // SAFETY: the layouts are CoreGraphics', of `CGFloat`s, which are `f64` on every device these systems run on.
  unsafe impl objc2::Encode for Point {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
  }

  unsafe impl objc2::Encode for Size {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
  }

  unsafe impl objc2::Encode for Rect {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
  }

  /// The place a sheet comes from in a view with `bounds`: `at`, or the middle of the view. `flipped` for a
  /// view whose y grows down, as lurq's rects do.
  pub(super) fn place(at: Option<crate::core::ElementRect>, bounds: Rect, flipped: bool) -> Rect {
    let Some(at) = at else {
      return Rect {
        origin: Point {
          x: bounds.origin.x + bounds.size.width / 2.0,
          y: bounds.origin.y + bounds.size.height / 2.0,
        },
        size: Size {
          width: 1.0,
          height: 1.0,
        },
      };
    };
    let (y, height) = (f64::from(at.y), f64::from(at.height));
    Rect {
      origin: Point {
        x: f64::from(at.x),
        y: if flipped { y } else { bounds.size.height - y - height },
      },
      size: Size {
        width: f64::from(at.width),
        height,
      },
    }
  }
}

#[cfg(target_os = "macos")]
mod system {
  use std::ffi::c_void;

  use objc2::{
    class, msg_send,
    rc::{Allocated, Retained},
    runtime::AnyObject,
  };
  use objc2_foundation::{NSArray, NSString, NSURL};
  use raw_window_handle::RawWindowHandle;

  use super::{
    Share, ShareError,
    geometry::{Rect, place},
  };

  pub(super) fn supported() -> bool {
    true
  }

  /// A share on its way to the main queue.
  struct Asked {
    view: Retained<AnyObject>,
    share: Share,
  }

  // SAFETY: the view is touched on the main thread alone, where it was taken and where it is used.
  unsafe impl Send for Asked {}

  pub(super) fn show(window: Option<RawWindowHandle>, share: &Share) -> Result<(), ShareError> {
    let Some(RawWindowHandle::AppKit(handle)) = window else {
      return Err(ShareError::Failed("the window has no view of the system".to_owned()));
    };
    if NSURL::URLWithString(&NSString::from_str(&share.url)).is_none() {
      return Err(ShareError::NotAnAddress);
    }
    // SAFETY: the view is the shell's own window's, which is there while its commands are run.
    let Some(view) = (unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<AnyObject>()) }) else {
      return Err(ShareError::Failed("the window has no view of the system".to_owned()));
    };
    // The picker is a menu that keeps the loop until it is closed: it is shown once the shell is out of the
    // turn that asked for it, as the main queue gets to it.
    let asked = Asked {
      view,
      share: share.clone(),
    };
    on_main(Box::new(move || pick(&asked)));
    Ok(())
  }

  /// Shows the menu of services. On the main thread only.
  fn pick(asked: &Asked) {
    let Some(address) = NSURL::URLWithString(&NSString::from_str(&asked.share.url)) else {
      return;
    };
    let mut items: Vec<Retained<AnyObject>> = vec![Retained::into_super(Retained::into_super(address))];
    if let Some(text) = &asked.share.text {
      items.push(Retained::into_super(Retained::into_super(NSString::from_str(text))));
    }
    let items = NSArray::from_retained_slice(&items);
    // SAFETY: on the main thread; the picker and the view are AppKit's, and take what is said here.
    unsafe {
      let picker: Allocated<AnyObject> = msg_send![class!(NSSharingServicePicker), alloc];
      let picker: Retained<AnyObject> = msg_send![picker, initWithItems: &*items];
      let bounds: Rect = msg_send![&*asked.view, bounds];
      let flipped: bool = msg_send![&*asked.view, isFlipped];
      let rect = place(asked.share.at, bounds, flipped);
      // Under the element: the edge of its rect where y is the greatest in a view whose y grows down, and the
      // least in one whose y grows up (`NSRectEdgeMaxY`, `NSRectEdgeMinY`).
      let under: usize = if flipped { 3 } else { 1 };
      let _: () = msg_send![&*picker, showRelativeToRect: rect, ofView: &*asked.view, preferredEdge: under];
    }
  }

  unsafe extern "C" {
    /// The main queue of libdispatch, which `dispatch_get_main_queue()` returns.
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
  }

  /// Runs `work` on the main thread, as the main queue gets to it.
  fn on_main(work: Box<dyn FnOnce() + Send>) {
    extern "C" fn run(context: *mut c_void) {
      // SAFETY: `context` is the box made below, given to this call once.
      let work = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce() + Send>>()) };
      work();
    }
    let context = Box::into_raw(Box::new(work)).cast::<c_void>();
    // SAFETY: the main queue lives as long as the process; `run` takes the box back exactly once.
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
  }
}

#[cfg(target_os = "ios")]
mod system {
  use std::{ffi::c_void, sync::Mutex};

  use objc2::{
    ClassType, class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, AnyProtocol, Bool, ClassBuilder, Sel},
    sel,
  };
  use objc2_foundation::{NSArray, NSObject, NSString, NSURL};
  use raw_window_handle::RawWindowHandle;

  use super::{
    Share, ShareError,
    geometry::{Rect, place},
  };

  /// The address the sheet that is shown shares, and what it is called: the sheet's item reads them when the
  /// sheet asks. One sheet at a time.
  static SHARED: Mutex<Option<(String, String)>> = Mutex::new(None);

  pub(super) fn supported() -> bool {
    true
  }

  pub(super) fn show(window: Option<RawWindowHandle>, share: &Share) -> Result<(), ShareError> {
    let Some(RawWindowHandle::UiKit(handle)) = window else {
      return Err(ShareError::Failed("the window has no view of the system".to_owned()));
    };
    if objc2::MainThreadMarker::new().is_none() {
      return Err(ShareError::Failed("the sheet is shown from the main thread".to_owned()));
    }
    if NSURL::URLWithString(&NSString::from_str(&share.url)).is_none() {
      return Err(ShareError::NotAnAddress);
    }
    // SAFETY: on the main thread; the view is the shell's own window's, and the controllers are UIKit's.
    unsafe {
      let view: &AnyObject = handle.ui_view.cast::<AnyObject>().as_ref();
      let window: Option<Retained<AnyObject>> = msg_send![view, window];
      let root: Option<Retained<AnyObject>> = match &window {
        Some(window) => msg_send![&**window, rootViewController],
        None => None,
      };
      let Some(mut over) = root else {
        return Err(ShareError::Failed("the window has no view controller".to_owned()));
      };
      // Over what the window presents already: a picker of the system, another sheet.
      loop {
        let presented: Option<Retained<AnyObject>> = msg_send![&*over, presentedViewController];
        match presented {
          Some(presented) => over = presented,
          None => break,
        }
      }
      *SHARED.lock().unwrap() = Some((share.url.clone(), share.title.clone().unwrap_or_default()));
      let item: Retained<AnyObject> = msg_send![item_class(), new];
      let mut items = vec![item];
      if let Some(text) = &share.text {
        items.push(Retained::into_super(Retained::into_super(NSString::from_str(text))));
      }
      let items = NSArray::from_retained_slice(&items);
      let sheet: Allocated<AnyObject> = msg_send![class!(UIActivityViewController), alloc];
      let sheet: Retained<AnyObject> = msg_send![
        sheet,
        initWithActivityItems: &*items,
        applicationActivities: None::<&NSArray<AnyObject>>
      ];
      // On an iPad the sheet stands in a popover, which must be told what it hangs from.
      let popover: Option<Retained<AnyObject>> = msg_send![&*sheet, popoverPresentationController];
      if let Some(popover) = popover {
        let bounds: Rect = msg_send![view, bounds];
        let _: () = msg_send![&*popover, setSourceView: view];
        let _: () = msg_send![&*popover, setSourceRect: place(share.at, bounds, true)];
        if share.at.is_none() {
          // From the middle of the window it points at nothing.
          let _: () = msg_send![&*popover, setPermittedArrowDirections: 0usize];
        }
      }
      let _: () =
        msg_send![&*over, presentViewController: &*sheet, animated: Bool::YES, completion: std::ptr::null::<c_void>()];
    }
    Ok(())
  }

  /// The address that is shared, for the sheet: given back to the pool that called.
  fn address() -> *mut AnyObject {
    let shared = SHARED.lock().unwrap();
    let address = shared
      .as_ref()
      .and_then(|(address, _)| NSURL::URLWithString(&NSString::from_str(address)));
    match address {
      Some(address) => Retained::autorelease_ptr(Retained::into_super(Retained::into_super(address))),
      None => std::ptr::null_mut(),
    }
  }

  /// What stands for the item before it is asked for: the address itself.
  extern "C-unwind" fn placeholder(_this: &AnyObject, _cmd: Sel, _sheet: *mut AnyObject) -> *mut AnyObject {
    address()
  }

  /// The item, for whatever takes it.
  extern "C-unwind" fn item(
    _this: &AnyObject,
    _cmd: Sel,
    _sheet: *mut AnyObject,
    _activity: *mut AnyObject,
  ) -> *mut AnyObject {
    address()
  }

  /// What the item is called, for what takes a subject (a mail).
  extern "C-unwind" fn subject(
    _this: &AnyObject,
    _cmd: Sel,
    _sheet: *mut AnyObject,
    _activity: *mut AnyObject,
  ) -> *mut AnyObject {
    let shared = SHARED.lock().unwrap();
    let title = shared.as_ref().map_or("", |(_, title)| title.as_str());
    Retained::autorelease_ptr(Retained::into_super(Retained::into_super(NSString::from_str(title))))
  }

  /// The class of what the sheet shares (`UIActivityItemSource`), registered once.
  fn item_class() -> &'static AnyClass {
    if let Some(class) = AnyClass::get(c"LurqShareItem") {
      return class;
    }
    let mut builder = ClassBuilder::new(c"LurqShareItem", NSObject::class()).expect("a class name of its own");
    if let Some(protocol) = AnyProtocol::get(c"UIActivityItemSource") {
      builder.add_protocol(protocol);
    }
    // SAFETY: the methods have the selectors' types: objects taken, and an object given.
    unsafe {
      builder.add_method(
        sel!(activityViewControllerPlaceholderItem:),
        placeholder as extern "C-unwind" fn(_, _, _) -> _,
      );
      builder.add_method(
        sel!(activityViewController:itemForActivityType:),
        item as extern "C-unwind" fn(_, _, _, _) -> _,
      );
      builder.add_method(
        sel!(activityViewController:subjectForActivityType:),
        subject as extern "C-unwind" fn(_, _, _, _) -> _,
      );
    }
    builder.register()
  }
}

#[cfg(target_arch = "wasm32")]
mod system {
  use raw_window_handle::RawWindowHandle;
  use wasm_bindgen::{JsCast, JsValue};

  use super::{Share, ShareError};

  /// The page's `navigator`, and its `share` where it has one.
  fn sharing() -> Option<(web_sys::Navigator, js_sys::Function)> {
    let navigator = web_sys::window()?.navigator();
    let share = js_sys::Reflect::get(&navigator, &JsValue::from_str("share")).ok()?;
    share
      .dyn_into::<js_sys::Function>()
      .ok()
      .map(|share| (navigator, share))
  }

  pub(super) fn supported() -> bool {
    sharing().is_some()
  }

  pub(super) fn show(_window: Option<RawWindowHandle>, share: &Share) -> Result<(), ShareError> {
    let (navigator, function) = sharing().ok_or(ShareError::Unsupported)?;
    let data = js_sys::Object::new();
    let say = |name: &str, value: &str| {
      let _ = js_sys::Reflect::set(&data, &JsValue::from_str(name), &JsValue::from_str(value));
    };
    say("url", &share.url);
    if let Some(title) = &share.title {
      say("title", title);
    }
    if let Some(text) = &share.text {
      say("text", text);
    }
    let answer = function
      .call1(&navigator, &data)
      .map_err(|error| ShareError::Failed(format!("{error:?}")))?;
    // What it gives is a promise that fails where the person closed the sheet: heard, so that the page's
    // console is not told of a failure nobody waited for.
    if let Ok(promise) = answer.dyn_into::<js_sys::Promise>() {
      wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
      });
    }
    Ok(())
  }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_arch = "wasm32")))]
mod system {
  use raw_window_handle::RawWindowHandle;

  use super::{Share, ShareError};

  pub(super) fn supported() -> bool {
    false
  }

  pub(super) fn show(_window: Option<RawWindowHandle>, _share: &Share) -> Result<(), ShareError> {
    Err(ShareError::Unsupported)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Mutex;

  use super::*;

  /// What a stand-in was given, as lines.
  fn given() -> (Arc<Mutex<Vec<String>>>, StandIn) {
    let shared = Arc::new(Mutex::new(Vec::<String>::new()));
    let log = shared.clone();
    let held = stand_in(move |share| {
      log
        .lock()
        .unwrap()
        .push(format!("{} | {:?} | {:?}", share.url, share.title, share.text));
    });
    (shared, held)
  }

  #[test]
  fn a_stand_in_is_given_what_is_shared_with_its_address_trimmed() {
    let (shared, _held) = given();
    assert!(supported());
    let asked = ask(Share::url(" https://lurq.dev/a ").title("The docs").text("Read this"));
    assert!(matches!(asked, Ok(Asked::Done)));
    // A title of spaces is none.
    assert!(matches!(
      ask(Share::url("mailto:ivan@lurq.dev").title("  ")),
      Ok(Asked::Done)
    ));
    assert_eq!(
      *shared.lock().unwrap(),
      [
        "https://lurq.dev/a | Some(\"The docs\") | Some(\"Read this\")",
        "mailto:ivan@lurq.dev | None | None",
      ]
    );
  }

  #[test]
  fn only_an_address_with_a_scheme_is_shared() {
    let (shared, _held) = given();
    for url in ["", "lurq.dev", "C:\\Users\\notes.txt", "file:///etc/passwd"] {
      assert!(matches!(ask(Share::url(url)), Err(ShareError::NotAnAddress)), "{url:?}");
    }
    assert!(shared.lock().unwrap().is_empty(), "nothing was given on");
  }

  #[test]
  fn the_stand_in_before_is_back_once_one_is_dropped() {
    let (outer, _held) = given();
    {
      let (inner, _held) = given();
      let _ = ask(Share::url("https://lurq.dev/inner"));
      assert_eq!(inner.lock().unwrap().len(), 1);
    }
    let _ = ask(Share::url("https://lurq.dev/outer"));
    assert_eq!(*outer.lock().unwrap(), ["https://lurq.dev/outer | None | None"]);
  }

  #[test]
  fn a_share_takes_the_place_of_the_element_it_comes_from() {
    let rect = ElementRect {
      x: 10.0,
      y: 20.0,
      relative_x: 0.0,
      relative_y: 0.0,
      width: 30.0,
      height: 40.0,
    };
    assert_eq!(Share::url("https://lurq.dev").at_rect(rect).at, Some(rect));
    // An element that is in no tree has no place: the sheet comes from the middle of the window.
    assert_eq!(Share::url("https://lurq.dev").at(&ElementRef::new()).at, None);
  }
}
