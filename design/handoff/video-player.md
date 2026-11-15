## For `docs/src/content/docs/design-system.md`

## Video Player

A picture the app gives, the controls over it, and a layer of the app's own over the picture: an ordinary player, or only the picture with the app's own controls, as in a vertical feed. Nothing is decoded here: the app draws its frame as an element and holds every value of the player; the player tells it what a person asks for.

```rust
use lurq::ui::{VideoPlayer, VideoPlayerSize};

// Bound to the app's signals: a press sets the signal, and then the handlers are told.
VideoPlayer::new()
  .media(frame)
  .duration(250.0)
  .buffered(140.0)
  .playing(playing.clone())
  .position(position.clone())
  .volume(volume.clone())
  .muted(muted.clone())
  .on_position_change_end(move |to| decoder.seek(to))
  .mount(ctx);

// Held by the app: a press changes nothing until the app answers by setting the prop.
VideoPlayer::new()
  .media(frame)
  .duration(250.0)
  .playing_controlled(is_playing, move |play| decoder.set_playing(play))
  .position_controlled(at, move |to| decoder.seek(to));

// The picture alone, with the app's own controls over it.
VideoPlayer::new().media(frame).controls(false).child(my_controls).mount(ctx);
```

| Builder | |
| --- | --- |
| `new()` | A player of nothing yet: a black picture, paused at the start, of no known length |
| `media(element)` | What is drawn as the picture, filling the player |
| `poster(element)` | Drawn over the picture before the first play (not playing, at the start, not waiting, ended or failed), with the length in a chip at its bottom end and no bar |
| `child(element)`, `children(..)` | The app's own layer over the whole picture, under the middle and the bar, with the controls and without them |
| `controls(bool)` | The bar and the middle of the picture; on by default. Without them the player is its picture and the app's layer |
| `playing(Signal<bool>)`, `playing_controlled(bool, handler)`, `on_playing_change(handler)` | Whether it plays. Play, the disc in the middle, a press of the picture, Space and K tell what it is to be |
| `position(Signal<f32>)`, `position_controlled(f64, handler)`, `on_position_change(handler)`, `on_position_change_end(handler)` | Where it is, in seconds. `on_position_change` is told as the seek bar's handle moves, `on_position_change_end` where it is let go; a key that seeks tells both |
| `duration(f64)`, `buffered(f64)`, `chapters(&[f64])` | How long it is (until said, the time reads `-:-- / -:--`), how far it is loaded, and where its chapters start |
| `volume(Signal<f32>)`, `volume_controlled(f64, handler)`, `on_volume_change(handler)`, `on_volume_change_end(handler)` | How loud, from 0 to 1 |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)`, `on_muted_change(handler)` | Whether the sound is off; the volume is kept |
| `rate(Signal<f32>)`, `rate_controlled(f64, handler)`, `on_rate_change(handler)`, `rates(&[f32])` | How fast it plays, and the speeds offered (`MEDIA_RATES` unless told) |
| `captions(Signal<bool>)`, `captions_controlled(bool, handler)`, `on_captions_change(handler)` | Whether the captions show |
| `caption_text(&str)`, `captions_track(&str)` | The line to show now, and the name of the track the settings say ("English") |
| `qualities(&[&str])`, `quality(Signal<Option<usize>>)`, `quality_controlled(..)`, `on_quality_change(handler)` | The qualities offered after "Auto", and the one it plays in: `None` for the one the player picks |
| `fullscreen(Signal<bool>)`, `fullscreen_controlled(..)`, `on_fullscreen_change(handler)` | Whether it fills the screen; its button then exits. The app makes it so |
| `picture_in_picture(Signal<bool>)`, `picture_in_picture_controlled(..)`, `on_picture_in_picture_change(handler)` | Whether it plays in a small window of its own |
| `waiting(bool)` | It waits for what it plays: a spinner in the middle, and the bar stays |
| `ended(bool)` | It played to the end: the veil, replay in the middle and in the bar, the seek bar full. Replay tells `playing` true: the app plays again from the start |
| `live(bool)` | No end: the LIVE badge for the time, the seek bar full (or the window that can be gone back in, given as the duration), no speed |
| `error(&str)`, `error_description(&str)`, `on_retry(handler)` | That it cannot play: the veil, the title and the line in the middle, and no bar. The button ("Try again") is drawn once `on_retry` is given |
| `size(VideoPlayerSize)` | `Lg`, `Md` (compact) or `Sm` (minimal) in place of the form its width gives it |
| `width(Length)`, `height(Length)` | Its size, in place of its form's (640 by 360 for the large one). A width with no height gives a height of 16 by 9 |
| `force_menu(Option<VideoPlayerMenu>)`, `force_controls_shown(Option<bool>)` | A menu drawn open, or the bar shown or faded, with nothing pointing at it: for a gallery or a test |
| `label`, `play_label`, `pause_label`, `replay_label`, `mute_label`, `unmute_label`, `volume_label`, `seek_label`, `captions_label`, `speed_label`, `settings_label`, `quality_label`, `picture_in_picture_label`, `fullscreen_label`, `exit_fullscreen_label`, `more_label`, `retry_label`, `speed_heading_text`, `auto_text`, `on_text`, `off_text`, `live_text` | Every word it says and every name it gives a control |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the buttons of the bar as under the pointer or focused. The controls have ids after the player's: `{id}-play`, `{id}-seek`, `{id}-mute`, `{id}-time`, `{id}-captions`, `{id}-speed`, `{id}-settings`, `{id}-pip`, `{id}-fullscreen`, `{id}-more`, `{id}-center`, `{id}-retry`, `{id}-controls`, `{id}-caption`, `{id}-overlay`, and the menus' panels `{id}-settings-menu`, `{id}-speed-menu`, `{id}-more-menu` |
| `mount(ctx)` | Keeps its width, the timer that fades the bar, its menus and its tooltips. Converted into an element on its own the player is its still form: the bar shown, the large form unless told, its menus not opened, no tooltips |

- A press on a control tells the app what the value is to be and changes nothing by itself: bound, the signal is set first; held, the app answers with the next render. No handler is called when the app sets a value, nor for a choice of what is already chosen.
- **Forms.** The large form has the seek bar over a row of play, the volume (its slider folds into its button under 480), the time, then captions, the speed ("1×"), settings, picture in picture and fullscreen. The compact form, under 400 wide, has play, the time, mute, a "more" menu of captions, speed, quality and picture in picture, and fullscreen. The minimal form, under 280, is one row: play, the seek bar and the elapsed time. Mounted, the form follows the width it is laid out at from the first frame (`ElementRef::observe_rect_before_paint`); `size` says another. `VideoPlayerSize::for_width` gives the form of a width, and `VIDEO_PLAYER_COMPACT_BELOW`, `VIDEO_PLAYER_MINIMAL_BELOW` and `VIDEO_PLAYER_VOLUME_FOLDS_BELOW` are the widths.
- **Layers**, bottom to top: the picture (and the poster), the veil, the app's layer, the middle (the 64 overlay play button while paused, 48 in the compact form; a spinner while it waits; replay when it ended; the error), the captions' line, the poster's length, and the bar on its gradient. A press of the picture plays and pauses, a double press asks for fullscreen; the app's layer and the middle let a press through where they hold nothing.
- **Hiding.** While it plays the bar fades out `VIDEO_PLAYER_HIDE_AFTER` (3 s) after the pointer rests, over the slow duration, and comes back on a move of the pointer, on pause, while the keyboard's focus is in the player and while a menu is open, over the base one. Paused, waiting, ended and failed keep it. The captions' line moves down to 12 over the bottom while the bar is hidden.
- **Keys**, while the focus is in the player: Space and K play and pause, Left and Right seek 5 s (`VIDEO_PLAYER_SEEK_STEP`), J and L 10 s, Home and End go to the start and the end, Up and Down move the volume by 0.05, M mutes, C turns the captions, F asks for fullscreen. Space and the arrows are the controls' own while the keyboard's focus is on one of them. A press of the picture gives the player the focus.
- **Menus.** Settings holds Quality (when `qualities` are given), Speed (not live) and Captions, each with its value at its end and its choices beside it; the speed button opens the speeds; the compact form's "more" holds captions, speed, quality and picture in picture. They hang over the seek bar, their end on the bar's end, and stay the theme's.
- Everything over the picture is white on black at the board's alphas, the same in a light and a dark theme: the picture is not themed, so themed controls would lose their contrast on it. The literals are in the recipe. The menus, the tooltips and the live badge are the theme's.
- Every control has a tooltip with its key (mounted) and a name to what reads the tree; the toggles say `a11y_pressed`, the menu buttons `a11y_haspopup` and `a11y_expanded`. The player is a `group` called by its label.
- Parts: `VideoPlayerPart::Root`, `Media`, `Veil`, `Overlay`, `Center`, `Spinner`, `Error`, `ErrorIcon`, `Text`, `Title`, `Description`, `Retry`, `Captions`, `Caption`, `Chip`, `ChipValue`, `Controls` (its background is the color its gradient ends in), `Row`, `Control`, `ControlIcon`, `Speed`, `Time`, `Elapsed`, `Menus` (its gap is how far a menu stands over the row), `SettingsMenu`, `SpeedMenu`, `MoreMenu`, `MenuValue`. States: `VideoPlayerState::Playing`, `Waiting`, `Ended`, `Error`, `Live`, `Muted`, `Captions`, `Fullscreen`, `Poster`, `Bare`, `Hidden`, `Narrow`, and `On`, resolved for a button of the bar that is on. Sizes: `VideoPlayerSize::Sm`, `Md`, `Lg`.
- In the design (`Video Player` `yKk3P`, `Compact` `cak7v`, `Minimal` `OUkUD`) the large player is 640 by 360 with the large radius. Its bar is 88 high, padded 24 at the top, 12 at the sides and 8 at the bottom, 4 between the seek bar and the row, on a gradient from nothing to `#000000B3`; the row is 36 high, 4 between buttons of 36 with white glyphs of 20, the middle radius, `#FFFFFF1F` under the pointer, `#FFFFFF33` while on, and a white ring of 2 from the keyboard. The compact bar is 72 high, padded 20 8 6, a row of 28 with 2 between buttons of 28 and glyphs of 16; the minimal one is a row 48 high padded 16 8 4 4, 6 apart. The captions' line is 12 over the bar and 24 from the sides, 16/500 white with a line of 1.4 in a box of `#000000B3` padded 2 8. The poster's length is mono 11 in a box of `#000000B3` padded 2 6, 12 from the bottom and the end. The veil is `#00000099`. The error is a white glyph of 24, a title 14/500, a line 13 in `#FFFFFF99`, and a white button 32 high.
- A renderer that draws another player starts from the view: `view.root()` (the box with its behavior), `media()`, `veil()`, `overlay()`, `center()`, `captions()`, `chip()`, `bar()`, or `view.bar` for the controls made (`VideoPlayerBar`: the seek bar, play, the volume, the time, the buttons at the end and the menus).

## For `CHANGELOG.md`

- `ui::VideoPlayer`: a video player of the design system that draws the app's picture with the board's controls over it in three forms that follow its width (large, compact, minimal), or the picture alone with the app's own layer over it; every value it shows is the app's, bound or held, and told back on a press, a drag or a key.

## Decided

- **Every value is named, none is "the" value.** A player holds nine values of the app and no one of them is what the player is, so none takes the plain `on_change`: an `on_change(bool)` on a player would not say what changed. Each value follows the rule for a second value of a component: `x(Signal<T>)` bound, `x_controlled(value, handler)` held, `on_x_change(handler)` to add a handler (`playing`, `position`, `volume`, `muted`, `rate`, `captions`, `quality`, `fullscreen`, `picture_in_picture`). `new()` takes nothing, since a player is drawn before it has anything to play. The bound forms take the signals the shared parts take (`Signal<f32>` for the position, the volume and the rate, as `SeekBar` and `MediaVolume`), and the handlers are told `f64`, as theirs are.
- **Seeking tells twice**: `on_position_change` as the handle moves, `on_position_change_end` where it is let go, the names of the rule for what is dragged ("While it moves and when it ends"); `on_volume_change_end` likewise. A key that seeks tells both, so an app that seeks only on the end hears every key.
- **The picture is `media(..)` and the app's layer is `child(..)`**, by the rules: a picture a component is given is `media`, free content is `child`/`children`. The poster is `poster(..)`, an element too.
- **States the app sets are plain values** (`waiting`, `ended`, `live`, `error(title)` with `error_description` and `on_retry`): the player shows them and has nothing to tell of them. Replay tells `playing` true.
- **A mounted component.** It keeps a timer (the bar fading 3 s after the pointer rests), its measured width (the form and the folding of the volume, before the paint), the open state of its menus and their pages, whether the time reads what is left, and its tooltips, which need a `Ctx`. None of it is the app's. Converted on its own it is the still form.
- **The form follows the width, and the height follows a width given alone**, at 16 by 9, from the first frame: a player given `width(Length::Fill)` has the height of its picture. Given both, it is as told.
- **The bar's gradient is drawn from the part's background**: `PartStyle` has no gradient, so `Controls`' background is the color the gradient ends in at the bottom, and the renderer draws from nothing to it.
- **The menus hang from the bar's row**, `Placement::TopEnd`, `Menus`' gap (24, 22 compact) above it: their bottom 4 over the seek bar and their end on the bar's end, as the board has the settings menu. The row is no trigger, so a press of a menu's button while it is open closes it (the press outside), and the next press opens it.
- **Captions and the error are part of the controls**: without controls only the picture and the app's layer are drawn, as the brief has it; an app of a feed draws its own.
- Thresholds: compact under 400, minimal under 280 (the board's 432 state examples are the large form, its compact one 320, its minimal 200); the volume folds under 480, the board's number.
- The keys act while the focus is within the player; the player is focusable without being a stop of Tab, so a press of the picture gives it the focus. Space and the arrows are left to a control that has the keyboard's focus (a button presses on Space, the seek bar moves on the arrows); the letters always act.

## Not done

- **The cursor does not hide** while the bar has faded: the engine has no hidden cursor (`CursorIcon` has no `None`). Smallest change: a `CursorIcon::None` the shells map to hiding the pointer (winit's `set_cursor_visible(false)`, the browser's `cursor: none`), then one rule in the recipe: `.state(VideoPlayerState::Hidden, Root, |s| s.cursor(CursorIcon::None))`.
- **The settings' choices open beside their row**, as a submenu, and not in its place with a back row (`Media/Settings Menu/Page`, `TymGG`): `ui::Menu` has no pages, and an item that does not close its menu is only one with a submenu. Smallest change: `Menu::page(..)`, or an item that replaces the menu's items with others and a back row; the player would give its three pages to it in place of `MenuItem::submenu`.
- **The seek preview** (`Media/Seek Preview`, the board's "seeking" state `L7ybi0`): a frame of 128 by 72 over the pointer needs a frame for a time, which the media engine is to give. Smallest change: `SeekBar::preview(Fn(f64) -> Option<Element>)` in the shared part, and `VideoPlayer::preview(..)` passing it on.
- The time in the compact form is to be mono 11 and in the large one mono 12 (see "Needed from the media parts").

## Differs from the board

- The settings' second level opens beside the menu (see "Not done").
- The value at the end of a settings row is the menu item's shortcut, restyled to the body's type at 13 (`MenuValue`): the board's has the same look.
- The speed menu is a `ui::Menu` with the speeds of `MEDIA_RATES` and the heading "Playback speed", built by the player, and not `ui::MediaSpeedMenu` (see "Needed from the media parts"); it looks the same.
- The tooltip of the volume is on the whole volume (its button and its slider), as `Tooltip::mount` takes one element.
- The error's button is white with `#18181B` text, a literal of the board's, and white at 90% under the pointer, which the board does not draw.
- The cursor stays (see "Not done").

## Needed from the media parts

- **`MediaSpeedMenu`**: `placement(Placement)`, `offset(..)`, `id(..)` and `rules(&Recipe)` (or `style`), as `Menu` has them. The player's speed menu hangs over the seek bar (`Placement::TopEnd` from the bar's row, 24 above it) with a panel 168 wide, which `MediaSpeedMenu` cannot say; the player builds the same menu with `ui::Menu` until it can, in `Make::speed_menu`.
- **`MediaTime`**: a size, or `style`/`rules`: the board's time is mono 12 in the large bar and mono 11 in the compact one (`puY85`). Smallest: `MediaTime::size(MediaTimeSize)` with `Md` 12 and `Sm` 11. The player passes nothing for it yet.
- **`MediaVolume`**: nothing new. The player uses `size(Lg)` (a button of 36 with a glyph of 20), `length(64.0)`, `collapsed(narrow)` and `on_media(true)`; the board's slider in the bar has a white range, a 12 white thumb and a `#FFFFFF33` track, which `on_media` is to give.
- **`SeekBar`**: nothing new for this pass; the bar over the picture is `on_media(true)`, 16 high (the player's bar is laid out for 16 and its row stands on the bottom padding whatever the seek bar's height). A live stream with nothing to go back in is drawn as a bar of 1 second at 1, full, and a press of it tells nothing; a `SeekBar::full()` or a live form of the part would say it better. The preview is under "Not done".
- **`PlayButton`**: nothing new; the player uses `variant(Overlay)` with `Xl` (64) and `Lg` (48), `replay`, and the labels.
