## For `docs/src/content/docs/design-system.md`

## Pagination

The pages of a list, one of them the current one: the results of a search, the rows of a table, the posts of a feed.

```rust
use lurq::ui::{Pagination, Select};

// Pages count from 1, as a person reads them. The signal has the current one, and a press sets it.
Pagination::new(page.clone(), 20);

// The footer of a table: 284 rows, so many to a page, with a select of how many.
Pagination::for_items(page.clone(), 284, size.get())
  .compact(true)
  .page_size("Rows per page", Select::new(size.clone(), [(10, "10"), (20, "20"), (50, "50")]));

// The app holds the page: go is told the page of each press.
Pagination::controlled(current, 12, move |to| go(to));
```

| Builder | |
| --- | --- |
| `new(Signal<usize>, pages)` | `pages` pages, of which the current one is the signal's, counted from 1 |
| `for_items(Signal<usize>, total, page_size)` | The pages of `total` items, `page_size` to a page |
| `controlled(current, pages, go)` | The app holds the page: `current`, counted from 1, and `go` told the page of each press |
| `items(total, page_size)` | The pages of so many items, in place of a number of pages; the compact form then says "1–20 of 284" |
| `siblings(usize)` | How many pages are drawn on each side of the current one: 1 unless it is said |
| `compact(bool)` | Previous and next with where the list is, and no pages |
| `page_size(label, Select)` | A select of how many to a page, after its label, at the start; the pagination is then as wide as what holds it |
| `summary(&str)` | What is said of where the list is, in place of "1–20 of 284" or "Page 3 of 12": in another language |
| `label(&str)` | What the whole is called to what reads the tree: "Pagination" unless it is said |
| `on_change(handler)` | Called with the page of each press |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. Its buttons have ids after the pagination's: `{id}-previous`, `{id}-next`, `{id}-page-3` |

- Which pages are drawn is the pagination's: the first and the last always, the current one with its siblings on each side, and an ellipsis where pages are left out. While every page fits in `2 × siblings + 5` every page is drawn; once they do not, that many things are drawn wherever the current page is, so that the row does not change its length as a person goes through it. Near an end the pages run on to that end and take the room of the ellipsis that is not needed there: on page 1 of 20, `1 2 3 4 5 … 20`; on page 10, `1 … 9 10 11 … 20`; on page 20, `1 … 16 17 18 19 20`. An ellipsis always stands for two pages or more. `Pagination::pages_shown(current, pages, siblings)` says what is drawn, as `PageItem::Page(n)` and `PageItem::Ellipsis`.
- A current page past the ends is drawn as the end it is past; the signal is not changed. With no pages, previous and next are drawn, disabled, and nothing between them.
- Previous is disabled on the first page and next on the last. A press of a page goes to it; a press of the current one does nothing.
- Each page, previous and next is a button and a stop of Tab of its own; Left and Right move along the row (`FocusGroup::horizontal()`), from the last on to the first, and Home and End go to its ends. Enter and Space press the button the focus is on. A disabled button is passed by.
- The page-size select is the app's `ui::Select`, bound to its own signal; the pagination gives its trigger the board's size over the select's recipe (`PaginationPart::PageSize`, laid over `SelectPart::Trigger`), and calls it by the label beside it.
- To what reads the tree (`lurq_snapshot` of the MCP server) the whole is a `navigation` called "Pagination" (or its `label`), each page a button called "Page 3", and the current one has `a11y_current` "page". Previous and next are called "Previous page" and "Next page".
- Parts: `PaginationPart::Root`, `Nav` (the row of buttons the arrow keys move along), `Item` (laid over each button's root), `Current` (laid over the current page's, after `Item`), `Ellipsis`, `EllipsisIcon`, `Summary`, `PageSizeLabel`, `PageSize` (laid over the select's trigger). States: `PaginationState::Compact`, `PageSize`, `Disabled`.
- In the design the pages are ghost buttons of 36, 4 apart, the current one an outline button; previous and next are ghost buttons of 36 with a chevron of 16. The ellipsis is a box of 36 with `more-horizontal` of 16 in `MutedForeground`. A page button is at least 36 wide and 6 from its sides, so that a number of four digits or more still fits. The compact form has outline buttons of 30 for previous and next, 12 after where the list is, in the body's type at 13 in `Foreground`; its page-size select is 72 by 30 with its text at 13, 12 after its label in `MutedForeground`. A disabled button is dimmed as a button is.
- A renderer is given a `PaginationView`: where the list is (`current`, `pages`, `shown`, `summary`), and `view.previous()`, `view.next()`, `view.page(n)`, `view.ellipsis(i)`, `view.summary()`, `view.nav()` (the row of buttons) and `view.page_size()` (its label and the select) built in the recipe's style with their behavior.

## For `CHANGELOG.md`

- `ui::Pagination`: the pages of a list with one current, counted from 1, bound to a `Signal<usize>` (`new`, `for_items`) or held by the app (`controlled`). The first and the last page, the current one with `siblings` on each side and an ellipsis where pages are left out, in the same number of slots wherever the current page is (`Pagination::pages_shown`); previous and next disabled at the ends; each a stop of Tab with Left and Right moving between them, a `navigation` with pages called "Page 3" and `a11y_current` on the current one. A `compact` form with "1–20 of 284" or "Page 3 of 12", a `page_size` slot for a `ui::Select` of how many to a page, `summary` and `label` for other words, the parts `Root`, `Nav`, `Item`, `Current`, `Ellipsis`, `EllipsisIcon`, `Summary`, `PageSizeLabel`, `PageSize` and the states `Compact`, `PageSize`, `Disabled` for a recipe, and `PaginationView` for a renderer.

## Decided

- Pages count from 1 everywhere: the signal, `controlled`, `on_change`, `pages_shown` and the ids. 0 is the current page only of a pagination with no pages.
- A builder, not a mounted component: the page is the app's signal, and nothing else is kept.
- The buttons are `ui::Button`s with the pagination's rules laid over the button's recipe (as `ButtonGroup` does), so hover, press, focus ring and disabled are the button's. The variant is chosen in code: ghost for a page, outline for the current one, ghost for previous and next beside pages and outline in the compact form, as the board draws them.
- `siblings` is 1 unless it is said, as a pagination of a web library has it; the board's row, `1 2 3 … 12` on page 1 of 12, is `siblings(0)` (see `## Differs from the board`).
- The rule for the slots: `2 × siblings + 5` (first, ellipsis, the current with its siblings, ellipsis, last). Near the start (current ≤ siblings + 3) pages 1 to `2 × siblings + 3`, an ellipsis and the last; near the end (current ≥ pages − siblings − 2) the first, an ellipsis and the last `2 × siblings + 3` pages; between, both ellipses. An ellipsis therefore never stands for a single page, which a page in its place would say better.
- A page button's width is a least width of 36 with 6 of padding, not the board's 36 with none, so that "1024" still fits; every number of up to three digits is 36 wide, as on the board.
- The arrow keys move along the buttons only (`Nav` holds the focus group), not into the page-size select, which keeps the arrow keys for itself.
- Each button is keyed by what it is (`page-7`, `previous`), so that after a press the focus stays on the page pressed, wherever it now stands in the row.
- The full form says where the list is only when it is given a `summary`; the compact form always does. The page-size slot is drawn in either form.
- The Data Grid of the board has a pager of its own in its footer ("Page 1 of 8" with ghost buttons of 32), drawn in the frame, not an instance of Pagination: it is the data grid's to build, from `Pagination::controlled(..).compact(true)` with a style of 32 if it likes.

## Not done

- **The focus is lost when the button it is on becomes disabled.** Next pressed onto the last page, by Enter, disables the next button, and the engine blurs a focused element that is disabled (`blur_focus_in_dropped_child` in `crates/lurq/src/app/runtime.rs`); the next Tab starts again from the start of the window. A person expects the focus to stay in the pagination. Needs the engine, where the focused element becomes disabled inside a focus group, to move the focus to the nearest element of the group that takes input (the one before it, else the one after) instead of blurring it. That is a change of focus groups, the Tabs session's to make; the pagination needs nothing else for it.

## Differs from the board

- The board's Pagination is drawn on page 1 of 12 with previous at full strength; here previous is disabled (dimmed, as the board's compact form draws it) on the first page, and next on the last.
- The board's Pagination on page 1 of 12 draws `1 2 3 … 12`, which is what `siblings(0)` draws; the default of one sibling draws `1 2 3 4 5 … 12` (see `## Decided`).
- A page button is a least width of 36 with 6 of padding, where the board's is 36 with none: the same for numbers of up to three digits.
- The board's compact form is a frame 440 wide; here it is as wide as what holds it when it has a page-size select, and as wide as it is otherwise.
