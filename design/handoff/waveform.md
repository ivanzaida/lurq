## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`: the line `pub use waveform::Waveform;` exports `WaveformPart`, `WaveformRenderer`, `WaveformState` and `WaveformView` too.
- The waveform's drag and keys are the seek bar's (`seek_control` in `ui/seek_bar.rs`, `pub(super)`), one session's two files.

## For `docs/src/content/docs/design-system.md`

## Waveform

The loudness of a recording as a row of bars, the played part of it marked: a voice message's seek bar.

```rust
use lurq::ui::Waveform;

// Shown and not operated, a third of it played.
Waveform::at(0.33, &peaks);

// Bound to how far it is played, from 0 to 1, which a press, a drag and the keys set.
Waveform::new(progress.clone(), &peaks).on_change_end(move |to| player.seek(to * length));

// The app holds it, and is told where a person presses or drags to.
Waveform::controlled(at, &peaks, move |to| seek_to(to));

// Not heard yet: every bar in the color of what is not played.
Waveform::at(0.0, &peaks).unheard(true);
```

| Builder | |
| --- | --- |
| `at(progress, &[f32])` | Shown and not operated: `progress` of it played, from 0 to 1; no stop of Tab, and a press does nothing |
| `new(Signal<f32>, &[f32])` | Bound to how far it is played, from 0 to 1, which a press, a drag and the keys set |
| `controlled(progress, &[f32], on_change)` | The app holds how far it is; the waveform shows it and tells `on_change` where a person presses or drags to |
| `unheard(bool)` | Nothing of it was heard yet: every bar in the color of what is not played |
| `step(f64)` | What an arrow key moves it, from 0 to 1: a twentieth unless told |
| `on_change(Fn(f64))` | Where a person moves it, from 0 to 1, as a press or a drag moves it. Bound, it is called after the signal is set. Each call adds |
| `on_change_end(Fn(f64))` | Where it ends: the drag let go, the press or a key's step made, where it moved. Each call adds |
| `label(&str)` | What it is called, to what reads the tree: "Seek" unless told |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what is pressed where it is operated; `force` shows only on one that is operated |

- `peaks` are the heights of its bars, each from 0 to 1, one for each bar: about 36 for a voice message. How many are played is `progress` of them, rounded.
- Operated it is the engine's slider over the bars, as large as they are and drawing nothing but the ring of the focus: the arrow keys move it a step, Page Up and Page Down a tenth, Home and End to the ends. A disabled one is not a stop of Tab and takes nothing.
- Parts: `WaveformPart::Root`, `Bars` (the row), `Bar` (one not played), `Played`, `Control` (the engine's slider). States: `WaveformState::Disabled`, `Unheard`, `Operated`.
- In the design (Media/Waveform) the bars are 3 wide and 2 apart, fully round, at most 24 high and no lower than they are wide; the played ones in `Primary`, the rest in `Input`. Operated, the ring of the focus is around the bars at a radius sm; disabled, it is at half its opacity.
- To what reads the tree an operated waveform is a `slider` called by its label, with `a11y_value` the percent played.

## For `CHANGELOG.md`

- `ui::Waveform` is built to the board and is a seek surface: `Waveform::new(Signal<f32>, peaks)` beside `controlled` and `at`, a press, a drag and the keys telling `on_change` and `on_change_end` a fraction, `step`, the ring of the focus and a disabled state.

## Decided

- **In less room than its bars take, a waveform keeps its bars and closes its gaps.** Its width (the bars and the recipe's gap between them) is the most it takes, not the least: in a row narrower than that (a voice message in a window as narrow as a phone) every bar is still drawn at its width, spread from the start of the room to its end, and what is pressed is as wide as the bars, so a press still seeks to the share of the whole under it. At its own width nothing changes. It does not drop or merge peaks: how many bars there are is the app's (`tests/ui/narrow_room.rs`).
- The arrow keys move a twentieth (`step`, from 0 to 1): a waveform knows no seconds, and a player that wants 5 seconds gives `step(5.0 / length)`.
- A bar no taller than its peak would make it is no lower than it is wide (3), as the board's least bar is a dot and never nothing; the board's lowest is 6.
- The ring of the focus is around the whole waveform (the board draws none): the waveform is one control.

## Not done

## Differs from the board
