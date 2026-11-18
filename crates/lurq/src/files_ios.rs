//! The file dialog on iOS: the system's document picker (`UIDocumentPickerViewController`), shown over the app.
//!
//! - Files to open are taken as copies (`asCopy`): the system puts them in the app's own temporary folder, and the app
//!   reads them by their paths, as on a desktop.
//! - A folder is the folder itself, which the app may read and write while it runs: its access is asked of the system
//!   (`startAccessingSecurityScopedResource`) and kept.
//! - Where to save: iOS chooses no path ahead of a file. An empty file of the dialog's file name is made in the
//!   temporary folder, and the picker has the person put it where they choose (Files, iCloud Drive, a folder on the
//!   phone); the path it has there, with access to it kept, is what the app writes, as on a desktop.
//!
//! The picker is shown on the main thread, over the top of what the app's window shows; one is open at a time.

use std::{ffi::c_void, path::PathBuf, sync::Mutex};

use objc2::{
  ClassType, MainThreadMarker, msg_send,
  rc::Retained,
  runtime::{AnyClass, AnyObject, AnyProtocol, Bool, ClassBuilder, Sel},
  sel,
};
use objc2_foundation::{NSArray, NSObject, NSString, NSURL};

use super::{Answer, Answering, Asks, DroppedFile, FileDialog};

#[link(name = "UniformTypeIdentifiers", kind = "framework")]
unsafe extern "C" {}

unsafe extern "C" {
  /// The main queue of libdispatch, which `dispatch_get_main_queue()` returns.
  static _dispatch_main_q: c_void;
  fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// The dialog that is open: what it asks, and what is told what was chosen. One at a time.
struct Open {
  asks: Asks,
  answering: Answering,
  /// The picker, held while it is shown, and the object that hears it.
  _picker: Retained<AnyObject>,
  _delegate: Retained<AnyObject>,
}

// SAFETY: the picker and its delegate are touched on the main thread alone; the slot is only a place to keep them.
unsafe impl Send for Open {}

static OPEN: Mutex<Option<Open>> = Mutex::new(None);

pub(super) fn pick(dialog: FileDialog) -> Answer {
  let (answer, answering) = Answer::new();
  on_main(move |main| present(main, dialog, answering));
  answer
}

/// Runs `work` on the main thread, as the main queue gets to it.
fn on_main(work: impl FnOnce(MainThreadMarker) + Send + 'static) {
  type Work = Box<dyn FnOnce(MainThreadMarker) + Send>;
  extern "C" fn run(context: *mut c_void) {
    // SAFETY: `context` is the box made below, given to this call once.
    let work = unsafe { Box::from_raw(context.cast::<Work>()) };
    if let Some(main) = MainThreadMarker::new() {
      work(main);
    }
  }
  let context = Box::into_raw(Box::new(Box::new(work) as Work)).cast::<c_void>();
  // SAFETY: the main queue lives as long as the process; `run` takes the box back exactly once.
  unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
}

/// Shows the picker the dialog asks for. Where none can be shown, or one is open already, nothing is chosen.
fn present(_main: MainThreadMarker, dialog: FileDialog, answering: Answering) {
  let mut open = OPEN.lock().unwrap();
  if open.is_some() {
    answering.give(Vec::new());
    return;
  }
  let Some(over) = top_controller() else {
    answering.give(Vec::new());
    return;
  };
  // SAFETY: on the main thread, with UIKit's classes and the selectors they answer.
  let picker = unsafe {
    let class = objc2::class!(UIDocumentPickerViewController);
    let picker: Option<Retained<AnyObject>> = match dialog.asks {
      Asks::Files => {
        let types = content_types(&dialog.filters);
        msg_send![msg_send![class, alloc], initForOpeningContentTypes: &*types, asCopy: Bool::YES]
      }
      Asks::Folder => {
        let types = NSArray::from_retained_slice(&[type_of("public.folder")]);
        msg_send![msg_send![class, alloc], initForOpeningContentTypes: &*types, asCopy: Bool::NO]
      }
      Asks::Save => {
        let Some(made) = empty_file(dialog.file_name.as_deref()) else {
          answering.give(Vec::new());
          return;
        };
        let urls = NSArray::from_retained_slice(&[made]);
        // Moved, not copied: the file is then the one where the person put it, which the app may write.
        msg_send![msg_send![class, alloc], initForExportingURLs: &*urls, asCopy: Bool::NO]
      }
    };
    let Some(picker) = picker else {
      answering.give(Vec::new());
      return;
    };
    if dialog.asks == Asks::Files && dialog.multiple {
      let _: () = msg_send![&*picker, setAllowsMultipleSelection: Bool::YES];
    }
    if let Some(folder) = &dialog.folder
      && let Some(url) = file_url(folder)
    {
      let _: () = msg_send![&*picker, setDirectoryURL: &*url];
    }
    picker
  };
  let delegate: Retained<AnyObject> = unsafe { msg_send![delegate_class(), new] };
  unsafe {
    let _: () = msg_send![&*picker, setDelegate: &*delegate];
    let _: () =
      msg_send![&*over, presentViewController: &*picker, animated: Bool::YES, completion: std::ptr::null::<c_void>()];
  }
  *open = Some(Open {
    asks: dialog.asks,
    answering,
    _picker: picker,
    _delegate: delegate,
  });
}

/// The kinds of files the filters name, by their extensions; every file where they name none.
fn content_types(filters: &[(String, Vec<String>)]) -> Retained<NSArray<AnyObject>> {
  let class = AnyClass::get(c"UTType");
  let mut types: Vec<Retained<AnyObject>> = Vec::new();
  if let Some(class) = class {
    for extension in filters.iter().flat_map(|(_, extensions)| extensions) {
      // SAFETY: `typeWithFilenameExtension:` takes a string and gives a type or nil.
      let found: Option<Retained<AnyObject>> =
        unsafe { msg_send![class, typeWithFilenameExtension: &*NSString::from_str(extension)] };
      types.extend(found);
    }
  }
  if types.is_empty() {
    types.push(type_of("public.item"));
  }
  NSArray::from_retained_slice(&types)
}

/// The type of the identifier, as `UTType` knows it.
fn type_of(identifier: &str) -> Retained<AnyObject> {
  let class = AnyClass::get(c"UTType").expect("UniformTypeIdentifiers is linked");
  // SAFETY: `typeWithIdentifier:` takes a string; the identifiers given here are the system's own.
  unsafe { msg_send![class, typeWithIdentifier: &*NSString::from_str(identifier)] }
}

fn file_url(path: &std::path::Path) -> Option<Retained<NSURL>> {
  let path = path.to_str()?;
  Some(NSURL::fileURLWithPath(&NSString::from_str(path)))
}

/// An empty file of `name` in a folder of its own in the temporary folder, to be put where the person says.
fn empty_file(name: Option<&str>) -> Option<Retained<AnyObject>> {
  let name = name.filter(|name| !name.is_empty()).unwrap_or("Untitled");
  let folder = std::env::temp_dir().join(format!("lurq-save-{}", std::process::id()));
  std::fs::create_dir_all(&folder).ok()?;
  let path = folder.join(name);
  std::fs::write(&path, []).ok()?;
  file_url(&path).map(Retained::into_super).map(Retained::into_super)
}

/// The path of a URL the picker gave, with its access kept where it is outside the app's own folders.
fn path_of(url: &AnyObject, keep_access: bool) -> Option<PathBuf> {
  // SAFETY: an `NSURL` the picker gave; `path` gives its path, and access is asked of a URL the picker gave.
  unsafe {
    if keep_access {
      let _: Bool = msg_send![url, startAccessingSecurityScopedResource];
    }
    let path: Option<Retained<NSString>> = msg_send![url, path];
    path.map(|path| PathBuf::from(path.to_string()))
  }
}

/// The picker chose: what it chose is given to the dialog's future.
extern "C-unwind" fn picked(_this: &AnyObject, _cmd: Sel, _picker: &AnyObject, urls: &NSArray<AnyObject>) {
  let Some(open) = OPEN.lock().unwrap().take() else {
    return;
  };
  // A copy is the app's own; a folder and a file put somewhere else are reached with the access the system gives.
  let keep_access = open.asks != Asks::Files;
  let files = urls
    .iter()
    .filter_map(|url| path_of(&url, keep_access))
    .map(DroppedFile::from_path)
    .collect();
  open.answering.give(files);
}

/// The picker was dismissed: nothing is chosen.
extern "C-unwind" fn cancelled(_this: &AnyObject, _cmd: Sel, _picker: &AnyObject) {
  if let Some(open) = OPEN.lock().unwrap().take() {
    open.answering.give(Vec::new());
  }
}

/// The class of what hears the picker, registered once.
fn delegate_class() -> &'static AnyClass {
  if let Some(class) = AnyClass::get(c"LurqDocumentPickerDelegate") {
    return class;
  }
  let mut builder =
    ClassBuilder::new(c"LurqDocumentPickerDelegate", NSObject::class()).expect("a class name of its own");
  if let Some(protocol) = AnyProtocol::get(c"UIDocumentPickerDelegate") {
    builder.add_protocol(protocol);
  }
  // SAFETY: the methods have the selectors' types: an object and an array, and an object.
  unsafe {
    builder.add_method(
      sel!(documentPicker:didPickDocumentsAtURLs:),
      picked as extern "C-unwind" fn(_, _, _, _),
    );
    builder.add_method(
      sel!(documentPickerWasCancelled:),
      cancelled as extern "C-unwind" fn(_, _, _),
    );
  }
  builder.register()
}

/// The view controller over everything the app shows: the key window's root, or what it presents.
fn top_controller() -> Option<Retained<AnyObject>> {
  // SAFETY: on the main thread; the scenes, windows and controllers are UIKit's own.
  unsafe {
    let app: Retained<AnyObject> = msg_send![objc2::class!(UIApplication), sharedApplication];
    let scenes: Retained<AnyObject> = msg_send![&*app, connectedScenes];
    let scenes: Retained<NSArray<AnyObject>> = msg_send![&*scenes, allObjects];
    let mut top: Option<Retained<AnyObject>> = None;
    for scene in scenes.iter() {
      let is_window_scene: Bool = msg_send![&*scene, isKindOfClass: objc2::class!(UIWindowScene)];
      if !is_window_scene.as_bool() {
        continue;
      }
      let window: Option<Retained<AnyObject>> = msg_send![&*scene, keyWindow];
      let window = match window {
        Some(window) => Some(window),
        None => {
          let windows: Retained<NSArray<AnyObject>> = msg_send![&*scene, windows];
          windows.iter().next()
        }
      };
      if let Some(window) = window {
        top = msg_send![&*window, rootViewController];
        if top.is_some() {
          break;
        }
      }
    }
    let mut top = top?;
    loop {
      let presented: Option<Retained<AnyObject>> = msg_send![&*top, presentedViewController];
      match presented {
        Some(presented) => top = presented,
        None => return Some(top),
      }
    }
  }
}
