//! The gallery's tabs: several pages open side by side, as a browser has them. Each tab is a router with a history
//! of its own in the engine's set of tabs (`Tabs`), and the page of a tab that is not shown stays as it was left,
//! with what its playground was set to and where it was scrolled. The strip that chooses one is the design
//! system's `WindowTabs`, in the title bar; the set's outlet is mounted with no bar of its own.

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicUsize, Ordering},
};

use lurq::{
  app::ctx::Ctx,
  components::{Column, Outlet, ScrollVertical, TabItem, TabOutlet, TabOutletProps, Tabs},
  core::ElementRef,
  node::Element,
  router::{RouterHandle, Routes},
  ui::{WindowTab, WindowTabs},
};

use crate::{
  kit::FILL,
  pages::{PAGES, page_at},
};

/// The tabs that are open, and what opens another.
#[derive(Clone)]
pub struct OpenTabs {
  set: Tabs,
  /// How many tabs were ever opened: the next one's key.
  opened: Arc<AtomicUsize>,
  /// The search has gone to a page, and the focus is to go to it once it is built.
  focus_page: Arc<AtomicBool>,
}

impl OpenTabs {
  /// One tab, at `path`.
  pub fn new(path: &str) -> Self {
    let open = Self {
      set: Tabs::new(),
      opened: Arc::default(),
      focus_page: Arc::default(),
    };
    open.open(path);
    open
  }

  /// Opens a tab at `path` after the others and shows it.
  pub fn open(&self, path: &str) {
    let key = format!("tab-{}", self.opened.fetch_add(1, Ordering::SeqCst));
    let router = RouterHandle::new(routes(self.focus_page.clone()));
    router.replace(path);
    self.set.add(key.as_str(), TabItem::new(page_at(path).name), router);
    self.set.select(&key);
  }

  /// Closes a tab, with its history and its page. The last one stays: the window always shows a page.
  pub fn close(&self, key: &str) {
    if self.set.len() > 1 {
      self.set.remove(key);
    }
  }

  /// The router of the tab that is shown: where the side bar, the search and a link from outside the pages go.
  /// A render that reads it is run again when another tab is shown.
  pub fn router(&self) -> RouterHandle {
    let key = self.set.active();
    self.set.router(&key).expect("a tab is always open")
  }

  /// The page the next navigation builds takes the focus: the search went there.
  pub fn focus_next_page(&self) {
    self.focus_page.store(true, Ordering::SeqCst);
  }

  /// The strip in the title bar: a tab for each page that is open, called as its page is, with what closes it,
  /// moves it and opens another.
  pub fn strip(&self, ctx: &mut Ctx) -> Element {
    let entries = self.set.list();
    let alone = entries.len() == 1;
    let tabs = entries.iter().map(|entry| {
      // A tab is called as the page it is at: read here, so that the strip follows each tab's own navigation.
      let page = page_at(&entry.router.path().get());
      let mut tab = WindowTab::new(entry.key.clone(), page.name)
        .key(entry.key.as_str())
        .close_button(!alone);
      if let Some(icon) = page.icon {
        tab = tab.icon(icon);
      }
      tab
    });
    let (closed, added, moved) = (self.clone(), self.clone(), self.set.clone());
    let keys: Vec<String> = entries.iter().map(|entry| entry.key.clone()).collect();
    WindowTabs::new(self.set.active_signal(), tabs)
      .label("Open pages")
      .add_label("New tab")
      .on_close(move |key: &String| closed.close(key))
      .on_add(move || added.open("/"))
      .on_reorder(move |from, to| {
        if let Some(key) = keys.get(from) {
          moved.move_to(key, to);
        }
      })
      .mount(ctx)
  }

  /// The page of the tab that is shown, with those of the others kept behind it.
  pub fn outlet(&self, ctx: &mut Ctx) -> Element {
    TabOutlet::mount(ctx, TabOutletProps::new(self.set.clone()))
  }
}

/// A tab's routes: every page, in a scroll of the tab's own.
fn routes(focus_page: Arc<AtomicBool>) -> Routes {
  // The page's own scroll, its main content: where the focus goes when the search has gone to a page.
  let main = ElementRef::new();
  Routes::new().layout(
    "/",
    move |ctx| {
      let current = page_at(&ctx.route_path()).path;
      // The search went to this page: what it found has the focus, and the next Tab goes on from its start. Asked
      // for here, by the render that builds the page, so that the page asked for is the new one.
      if focus_page.swap(false, Ordering::SeqCst) {
        ctx.focus(&main);
      }
      // What scrolls the page, said to the page: the list of its sections follows it.
      ctx.provide(crate::doc::PageScroll(main.clone()));
      // Each page has a scroll of its own: a page opened from another starts at its top, wherever the one before
      // it was scrolled to. It takes the focus but is no stop of Tab.
      let page = ScrollVertical::new(Outlet::mount(ctx))
        .key(current)
        .size(FILL, FILL)
        .ref_element(main.clone())
        .tab_index(-1)
        .describe("a11y_role", "main");
      // In a box of the tab's own, which the router names as the root of the routes' layout: the scroll keeps its
      // own name to what reads the tree.
      Column::new().size(FILL, FILL).child(page).into()
    },
    |mut routes| {
      for page in PAGES {
        routes = routes.route(page.path, page.mount);
      }
      routes.fallback(PAGES[0].mount)
    },
  )
}

#[cfg(test)]
mod tests {
  use super::OpenTabs;

  fn keys(open: &OpenTabs) -> Vec<String> {
    open.set.list().into_iter().map(|tab| tab.key).collect()
  }

  #[test]
  fn a_new_tab_is_the_one_shown_and_goes_its_own_way() {
    let open = OpenTabs::new("/button");
    let first = open.router();
    assert_eq!(first.path().get_untracked(), "/button");

    open.open("/");
    let second = open.router();
    assert!(second != first, "the new tab is the one shown");
    assert_eq!(keys(&open).len(), 2);

    // Where the second goes, the first stays; each has a history of its own.
    second.push("/tokens");
    assert_eq!(second.path().get_untracked(), "/tokens");
    assert_eq!(first.path().get_untracked(), "/button");
    assert!(second.can_back());
    assert!(!first.can_back());
  }

  #[test]
  fn a_closed_tab_gives_its_place_to_the_next_and_the_last_one_stays() {
    let open = OpenTabs::new("/button");
    open.open("/tokens");
    open.open("/icons");
    let all = keys(&open);
    // The one shown is closed: the one before it is shown, as it was the last.
    open.close(&all[2]);
    assert_eq!(keys(&open), all[..2]);
    assert_eq!(open.router().path().get_untracked(), "/tokens");
    // One that is not shown is closed: the one shown stays shown.
    open.close(&all[0]);
    assert_eq!(keys(&open), all[1..2]);
    assert_eq!(open.router().path().get_untracked(), "/tokens");
    // The window always shows a page.
    open.close(&all[1]);
    assert_eq!(keys(&open), all[1..2]);
  }
}
