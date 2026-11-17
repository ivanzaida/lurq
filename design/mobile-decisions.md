# Mobile decisions

What was decided about lurq on a phone and a tablet, by whom and why, newest first: touch, gestures, the phone's
forms of components, emulating a phone on a desktop, iOS and Android. A decision is written here the day it is
taken, before the work on it starts. Everything else is in [decisions.md](decisions.md); the plan's phase 7 (mobile)
is in [design-system-plan.md](design-system-plan.md#7-phases-27--components).

Each entry: **the question**, **the decision**, **why**, and its **state** (proposed, decided, in work on a branch,
built, dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-17

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
- **State.** Waiting for Ivan's go; to follow the Pointer work on `claude/touch-form`.

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
- **State.** In work on `claude/touch-form`.

## Before 2026-11-17 (from the plan and the handoffs)

### Touch sizes are the `Touch` density

- **Decision (plan, phase 7).** A theme at `Density::Touch` has a finger's sizes (rows of 44–48, larger targets);
  `TabBar`, `NavigationBar` and `Sheet` go on recipes, their iOS look kept as a renderer rather than hard-coded.
- **Since 2026-11-17.** Density is sizes only; it does not choose a component's phone form (see the pointer above).
- **State.** The density is built; the three components on recipes are in work on `claude/legacy-styled`.

### The board's Mobile frames wait for phase 7

- **Decision (handoffs of the media batch, 2026-11-16).** The touch forms the board draws in its Mobile frames are
  built when phase 7 is; until then a component has a touch form only where it was asked for (`VideoPlayer::touch`,
  `ActionSheet`, the phone's `Lightbox` and `Picture`).
- **State.** Open. The phone's slider thumb takes no shadow (`SliderPartStyle` has none).

### Out of scope for now

- **Decision (plan, goal).** Android, iPad and tablet components, and finished native themes for every platform, are
  not in the design system's first scope; iOS is, with one iOS-looking skin as the proof that a theme can go that far.
- **Open question (plan, 12).** Does lurq ship a native-looking iOS theme itself, or leave it to apps?
