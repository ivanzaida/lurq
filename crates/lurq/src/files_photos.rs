//! The photo library as a source of files, on iOS and macOS: the system's picker of photos
//! (`PHPickerViewController`), which shows the pictures and videos the dialog's filters take and gives only what the
//! person chose, with no permission asked. Each one chosen is copied, in its own format (`HEIC`, `MOV`), to a folder
//! of the app's temporary folder, and given as a file with its path, as one of Files is.
//!
//! The picker is shown on the main thread: over the top of what the app shows on iOS, as a sheet over the window in
//! front on macOS. One is open at a time.

use std::{
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
  },
};

use block2::RcBlock;
use objc2::{
  ClassType, MainThreadMarker, msg_send,
  rc::Retained,
  runtime::{AnyClass, AnyObject, AnyProtocol, Bool, ClassBuilder, Sel},
  sel,
};
use objc2_foundation::{NSArray, NSObject, NSString};

use super::{Answer, Answering, DroppedFile, FileDialog};

#[link(name = "PhotosUI", kind = "framework")]
unsafe extern "C" {}

/// The picker that is open, and what is told what was chosen.
struct Open {
  answering: Answering,
  picker: Retained<AnyObject>,
  _delegate: Retained<AnyObject>,
  /// On macOS, the window the picker is a sheet of, and the sheet.
  #[cfg(target_os = "macos")]
  sheet: Option<(Retained<AnyObject>, Retained<AnyObject>)>,
}

// SAFETY: what it holds of UIKit's and AppKit's is touched on the main thread alone.
unsafe impl Send for Open {}

static OPEN: Mutex<Option<Open>> = Mutex::new(None);

/// Shows the photo library for `dialog`, on the main thread; its answer is what was chosen. On iOS it is opened from
/// the menu of sources instead.
#[cfg_attr(target_os = "ios", allow(dead_code))]
pub(super) fn pick(dialog: FileDialog) -> Answer {
  let (answer, answering) = Answer::new();
  super::on_main(move |main| present(main, &dialog, answering));
  answer
}

/// Shows the photo library, its answer given to `answering`: from the main thread, as a menu's choice is.
pub(super) fn present(_main: MainThreadMarker, dialog: &FileDialog, answering: Answering) {
  let mut open = OPEN.lock().unwrap();
  if open.is_some() {
    answering.give(Vec::new());
    return;
  }
  let (images, videos) = dialog.takes_media();
  // SAFETY: on the main thread, with PhotosUI's classes and the selectors they answer.
  let picker = unsafe {
    let configuration: Retained<AnyObject> = msg_send![objc2::class!(PHPickerConfiguration), new];
    let limit: isize = if dialog.multiple { 0 } else { 1 };
    let _: () = msg_send![&*configuration, setSelectionLimit: limit];
    // The files as they are in the library, not made over into another format.
    let _: () = msg_send![&*configuration, setPreferredAssetRepresentationMode: 1isize];
    let filter_class = objc2::class!(PHPickerFilter);
    let filter: Retained<AnyObject> = match (images, videos) {
      (true, false) => msg_send![filter_class, imagesFilter],
      (false, true) => msg_send![filter_class, videosFilter],
      _ => {
        let both: [Retained<AnyObject>; 2] = [
          msg_send![filter_class, imagesFilter],
          msg_send![filter_class, videosFilter],
        ];
        let both = NSArray::from_retained_slice(&both);
        msg_send![filter_class, anyFilterMatchingSubfilters: &*both]
      }
    };
    let _: () = msg_send![&*configuration, setFilter: &*filter];
    let picker: Retained<AnyObject> =
      msg_send![msg_send![objc2::class!(PHPickerViewController), alloc], initWithConfiguration: &*configuration];
    picker
  };
  let delegate: Retained<AnyObject> = unsafe { msg_send![delegate_class(), new] };
  unsafe {
    let _: () = msg_send![&*picker, setDelegate: &*delegate];
  }
  #[cfg(target_os = "ios")]
  {
    let Some(over) = super::native::top_controller() else {
      answering.give(Vec::new());
      return;
    };
    unsafe {
      let _: () = msg_send![&*over, presentViewController: &*picker, animated: Bool::YES, completion: std::ptr::null::<std::ffi::c_void>()];
    }
    *open = Some(Open {
      answering,
      picker,
      _delegate: delegate,
    });
  }
  #[cfg(target_os = "macos")]
  {
    // SAFETY: on the main thread; the window in front is AppKit's, and the sheet is made of the picker.
    let sheet = unsafe {
      let app: Retained<AnyObject> = msg_send![objc2::class!(NSApplication), sharedApplication];
      let parent: Option<Retained<AnyObject>> = msg_send![&*app, keyWindow];
      let parent = match parent {
        Some(parent) => Some(parent),
        None => msg_send![&*app, mainWindow],
      };
      let Some(parent) = parent else {
        answering.give(Vec::new());
        return;
      };
      let sheet: Retained<AnyObject> = msg_send![objc2::class!(NSWindow), windowWithContentViewController: &*picker];
      let _: () = msg_send![&*parent, beginSheet: &*sheet, completionHandler: std::ptr::null::<std::ffi::c_void>()];
      (parent, sheet)
    };
    *open = Some(Open {
      answering,
      picker,
      _delegate: delegate,
      sheet: Some(sheet),
    });
  }
}

/// Puts the picker away.
fn dismiss(open: &Open) {
  #[cfg(target_os = "ios")]
  unsafe {
    let _: () = msg_send![&*open.picker, dismissViewControllerAnimated: Bool::YES, completion: std::ptr::null::<std::ffi::c_void>()];
  }
  #[cfg(target_os = "macos")]
  if let Some((parent, sheet)) = &open.sheet {
    unsafe {
      let _: () = msg_send![&**parent, endSheet: &**sheet];
    }
  }
  let _ = &open.picker;
}

/// The person chose (or chose nothing, which the picker says as an empty choice): each one chosen is copied out of
/// the library, and when all are, they are given.
extern "C-unwind" fn finished(_this: &AnyObject, _cmd: Sel, _picker: &AnyObject, results: &NSArray<AnyObject>) {
  let Some(open) = OPEN.lock().unwrap().take() else {
    return;
  };
  dismiss(&open);
  let results: Vec<Retained<AnyObject>> = results.iter().collect();
  if results.is_empty() {
    open.answering.give(Vec::new());
    return;
  }
  let gathered = Arc::new(Mutex::new(Gathered {
    left: results.len(),
    files: vec![None; results.len()],
    answering: Some(open.answering),
  }));
  for (at, result) in results.iter().enumerate() {
    copy_out(result, at, gathered.clone());
  }
}

/// What the copies of a choice came to, as they come: in the order they were chosen.
struct Gathered {
  left: usize,
  files: Vec<Option<DroppedFile>>,
  answering: Option<Answering>,
}

impl Gathered {
  fn came(&mut self, at: usize, file: Option<DroppedFile>) {
    self.files[at] = file;
    self.left -= 1;
    if self.left == 0
      && let Some(answering) = self.answering.take()
    {
      answering.give(self.files.drain(..).flatten().collect());
    }
  }
}

/// Copies the file of one result out of the library, into a folder of its own of the app's temporary folder.
fn copy_out(result: &AnyObject, at: usize, gathered: Arc<Mutex<Gathered>>) {
  // SAFETY: a result the picker gave: its item provider loads a file of a type it has, and calls the block once,
  // on a queue of its own, with the file's URL while it lasts.
  unsafe {
    let provider: Retained<AnyObject> = msg_send![result, itemProvider];
    let movie = NSString::from_str("public.movie");
    let is_movie: Bool = msg_send![&*provider, hasItemConformingToTypeIdentifier: &*movie];
    let kind = if is_movie.as_bool() {
      movie
    } else {
      NSString::from_str("public.image")
    };
    let suggested: Option<Retained<NSString>> = msg_send![&*provider, suggestedName];
    let suggested = suggested.map(|name| name.to_string());
    let block = RcBlock::new(move |url: *mut AnyObject, _error: *mut AnyObject| {
      let file = (!url.is_null())
        .then(|| {
          let path: Option<Retained<NSString>> = msg_send![&*url, path];
          path.map(|path| PathBuf::from(path.to_string()))
        })
        .flatten()
        .and_then(|from| kept(&from, suggested.as_deref()));
      gathered.lock().unwrap().came(at, file.map(DroppedFile::from_path));
    });
    let _: Option<Retained<AnyObject>> =
      msg_send![&*provider, loadFileRepresentationForTypeIdentifier: &*kind, completionHandler: &*block];
  }
}

/// The file at `from`, which lasts while its block runs, copied to a folder of its own: under the name the library
/// gives it, with the file's own extension.
fn kept(from: &std::path::Path, suggested: Option<&str>) -> Option<PathBuf> {
  static NEXT: AtomicU64 = AtomicU64::new(0);
  let folder = std::env::temp_dir()
    .join(format!("lurq-photos-{}", std::process::id()))
    .join(NEXT.fetch_add(1, Ordering::Relaxed).to_string());
  std::fs::create_dir_all(&folder).ok()?;
  let extension = from
    .extension()
    .map(|extension| extension.to_string_lossy().into_owned());
  let name = match (suggested, &extension) {
    (Some(name), Some(extension)) if !name.is_empty() => format!("{name}.{extension}"),
    _ => from.file_name()?.to_string_lossy().into_owned(),
  };
  let to = folder.join(name);
  std::fs::copy(from, &to).ok()?;
  Some(to)
}

/// The class of what hears the picker, registered once.
fn delegate_class() -> &'static AnyClass {
  if let Some(class) = AnyClass::get(c"LurqPhotoPickerDelegate") {
    return class;
  }
  let mut builder = ClassBuilder::new(c"LurqPhotoPickerDelegate", NSObject::class()).expect("a class name of its own");
  if let Some(protocol) = AnyProtocol::get(c"PHPickerViewControllerDelegate") {
    builder.add_protocol(protocol);
  }
  // SAFETY: the method has the selector's types: the picker and an array of its results.
  unsafe {
    builder.add_method(
      sel!(picker:didFinishPicking:),
      finished as extern "C-unwind" fn(_, _, _, _),
    );
  }
  builder.register()
}
