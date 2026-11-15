## Changed outside the component's own files

- `crates/lurq/src/ui/slider.rs`: `engine_part` (a part's style as the engine's slider paints a part) is `pub(super)` in place of private, so that the seek bar dresses the engine's thumb as `ui::Slider` does. Nothing else of the slider changed.
- `crates/lurq/src/ui/mod.rs`: the line `pub use seek_bar::SeekBar;` exports `SeekBarPart`, `SeekBarRenderer`, `SeekBarSize`, `SeekBarState`, `SeekBarView` and `SeekPreview` too.

## For `docs/src/content/docs/design-system.md`

## Seek Bar

Where a player is in what it plays, how much of it is loaded, and the handle that moves it; over the pointer, the time it is on.

```rust
use lurq::ui::{SeekBar, SeekBarSize};

// Bound to the app's position, in seconds, of a recording 250 seconds long.
SeekBar::new(position.clone(), 250.0)
  .buffered(loaded)
  .on_change_end(move |to| player.seek(to))
  .mount(ctx);

// Over a video, cut into chapters, with a frame of the moment under the pointer.
SeekBar::new(position.clone(), duration)
  .on_media(true)
  .chapters(&[0.0, 312.0, 840.0])
  .preview(move |time| thumbnail_at(time))
  .mount(ctx);

// The app holds the position, and is told where a person moves it.
SeekBar::controlled(at, 250.0, move |to| seek_to(to));
```

| Builder | |
| --- | --- |
| `new(Signal<f32>, duration)` | Bound to the position, in seconds, of something `duration` seconds long (0 where it is not known). A drag, a press on the track and the keys set the signal |
| `controlled(position, duration, on_change)` | The app holds the position; the bar shows it and tells `on_change` where a person moves it, and shows what the app answers |
| `buffered(seconds)` | How far it is loaded: drawn between what is played and the rest |
| `chapters(&[f64])` | Where the chapters start, in seconds: the track is cut there, 2 apart. The first starts at 0, given or not |
| `step(seconds)` | What an arrow key moves it: 5 seconds unless told. A drag is not snapped to it |
| `on_media(bool)` | It stands over a picture: white, the same in a light and a dark theme |
| `size(SeekBarSize)` | `Md` (16 high) or `Lg`, a phone's: 28 high with a thumb of 28, 16 over a picture |
| `preview(Fn(f64) -> Element)` | A picture of the moment under the pointer, given its time: shown in a frame of 128 by 72 over the time. Shown by a mounted bar |
| `on_change(Fn(f64))` | The position, in seconds, each time a person moves it. Bound, it is called after the signal is set. Each call adds |
| `on_change_end(Fn(f64))` | Where a drag is let go, a press is over or a key's step is made, where it moved. Each call adds |
| `on_hover(Fn(Option<f64>))` | The time under the pointer as it moves over a mounted bar, and `None` when it leaves. Each call adds |
| `empty(bool)` | Nothing is loaded: the track alone, with nothing played or loaded and no thumb, and it takes no input (no stop of Tab). A player draws it while it loads and after an error. It is not dimmed |
| `live(bool)` | A live stream with nothing to go back in: the bar full, the thumb at its end, and a press, a drag and the keys tell nothing (no stop of Tab). The whole may be of no length |
| `label(&str)` | What it is called, to what reads the tree: "Seek" unless told |
| `mount(ctx)` | Puts it where it goes keeping the time under the pointer and the width it is laid out at: it shows the chip with the time, and the preview over it. Converted into an element on its own it is the still form: dragged, pressed and moved by the keys, with no chip, and it reads nothing of a pointer that does not press it |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what is dragged and focused. A mounted bar forced `Hovered` shows its position in the chip |

- It is the engine's slider over a track the bar draws itself: the engine's slider takes the drag, the press on the track and the focus, and paints the thumb; the track, what is played, what is loaded and the chapters' gaps are the bar's. The track's ends are half the thumb wide, so what is played ends under the thumb's middle.
- The keys: Right and Up a step later, Left and Down a step earlier, Page Up and Page Down a tenth of the whole in whole steps, Home and End to the ends. A disabled bar is not a stop of Tab and takes nothing.
- Parts: `SeekBarPart::Root` (the bar), `Track`, `Chapter` (the room between two is its padding at the side they share), `Played`, `Buffered`, `Rest`, `Control` (the engine's slider), `Thumb`, `Hover` (what stands over the pointer), `Preview` (the frame), `Tip` (the chip with the time). States: `SeekBarState::Disabled`, `OnMedia`, `Chapters`, `Buffered`, `Preview`, `Empty`, `Live`. Sizes: `SeekBarSize::Md`, `Lg`.
- In the design (Media/Seek Bar) it is 16 high with a track of 4 whose ends are round by 2: played in `Primary`, loaded in `MutedForeground` at half its opacity, the rest in `Secondary`, and the Slider's thumb, 16, in `Background` with an edge of 1.5 in `Primary`. The ring of the focus is the thumb's. Disabled it is at half its opacity and draws nothing loaded. Over a picture (Media/Seek Bar/On Media) played and a solid thumb of 12 are `#FFFFFF`, loaded `#FFFFFF59`, the rest `#FFFFFF33`, and the ring of the focus is white: the picture is not themed. The time under the pointer (Media/Seek Preview) is in a tooltip's chip, the mono type at 12 on `Primary`, 4 above the bar and centred on the pointer; the preview's frame is 6 above the chip, with a radius md and an edge of 2 in white, and the two stand 6 above the bar.
- To what reads the tree it is a `slider` called by its label, with `a11y_value` reading "1:24 of 4:10".
- A renderer that draws another bar starts from `view.dressed(view.control())`, the behavior with the thumb's style; `view.track()` is the default track and `view.hover_layer()` what stands over the pointer.

## For `CHANGELOG.md`

- `ui::SeekBar` is built to the board: played, loaded and the rest on a track the engine's slider drags, a white form over a picture, chapters 2 apart, the time under the pointer in a chip and the app's preview over it (`mount`, `on_hover`, `preview`), keys of a step of 5 seconds, a phone's `SeekBarSize::Lg`, and the forms `empty` (nothing loaded) and `live` (a live stream with nothing to go back in).

## Decided

- `empty(true)` draws the track alone in the rest's color: the thumb is of no size (`SeekBarState::Empty`), nothing played or loaded is drawn, and the engine's slider is disabled without the bar's disabled look. `live(true)` draws all of it played with the thumb at the end, the engine's slider running from 0 to 1 at 1 so that a whole of no length still puts the thumb at the end, and disabled the same way; `a11y_value` reads "live". Both draw nothing loaded and show no chip.
- The bar grows to the room it is given: its width is the whole of what holds it and it gives way beside what keeps its own (`flex_shrink(1)`, `min_width(0)`), so in a row beside a play button and the time it takes what they leave, and in a column it is as wide as the column. Tested.
- The still form holds no element ref. One made anew each render, on the bar's root, made the runtime treat the bar as a new element at each render: the focus left it and a drag lost its end after the first change it made (the merge's failing tests). So only a mounted bar, whose ref is kept, reads the pointer: `on_hover` is a mounted bar's.
- The chip's layer is declared in the track, under the engine's slider. Declared in the root it is a point of no size at the middle of the stack, over the slider, and a pointer on that very point was on it: the bar heard the pointer leave.
- Held by the app (`controlled`), the engine's slider goes back to the app's position after each change, so the place a press is let go at is a change to it again; what was just told is not told twice. A press tells `on_change` once.

- The engine's slider is bound to a signal of its own, made anew each render at the app's position, and not to the app's signal: the engine's slider snaps and clamps what it is bound to when its range and step are set as it is built, which would write a rounded position back into the player's signal on every render. Bound, a change sets the app's signal and then tells the handlers.
- The engine's slider has a step of nothing (no snapping), and the keys are the bar's own (`on_key_down` before the engine's default): a seek bar's drag goes where the pointer is, and its arrow keys move a step of seconds. Page keys move a tenth of the whole in whole steps, as the slider's do.
- The time `on_hover` is told is to the hundredth of a second, so a move of the pointer within one tells nothing.
- The chip and the preview are a layer (an `Overlay` placed at the top of the bar, moved along it to the time, clamped to the window, not hit by the pointer), so that what holds the bar does not cut them: a player's bar has 24 above it and the preview is 106 high. Only a mounted bar shows them; the still form has nothing to keep the time under the pointer or the width in.
- The board draws the chip 4 above the bar alone and the preview with the chip 6 above it (`Media/Seek Preview` in the "Preview" example): `Hover` has a bottom padding of 4, 6 in the state `Preview`.
- The ring of the focus over a picture is white (2, offset 2), as the video player's buttons' is on the board; the board draws the seek bar's focus only on a surface.
- `SeekBarSize::Lg` is the phone's: on a surface the audio sheet's bar (28 high, a thumb of 28 edged by 1 in `Border`), over a picture the video's (16, a thumb of 16).
- A chapter's name is not shown: the stand-in's `chapters(&[f64])` has the starts only, and the board draws no name (the design's notes put it in the preview's tooltip later).

## Not done

- Not the bar's, found building it, for the engine's owner: an ancestor given a new `ElementRef` each render loses the focus and the drag of a slider in it (`Stack::new().ref_element(ElementRef::new())` around a `ui::Slider::controlled`: the second arrow key does nothing); and a layer declared in a `Stack` is hit at the point where it is declared. The bar works around both.

- The phone's thumb of 28 has no shadow (the board's `Mobile/Slider` thumb has shadow-md): the engine's slider paints a part's size, fill, corners, border and ring, not a shadow. Needs `SliderPartStyle` to take a `BoxShadow` and the layout engine to paint it under the thumb's quad; then `size(Lg, Thumb, |s| s.shadow(ShadowSize::Md))` in the recipe and `engine_part` passing it on.

## Differs from the board

- The board draws played up to the thumb's left edge and the thumb beside it; the bar draws played up to the thumb's middle, under the thumb, as the Slider does. What shows is the same.
