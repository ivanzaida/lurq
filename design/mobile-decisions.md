# Mobile decisions

What was decided about lurq on a phone and a tablet, by whom and why, newest first: touch, gestures, the phone's
forms of components, emulating a phone on a desktop, iOS and Android. A decision is written here the day it is
taken, before the work on it starts. Everything else is in [decisions.md](decisions.md); the plan's phase 7 (mobile)
is in [design-system-plan.md](design-system-plan.md#7-phases-27--components).

Each entry: **the question**, **the decision**, **why**, and its **state** (proposed, decided, in work on a branch,
built, dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-18

### On iOS a choice of files offers the photo library and the camera, as Safari does

- **Question.** On the iPhone the file dialog opened Files alone: a photo, which is in the photo library, could not
  be given to a file upload, and no photo could be taken for it.
- **Decision (Ivan, 2026-11-18, at the phone: "ок, а як тоді галерею відкрити?", then "так - дороблюй").** A choice of
  files to open (`FileDialog::pick`) first shows the system's menu, as Safari's file input does: Photo Library,
  Take Photo, Choose File, and Cancel. Photo Library (`PHPickerViewController`) shows pictures, videos or both, as
  the dialog's filters take them, and is there only where they take one of them or anything; Take Photo
  (`UIImagePickerController`, the camera) only where they take pictures, the device has a camera and the app says
  why it uses it (`NSCameraUsageDescription` in its Info.plist); Choose File is the document picker. What is chosen
  is copied to the app's temporary folder and given as a file with its path, as a file of Files is. The words are
  the locale's (`FieldWords::photo_library`, `take_photo`, `choose_file`, and `common.cancel`).
- **Where the files come from, told (Ivan, the same day: "як ти це робитимеш, який дев експ буде?", "а соурси як
  будуть на пк і маку працювать", then "так давай").** Nothing is asked of an app: the menu is made of what the
  filters take. An app that wants its own says the sources (`FileSource::PhotoLibrary`, `Camera`, `Files`) to the
  dialog, a file upload or a drop zone (`.sources([..])`); one source alone opens at once, with no menu. Each system
  offers of them what it has, and the dialog of files where it has none of them: on Windows the dialog of files,
  opened in the user's Pictures where the photo library is asked; on macOS the photo library of Photos
  (`PHPickerViewController`, as a sheet over the window) where it is the one source asked, the dialog of files
  otherwise. `FileSource::Camera` is a phone's alone (Ivan: "можемо просто камеру прибрати якщо таргет не мобіла?",
  "приберай"): it is not there in a build for a desktop, and an app that offers it on a phone says so under
  `#[cfg(target_os = "ios")]`. Take Photo is offered only where the app's Info.plist says why it uses the camera;
  where it does not, lurq says once in its log which key to add.
- **Why.** A person on a phone gives a picture from the photo library or the camera far more often than from Files;
  an app is written once, and runs on every system with the same calls.
- **State.** In work on `feat/design-system`.

### Files are chosen and saved on iOS through the system's picker

- **Question.** On iOS `FileDialog` gave nothing (`rfd`, the desktop's dialogs, has no iOS backend): a drop zone's
  "Choose file", a path field's Browse and Save as did nothing on the iPhone, and so did a Download that saves.
- **Decision (Ivan, 2026-11-18, at the phone: "довнлоад в лайтбоксі не працює - нам треба ще протестувати fs на
  айфоні", then "і ччуз файл не працюжє").** On iOS `FileDialog` is the system's document picker
  (`UIDocumentPickerViewController`), over the app: `pick` opens the files chosen as copies in the app's own
  temporary folder, which the app reads by their paths as on a desktop; `pick_folder` gives a folder the app may
  read and write while it runs; `pick_save` makes an empty file of its file name and has the person put it where
  they choose (Files, iCloud Drive, a folder on the phone), and gives its path there, which the app writes, as on
  a desktop. The gallery's Download in a lightbox saves the picture through `pick_save` on every system.
- **Why.** An app written once chooses and saves files on a phone as on a desktop, through the same calls.
- **State.** In work on `feat/design-system`.

### A lightbox in a page opens over the whole screen at a tap, on a phone

- **Question.** A lightbox that stands in a page (`Lightbox::inline`) stayed in its box on a phone: a tap on its
  picture did nothing, where a person expects the picture to fill the screen, as a photo in a feed does.
- **Decision (Ivan, 2026-11-18, at the phone: "а лайтбокс має відкривати картинку на фулскрін?", then "роби, і
  попереднє теж").** On a phone (the touch form), a tap on the picture of a lightbox in a page opens the same
  pictures over the whole screen, at the picture shown, in the phone's form; closing it (a drag down, Close) leaves
  the one in the page at the picture it was closed on. On a desktop nothing changes: the box is the lightbox.
- **Why.** In a box as small as a page's, a phone's picture is too small to look at; the phone's viewer is made for
  the whole screen.
- **State.** Built on `feat/design-system`; Ivan tried it on his iPhone.

### The gallery has a phone's stack over the whole screen

- **Question.** The gallery showed a stack of screens in a box of its Router page. A finger from the edge of the
  screen landed on the gallery around it, which opens its side bar, so a phone's way back by a swipe from the left
  edge, pull to refresh and a row's actions under a swipe could not be tried in the gallery (they were in
  `crates/mobile_demo` alone).
- **Decision (Ivan, 2026-11-18, at the phone: "блядь яке нахуй мобайл демо", then "роби").** The Router page has a
  button that opens a phone's stack over the whole window: a navigation bar, screens pushed on it, a list that is
  pulled to refresh, and rows swiped for their actions. Over it, a swipe from the left edge takes the screen back;
  with the stack closed it opens the side bar, as before.
- **Why.** What is tried on a phone is tried in one app.
- **State.** Built on `feat/design-system`; Ivan tried it on his iPhone.

### A tap outside the text input lets it go, and the keyboard goes down

- **Question.** On the iPhone a tap outside the text input being typed into left it the focus and the keyboard up,
  as UIKit's own text fields do: only another text input took the focus from it (`docs/ios-gaps.md`, "Putting the
  keyboard away").
- **Decision (Ivan, 2026-11-18, at the phone: "і чому коли інпут на айфоні втрачає фокус, клава не ховається?").**
  A finger that taps outside the input lets it go, as a mouse's press does and as a page in Safari does, and the
  keyboard goes down with it. A tap on what takes the focus gives it the focus. A finger that scrolls a list does
  not tap: the keyboard stays up, unless the list is set to put it away (`ScrollState::set_keyboard_dismissal`).
- **Why.** The person expects a tap on nothing to be done with typing; leaving the keyboard up hid half the
  screen with no way to put it away but a list set to.
- **State.** Built on `feat/design-system`. Seen on Windows on 2026-11-18, through the gallery's MCP touch
  (`lurq_touch`, the path a touch screen's fingers take) on the Input page: a tap on the input focuses it, a finger
  that scrolls the page 130 px leaves it the focus, and a tap on the empty stage beside it takes the focus from
  it.

## 2026-11-17

### A finger that taps while another is held clicks what it taps

- **Question.** On a phone, with three toasts up and one finger held on them so that they stay spread, a tap of
  another finger on a toast's Close did nothing. The engine has one finger as the pointer: a second one that lands
  is part of a gesture or nothing, and "a click is always one finger" ("A click is the action, a tap is the
  gesture", point 3).
- **Decision (Ivan, 2026-11-17, at the phone: "я думаю у нас пройоб в двигуні на трек більше 1 пальця").** A tap
  of two fingers is two that are down together and lift as one tap; that still clicks nothing. A finger that lands
  while another is already pressing, away from any gesture of the two (no pinch, no rotation, no scroll it joins),
  and lifts where it landed, is a tap of its own: it clicks what it is on, as a second finger presses a second
  button on a phone. The finger that was held goes on as it was: its press, its drag and what it holds are not
  touched, and nothing under the second finger is shown as pressed or hovered, since the pointer is the first
  finger's.
- **Why.** UIKit gives each finger to the view it landed on; a person holds one thing and taps another without
  thinking of it. The pointer stays one, so that a drag is never ended by another finger.
- **State.** In work on `feat/design-system`.

### Every gesture, the emulator and MCP's touch

- **Decision (Ivan: "let's add support for all the gestures, wire the emulator and MCP, and leave it in the
  macOS/iOS handover to check and adapt").**
  - **The gestures the engine has** stay as they are: tap (`fingers`, `count`), double tap, long press, pan, swipe,
    the edge pan, pinch, rotate, two-finger pan, momentum, pull to refresh, swipe row, force, the pen.
  - **The ones it lacks are added**: one-finger zoom (a double tap held and dragged, as `on_pinch`); three fingers
    swiped left and right as `EditCommand::Undo` / `Redo`, pinched in and out as `Copy` / `Paste`, tapped as the
    menu of editing commands (iOS's); in text, a long press shows a loupe and moves the caret, selection by touch
    has two handles and a menu over it (the editing commands); the bounce at a scroll's edge on iOS (checked first:
    built only if missing); a pencil's hover as hover.
  - **The emulator** as the entry "Phone emulation on a desktop" says, now decided: `LURQ_DEVICE=phone|tablet`,
    `App::emulate(Device::..)`, `--device` in the gallery; the mouse is fingers (a click a tap, a double click a
    double tap, a drag a pan or a swipe, from an edge an edge pan, held still a long press, **Alt**+drag a pinch and a
    rotation mirrored about where it began, **Alt+Shift**+drag two fingers side by side, **Alt+Shift+Ctrl**+drag
    three fingers), the fingers drawn while a modifier is held, the window a phone's size with a safe area.
  - **MCP** gets touch actions through `Tree::touch_event`: tap (`fingers`, `count`), long press, pan, swipe (a
    direction and a speed), edge pan, pinch (a scale), rotate (an angle), three-finger swipe and pinch.
  - **What only a device shows** goes into [apple-checks.md](apple-checks.md) to be checked and adapted on a Mac and
    a phone.
- **State.** Built on `claude/gestures-769083` (2026-11-17): `Tree::finger` is the one path of a finger, the
  gestures, the menu of editing commands, the loupe and the handles, a pen's hover, the emulator and `lurq_touch`.
  The bounce at a list's edge was there already (momentum carries a list past its end and it springs back,
  `UIScrollView`'s constants): nothing was built for it. What only a device shows is in
  [apple-checks.md](apple-checks.md) §5.

### A click is the action, a tap is the gesture

- **Question.** A finger's tap sends both `on_click` (the finger is the pointer) and `on_tap` (the gesture), and an
  element with both runs twice; an app with breakpoints, one code base for a phone and a desktop, should not have to
  write an `on_touch` beside its `on_click`.
- **Decision (Ivan, all six points).**
  1. `on_click` is "activate", from any input: a mouse, a finger, a pen, Enter or Space, the mouse under emulation.
     An app writes its actions there. `on_tap` is the raw gesture, from touch only.
  2. `on_tap` comes first, then `on_click`; a tap a handler takes (`prevent_default`) clicks nothing, as a long
     press that is taken already does. An element with both runs once.
  3. A tap of two fingers or more never clicks: it is `on_tap` with its `fingers`. A click is always one finger.
  4. `TapEvent` has `count` (1, 2, 3). A click never waits for a second tap, except on an element that listens for
     one (`on_tap` asking `count == 2`, or `on_double_tap`): there the single click is held back for the double
     tap's time, as a click is now on an element with `on_dblclick`. A double tap is not `on_dblclick`: on a phone it
     zooms.
  5. `MouseEvent` says what pressed: `pointer_kind: Mouse | Touch | Pen`, as the web's `pointerType`; one handler,
     told its source.
  6. A pen's tap is a click and a tap, as a finger's. A pen's hover is later.
- **Why.** One handler for the action on every platform; gestures where only a finger has them; never twice; no
  delay of every click on a phone for the sake of a double tap only some elements want.
- **What a double click did, for a finger (Ivan, the same day).** `Zoom`, `Splitter` and `DocumentTabs` listen to
  `on_double_tap` too (a held-back single tap does not matter there); `Table` and `TreeView` open a row on a single
  tap in the touch form, so a tap on a row is never delayed.
- **State.** Built (`claude/touch-form`, merged 2026-11-17).
- **Since (Ivan, 2026-11-17).** What listened only to `on_dblclick` answers a finger too: `components::Zoom`
  (`2bb08839`), `ui::Splitter` (a double tap on the handle restores the size) and `ui::DocumentTabs` (a double tap
  keeps a preview) listen to `on_double_tap` of one finger as well; `ui::Table` and `ui::TreeView` in the touch form
  (`ui::touch_form()`) open a row at a single tap, with no double tap listened for, so a tap is never held back; on a
  mouse as before. Built on `claude/gestures-769083`.

### Phone emulation on a desktop

- **Question.** How does a person on a PC run a lurq app as it would be on a phone, to try its touch forms and
  gestures?
- **Proposed, not yet decided.** One switch for any app, `LURQ_DEVICE=phone` (and `App::emulate(Device::phone())`,
  `--device phone` in the gallery): the pointer is `Coarse`, the mouse is fingers (a click a tap, a drag a pan and a
  swipe, a drag from an edge an edge pan, a held press a long press, Alt+drag a pinch and a rotation mirrored about
  where it began, Alt+Shift+drag two fingers side by side), the fingers are drawn while Alt is held, the window is
  390×844 with a safe area. MCP gets touch actions (tap, swipe, long press, pinch, rotate) through `Tree::touch_event`,
  so agents test gestures without real input. A Device panel in DevTools later, on `feat/devtools`.
- **Why.** The engine already recognizes every gesture from touch events (`app/runtime/touch_pointer.rs`); on a
  desktop nothing feeds it. A `Coarse` pointer without the mouse acting as a finger shows the phone's forms but none
  of their gestures work.
- **An app writes its handlers once.** A finger is already the pointer in lurq: a tap clicks (`on_click`), a finger's
  drag is a drag (`on_drag_*`), what a person does by touch or by mouse reaches the same handler, and the
  components that have a gesture take its desktop twin too (a context menu opens on a right click and on a long
  press; a zoom on a pinch and on the wheel). Emulation feeds the mouse into that same path as fingers, so an app
  with breakpoints, one code base for a phone and a desktop, adds no `on_touch` beside its `on_click` to be tried
  on a PC; `on_touch_*`, `on_pinch`, `on_swipe` are for what only a finger does. A finger hovers nothing, so under
  emulation nothing shows its hovered style, as on a phone.
- **Not emulated.** The on-screen keyboard, the system's gestures, more than two fingers, a phone's speed.
- **State.** Built on `claude/gestures-769083` (2026-11-17), as "Every gesture, the emulator and MCP's touch" above
  says: `App::emulate`, `Device`, `LURQ_DEVICE`, the gallery's `--device`. Alt+Shift+Ctrl is three fingers side by
  side, which swipe and tap, and with the wheel three fingers pinched about the mouse (away spreads, toward closes;
  the keys let go lift them). The DevTools Device panel is later, on `feat/devtools`.

### The pointer, not the density, decides a component's phone form

- **Question.** What makes a component take its touch form (swipes, dots for arrows, the phone's chrome of a
  lightbox or a picture)? Lightbox took it from a window narrower than `Sm`, Picture from `Density::Touch`.
- **Decision (Ivan).** A signal of its own, `lurq::app::Pointer { Fine, Coarse }`, read by one function,
  `ui::touch_form()`. It starts from the platform (iOS and Android `Coarse`, a desktop `Fine`, a browser by
  `pointer: coarse`), follows the last input by default (a touch makes it `Coarse`, a mouse `Fine`; an open lightbox
  keeps its form until it closes), and `App::set_pointer` pins it, `App::follow_pointer(false)` stops the following.
  An explicit `variant(..)` or `touch(bool)` on a component wins. **Density is about sizes only** and starts at
  `Default` on every platform. The window's width decides only a component's narrow layout.
- **Why.** Density decided two things that part: an iPad with a trackpad, a laptop with a touch screen, a dense
  phone app that wants `Compact` rows and still its swipes. Deciding the form by density, a phone app could not pick
  another density.
- **Replaces.** The first idea of the same day: `Density::Touch` decides the form and is the default on phones (a WIP
  commit `8e653322` on `claude/touch-form`).
- **State.** Built (`claude/touch-form`, merged 2026-11-17).

## Before 2026-11-17 (from the plan and the handoffs)

### Touch sizes are the `Touch` density

- **Decision (plan, phase 7).** A theme at `Density::Touch` has a finger's sizes (rows of 44–48, larger targets);
  `TabBar`, `NavigationBar` and `Sheet` go on recipes, their iOS look kept as a renderer rather than hard-coded.
- **Since 2026-11-17.** Density is sizes only; it does not choose a component's phone form (see the pointer above).
- **State.** Built: the density, and the three components on recipes (`claude/legacy-styled`, merged 2026-11-17).

### The board's Mobile frames wait for phase 7

- **Decision (handoffs of the media batch, 2026-11-16).** The touch forms the board draws in its Mobile frames are
  built when phase 7 is; until then a component has a touch form only where it was asked for (`VideoPlayer::touch`,
  `ActionSheet`, the phone's `Lightbox` and `Picture`).
- **State.** Open. The phone's slider thumb takes no shadow (`SliderPartStyle` has none).

### Out of scope for now

- **Decision (plan, goal).** Android, iPad and tablet components, and finished native themes for every platform, are
  not in the design system's first scope; iOS is, with one iOS-looking skin as the proof that a theme can go that far.
- **Open question (plan, 12).** Does lurq ship a native-looking iOS theme itself, or leave it to apps?
