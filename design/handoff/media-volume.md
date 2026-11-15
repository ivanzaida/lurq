# Volume

`ui::MediaVolume`, from the board's `Media/Volume` (`hRRVr`) in the section `Media` (`gr6ym`, dark `tdnDU`) of
`Components / light`, with the low, muted and collapsed forms drawn beside it (`ES9D2`), and its uses in the Video
Player's bar (`fIhZR`, over the picture) and the Audio Player (`z13N9g`, `V8KZcO`). Branch
`claude/ds-media-controls-14820a`, cut from `claude/ds-media`.

## Changed outside the component's own files

- The lines of the three `mod.rs` files, named in `play-button.md`. Nothing else.

## For `docs/src/content/docs/design-system.md`

## Volume

A button that mutes and a slider of how loud, in a player's bar. Muting keeps the volume, so that the sound comes back as loud as it was.

```rust
use lurq::ui::{MediaVolume, MediaVolumeSize};

// volume: Signal<f32> from 0 to 1, muted: Signal<bool>.
MediaVolume::new(volume.clone()).muted(muted.clone());

// The app holds both, and is told what a person makes them.
MediaVolume::controlled(player.volume(), move |volume| player.set_volume(volume))
  .muted_controlled(player.muted(), move |muted| player.set_muted(muted));

// In a video's bar, over the picture; in a narrow one, the button alone until the pointer or the focus is in it.
MediaVolume::new(volume.clone()).muted(muted.clone()).on_media(true).size(MediaVolumeSize::Lg).length(64.0);
MediaVolume::new(volume.clone()).muted(muted.clone()).on_media(true).collapsed(true).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<f32>)` | Bound to the app's volume, from 0 to 1, which a drag, a press on the slider and the keys set |
| `controlled(volume, on_change)` | The app holds the volume and is told what a person makes it |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)` | Muted while true, which a press of the button turns. Muted, the slider stands at nothing and the volume is kept |
| `on_change(handler)` | Called with the volume each time a person moves it. Bound, after the signal is set |
| `on_change_end(handler)` | Called with the volume when a drag is let go or a key's step is made |
| `on_muted_change(handler)` | Called with whether it is to be muted: on a press of the button, and with `false` when the slider is moved up from nothing while muted |
| `collapsed(bool)` | The button alone, the slider beside it while the pointer is over the control, the focus is in it or it is being dragged. What it keeps for that is `mount(ctx)`'s; a still picture (`build(theme)`) is the button alone |
| `on_media(bool)` | Over a picture: white, the same in a light and a dark theme |
| `size(MediaVolumeSize)` | `Md`: a button of 32 with a glyph of 16 (the default); `Lg`: 36 with 20, as a video's bar has it |
| `length(f32)` | How long the slider is, in place of the recipe's 72 |
| `label(&str)` | What the slider is called: "Volume" |
| `mute_label(&str)`, `unmute_label(&str)` | What the button is called while a press would mute or bring the sound back: "Mute", "Unmute" |
| `mount(ctx)` | Puts it in the tree of `ctx`'s component, keeping whether the pointer is over it, the focus is in it and its slider is being dragged |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` and `force` are the button's |

- The glyph is `volume` from a half up, `volume-low` under it, and `volume-off` while muted or at nothing. Muted, the slider stands at nothing and the app's volume is not touched; moving the slider up from there sets the volume and tells `on_muted_change(false)` once in the gesture.
- Tab reaches the button, then the slider: Space and Enter press the button, the arrows move the slider a twentieth, Page Up and Page Down a tenth, Home and End to nothing and to the whole. A disabled control is passed by.
- Parts: `MediaVolumePart::Root` (the button and the slider, side by side), `Button` and `Icon` (laid over a [button](#button)'s `Root` and `Icon`), `Slider`, `Track`, `Fill`, `Thumb` (laid over a [slider](#slider)'s parts). States: `MediaVolumeState::Muted`, `Collapsed`, `OnMedia`, `Disabled`. Sizes `MediaVolumeSize::Md`, `Lg`.
- In the design the button is a ghost one of 32 with a glyph of 16 and the slider is 72 long, 4 apart. Over a picture the glyph is white, the button `#FFFFFF1F` under the pointer and `#FFFFFF33` held down with a white ring of the focus; the slider's range and thumb are white, the thumb 12 with no edge, and its rail is `#FFFFFF33`.
- To what reads the tree the button is a `button` called by what a press would do, and the slider a `slider` called by `label`.
- A renderer that draws another volume control starts from `view.button()` and `view.slider()`, the button and the slider dressed by the recipe with their behavior; `view.revealed` says whether the slider is shown.

## For `CHANGELOG.md`

- `ui::MediaVolume` is a component of the design system: a ghost button that mutes and a slider 72 long, the glyph of the level (loud, low, crossed), muting that keeps the volume and a slider moved up from nothing that turns the sound back on, a collapsed form whose slider comes under the pointer and with the focus (`mount(ctx)`), and a white form over a picture (`on_media`); `MediaVolumePart`, `MediaVolumeState`, `MediaVolumeView` and `MediaVolumeRenderer` are new.

## Decided

- **Mounted, the only way** (Ivan, 2026-11-16). `MediaVolume` is not an element by itself: there is no `From<MediaVolume> for Element`, and `.child(MediaVolume::..)` does not compile; an app writes `.mount(ctx)`. The form that became an element without a `Ctx` kept nothing between renders, and was the one in which a collapsed control never showed its slider. `build(theme)` stays as a still picture for a test or a screenshot, said by name.
- The slider is always a controlled `ui::Slider` showing the volume, or nothing while muted. Bound, a person's move sets the app's signal; muted, the app's volume is not set to nothing (the stand-in did).
- `on_muted_change(false)` is told once in a gesture that moves the slider up from nothing while muted: the handlers of a drag hear many values before the app renders the control unmuted.
- A button at nothing that is not muted is still called "Mute" and its press mutes; its glyph is crossed, since there is no sound.
- Collapsed, what asks for the slider is the pointer over the whole control, the focus on its button or its slider from the pointer or the keyboard, and a drag of the slider (so that a drag that leaves the control does not take the slider from under the pointer). The slider comes and goes at once: the board draws no motion.
- The button and the slider take the recipe's parts through `rules_for`, as `ui::ActionBar` dresses its buttons: the sizes of the button and its glyph, the slider's length and the on-media colors are all in this recipe.
- On media, the button's pressed veil is cleared: the board has white at 20% alone, which the theme's veil would darken to grey.

## Not done

- Nothing.

## Differs from the board

- The ring of the focus is the theme's (an outline of 2, 2 out), and white over a picture: the board draws no focused volume control.
- The board's Audio Player gives the slider 96 (`z13N9g`) and the video's bar 64 (`fIhZR`): the players say so with `length(..)`; 72 is the recipe's.
