# Element DSL

## Import

```rust
use lurq::{
  layout::{Alignment, StackAlignment},
  node::{Element, color::Color},
};
```

`Element` is the public UI builder. The internal `Node` type is crate-private.

## Constructors

| Constructor | Description |
|-------------|-------------|
| `Element::new()` | Empty zero-size element |
| `Element::row()` | Horizontal container |
| `Element::column()` | Vertical container |
| `Element::stack()` | Overlay container; later children paint on top |
| `Element::text("content")` | Text with default style |
| `Element::styled_text("content", style)` | Text with custom `TextStyle` |
| `Element::rect(width, height)` | Fixed-size rectangle leaf |
| `Element::spacer()` | Empty leaf, often used with `.flex(1.0)` |
| `Element::scroll_vertical(child)` | Vertical scroll container |
| `Element::scroll_horizontal(child)` | Horizontal scroll container |
| `Element::scroll_both(child)` | Two-axis scroll container |

## Children

```rust
Element::column()
  .child(Element::text("one"))
  .child(Element::text("two"))
  .child(Element::text("three"))
```

```rust
let items = vec![Element::text("a"), Element::text("b"), Element::text("c")];
Element::column().with_children(items)
```

## Containers

```rust
Element::row()
  .spacing(12.0)
  .align_items(Alignment::Center)
  .child(Element::rect(40.0, 40.0))
```

```rust
Element::stack()
  .stack_align(StackAlignment::BottomEnd)
  .child(Element::rect(200.0, 120.0))
```

## Sizing

```rust
Element::spacer().size(200.0, 100.0)
Element::spacer().width(200.0)
Element::spacer().height(100.0)
Element::rect(80.0, 80.0)
```

## Visuals

```rust
Element::rect(100.0, 50.0)
  .fill("#3b82f6")
  .rounded(8.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
```

## Padding

```rust
Element::column().pad(16.0)
Element::column().pad_xy(16.0, 8.0)
Element::column().pad_left(10.0)
Element::column().pad_right(10.0)
Element::column().pad_top(10.0)
Element::column().pad_bottom(10.0)
```

## Flex

```rust
Element::row()
  .child(Element::rect(100.0, 50.0))
  .child(Element::spacer().flex(1.0))
  .child(Element::rect(100.0, 50.0))
```

Flex applies inside `Row` and `Column`.

## Relative Positioning

```rust
Element::rect(50.0, 50.0).relative(10.0, 20.0)
```

`relative(x, y)` is an alias for `offset(x, y)`. It shifts the element visually without changing the space it takes in parent layout.

## Absolute Positioning

Absolute positioning is supported in `Stack`.

```rust
Element::stack()
  .child(
    Element::rect(300.0, 120.0)
      .fill("#f8fafc")
      .rounded(12.0),
  )
  .child(
    Element::rect(86.0, 34.0)
      .fill("#f97316")
      .rounded(8.0)
      .absolute(190.0, 24.0, 86.0, 34.0),
  )
  .child(
    Element::text("absolute")
      .absolute_position(201.0, 31.0),
  )
```

Rules:

- `absolute(x, y, width, height)` positions the child at `(x, y)` inside the stack and forces its size.
- `absolute_position(x, y)` positions the child but lets it keep its measured size.
- Absolute children do not affect stack size.
- There is no z-index API. Rendering follows child order; later stack children paint on top.

## Alignment Override

```rust
Element::column()
  .align_items(Alignment::Start)
  .child(Element::text("left"))
  .child(Element::text("right").align(Alignment::End))
```

## Events

```rust
Element::rect(100.0, 40.0)
  .fill("#3b82f6")
  .on_click(|e| println!("clicked at {}, {}", e.x, e.y))
  .on_mouse_enter(|| println!("hover in"))
  .on_mouse_leave(|| println!("hover out"))
  .on_key_down(|e| println!("key: {}", e.key))
```

## Text Styling

```rust
use lurq::{
  layout::text_style::{FontStyle, FontWeight, TextStyle},
  node::color::Color,
};

Element::styled_text("Bold title", TextStyle {
  font_size: 24.0,
  weight: FontWeight::Bold,
  style: FontStyle::Normal,
  color: Color::from_hex("#1e293b"),
  ..TextStyle::default()
})
```

## Components

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  node::Element,
};

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    let count = self.count.clone();
    Element::column()
      .spacing(8.0)
      .child(Element::text(&format!("Count: {}", self.count.get())))
      .child(
        Element::text("Increment")
          .on_click(move |_| count.update(|n| *n += 1)),
      )
  }
}

fn render_parent(ctx: &mut Ctx) -> Element {
  Element::column().child(ctx.mount::<Counter>(()))
}
```
