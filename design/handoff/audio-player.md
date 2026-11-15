## Changed outside the component's own files

Nothing of shared code or the engine. Lines added to the three `mod.rs` files only: `crates/lurq/src/ui/mod.rs`
(`mod audio_player;`, `mod audio_queue;` and their `pub use`), `crates/lurq/tests/ui/mod.rs` (the two test modules),
`crates/gallery/src/pages/mod.rs` (the two `mod` lines and two `Page` entries in `Group::Media`, at the end of
`PAGES`, where the group had no page yet).

## For `docs/src/content/docs/design-system.md`

## Audio Player

Sound without a picture, in four forms: a full card, one row of 48 for a list or a message, a voice message, and a
mini player docked to the bottom of a window. Every value is the app's; a press of a control tells the app what it
is to be and changes nothing by itself.

```rust
use lurq::ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat};

// The card, bound to the app's signals: a press of play sets `playing`, a drag on the bar sets `position`.
AudioPlayer::new(position.clone(), 250.0)
  .title("Night Drive")
  .description("Aurora Lane · Low Tide")
  .media(artwork)
  .playing(playing.clone())
  .volume(volume.clone())
  .muted(muted.clone())
  .rate(rate.clone())
  .shuffle(shuffle.clone())
  .repeat(repeat.clone())
  .on_previous(|| previous())
  .on_next(|| next())
  .mount(ctx);

// A voice message: the app holds where it is and seeks where the waveform is let go.
AudioPlayer::controlled(17.0, 42.0, |_| {})
  .variant(AudioPlayerVariant::Voice)
  .peaks(&peaks)
  .unheard(!heard)
  .playing_controlled(false, move |play| player.set_playing(play))
  .on_change_end(move |to| player.seek(to))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<f32>, duration)` | Bound to the position, in seconds, of something `duration` seconds long. A drag on the bar sets the signal |
| `controlled(position, duration, on_change)` | The position held by the app, which is told where a person moves it |
| `variant(AudioPlayerVariant)` | `Full` (the card, the default), `Compact`, `Voice`, `Mini` |
| `on_change(Fn(f64))`, `on_change_end(Fn(f64))` | Where a person moves the position, as a drag moves it and where it is let go, a key's step is made or the mini player's line is pressed |
| `buffered(f64)` | How far it is loaded, in seconds |
| `playing(Signal<bool>)`, `playing_controlled(bool, handler)`, `on_playing_change(Fn(bool))` | Whether it plays. A press of play, or Space with the focus on the bar or the volume, tells what it is to be |
| `volume(Signal<f32>)`, `volume_controlled(f64, handler)`, `on_volume_change(Fn(f64))` | The volume, from 0 to 1: the card and the mini player |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)`, `on_muted_change(Fn(bool))` | Whether it is muted; muting keeps the volume |
| `rate(Signal<f32>)`, `rate_controlled(f64, handler)`, `on_rate_change(Fn(f64))` | The speed: the card's speed button opens `MediaSpeedMenu`; a voice message's badge steps 1×, 1.5×, 2× (`VOICE_RATES`) while it plays |
| `shuffle(Signal<bool>)`, `shuffle_controlled(bool, handler)`, `on_shuffle_change(Fn(bool))` | Shuffle, in the card |
| `repeat(Signal<AudioRepeat>)`, `repeat_controlled(AudioRepeat, handler)`, `on_repeat_change(Fn(AudioRepeat))` | `Off`, `All`, `One`; a press steps it (`AudioRepeat::next`), and `One` has the `repeat-one` glyph |
| `loading(bool)` | Lines of skeleton for the texts, "-:--" for the times, an empty bar, a spinner in play, previous and next dimmed |
| `error(Option<&str>)`, `on_retry(Fn())`, `retry_text(&str)` | What went wrong: a line in the destructive color in place of the times (in place of the bar in a row, where play turns to a retry button; in place of the description in the mini player). "Try again" at its end in the card |
| `unheard(bool)` | A voice message not heard yet: a dot of 8 in the info color, and the whole waveform in the color of what is not played |
| `title(&str)`, `description(&str)`, `media(element)` | What plays, who it is by, and the artwork (64 in the card, 36 in the mini player); the `music` glyph on `Muted` where there is none |
| `peaks(&[f32])` | The loudness of each bar of a waveform, from 0 to 1: a voice message draws it, the compact row in place of the bar |
| `on_previous`, `on_next`, `on_more`, `on_queue`, `on_expand` | Commands; each call adds a handler. `more_anchor(ElementRef)` is the ref of the card's more button, for a menu of the app |
| `label(&str)` and `play_label`, `pause_label`, `previous_label`, `next_label`, `shuffle_label`, `repeat_label`, `more_label`, `speed_label`, `queue_label`, `expand_label`, `seek_label` | What the player and its controls are called |
| `mount(ctx)` | The tooltips of the buttons, the speed menu, the mini player's form from its own width, and the time over the mini player's line under the pointer. Converted into an element (`build(theme)`) it is the still form of itself |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the play button; `id` is the root's, and each control has it with `-play`, `-seek`, `-previous`, `-next`, `-shuffle`, `-repeat`, `-more`, `-speed`, `-volume`, `-queue`, `-expand`, `-retry`, `-rate`, `-line` after it |

- Built of the media parts: `PlayButton`, `SeekBar`, `Waveform`, `MediaTime`, `MediaVolume`, `MediaSpeedMenu`. Shuffle and repeat are `Toggle`s; previous, next, more, queue and expand ghost `Button`s; "Try again" a `Link`.
- Keys: Tab reaches each control; Space on play presses it, and with the focus on the bar or the volume plays and pauses; the arrows move the bar and the volume.
- The mini player measures itself before its first frame is painted: under 480 wide (`AUDIO_MINI_NARROW`) it keeps the track, play and next.
- Parts: `AudioPlayerPart::Root`, `Bar`, `Header`, `Media`, `MediaIcon`, `Text`, `Title`, `Description`, `Skeleton`, `More`, `Progress`, `Times`, `Elapsed`, `Total`, `Time`, `Transport`, `Shuffle`, `Repeat`, `Footer`, `Speed`, `Error`, `ErrorIcon`, `ErrorText`, `Retry`, `Rate`, `Dot`, `Track`, `Fill`, `Tools`. States: `AudioPlayerState::Playing`, `Loading`, `Error`, `Disabled`, `Media`, `Unheard`, `Shuffle`, `Narrow`, `Scrubbing`.
- In the design the card is 360 wide on `Card` edged in `Border`, `radius-xl`, padded 20 with 16 between rows; the artwork 64 with `radius-md`; the title 14/600 and who it is by 13 in `MutedForeground`; the times mono 11 6 under the bar; the transport 8 apart (shuffle 32, previous 36, play 48, next 36, repeat 32, icons 16); shuffle and repeat in `MutedForeground` off and on `Highlight` on. The compact row is 320 by 48, `radius-lg`, padded 8 and 12, 10 apart. The voice message is padded 8 12 8 8, `radius-xl`. The mini player is on `Background` with a line of `Border` on top, a progress line of 2 (4 under the pointer) in `Primary` over `Secondary`, and a row of 54 padded 12.
- To what reads the tree the player is a `group` called by its label; the error line is an `alert`; the mini player's line a `progressbar`; repeat says its state in `a11y_value`.
- A renderer that draws another player starts from `AudioPlayerView`: the controls are built (`view.controls`), and `media()`, `text()`, `time()`, `error_line()` and `line()` are the default pieces.

## For `CHANGELOG.md`

- `ui::AudioPlayer`: an audio player in four forms (`AudioPlayerVariant::Full`, `Compact`, `Voice`, `Mini`), built of the media parts, with bound and controlled forms of the position, playing, the volume, muted, the speed, shuffle and repeat (`AudioRepeat`), loading, an error with "Try again", a voice message's speed badge and unheard dot, and a mini player that keeps the track, play and next under 480 wide.

## Decided

- **One component with four variants, the queue apart.** The card, the compact row, the voice message and the mini player hold the same values to an app (one track: where it is, whether it plays, the volume, the speed...) and differ in what they show, so they are one `AudioPlayer` with `AudioPlayerVariant`; a playground offers only the props of the chosen form. The queue holds something else (a list and which one is current) and is `AudioQueue` (its own handoff).
- **The position is the first value** (`new(Signal<f32>, duration)` / `controlled(position, duration, on_change)`, with `on_change_end`), as `SeekBar` has it. Every other value is a second value named by the playbook: `playing`/`playing_controlled`/`on_playing_change`, and the same for `volume`, `muted`, `rate`, `shuffle`, `repeat`. Bound, the signal is set first and the handlers are called after; no handler is called when the app sets a signal.
- **`description`, not `artist`**, for who it is by (the playbook's word for the text under a title); **`media`** for the artwork.
- **`error(Option<&str>)`** is the text shown; the state is `Error` (the part it shows). Its command is `on_retry` with `retry_text`.
- **Mounted** (`mount(ctx)`): tooltips, the speed menu's open signal and anchor, the mini player's own width and the pointer on its line are kept by a host; a builder has no `Ctx`. Converted into an element it is still.
- **A disabled player** is taken out of input as a whole (the root node is disabled) and dimmed once on the root, so the controls are not dimmed twice.
- **Space** plays and pauses from the bar, the waveform and the volume (a handler around each that checks `focus_within`); on a button Space presses the button.
- **The compact row and the voice message's one time** read the length until it has started and what is gone after, as the board draws "0:42" not heard and "0:17" playing.
- **The mini player's error** is drawn in place of the description, in the destructive color, and play is disabled: the board draws no error for it.
- **The speed badge** of a voice message shows while it plays and steps 1×, 1.5×, 2× and back to 1×; the dot shows while it is not heard and does not play.

## Not done

- Nothing of the board is left unbuilt but what is in "Needed from the media parts" above (the play button of 36, a bar that fills its room, an empty bar with no thumb), each worked around.

## Differs from the board

- The voice message's and the mini player's play button is 40, not 36, until `PlayButton` has 36 (see above): the voice message is 56 high and not 52.
- The seek bar is 240 wide in the card and the compact row, not the row's width, until the rebuilt `SeekBar` fills its room.
- While loading and after an error the bar's thumb is drawn at its start (the board draws none).
- The mini player's progress line grows from 2 to 4 downwards, over the top of its row, so that the row does not move; the pointer finds it in the top 8 of the player.

## Needed from the media parts

Worked around until the merge; each is the smallest addition that would give it.

- **`PlayButton` of 36 with an icon of 16.** The board's voice message (`CjYWK`) and mini player (`Jp10J`) have a
  play button of 36; `PlayButtonSize` has 32, 40, 48 and 64. Add `PlayButtonSize::Ms` (or the like) of 36 with 16.
  Until then both use `Md` (40), so the voice message is 56 high and not 52.
- **`SeekBar` that fills the width it is given.** It is drawn by `ui::Slider`, whose root is 240 wide at most and
  does not grow: in the card (320 of room) and the compact row the bar is 240 and not the row's width. The
  rebuilt bar should fill its room (a root of `Length::Fill`), or take `full_width(bool)`.
- **`SeekBar` and `Waveform` empty with no thumb.** Loading and an error draw "an empty bar with no thumb". The
  player gives the bar a position of 0 and `disabled(true)`; the thumb still shows at the start. A
  `thumb(bool)` (or an `empty(bool)`) on `SeekBar` would give it.
- **`MediaTime` in mono 11 for the card and the compact row.** The card's elapsed and total and the rows' one time
  are drawn by the player itself (mono 11, `MutedForeground`), since `MediaTime` reads "elapsed / total" on one
  line in 12. Nothing to add unless `MediaTime` is to be used there: an `elapsed_only(bool)` and a size.
- **`MediaVolume::force`** reaches only its button; nothing needed, noted for the merge's tests.
- **`PlayButton::rules`/`style`.** The player cannot lay its own recipe over the play button (it has neither); the
  board needs nothing of it today.
