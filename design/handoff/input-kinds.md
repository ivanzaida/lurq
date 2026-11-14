# Password Input, Search Field and Input Group: hand-off

`ui::PasswordInput`, `ui::SearchField` and `ui::InputGroup`, from the board's `Password Input` (`t12ga`), `Search Field`
(`g3gkxg`) and `Input Group` (`oPRo8`) in Components / light (`Ovt1D`), and where they are used: Specialized Inputs /
Values (`o1Rf4T`: the password with its meter), the mobile Password field (`N1Tak`: the eye at the end of a text
field), Forms & Fields / Search (`db7i4`: the search field with its hint, and filled and focused over its
suggestions with the clear button in place of the hint), the table toolbar's search (`nX63b`), and Forms & Fields /
Input groups (`mnyXS`: a prefix, a suffix, both). The `Kbd` (`zGoRU`) at the end of the search field and the icons
`Icon/eye` (`N1JL9T`), `Icon/search` (`w3UcqD`) and `Icon/close-circle` (`MQpqQ`) were read for their sizes.

Files: `crates/lurq/src/ui/password_input.rs`, `search_field.rs`, `input_group.rs`,
`crates/lurq/tests/ui/input_kinds.rs`, `crates/gallery/src/pages/password_input.rs`, `search_field.rs`,
`input_group.rs` (pages "Password Input", "Search Field" and "Input Group", group Inputs),
`design/handoff/input-kinds.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

- `crates/lurq/src/node/node_kind/text_input/styled.rs` and `styled_off.rs`, `TextInputState::clipboard_selection`: a
  masked input copies and cuts nothing (Ctrl+C, Ctrl+X, Ctrl+Insert and Shift+Delete do nothing), as a native
  password field. Before, a masked input put its plain value on the clipboard. Paste is unchanged.
- `crates/lurq/src/ui/input.rs`:
  - `Input::view(self, theme) -> InputView` (crate-private) is what `Input::build` did before it called the renderer;
    `build` now calls it. A component made of an input takes the view, adds to `view.control()` and lays its own parts
    over it. The field `Input::boxed` is crate-private, so that such a component sets the input's states and passes
    a field's (`boxed.in_field`).
  - `InputView::end_room(width)` and `InputView::render_with_end(control, end)` (crate-private): the room the control
    keeps at its end for what lies over it there, and the input's stack with `end` laid over the end of the box (the
    pointer reaches the control around what is in it).
  - `InputView::text_start` and `InputView::icon` read the control's padding at its start (`padding_start` over
    `padding_x`), not `padding_x` alone. Nothing changes for `Input`, whose recipe gives `padding_x` only; the search
    field pads its start 10 and its end 8.
- `crates/lurq/src/ui/text_box.rs`: `BoxStates::role`, what the box is to tooling in place of `textbox` (the search
  field's `searchbox`).

## For `docs/src/content/docs/design-system.md`

## Password Input

An input whose text is masked, with an eye at its end that shows it and hides it again, and under it, where the app says so, a meter of how strong the password is.

```rust
use lurq::ui::{PasswordInput, PasswordStrength};

// password: Signal<String>, shown: Signal<bool>
PasswordInput::new(password.clone())
  .placeholder("Password")
  .bind_reveal(shown.clone());

// A password being chosen, judged by the app as it is typed.
PasswordInput::new(chosen.clone())
  .new_password(true)
  .bind_reveal(shown.clone())
  .strength(judge(&chosen.get()));
```

| Builder | |
| --- | --- |
| `PasswordInput::new(value)` | A password input bound to a `Signal<String>`: what is typed is set on it |
| `reveal(bool)`, `on_reveal(Fn(bool))` | Whether the password is shown as it is; called on a press of the eye with whether it is to be shown |
| `bind_reveal(Signal<bool>)` | Shown while the signal is true, which the eye turns |
| `reveal_labels(show, hide)` | What the eye is called to tooling, in place of "Show password" and "Hide password" |
| `strength(Option<PasswordStrength>)` | The meter under the input: `Weak`, `Fair`, `Good`, `Strong` fill 1 to 4 of its bars |
| `strength_message(&str)` | What the meter says, in place of "Weak password" to "Strong password" |
| `new_password(bool)` | The password is being chosen, not entered: the platform may offer a strong one |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `icon`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- It is an [input](#input) in every other way: its box, its sizes, its judgments, a field's label. The reveal is the app's, as a switch's state is: a `bool` and a handler, or a `Signal<bool>` bound with `bind_reveal`. The eye is drawn where something hears it.
- Tab reaches the input and then the eye, which Space and Enter press. To tooling the eye is a `button` called "Show password" or "Hide password", with `a11y_pressed`.
- Masked, the password is not copied or cut, the platform's keyboard keeps it out of its suggestions, and tooling (`lurq_snapshot`, `lurq_set_value` of the MCP server) reads only its dots and `masked=true`. Shown, it is what the screen shows, to tooling too. Shown or not it holds a password to the platform (`ContentKind::Password`, or `NewPassword`), which may offer to fill it in.
- Parts: `PasswordInputPart::Root` (the input, the meter and its message, one under another), `Mask` (the room between the dots), `Toggle` (the eye), `Strength` (the meter), `Bar` (a bar the strength does not reach), `Filled` (one it does), `Message`. States: `PasswordInputState::Revealed`, `Disabled`, `FullWidth`, and the strength by its name (`weak`, `fair`, `good`, `strong`). The box, its text and its placeholder are the input's parts, styled by the theme's recipe for an input or `input_rules`.
- In the design the dots are 1 apart; the eye is 16 in `MutedForeground`, 12 from the end of the box and 8 after the text, in `Foreground` under the pointer and while the password is shown. The meter is 8 under the box: four bars 4 high and 4 apart, round, in `Input` where the strength does not reach and in its color where it does, `Destructive` for weak, `Warning` for fair, `Success` for good and strong; the message is 8 under it in a caption's type in the same color.
- A renderer that draws another password input has `view.input` (the input's view: `render_with_end` is crate-private, so it lays the eye over `view.control()` itself), `view.control()` (the masked text input with room at its end), `view.toggle()`, `view.strength_meter()` and `view.message()`.

## Search Field

An input for a query: the search icon before the text, a button that empties it, and the hint of the key that reaches it.

```rust
use lurq::ui::SearchField;

// query: Signal<String>
SearchField::new(query.clone())
  .placeholder("Search components")
  .shortcut("/")
  .on_submit(|query| search(query));
```

| Builder | |
| --- | --- |
| `SearchField::new(value)` | A search field bound to a `Signal<String>`, which the app filters by as it changes |
| `shortcut(&str)` | The key that reaches the field, at its end while it is empty: `/`, `Ctrl K`. The app binds the key |
| `on_submit(Fn(&str))` | Called on Enter with what the field holds. The focus stays in the field |
| `clear_label(&str)` | What the clear button is called to tooling, in place of "Clear search" |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- While it holds something a clear button is at its end, which empties it and puts the caret back in it; Escape does the same while the focus is in it, and keeps the focus there. Escape in an empty field does what it does in any input. A read-only or disabled field has no clear button, and Escape does not empty it.
- The clear button is no stop of Tab: from the keyboard Escape does what it does. The hint shows while the field is empty; the field does not bind the key, which the app does and focuses the field on (`ctx.focus`).
- To tooling the field is a `searchbox` and its button a `button` called "Clear search".
- Parts: `SearchFieldPart::Root` and `Control` (what the field says of the input's root and box: its width, its padding), `End` (what is at the end of the box), `Clear`, `Shortcut` (the hint). States: `SearchFieldState::Filled`, `FullWidth`. The box, its text, its placeholder and its icon are the input's parts.
- In the design the field is 280 by 36, padded 10 at its start and 8 at its end; the search icon is 16 in `MutedForeground`, 8 before the text. The clear button is a circled cross of 16 in `MutedForeground`, `Foreground` under the pointer. The hint is 20 high, padded 6, 4 round on `Muted` in an edge of 1 in `Border`, in the mono type of 11 in `MutedForeground`.
- The field renders again as it fills and empties, to show the clear button or the hint: it reads its signal as it is built.
- A renderer that draws another search field has `view.input`, `view.control()` (the text input with its keys and room at its end), `view.clear()` and `view.shortcut()`.

## Input Group

An input joined with what stands before and after it into one box: a scheme, a domain, a unit, an icon, the button that sends it.

```rust
use lurq::ui::{Button, ButtonVariant, Input, InputAddon, InputGroup, Select, icons};

InputGroup::new(Input::new(site.clone())).before("https://").after(".com");

InputGroup::new(Input::new(email.clone()).placeholder("Email"))
  .before(InputAddon::icon(icons::MAIL))
  .after(Button::new("Invite").variant(ButtonVariant::Ghost).on_click(invite));

InputGroup::new(Input::new(site.clone())).before(Select::new(scheme.clone(), [("https", "https://"), ("http", "http://")]));
```

| Builder | |
| --- | --- |
| `InputGroup::new(input)` | A group around an [input](#input), which keeps its own value, handlers and id |
| `before(addon)`, `after(addon)` | What stands before and after the input, in order: a text (`&str`), `InputAddon::icon(name)`, a `Button`, a `Select` |
| `size(InputSize)` | The input's, and each button's and select's in the group |
| `invalid`, `warning`, `valid`, `disabled`, `full_width`, `on_input` | The input's judgment and the box's edge; nothing in a disabled group takes input; 280 wide or as wide as what holds it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- The corners of the box are the group's: what is at its ends is round where the box is, every other corner is square. A line is between two neighbours. The input has no edge or fill of its own: the box's edge says what it holds, and turns the ring's color under the pointer. The ring of the focus is the focused part's own, drawn over its neighbours and the box's edge.
- An icon beside the input is in its box: before it, the input's own icon; after it, over the end of the input. An icon away from the input stands on the muted fill as a text does.
- A button and a select keep what they do and their variant; the group gives them its size and its corners, and a select no edge or fill of its own. Each is its own stop of Tab.
- In a [field](#field) the input is called by the field's label and judged as the field is, and the group is disabled where the field is.
- Parts: `InputGroupPart::Root` (the box), `Field` (what the group says of the input's root), `Control` (of the input's box), `Addon` (a text or an icon on the muted fill), `AddonIcon`, `Separator` (the line), `Button` (of a button's root), `Select` (of a select's trigger). States: `InputGroupState::Disabled`, `FullWidth`, and the input's size and judgment by their names.
- In the design the box is 280 by 36, `Background` in an edge of 1 in `Input`, 6 round. A text addon is on `Muted`, padded 10, in 13 and `MutedForeground` (12 small, 14 large); the line between two parts is 1 in `Input`; the input is padded 10. A disabled group is half opaque on `Muted`.
- A renderer that draws another group has `view.input`, `view.before`, `view.after`, `view.addon(addon, index)`, `view.field()` and `view.separator()`.

## For `CHANGELOG.md`

- `ui::PasswordInput`: an input whose text is masked, with an eye at its end that shows it and hides it again (a `bool` and `on_reveal`, or a bound `Signal<bool>`), and a meter of the password's strength in four bars with what it says of it; a masked text input no longer copies or cuts its value.
- `ui::SearchField`: an input for a query with the search icon before its text, a clear button while it holds something, Escape that empties it and Enter that submits it, and the hint of the key that reaches it; a `searchbox` to tooling.
- `ui::InputGroup`: an input joined with texts, icons, a `Button` and a `Select` before and after it into one box, with the group's corners at its ends and a line between neighbours.

## Decided

- **Builders over an input, not new boxes.** Each of the three holds an `ui::Input` and builds from its view (`Input::view`): the box, the sizes, the judgments, read only, disabled, a field's label and the ring are the input's, styled by the theme's recipe for an input. What a component says of the input (the search field's width and padding, the group's input with no edge) is its own part, laid over the input's recipe with `rules_for`, as a button group does for its buttons. A theme's renderer for `Input` is not used by them: each has a renderer of its own, whose view holds the input's.
- **The search field is not an input with three props.** It has a clear button, a hint, keys and another role to tooling, so it is a builder of its own over `Input`, with the search icon as the input's own icon.
- **The reveal is the app's**, as the brief asked: `reveal(bool)` with `on_reveal`, or `bind_reveal(Signal<bool>)` (as `Chip::bind`). A builder keeps nothing. The eye is drawn only where something hears it: an eye that does nothing would be a dead button.
- **Shown, the password is readable to tooling.** The board draws only the masked password and says nothing of one shown; shown, it is on the screen, so it is what the screen shows. The input keeps `ContentKind::Password` while shown, so the platform still treats it as a password. Masked, the engine's text input now copies and cuts nothing (the change above), which a native password field does not either.
- **The eye is the same glyph shown or hidden**, in `Foreground` while the password is shown: the icon set has no eye with a line through it (under "Not done").
- **The strength is the app's judgment**, four steps: weak 1 bar destructive, fair 2 warning, good 3 success, strong 4 success. The board draws one sample, three bars in success under "Strong password"; a meter whose strongest step fills three of four bars would never be full, so strong fills four. The message is under the meter, as the board draws it, and is the meter's, not a field's description.
- **The clear button is no stop of Tab**: Escape empties the field from the keyboard, and the button exists only while there is text. A press of it puts the caret back in the field (`.focuses`).
- **The hint is laid over the end of the box before it is laid out**, so the box keeps room for it by its characters at 0.6 of the size of its type, a mono cell, and its padding. It shows only while the field is empty, so only a long placeholder could run under it.
- **Enter submits and keeps the focus** where the app listens for it (`on_submit`); without a handler Enter does what it does in any input (submits a form, leaves the input). Escape that empties the field keeps the focus; Escape in an empty field leaves it.
- **The group's edge is the box's**, painted over its parts, and the input in it has none, so the line between the input and an addon is the addon's separator, as on the board. The focus ring, an outline, is drawn at the end of its scope, over the box's edge and the neighbours: the focused part's ring is whole.
- **An icon beside the input is in its box**, as the brief asked: before it, `Input::icon`; after it, over the end of the input's box. The group's input takes the room the addons leave in a row that grows (`flex(1.0)`), its root as wide as that row.
- **A disabled group is dimmed once**, on its box: the input, buttons and selects in it are disabled and not dimmed again.

## Not done

- **An eye with a line through it while the password is shown.** Neither the board nor the icon font has one. Smallest change: draw `Icon/eye-off` on the board beside `Icon/eye` (`N1JL9T`), run `python scripts/build-icon-font.py --from-board`, add `icons::EYE_OFF`, and in `PasswordInputView::toggle` take `icons::EYE_OFF` while `revealed`.

## Differs from the board

- Shown, the eye is the same glyph in `Foreground`; the board draws no shown state (under "Not done").
- The board's meter shows three bars under "Strong password"; lurq's strong fills four, and good three. The destructive and warning colors of a weak and a fair password are lurq's: the board draws only success.
- The board draws the shortcut as a `Kbd` component; lurq's is a part of the search field with the Kbd's numbers. There is no `ui::Kbd`.
- The board's input groups have text addons only; the icon, button and select addons, the edge under the pointer and the edge of a judgment are lurq's, in the input's colors.
