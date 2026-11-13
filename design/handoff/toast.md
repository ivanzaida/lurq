# Toast: hand-off

`ui::Toast`, the panel (board: `I7NIJT` Toast and `hCti2` Toast/Rich, in "Overlays" of `Components / light`), and
`ui::Toasts`, what shows toasts from anywhere and takes them away after a while.

## Changed outside the component's own files

- `crates/lurq/src/ui/button.rs`, `crates/lurq/src/ui/link.rs`: `#[derive(Clone)]` on `Button` and on `Link`, and
  nothing else. Every field of both was `Clone` already. A `Toasts` keeps the toasts it was sent and builds each
  again whenever it renders, in the theme it renders in, so a `Toast` is `Clone`, and the action it holds with it.

The host uses `Modal::window_chrome_layer()`, which is `pub(crate)` in `app/ctx.rs` and was not changed (see
Decided).

## For `docs/src/content/docs/design-system.md`

## Toast

A short message that something happened, in a corner of the window over everything, that leaves by itself:
settings saved, a file moved to the trash, a connection that failed.

```rust
use lurq::ui::{Button, ButtonSize, ButtonVariant, Toast, ToastVariant, Toasts};

// In the root component's create, once; its render puts the host last, so that its layer is over every other.
let toasts = Toasts::new();
column.child(self.toasts.host(ctx));

// Anywhere, with a clone of it.
toasts.show(Toast::new("Settings saved").description("Your theme applies to every open window.").variant(ToastVariant::Success));
toasts.show(
  Toast::new("File moved to trash")
    .description("theme-dark.json")
    .action(Button::new("Undo").variant(ButtonVariant::Outline).size(ButtonSize::Sm).on_click(move |_| restore()))
    .timer(true),
);
```

| Builder | |
| --- | --- |
| `Toast::new(title)`, `description(&str)` | What happened, and more of it under the title |
| `variant(ToastVariant)` | `Neutral` (the default, no icon), `Success`, `Info`, `Warning`, `Error`: the icon and its color |
| `icon(name)`, `icon_hidden(bool)` | An icon of the theme's set in place of the variant's, or none |
| `action(Button \| Link)` | What can be done about it, before the close control |
| `closable(bool)`, `on_close(handler)` | Whether it has its close control (it has by default), and what a press of it calls |
| `timer(bool)` | A rail along the bottom edge that says how long it has left |
| `duration(Duration)`, `persistent(bool)` | How long a `Toasts` shows it (`TOAST_DURATION`, 5 s), or that it stays until it is closed |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel; `force` draws the close control |

| `Toasts` | |
| --- | --- |
| `Toasts::new()` | No toasts yet. It needs no `Ctx`: make it once and clone it into whatever shows a toast |
| `show(toast) -> ToastId`, `dismiss(id)`, `clear()` | Shows one in front of the others; takes one away; takes all away |
| `ids()`, `len()`, `is_empty()` | The toasts shown and waiting, the oldest first |
| `host(ctx)`, `host_at(ctx, ToastPosition)` | What draws them: at the bottom right, or at `TopLeft`, `Top`, `TopRight`, `BottomLeft`, `Bottom` |

- A toast is a panel like any other component and can be put anywhere. Sent to a `Toasts`, it is drawn by the
  host in a layer over the window: the layer takes a press only on the toasts, and Tab reaches their controls
  from the page as it reaches any other.
- It leaves after its duration, on a press of its close control or of its action, and by `dismiss`. While the
  pointer is on the toasts their time stands still; when it leaves, each has its whole duration again.
- Several are stacked: the newest in front, an edge of each of two others behind it, on the side away from the
  window's edge. With the pointer on them they are laid out one over another, 10 apart, the newest nearest the
  edge. At most `TOASTS_SHOWN` (3) are shown; the others wait, and only the front one's time runs while stacked.
- Parts: `ToastPart::Root` (the panel), `Content` (what is in it above the timer, side by side), `Icon`, `Text`
  (the title over the description), `Title`, `Description`, `Action`, `Close`, `Timer` (the rail), `Remaining`
  (what is left of it); and the host's `Viewport` (how far from the window's edges), `List` (the toasts one over
  another), `BehindNear`, `BehindFar` (the edges behind the front one). State: `ToastState::Action`. Variants:
  `ToastVariant`.
- In the design the panel is 360 wide, in `Popover` with an edge of `Border`, large corners and the large
  shadow; 16 inside it are the icon of 16 in the variant's color (`Success`, `Info`, `Warning`, `Destructive`),
  12 from the title (14, medium) over the description (13, `MutedForeground`), and the close control of 16 in
  `MutedForeground`. With an action it is 14 high inside, centered on the action, and the title is 2 over the
  description. The timer is a rail of 2 in `Secondary` with what is left in `Primary`; it runs out over the
  toast's duration. Toasts are 16 from the window's edges; the edges behind are 24 and 48 narrower than the
  panel, three quarters and half opaque.
- To what reads the tree the panel is a `status` called by its title, and its close control a `button` called
  "Close". Tab reaches the action and the close control; Space and Enter press them.
- A renderer that draws another toast starts from the view's parts: `view.icon()`, `view.text()`,
  `view.action()`, `view.close()` (the close control with its behavior) and `view.timer()`.

## For `CHANGELOG.md`

- `ui::Toast` and `ui::Toasts`: a short message in a corner of the window that leaves by itself. The panel with an icon by its variant (`Neutral`, `Success`, `Info`, `Warning`, `Error`), a title and a description, an action (a `Button` or a `Link`), a close control and a timer; and a handle that shows toasts from anywhere, with a host the app's root renders once, that stacks them, pauses them under the pointer and takes them away after their duration. `ui::Button` and `ui::Link` are `Clone`.

## Decided

- **The host is a mounted component, and each toast in it one too.** A toast leaves after a while: that is a
  timer, which needs a `Ctx`. A builder has none, so `Toasts::host` mounts `ToastHost`, which mounts a `ToastItem`
  for each toast it shows, keyed by its id; each item creates a `Timeout` of the toast's duration in `create`.
- **The layer is the window chrome's**: `Modal::new(..).target(Root).window_chrome_layer()`, declared by the host.
  A dialog's layer (`Modal` as it is) traps Tab, so a toast would hold the keyboard while it is shown; a `Popup`
  needs an anchor and closes on a press outside it; a stack over the root would be under every dialog. The window
  chrome's layer is a modal without the modal's role: it is laid over the window, takes a press only on what is in
  it (`ContentOnly`, as the host's own full-window column is too), and is no scope of Tab. Declared by the root's
  last child, it is over the dialogs the page declares before it (the board's layering puts toasts at 1400, over
  modals at 1300). It is named for the window chrome; a layer of its own for toasts could be given a name in the
  engine.
- **`Toasts::new()` needs no `Ctx`**: its lists are `Signal::new`s, which the host's render reads and so renders
  again on. The app can make it in its root's `create` or anywhere else.
- **A toast in the list is behind a lock**: `Signal<Vec<Shown>>` with `Shown { toast: Arc<Mutex<Toast>> }`. A
  signal's value has to be `Sync`, and an element is not (a `Button` can hold one).
- **The pause is for all of them**, while the pointer is on the stack or the list, and it resets: a `Timeout` has
  one duration, so on leaving each toast has its whole duration again. The timer's rail shows that: full while
  paused, then running out again from full.
- **Stacked, only the front toast's time runs.** The ones behind are not mounted, only their edges are drawn; they
  come to the front in turn. At most three are shown; more wait, in order. With one toast it is drawn alone.
- **A press of the action takes the toast away** after the action's own handler, as a press of the close control
  does after `on_close`.
- **The timer runs out by a transition of its width**, from the panel's width to nothing over the toast's
  duration, linear. It starts on the item's second render (`on_mounted` sets a signal), since a transition runs
  from one value to another. Where a theme gives the panel no width in pixels, the rail is shown full.
- **Variants**: the board draws success and error on the desktop, and a neutral one (no color, its own icon) on the
  phone and the tablet. Info and warning follow the same rule with the theme's roles and the set's `info` and
  `warning` icons. A neutral toast has no icon unless it is given one, in the panel's foreground.
- **Alignment**: without an action, the icon, the text and the close control are at the top (a long description
  wraps under the title); with one, everything is centered on the action, as the board's Rich toast is.
- **The close control** is the board's 16 by 16, a `pressable` with the theme's focus ring and small corners for
  it, in `MutedForeground` and `PopoverForeground` under the pointer.

## Not done

- **Entering and leaving are not animated.** An exit animation needs keyframes registered on the tree
  (`Tree::register_keyframes`); lurq registers none for components besides the spinner's. A toast appears and
  leaves at once, and the stack does not animate between stacked and laid out.
- **The time left is not kept over a pause**: it starts again (see Decided). Keeping it needs a timer whose
  duration can change, or the host's own clock.
- No swipe to dismiss, no Escape, no key that focuses the toasts, no limit by the window's height.
- The host does not keep clear of a safe area (a phone's notch, home indicator).
- The phone's and the tablet's toasts (`Mobile/Toast`, `Tablet/Toast`: full width, larger), `Notification`,
  `Notification Center`, `Banner` and `Alert` were not asked for.
- **A press on a toast while a popup is open** is a press outside the popup: the outside-press lookup counts the
  window chrome's layer as the page (`outside_press.rs`, `pressed_layer`). With `OutsidePress::Consume` the press
  only closes the popup and the toast's control is not pressed; the next press reaches it.
- No tests were run; the host's tests drive the timers with `Tree::tick_timers_at` and size the window to the
  stage with `tree.resize(400, 300)`.

## Differs from the board

- **The Rich toast's padding**: the board has 14 above and under, 16 at the start and 12 at the end. A part's
  style has one padding for both sides, so it is 16 at both.
- **The Rich toast's description** is in the board's mono at 12 (`theme-dark.json`): that is the look of a file
  name, not of every description. Here a description is the plain toast's 13 in the sans family; the gallery
  writes the file name as a description too.
- **The edges behind** are drawn as panels three times as tall as what shows of them, their far part under the
  toast in front: the board draws an 8 high strip with its top corners rounded, which a part's style (one radius
  for every corner) cannot say. What shows is the same.
- **The timer** on the board is frozen at 210 of 360. A toast that is not counted down shows all of it.
- **The distance from the window's edges** is 16: the board's "toast positions" is a miniature (slots 10 from the
  edge of a 300 wide window) and gives no number for a real window.
- **Laid out one over another**, toasts are 10 apart, as the tablet's toast stack has them: the desktop board
  draws only the stacked form.
