# Gallery

`ui::Gallery`, from the board's `Gallery` (`mKDlC`, 392 × 144) in the section `Picture` (`wfriA`, dark `Gg1rp`) of
`Components / light`. Branch `claude/picture`. Its props as they were agreed: `image-design.md` in the sessions'
shared notes ("`ui::Lightbox` and `ui::Gallery`: the props"); how it is built: `picture-code.md` there.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the lines of
  `gallery`.
- `ui::Picture`: `in_grid` and `count` (crate-private): the picture as a gallery shows it, and the veil with how
  many more there are.
- Nothing of the engine.

## For `docs/src/content/docs/design-system.md`

## Gallery

Pictures in rows, each in a room of one shape, with one [lightbox](#lightbox) for them all; a gallery that chooses
keeps which of them are chosen.

```rust
use lurq::ui::{Gallery, Picture};

// A press of a picture opens it in a lightbox that holds them all.
Gallery::new(photos.iter().map(|photo| Picture::from_file(&photo.path).label(&photo.name)))
  .columns(4)
  .max_shown(8);

// chosen: Signal<Vec<PhotoId>>. A press chooses, Enter opens.
Gallery::keyed(photos.iter().map(|photo| (photo.id, Picture::from_file(&photo.path))))
  .selectable(true)
  .selected(chosen.clone());
```

| Builder | |
| --- | --- |
| `new(pictures)`, `picture(Picture)`, `pictures(..)` | The pictures, told apart by their place |
| `keyed(items)`, `item(key, Picture)` | The pictures, each with a key of the app's (`GalleryKey`: `Clone + PartialEq + Send + Sync`): what is chosen is a list of those keys, which stays right when the pictures are put in another order |
| `columns(usize)`, `gap(f32)`, `aspect_ratio(f32)` | How many stand in a row (4), the room between them across and down (8), and the width of a picture's room to its height (92 to 68). A row the pictures do not fill keeps its empty rooms: its pictures are as wide as the others |
| `width(Length)` | How wide the gallery is: all the room there is unless told, and never wider than its room. The pictures share it |
| `size(PictureSize)` | The form of its pictures: `Sm` unless told |
| `max_shown(usize)`, `more_text(Fn(usize) -> String)` | The most pictures it shows: with more, the last one shown lies under a veil that says how many more there are, itself among them ("+5"). A press of it opens that picture in the lightbox, which holds every picture |
| `selectable(bool)`, `selected(Signal<Vec<K>>)`, `selected_controlled(keys, on_change)`, `on_selection_change(Fn(&[K]))` | Pictures that can be chosen, and the keys of the ones that are. Without a signal the gallery keeps them |
| `lightbox(bool)`, `open(Signal<Option<usize>>)`, `on_open_change(Fn(Option<usize>))` | Whether a picture opens in the lightbox (it does unless told), and which one is open, `None` while it is closed |
| `on_select(Fn(usize))` | A press of a picture, with its place, in a gallery that neither chooses nor opens |
| `label(&str)`, `disabled(bool)` | What it is called to tooling; and a gallery that takes nothing, its pictures at half the opacity |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the gallery; its pictures are `<id>-0`, `<id>-1`, … |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which picture is open, and what is
  chosen where the app gives no signal.
- Each picture is the app's, without what lies over it (its bar, its badge, its own press and its own lightbox):
  the gallery gives it its press, its mark and its place in the lightbox. Its title and description are what the
  lightbox says under it.
- In a gallery that chooses, a press chooses a picture and takes it out again, and Enter opens the one with the
  focus; the picture that stands for the rest opens and is not chosen.
- Tab reaches the gallery once, and the arrows move the focus among its pictures, across and down.
- Parts: `GalleryPart::Root`, `Row`, `Cell`. States: `GalleryState::Selectable`, `Disabled`. What a picture looks
  like in it is the picture's recipe: `.rules(..)` on the pictures the app gives.
- To what reads the tree it is a `list` of pictures, or a `listbox` that takes several where it chooses.
- A renderer that draws another gallery has `view.render()` and the parts' styles; the rows are of `flex(1)` rooms
  with the ratio, so a picture is as wide as its share.

## For `CHANGELOG.md`

- `ui::Gallery` is a component of the design system: pictures in rows of rooms of one shape, with one lightbox for
  them all, a veil over the last one shown that says how many more there are, and pictures that are chosen by the
  app's own keys.

## Decided

- The "+5" count is the gallery's (`max_shown`), not a prop of a picture: a picture alone has nothing to count.
- The lightbox holds every picture, not only the ones shown: the veil says there are more, and the lightbox is
  where they are.
- What is chosen is a list of keys, not of places: it stays right when the pictures are put in another order. A
  gallery made with `new` uses a picture's place.
- In a gallery that chooses, Enter opens: a press and Space are taken by choosing, and without a key there would be
  no way to look at a picture before choosing it.

## Not done

- Pictures of their own shapes in rows of one height (a justified grid, a masonry) are not on the board and not
  built: every room has one ratio.
- A gallery does not make its rows shorter in a narrow room: four in a row stay four, smaller. `columns` is the
  app's to change.
- Not seen by hand: the lightbox opened from a gallery and the arrows among its pictures; the tests cover both.

## Differs from the board

- Nothing known.
