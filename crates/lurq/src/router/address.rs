//! A router that the page's address follows; see [`RouterHandle::follow_address`].
//!
//! The router's path is the address's fragment (`index.html#/settings`), which a page changes without the
//! browser asking the server for anything. The router's history and the browser's are kept entry for entry:
//!
//! - what the router does to its history it does to the browser's ([`record`]): a push is `pushState`, a replace
//!   `replaceState`, a move back or forward `history.go`;
//! - what the browser does to its history reaches the router as a `popstate` event ([`arrived`]): its back and forward
//!   buttons, and an address a person types or follows. A move to the entry just before or after the router's is going
//!   back or forward ([`NavigationCommand`](crate::app::events::NavigationCommand)), run on the window the router is
//!   mounted in as Alt+← is: where something on the page takes it (a dialog closes) and the router does not move, the
//!   browser is put back where the router is, which brings a `popstate` for the entry the router is at.
//!
//! Every entry the router makes carries its [`token`] as the browser's state for it, so a `popstate` says which
//! entry of the router the browser is now at. An entry without one is new to the router: an address typed into
//! the bar, which is navigated to. One with the token of another load of the page is from before a reload,
//! when the router had another history: the router's starts again there.

#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, sync::Arc};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[cfg(target_arch = "wasm32")]
use super::handle::{HistoryChange, RouterHandle};

const TOKEN: &str = "lurq-router";

/// What the browser keeps with an entry of its history: which load of the page made it, and which entry of the
/// router's it is.
fn token(load: u32, entry: u64) -> String {
  format!("{TOKEN}:{load}:{entry}")
}

/// The router's entry a browser's state names, when this load of the page made it.
fn entry_of(state: &str, load: u32) -> Option<u64> {
  let (made_by, entry) = state.strip_prefix(TOKEN)?.strip_prefix(':')?.split_once(':')?;
  (made_by.parse::<u32>().ok()? == load).then(|| entry.parse().ok())?
}

/// The router's path in an address's fragment (`#/settings?tab=2`), when it has one: a fragment that is not a
/// path, such as an anchor of the page, is none.
fn path_of(fragment: &str) -> Option<&str> {
  let path = fragment.strip_prefix('#').unwrap_or(fragment);
  path.starts_with('/').then_some(path)
}

#[cfg(target_arch = "wasm32")]
thread_local! {
  /// This load of the page, to tell its entries from those of the loads before it.
  static LOAD: u32 = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
  /// The router is being moved by the address: what it does is not told back to the address.
  static FROM_ADDRESS: Cell<bool> = const { Cell::new(false) };
}

#[cfg(target_arch = "wasm32")]
fn load() -> u32 {
  LOAD.with(|load| *load)
}

/// Runs `navigate` on the router without [`record`] telling the address, which is where it comes from.
#[cfg(target_arch = "wasm32")]
fn from_address(navigate: impl FnOnce()) {
  FROM_ADDRESS.with(|flag| flag.set(true));
  navigate();
  FROM_ADDRESS.with(|flag| flag.set(false));
}

/// The path in the page's address, decoded.
#[cfg(target_arch = "wasm32")]
fn address_path(window: &web_sys::Window) -> Option<String> {
  let fragment = window.location().hash().ok()?;
  let decoded = js_sys::decode_uri(&fragment).ok().map(String::from);
  path_of(decoded.as_deref().unwrap_or(&fragment)).map(str::to_owned)
}

/// The address of the page with the router's path as its fragment; the page's own address for no path.
#[cfg(target_arch = "wasm32")]
fn address_of(path: &str) -> Option<String> {
  if path.is_empty() {
    return None;
  }
  let encoded = js_sys::encode_uri(path);
  Some(format!("#{}", String::from(encoded)))
}

/// Makes the browser's current entry the router's current one: its token, and its path in the address.
#[cfg(target_arch = "wasm32")]
fn mark_current(window: &web_sys::Window, handle: &RouterHandle) {
  let Ok(history) = window.history() else {
    return;
  };
  let state = JsValue::from_str(&token(load(), handle.current_entry()));
  let address = address_of(&handle.inner.current_path.get_untracked());
  let _ = history.replace_state_with_url(&state, "", address.as_deref());
}

/// Starts following: the router goes where the address already is, the address says where the router is, and
/// the browser's moves are listened for.
#[cfg(target_arch = "wasm32")]
pub(super) fn follow(handle: &RouterHandle) {
  use std::sync::atomic::Ordering;

  if handle.inner.follows_address.swap(true, Ordering::Relaxed) {
    return;
  }
  let Some(window) = web_sys::window() else {
    return;
  };
  // A page loaded with a path in its address starts there. Otherwise the router's own path is the first
  // entry, which its history may not have yet.
  let path = address_path(&window).unwrap_or_else(|| handle.inner.current_path.get_untracked());
  from_address(|| handle.replace(path));
  mark_current(&window, handle);

  let router = Arc::downgrade(&handle.inner);
  let listener = Closure::<dyn FnMut(web_sys::PopStateEvent)>::new(move |event: web_sys::PopStateEvent| {
    if let Some(inner) = router.upgrade() {
      arrived(&RouterHandle { inner }, &event.state());
    }
  });
  let _ = window.add_event_listener_with_callback("popstate", listener.as_ref().unchecked_ref());
  // The listener lives as long as the page; a router that is gone is not found by it.
  listener.forget();
}

/// The router changed its history: the browser's is changed the same way.
#[cfg(target_arch = "wasm32")]
pub(super) fn record(handle: &RouterHandle, change: HistoryChange) {
  if FROM_ADDRESS.with(Cell::get) {
    return;
  }
  let Some(history) = web_sys::window().and_then(|window| window.history().ok()) else {
    return;
  };
  let state = JsValue::from_str(&token(load(), handle.current_entry()));
  let address = address_of(&handle.inner.current_path.get_untracked());
  let _ = match change {
    HistoryChange::Pushed => history.push_state_with_url(&state, "", address.as_deref()),
    HistoryChange::Replaced => history.replace_state_with_url(&state, "", address.as_deref()),
    // The browser answers with a `popstate` for the entry the router is already at.
    HistoryChange::Moved(entries) => history.go_with_delta(entries),
  };
}

/// The browser moved in its history, or was given an address: the router goes there.
#[cfg(target_arch = "wasm32")]
fn arrived(handle: &RouterHandle, state: &JsValue) {
  let Some(window) = web_sys::window() else {
    return;
  };
  let entry = state.as_string().and_then(|state| entry_of(&state, load()));
  if let Some(entry) = entry
    && let Some(command) = handle.command_to(entry)
    && let Some(mounted) = handle.window()
  {
    let router = handle.clone();
    let delta = match command {
      crate::app::events::NavigationCommand::Back => -1,
      crate::app::events::NavigationCommand::Forward => 1,
    };
    mounted.request_navigation(crate::app::window::NavigationRequest {
      command,
      source: crate::app::events::NavigationSource::Browser,
      origin: crate::app::window::NavigationOrigin::Focus,
      done: Some(Box::new(move |_| {
        if router.current_entry() != entry
          && let Some(history) = web_sys::window().and_then(|window| window.history().ok())
        {
          // Something on the page took it, or nothing did: the browser goes back to where the router is.
          let _ = history.go_with_delta(-delta);
        }
      })),
    });
    return;
  }
  if entry.is_some_and(|entry| handle.go_to_entry(entry)) {
    return;
  }
  let path = address_path(&window).unwrap_or_else(|| "/".to_owned());
  from_address(|| {
    if state.is_null() || state.is_undefined() {
      // An address typed into the bar or followed: a new entry after the current one, as the browser made.
      // A guard that refuses it leaves the router where it is, and the browser's new entry is that same place.
      handle.push(path);
    } else {
      // An entry from before the page was loaded, or one this router no longer has.
      handle.restart_at(path);
    }
  });
  mark_current(&window, handle);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_token_names_an_entry_of_the_load_that_made_it() {
    let state = token(7, 42);
    assert_eq!(entry_of(&state, 7), Some(42));
    assert_eq!(entry_of(&state, 8), None, "an entry from before a reload");
    assert_eq!(entry_of("something else", 7), None);
    assert_eq!(entry_of("lurq-router:7:", 7), None);
    assert_eq!(entry_of("lurq-router:7", 7), None);
  }

  #[test]
  fn a_fragment_is_a_path_when_it_starts_as_one() {
    assert_eq!(path_of("#/settings"), Some("/settings"));
    assert_eq!(path_of("#/search?q=a b"), Some("/search?q=a b"));
    assert_eq!(path_of("#/"), Some("/"));
    assert_eq!(path_of("#section"), None, "an anchor of the page");
    assert_eq!(path_of("#"), None);
    assert_eq!(path_of(""), None);
  }
}
