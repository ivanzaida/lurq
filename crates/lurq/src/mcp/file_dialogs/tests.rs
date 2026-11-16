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
  // As `enable_mcp` does: the tree's window asks this broker, and no other test's.
  let tree = Tree::new();
  tree.window().serve_file_dialogs(&broker.serving());
  (broker, shared, tree)
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

/// The one request `broker` holds: its id and what was asked.
fn asked(broker: &FileDialogBroker) -> (String, FileDialogRequest) {
  let pending: Vec<(String, FileDialogRequest)> = broker
    .0
    .lock()
    .unwrap()
    .pending
    .iter()
    .map(|(id, pending)| (id.clone(), pending.request.clone()))
    .collect();
  assert_eq!(
    pending.len(),
    1,
    "the dialog asked the broker of its window's tree, once"
  );
  pending.into_iter().next().unwrap()
}

fn wait<T>(future: impl std::future::Future<Output = T>) -> T {
  tokio::runtime::Builder::new_current_thread()
    .build()
    .unwrap()
    .block_on(future)
}

#[test]
fn lurqs_own_file_dialog_asks_the_broker_of_its_windows_tree_and_never_opens_a_native_one() {
  let (broker, _shared, tree) = fixture();
  let picked = crate::files::FileDialog::new()
    .title("Add files")
    .filter("Text", &["txt"])
    .multiple(true)
    .pick(&tree.window().handle());
  let (id, request) = asked(&broker);
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
  let files = wait(picked);
  assert_eq!(files.len(), 1);
  assert_eq!(files[0].name(), "Unicode-α.txt");
  assert_eq!(files[0].path(), Some(path().as_path()));
}

#[test]
fn lurqs_dialog_for_a_folder_and_for_where_to_save_ask_the_broker_as_what_they_are() {
  for operation in [FileDialogOperation::OpenFolder, FileDialogOperation::SaveFile] {
    let (broker, _shared, tree) = fixture();
    let window = tree.window().handle();
    let dialog = crate::files::FileDialog::new().title("Choose").file_name("report.md");
    let picked: std::pin::Pin<Box<dyn std::future::Future<Output = Option<PathBuf>>>> = match operation {
      FileDialogOperation::OpenFolder => Box::pin(dialog.pick_folder(&window)),
      _ => Box::pin(dialog.pick_save(&window)),
    };
    let (id, request) = asked(&broker);
    assert_eq!(request.operation, operation);
    assert_eq!(request.title, "Choose");
    // The name a file starts with is said of a file to save, and of nothing else.
    assert_eq!(
      request.suggested_name.as_deref(),
      (operation == FileDialogOperation::SaveFile).then_some("report.md")
    );
    broker.respond(&id, tree.window(), operation, Some(selected())).unwrap();
    assert_eq!(
      wait(picked).as_deref(),
      Some(path().as_path()),
      "the path that was chosen"
    );
  }
}

#[test]
fn a_dialog_over_a_window_of_another_tree_is_not_this_brokers() {
  // Two trees in one process, each with a broker: a dialog over a window asks its own tree's, whichever was
  // registered last, and a tree with no broker asks none.
  let (first, _first_shared, first_tree) = fixture();
  let (second, _second_shared, second_tree) = fixture();
  let _picked = crate::files::FileDialog::new()
    .title("The first tree's")
    .pick(&first_tree.window().handle());
  assert_eq!(asked(&first).1.title, "The first tree's");
  assert!(
    second.0.lock().unwrap().pending.is_empty(),
    "the other tree's broker heard nothing"
  );
  let _picked = crate::files::FileDialog::new()
    .title("The second tree's")
    .pick_folder(&second_tree.window().handle());
  assert_eq!(asked(&second).1.title, "The second tree's");
  assert_eq!(first.0.lock().unwrap().pending.len(), 1);

  // Without the system's dialog built in, a tree with no broker has none to open, and asks neither of these.
  #[cfg(not(feature = "file_dialog"))]
  {
    let alone = Tree::new();
    let _picked = crate::files::FileDialog::new().pick(&alone.window().handle());
    assert_eq!(
      first.0.lock().unwrap().pending.len(),
      1,
      "a tree with no broker asks none"
    );
    assert_eq!(second.0.lock().unwrap().pending.len(), 1);
  }
}
