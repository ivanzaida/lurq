//! Sequencing and ownership checks exercise the actual writer with fake APIs.

use std::{cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

use super::{Platform, Session, Writer};

#[derive(Clone, Copy, Default)]
enum Failure {
  #[default]
  None,
  Prepare,
  Owner,
  Open,
  Empty,
  Put,
  Close,
  Panic,
}

#[derive(Default)]
struct State {
  failure: Failure,
  calls: Vec<&'static str>,
  frees: usize,
  transfers: usize,
  owners: usize,
  destroyed: usize,
  open: bool,
  words: Vec<Vec<u16>>,
}

type Shared = Rc<RefCell<State>>;

struct Fake(Shared);
struct Owner(Shared);
struct Data(Shared, bool);
struct Open(Shared, bool);

impl Platform for Fake {
  type Owner = Owner;
  type Data = Data;
  type Session = Open;

  fn prepare(&self, text: &str) -> Option<Data> {
    let mut state = self.0.borrow_mut();
    state.calls.push("prepare");
    if matches!(state.failure, Failure::Prepare) {
      return None;
    }
    state
      .words
      .push(text.encode_utf16().chain(std::iter::once(0)).collect());
    Some(Data(self.0.clone(), false))
  }

  fn owner(&self) -> Option<Owner> {
    let mut state = self.0.borrow_mut();
    state.calls.push("owner");
    if matches!(state.failure, Failure::Owner) {
      return None;
    }
    state.owners += 1;
    Some(Owner(self.0.clone()))
  }

  fn open(&self, owner: &Owner) -> Option<Open> {
    assert!(Rc::ptr_eq(&self.0, &owner.0));
    let mut state = self.0.borrow_mut();
    state.calls.push("open");
    assert!(!state.open);
    if matches!(state.failure, Failure::Open) {
      return None;
    }
    assert_eq!(state.owners, 1);
    assert_eq!(state.destroyed, 0);
    state.open = true;
    Some(Open(self.0.clone(), true))
  }
}

impl Session for Open {
  type Data = Data;

  fn empty(&mut self) -> bool {
    let mut state = self.0.borrow_mut();
    state.calls.push("empty");
    assert!(state.open);
    !matches!(state.failure, Failure::Empty)
  }

  fn put(&mut self, mut data: Data) -> bool {
    let mut state = self.0.borrow_mut();
    state.calls.push("put");
    assert!(state.open);
    assert!(Rc::ptr_eq(&self.0, &data.0));
    let failure = state.failure;
    drop(state); // A failed transfer/unwind must allow Data's cleanup.
    assert!(!matches!(failure, Failure::Panic), "synthetic transfer panic");
    if matches!(failure, Failure::Put) {
      return false;
    }
    data.1 = true;
    self.0.borrow_mut().transfers += 1;
    true
  }

  fn close(&mut self) -> bool {
    assert!(self.1, "exactly one close");
    self.1 = false;
    let mut state = self.0.borrow_mut();
    state.calls.push("close");
    state.open = false;
    !matches!(state.failure, Failure::Close)
  }
}

impl Drop for Open {
  fn drop(&mut self) {
    if self.1 {
      self.close();
    }
  }
}

impl Drop for Data {
  fn drop(&mut self) {
    if !self.1 {
      self.0.borrow_mut().frees += 1;
    }
  }
}

impl Drop for Owner {
  fn drop(&mut self) {
    let mut state = self.0.borrow_mut();
    assert!(!state.open);
    state.destroyed += 1;
  }
}

fn fixture(failure: Failure) -> (Writer<Fake>, Shared) {
  let shared = Rc::new(RefCell::new(State {
    failure,
    ..State::default()
  }));
  (Writer::new(Fake(shared.clone())), shared)
}

#[test]
fn windows_clipboard_owner_survives_two_returns_and_transfers_are_not_freed() {
  let (mut writer, shared) = fixture(Failure::None);
  assert!(writer.write("A😀"));
  assert_eq!(shared.borrow().destroyed, 0);
  assert!(writer.write(""));
  let state = shared.borrow();
  assert_eq!(state.owners, 1);
  assert_eq!(state.transfers, 2);
  assert_eq!(state.frees, 0);
  assert_eq!(state.words, [vec![0x41, 0xd83d, 0xde00, 0], vec![0]]);
  assert_eq!(
    state.calls,
    [
      "prepare", "owner", "open", "empty", "put", "close", "prepare", "open", "empty", "put", "close"
    ]
  );
  drop(state);
  drop(writer);
  assert_eq!(shared.borrow().destroyed, 1);
}

#[test]
fn windows_clipboard_prepare_owner_or_open_failure_never_empties() {
  for failure in [Failure::Prepare, Failure::Owner, Failure::Open] {
    let (mut writer, shared) = fixture(failure);
    assert!(!writer.write("synthetic"));
    let state = shared.borrow();
    assert!(!state.calls.contains(&"empty"));
    assert!(!state.calls.contains(&"put"));
    assert_eq!(state.transfers, 0);
    assert_eq!(state.frees, usize::from(!matches!(failure, Failure::Prepare)));
  }
}

#[test]
fn windows_clipboard_empty_failure_closes_and_frees_without_transfer() {
  let (mut writer, shared) = fixture(Failure::Empty);
  assert!(!writer.write("synthetic"));
  let state = shared.borrow();
  assert_eq!(state.calls, ["prepare", "owner", "open", "empty", "close"]);
  assert_eq!((state.frees, state.transfers), (1, 0));
  assert!(!state.open);
}

#[test]
fn windows_clipboard_transfer_failure_frees_and_closes_once() {
  let (mut writer, shared) = fixture(Failure::Put);
  assert!(!writer.write("synthetic"));
  let state = shared.borrow();
  assert_eq!(state.calls.iter().filter(|call| **call == "put").count(), 1);
  assert_eq!(state.calls.iter().filter(|call| **call == "close").count(), 1);
  assert_eq!((state.frees, state.transfers), (1, 0));
  assert!(!state.open);
}

#[test]
fn windows_clipboard_close_failure_is_not_success_or_double_free() {
  let (mut writer, shared) = fixture(Failure::Close);
  assert!(!writer.write("synthetic"));
  let state = shared.borrow();
  assert_eq!((state.frees, state.transfers), (0, 1));
  assert_eq!(state.calls.iter().filter(|call| **call == "close").count(), 1);
}

#[test]
fn windows_clipboard_unwind_closes_before_owner_and_frees_untransferred_data() {
  let (mut writer, shared) = fixture(Failure::Panic);
  assert!(std::panic::catch_unwind(AssertUnwindSafe(|| writer.write("synthetic"))).is_err());
  assert!(!shared.borrow().open);
  assert_eq!((shared.borrow().frees, shared.borrow().transfers), (1, 0));
  assert_eq!(shared.borrow().calls.iter().filter(|call| **call == "close").count(), 1);
  assert_eq!(shared.borrow().destroyed, 0);
  drop(writer);
  assert_eq!(shared.borrow().destroyed, 1);
}
