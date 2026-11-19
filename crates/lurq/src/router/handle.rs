use std::{any::Any, sync::Arc};

use parking_lot::Mutex;

use super::{Routes, guard::GuardAction, route_match::Params};
use crate::core::signal::Signal;

#[derive(Clone)]
pub struct RouterHandle {
  pub(crate) inner: Arc<RouterInner>,
}

pub(crate) struct RouterInner {
  pub(crate) routes: Routes,
  pub(crate) current_path: Signal<String>,
  pub(crate) history: Mutex<HistoryStack>,
  /// Whether the page's address follows this router; see [`RouterHandle::follow_address`].
  #[cfg(target_arch = "wasm32")]
  pub(crate) follows_address: std::sync::atomic::AtomicBool,
  /// The window the router is mounted in, which a browser's back and forward are run in.
  #[cfg(target_arch = "wasm32")]
  pub(crate) window: Mutex<Option<crate::app::window::Window>>,
  _watch_handle: Option<Box<dyn std::any::Any + Send + Sync>>,
}

/// What a navigation did to the history, for the page's address to do the same.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryChange {
  /// A new entry after the current one.
  Pushed,
  /// The current entry is another.
  Replaced,
  /// The history moved by that many entries, back when negative.
  Moved(i32),
}

struct HistoryEntry {
  /// Its own among every entry made, and kept as the history moves back and forward over it: what a stack of
  /// screens tells its screens apart by.
  id: u64,
  path: String,
  state: Option<Arc<dyn Any + Send + Sync>>,
}

impl HistoryEntry {
  fn new(path: String, state: Option<Arc<dyn Any + Send + Sync>>) -> Self {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    Self {
      id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
      path,
      state,
    }
  }
}

/// An entry of the history up to where it now is: a screen of a stack.
#[derive(Clone)]
pub(crate) struct StackEntry {
  pub(crate) id: u64,
  pub(crate) path: String,
  pub(crate) state: Option<Arc<dyn Any + Send + Sync>>,
}

#[derive(Default)]
pub(crate) struct HistoryStack {
  entries: Vec<HistoryEntry>,
  cursor: usize,
}

impl std::fmt::Debug for RouterHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("RouterHandle").field(&Arc::as_ptr(&self.inner)).finish()
  }
}

impl PartialEq for RouterHandle {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl RouterHandle {
  /// A [`super::Navigator`] for programmatic navigation over this router —
  /// the same handle components get from `ctx.navigator()`.
  pub fn navigator(&self) -> super::Navigator {
    super::Navigator { handle: self.clone() }
  }

  /// A router over `routes`, at no path yet: `replace` gives it its first. What [`Ctx::router`] makes in a render;
  /// this makes one anywhere, for a place that is added while the app runs, such as a tab a person opens.
  ///
  /// [`Ctx::router`]: crate::app::ctx::Ctx::router
  pub fn new(routes: Routes) -> Self {
    Self::new_with_signal(routes, Signal::new(String::new()))
  }

  pub(crate) fn new_with_signal(routes: Routes, current_path: Signal<String>) -> Self {
    Self {
      inner: Arc::new(RouterInner {
        routes,
        current_path,
        history: Mutex::new(HistoryStack::default()),
        #[cfg(target_arch = "wasm32")]
        follows_address: std::sync::atomic::AtomicBool::new(false),
        #[cfg(target_arch = "wasm32")]
        window: Mutex::new(None),
        _watch_handle: None,
      }),
    }
  }

  pub fn push(&self, path: impl Into<String>) {
    self.push_inner(path.into(), None);
  }

  pub fn push_with_state<S: Any + Send + Sync>(&self, path: impl Into<String>, state: S) {
    let state: Arc<dyn Any + Send + Sync> = Arc::new(state);
    self.push_inner(path.into(), Some(state));
  }

  fn push_inner(&self, path: String, state: Option<Arc<dyn Any + Send + Sync>>) {
    if self.inner.current_path.get_untracked() == path {
      return;
    }

    if let Some(redirect) = self.check_guards(&path) {
      if redirect == self.inner.current_path.get_untracked() {
        return;
      }
      self.navigate_internal(redirect, None);
      return;
    }

    self.navigate_internal(path, state);
  }

  fn navigate_internal(&self, path: String, state: Option<Arc<dyn Any + Send + Sync>>) {
    {
      let mut history = self.inner.history.lock();
      let cursor = history.cursor;
      if !history.entries.is_empty() {
        history.entries.truncate(cursor + 1);
      }
      history.entries.push(HistoryEntry::new(path.clone(), state));
      history.cursor = history.entries.len() - 1;
    }
    tracing::debug!(
      target: "lurq::router",
      "[lurq/router] navigate set path={}",
      path
    );
    self.inner.current_path.set(path);
    self.tell_address(HistoryChange::Pushed);
  }

  pub fn replace(&self, path: impl Into<String>) {
    self.replace_inner(path.into(), None);
  }

  pub fn replace_with_state<S: Any + Send + Sync>(&self, path: impl Into<String>, state: S) {
    let state: Arc<dyn Any + Send + Sync> = Arc::new(state);
    self.replace_inner(path.into(), Some(state));
  }

  fn replace_inner(&self, path: String, state: Option<Arc<dyn Any + Send + Sync>>) {
    if let Some(redirect) = self.check_guards(&path) {
      if redirect == self.inner.current_path.get_untracked() {
        return;
      }
      self.replace_internal(redirect, None);
      return;
    }

    self.replace_internal(path, state);
  }

  fn replace_internal(&self, path: String, state: Option<Arc<dyn Any + Send + Sync>>) {
    {
      let mut history = self.inner.history.lock();
      if history.entries.is_empty() {
        history.entries.push(HistoryEntry::new(path.clone(), state));
        history.cursor = 0;
      } else {
        let cursor = history.cursor;
        history.entries[cursor] = HistoryEntry::new(path.clone(), state);
      }
    }
    tracing::debug!(
      target: "lurq::router",
      "[lurq/router] replace set path={}",
      path
    );
    self.inner.current_path.set(path);
    self.tell_address(HistoryChange::Replaced);
  }

  /// In-memory state attached to the current history entry, if any.
  pub(crate) fn current_state(&self) -> Option<Arc<dyn Any + Send + Sync>> {
    let history = self.inner.history.lock();
    history
      .entries
      .get(history.cursor)
      .and_then(|entry| entry.state.clone())
  }

  /// The history from its first entry to the current one, the current last. A path set on the router's signal
  /// by other means than this handle is not an entry.
  pub(crate) fn stack(&self) -> Vec<StackEntry> {
    let history = self.inner.history.lock();
    if history.entries.is_empty() {
      // Nothing was pushed yet: the router shows the path it started with.
      return vec![StackEntry {
        id: 0,
        path: self.inner.current_path.get_untracked(),
        state: None,
      }];
    }
    history
      .entries
      .iter()
      .take(history.cursor + 1)
      .map(|entry| StackEntry {
        id: entry.id,
        path: entry.path.clone(),
        state: entry.state.clone(),
      })
      .collect()
  }

  /// Goes back one entry, where there is one before: whether it moved. This moves the router alone; going back as a
  /// person does, through what the page does with it first (a dialog that closes on it), is
  /// [`Ctx::history_back`](crate::app::ctx::Ctx::history_back), which ends here where nothing else takes it.
  pub fn back(&self) -> bool {
    self.step(-1, true)
  }

  /// Moves the history by one entry, back where `delta` is -1 and forward where it is 1, telling the page's address
  /// where `tell`: whether it moved.
  fn step(&self, delta: i32, tell: bool) -> bool {
    let new_path = {
      let mut history = self.inner.history.lock();
      let Some(cursor) = history.cursor.checked_add_signed(delta as isize) else {
        return false;
      };
      if cursor >= history.entries.len() {
        return false;
      }
      history.cursor = cursor;
      history.entries[cursor].path.clone()
    };
    self.inner.current_path.set(new_path);
    if tell {
      self.tell_address(HistoryChange::Moved(delta));
    }
    true
  }

  /// Goes back or forward by a command lurq runs on the window the router is mounted in, which reached the router:
  /// whether it moved. A browser's back and forward have moved the page's address already: the router that follows
  /// it moves without telling it again.
  pub(crate) fn run_command(
    &self,
    command: crate::app::events::NavigationCommand,
    source: crate::app::events::NavigationSource,
  ) -> bool {
    let delta = match command {
      crate::app::events::NavigationCommand::Back => -1,
      crate::app::events::NavigationCommand::Forward => 1,
    };
    let tell = !(source == crate::app::events::NavigationSource::Browser && self.follows_address_now());
    self.step(delta, tell)
  }

  /// Whether the page's address follows this router.
  fn follows_address_now(&self) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
      self.inner.follows_address.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      false
    }
  }

  /// The command that moves the history from where it is to its entry `id`, where that is the entry just before or
  /// just after: what a browser's back or forward did when its `popstate` names it.
  #[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
  pub(crate) fn command_to(&self, id: u64) -> Option<crate::app::events::NavigationCommand> {
    let history = self.inner.history.lock();
    let index = history.entries.iter().position(|entry| entry.id == id)?;
    if index + 1 == history.cursor {
      Some(crate::app::events::NavigationCommand::Back)
    } else if index == history.cursor + 1 {
      Some(crate::app::events::NavigationCommand::Forward)
    } else {
      None
    }
  }

  /// Says which window the router is mounted in, for a browser's back and forward to be run in.
  #[cfg(target_arch = "wasm32")]
  pub(crate) fn set_window(&self, window: Option<crate::app::window::Window>) {
    let mut held = self.inner.window.lock();
    if held.is_none() || window.is_some() {
      *held = window;
    }
  }

  /// The window the router is mounted in, where it was rendered in one.
  #[cfg(target_arch = "wasm32")]
  pub(crate) fn window(&self) -> Option<crate::app::window::Window> {
    self.inner.window.lock().clone()
  }

  /// Goes back several entries at once, as far as there are entries: the screens between are passed over
  /// and not shown. Tells whether it moved.
  pub fn back_by(&self, steps: usize) -> bool {
    let new_path = {
      let mut history = self.inner.history.lock();
      let steps = steps.min(history.cursor);
      if steps == 0 {
        return false;
      }
      history.cursor -= steps;
      history.entries[history.cursor].path.clone()
    };
    self.inner.current_path.set(new_path);
    self.tell_address(HistoryChange::Moved(-(steps as i32)));
    true
  }

  /// Goes forward one entry, where there is one after: whether it moved. As [`back`](Self::back), the router alone;
  /// [`Ctx::history_forward`](crate::app::ctx::Ctx::history_forward) goes forward as a person does.
  pub fn forward(&self) -> bool {
    self.step(1, true)
  }

  /// In a browser, ties the router to the page's address, so that the two say the same from then on:
  ///
  /// - the address has the router's path as its fragment (`index.html#/settings`), and each `push` is an entry of the
  ///   browser's history as it is one of the router's, each `replace` the same entry with another path;
  /// - the browser's back and forward buttons go back and forward as Alt+← and the mouse's side buttons do (a dialog
  ///   that closes on going back closes, and the browser is put back where the router is), and `back` and `forward`
  ///   move the browser;
  /// - an address a person types or follows is navigated to, through the routes' guards as a `push` is;
  /// - a page that is loaded with a path in its address starts there, in place of the path the router was given, so a
  ///   reload stays where it was and a link opens what it names.
  ///
  /// Call it once the router has its first path. Only one router of an app follows the address: the one whose
  /// screens the address should name. Outside a browser a window has no address, and this does nothing.
  pub fn follow_address(&self) {
    #[cfg(target_arch = "wasm32")]
    super::address::follow(self);
  }

  /// The id of the entry the history is at; 0 before the history has one.
  #[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
  pub(crate) fn current_entry(&self) -> u64 {
    let history = self.inner.history.lock();
    history.entries.get(history.cursor).map_or(0, |entry| entry.id)
  }

  /// Moves the history to its entry `id`, wherever from, as going back or forward by as many entries does,
  /// without telling the address, which is where the move came from. `false` when the history has no such
  /// entry.
  #[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
  pub(crate) fn go_to_entry(&self, id: u64) -> bool {
    let new_path = {
      let mut history = self.inner.history.lock();
      let Some(index) = history.entries.iter().position(|entry| entry.id == id) else {
        return false;
      };
      if index == history.cursor {
        return true;
      }
      history.cursor = index;
      history.entries[index].path.clone()
    };
    self.inner.current_path.set(new_path);
    true
  }

  /// Forgets the history and starts it again at `path`, or at where the routes' guards send it: for an entry
  /// of the browser's history that is none of this router's, such as one from before the page was loaded.
  #[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
  pub(crate) fn restart_at(&self, path: String) {
    let path = self.check_guards(&path).unwrap_or(path);
    {
      let mut history = self.inner.history.lock();
      history.entries.clear();
      history.entries.push(HistoryEntry::new(path.clone(), None));
      history.cursor = 0;
    }
    if self.inner.current_path.get_untracked() != path {
      self.inner.current_path.set(path);
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn tell_address(&self, change: HistoryChange) {
    if self.inner.follows_address.load(std::sync::atomic::Ordering::Relaxed) {
      super::address::record(self, change);
    }
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn tell_address(&self, _change: HistoryChange) {}

  /// Whether [`RouterHandle::back`] would move — there is a prior entry.
  pub fn can_back(&self) -> bool {
    self.inner.history.lock().cursor > 0
  }

  /// Whether [`RouterHandle::forward`] would move — there is a later entry.
  pub fn can_forward(&self) -> bool {
    let history = self.inner.history.lock();
    history.cursor + 1 < history.entries.len()
  }

  pub fn path(&self) -> Signal<String> {
    self.inner.current_path.clone()
  }

  pub fn params(&self) -> Params {
    let path = self.inner.current_path.get();
    let matches = self.inner.routes.resolve(&path);
    matches.last().map(|m| m.params.clone()).unwrap_or_default()
  }

  fn check_guards(&self, path: &str) -> Option<String> {
    let matches = self.inner.routes.resolve(path);
    for m in &matches {
      if let Some(guard) = self.inner.routes.guard_for(m.route_index) {
        match guard(m) {
          GuardAction::Allow => {}
          GuardAction::Deny => return Some(self.inner.current_path.get_untracked()),
          GuardAction::Redirect(target) => return Some(target),
        }
      }
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn router() -> RouterHandle {
    let router = RouterHandle::new_with_signal(Routes::new(), Signal::new(String::new()));
    router.replace("/");
    router
  }

  #[test]
  fn the_history_goes_to_an_entry_by_its_id() {
    let router = router();
    let first = router.current_entry();
    router.push("/a");
    let second = router.current_entry();
    router.push("/b");
    assert_ne!(first, second);

    assert!(router.go_to_entry(first));
    assert_eq!(router.path().get_untracked(), "/");
    assert!(!router.can_back() && router.can_forward());

    assert!(router.go_to_entry(second));
    assert_eq!(router.path().get_untracked(), "/a");
    assert!(router.can_back() && router.can_forward());

    // The entry it is at: there, and nothing moves.
    assert!(router.go_to_entry(second));
    assert_eq!(router.current_entry(), second);

    assert!(!router.go_to_entry(u64::MAX), "no such entry");
    assert_eq!(router.path().get_untracked(), "/a");
  }

  #[test]
  fn a_browsers_move_to_the_entry_beside_is_going_back_or_forward() {
    use crate::app::events::{NavigationCommand, NavigationSource};
    let router = RouterHandle::new(Routes::new());
    router.replace("/a");
    let first = router.current_entry();
    router.push("/b");
    let second = router.current_entry();
    router.push("/c");
    let third = router.current_entry();
    assert_eq!(router.command_to(second), Some(NavigationCommand::Back));
    assert_eq!(
      router.command_to(first),
      None,
      "two back is no command: the router goes there"
    );
    assert_eq!(router.command_to(third), None, "where it is");
    assert!(router.back());
    assert_eq!(router.command_to(third), Some(NavigationCommand::Forward));

    // The command, as lurq runs it on the router mounted in a window.
    assert!(router.run_command(NavigationCommand::Back, NavigationSource::Browser));
    assert_eq!(router.current_entry(), first);
    assert!(
      !router.run_command(NavigationCommand::Back, NavigationSource::Key),
      "nowhere to go"
    );
    assert!(router.run_command(NavigationCommand::Forward, NavigationSource::Mouse));
    assert_eq!(router.path().get_untracked(), "/b");
  }

  #[test]
  fn a_history_that_starts_again_has_one_entry() {
    let router = router();
    router.push("/a");
    router.push("/b");
    let before = router.current_entry();

    router.restart_at("/c".to_owned());
    assert_eq!(router.path().get_untracked(), "/c");
    assert!(!router.can_back() && !router.can_forward());
    assert_ne!(router.current_entry(), before);
    assert!(!router.go_to_entry(before), "the entries of the old history are gone");
  }

  #[test]
  fn following_the_address_changes_nothing_outside_a_browser() {
    let router = router();
    router.follow_address();
    router.push("/a");
    assert!(router.back());
    assert_eq!(router.path().get_untracked(), "/");
  }
}
