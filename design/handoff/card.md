# Handoff: Card and Separator

`ui::Card` (`crates/lurq/src/ui/card.rs`) and `ui::Separator` (`crates/lurq/src/ui/separator.rs`), with their tests
(`crates/lurq/tests/ui/card.rs`, `separator.rs`) and their gallery pages (`crates/gallery/src/pages/card.rs` in
Surfaces & overlays, `separator.rs` in Data). Nothing outside these files changed but the lines added to the three
`mod.rs` files: no shared code, no engine, no keyframes.

## For `docs/src/content/docs/design-system.md`

## Card

A surface that holds one thing: a group of settings, a summary of a project, a form of a few fields.

```rust
use lurq::ui::{Button, ButtonVariant, Card, CardSize, CardVariant};

Card::new()
  .title("Appearance")
  .description("How lurq looks on this device.")
  .child(accent_field)
  .child(dark_mode_row)
  .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
  .footer(Button::new("Save"));

// Pressed as a whole: one stop of Tab.
Card::new()
  .variant(CardVariant::Elevated)
  .size(CardSize::Sm)
  .title("lurq")
  .description("A Rust UI toolkit with GPU-backed rendering.")
  .on_click(move |_| open_project());
```

| Builder | |
| --- | --- |
| `title(&str)` | What the card is about, 16/600. It is what the card is called, and wraps in the width the card has |
| `description(&str)` | More of it, under the title, 14 in the muted color |
| `action(element)` | At the end of the header, beside the title: a button, a menu, a badge |
| `child(element)`, `children(..)` | The content, under the header |
| `footer(element)` | Added to a row at the end of the card, under a line: its least action first and its main one last |
| `variant(..)` | `CardVariant::Outline` (the default), `Elevated`, `Filled`, or a variant the theme's recipe names |
| `size(..)` | `CardSize::Md` (the default) or `Sm`, or a size the theme's recipe names |
| `width(..)` | A width of its own. Without one the card is as wide as the room it is given |
| `on_click(handler)` | Makes the card pressed as a whole, from the pointer and the keyboard |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the surface |

- Every part is optional: a card with children alone is a padded surface. The header is there where the card has a title, a description or an action; the footer where it has something in it, set off by a line only where there is a body over it.
- A card is as wide as the room it is given, as a block is. Cards side by side in a row are each given a share of it: a `Column::new().flex(1.0)` around each, or a grid. The title and the description wrap in the room the action leaves them.
- A card given `on_click` is pressed as a whole: one stop of Tab, with the focus ring around it, pressed by a click, Enter and Space. A press of a control in its action or its footer is that control's and not the card's; a control in its content is pressed with the card, so a pressed card keeps its controls in the action and the footer. A card without `on_click` is read: it is no stop of Tab, and each control in it is a stop of its own.
- Parts: `CardPart::Root` (the surface; what is pressed), `Body` (the header over the content, padded), `Header` (the heading and the action), `Heading` (the title over the description), `Title`, `Description`, `Action`, `Content`, `Footer`. States: `CardState::Pressable`, `Disabled`, `Footed` (it has a footer under a body; the line over the footer is the body's `underline`).
- In the design a card is in `Card` with `CardForeground` ink and corners of `radius-xl`. The outlined card has an edge of 1 in `Border` and the small shadow, the elevated one the medium shadow and no edge, the filled one `Muted` and no edge. Its body is padded 24 with 24 between the header and the content; the title is 16/600, the description 6 under it in 14 of `MutedForeground`; what is in the content is 16 apart; the footer is padded 16 and 24 with its buttons 8 apart at its end. A small card is padded 16, with a title of 14 and a description of 13. A pressed card takes an edge of `Ring` and the medium shadow under the pointer, and the small shadow again while it is held down; a disabled one is dimmed.
- The settings card and the stat card of the design are cards with their parts styled: `style(CardPart::Body, |s| s.padding_x(14.0).padding_y(14.0))`, `style(CardPart::Root, |s| s.radius(RadiusSize::Lg).shadow(BoxShadowValue::none()))`, and what they hold as the content.
- To what reads the tree (`lurq_snapshot` of the MCP server) a card is a `group` called by its title, and a pressed card a `button`.
- A renderer that draws another card (`CardRenderer`) starts from `view.read_root()` or `view.pressed_root()`, the surface with its behavior, and puts in it what it likes: `view.body()`, `view.header()`, `view.heading()`, `view.action()`, `view.content()` and `view.footer()` are the default ones.

## Separator

A line between things: sections of a page, groups of a toolbar, the two ways to sign in.

```rust
use lurq::ui::Separator;

Column::new()
  .child(github_button)
  .child(Separator::horizontal().label("or"))
  .child(email_button);

// Down a row that centers what is in it: a length of its own.
Row::new()
  .align_items(Alignment::Center)
  .child(undo)
  .child(Separator::vertical().length(20.0))
  .child(bold);
```

| Builder | |
| --- | --- |
| `Separator::horizontal()`, `vertical()`, `new(orientation)` | A line across a column, or down a row |
| `label(&str)` | A word in the middle, with the line on either side of it |
| `length(f32)` | How long it is: its width across a column, its height down a row |
| `decorative(bool)` | Whether tooling passes it by. On by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- A separator across a column is as wide as the room it is given. One down a row is as high as the row where the row stretches its children (`align_items(Alignment::Stretch)`); in a row that does not, it is given a `length`. An inset is the root's padding: `style(SeparatorPart::Root, |s| s.padding_x(16.0))`.
- Parts: `SeparatorPart::Root` (the line, or the lines and the label), `Line`, `Label`. Its orientation is the variant of its recipe: `SeparatorOrientation::Horizontal`, `Vertical`.
- In the design the line is 1 in `Border`. A label is in the caption's type in `MutedForeground`, 8 from the line on either side.
- A separator is decoration to what reads the tree. One given `decorative(false)` is a `separator` with `a11y_orientation`, called by its label where it has one: for one between parts a person moves between.
- Tabs, the dialog and the menu draw lines of their own from their recipes.

## For `CHANGELOG.md`

- `ui::Card` and `ui::Separator`: a surface with a title, a description and an action in its header, its content and a footer of buttons, outlined, elevated or filled, in two sizes, and pressed as a whole with Tab and the focus ring where the app gives it `on_click`; and a line across a column or down a row, with a word in its middle.

## Decided

- **A card is as wide as the room it is given**, not as wide as its content. A title and a description that wrap need a
  width to wrap in, and a column whose width is its content's gives them none; the engine's `Row` and `Column` take no
  `align(..)` of a child (`align_self` is read only by a `Stack`), and a width of `Pct(100)` resolves against the room
  offered. So the root is `Length::Fill` in the recipe, `width(..)` sets a number over it, and cards side by side are
  each put in a column with `flex(1.0)` (the gallery's page does this).
- **The variants** are the board's three card styles. The default, outline, is the board's Card: an edge and the small
  shadow, as the Card component and the Stat Card draw it.
- **The small size** is the board's Card/Simple: padded 16, a title of 14, a description of 13 at a line height of 1.4.
  What the board does not draw of it (a header gap of 12 to an action, 12 between what is in the content, a footer
  padded 12 and 16) follows the medium card's proportions.
- **The body** is a part of its own that pads the header and the content together (24 around, 24 between), so that a
  card with children alone is a padded surface and the footer's line can run from edge to edge.
- **The line over the footer** is the body's `underline` under the `Footed` state, not a top edge of the footer:
  `PartStyle` has a line along the bottom and along the start, none along the top, and the line is then where the
  board draws it without a change to shared code. A card with a footer and nothing over it has no line.
- **A pressed card** is built on `shared::pressable`: its root is the engine's `Button` (a row) holding a column as wide
  as it, with the body and the footer. Its hover is the board's "interactive · hover": an edge of `Ring` and the medium
  shadow. Held down it keeps the edge and sinks back to the small shadow. The focus ring is `focus_ring()` on the root,
  outside it; what holds a card gives it room or `overflow_visible()` for it and for the shadow.
- **A press of the action or the footer is not the card's**: each is a row that stops the click from going on to the
  card, where the card is pressed. The content does not, since a press of its text is a press of the card.
- **A disabled card** is dimmed by the theme's opacity for what is disabled, and a pressed one is not pressed, not
  reached by Tab and does not answer the pointer (its `Pressable` state is off).
- **To tooling** a card is a `group` called by its title, a pressed one a `button`; a separator is nothing unless it is
  `decorative(false)`, then a `separator` with `a11y_orientation` and, where it has a label, `a11y_name`.
- **A vertical separator has no height of its own**: a height of `Pct(100)` in a row whose height is its content's would
  be all the room the row is offered. It is stretched by a row with `align_items(Stretch)`, or given a `length`.
- **The separator's label** is drawn though the board does not draw one, as the brief and the gallery's playground ask:
  the caption's type in the muted color, 8 from the lines, which share what the label leaves.

## Not done

Nothing.

## Differs from the board

- **The outline card's shadow**: the board's Card/Simple swatch named "outline" says "no shadow" and draws none, while
  the Card component (and the Stat Card) draws the small shadow. The card follows the Card component; a theme takes the
  shadow off with `.variant(CardVariant::Outline, CardPart::Root, |s| s.shadow(BoxShadowValue::none()))`.
- **The width**: the board's Card is 360 and Card/Simple 196; a card here is as wide as the room it is given unless
  given a width (see Decided). The gallery gives the board's widths.
- **The description of the medium card** has the theme's line height for its type; the board gives it none.
- **Not on the board**, and decided here: the action at the end of the header (16 beside the heading, at its top), the
  card held down, the focus ring of a pressed card, a disabled card, the hover of an elevated or a filled card (the
  same edge of `Ring` and medium shadow as the outlined one), the vertical separator (1 wide) and the separator's label.
- **The settings card and the stat card** are not components here: they are composed from a card in the gallery's page,
  with the board's paddings (14 and 20) and the settings card's `radius-lg` and no shadow set by `style(..)`.
