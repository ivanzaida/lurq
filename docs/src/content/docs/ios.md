---
title: iOS
description: Run a lurq app on an iPhone, and what the shell does there for touch, scrolling, the keyboard and the safe area.
---

Added in **lurq 0.61.0**, for the iPhone. The `winit` shell with the `wgpu` renderer runs the same component
tree on a phone as on a desktop; this page says what an app has to set up and what it gets. What is not done yet
is listed in `docs/ios-gaps.md` in the repository. The iPad and several windows are not supported.

`crates/mobile_demo` is a complete example: a screen for each thing below, and the Xcode project that wraps it.
The gallery has one too, `crates/gallery/ios/Gallery.xcodeproj`, to look at every component on a phone or a
tablet; it is built and run as below, with `BUNDLE_ID_PREFIX=<yours>` (its bundle is `<yours>.lurq.gallery`) and,
for a feature such as `mcp`, `CARGO_FEATURES=mcp`.

## Setting up

An iOS app is a bundle that Xcode builds and signs, so the Rust binary needs a small Xcode project around it.

1. Add the targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`.
2. Depend on lurq with the `winit` and `wgpu` features. The app's `main` is the one it has on a desktop.
3. Make an Xcode project whose only build phase is a script that runs `cargo build --target ...` and copies the
   binary to `$TARGET_BUILD_DIR/$EXECUTABLE_PATH`. Name that path as the phase's output, or Xcode will not sign
   what the script put there. `crates/mobile_demo/ios/build-rust.sh` is such a script; it clears Xcode's
   environment first, because cargo's build scripts must not link against the iOS SDK.
4. Add the two keys below to the app's `Info.plist`.

### Info.plist

```xml
<!-- The app's one scene. lurq makes the delegate's class. -->
<key>UIApplicationSceneManifest</key>
<dict>
  <key>UIApplicationSupportsMultipleScenes</key>
  <false/>
  <key>UISceneConfigurations</key>
  <dict>
    <key>UIWindowSceneSessionRoleApplication</key>
    <array>
      <dict>
        <key>UISceneConfigurationName</key>
        <string>Default</string>
        <key>UISceneDelegateClassName</key>
        <string>LurqSceneDelegate</string>
      </dict>
    </array>
  </dict>
</dict>
<!-- Lets an iPhone with a 120 Hz display draw above 60 frames a second. -->
<key>CADisableMinimumFrameDurationOnPhone</key>
<true/>
```

- **The scene manifest is required.** UIKit refuses to start an app built with the iOS 27 SDK that declares no
  scene: it traps at launch. The shell registers the class `LurqSceneDelegate` and puts its window into the
  scene; the app only names the class.
- Without `CADisableMinimumFrameDurationOnPhone` an iPhone holds the app to 60 frames a second.

### Running

```sh
# A simulator
xcodebuild -project ios/MyApp.xcodeproj -scheme MyApp -sdk iphonesimulator \
  -destination "platform=iOS Simulator,id=<simulator id>" build
xcrun simctl install <simulator id> <path to MyApp.app>
xcrun simctl launch --console-pty <simulator id> <bundle id>

# A phone
xcodebuild -project ios/MyApp.xcodeproj -scheme MyApp -sdk iphoneos \
  -destination "id=<device id>" DEVELOPMENT_TEAM=<team> -allowProvisioningUpdates build
xcrun devicectl device install app --device <device id> <path to MyApp.app>
xcrun devicectl device process launch --console --device <device id> <bundle id>
```

`--console` shows what the app writes to standard error. A phone has to be unlocked for the launch.

## The safe area

The window covers the whole screen, under the status bar and the home indicator. `WindowInfo::safe_area` says
how far in from each edge content stays clear of them, in logical pixels; it is zero on a desktop.

```rust
let safe_area = ctx.window().safe_area;
Column::new()
    .background(background) // reaches the screen's edges
    .padding_top(safe_area.top)
    .padding_bottom(safe_area.bottom)
    .padding_left(safe_area.left)
    .padding_right(safe_area.right)
```

The on-screen keyboard counts: while it is up `safe_area.bottom` is its height, frame by frame as it rises, so
content padded by the safe area ends above it. The focused text input is scrolled into view as it does.
`safe_area.keyboard` is how much of `bottom` the keyboard adds, 0 with it down.

## Touch

What follows holds wherever winit tells touches, not on iOS alone: Android, a Windows or Linux touch screen and the
browser too (see [A Finger Or A Mouse](../styling-events/#a-finger-or-a-mouse)).

One finger is the pointer. `on_click`, `on_mouse_down` and the rest fire as for a mouse, with these differences,
which are the platform's:

- **A press waits** 150 ms over content that scrolls, to see whether the finger scrolls instead. A finger that
  moves more than 10 points scrolls, and there is no click. A quick tap still clicks at once.
- **Nothing is hovered.** Hover styles are not applied under a finger.
- **A tap shows as pressed** for a moment even when the finger was down for less than a frame.
- `MouseEvent::pointer_kind` is `PointerKind::Touch` for these events (`Pen` for a pen's), for an element that must tell.

A finger's own events are separate, and bubble like the mouse's:

| Handler | Event | When |
| --- | --- | --- |
| `on_touch_start`, `on_touch_move`, `on_touch_end`, `on_touch_cancel` | `TouchEvent` | each finger, by its `id` |
| `on_tap` | `TapEvent` | a tap, with how many `fingers` and which of a row it is (`count`: 1, 2, 3) |
| `on_double_tap` | `TapEvent` | the second of two taps in a row |
| `on_long_press` | `LongPressEvent` | a finger held for half a second |
| `on_swipe` | `SwipeEvent` | a quick move, with its `direction` and `velocity` |
| `on_pan` | `PanEvent` | a drag, from start to end |
| `on_pinch` | `PinchEvent` | two fingers moving apart or together: `scale` |
| `on_rotate` | `RotateEvent` | two fingers turning: `rotation`, in radians |
| `on_edge_pan` | `EdgePanEvent` | a finger in from a screen edge |
| `on_history_swipe` | `HistorySwipeEvent` | the same from the left edge (back) or the right (forward); then [going back](../styling-events/#going-back-and-forward) |

A double tap held and dragged zooms with one finger (`on_pinch`), three fingers swiped, pinched or tapped are the
editing commands and their menu, and a pen's hover hovers: [every gesture](../styling-events/#every-gesture) has
the table, with how each is made on a phone, in the emulator on a desktop and through MCP. In text, iOS draws its
own caret, loupe and handles; elsewhere lurq draws them.

A finger from the left edge goes back by itself: the router mounted in the window takes the command, after anything
on the way that handles `on_history_back` (a sheet that closes on it).

```rust
Column::new()
    .on_tap(move |event: TapEvent| fingers.set(event.fingers))
    .on_history_back(move |event: HistoryEvent| {
        if sheet_open.get_untracked() {
            sheet_open.set(false);
        } else {
            event.pass_on();
        }
    })
```

A handler of `on_touch_start` that calls `prevent_default` takes the finger: it is not the pointer, so it
neither clicks nor scrolls. Two fingers that pinch or rotate are not the pointer either.

**An action is `on_click`; a gesture is `on_tap`, `on_double_tap`, `on_swipe`, `on_pinch`, `on_long_press`.** A
mouse, a finger, a pen, Enter and Space all reach `on_click`, so an app with one code base for a phone and a desktop
writes its actions there once. The gestures are a finger's (and a pen's) only:

- `on_tap` comes first and `on_click` after it, and an element with both runs each once. A tap a handler takes
  (`prevent_default`) clicks nothing, as a long press that is taken does not.
- A tap of two fingers or more never clicks: it is `on_tap` with its `fingers`. A click is always one finger's.
- A click never waits for a second tap, except on an element that listens for one with `on_double_tap`: there the
  single click is held back for the time of a double click (`Tree::set_double_click_interval`, timed on the clock
  of presses), and two taps are the double tap alone, with no click. A double tap is not `on_dblclick`: on a phone
  it zooms, and a finger never makes a double click.
- A pen's tap is a click and a tap, as a finger's; `MouseEvent::pointer_kind` says `Pen`.

### Haptics

```rust
ctx.window().haptic(Haptic::Selection);
```

`Haptic` is `Light`, `Medium`, `Heavy`, `Selection`, `Success`, `Warning` or `Error`. It does nothing where there
is no haptic engine.

## Screens

`StackOutlet` moves between the router's screens as the platform's apps do: a push comes in from the right, a
finger from the left edge drags the top screen back, the screens under stay alive, and a navigation bar shows
the title and the way back. See [A Stack Of Screens](../routing/#a-stack-of-screens).

`TabOutlet` puts several such stacks side by side under a bar of tabs, each kept as it was left. See
[Tabs](../routing/#tabs). Under the bar a screen pads its content by `ctx.safe_area()`, which leaves out what
the bar already covers.

`Sheet` brings content up from the bottom over the screens and the bar: it stands at its heights, and a finger
drags it between them and away. See [Sheets](../modals/#sheets).

## Scrolling

Scroll containers scroll under a finger as `UIScrollView` does: the content follows the finger, goes on with
momentum when it lifts, stretches past its ends and springs back, and stops under a finger that lands on it.
One gesture scrolls one container, the innermost that has room in the finger's direction.

- **The scroll indicator.** On iOS a bar is `ScrollBarStyle::indicator()` unless styled otherwise: thin, over
  the content, shown while it scrolls.
- **A tap on the status bar** scrolls the outermost vertical list to its top.
  `ScrollState::set_scrolls_to_top(false)` leaves a list out of it.
- **The keyboard** stays up when a list is scrolled, unless the list says otherwise with
  `ScrollState::set_keyboard_dismissal(..)`: `KeyboardDismissal::OnDrag` puts it away when a finger starts
  to scroll, and `Interactive` has it go down with the finger once the finger reaches it, and come back if
  the finger goes back up. `set_dismisses_keyboard(true)` is `OnDrag`.
- **Momentum** can be changed for the whole window with `Tree::set_touch_momentum(TouchMomentum { .. })`:
  the deceleration rate, and the slowest and fastest speeds. The defaults are the platform's.
- **By pages.** `ScrollState::set_paging(true)` has a finger scroll a list a page at a time, a page being as
  long as the list's viewport: let go, the list goes to the nearest page; thrown, to the next one that way,
  and never further than the page beside the one the finger started on. `ScrollState::page()` says which
  page it is at. A wheel and a scroll bar scroll such a list as any other.

`VirtualizedList` scrolls the same way.

### Pinch to zoom

`ZoomScroll` goes around content of a size the app says, a photo or a map: two fingers pinch it larger and
smaller about their middle and carry it with them, a double tap goes in at the point tapped and back out,
and one finger scrolls what is shown.

```rust
ZoomScroll::mount(
  ctx,
  ZoomScrollProps::new(photo_width, photo_height).range(fit, 4.0).zoom(self.zoom.clone()),
  Image::new(photo).size(photo_width, photo_height),
)
```

- The list fills what it is put in. `range` is the smallest and the largest scale, 1 and 4 unless set;
  content shown smaller than the list lies in its middle.
- `zoom(signal)` is the scale, to read and to set; `scroll(state)` is the list's `ScrollState`.
  `double_tap(None)` leaves a double tap to the content. A scale set from outside, by a button or a key, keeps the
  middle of the view where it is.
- With a mouse: `wheel_zoom(step)` makes the wheel bring the content closer and further about the pointer, by
  `step` at a notch, in place of scrolling it, and `drag_pan(true)` lets a press held and moved carry the content
  as a finger does. Neither is on unless asked for: without them the wheel scrolls the content as in any list.
- The content is laid out once, at its own size, and drawn scaled: text in it is not laid out anew for the
  size it is shown at.
- A pinch stops at the smallest and the largest scale, where the platform's gives and springs back, and a
  double tap changes the scale at once, with no move.

### Pull to refresh

`PullToRefresh` goes around a list. Pulled down past its top far enough, the list sets its signal and calls
`on_refresh`; it stays down with a spinner over it until the app clears the signal.

```rust
let list = ScrollVertical::new(rows).with_scroll_state(self.scroll.clone());
PullToRefresh::mount(
  ctx,
  PullToRefreshProps::new(self.refreshing.clone(), self.scroll.clone()).on_refresh(move || load()),
  list,
)
```

- Around a stack screen's list the state is `StackOutlet::screen_scroll(ctx)`, and `.top(StackOutlet::bar_inset(ctx))`
  puts the spinner under the navigation bar.
- `PullToRefreshStyle`: `threshold` (100), how far the list is pulled before it asks; `hold` (60), how far
  down it stays meanwhile; `spinner_size`, `spinner_color`, `duration` and `haptic`. The spinner's color is the
  theme's `MutedForeground`, light or dark as the theme is (see [Colors from the theme](#colors-from-the-theme)).
- A spinner by itself is `lurq::ui::Spinner`. The platform's spokes, `ActivityIndicator`, are deprecated for an
  app since 0.63; the pull to refresh still draws them.

### Swipe actions

`SwipeRow` goes around a row of a list. Swiped towards the leading edge it shows its trailing actions, and
the other way its leading ones.

```rust
SwipeRow::mount_keyed(
  ctx,
  &message.id,
  SwipeRowProps::new()
    .trailing([
      SwipeAction::destructive("Delete", move || delete(id)),
      SwipeAction::new("Flag", move || flag(id)),
    ])
    .leading([SwipeAction::new("Read", move || read(id))]),
  row_content,
)
```

- Let go far enough, or thrown, the row stays open; a press on an action does it and closes the row, and a
  press on the open row closes it.
- A swipe all the way (`SwipeRowStyle::full_swipe`, 0.6 of the row's width) does the first action of that
  side at once, with a tick of the device.
- The first action given is the one at the edge. `SwipeAction::icon` puts a picture over the title.
- `SwipeRowProps::open(signal)` says which side is open (-1 trailing, 0, 1 leading), for a list that closes
  the other rows when one opens.
- The content paints its own background: the actions are behind it.
- An action is in the theme's colors: `SwipeAction::new` on `MutedForeground` with the `Background`'s ink,
  `SwipeAction::destructive` in `Destructive` and `DestructiveForeground`. `background` and `color` set others.

### Colors from the theme

The sheet, the tab bar, the navigation bar, the pull to refresh and the swipe row take their colors from the
theme, where their styles are made: `SheetStyle::new()`, `TabBar::new()`, `NavigationBar::new()`,
`PullToRefreshStyle::new()` and `SwipeAction::new(..)` read the recipe the theme keeps for each (`phone_sheet`,
`phone_tab_bar`, `phone_navigation_bar`, `phone_pull_to_refresh`, `phone_swipe_row`), which names the design's
roles. Built in a dark theme they are dark: there is no `dark()` to choose any more (it is deprecated, with the
colors of 0.62). `ThemeDefinition::legacy()` gives them the colors they had in 0.62. A field set on a style is
that color whatever the theme; a theme restyles every one of them with a patch:

```rust
use lurq::{
  app::theme::{PaletteColor, ThemeDefinition},
  components::{TabBar, TabBarPart},
};

let mut theme = ThemeDefinition::neutral();
// The active tab in the brand's color, in every tab bar of the app.
theme.ui.patch::<TabBar>(|rules| rules.base(TabBarPart::SelectedItem, |s| s.foreground(PaletteColor::Primary)));
```

The parts are `SheetNode` (`Panel`, `Grabber`, `Backdrop`, whose opacity is the sheet's `dim`), `SheetHostPart`,
`TabBarPart`, `NavigationBarPart`, `PullToRefreshPart` and `SwipeRowPart`. The board draws design-system
components for a phone (a tab bar, an app bar, a bottom sheet, a swipe row, a pull to refresh) that `lurq::ui`
does not have yet; these are the engine's, in the board's roles.

## Text inputs

A focused `TextInput` brings the on-screen keyboard up, and loses it with the focus. The platform does the
editing by finger: its own caret, the handles at a selection's ends, the loupe, and the menu to copy, cut, paste
and select all. Suggestions, autocorrection, dictation and the caret under the space bar work as in any text
field.

What the keyboard is, an input says:

```rust
TextInput::new(email)
    .single_line()
    .keyboard(KeyboardKind::Email)
    .content(ContentKind::Email)
    .return_key(ReturnKey::Next)
```

- `keyboard`: `Text`, `Email`, `Url`, `Number`, `Decimal`, `Phone` or `Search`.
- `return_key`: what the Return key says: `Default`, `Done`, `Go`, `Next`, `Search` or `Send`. It is a label:
  the key is an Enter key press to the input, as on a desktop.
- `content`: what the text is, for the platform to offer the person's own: `Name`, `Email`, `Username`,
  `Password`, `NewPassword`, `OneTimeCode`, `Phone`; `None` to offer nothing.
- `mask()` makes the keyboard secure: no suggestions, and the keyboard is left out of a screen recording.

**A tap outside an input leaves the keyboard up**, as in the platform's apps: only another text input takes the
focus from the one being typed into. A screen of inputs gives a way to put it away, such as a list that
dismisses the keyboard when scrolled (above).

### Saved passwords

Over an input that hides what is typed (`mask()`), or says it is for a password, the keyboard offers the
person's saved logins. One that is chosen fills the input, and the other inputs of the login with it: every
text input in the tree whose `content` is `Username`, `Email`, `Password` or `NewPassword`.

```rust
TextInput::new(name).single_line().content(ContentKind::Username)
TextInput::new(password).single_line().mask().content(ContentKind::Password)
```

What the keyboard does around a saved login is the system's, and the same as for a text field of its own.
Its bar of saved logins goes for a few frames as the keyboard moves between two of a login's inputs. And when
the login comes from another app's list that had a keyboard up, a password manager's, the input is left and
the keyboard stays down; from the system's own list the keyboard comes back, with the caret after the
password.

Which logins are offered first, and whether the system offers to save a new one, depends on the app's
associated domains (`webcredentials:` in its entitlements, and the domain's `apple-app-site-association`):
that is the app's to set up, and nothing of lurq's.

## The app and the system

`ctx.window().on_app_event(..)` hears what the system says of the app. Register it in the root component's
`create`: what the app was started with is told once there is a handler.

```rust
ctx.window().on_app_event(move |event| match event {
    AppEvent::Background => save.set(true),
    AppEvent::OpenUrl(url) => opened.set(url.clone()),
    AppEvent::Rotated(orientation) => turned.set(*orientation),
    _ => {}
});
```

| `AppEvent` | |
| --- | --- |
| `Active`, `Inactive` | The app takes input, or something of the system's is over it |
| `Background`, `Foreground` | The app is no longer seen, or is coming back |
| `OpenUrl(url)` | The app was opened with a link: a scheme of its own (`CFBundleURLTypes` in `Info.plist`) |
| `Shortcut(kind)` | The app was opened by one of its home screen shortcuts |
| `Rotated(orientation)` | The way the content is turned: at the start, and whenever it changes |
| `GraphicsDeviceLost` | The window's graphics device was lost, and lurq draws it with another: on every system ([A Lost Device](../gpu-viewport/#a-lost-device)) |

### More than one window

A window an app opens with `ctx.window_opener()` is a scene of its own on iOS: a tablet shows it beside the
first, or in its place, and the person moves between them as between the windows of any app. On a phone
there is one scene, and a further window waits until a display is plugged in.

```rust
let opener = ctx.window_opener();
ctx.window().on_app_event(move |event| {
  // The person asked the system for another window, or plugged a display in.
  if let AppEvent::WindowAsked { external } = event {
    opener.open("Notes", 480, 640, move |app, tree| tree.mount_root::<Notes>(app, *external));
  }
});
```

- `AppEvent::WindowAsked` says the system has a place with no window in it: the next window the app opens is
  shown there. While the app opens none the place is empty, and a display shows what the device does.
- A window the person closes, and one on a display that is unplugged, is closed for the app too. A window
  the app closes takes its scene with it, and the first window comes back in front.
- The app's `Info.plist` allows several scenes, and names the delegate for a display's:

```xml
<key>UIApplicationSupportsMultipleScenes</key>
<true/>
<!-- in UISceneConfigurations, beside UIWindowSceneSessionRoleApplication -->
<key>UIWindowSceneSessionRoleExternalDisplayNonInteractive</key>
<array>
  <dict>
    <key>UISceneConfigurationName</key>
    <string>Display</string>
    <key>UISceneDelegateClassName</key>
    <string>LurqSceneDelegate</string>
  </dict>
</array>
```

A finger presses and scrolls in a further window as in the first. It has no keyboard: a text input in it is
not typed into. `Active`, `Background` and the rest are told of the first window alone.

### What the person set for the system

The appearance, the text size, bold text, reduced motion and the rest of what a person sets for every app are
in `ctx.window().system`, and are read again whenever the app comes back from the settings or the control
centre. The theme follows the appearance, the text size (Dynamic Type) and bold text by itself, and the stack,
the bars and the sheets follow reduced motion and transparency. See
[The System's Settings](../theme/#the-systems-settings).

### Face ID and Touch ID

`lurq::app::biometry` asks the person to prove who they are, before an app opens or shows a part of itself.

```rust
use lurq::app::biometry::{self, AuthPolicy, Biometry};

if biometry::available() != Biometry::None { /* offer the lock */ }

let unlocked = self.unlocked.clone();
biometry::authenticate("Open your notes", AuthPolicy::BiometryOrPasscode, move |answer| {
  unlocked.set(answer.is_ok());
});
```

- The system shows its own prompt. The answer comes to the callback on a thread of the system's: set a signal
  from it.
- `AuthPolicy::BiometryOrPasscode` falls back to the device's passcode; `BiometryOnly` does not.
- The error says why not: `Cancelled`, `Failed`, `Unavailable`, `LockedOut` or `Other(text)`.
- The app's `Info.plist` says why it asks, or the system ends the app at the first ask:

```xml
<key>NSFaceIDUsageDescription</key>
<string>To open your notes.</string>
```

Off a phone there is nothing to ask: `available()` is `Biometry::None` and the answer is `Unavailable` at once.

#### A secret behind the check

An ask of the app's own guards nothing by itself: what it guards is in the app, which can be made to skip the
ask. A token or a key an app logs in with is kept in the device's keychain instead, where the system asks for
the face or the finger before it gives the secret back.

```rust
use lurq::app::biometry::{self, AuthPolicy, SecretError};

biometry::keep_secret("session", token.as_bytes(), AuthPolicy::BiometryOrPasscode)?;

let session = self.session.clone();
biometry::read_secret("session", "Log you in", move |answer| match answer {
  Ok(token) => session.set(Some(token)),
  Err(SecretError::NotFound) => session.set(None), // nothing kept: ask for the password
  Err(SecretError::Auth(_)) => {}                  // not let through
});
```

- `keep_secret` asks nothing and replaces what was kept under the name. `read_secret` shows the system's
  prompt, and answers on another thread. `has_secret` and `forget_secret` ask nothing either.
- The secret stays on the device and out of its backups, and a device with no passcode keeps none
  (`AuthError::Unavailable`).
- With `AuthPolicy::BiometryOnly` the secret is for the faces or the fingers the device knows when it is kept:
  once those change it is gone (`NotFound`), and the app asks for the password again.
- In a simulator the keychain takes an app's secrets only when the entitlements Xcode makes up for a
  simulator are in the binary. A binary cargo builds has none: `crates/mobile_demo/ios/build-rust.sh` shows
  how they are linked in.

### Turning

A phone held on its side turns the app with it, with no status bar, and the safe area follows. Which ways the
app may be turned is said before the window runs, with `Orientation`s joined by `|`:

```rust
WinitWindow::new(app, tree)
    .with_orientations(Orientation::Portrait | Orientation::LandscapeLeft)
    .run();
```

`Orientation` is `Portrait`, `PortraitUpsideDown`, `LandscapeLeft` (the device's top at the left) or
`LandscapeRight`; `Orientations::LANDSCAPE` is both sides and `Orientations::ALL` every way. The app's
`Info.plist` names the ways it supports too (`UISupportedInterfaceOrientations`), and a way has to be in both.
Without `with_orientations` the app turns every way the `Info.plist` names.

### A keyboard attached to the device

Shortcuts take Command, and moving by words Option, as on a Mac. In a text input the keys are the text
system's, as the screen keyboard's are, but for Tab, which moves the focus, and Escape. Outside an input every
key reaches the tree as on a desktop.

## Frames

The shell presents from a `CADisplayLink` at the display's full rate, and only while something changes: an idle
app draws nothing. Going to the background and coming back needs nothing from the app.
