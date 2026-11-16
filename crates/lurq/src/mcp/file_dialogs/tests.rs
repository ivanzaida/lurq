use std::{
  collections::HashSet,
  path::PathBuf,
  sync::atomic::{AtomicUsize, Ordering},
  task::{Wake, Waker},
};

use super::*;
use crate::app::{App, Tree};

fn fixture() -> (FileDialogBroker, Arc<McpShared>, Tree) {
  let broker = FileDialogBroker::new();
  let shared = Arc::new(McpShared::new(
    HashSet::from([scope()]),
    HashSet::new(),
    "test".into(),
    "test".into(),
    None,
  ));
  broker.attach(&shared);
  *shared.file_dialogs.lock().unwrap() = Some(broker.clone());
  (broker, shared, Tree::new())
}

/// Held by a test that opens lurq's own dialog: the dialog asks whichever broker was registered last in the
/// process, so two such tests at once would find each other's requests in their brokers.
static OWN_DIALOG: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Whether `broker` is the one lurq's own dialog would ask now: the fixtures of the tests beside this one register
/// theirs in parallel.
fn is_asked(broker: &FileDialogBroker) -> bool {
  FileDialogBroker::registered().is_some_and(|asked| Arc::ptr_eq(&asked.0, &broker.0))
}

fn path() -> PathBuf {
  std::env::current_dir().unwrap().join("Unicode-α.txt")
}
fn selected() -> FileDialogSelection {
  FileDialogSelection {
    paths: vec![path()],
    overwrite: false,
  }
}
fn request(broker: &FileDialogBroker, tree: &Tree) -> FileDialogFuture {
  broker
    .request(
      &tree.window().handle(),
      FileDialogRequest::new(FileDialogOperation::OpenFile),
    )
    .unwrap()
    .unwrap()
}
fn complete(future: FileDialogFuture) -> Result<FileDialogSelection, FileDialogError> {
  tokio::runtime::Builder::new_current_thread()
    .build()
    .unwrap()
    .block_on(future)
}

#[test]
fn file_dialogs_unregistered_native_boundary_and_one_shot_windows() {
  let tree = Tree::new();
  assert!(
    FileDialogBroker::new()
      .request(
        &tree.window().handle(),
        FileDialogRequest::new(FileDialogOperation::OpenFile)
      )
      .unwrap()
      .is_none()
  );
  let (broker, _shared, first) = fixture();
  let second = Tree::new();
  let a = request(&broker, &first);
  let b = request(&broker, &second);
  assert_ne!(a.id, b.id);
  assert_eq!(
    broker.respond(&a.id, second.window(), FileDialogOperation::OpenFile, Some(selected())),
    Err(FileDialogError::WindowClosed)
  );
  assert_eq!(
    broker.respond(&a.id, first.window(), FileDialogOperation::SaveFile, Some(selected())),
    Err(FileDialogError::WrongOperation)
  );
  let id = a.id.clone();
  broker
    .respond(&id, first.window(), FileDialogOperation::OpenFile, Some(selected()))
    .unwrap();
  assert_eq!(complete(a), Ok(selected()));
  assert_eq!(
    broker.respond(&id, first.window(), FileDialogOperation::OpenFile, Some(selected())),
    Err(FileDialogError::StaleRequest)
  );
  broker
    .respond(&b.id, second.window(), FileDialogOperation::OpenFile, None)
    .unwrap();
  assert_eq!(complete(b), Err(FileDialogError::Cancelled));
}

#[test]
fn file_dialogs_future_drop_and_closed_or_replaced_window_retire_capacity() {
  let (broker, _shared, tree) = fixture();
  let pending: Vec<_> = (0..MAX_PENDING).map(|_| request(&broker, &tree)).collect();
  assert!(matches!(
    broker.request(
      &tree.window().handle(),
      FileDialogRequest::new(FileDialogOperation::OpenFile)
    ),
    Err(FileDialogError::TooManyPending)
  ));
  drop(pending);
  assert!(broker.0.lock().unwrap().pending.is_empty());
  let future = request(&broker, &tree);
  let replacement = Tree::new();
  broker.reconcile(&[("main".into(), replacement.window().clone())]);
  assert_eq!(complete(future), Err(FileDialogError::WindowClosed));
  let future = request(&broker, &tree);
  tree.window().handle().close();
  broker.reconcile(&[("main".into(), tree.window().clone())]);
  assert_eq!(complete(future), Err(FileDialogError::WindowClosed));
  assert!(broker.0.lock().unwrap().pending.is_empty());
}

#[test]
fn file_dialogs_vetoed_close_keeps_request_and_accepted_close_cancels() {
  let (broker, _shared, tree) = fixture();
  let future = request(&broker, &tree);
  tree.window().handle().on_close_requested(|request| request.cancel());
  tree.window().handle().request_close();
  assert!(tree.window().take_shell_commands().is_empty());
  broker.reconcile(&[("main".into(), tree.window().clone())]);
  assert_eq!(broker.0.lock().unwrap().pending.len(), 1);
  tree.window().handle().clear_close_requested_handler();
  tree.window().handle().request_close();
  tree.window().take_shell_commands();
  // Root's live-window cohort excludes the actually closed window.
  broker.reconcile(&[]);
  assert_eq!(complete(future), Err(FileDialogError::WindowClosed));
}

#[test]
fn file_dialogs_revocation_shutdown_and_retained_clone_never_fall_back() {
  for mode in 0..4 {
    let (broker, shared, tree) = fixture();
    let future = request(&broker, &tree);
    match mode {
      0 => shared.set_enabled(false),
      1 => shared.remove_scope(&scope()),
      2 => shared.deny_tool(RESPOND_TOOL),
      _ => broker.shutdown(),
    }
    assert_eq!(complete(future), Err(FileDialogError::Unavailable));
    assert!(matches!(
      broker.request(
        &tree.window().handle(),
        FileDialogRequest::new(FileDialogOperation::OpenFile)
      ),
      Err(FileDialogError::Unavailable)
    ));
    assert!(broker.0.lock().unwrap().pending.is_empty());
  }
}

#[test]
fn file_dialogs_accepted_close_is_terminal_after_command_drain_for_retained_handles() {
  for direct in [false, true] {
    let (broker, _shared, tree) = fixture();
    let handle = tree.window().handle();
    let future = request(&broker, &tree);
    if direct {
      handle.close();
    } else {
      handle.request_close();
    }
    assert_eq!(
      tree.window().take_shell_commands(),
      vec![crate::app::window::WindowCommand::Close]
    );
    assert!(matches!(
      broker.request(&handle, FileDialogRequest::new(FileDialogOperation::OpenFile)),
      Err(FileDialogError::WindowClosed)
    ));
    broker.reconcile(&[("main".into(), tree.window().clone())]);
    assert_eq!(complete(future), Err(FileDialogError::WindowClosed));
    let replacement = Tree::new();
    let fresh = request(&broker, &replacement);
    assert!(broker.0.lock().unwrap().pending.contains_key(&fresh.id));
    drop(fresh);
  }
}

#[test]
fn file_dialogs_paths_cardinality_and_bounded_metadata_are_checked_before_completion() {
  let (broker, _shared, tree) = fixture();
  let mut bad = FileDialogRequest::new(FileDialogOperation::OpenFile);
  bad.title = "x".repeat(4097);
  assert!(matches!(
    broker.request(&tree.window().handle(), bad),
    Err(FileDialogError::InvalidMetadata)
  ));
  let future = request(&broker, &tree);
  for paths in [vec![], vec!["relative".into()], vec![path(), path()]] {
    assert_eq!(
      broker.respond(
        &future.id,
        tree.window(),
        FileDialogOperation::OpenFile,
        Some(FileDialogSelection {
          paths,
          overwrite: false
        })
      ),
      Err(FileDialogError::InvalidSelection)
    );
  }
  broker
    .respond(
      &future.id,
      tree.window(),
      FileDialogOperation::OpenFile,
      Some(selected()),
    )
    .unwrap();
  assert_eq!(complete(future), Ok(selected()));
  let save = broker
    .request(
      &tree.window().handle(),
      FileDialogRequest::new(FileDialogOperation::SaveFile),
    )
    .unwrap()
    .unwrap();
  let selection = FileDialogSelection {
    paths: vec![path()],
    overwrite: true,
  };
  broker
    .respond(
      &save.id,
      tree.window(),
      FileDialogOperation::SaveFile,
      Some(selection.clone()),
    )
    .unwrap();
  assert_eq!(complete(save), Ok(selection));
}

struct Reenter {
  broker: FileDialogBroker,
  calls: AtomicUsize,
}
impl Wake for Reenter {
  fn wake(self: Arc<Self>) {
    self.wake_by_ref();
  }
  fn wake_by_ref(self: &Arc<Self>) {
    assert!(self.broker.0.try_lock().is_ok(), "wake must occur outside broker lock");
    self.broker.permission_changed();
    self.calls.fetch_add(1, Ordering::SeqCst);
  }
}

#[test]
fn file_dialogs_response_and_revocation_wake_outside_internal_locks() {
  for cancel in [false, true] {
    let (broker, shared, tree) = fixture();
    let mut future = request(&broker, &tree);
    let reenter = Arc::new(Reenter {
      broker: broker.clone(),
      calls: AtomicUsize::new(0),
    });
    let waker = Waker::from(reenter.clone());
    assert!(
      Pin::new(&mut future)
        .poll(&mut Context::from_waker(&waker))
        .is_pending()
    );
    if cancel {
      shared.set_enabled(false);
    } else {
      broker
        .respond(
          &future.id,
          tree.window(),
          FileDialogOperation::OpenFile,
          Some(selected()),
        )
        .unwrap();
    }
    assert_eq!(reenter.calls.load(Ordering::SeqCst), 1);
  }
}

#[test]
fn file_dialogs_opt_in_tools_and_actual_drain_cancel_without_later_mcp_call() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let broker = FileDialogBroker::new();
  let handle = tree.enable_mcp(
    super::super::McpConfig::new()
      .scope(scope())
      .file_dialogs(broker.clone()),
  );
  let future = request(&broker, &tree);
  tree.window().handle().close();
  tree.drain_mcp_requests(&mut app);
  assert_eq!(complete(future), Err(FileDialogError::WindowClosed));
  let retained = broker.clone();
  tree.shutdown_mcp();
  handle.set_enabled(true);
  assert!(matches!(
    retained.request(
      &Tree::new().window().handle(),
      FileDialogRequest::new(FileDialogOperation::OpenFile)
    ),
    Err(FileDialogError::Unavailable)
  ));
}

#[test]
fn lurqs_own_file_dialog_asks_the_registered_broker_and_never_opens_a_native_one() {
  // Brokers of other tests register in parallel; the dialog asks the last one, so this tries until the one it
  // made is the one asked.
  let _own = OWN_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  for _ in 0..100 {
    let (broker, _shared, tree) = fixture();
    if !is_asked(&broker) {
      continue;
    }
    let picked = crate::files::FileDialog::new()
      .title("Add files")
      .filter("Text", &["txt"])
      .multiple(true)
      .pick(&tree.window().handle());
    let pending: Vec<(String, FileDialogRequest)> = broker
      .0
      .lock()
      .unwrap()
      .pending
      .iter()
      .map(|(id, pending)| (id.clone(), pending.request.clone()))
      .collect();
    let Some((id, request)) = pending.into_iter().next() else {
      continue;
    };
    assert_eq!(
      request.operation,
      FileDialogOperation::OpenFiles,
      "several files: multiple"
    );
    assert_eq!(request.title, "Add files");
    assert_eq!(request.filters, vec![("Text".to_owned(), vec!["txt".to_owned()])]);
    broker
      .respond(&id, tree.window(), FileDialogOperation::OpenFiles, Some(selected()))
      .unwrap();
    let files = tokio::runtime::Builder::new_current_thread()
      .build()
      .unwrap()
      .block_on(picked);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name(), "Unicode-α.txt");
    assert_eq!(files[0].path(), Some(path().as_path()));
    return;
  }
  panic!("the dialog never asked the broker this test registered");
}

#[test]
fn lurqs_dialog_for_a_folder_and_for_where_to_save_ask_the_broker_as_what_they_are() {
  // As above: the dialog asks the last broker registered, so each is tried until the one made here is the one asked.
  let _own = OWN_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let asked = |operation: FileDialogOperation| -> bool {
    for _ in 0..100 {
      let (broker, _shared, tree) = fixture();
      if !is_asked(&broker) {
        continue;
      }
      let window = tree.window().handle();
      let dialog = crate::files::FileDialog::new().title("Choose").file_name("report.md");
      let picked: std::pin::Pin<Box<dyn std::future::Future<Output = Option<PathBuf>>>> = match operation {
        FileDialogOperation::OpenFolder => Box::pin(dialog.pick_folder(&window)),
        _ => Box::pin(dialog.pick_save(&window)),
      };
      let pending: Vec<(String, FileDialogRequest)> = broker
        .0
        .lock()
        .unwrap()
        .pending
        .iter()
        .map(|(id, pending)| (id.clone(), pending.request.clone()))
        .collect();
      let Some((id, request)) = pending.into_iter().next() else {
        continue;
      };
      assert_eq!(request.operation, operation);
      assert_eq!(request.title, "Choose");
      // The name a file starts with is said of a file to save, and of nothing else.
      assert_eq!(
        request.suggested_name.as_deref(),
        (operation == FileDialogOperation::SaveFile).then_some("report.md")
      );
      broker.respond(&id, tree.window(), operation, Some(selected())).unwrap();
      let chosen = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(picked);
      assert_eq!(chosen.as_deref(), Some(path().as_path()), "the path that was chosen");
      return true;
    }
    false
  };
  assert!(
    asked(FileDialogOperation::OpenFolder),
    "the dialog for a folder never asked the broker this test registered"
  );
  assert!(
    asked(FileDialogOperation::SaveFile),
    "the dialog for where to save never asked the broker this test registered"
  );
}
