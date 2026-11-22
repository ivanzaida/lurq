---
title: Glass
description: An element that is a piece of glass over what is painted behind it, as a phone's newest system draws its bars and buttons.
---

# Glass

`.glass(Glass)` makes an element a piece of glass lying on the page. What is painted behind its box is:

- **bent** along the element's edge, as through a lens: the middle shows what is behind it where it is, and the edge
  pulls it in from further inside, the more the nearer the edge;
- **blurred**, saturated, brightened and **tinted**;
- lit along the **rim**, with a **shadow** under the glass.

The element then paints itself over the glass. It wants no fill of its own.

```rust
use lurq::{app::theme::RadiusSize, components::{Row, Text}, node::Glass};

Row::new()
  .padding_horizontal(18.0)
  .height(48.0)
  .rounded(RadiusSize::Full)
  .glass(Glass::regular())
  .child(Text::new("Glass"))
```

Glass is drawn by lurq's own renderer on every platform, not by a view of the system. The lens and the three materials
have the numbers measured on the system's Liquid Glass in an iOS 26 simulator, with test patterns passed behind it.

## The Materials

| Material | For | What it does |
|---|---|---|
| `Glass::regular()` | Buttons, bars, menus | Blurs what is behind it enough for words on it to be read over anything, and [follows it](#it-follows-what-is-behind-it) |
| `Glass::panel()` | A sheet, a sidebar, a large surface | Whiter, brings out the colours behind it; light or dark by the theme, not by what is behind it |
| `Glass::clear()` | Over a picture or a video | Leaves what is behind it nearly sharp; gives no colour to what is on it |

Every number of a material can be changed from there:

| Builder | Default (regular) | What it is |
|---|---|---|
| `bend(f32)` | 45 | How far inside itself the very edge shows the backdrop from. 0 bends nothing |
| `bezel(f32)` | 19.5 | The width of the curved edge; the glass is flat inside it |
| `blur(f32)` | 10.5 | The blur of what is behind, as [`backdrop_blur`](../blur/#frosted-glass)'s radius |
| `saturation(f32)`, `brightness(f32)` | 1.25, 0.1 | What is done to the colours behind before the tint |
| `tint(Color)` | white, half | The colour of the glass, laid over what is behind by its alpha |
| `rim(f32)` | 0.68 | The light along the rim |
| `shadow(GlassShadow)` | 32 wide, 8 down, 0.12 | The shadow under the glass; `GlassShadow::NONE` for none |
| `dispersion(f32)` | 0 | How far apart red and blue are bent: colours the edges of what the lens shows |
| `ink(Option<Color>)` | black | The colour of what is on the glass |
| `over_dark(Option<GlassLook>)` | a dark glass | The look over a dark backdrop; `None` for one look over anything |
| `follows(GlassFollows)` | `Backdrop` | What decides between the two looks |
| `interactive(bool)` | false | [Answers a press](#it-answers-a-press) |
| `amount(f32)` | 1 | How much of the material is there |

Lengths are logical pixels.

## It Follows What Is Behind It

A regular glass has two looks. Over something light it is a light glass, and over something dark a dark one; when
what is behind it changes (a list scrolls under a bar) it turns from one to the other in a quarter of a second.

What is on the glass turns with it: a `Text` that names no colour, and everything else that takes the inherited
[`foreground`](../styling-events/), is drawn in the glass's `ink`, black on the light glass and white on the dark one.
What has a colour of its own to choose, a picture or a mark, hears the glass turn:

```rust
let dark = ctx.signal(false);
let set = dark.clone();
Row::new()
  .glass(Glass::regular())
  .on_glass_look(move |now_dark| set.set(now_dark))
  .child(icon(if dark.get() { WHITE } else { BLACK }))
```

- In a light theme the glass turns dark over a backdrop darker than the middle grey; in a dark theme it is dark over
  all but a very light one, as the system's is. A theme is dark when its page (`PaletteColor::Background`) is.
- How light the backdrop is is told from what is painted behind the glass: fills, gradients and pictures whose
  pixels are in memory. Words are not counted, nor a video's frames; a glass over a video keeps the look it had.
- `Glass::panel()` follows the theme alone (`GlassFollows::Appearance`). A glass with a `tint` of its own, and
  `Glass::clear()`, follow nothing.

## Shapes That Flow Together

A glass does not bend another glass as glass. Glasses that are one surface are the children of a container:

```rust
Row::new()
  .glass_container(24.0)
  .spacing(8.0)
  .child(Row::new().size(48.0, 48.0).rounded(RadiusSize::Full).glass(Glass::regular()))
  .child(Row::new().size(160.0, 48.0).rounded(RadiusSize::Full).glass(Glass::regular()))
```

- Two shapes that come within the container's distance of each other flow into one, as two drops do, and part again
  further off. Move them with a transition and they join and part as they go.
- The glass elements may be anywhere under the container; the material is the first one's. Everything else in the
  container is painted over the glass.
- A glass has at most eight shapes (`MAX_GLASS_SHAPES`); the glass elements after the eighth are glasses of their own.

## It Answers a Press

```rust
Row::new().rounded(RadiusSize::Full).glass(Glass::regular().interactive(true)).on_click(..)
```

While it is held down an interactive glass grows a little, with what is on it, and is lit from where the pointer or
the finger is; the light follows a finger that moves. Let go, it springs back.

## Shape and Presence

A glass has its element's box and corner radii, inside the clips the element is under, and under a `.transform(...)`
the transformed box. An element that grows, moves or changes its corners with a [transition](../animation-transforms/)
is a glass that flows from one shape to the other.

A glass comes and goes as a material, not by fading: `Transition::glass()` takes it to and from no glass through its
`amount`, so that its bend, its blur and its light grow in together. Between two materials every number moves.

```rust
let mut pane = Row::new().rounded(RadiusSize::Full).transition(Transition::glass().duration_ms(450));
if shown {
  pane = pane.glass(Glass::regular());
}
```

## The Light

The rim is lit the same all the way round by default. `Tree::set_glass_light([x, y])` says where the light is: a
vector across the window toward the light, as long as the light is off to that side (up to 1). On a phone lurq's shell
sets it from the way the device is held, so the light stays above the person as the device is tilted; the sensor is
read only while there is glass on screen.

## lurq's Own Components

On a phone these are of glass, unless the person asked the system to reduce transparency or increase contrast:

| Component | With glass |
|---|---|
| [`TabBar::floating()`](../routing/) | The bar and the round place beside it are one glass that follows what scrolls under it. The mark of the chosen tab slides from place to place; under a finger dragged along the bar it is a lens of clear glass, and the tab it is let go over is chosen |
| `NavigationBar` | No fill of its own: what scrolls under it fades into the page. A screen's toolbar is a bar of glass that floats over the screen, and the list of the screens to go back to is of glass |
| `Sheet` | The sheet is of `Glass::panel()` |

`TabBar::glass(..)`, `NavigationBar::glass(..)` and `SheetStyle::glass(..)` give each another glass, or `None` for
the fill and the blur they had. `NavigationBar` and `Sheet` are of glass by default on iOS only.

## What It Costs, and Where It Is Drawn

A glass is a backdrop blur with one more pass, and is kept from frame to frame as a backdrop blur is: while nothing
painted behind it changes, the blurred copy is not made again. A press, a light that moves and a turn to the other
look repaint the glass without blurring anew.

- The `wgpu` renderer (Metal, Vulkan, Direct3D 12 through wgpu, WebGPU and WebGL) draws glass.
- lurq's own `dx12` renderer and DevTools' software capture do not have the pass yet: they draw a glass as its blur
  alone, inside its box (the bounds of the shapes, for a container).
