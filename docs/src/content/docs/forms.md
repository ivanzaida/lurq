---
title: Forms
description: Form handling, field binding, submission, and the Button component.
---

# Forms

Requires the `form` feature flag.

```toml
lurq = { version = "0.4", features = ["form"] }
```

## Creating A Form

Create a `FormHandle` during `Component::create` using `ctx.form`. The handle owns field signals and a submit callback.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{FormHandle, FormOptions},
  node::Element,
};

struct Settings {
  form: FormHandle,
}

impl Component for Settings {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let form = ctx
      .form(FormOptions::new().field("user", "Ada").field("email", "ada@example.com"))
      .on_submit(|values| {
        println!("user: {}", values.get_string("user").unwrap_or_default());
        println!("email: {}", values.get_string("email").unwrap_or_default());
      });
    Self { form }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Element::new()
  }
}
```

`FormOptions` supports a builder pattern for default values:

```rust
FormOptions::new()
  .field("user", "Ada Lovelace")
  .field("email", "ada@example.com")
  .field("active", true)
```

Or pass all defaults at once:

```rust
FormOptions::new().default(FormValues::new().with("user", "Ada").with("volume", 50))
```

## Field Signals

`FormHandle` exposes typed signals for each named field. Calling a field accessor creates the signal on first access and returns the same signal on subsequent calls.

```rust
let name: Signal<String> = form.string("user");
let volume: Signal<f64> = form.number("volume");
let active: Signal<bool> = form.bool("active");
```

Bind these signals to input components:

```rust
use lurq::components::{Column, TextInput, Checkbox, Slider, Form, FormProps};

fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  Form::mount(
    ctx,
    FormProps::new(self.form.clone()),
    Column::new()
      .child(TextInput::new(self.form.string("user")).name("user"))
      .child(TextInput::new(self.form.string("email")).name("email"))
      .child(Checkbox::new(self.form.bool("notifications")).name("notifications"))
      .child(Slider::new(self.form.number("volume")).name("volume")),
  )
}
```

Give each input a `.name(...)` so the form can collect its value on submission. Inputs without a name are ignored during submit.

## Form Component

`Form` wraps children in a logical form boundary. It can be used in two ways:

**Mounted component** — uses `ctx.mount` internally, supports reactive re-rendering:

```rust
Form::mount(ctx, FormProps::new(form.clone()), Column::new().child(...))
```

**Static element** — no component lifecycle, useful in tests or static trees:

```rust
Form::element(FormProps::new(form.clone()), Column::new().child(...))
```

A `Form` accepts exactly one child. Wrap multiple children in `Column`, `Row`, or `Stack`.

## Submission

Forms submit when:

- A focused input receives `Enter`.
- A `Button::submit()` inside the form is clicked.

```rust
use lurq::components::Button;

Column::new()
  .child(TextInput::new(form.string("user")).name("user"))
  .child(Button::new("Save").submit())
```

On submit the form collects all named input values, updates the corresponding field signals, and calls the `on_submit` callback with a `FormValues` snapshot.

## FormValues

`FormValues` is the typed map received in the `on_submit` callback.

| Method | Returns |
| --- | --- |
| `get(name)` | `Option<&FormValue>` |
| `get_string(name)` | `Option<&str>` |
| `get_number(name)` | `Option<f64>` |
| `get_bool(name)` | `Option<bool>` |
| `entries()` | Iterator over `(&str, &FormValue)` |
| `len()` / `is_empty()` | Field count |

`FormValue` is an enum with `String(Arc<str>)`, `Number(f64)`, and `Bool(bool)` variants. It converts from `&str`, `String`, `bool`, `i32`, and `f64`.

## Button

`Button` is a row-based component that renders a clickable element.

```rust
use lurq::components::Button;

Button::new("Click me")
  .on_click(|_| println!("clicked"))
```

| Method | Purpose |
| --- | --- |
| `Button::new(label)` | Button with a text label. |
| `Button::empty()` | Button with no label. Use `.child(...)` to add content. |
| `.child(element)` | Append a child element. |
| `.with_children(iter)` | Append multiple children. |
| `.submit()` | Mark as a submit button (requires `form` feature). Triggers form submission on click. |
| `.button()` | Mark as a regular button (default). |
| `.kind(ButtonKind)` | Set the button kind explicitly. |
| `.spacing(value)` | Gap between children. |
| `.align_items(Alignment)` | Cross-axis alignment. |
| `.justify(Justify)` | Main-axis justification. |

All standard visual modifiers (`.background(...)`, `.rounded(...)`, `.padding(...)`, `.on_click(...)`, etc.) work on `Button` because it implements the typed component API.
