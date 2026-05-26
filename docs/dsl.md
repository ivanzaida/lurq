# DSL

## Import

```rust
use lurq::node::dsl::*;
```

## Constructors

| Function | Description |
|----------|-------------|
| `row()` | Horizontal flex container |
| `column()` | Vertical flex container |
| `stack()` | Z-axis overlay container |
| `text("content")` | Text node with default style |
| `styled_text("content", style)` | Text node with custom `TextStyle` |
| `rect(width, height)` | Fixed-size leaf node |
| `spacer()` | Empty leaf (use with `.flex(1.0)` to fill space) |

## Children

```rust
column()
  .child(text("one"))
  .child(text("two"))
  .child(text("three"))
```

```rust
let items = vec![text("a"), text("b"), text("c")];
column().with_children(items)
```

## Container Settings

```rust
row()
  .spacing(12.0)
  .align_items(Alignment::Center)
  .child(...)
```

```rust
stack()
  .stack_align(StackAlignment::BottomEnd)
  .child(...)
```

## Sizing

```rust
spacer().size(200.0, 100.0)   // fixed width and height
spacer().width(200.0)          // fixed width only
spacer().height(100.0)         // fixed height only
rect(80.0, 80.0)               // shorthand for spacer().size(80, 80)
```

## Background

```rust
rect(100.0, 50.0).fill("#3b82f6")

// or with a Color value
rect(100.0, 50.0).background(Color::new(255, 0, 0, 255))
```

## Padding

```rust
column().pad(16.0)                    // all sides
column().pad_xy(16.0, 8.0)           // horizontal, vertical
column().pad_left(10.0)              // single side
column().pad_right(10.0)
column().pad_top(10.0)
column().pad_bottom(10.0)
```

## Flex

```rust
row()
  .child(rect(100.0, 50.0))           // fixed 100px
  .child(spacer().flex(1.0))           // fills remaining
  .child(rect(100.0, 50.0))           // fixed 100px
```

## Offset

```rust
rect(50.0, 50.0).offset(10.0, 20.0)   // visual shift, doesn't affect layout
```

## Alignment Override

```rust
// child overrides parent's align_items
column()
  .align_items(Alignment::Start)
  .child(text("left-aligned"))
  .child(text("right-aligned").align(Alignment::End))
```

## Events

```rust
rect(100.0, 40.0)
  .fill("#3b82f6")
  .on_click(|e| println!("clicked at {}, {}", e.x, e.y))
  .on_mouse_enter(|| println!("hover in"))
  .on_mouse_leave(|| println!("hover out"))
  .on_key_down(|e| println!("key: {}", e.key))
```

## Text Styling

```rust
use lurq::layout::text_style::{TextStyle, FontWeight, FontStyle};

styled_text("Bold title", TextStyle {
  font_size: 24.0,
  weight: FontWeight::Bold,
  color: Color::from_hex("#1e293b"),
  ..TextStyle::default()
})
```

## Components

```rust
use lurq::app::component::Component;
use lurq::app::ctx::Ctx;

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, ctx: &mut Ctx) -> Node {
    let count = self.count.clone();
    column()
      .spacing(8.0)
      .child(text(&format!("Count: {}", self.count.get())))
      .child(
        text("Increment")
          .on_click(move |_| count.update(|n| *n += 1))
      )
  }
}

// Mount in parent
fn render(&self, ctx: &mut Ctx) -> Node {
  column()
    .child(ctx.mount::<Counter>(()))
}
```

## Full Example

```rust
use lurq::node::dsl::*;
use lurq::layout::Alignment;
use lurq::layout::text_style::{TextStyle, FontWeight};
use lurq::node::color::Color;

fn build_ui() -> Node {
  column()
    .spacing(16.0)
    .align_items(Alignment::Center)
    .child(styled_text("My App", TextStyle {
      font_size: 32.0,
      weight: FontWeight::Bold,
      color: Color::from_hex("#1e293b"),
      ..TextStyle::default()
    }))
    .child(
      row()
        .spacing(12.0)
        .child(rect(80.0, 80.0).fill("#ef4444"))
        .child(rect(80.0, 80.0).fill("#22c55e"))
        .child(rect(80.0, 80.0).fill("#a855f7"))
    )
    .child(text("Hello from lurq!"))
    .pad(32.0)
}
```
