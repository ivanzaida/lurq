# Breadcrumb: hand-off

`ui::Breadcrumb` and `ui::BreadcrumbStep`, from the board's `Breadcrumb` (`sWePu`) and `Breadcrumb/Collapsed`
(`WvfRh`) in Navigation of Components / light (`Ovt1D`: "Trails" and "Compact"), used once each in Components /
dark. The board draws those two and nothing else of it: no hover, focus, disabled, icon on a step or slash. What
lurq has for those is under "Decided" and "Differs from the board".

Files: `crates/lurq/src/ui/breadcrumb.rs`, `crates/lurq/tests/ui/breadcrumb.rs`,
`crates/gallery/src/pages/breadcrumb.rs` (page "Breadcrumb", group Navigation), and a line each in the three
`mod.rs` files. Nothing in shared code or the engine was changed.

## For `docs/src/content/docs/design-system.md`

## Breadcrumb

The way to where one is: a step for each place on it, each but the last a link back there, the last the page one is on.

```rust
use lurq::ui::{Breadcrumb, BreadcrumbSeparator, BreadcrumbStep, icons};

Breadcrumb::new()
  .step(BreadcrumbStep::new("Settings").value("/settings"))
  .step(BreadcrumbStep::new("Appearance").value("/settings/appearance"))
  .step("Theme")
  .on_select(|route| go_to(route));

// More steps than four: the first, an ellipsis that opens a menu of the hidden ones, and the last three.
// anchor: ElementRef, open: Signal<bool>, kept by the app as for any menu.
Breadcrumb::new()
  .step(BreadcrumbStep::new("Home").icon(icons::HOME).value("/"))
  .steps(["Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
  .max_items(4)
  .menu(anchor.clone(), open.clone())
  .separator(BreadcrumbSeparator::slash());
```

| Builder | |
| --- | --- |
| `step(impl Into<BreadcrumbStep>)`, `steps(iter)` | Steps, in order, after those it has. The last is the page one is on: text, not a link. A `&str` is a step of that label |
| `on_select(Fn(&str))` | Called with the value of the step followed, after the step's own handlers |
| `max_items(usize)` | The most steps shown. With more, the first, an ellipsis and the last `max_items - 1`; never fewer than two. All of them without it |
| `menu(ElementRef, Signal<bool>)` | What the menu of the hidden steps hangs from, and whether it is open: the app's, as for a [menu](#menu). A press of the ellipsis turns it; choosing a step, a press outside and Escape close it |
| `separator(BreadcrumbSeparator)` | What stands between two steps: `BreadcrumbSeparator::chevron()` (the default), `slash()`, `Icon(name)` or `Text(..)` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws every link and the ellipsis in the state. The ellipsis's id is the root's with `-ellipsis` after it |
| `BreadcrumbStep::new(label)` | A step. `value(&str)` is what `on_select` is told (its label until then), `icon(name)` an icon before its text, `on_select(Fn())` called when it is followed, `id` the id of its link, of the page, or of its item in the menu while it is hidden |

- A link is followed by a click, and by Enter or Space while it is focused. The links and the ellipsis are stops of Tab in their order; the page is not. Choosing a hidden step in the menu follows it as a press of its link would.
- Parts: `BreadcrumbPart::Root` (the row), `Link` (a step that is a link: what is pressed, with its icon and text), `Page` (the last step), `Icon` (a step's icon), `Separator` and `SeparatorIcon`, `Ellipsis` (what is pressed to open the menu) and `EllipsisIcon`. States: `BreadcrumbState::Collapsed` (some steps are hidden), `Disabled`.
- In the design the steps are 8 apart in the body's 14; the links are in `MutedForeground` and the page in `Foreground` at 500; a chevron of 14 in `MutedForeground` stands between each two. The ellipsis is a square of 24 filled with `Highlight` (the board's `$accent`), with the small radius and an icon of 16 in `Foreground`. Under the pointer a link is in `Foreground`; the ring of the focus is around the link. A step's icon is 14, 4 before its text. A disabled breadcrumb is dimmed, follows nothing and is no stop of Tab.
- A theme gives the separator a renderer of its own with `set_part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator, ..)`, built once for each place between two steps; or restyles it by the recipe.
- To what reads the tree (`lurq_snapshot` of the MCP server) the root is a `navigation` called "Breadcrumb", each link a `link` called by its label, the page has `a11y_current` `page`, and the ellipsis says `a11y_haspopup` `menu` and whether it is `a11y_expanded`.
- A renderer that draws another breadcrumb has `view.shown()` (the steps to draw, by index), `view.step(index)` (a link with its behavior, or the page), `view.separator()`, `view.ellipsis()` (the button with its menu, while some steps are hidden), `view.root()` and `view.follow(index)` (what following a step does).

## For `CHANGELOG.md`

- `ui::Breadcrumb`: the way to where one is, its steps links that a press or Enter follows and the last the page; a chevron or a slash between them, an icon on a step, and with more steps than `max_items` an ellipsis that opens a menu of the hidden ones.

## Decided

- **A builder, with the menu's anchor and open signal the app's.** A breadcrumb keeps nothing but whether its menu is open, which `Menu`, `Popover` and `Dialog` already leave to the app's `Signal<bool>`; `menu(anchor, open)` takes it the same way. The anchor is the app's too, and not an `ElementRef::new()` made in `build`: the runtime knows an open overlay by the handle of its anchor (`opened_overlay_focus`, `note_focus_returns` in `app/runtime/overlay_layers.rs`), so an anchor made anew in each render would make every render of what holds the breadcrumb, while the menu is open, a menu that has just opened, and the focus would jump to its first item. Without `menu(..)` the ellipsis is drawn and opens nothing (a builder has nothing to keep `open` in); the docs and the gallery always give it.
- **Collapsed by a count (`max_items`), not by what fits.** `ShrinkLimit::Drop` drops the middle steps of a narrow row, but nothing tells the breadcrumb which of them it dropped: the ellipsis would have to appear exactly when a step is dropped, and its menu list exactly the dropped steps. A dropped child only reports a zero-size rect through its element refs, read after the layout, so a builder cannot decide it, and a mounted component would decide it a frame late, the ellipsis taking room that changes what drops. A count is what the board's `Breadcrumb/Collapsed` shows (the first step, the ellipsis, the last) and what an app knows.
- `max_items(n)` counts the steps shown, the ellipsis apart: the first and the last `n - 1`. `n` under 2 is 2, the board's collapsed form. Without it nothing is hidden.
- A step's value is its label until `value(..)` gives another: a breadcrumb of plain `&str` steps tells `on_select` the label. A step's own `on_select` runs before the breadcrumb's.
- The last step is the page: never a link, never a Tab stop, never followed, whatever handlers it has. It has no role of its own; it is named by its label with `a11y_current` `page`.
- Under the pointer a link turns to `Foreground`, and its focus is the theme's ring around the link (radius `Sm`, the link has no padding): the board draws neither. The underline of `ui::Link` is not used: a breadcrumb's links are quiet until pointed at.
- The ellipsis is filled at rest as the board draws it, with a veil of `StateHover` under the pointer and of `StatePressed` held down, as the button family does with a fill.
- The separator: `BreadcrumbSeparator::Icon(name)` or `Text(..)`; `slash()` is the text "/" in the steps' type, since lurq's icon font has no slash. A theme changes it for every breadcrumb by a part renderer of `BreadcrumbPart::Separator`.
- A step's icon is before its text (a house before "Home"), 14 and 4 from it, as `ui::Link` sizes and spaces its icon after its text.
- Disabled is the whole breadcrumb's; a step has no disabled of its own (a place on the way to here can always be gone back to).
- The gallery's page header (`crates/gallery/src/doc.rs`, `page_header`) still draws its breadcrumb by hand, as asked. It draws exactly this component's recipe and could be `ui::Breadcrumb` with its steps not followed.

## Not done

Nothing. The tests in `crates/lurq/tests/ui/breadcrumb.rs` were written and compiled, not run, as the batch's flow has it.

## Differs from the board

- The board draws no hover, focus, disabled, icon on a step or slash separator: what lurq has for them is under "Decided".
- The board's `$accent` is lurq's `Highlight` (`design/tokens/variables.tsv`, `tests/theme/parity`), so the ellipsis is filled with `Highlight`; `PaletteColor::Accent` is a deprecated name of `Primary`.
