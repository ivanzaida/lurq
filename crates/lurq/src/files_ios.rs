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

use super::{Answer, Answering, Asks, DroppedFile, FileDialog, FileSource};

#[link(name = "UniformTypeIdentifiers", kind = "framework")]
unsafe extern "C" {}

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
  super::on_main(move |main| choose(main, dialog, answering));
  answer
}

/// For files to open, the sources the device has, in a menu where there are more than one; the dialog of what it
/// asks otherwise.
fn choose(main: MainThreadMarker, dialog: FileDialog, answering: Answering) {
  if dialog.asks != Asks::Files {
    present(main, dialog, answering);
    return;
  }
  let mut has = vec![FileSource::PhotoLibrary];
  if camera_ready() {
    has.push(FileSource::Camera);
  }
  has.push(FileSource::Files);
  let offered = dialog.offered(&has);
  if let [source] = offered.as_slice() {
    open_source(main, *source, dialog, answering);
    return;
  }
  menu(main, &offered, dialog, answering);
}

/// Opens one source for the dialog.
fn open_source(main: MainThreadMarker, source: FileSource, dialog: FileDialog, answering: Answering) {
  match source {
    FileSource::Files => present(main, dialog, answering),
    FileSource::PhotoLibrary => super::photos::present(main, &dialog, answering),
    FileSource::Camera => camera(main, answering, dialog.multiple),
  }
}

/// The system's menu of sources, as Safari's file input has it, and Cancel; over the top of what the app shows.
fn menu(main: MainThreadMarker, offered: &[FileSource], dialog: FileDialog, answering: Answering) {
  let Some(over) = top_controller() else {
    answering.give(Vec::new());
    return;
  };
  let words = dialog
    .words
    .clone()
    .unwrap_or_else(|| super::SourceWords::of(&crate::core::Locale::english()));
  // What the menu answers with: taken by the first of its actions to run.
  let held = std::sync::Arc::new(Mutex::new(Some((dialog, answering))));
  // SAFETY: on the main thread, with UIKit's classes; each action's block is called once, on the main thread.
  unsafe {
    let alert: Retained<AnyObject> = msg_send![
      objc2::class!(UIAlertController),
      alertControllerWithTitle: std::ptr::null::<NSString>(),
      message: std::ptr::null::<NSString>(),
      preferredStyle: 0isize
    ];
    for source in offered {
      let title = match source {
        FileSource::PhotoLibrary => &words.photo_library,
        FileSource::Camera => &words.take_photo,
        FileSource::Files => &words.choose_file,
      };
      let (source, held) = (*source, held.clone());
      let block = block2::RcBlock::new(move |_action: *mut AnyObject| {
        if let Some((dialog, answering)) = held.lock().unwrap().take() {
          open_source(main, source, dialog, answering);
        }
      });
      let action: Retained<AnyObject> = msg_send![
        objc2::class!(UIAlertAction),
        actionWithTitle: &*NSString::from_str(title),
        style: 0isize,
        handler: &*block
      ];
      let _: () = msg_send![&*alert, addAction: &*action];
    }
    let cancelled = held.clone();
    let block = block2::RcBlock::new(move |_action: *mut AnyObject| {
      if let Some((_, answering)) = cancelled.lock().unwrap().take() {
        answering.give(Vec::new());
      }
    });
    let cancel: Retained<AnyObject> = msg_send![
      objc2::class!(UIAlertAction),
      actionWithTitle: &*NSString::from_str(&words.cancel),
      style: 1isize,
      handler: &*block
    ];
    let _: () = msg_send![&*alert, addAction: &*cancel];
    // On an iPad the menu stands in a popover, which hangs from the middle of what is shown.
    let popover: Option<Retained<AnyObject>> = msg_send![&*alert, popoverPresentationController];
    if let Some(popover) = popover {
      let view: Retained<AnyObject> = msg_send![&*over, view];
      let bounds: Rect = msg_send![&*view, bounds];
      let middle = Rect {
        origin: Point {
          x: bounds.size.width / 2.0,
          y: bounds.size.height / 2.0,
        },
        size: Size {
          width: 0.0,
          height: 0.0,
        },
      };
      let _: () = msg_send![&*popover, setSourceView: &*view];
      let _: () = msg_send![&*popover, setSourceRect: middle];
      let _: () = msg_send![&*popover, setPermittedArrowDirections: 0usize];
    }
    let _: () =
      msg_send![&*over, presentViewController: &*alert, animated: Bool::YES, completion: std::ptr::null::<c_void>()];
  }
}

/// Whether a picture can be taken: the device has a camera, and the app's Info.plist says why it uses it, without
/// which iOS ends the app that asks for it. Where it has the camera and not the words, the log says once what to add.
fn camera_ready() -> bool {
  // SAFETY: class methods of UIKit and Foundation that take what is given here.
  unsafe {
    let has: Bool = msg_send![objc2::class!(UIImagePickerController), isSourceTypeAvailable: 1isize];
    if !has.as_bool() {
      return false;
    }
    let bundle: Retained<AnyObject> = msg_send![objc2::class!(NSBundle), mainBundle];
    let said: Option<Retained<AnyObject>> =
      msg_send![&*bundle, objectForInfoDictionaryKey: &*NSString::from_str("NSCameraUsageDescription")];
    if said.is_none() {
      static SAID: std::sync::Once = std::sync::Once::new();
      SAID.call_once(|| {
        tracing::warn!(
          "lurq: the file dialog offers no camera: the app's Info.plist has no NSCameraUsageDescription, the words \
           iOS shows when it asks for the camera"
        );
      });
      return false;
    }
    true
  }
}

/// The camera that is open, and what is told the pictures.
struct Shooting {
  answering: Answering,
  picker: Retained<AnyObject>,
  _delegate: Retained<AnyObject>,
  /// Whether the dialog takes several files: the camera opens again after each picture kept, until it is put
  /// away, and the pictures kept are given then.
  multiple: bool,
  taken: Vec<DroppedFile>,
}

// SAFETY: touched on the main thread alone.
unsafe impl Send for Shooting {}

static SHOOTING: Mutex<Option<Shooting>> = Mutex::new(None);

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
  fn UIImageJPEGRepresentation(image: *mut AnyObject, quality: f64) -> *mut AnyObject;
}

/// The system's camera: for one picture, or for one after another where the dialog takes several. iOS's camera
/// takes one at a time; it is shown anew after each picture kept.
fn camera(_main: MainThreadMarker, answering: Answering, multiple: bool) {
  show_camera(answering, multiple, Vec::new());
}

/// Shows the camera, with the pictures kept so far.
fn show_camera(answering: Answering, multiple: bool, taken: Vec<DroppedFile>) {
  let mut shooting = SHOOTING.lock().unwrap();
  let over = top_controller();
  let (Some(over), None) = (over, shooting.as_ref()) else {
    answering.give(taken);
    return;
  };
  // SAFETY: on the main thread, with UIKit's picker of the camera.
  unsafe {
    let picker: Retained<AnyObject> = msg_send![objc2::class!(UIImagePickerController), new];
    let _: () = msg_send![&*picker, setSourceType: 1isize];
    let delegate: Retained<AnyObject> = msg_send![camera_delegate_class(), new];
    let _: () = msg_send![&*picker, setDelegate: &*delegate];
    let _: () =
      msg_send![&*over, presentViewController: &*picker, animated: Bool::YES, completion: std::ptr::null::<c_void>()];
    *shooting = Some(Shooting {
      answering,
      picker,
      _delegate: delegate,
      multiple,
      taken,
    });
  }
}

/// The picture taken: kept as a JPEG in a folder of its own of the temporary folder, and given; where the dialog
/// takes several, kept with the others and the camera shown again.
extern "C-unwind" fn shot(_this: &AnyObject, _cmd: Sel, _picker: &AnyObject, info: &AnyObject) {
  let Some(mut shooting) = SHOOTING.lock().unwrap().take() else {
    return;
  };
  // SAFETY: the picker's info, a dictionary that holds the picture taken under its key.
  let file = unsafe {
    let image: Option<Retained<AnyObject>> =
      msg_send![info, objectForKey: &*NSString::from_str("UIImagePickerControllerOriginalImage")];
    image.and_then(|image| {
      let data = UIImageJPEGRepresentation(Retained::as_ptr(&image).cast_mut(), 0.9);
      if data.is_null() {
        return None;
      }
      let folder = std::env::temp_dir().join(format!("lurq-camera-{}", std::process::id()));
      std::fs::create_dir_all(&folder).ok()?;
      let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or_default();
      let path = folder.join(format!("Photo-{seconds}.jpg"));
      let written: Bool = msg_send![data, writeToFile: &*NSString::from_str(path.to_str()?), atomically: Bool::YES];
      written.as_bool().then_some(path)
    })
  };
  shooting.taken.extend(file.map(DroppedFile::from_path));
  let Shooting {
    answering,
    picker,
    multiple,
    taken,
    ..
  } = shooting;
  if !multiple {
    // SAFETY: on the main thread, the picker shown.
    unsafe {
      let _: () = msg_send![&*picker, dismissViewControllerAnimated: Bool::YES, completion: std::ptr::null::<c_void>()];
    }
    answering.give(taken);
    return;
  }
  // The next picture: once this camera is put away, another is shown.
  let next = Mutex::new(Some((answering, taken)));
  let again = block2::RcBlock::new(move || {
    if let Some((answering, taken)) = next.lock().unwrap().take() {
      show_camera(answering, true, taken);
    }
  });
  // SAFETY: on the main thread, the picker shown; the block is called once, when it is put away.
  unsafe {
    let _: () = msg_send![&*picker, dismissViewControllerAnimated: Bool::YES, completion: &*again];
  }
}

/// The camera was put away: the pictures kept before are given, none for a camera of one picture.
extern "C-unwind" fn shot_none(_this: &AnyObject, _cmd: Sel, _picker: &AnyObject) {
  if let Some(shooting) = SHOOTING.lock().unwrap().take() {
    unsafe {
      let _: () =
        msg_send![&*shooting.picker, dismissViewControllerAnimated: Bool::YES, completion: std::ptr::null::<c_void>()];
    }
    shooting.answering.give(shooting.taken);
  }
}

/// The class of what hears the camera, registered once.
fn camera_delegate_class() -> &'static AnyClass {
  if let Some(class) = AnyClass::get(c"LurqCameraDelegate") {
    return class;
  }
  let mut builder = ClassBuilder::new(c"LurqCameraDelegate", NSObject::class()).expect("a class name of its own");
  for protocol in [c"UIImagePickerControllerDelegate", c"UINavigationControllerDelegate"] {
    if let Some(protocol) = AnyProtocol::get(protocol) {
      builder.add_protocol(protocol);
    }
  }
  // SAFETY: the methods have the selectors' types.
  unsafe {
    builder.add_method(
      sel!(imagePickerController:didFinishPickingMediaWithInfo:),
      shot as extern "C-unwind" fn(_, _, _, _),
    );
    builder.add_method(
      sel!(imagePickerControllerDidCancel:),
      shot_none as extern "C-unwind" fn(_, _, _),
    );
  }
  builder.register()
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
pub(super) fn top_controller() -> Option<Retained<AnyObject>> {
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

/// A rectangle of UIKit's (`CGRect`), as a view's bounds and a popover's place are.
#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
  origin: Point,
  size: Size,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
  x: f64,
  y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
  width: f64,
  height: f64,
}

// SAFETY: the layouts are CoreGraphics', of `CGFloat`s, which are `f64` on every device iOS runs on.
unsafe impl objc2::Encode for Point {
  const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl objc2::Encode for Size {
  const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl objc2::Encode for Rect {
  const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
}
