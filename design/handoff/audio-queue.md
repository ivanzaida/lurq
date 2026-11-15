## Changed outside the component's own files

Nothing of shared code or the engine; the lines in the three `mod.rs` files are listed in `audio-player.md`.

## For `docs/src/content/docs/design-system.md`

## Audio Queue

The tracks a player plays one after another, the one it is on marked. The arrow keys move between the rows.

```rust
use lurq::ui::{AudioQueue, AudioQueueItem};

// Bound to the app's current track, which a press of a row sets.
AudioQueue::new(
  current.clone(),
  [
    AudioQueueItem::new("Night Drive").description("Aurora Lane").duration(250.0).media(artwork),
    AudioQueueItem::new("Paper Boats").description("Mira Sol").duration(232.0),
    AudioQueueItem::new("Glass Garden").description("The Fernweh").duration(301.0).disabled(true),
  ],
)
.playing(true)
.mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<usize>>, items)` | The tracks, on the one whose index the signal holds, which a press of a row sets |
| `controlled(Option<usize>, items, on_change)` | The current track held by the app, which is told the index of a row pressed |
| `on_change(Fn(usize))` | The index of a row pressed, where it is another than the current one and can be played. A second handler is called after the first |
| `playing(bool)` | Whether the current track plays: the `equalizer` glyph stands for its number |
| `item(AudioQueueItem)`, `items(..)` | Adds tracks at the end |
| `label(&str)`, `more_label(&str)`, `playing_label(&str)` | What the queue ("Queue"), a row's more button ("More") and the equalizer ("Now playing") are called |
| `mount(ctx)` | The tooltips of the rows' more buttons. Converted into an element they have none |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the row Tab would stop on |

| `AudioQueueItem` | |
| --- | --- |
| `new(&str)` | A track by its title |
| `description(&str)`, `media(element)`, `duration(f64)` | Who it is by, the artwork (cut to 32; the `music` glyph on `Muted` where there is none), and its length in seconds |
| `disabled(bool)` | It cannot be played: half the opacity, a press does nothing, the arrows pass it by |
| `on_select(Fn())` | Called on a press of the row, after the queue's `on_change` |
| `on_more(Fn())`, `more_anchor(ElementRef)` | The row's more button, there once a handler is given, and the ref a menu of the app hangs from |
| `id`, `key` | The id of the row; its more button has it with `-more` after it |

- Tab enters the queue at the current row (or the first that can be played) and stops in it once; the arrows move between the rows; Enter and Space play one. The more buttons are stops of Tab of their own, after their row, and the arrows do not go to them.
- Parts: `AudioQueuePart::Root`, `Item`, `Lead`, `Number`, `LeadIcon`, `Media`, `MediaIcon`, `Text`, `Title`, `Description`, `Duration`, `More`. States: `AudioQueueState::Current`, `Playing`, `Unavailable`, `Media` (on a row's parts) and `Disabled`.
- In the design a row is 48 high (following the density), padded 8 with 12 between its parts, `radius-md`: the number 20 wide in mono 12 in `MutedForeground`, the artwork 32 with `radius-sm`, the title 13/500 over who it is by 12, the length in mono 11, a more button of 28 with an icon of 14. The current row is on `Highlight` with its title 600; while it plays the `equalizer` 16 stands for its number. Under the pointer the row takes the hover veil, `play` 14 stands for the number and the more button shows; the focus is a ring of 2 in `Ring` inside the row. An unavailable row is at half the opacity.
- To what reads the tree the queue is a `listbox` called by its label, each row an `option` called by its title and who it is by, `a11y_selected` on the current one.

## For `CHANGELOG.md`

- `ui::AudioQueue` and `ui::AudioQueueItem`: the rows of the tracks a player plays, the current one on the accent fill with the equalizer while it plays, a more button under the pointer, unavailable tracks, and the arrow keys between the rows.

## Decided

- **A list and its rows, apart from `AudioPlayer`**: what it holds (the tracks and which one is current) is not what a player holds. The row is `AudioQueueItem` (the playbook's word for what a list holds), its part `Item`.
- **The current track is an index**, `Signal<Option<usize>>` (a queue may be on none), and `on_change(Fn(usize))` is told the index of the row pressed: a person cannot press "none". It is not called for the current row or an unavailable one.
- **`playing(bool)`** is a plain value of the queue: whether the player it goes with plays is the player's, the queue only shows it.
- **The more button** is shown under the pointer and with the focus in the row (`within`), so that the keyboard finds it; its press does not press the row.
- **One stop of Tab** (`FocusGroup::vertical().one_stop(true)`): Tab enters at the current row (or the first that can be played), the arrows move between the rows past one that cannot be played, and from a row Tab goes on to its more button, then out of the queue.
- **The play glyph for the number** shows under the pointer only (`within_hovered`), and not on the current row or an unavailable one.

## Not done

- Nothing.

## Differs from the board

- Nothing known.
