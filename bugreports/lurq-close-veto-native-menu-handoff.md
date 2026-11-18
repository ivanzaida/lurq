# lurq handoff — window close veto and native macOS menu bar

2026-09-10. Written by the Kontur desktop orchestrator for an agent working in the lurq repository. Two toolkit gaps block interaction-contract rows of the desktop design ([docs/design/kontur-desktop-app.md](H:/projects/pencil-web/docs/design/kontur-desktop-app.md), frames 79 and 83): the OS close request cannot be intercepted, and macOS has no native menu bar. Everything below was verified against lurq `0.18.0` (`def8cf4`, `H:/projects/lurq`) and Kontur `main` at `66e7641`.

```text
Archetype: implementation in lurq (toolkit owner), not a Kontur task
Repository: H:/projects/lurq (work in a worktree; release as 0.19.0)
Consumer: H:/projects/pencil-web/apps/desktop (kontur-desktop, lurq features winit+wgpu+mcp+form+persistent_storage)
Scope: (1) close-request veto for primary and secondary windows, (2) declarative menu bar with a native macOS backend
Out of scope: Linux/Windows native menus (Kontur keeps its drawn ☰ there), tray icons, dock menus, session-end (logoff/shutdown) vetoes
Mutation policy: implementation in lurq; Kontur adapts afterwards (follow-up tasks listed at the end)
Next owner: Kontur desktop orchestrator, after the lurq release is published
```

## 1. Close-request veto

### What lurq does today

- Primary window: `crates/lurq/src/app/winit_shell.rs:630` — `WindowEvent::CloseRequested` calls `event_loop.exit()` when `close_exits` is set (the app window) or drops the window otherwise. The same arm exists at `winit_shell.rs:371` for the resumed path. No callback runs; the app never learns that the OS asked.
- Secondary windows (`WindowOpener`, e.g. Kontur's Preferences window): `winit_shell.rs:1071` calls `request_close()` (`:822`), which just sets `close_requested` so the window is torn down on the next pass (`:1553`).
- Programmatic close: `WindowHandle::close()` (`crates/lurq/src/app/window.rs:116`) pushes `WindowCommand::Close` (`window.rs:282`). `WindowChrome`'s drawn close button calls the app's `on_close` handler (`components/window_chrome.rs:487`, `:537`), so drawn chrome is already interceptable — only the OS path (Alt+F4, taskbar/Dock close, ⌘W on a native title bar, Windows "Close window" from the thumbnail) bypasses the app.

### What Kontur needs

Kontur's shell owns the decision (`apps/desktop/src/shell/lifecycle.rs`): `Lifecycle::request_close_window()` closes at once when every document is synced, otherwise it sets `pending = CloseRequest::Window` and the `Close with unsynced changes?` dialog (frame 83, ids `dialog-close-window`, `dialog-cancel`, `dialog-confirm`) decides; `confirm()` then calls the closure that today is `ctx.window().close()` (`shell/mod.rs:289`). The OS path must enter exactly this function instead of exiting. The README currently documents the gap: "intercepting the OS close request (Alt+F4 / taskbar close bypasses the dialog; lurq 0.18 offers no veto)".

### Proposed API

```rust
/// Registered on a window handle; runs on the event-loop thread for every
/// OS-initiated close of that window (Alt+F4, taskbar/Dock close, native ⌘W,
/// `WM_CLOSE`). While a handler is registered lurq neither exits nor drops the
/// window on `CloseRequested`; the handler owns the outcome.
impl WindowHandle {
    pub fn on_close_requested(&self, handler: impl Fn(CloseRequest) + Send + Sync + 'static);
    /// Programmatic close; never consults the handler (the app already decided).
    pub fn close(&self);
    /// Same path as the OS request: goes through the handler if one exists.
    pub fn request_close(&self);
}

pub struct CloseRequest { /* window id, source: Os | App */ }
impl CloseRequest {
    pub fn source(&self) -> CloseRequestSource;
    /// Let the window close (primary window: exit the loop as `close_exits` does today).
    pub fn proceed(self);
    /// Keep the window; nothing else happens. Dropping the request without a
    /// decision must also keep the window (fail closed, never lose data).
    pub fn cancel(self);
}
```

Requirements behind the shape:

- Fail closed: if a handler exists and neither `proceed` nor `cancel` is called (handler stored the request to decide later, e.g. after a dialog), the window stays open. Kontur decides asynchronously: the dialog is rendered by the tree and `confirm()` runs on a later frame, so `CloseRequest` must be `Send + 'static` and usable outside the handler.
- One code path for both window kinds: the primary window and `WindowOpener` secondaries take the same handler type; Kontur's Preferences window has nothing unsynced and will simply `proceed()`.
- Programmatic `close()` must not recurse into the handler; otherwise `confirm()` → `close()` → handler → dialog again.
- macOS ⌘Q and Dock "Quit": winit does not deliver `CloseRequested` for application termination; the app delegate's `applicationShouldTerminate:` must be answered `NSTerminateLater`/`Cancel` and routed through the same handler (source `Os`). Without this, the veto covers ⌘W but not ⌘Q, which is the case the design cares about (`Close window / Quit` row).
- Existing behaviour is preserved when no handler is registered (no API break for other lurq apps).
- Headless/MCP: expose the request in the embedded MCP server so QA can drive it without OS input — `lurq_interact` on a window with an action such as `request_close` (or a dedicated `lurq_window_close_request` tool) that triggers the same handler and reports whether the window stayed open. Kontur's harness (`scripts/desktop/qa.py`, `scripts/desktop/mcp.py`) will use it as the acceptance check for the Alt+F4 row.

### Acceptance in lurq

1. Demo (`crates/demo`) with a "dirty" toggle: Alt+F4 / ⌘Q / taskbar close show an in-app dialog when dirty and close at once when clean; the drawn chrome button and the OS path share the handler.
2. Tests: runtime test that a registered handler receives `CloseRequested` and that `cancel()` / dropped request keeps the window, `proceed()` closes it; secondary-window variant; `close()` never invokes the handler.
3. Docs page under `docs/src/content` and a CHANGELOG entry; semver minor.

## 2. Native macOS menu bar

### What lurq does today

No menu API exists: `grep -rn "NSMenu\|muda\|menu_bar" crates/` finds nothing; "menu" appears only for `Select` popups and overlays. `objc2 = "0.6"` is already a dependency (`crates/lurq/Cargo.toml:112`), so an AppKit `NSMenu` backend needs no new foundation crate; `muda` is the alternative if a cross-platform model is preferred.

### What Kontur needs

The design puts the application menu in the OS menu bar on macOS and in a drawn ☰ popup on Windows/Linux (`docs/design/kontur-desktop-app.md`, "Title bar"). `apps/desktop/src/platform.rs` already models this: `MenuHost::{NativeMenuBar, DrawnMenu}`, `Os::preferred_menu_host()` (macOS → native), and the switch `NATIVE_MENU_BAR_AVAILABLE: bool = false` (`platform.rs:59`) with its doc comment naming lurq 0.18 as the reason. Flipping that constant hides the ☰ button on macOS (`title_bar.rs:90`) — the shell is ready for a native host; only the toolkit call is missing.

Kontur's menu model (`apps/desktop/src/shell/app_menu.rs`, `ENTRIES`): eleven items with dividers — `newDocument`, `openDocument`, `openRecent`, `saveCopy`, `import`, `export`, `preferences`, `shortcuts`, `checkUpdates`, `signOut`, `quit` — each with an i18n label (`desktop.menu.*`, `apps/desktop/locales/en.json`), an optional accelerator from the shared table (`apps/desktop/src/shortcuts.rs`: `⌘N`, `⌘O`, `⇧⌘S`, `⇧⌘E`, `⌘,`, `⌘/`, `⌘Q`, `⌘W`, `⌘Tab`), an `available` flag (unimplemented entries render disabled), and a conditional entry (`signOut` only while an account is signed in). Labels change with the locale; enabled state changes at runtime.

### Proposed API

```rust
/// Declarative, reactive menu bar. Set once on the App; rebuild or mutate
/// through signals as state changes.
pub struct MenuBar { pub menus: Vec<Menu> }
pub struct Menu { pub title: Arc<str>, pub items: Vec<MenuItem> }
pub enum MenuItem {
    Item { id: Arc<str>, label: Arc<str>, accelerator: Option<Accelerator>, enabled: bool },
    Separator,
    Submenu(Menu),
}
pub struct Accelerator { pub key: Key /* winit logical key or code */, pub modifiers: Modifiers }

impl App {
    /// `MenuBarSupport::Native` on macOS (AppKit menu bar), `Unavailable` elsewhere in this slice.
    pub fn menu_bar_support(&self) -> MenuBarSupport;
    pub fn set_menu_bar(&self, bar: MenuBar);
    /// Activation by click or accelerator, delivered on the event-loop thread with the item id.
    pub fn on_menu_activate(&self, handler: impl Fn(&str) + Send + Sync + 'static);
}
```

Requirements:

- macOS standard structure: an application menu (About, Preferences `⌘,`, Services, Hide/Show, Quit `⌘Q`) plus the app's menus; lurq should own the application menu skeleton and let the app fill Preferences/Quit/About with ids so Kontur can route Preferences to its window and Quit through the close veto of section 1 (Quit must never terminate directly).
- Accelerators are dispatched once: on macOS the menu's key equivalent wins and lurq must not also deliver the key event to the tree's shortcut handling (or must mark it consumed). Kontur handles `⌘Q`, `⌘W`, `⌘,`, `⌘Tab` itself today (`shell/shortcuts.rs`); after adoption it will drop the macOS duplicates for entries the menu owns. Document which side handles what.
- Reactive updates: labels (locale change), `enabled` (availability), presence (`signOut`) — updating the bar must be cheap enough to call from a signal effect.
- Windows/Linux: `MenuBarSupport::Unavailable` and `set_menu_bar` is a no-op; Kontur keeps ☰. A drawn cross-platform fallback inside lurq is welcome but not required.
- MCP: expose the current menu model (`lurq_read_tree` node or a `lurq_menu` tool listing menus/items/enabled/accelerator) and allow `lurq_interact` to activate an item by id, so Kontur's harness can verify the menu on macOS without UI automation.

### Acceptance in lurq

1. Demo with a File/Edit/Help bar on macOS, enabled state toggled by a signal, an item activated by accelerator and by click, Quit routed through the close veto.
2. Tests: model → activation mapping headless (ids, enabled items ignore activation), accelerator table rendering; a macOS-only integration check behind `cfg(target_os = "macos")`.
3. Docs + CHANGELOG; released together with section 1 as `0.19.0`.

## Kontur follow-ups after the release (not for the lurq agent)

- Bump `lurq` to `0.19` in `apps/desktop/Cargo.toml`; register `on_close_requested` in `shell/mod.rs` so `Lifecycle::request_close_window()` handles the OS path and `confirm()` calls `proceed()`; Preferences window handler proceeds. Add the harness check through the new MCP action and remove the Alt+F4 line from the README's "Not implemented yet".
- Flip `NATIVE_MENU_BAR_AVAILABLE` to `true` under `cfg(target_os = "macos")` (or replace it with `App::menu_bar_support()`), build the `MenuBar` from `app_menu::ENTRIES` with the i18n labels and the shortcut table, route activations to the existing `Command`s, drop the duplicated macOS shortcut handlers, and update `platform.rs` tests and the README "Platforms" section. macOS still cannot be run here; the headless `platform_layout` tests and `scripts/desktop/check-targets.py` remain the evidence until a Mac is available.
