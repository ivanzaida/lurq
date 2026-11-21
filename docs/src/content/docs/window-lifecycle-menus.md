---
title: Window lifecycle and native menus
description: Veto OS close requests, defer a decision to a dialog, and install reactive macOS menus.
---

Introduced in **lurq 0.18.1**; the behavior below describes **0.65.0**. The `winit` shell delivers close requests and menu
activations on its event-loop thread. These APIs also have headless MCP coverage.

## Closing a window

Register once during a component's `create`, on that component's window:

```rust
let pending = Arc::new(Mutex::new(None::<lurq::app::CloseRequest>));
let save_request = pending.clone();
let show_dialog = dialog_open.clone();
let has_changes = dirty.clone();
ctx.window().on_close_requested(move |request| {
    if has_changes.get_untracked() {
        *save_request.lock().unwrap() = Some(request);
        show_dialog.set(true);
    } else {
        request.proceed();
    }
});

// A later dialog button accepts the retained request:
if let Some(request) = pending.lock().unwrap().take() {
    request.proceed();
}
```

- `on_close_requested` replaces the handler for this window. Primary and
  `WindowOpener` secondary windows use the same API.
- `CloseRequest::source()` is `Os` for native close/termination and `App` for
  `WindowHandle::request_close()` or MCP `request_close`.
- `proceed(self)` queues unconditional close and wakes the shell. A primary
  window exits the event loop; a secondary window closes independently.
- `cancel(self)` or dropping a request keeps the window open. Requests are
  `Send + 'static` and may be retained across frames or moved to another thread.
- `WindowHandle::close()` is unconditional and never invokes the handler.
  Existing confirmation buttons that call it continue to work.
- With no handler (or after `clear_close_requested_handler()`), requests close
  immediately as in previous releases.

The built-in `WindowControls` close button invokes its optional `on_close`
callback and then calls unconditional `close()`. That callback cannot veto
closing in 0.20.0. For drawn chrome that needs confirmation, replace the built-in
controls with your own button:

```rust
use lurq::components::{Button, ChromeTitleBar};

let window = ctx.window();
let title_bar = ChromeTitleBar::new()
    .without_controls()
    .trailing(Button::new("Close").on_click(move |event: lurq::app::events::MouseEvent| {
        window.request_close();
        event.prevent_default();
        event.stop_immediate_propagation();
    }));
```

Pass `title_bar` to `WindowChrome::title_bar`. Handlers should return promptly; display a
dialog instead of blocking the event loop. Repeated OS requests are delivered
individually, so an application can keep or replace its pending decision.

On macOS, lurq also handles `applicationShouldTerminate:` (Dock Quit and the
standard Quit action). It answers `NSTerminateCancel` and queues an OS request
for the primary window. This allows the normal event loop to render a dialog;
acceptance then exits winit normally. The primary handler should consider every
document that application termination would close. Session-end vetoes and force
termination are outside this API.

## Building a native menu

```rust
use lurq::app::{App, Accelerator, ApplicationMenu, Menu, MenuAction, MenuBar};

let app = App::new();
app.set_menu_bar(MenuBar {
    application: ApplicationMenu {
        name: "My App".into(),
        about: Some(MenuAction::new("about", "About My App")),
        preferences: Some(MenuAction::new("preferences", "Preferences…")
            .accelerator(Accelerator::command(","))),
        ..Default::default()
    },
    menus: vec![Menu {
        title: "File".into(),
        items: vec![
            MenuAction::new("newDocument", "New Document")
                .accelerator(Accelerator::command("n")).into(),
            MenuAction::new("export", "Export").enabled(false).into(),
        ],
    }],
});
```

`MenuItem::Separator` and `MenuItem::Submenu(Menu)` add dividers and nested menus.
IDs must be unique across the entire bar, including application commands.
Unknown, duplicate, removed, and disabled IDs cannot activate. Labels are
independent of IDs, so translations do not change command routing.

The application menu supplies Services, Hide, Hide Others, and Show All. Its
About and Preferences entries are optional; set their localized labels and IDs.
The default Quit command is `quit` with `⌘Q`. Customize its label/ID through
`ApplicationMenu::quit`.

Register command handling with `app.on_menu_activate(...)`. Route your Quit
command to the primary window's `request_close()` (or the same lifecycle decision
function), and Preferences to your `WindowOpener`. **With a menu handler installed,
that handler owns all commands, including Quit.** Without one, the default Quit
action automatically makes a vetoable OS close request. Native Quit never calls
AppKit termination directly.

`app.menu_bar_support()` reports `Native` on macOS with `winit`, and `Unavailable`
elsewhere. Setting a bar has no native UI effect on Windows/Linux. The model is
retained on every platform for headless QA; keep the application's drawn menu on
those platforms.

## Reactive updates and shortcuts

Clone `app.menu_controller()` into `ctx.on_effect` and call `controller.set(bar)`
with signal-derived labels, enabled flags, and item presence. Equal models are
ignored; a changed model wakes the event loop. Native objects are rebuilt only
when the model changes, on the main thread. No AppKit objects enter signal state.
Availability is checked again at dispatch, so a queued activation of an item
that has since become disabled or disappeared is ignored.

An `Accelerator` contains a lowercase character/named key and `SyntheticModifiers`
(`meta`, `ctrl`, `alt`, `shift`). `Accelerator::command("s")` means `⌘S`; set
`modifiers.shift = true` for `⇧⌘S`. Supported named keys include Tab, Enter, Escape,
Backspace, Delete, ArrowUp/Down/Left/Right, and F1–F24. Invalid multi-character
keys have no native equivalent. `display()` gives a readable modifier/key label.

An `Accelerator` is the key combination of lurq everywhere: what a menu item is chosen by, what `ui::Kbd` draws and what
a `ui::ShortcutRecorder` holds. `Accelerator::parse("Ctrl+Shift+P")` reads one as a person writes it (`Mod` is ⌘ on a
Mac and Ctrl elsewhere, `Cmd`, `Option`, `Esc`, `Space` and `F5` too), and its `Display` writes it back in that form,
the modifiers in the order Ctrl, Alt, Shift, Meta: what an app stores. `Accelerator::from_key_event(&event)` is the
combination a key that went down makes (a letter or a digit unshifted, `None` for a modifier alone), and
`accelerator.matches(&event)` asks whether a key is it. `has_modifier()` says whether a modifier is held with the key.

```rust
use lurq::app::{Accelerator, events::KeyboardEvent};

let save = Accelerator::parse("Mod+S").unwrap();
Row::new().on_key_down(move |event: KeyboardEvent| {
  if save.matches(&event) {
    event.prevent_default();
    // save
  }
});
```

On macOS, a handled native key equivalent consumes both key down and its matching
key up before the tree sees them. Keep menu-owned shortcuts in the menu model;
remove their duplicate macOS tree handlers. Unhandled keys continue to the tree.
On other platforms, keep your existing drawn-menu shortcut handlers.

The bar has no Edit menu of its own. An app's Undo, Redo, Cut, Copy, Paste and Select All items take
`EditCommand::accelerator()` as their accelerator and run the command on what has the focus with
`window.edit(EditCommand::Undo)` (a `WindowHandle` the menu handler keeps), as the key would: a focused text input's
own history first, then the `on_undo` handlers of the focused element and its ancestors. See
[Editing commands](../styling-events/#editing-commands-undo-redo-copy-cut-paste).

## MCP and verification

With `mcp` enabled and `Tree::enable_mcp` configured:

| Tool/action | Scope | Result |
| --- | --- | --- |
| `lurq_menu` | Observe | Platform support and current model, including nested items, labels, IDs, enabled flags, and accelerators |
| `lurq_interact {"action":"menu_activate","id":"preferences"}` | Interact | `activated: true` only for a currently enabled, unique ID |
| `lurq_interact {"action":"request_close","window":"main"}` | Interact | Invokes the same close dispatcher and returns `close_queued` / `stayed_open` |

Close results describe the instant after handler dispatch. A retained request
may proceed later, and actual OS teardown is asynchronous. Use tree/window
inspection to verify a dialog or secondary-window removal. A successful primary
close also shuts down its MCP server. Synthetic `key` input goes directly to the
tree; use `menu_activate` to exercise the menu model headlessly.

Run the interactive example with:

```sh
cargo run -p lurq --example lifecycle --features mcp
```

It has a dirty checkbox, a deferred confirmation dialog, drawn chrome, a
Preferences window, and File/Edit/Help menus. Save is enabled only while dirty.
Use its explicit request-close action or an OS close request to test vetoes;
the example's built-in chrome close button still has the unconditional-close
behavior described above.
On Windows, exercise native close and MCP together with:

```sh
cargo build -p lurq --example lifecycle --features mcp
python scripts/windows-lifecycle-check.py target/debug/examples/lifecycle.exe target/lifecycle-evidence
```

The main-thread native integration test runs with:

```sh
cargo test -p lurq --test native_macos --features winit,mcp
```

## Video diagnostics

Routine video timeline, frame timing, and native-image diagnostic logs are off
by default, even if the host application's tracing filter enables debug output.
Set **`LURQ_VIDEO_LOGS=1` before process startup** to opt in, then enable the
desired tracing targets (for example `RUST_LOG=video=debug`). The switch is cached
on first use. Renderer errors continue to use normal tracing filters.
