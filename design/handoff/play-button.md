# Play Button

`ui::PlayButton`, from the board's `Media/Play Button` (`GN7mn`) and `Media/Play Button/Overlay` (`PfeN2`) in the
section `Media` (`gr6ym`, dark `tdnDU`) of `Components / light`, with the sizes and states drawn beside it (`ILinP`) and
its uses in the Video Player (`BlxMd`, `lB6uT`) and the Audio Player (`jXwYO`, `GXAHi`, `IYjVR`, `ekK4U`, `j0G3b`).
Branch `claude/ds-media-controls-14820a`, cut from `claude/ds-media`.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`: the `pub use` of `play_button`, `media_volume` and `media_speed_menu` name the new types
  (`PlayButtonPart`, `PlayButtonRenderer`, `PlayButtonState`, `PlayButtonView`, and the same four of `MediaVolume` and
  of `MediaSpeedMenu`). rustfmt merged the base's line of each module with the added one: three lines changed, all of
  this session's modules.
- `crates/lurq/tests/ui/mod.rs`: `mod play_button;`, `mod media_volume;`, `mod media_speed_menu;`.
- `crates/gallery/src/pages/mod.rs`: the three `mod` lines and three `Page` entries in `Group::Media`, after the last
  page of the list.
- Nothing of the engine, `shared.rs`, `part_style.rs` or `recipe.rs` was changed. No keyframes were registered: the
  spinner turns by `ui::SPIN`, as `shared::glyph(.., spin: true)` has it.

## For `docs/src/content/docs/design-system.md`

## Play Button

The round control that starts and pauses what a player plays, and plays it again once it has ended.

```rust
use lurq::ui::{PlayButton, PlayButtonSize, PlayButtonVariant};

// playing: Signal<bool>. A press tells what it is to be.
PlayButton::new()
  .playing(playing.get())
  .on_change(move |next| playing.set(next));

// In the middle of a paused video, over the picture.
PlayButton::new()
  .variant(PlayButtonVariant::Overlay)
  .size(PlayButtonSize::Xl)
  .replay(ended.get())
  .on_change(move |next| play(next));
```

| Builder | |
| --- | --- |
| `playing(bool)` | Whether what it plays is playing: the glyph is `pause`. The button does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what it is to be: playing if it was paused or had ended. Not while it is loading or disabled. A second handler is called after the first |
| `replay(bool)` | What it played has ended: the glyph is `rotate-left`, and a press tells `true` |
| `loading(bool)` | What it would play is still being fetched: the `spinner` glyph turns in the glyph's place, the disc is at 75%, and a press does nothing |
| `size(PlayButtonSize)` | `Sm` 32 with a glyph of 14, `Md` 40 with 16 (the default), `Lg` 48 with 20, `Xl` 64 with 28 |
| `variant(PlayButtonVariant)` | `Primary` on a surface of the theme; `Overlay` over a picture |
| `play_label(&str)`, `pause_label(&str)`, `replay_label(&str)` | What it is called while a press would play, pause or play again: "Play", "Pause", "Replay" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the disc |

- A click, Enter and Space press it; Tab reaches it, and the ring of the focus is drawn from the keyboard.
- Parts: `PlayButtonPart::Root` (the disc, what is pressed), `Icon`. States: `PlayButtonState::Playing`, `Replay`, `Loading`, `Disabled`. Variants `PlayButtonVariant::Primary`, `Overlay`; sizes `PlayButtonSize::Sm`, `Md`, `Lg`, `Xl`.
- In the design the disc is round, in `Primary` with the glyph in `PrimaryForeground`, `PrimaryHover` under the pointer; held down it is `Primary` under the pressed veil, as the primary button is. Loading it is at 75%, disabled at half.
- Over a picture the disc is `#00000099` (black at 60%) with a white glyph and a white ring of the focus, the same in a light and a dark theme: the picture is not themed, and a themed control would lose its contrast on it. The board draws no overlay under the pointer: it is `#000000B3`, the end of the board's bar gradient. The board uses the overlay at `Xl` in a player and `Lg` in a compact one.
- To what reads the tree it is a `button` called by the label of what a press would do; loading it says `a11y_busy`.
- A renderer that draws another play button starts from `view.root()`, the behavior with the disc's style, and `view.icon()`, the glyph of the state it is in (`view.icon_name()`).

## For `CHANGELOG.md`

- `ui::PlayButton` is a component of the design system: a round primary disc in four sizes (32, 40, 48, 64) with the glyph of what a press would do (play, pause, play again, the turning spinner while loading), and an `Overlay` variant over a picture, black at 60% with a white glyph in both appearances; `PlayButtonPart`, `PlayButtonState`, `PlayButtonView` and `PlayButtonRenderer` are new.

## Decided

- The play button is its own pressable (`shared::pressable`) with a glyph, not a `ui::Button` dressed by rules as the stand-in was: a button that is an icon alone has no size of 14 or 28 for its glyph and draws a `ui::Spinner` while loading, where the board turns the `spinner` glyph.
- The overlay's ring is an outline of 2 in white, 2 out from the disc, as wide and as far out as the theme's ring: the ring's color is lost on a picture.
- The overlay under the pointer and held down is `#000000B3`: the board draws neither, and a disc that does not answer the pointer reads as broken. The literal is the board's (the bar's gradient end, decision 1 of the media design).
- Held down, the primary disc is `Primary` under the pressed veil, as the primary button's is.
- While loading, the button keeps its name of what a press would do and says `a11y_busy`; a press, Enter and Space do nothing.
- `on_change` is told `true` for a press of a button that has ended (`replay`), whatever `playing` says.

## Not done

- Nothing.

## Differs from the board

- The board draws the ring of the focus as a stroke of 2 on the disc; lurq draws the theme's ring, an outline of 2 that
  stands 2 out from the disc, as every control of the design system does (the stroke would be lost under the hover's
  fill, see the playbook's first batch).
- The Audio Player's `Voice` and `Mini` (`IYjVR`, `ekK4U`) use the play button at 36 with a glyph of 16, which is none
  of the four sizes. The audio player gives it with a rule: `.style(PlayButtonPart::Root, |s| s.square(36.0))`.
