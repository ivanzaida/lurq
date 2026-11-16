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
| `source(&Source)` (feature `media`) | What it plays: a file, bytes, a resource of the app, an address. The player opens it as a sound and plays it; where it is, how long it is and whether it plays, loads or failed are then the sound's, and its controls move it. An address is loaded when it is first played, once, and kept (`media::sound_cache()`). See `media-playback.md` |
| `locale(&Locale)` | The words this player speaks in, in place of the app's (`App::set_locale`): the names of its controls, "Try again", and what its source says when it cannot play. Said before the single words (`play_label`, `retry_text`), which stay what they were told |
| `loading(bool)` | Lines of skeleton for the texts, "-:--" for the times, an empty bar, a spinner in play, previous and next dimmed |
| `error(Option<&str>)`, `on_retry(Fn())`, `retry_text(&str)` | What went wrong: a line in the destructive color in place of the times (in place of the bar in a row, where play turns to a retry button; in place of the description in the mini player). "Try again" at its end in the card |
| `unheard(bool)` | A voice message not heard yet: a dot of 8 in the info color, and the whole waveform in the color of what is not played |
| `title(&str)`, `description(&str)`, `media(element)` | What plays, who it is by, and the artwork (64 in the card, 36 in the mini player); the `music` glyph on `Muted` where there is none |
| `peaks(&[f32])` | The loudness of each bar of a waveform, from 0 to 1: a voice message draws it, the compact row in place of the bar |
| `on_previous`, `on_next`, `on_more`, `on_queue`, `on_expand` | Commands; each call adds a handler. `more_anchor(ElementRef)` is the ref of the card's more button, for a menu of the app |
| `label(&str)` and `play_label`, `pause_label`, `previous_label`, `next_label`, `shuffle_label`, `repeat_label`, `more_label`, `speed_label`, `queue_label`, `expand_label`, `seek_label` | What the player and its controls are called |
| `mount(ctx)` | The tooltips of the buttons, the speed menu, the mini player's form from its own width, and the time over the mini player's line under the pointer. `.child(player)` is the same component, declared: what renders the tree mounts it |
| `build(theme)` | A still picture for a test or a screenshot: no tooltips, no speed menu, a mini player as wide as it is given |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the play button; `id` is the root's, and each control has it with `-play`, `-seek`, `-previous`, `-next`, `-shuffle`, `-repeat`, `-more`, `-speed`, `-volume`, `-queue`, `-expand`, `-retry`, `-rate`, `-line` after it |

- Built of the media parts: `PlayButton`, `SeekBar`, `Waveform`, `MediaTime`, `MediaVolume`, `MediaSpeedMenu`. Shuffle and repeat are `Toggle`s; previous, next, more, queue and expand ghost `Button`s; "Try again" a `Link`.
- Keys: Tab reaches each control; Space on play presses it, and with the focus on the bar or the volume plays and pauses; the arrows move the bar and the volume.
- The mini player measures itself before its first frame is painted: under 480 wide (`AUDIO_MINI_NARROW`) it keeps the track, play and next.
- Parts: `AudioPlayerPart::Root`, `Bar`, `Edge`, `Header`, `Media`, `MediaIcon`, `Text`, `Title`, `Description`, `Skeleton`, `More`, `Progress`, `Times`, `Elapsed`, `Total`, `Time`, `Transport`, `Shuffle`, `Repeat`, `Footer`, `Speed`, `Error`, `ErrorIcon`, `ErrorText`, `Retry`, `Rate`, `Dot`, `Track`, `Fill`, `Tools`. States: `AudioPlayerState::Playing`, `Loading`, `Error`, `Disabled`, `Media`, `Unheard`, `Shuffle`, `Narrow`, `Scrubbing`.
- In the design the card is 360 wide on `Card` edged in `Border`, `radius-xl`, padded 20 with 16 between rows; the artwork 64 with `radius-md`; the title 14/600 and who it is by 13 in `MutedForeground`; the times mono 11 6 under the bar; the transport 8 apart (shuffle 32, previous 36, play 48, next 36, repeat 32, icons 16); shuffle and repeat in `MutedForeground` off and on `Highlight` on. The compact row is 320 by 48, `radius-lg`, padded 8 and 12, 10 apart. The voice message is padded 8 12 8 8, `radius-xl`. The mini player is on `Background` with a line of `Border` on top, under a progress line of 2 (4 under the pointer) in `Primary` over `Secondary`, and a row of 54 padded 12.
- To what reads the tree the player is a `group` called by its label; the error line is an `alert`; the mini player's line a `progressbar`; repeat says its state in `a11y_value`.
- A renderer that draws another player starts from `AudioPlayerView`: the controls are built (`view.controls`), and `media()`, `text()`, `time()`, `error_line()` and `line()` are the default pieces.

## For `CHANGELOG.md`

- `ui::AudioPlayer`: an audio player in four forms (`AudioPlayerVariant::Full`, `Compact`, `Voice`, `Mini`), built of the media parts, with bound and controlled forms of the position, playing, the volume, muted, the speed, shuffle and repeat (`AudioRepeat`), loading, an error with "Try again", a voice message's speed badge and unheard dot, and a mini player that keeps the track, play and next under 480 wide.

## Decided

- **The width of the recipe is the most a player takes.** A card (360), a row (320) and a voice message are no wider than what holds them, as the design system's controls are since the narrow pass: in a window as narrow as a phone the card's lines of a loading track are no longer than the room beside the artwork, and a row's or a message's waveform closes its gaps (`design/handoff/waveform.md`). Checked in the gallery at 390 and 320 (`cargo test -p gallery narrow`) and in `tests/ui/narrow_room.rs`.
- **One component with four variants, the queue apart.** The card, the compact row, the voice message and the mini player hold the same values to an app (one track: where it is, whether it plays, the volume, the speed...) and differ in what they show, so they are one `AudioPlayer` with `AudioPlayerVariant`; a playground offers only the props of the chosen form. The queue holds something else (a list and which one is current) and is `AudioQueue` (its own handoff).
- **The position is the first value** (`new(Signal<f32>, duration)` / `controlled(position, duration, on_change)`, with `on_change_end`), as `SeekBar` has it. Every other value is a second value named by the playbook: `playing`/`playing_controlled`/`on_playing_change`, and the same for `volume`, `muted`, `rate`, `shuffle`, `repeat`. Bound, the signal is set first and the handlers are called after; no handler is called when the app sets a signal.
- **`description`, not `artist`**, for who it is by (the playbook's word for the text under a title); **`media`** for the artwork.
- **`error(Option<&str>)`** is the text shown; the state is `Error` (the part it shows). Its command is `on_retry` with `retry_text`.
- **Mounted** (`mount(ctx)`): tooltips, the speed menu's open signal and anchor, the mini player's own width and the pointer on its line are kept by a host; a builder has no `Ctx`.
- **A component by itself again, and now a kept one** (Ivan, 2026-11-16, after the line under this one). `AudioPlayer` implements `app::component::IntoComponent`, and the engine converts whatever does into an element that declares its component (`Element::component`): `.child(AudioPlayer::..)` compiles again, and is the host `mount(ctx)` mounts, mounted by the component that renders the tree once its render has returned. So the form without a `Ctx` keeps what the mounted one keeps: a mini player folds to its width and its buttons have their tooltips (tested). `mount(ctx)` stays, for mounting at once; `build(theme)` stays the still picture. What the line under this one says of `.child(..)` not compiling is no longer so; its reason is what this form answers.
- **Mounted, the only way** (Ivan, 2026-11-16). `AudioPlayer` is not an element by itself: there is no `From<AudioPlayer> for Element`, and `.child(AudioPlayer::..)` does not compile; an app writes `.mount(ctx)`. The form that became an element without a `Ctx` kept nothing between renders, and was the one in which a mini player did not fold to its width and no button had a tooltip. `build(theme)` stays as a still picture for a test or a screenshot, said by name.
- **A disabled player** is taken out of input as a whole (the root node is disabled) and dimmed once on the root, so the controls are not dimmed twice.
- **Space** plays and pauses from the bar, the waveform and the volume (a handler around each that checks `focus_within`); on a button Space presses the button.
- **The compact row and the voice message's one time** read the length until it has started and what is gone after, as the board draws "0:42" not heard and "0:17" playing.
- **The mini player's error** is drawn in place of the description, in the destructive color, and play is disabled: the board draws no error for it.
- **The voice message's and the mini player's play button** is `PlayButtonSize::Md` styled to the board's 36 (its glyph is 16).
- **The speed badge** of a voice message shows while it plays and steps 1×, 1.5×, 2× and back to 1×; the dot shows while it is not heard and does not play.
- **The mini player's end folds with its width**, so that nothing of it is ever cut (set at the merge: at 655 the time read "24 / 4:10"). From 690 (`AUDIO_MINI_VOLUME_FROM`) it has everything the board's 720 has; under it the volume is its button; under 560 (`AUDIO_MINI_TIME_FROM`) the time goes; under 480 (`AUDIO_MINI_NARROW`) it is the track, play and next. The board draws only the first and the last; the two steps between follow from what the end needs (264 with the slider, 188 without, 100 without the time) beside a transport of 108.
- **Loading and after an error** the bar is `SeekBar::empty(true)` (the track alone, no thumb, no input) and a waveform has every bar in the color of what is not played.

## Not done

- Nothing.

## Differs from the board

- The mini player's progress line grows from 2 to 4 downwards, over the top of its row, so that the row does not move; the pointer finds it in the top 8 of the player. The line of the border along its top edge is under the progress line (part `Edge`), which lies over it.

## Needed from the media parts

What the first pass asked for, now that the shared parts are real:

- **A play button of 36 with a glyph of 16** (the voice message, the mini player): done with `PlayButton::style(PlayButtonPart::Root, |s| s.square(36.0))` on `Md`. The voice message is 52 high; tested.
- **A seek bar that fills its room**: the rebuilt `SeekBar` does; the card's bar is 320 (the card's room) and the compact row's fills between the button and the time, 10 from each; tested.
- **An empty bar with no thumb**: `SeekBar::empty(bool)`, used while loading and after an error.
- **`MediaTime`**: the mini player uses it at its default size (`Md`, mono 12), as the board's `Media/Time` instance there has it. The card's two ends and the rows' one time are not an "elapsed / total" reading and stay the player's own (mono 11).
