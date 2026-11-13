# Avatar and Avatar Group: hand-off

Built on `claude/ds-avatar-c3aafd`: `ui::Avatar` with its dot of presence, and `ui::AvatarGroup`. Files of their own:
`crates/lurq/src/ui/avatar.rs`, `crates/lurq/tests/ui/avatar.rs`, `crates/gallery/src/pages/avatar.rs`, this file.

## Changed outside the component's own files

- `crates/gallery/Cargo.toml`: the gallery's `lurq` has the feature `image` too (which brings `raster`), so that its
  avatar page shows pictures, and one that fails to decode. `Cargo.lock` does not change: `lurq` had the `image`
  crate already.
- `crates/lurq/src/ui/mod.rs`: `mod avatar;` and its `pub use` lines.
- `crates/lurq/tests/ui/mod.rs`: `mod avatar;`.
- `crates/gallery/src/pages/mod.rs`: `mod avatar;` and the `Page` entry "Avatar" (`/avatar`, `Group::Data`), after
  Steps. Tag puts its entry there too: at the merge both go in the Data group, one after the other.
- **The engine is not changed** (this session owned changes about images). It has what an avatar needs:
  `Image::placeholder` and `Image::error_view` show an element while a picture decodes and once it failed
  (`app/runtime/loading_images.rs`); an `Image` node with a radius paints its picture with those corners, and a node
  that clips with a radius clips what it holds to them (`image.wgsl` and `image_nv12.wgsl` take the quad's radii and
  the clip's). Nothing in `shared.rs`, `part_style.rs` or `recipe.rs`; no keyframes.
- Two tests of `tests/ui/avatar.rs` are behind features: the picture with pixels behind `raster`, the decoded picture
  and the one that fails behind `image`. The batch's command (`--features form,router`) compiles neither; run them
  with `cargo test -p lurq --features form,router,image --test ui_tests avatar`. The pipeline's suite
  (`scripts/check.ps1`) has `image` and runs them.

## For `docs/src/content/docs/design-system.md`

## Avatar

Who a person is, at a glance: the initials of the name on a fill, a picture in their place, whether the person is there.

```rust
use lurq::{components::Image, ui::{Avatar, AvatarPresence, AvatarShape, AvatarSize, icons}};

Avatar::new("Ivan Zaida");
Avatar::new("Ivan Zaida").image(Image::from_file("ivan.png")).presence(AvatarPresence::Online);
Avatar::new("Ivy Chen").size(AvatarSize::Xl);

// What is not a person: an icon in place of the initials, in a square.
Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square);
```

| Builder | |
| --- | --- |
| `new(&str)` | Who it is. The initials are the first letters of the first word and the last: "Ivan Zaida" is "IZ", "Ivan" is "I". A name with no letters shows the icon of a person |
| `image(Image)` | A picture in place of the initials, which fills the face, cropped to it (`cover`) and clipped to its shape: the engine's image element as the app builds it, from pixels (`Image::new`) or from a file or bytes it decodes (`Image::from_file`, `Image::from_bytes`, feature `image`), with a cache key and a size to decode to where it wants them. Feature `raster` |
| `icon(name)` | An icon of the theme's set in place of the initials, for what is not a person: a bot, a team |
| `size(AvatarSize)` | `Xs` 20, `Sm` 24, `Md` 32 (the default), `Lg` 40, `Xl` 56, or a size the theme's recipe names |
| `shape(AvatarShape)` | `Circle` (the default), or `Square` with the corners of a control |
| `presence(AvatarPresence)` | The dot at the bottom end of the face: `Online`, `Away`, `Busy`, `Offline`; `None` for none |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the root's |

- While a picture decodes and when it fails (a broken file, a format the decoder does not read), the avatar shows its initials or its icon: they are the image's placeholder and its error view. A picture with clear pixels shows the face's fill through them.
- An avatar is read, not operated: it takes no press and is no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) it is an `img` called by its name, with its presence after it: "Ivan Zaida, away".
- Parts: `AvatarPart::Root` (the face and the dot, as large as the face), `Face` (the shape: side, corners, fill, the ring of a group; it clips what it holds), `Initials`, `Icon` (its width is the icon's side), `Image`, `Presence` (the dot). States: `AvatarState::Square`, and the avatar's `AvatarPresence` under its own name (`.state(AvatarPresence::Away, AvatarPart::Presence, ..)`). Sizes: `AvatarSize`.
- In the design an avatar is a circle of 32 in `Muted`, its initials 12 and medium in `Foreground`; at 20, 24, 40 and 56 they are 9, 10, 14 and 18. A square one has the corners of `RadiusSize::Md`. The icon is half the side, in `MutedForeground`. The dot is 10 on a face of 32 (6, 8, 12 and 14 on the others), at the face's bottom end corner, round, ringed 2 outside it in `Background`: `Success` online, `Warning` away, `Destructive` busy, `MutedForeground` offline.
- The ring of the dot stands outside the avatar's box, as the rings of a group do: what holds an avatar at its edge clips them, as a container clips what is in it, unless it leaves 2 of room or is `overflow_visible()`.
- A person with a name and a line under it (the board's Persona) is an avatar beside two texts, built where it is used: `Row::new().spacing(10.0).child(Avatar::new(name).presence(..)).child(Column::new().spacing(1.0).child(name).child(line))`, the name 14 and medium, the line 12 in `MutedForeground`.
- A renderer that draws another avatar has `view.face()` (the shape in its style, clipped), `view.fallback()` (the initials or the icon), `view.presence_dot()` and `view.label()`; `view.image` is the picture the app gave, to put in the face.

## Avatar group

Avatars laid over one another: who is in a document, who reviews a change.

```rust
use lurq::{
  app::theme::PaletteColor,
  node::border::Border,
  ui::{Avatar, AvatarGroup, AvatarGroupPart, AvatarSize},
};

let people = ["Ivan Zaida", "Anna K.", "Mark R.", "Ivy Chen", "Ivo Kovac", "Lena Quinn"];
// Three, and "+3" for the three it leaves out.
AvatarGroup::new(people.map(Avatar::new)).max(3).label("Reviewers");

// On a card, the ring is the card's color.
AvatarGroup::new(people.map(Avatar::new))
  .size(AvatarSize::Sm)
  .style(AvatarGroupPart::Item, |s| s.border(Border::outside(2.0, PaletteColor::Card)));
```

| Builder | |
| --- | --- |
| `new(avatars)` | The avatars, in order: each lies over the end of the one before it |
| `max(usize)` | The most avatars it shows; after them a last one says how many it leaves out, "+3" |
| `size(AvatarSize)` | The size of every avatar in it |
| `label(&str)` | What it is called to what reads the tree. Without one, the names it shows and how many more |
| `style(part, ..)`, `id`, `key` | As a [button](#button) has them |

- The group gives its avatars its size and what its recipe says of an item; each keeps everything else it was given: its picture, its icon, its presence.
- To what reads the tree it is a `group` called by its label; the count is an `img` called "3 more".
- Parts: `AvatarGroupPart::Root` (the row: how far each avatar lies over the one before, as a gap below nothing), `Item` (laid over each avatar's `Face`: its ring), `More` (laid over the count's `Face`, after `Item`). Sizes: `AvatarSize`.
- In the design avatars of 32 lie 8 over one another, each ringed 2 outside its face in `Background`, the color of the surface behind them; the count is in `Secondary`, its text as the initials. Smaller and larger avatars lie a quarter of their side over each other.
- A renderer that draws another group has `view.avatars` (ready, with the group's size and rules), `view.hidden` and `view.more()`, the count as an avatar.

## For `CHANGELOG.md`

- `ui::Avatar` and `ui::AvatarGroup`: who a person is, as initials on a fill, a picture clipped to the avatar's shape (with the initials while it decodes and when it fails) or an icon, in five sizes, round or square, with a dot of presence; and avatars laid over one another, ringed, with the count of those a group leaves out.

## Decided

- `image(..)` takes the engine's `components::Image`, not a source type of the avatar's own: every way the engine has to
  get pixels (decoded `ImageData`, a file, bytes, with a cache key and a cap) is the app's to pick, and the avatar sets
  the fit (`cover`), the part's style, the placeholder and the error view. It is behind `raster`, as `Image` is; the
  placeholder and the error view are set where the feature `image` is on, since only an image that decodes loads or
  fails.
- The fallback is the image's placeholder and error view, not something under the picture: a picture with clear
  pixels would show the letters through it.
- `max(n)` is the most avatars shown, and the count comes after them: `max(3)` of six draws the board's group, three
  and "+3". A most above the number of avatars leaves nobody out and draws no count.
- The initials are the first letter or digit of the first word and of the last, in capitals; a word with neither is
  passed over. An empty name, or one with no letters, shows the icon of a person (the board's "fallback icon").
- An avatar is called by its name; with a presence, by its name and the presence ("Ivan Zaida, away"), since the dot
  says nothing to a reader otherwise. A group with no label is called by the names it shows and "and 3 more".
- The dot keeps to the bottom end corner of the face at every size, and grows with it: 6, 8, 10, 12, 14. The icon is
  half the side. A smaller or larger group overlaps by a quarter of the side (5, 6, 8, 10, 14). A square avatar has
  the corners of `RadiusSize::Md` at every size.
- The rings are `Border::outside`: they stand outside the face and change no size, so a group of four is 104 wide, as
  on the board. The avatar's root and the group's row do not clip, so the dot's ring and the group's rings show; what
  holds them clips at its edge as for any child.
- The count is an avatar of its own (`Avatar::count`, crate-private) with "+3" for its letters, so that it has the
  avatars' sizes and type; the group styles its face with `Item` and then `More`.
- No test asserts the letters while a picture decodes: the decode runs on a worker and may end before the first
  paint, so such a test would race. The letters while it decodes are the same element as the letters once it failed,
  which a test asserts, and the engine's `components/decoded_image.rs` tests the placeholder.
- The board's Persona (`w5Ey2`) is an avatar with its presence beside a name and a line under it: not a component.
  The docs and the gallery page show how to build it.
- The playground's stage has the avatar and, beside it, a group of six with the chosen size and most. The gallery's
  pictures are portraits of nobody it draws itself (a head and shoulders over a gradient, 64 pixels square); no file
  was added.

## Not done

Nothing.

## Differs from the board

- The board draws one presence, online, in `Success`. Away (`Warning`), busy (`Destructive`) and offline
  (`MutedForeground`) are decided here.
- The board draws the dot, the fallback icon and the group's overlap at 32 only; at the other sizes they are decided
  (see Decided).
- The board's sidebars use an avatar of 28 with initials of 10 (`Svy8P`, `HO3ea`), which is not one of its five
  sizes: not a size here. An app gets it with `.style(AvatarPart::Face, |s| s.square(28.0))` and
  `.style(AvatarPart::Initials, |s| s.font_size(10.0))`.
