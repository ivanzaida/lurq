# Decisions

What was decided about lurq, by whom and why, newest first. A decision is written here the day it is taken, before
the work on it starts, so that the work, its review and a later session read the same thing. The design system's
first decisions (D1–D13: where components live, palette roles, recipes, breakpoints) are in
[design-system-plan.md](design-system-plan.md#3-decisions) and are not repeated; a decision here that changes one of
them says so. Decisions about phones and tablets are in [mobile-decisions.md](mobile-decisions.md).

Each entry: **the question**, **the decision**, **why**, and its **state** (decided, in work on a branch, built,
dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-20

### Every new component is shown in the gallery

- **Question.** `ui::StatusLabel` came with plan K4's item 7 with no gallery page. Is a component done without its page?
- **Decision (Ivan, 2026-11-20: "нові компоненти які додаєш мають бути додані в галерею лурка", "і додай це в
  агент мд і десіжни").** No. Every component lurq adds gets its page in the gallery before its branch is merged.
  Written in `AGENTS.md`.
- **Why.** The gallery is where a person and an agent see what lurq has and try it; a component missing from it is
  one nobody finds.
- **State.** In force. `StatusLabel`'s page is added on `claude/k4-f-misc`.

### Pressables that Tab passes by, and a press that opens beside what is open

- **Question.** Orchester's desktop app reaches its buttons, rows, tabs, segments and breadcrumb links with the
  pointer and with their own shortcuts, never with Tab: a press of one leaves the focus where it was, and Tab goes
  from field to field of a form. A `ui::Button`, `ListItem`, `Tab`, `Segment` and a breadcrumb's link are all stops
  of Tab, and a press focuses them, so the app could not move its kit onto them (plan K4, item 1: `button` with 403
  call sites, `icon_button`, `breadcrumb`, `tabs`, `row_group`, `setting_rows`). Its links also open as a browser's
  do: the middle button, or a press with Ctrl held (Cmd on a Mac), opens the place in a new tab and does not do what
  a plain press does. Should lurq's pressables do both, and how?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** Every pressable component of the item takes
  `.tab_stop(false)`: Tab passes it by and a press of it with the pointer leaves the focus where it was; it still
  takes the focus when the app gives it (`Ctx::focus`), Enter and Space still press it then, the arrows of a focus
  group it is in still move to it, and its role and name are as they were. On a component of several (`Tabs`,
  `SegmentedControl`, `Breadcrumb`) it is said once for all of them. The engine gets the two halves as two props of
  an element, `tab_stop(bool)` and `focus_on_press(bool)`. A `Button`, a `ListItem`, a `Tab` and a `BreadcrumbStep`
  take `.on_open_aside(handler)`, the name `SidebarItem` has: the middle button, or a primary press with Ctrl held
  (Cmd on a Mac), calls it in place of the press, and nothing else happens: no focus, no `on_click`, no selection,
  no submit, and the press does not reach what is around it. It is given the press, so the app tells a press with
  Shift held (a tab it switches to) from one without. `SidebarItem::on_open_aside` stays as it was, with no event.
- **Why.** What Orchester's kit does (`ui/pressable.rs`, `navigation/link.rs`: `Focus::Pointer`, `navigates`) is a
  rule of many desktop apps, and can be said once in lurq rather than worked around with `focusable(false)`, which
  also kept the focus a shortcut or a test gives. The event and not a bare call because the app's rule for where a
  page opens reads Shift.
- **State.** Built on `claude/k4-a-pointer-only` (2026-11-20).

### A tag's remove button that Tab passes by

- **Question.** Orchester's chips have a remove button that Tab does not stop at (plan K4, item 11); a `ui::Tag`'s
  is a stop of Tab. Should it be able not to be?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `Tag::tab_stop(false)`, by the mechanism of the
  entry above: its remove button is passed by Tab and a press leaves the focus where it was; focused by the app, a
  click, Enter, Space, Backspace and Delete still take the tag away.
- **Why.** The same rule as Orchester's other pressables, on the one pressable a tag has.
- **State.** Built on `claude/k4-a-pointer-only` (2026-11-20).
### Any component of `lurq::ui` says what the app gives it to tooling, on its root

- **Question.** Orchester's UI kit says the state of its elements to tooling that reads the tree (DevTools, an
  agent through MCP, its tests) with `describe`: a callout its `tone`, a meter its `share`, a tag its `tag`. A
  component of `lurq::ui` has no `describe`: what the kit moves onto `Callout`, `Badge` or `Progress` would lose what
  the MCP tree reads of it. Should every component take a `describe`, and where?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 9 of Orchester).** Every builder of
  `lurq::ui` takes `describe(name, value)` through `Styled`, the trait every builder already implements, as it takes
  `style`: what is described is carried by the instance's `Recipe` (`Recipe::describe`) and said on the node of the
  component's root part when its renderer gives that node the part's style, so no component writes it by itself and
  a renderer of a theme's says it too. What the app says comes before what the component says of itself, and
  tooling that reads one value of a name reads the app's.
- **Why.** The kit's elements keep the state their tests and an agent read when they become `lurq::ui` components,
  without a method on each of nearly two hundred builders.
- **State.** Built on `claude/k4-b-describe-avatar-spinner`.

### An avatar takes the letters it shows from the app

- **Question.** `Avatar::new(name)` shows the initials it takes from the name (the first letter of its first word
  and of its last). Orchester shows the first letters of the first two words (`Ada Lovelace` → `AL`, and a name of
  three words differs), and a project's single letter. Giving lurq the letters as the name made what tooling calls
  the avatar "A L". Can the app give the letters, and keep the name?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 10 of Orchester).**
  `Avatar::initials(letters)` puts the app's letters on the face in place of those taken from the name; the name
  stays what tooling calls the avatar. Empty letters show the icon of a person, as a name with no letters does.
- **Why.** How a name becomes letters is the app's (its languages, its design); who the avatar is, is the name.
- **State.** Built on `claude/k4-b-describe-avatar-spinner`.

### A spinner may turn in steps, and stand still where motion is reduced

- **Question.** A spinning icon and `Spinner` (its ring, its dots, its bars) move smoothly: the window draws every
  frame while one is shown, and they move under reduced motion too (lurq's choice: a spinner that stood still would
  say nothing). Orchester's loader turns in 24 poses a second, so the window draws 24 frames a second and none
  between, and its typing dots stand still where the system asks for reduced motion. Can lurq's do both?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 12 of Orchester).** A `SpinMotion`
  says how they move: `SpinMotion::SMOOTH` (the default, as before), or `SpinMotion::stepped(per_second)`, a pose held
  until the next and none drawn between, the dots' and the bars' offsets put on the same steps so that every one of
  them changes at the same moment; and `.still_when_reduced(true)`, which leaves them in their pose at rest where
  motion is reduced. An `Icon` takes it with `spin_motion`, a `Spinner` with `motion`, and a theme gives the one every
  spinner without its own takes (`Theme::set_spin_motion`), the ring of a loading button too. A spinner still moves
  under reduced motion unless it is told otherwise.
- **Why.** A window with a loader shown draws as few frames as its poses need, and an app whose design holds its
  loaders still for reduced motion can say so once, for lurq's spinners and its own.
- **State.** Built on `claude/k4-b-describe-avatar-spinner`.
### An alert has several actions, at its end or under its words

- **Question.** `ui::Alert::action` took one element, and a second call took the place of the first. Orchester's
  notices (`notice`, 101 call sites, and `setting_notice` in Settings) are a toned box with an icon, a title, what
  happened, and two or three buttons: at the notice's end, or under its words where the words are long (a stacked,
  flat or raised notice, every notice of Settings). Can an alert hold several actions, and where do they stand?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 2 of Orchester).** An alert has
  several actions, as a notification, an outcome and an empty state do: `action(..)` adds one after those there,
  `actions(..)` adds each of several, the least first and the main one last. They stand together in the part
  `action` (its name kept, so that a theme's rule on it still holds), at the alert's end as before, or, given
  `stacked(true)` (state `stacked`), under the description and the body at the start of the words, wrapping where
  one line does not hold them. A renderer is given them as `AlertView::actions`.
- **Why.** Orchester's kit builds its notices on `ui::Alert` only if the alert holds what a notice holds; a notice
  of one button is the exception there, not the rule. The two places are the two the kit draws.
- **State.** Built on `claude/k4-c-alert-card` (2026-11-20): `Alert::action` adds, `Alert::actions`,
  `Alert::stacked` and `AlertState::Stacked`, `AlertView::actions` (field and method, in place of `action`). A
  breaking change of the renderer's view and of a second `action` call, in the changelog. What the kit's Settings
  notice has after its actions (a quieter note) is not part of it.

### A card takes the layout of what is in it from its own builder

- **Question.** Orchester's `card` is a `Column` dressed as a card, and its 170 call sites chain the column's
  builder on it: `padding`, `spacing`, `align_items`, `width`, `height`, `flex`, `min_width`, `describe`, children.
  `ui::Card` builds into an element whose children are two levels down (its root, its padded content, its body),
  so none of those calls reaches them. Should a card take them, and to which of its parts do they go?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 13 of Orchester).** The card itself
  takes them, as it took `width` already: what lays out what is in it goes to its content and its body
  (`padding` is the content's, `gap` the room between the header and the body and between the children,
  `align_items` the children's across the body), and what places the card in what holds it goes to its root
  (`width`, `height` and their least and most, `flex`, `flex_shrink`, `absolute`, and `describe` for tooling). What a
  part's style has (padding, gap, sizes) is laid as a rule of the card over the theme's recipe, so a theme's renderer
  sees it in the part; the rest is laid on the node after the part's style. `Card::new().padding(..).gap(..)
  .child(..)` is then a card as the kit's call sites write it. The other surfaces (popover, sheet, hover card) are
  not changed: no call site of the kit asks it of them, and they take it the same way when one does.
- **Why.** A card the app cannot lay out as it lays out a column is not used: the kit kept its own column because of
  it.
- **State.** Built on `claude/k4-c-alert-card` (2026-11-20): `Card::padding`, `gap`, `align_items`, `height`,
  `min_width`, `max_width`, `min_height`, `max_height`, `flex`, `flex_shrink`, `absolute` and `describe`, on a read
  card and a pressed one. A length in percent is not a `Length`, so a call site's `Dimension::Pct(100.0)` is
  `Length::Fill`.

### A choice of a segmented control or a radio group has an id, a count, states and an element ref; a tab a reason and a badge

- **Question.** Orchester's segmented controls give each segment an id of its own (its tests and the MCP tree read
  `<id>-label` and `<id>-count`), a number after the label (*All 24*), a state the design draws (a choice that
  warns), and an `ElementRef` that a popup the segment opens anchors to; its tabs have a tab that is not built yet,
  dimmed with a lock and the reason in its tooltip, and a count of what waits for the person in a badge after the
  label. A `ui::Choice` has a value, a label, an icon and `disabled`; a `ui::Tab` has a count but no badge, and a
  disabled tab says nothing of why. Can lurq's choices and tabs hold what the kit's do (plan K4, item 5)?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** A `Choice` takes `id(..)` (the segment's or the
  radio's id, after which its parts are named: `<id>-label`, `<id>-count`), `count(..)` (a number after the label,
  in a part `count`), `state(name, on)` (a state of the theme's recipe on that choice alone) and
  `element_ref(ElementRef)` (on what is pressed: the segment, the radio's mark), beside `disabled` it had.
  `SegmentedControl` and `RadioGroup` draw all of them; a `Radio` takes `count` and `element_ref` too. A `Tab` takes
  `disabled_reason(&str)`: it is disabled, its reason is its description to tooling, and the tabs put in the tree
  with `mount` show it as the tab's tooltip; and `badge(..)`: a count in a pill after the label (part `badge`), for
  what waits for the person, where `count` stays the quiet number of what is there.
- **Why.** The kit's segments and tabs are found by the ids of their parts, and their counts, warnings, reasons and
  anchors are what they say and do; without them the kit keeps its own widgets.
- **State.** Built on `claude/k4-e-choices-dialog` (2026-11-20): `Choice::id`, `count`, `state` and `element_ref`,
  drawn by `SegmentedControl` and `RadioGroup` (parts `count` of each, `Radio::count` and `Radio::element_ref`), a
  choice's id and state taken by `ToggleGroup` and `ChipGroup`; `Tab::disabled_reason` (tooltip and
  `a11y_description`, `TabsView::tab_element`) and `Tab::badge` (part `badge`). The menu of the overflowing tabs shows
  a tab disabled for a reason as disabled, without the reason.

### A dialog has a mark, a note in its footer, and a confirm on Enter; a field an error icon and its control at the end

- **Question.** Orchester's confirmations (`dialog`, plan K4 item 6) have a tile with an icon before the title, a
  quiet note at the footer's start (why the action cannot be taken, what Esc does), and Enter runs the main action.
  Its forms show an error with a `circle-alert` before the words, and its settings put the label and the
  description on the left and the control at the row's end. `ui::Dialog` has a title, a description, a body and a
  footer of buttons; `ui::Field` has its error as words alone, and beside its label only the form with the label in
  a column of 110. Can they?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `Dialog::icon(name)` draws an icon in a well
  (part `mark`, its glyph `icon`) before the title, and `Dialog::media(element)` puts the app's element in its place,
  state `mark`; `Dialog::note(&str)` with `note_icon(name)` puts quiet words at the footer's start, the buttons at
  its end (parts `note`, `note_icon`, state `note`); `Dialog::on_confirm(handler)` runs on Enter while the dialog is
  open, unless the focus is on something Enter does a thing of its own with (a button, a link, a select, a text
  input of several lines) or an input method composes. The engine tells it: `KeyboardEvent::focus_takes_enter`.
  `Field::error_icon(name)` draws an icon before the description while the field is invalid (part `icon`), and
  `Field::control_end(true)` (state `control_end`) lays the label and the description in a column that takes the
  row's room (part `text`) and the control at the row's end.
- **Why.** The kit builds its confirmations and its settings rows on lurq's dialog and field only where they hold what
  the design draws. A focused button keeps Enter as a press of itself, so a dialog that opens with the focus on
  Cancel (a question whose answer destroys) is not confirmed by Enter, as the playbook's rule of the focus wants.
- **State.** Built on `claude/k4-e-choices-dialog` (2026-11-20): `Dialog::icon`, `media`, `note`, `note_icon` and
  `on_confirm` (parts `mark`, `icon`, `note`, `note_icon`, states `mark` and `note`; `DialogView::mark`, `note`,
  `confirmer`), the engine's `KeyboardEvent::focus_takes_enter`; `Field::error_icon` (part `icon`) and
  `Field::control_end` (part `text`, state `control_end`). The mark's well has one look; a tone (warning, info,
  danger) is the app's rule on the part `mark`.
### A tooltip that wraps where the app says, has a note, is in mono, and shows only while a text is cut

- **Question.** Orchester's tooltip (`ui/tooltip.rs` of its kit) wraps its words at a width of the app's (400, and
  220 with a note), says a second line under them (a note, in regular weight), is set in the mono family where it
  says a path or a command, and, on a label cut to its room with an ellipsis, shows only while the label is cut,
  saying it whole (`Truncated`). Its icon buttons have no words, and their label is their tooltip wherever they
  stand. `ui::Tooltip` wraps at the recipe's 200, says one text, has no mono form and shows whenever it is asked;
  an icon-only `ui::Button` has its label as a tooltip only inside a `Toolbar`. Can lurq's do what the kit's does
  (plan K4, item 3 of Orchester)?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 3 of Orchester).**
  `Tooltip::max_width(length)` is the widest its box is before its text wraps, over the recipe's.
  `Tooltip::note(text)` puts a second line under the text, in the part `note` (state `note`): the text is then its
  title, in a medium weight. `Tooltip::mono(true)` (state `mono`) sets its text in the theme's mono family.
  `Tooltip::when_cut(&text_ref)` shows it only while the text whose ref it is is cut: drawn in fewer letters than it
  has, by an ellipsis or by a limit of lines; what the pointer and the focus do is as before, and a text that fits
  shows nothing. For it the engine tells an element's ref whether its text is cut: `ElementRef::text_cut()`, read
  from what the layout drew. `Button::mount(ctx)` builds a button with a tooltip: an icon-only one with a label has
  its label as its tooltip, and `Button::tooltip(Tooltip)` gives it one of the app's (a shortcut, a placement, or a
  tooltip for a button with words).
- **Why.** The kit moves its tooltip and its icon button onto lurq's only if they do what its call sites ask of
  them; the cut text is read from the layout that cut it, not from a second, unseen copy laid out to be measured, as
  the kit does now.
- **State.** Built on `claude/k4-d-tooltip-menu` (2026-11-20): `Tooltip::max_width`, `note`, `mono`, `when_cut`,
  `TooltipPart::Note`, `TooltipState::Note` and `Mono`; `ElementRef::text_cut`; `Button::mount` and
  `Button::tooltip`. A tooltip mounted on a bare text now holds it in a row that gives way, so the text is cut in it.
  What the kit's `Truncated` also does is not part of it: a tooltip worded for whether the text is cut (`hint`), and
  a label laid out as a whole control (`framed`); the app reads `text_cut` for the first and lays out its own row for
  the second.

### A menu item with a detail, a reason it cannot be chosen and an element before its label; a popover and a menu kept inside an element

- **Question.** Orchester's menu rows (`ui/menu.rs` of its kit) have a line under the label that says more (an
  agent's model, a branch's remote), a row that cannot be chosen says why, faded, with a lock at its end, and some
  have an avatar or a mark before the label in place of an icon. Some of its popovers and menus must stay inside the
  column they belong to, not only inside the window (`popover_inside`). `ui::MenuItem` has a label and an icon of the
  theme's set, a disabled item says nothing of why, and an overlay is kept only inside the window. Can lurq's do
  these (plan K4, item 4 of Orchester)?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти", plan K4 item 4 of Orchester).**
  `MenuItem::detail(text)`: a second line under the label (part `detail`, state `detail`), the item as high as its
  two lines. `MenuItem::unavailable(reason)`: the item cannot be chosen (as `disabled`), it is in the state
  `unavailable`, ends in a lock (part `lock`) and says the reason as its detail where it has none of its own; tooling
  reads the reason as the item's description. `MenuItem::leading(element)`: an element before the label in place of
  the icon (part `leading`). The engine's overlay takes `within(element_ref)`: where its collision strategy flips
  and clamps it, it does so inside that element's box as it was last laid out, and inside the window as before;
  `Popup`, `Popover` and `Menu` take it too (`within`).
- **Why.** The kit's menus and popovers move onto `ui::Menu` and `ui::Popover` only if a row can say what the kit's
  rows say, and a panel can stay where the design keeps it.
- **State.** Built on `claude/k4-d-tooltip-menu` (2026-11-20): `MenuItem::detail`, `unavailable`, `leading`,
  `MenuItemPart::Leading`, `Detail`, `Lock`, `MenuItemState::Detail`, `Unavailable`; `Overlay::within`,
  `Popup::within`, `Popover::within`, `Menu::within`. Not part of it: the kit's detail on the label's line
  (`inline_detail`), a detail in the warning color, a reason said once in a menu's foot (`reason_below`), a reason
  without the lock (`without_lock`), the kit's hint at a row's end and its 40-high row: they are styles of the parts
  a theme gives (`detail`, `lock`, `root`), or the app's.
### A status label: an icon and toned words, with no box

- **Question.** Orchester says what a run or a step is doing with a status (`ui/status.rs`, every table row and page
  header): an icon and a word in one tone, 5 apart, with no fill and no edge, the icon turning while the run works
  (`Running`, `Working`), cut to its row's room. A `ui::Badge` is a filled pill in every variant, so the kit could not
  move its status onto lurq (plan K4, item 7). Should lurq have one, and as what?
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** A component of its own, `ui::StatusLabel`: an
  icon of the theme's set and a label in the tone of its variant (neutral, muted, info, success, warning,
  destructive), no box; `spin(true)` turns the icon as a spinner does, by the theme's `SpinMotion` or its own; two
  sizes; the label is one line, cut with an ellipsis where its room is short. To tooling it is a `status` called by
  its label.
- **Why.** A badge's recipe is a filled box in every variant; a label with no box is another component, not a
  variant that takes the box away again.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### A radio card holds the app's content, its mark, why it is disabled, and that it waits

- **Question.** Orchester's option card (`ui/option_card.rs`) is a radio card that holds more under its words (a
  picker, starter steps), shows an app's mark in place of an icon (the maker of an agent's models), says why it cannot
  be chosen (a lock in place of its dot and the reason under its words), and is held while the work it chose runs
  (dimmed, its dot kept). `ui::RadioCard` has none of the four (plan K4, item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** A radio card and a checkbox card (they share
  their props) take `content(element)`, any element under the description, in the part `content`; `mark(element)`,
  an element of the app's in place of the icon; `disabled_reason(&str)`, which disables the card, says the reason
  under its words (part `reason`) and to tooling, and shows a lock (part `lock`) in place of its mark or box; and
  `loading(bool)` (state `loading`), a card that takes no press while what it chose is under way, dimmed, its mark or
  box kept, and busy to tooling.
- **Why.** What the kit's card holds is what a settings page's choices hold in many apps; each is a small prop of
  the card the recipe styles.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### A code block copies another text than it shows, and says when the copy failed

- **Question.** Orchester's code box (`ui/code_box.rs`) copies a value and says Copied, or that it could not copy
  when the clipboard refused it; some of its boxes show a value and copy another (a command shown short, copied
  whole). `ui::CodeBlock` asks the window to copy, never hears whether it did, and copies what it shows (plan K4,
  item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `CodeBlock::copy_text(&str)` is what its button
  puts on the clipboard in place of the code it shows (`on_copy` is told it). A mounted block hears whether the
  clipboard took it: if not, the button shows an alert glyph in the destructive color, named by
  `copy_failed_text` (the locale's `EditingWords::copy_failed`, "Couldn't copy"), state `copy_failed`, for as long as
  Copied is shown. The window gets a way to ask for a copy and be told whether it was written.
- **Why.** A press that says Copied when nothing was copied is a lie the kit does not tell; and the text a person
  needs on the clipboard is not always the text that fits the box.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### A calendar has a footer

- **Question.** Orchester's date picker draws Today and Clear under its month (`ui/date_picker`). A `ui::Calendar`
  has nothing under its grid (plan K4, item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `Calendar::footer(element)`: an element of the
  app's under the months, across the panel, in the part `footer` (a line over it and room above it, as the kit
  draws it). What is in it is the app's.
- **Why.** What goes under a calendar differs from app to app; a slot is all lurq needs to give.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### A tag with a term before its label

- **Question.** Orchester shows pairs as one chip, a term and its value (`branch: main`). A `ui::Tag` has one label
  (plan K4, item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `Tag::term(&str)`: a term before the label, in the
  part `term`, quieter than the label (regular weight, the muted foreground on a neutral tag). The term is shown as
  the app writes it, its punctuation too (`branch:`), so no word of lurq's stands between them.
- **Why.** What separates a term from its value differs by language and by app; the app writes it.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### A breadcrumb's step in a state of the app's

- **Question.** Orchester's crumbs are drawn in ways of their own: a key in mono, the page's title in a title's
  type, a last crumb drawn as an ancestor (a folder whose file the page's title names). A `ui::BreadcrumbStep` is
  drawn as a link or as the page and nothing else (plan K4, item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `BreadcrumbStep::state(name, on)` puts one step
  in a state the theme's recipe names; the parts of that step (its link or page, its icon) are resolved in it, and
  the other steps are not. A theme says what the state looks like (`.when(Selector::new().state("ancestor"), Page,
  ..)`), as for any state of a component.
- **Why.** The ways a step is drawn are the app's design; lurq gives the step a state, as it gives a whole
  component one.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

### SVG icons in the theme's icon set

- **Question.** The theme's icon set draws a glyph of a font for a name. Orchester has marks that are SVG (GitHub's,
  the makers of agents' models: `ui/github_mark.rs`, `ui/agent_mark.rs`), so they could not be icons of its set
  (plan K4, item 7).
- **Decision (Ivan, 2026-11-20: "давай роби тоді ці всі пункти").** `ThemeIcons::set_svg(name, IconSvg)` (and
  `Theme::set_icon_svg`) gives a name an SVG in place of a glyph: `IconSvg::stroked(svg)` draws its lines in the
  icon's color (lucide's icons), `IconSvg::filled(svg)` its shapes, and `IconSvg::colored(svg)` keeps its own colors
  (a brand's mark). An `Icon` of that name draws it at its size, in its color or in the foreground of what it is in,
  and turns it where it spins. The engine paints an SVG in the foreground its node inherits where its data asks
  (`SvgData::fill_current_color`, `stroke_current_color`). SVG is drawn with the feature `canvas`; without it such
  an icon takes its room and draws nothing.
- **Why.** An app's marks are drawn by name wherever lurq's components draw icons, as its glyphs are.
- **State.** Built on `claude/k4-f-misc` (2026-11-20).

## 2026-11-19

### A component's id is the prefix of the ids of its parts

- **Question.** A component of `lurq::ui` given `.id(x)` put `x` on its root and nothing on the nodes of its other
  parts: a test, DevTools or an agent driving the app through MCP could find the badge, and not its label, a
  callout's title, a progress's fill or the term of one item of a data list, except by walking the tree. Should
  the parts be found by id too, and by which ids?
- **Decision (Ivan, 2026-11-19: "треба додати можливість ставити ід компоненту, а діти юзають його як префікс").**
  A component given `.id(x)` keeps `x` on its root and gives the node of every other part `x-<part>`, the part's
  name as its `names!` table writes it (`x-dot`, `x-label`, `x-title`, `x-fill`). A part on several nodes of one
  build is `x-<part>`, `x-<part>-2`, `x-<part>-3`, in build order. A node that has an id keeps it, and an item the
  app gave an id `y` prefixes its own parts with `y` (`y-term`, `y-value`). A component of `lurq::ui` built inside
  another and given no id by the app takes the id of the part it stands for (`x-close`), and so prefixes its own
  parts. A component with no id gives its parts none. The ids are the same on every render of the same content.
- **Refined (Ivan, 2026-11-20: "так давай").** A repeated part that has a key of its own is named by the key, not by
  its place: a day of a calendar `x-day-2026-11-20` (its ISO date), a weekday `x-weekday-mon` (the English short name
  in lower case, an identifier and not a word of the locale), the buttons of a calendar's sides `x-nav-previous` and
  `x-nav-next`, a preset, an option, an item, a tab or a segment by its own key or value where the component has
  one. The numbering by build order (`-2`, `-3`) is only where a repeated part has no key. A component built inside
  another takes `x-<its own short name>` where that is the only one of its kind there: a date picker `due` whose
  panel holds a calendar has the calendar `due-calendar` (not `due-panel-calendar`), its parts `due-calendar-title`
  and `due-calendar-day-2026-11-20`; the part's name is used where two would otherwise be named alike. A part's name
  goes into the id exactly as its `names!` table writes it, its underscores too (`x-trigger_icon`, `x-week_number`):
  a first refinement that made them hyphens was dropped ("це тупо і не очевидно").
- **Why.** What finds a node by id (`Tree::get_element_by_id`, a test, DevTools, an agent) reaches every part of a
  component the app named, with no id written by hand for each and no walk of the tree, and the names are the ones
  a recipe already styles the parts by.
- **State.** In work on `claude/part-ids`.

### The browser's work, in this order, with text through an input method and a store in localStorage

- **Question.** The sweep of the gallery in a browser (192 pages, WebGPU and WebGL2: no console error, every page
  drawn; the notes of the session that made the browser supported) left a list of work. In what order, and does it
  take two things the decision below did not name: text that comes through an input method, and the persistent
  store?
- **Decision (Ivan, 2026-11-19).** The order the session proposed ("Так, у цьому порядку"): the canvas's size and
  scale; the size of the page and its first frame; fonts; the network; sound; files; then text through an input
  method (a hidden field, as on iOS: "Так, у фазі 2"); decoding pictures off the frame; the clipboard's pictures;
  pointer, touch and pen; the persistent store over `localStorage` ("Зробити через localStorage"); the gallery's
  own gaps. The fonts are Inter and JetBrains Mono, from their projects' releases, under the SIL Open Font
  License, kept in the repository so that the web gallery can be put on a server. `wasm-opt` (binaryen) is not
  downloaded: the size is what cargo's profile and wasm-bindgen make.
- **State.** In work on `claude/browser-support-926cf3`.

### The browser becomes a supported platform

- **Question.** The gallery builds for the browser (WebGPU, WebGL2 with `webgl`) and runs, but the browser was "a
  build that exists, not supported": no video or sound, no network for what loads an address, one thread (pictures
  decode between frames), a clipboard of text only, file dialogs and `open_url` unchecked, fonts copied from
  Windows that may not be published, gestures and the pointer unchecked there, nothing measured.
- **Decision (Ivan, 2026-11-19: "Робимо підтримку").** The browser joins Windows, macOS and iOS as supported: every
  page of the gallery works there, what a desktop has works there or has the browser's own equivalent, and what
  cannot is said in the docs as the platform's limit, not a defect. The README and Getting Started say so once it
  holds. The decision of 2026-11-17 on video (no copy of each frame) stands unless Ivan changes it.
- **State.** Built on `claude/browser-support-926cf3` (2026-11-19), to be merged into `feat/design-system`: every
  page of the gallery runs in Chromium through WebGPU and WebGL2 with no error; sound, loads over `fetch`, files,
  composed text, browser-decoded pictures, the clipboard's pictures, touch and pen, a store in `localStorage`, fonts
  the gallery may publish, a page built for its size. README and Getting Started list the browser as supported;
  `web.md` says what a page cannot do. Firefox, Safari and phones are checked by hand with
  `design/browser-checks.md`; a video waits as decided on 2026-11-17.

## 2026-11-18

### A link preview's picture: plated when transparent, whole when wide in the compact form

- **Question.** A page's picture that is dark on nothing (Wikipedia's Rust article gives the black Rust logo, a
  transparent PNG) read dark on dark in the dark theme, inside the picture's muted box; the site's icon had been
  given a white plate for the same reason (`e0839086`). And in the compact form's 72 square, GitHub's 2:1 repo card
  showed only "…ida/lurq", cut from its middle.
- **Decision (Ivan, 2026-11-18).** A preview's picture that has transparency (a logo, as Wikipedia's) stands on a
  light plate, in both themes, as the site's icon does. In the compact form's 72 square a picture much wider or taller
  than square (beyond 1.5:1, as GitHub's 2:1 repo card) is drawn whole (contain); one near square keeps the cover
  crop. The card form keeps its ratio and crop.
- **How (the session's choices).** The plate is white, through the recipe (state `transparent` of `LinkPreview`, on
  its `media` part), so a theme restyles it. Transparent is one pixel in a hundred or more not opaque
  (`ImageData::has_transparency`): a photo's rounded corners do not make it so. A transparent picture is contained in
  every form, 16 from the plate's edges in a card and 8 in a row: a logo's edges are its shape, and cover cut the top
  and the bottom off a square logo in the card's 1.91 : 1 box. A wide opaque picture in the compact square is
  contained in the picture's own muted box, not on the white plate: the bands beside a photo read as its box in both
  themes, where white ones would glare beside a dark photo in the dark theme.
- **State.** Built, on `feat/design-system`.

### On Windows a run of typing is one step of undo, as on a Mac

- **Question.** "Undo takes back what was typed as the platform's own fields do" (2026-11-17, below) took Windows'
  fields to make each word typed a step, from what is known of WinUI's `TextBox`, Notepad and Word; nobody had seen
  it. The Mac's handoff (`design/handoff/apple-to-pc-2026-11-18.md`) asked the PC to compare and change
  `UndoGrouping::PLATFORM` if Windows does otherwise.
- **What Windows does.** Asked of Windows' own engines on Windows 11 (build 26100), in hidden controls of a
  script's own, with no input sent to another program ([evidence](../docs/evidence/windows-undo-grouping/README.md)):
  Windows' RichEdit (`Msftedit.dll`: WordPad's, and what XAML's `TextBox` and `RichEditBox` are built on) and
  Office's RichEdit that Windows 11's Notepad ships and types into both take back `hello big world` in one step,
  spaces, punctuation and a pause of 1.5 s notwithstanding; moving the caret ends a step, and in Notepad's so does
  turning from typing to deleting. No field makes a word a step; Word does, and Word is not a field. The classic
  edit control has one level of undo, a toggle, and is no model.
- **Decision (the session on the PC, as the handoff asked, 2026-11-18).** Windows groups as a Mac does:
  `UndoGrouping::PLATFORM` is `Runs` on every system. `Words` stays for a test and for an app that would group as
  Word, which no platform's field does. The pause that ends a step (`App::set_typing_undo_pause`, a second unless
  the app says other) is lurq's own and stays on every system.
- **Why.** "On Windows as on Windows" (Ivan, 2026-11-18): what Windows' fields do, now seen, is a run.
- **State.** Built on `claude/apple-to-pc-5b0fa9`; to be merged into `feat/design-system`. What Notepad's window
  itself adds on top of its engine (a host may end a group, `EM_STOPGROUPTYPING`) was not seen: driving another
  program is not done here. Ivan may type `hello big world` in Notepad and press Ctrl+Z to see it.

### The frames of a card say their colors

- **Question.** A frame of gray and color is made red, green and blue by a matrix (BT.709, BT.601, BT.2020) and in a
  range (the video one, or the full). Apple's frames say both, and the `wgpu` shader is told them
  (`shaders/yuv.wgsl`, the instance's `opacity.zw`); Windows' `CardFrames` said nothing, and both renderers drew
  them as BT.709 of the video range, which the `dx12` HLSL had written in. A video of standard definition (BT.601),
  of HDR (BT.2020) or of the full range came out in the wrong colors.
- **Decision (Ivan, in the handoff from the Mac, 2026-11-18: "give `CardFrames` (or each `CardFrame`) a way to say
  them").** `images::YuvColors` (`matrix: YuvMatrix`, `full_range: bool`; `YuvColors::BT709` where nothing is
  said) is told of each frame: `CardFrame::colors(YuvColors)`, as its crop and its fence are, for a decoder may
  change them between frames. Windows' media reads them from the decoder's type (`MF_MT_YUV_MATRIX`,
  `MF_MT_VIDEO_NOMINAL_RANGE`; BT.709 of the video range where it says nothing, as before, SMPTE 240M as BT.709),
  and its video processor, where it converts to red, green and blue for Vulkan, reads by them too (BT.601 or
  BT.709; BT.2020 as BT.709, which is all that processor's color space says). Both renderers pass a card frame's
  colors in `opacity.zw`; the `dx12` HLSL has the `yuv_to_rgb` of `yuv.wgsl`, the same numbers.
- **Why.** The decoder knows; the renderer only has to be told. Each frame and not the frames, for a stream may
  change format without its surfaces being made again.
- **State.** Built on `claude/apple-to-pc-5b0fa9`, with readback tests on both renderers against the standards'
  K_R and K_B (`app/yuv_readback_tests.rs`). A picture of memory (`ImageData::streaming_nv12`, `streaming_i420`)
  says no colors yet: it is drawn as BT.709 of the video range.

### A picture's and a media's events, as the web has them

- **Question.** `ui::Picture` has `on_load` and `on_error(&str)`; `ui::VideoPlayer` and `ui::AudioPlayer` have no
  load, error, end or waiting event — the engine's `MediaState` holds them (`duration`, `size`, `playing`,
  `waiting`, `ended`, `error`) and `Media::on_change` wakes from the media's thread without saying what changed.
- **Decision (Ivan, 2026-11-18).** On `VideoPlayer`, `AudioPlayer` and `Media`: `on_load(|info|)` (opened and
  ready: its duration, the frame's size, whether it has sound), `on_error(|e: &MediaError|)` (a kind; its words from
  the locale, as the player shows them), `on_ended()`, `on_playing_change(bool)` (the video's too), and
  `on_waiting_change(bool)` (buffering began or ended); each told on the UI thread, once per occurrence.
  `ui::Picture::on_error` is told a kind (`PictureError`), not a sentence.
- **What the session that built it chose, where the decision left it open.**
  1. A media's events are told on the thread that first listened to it: the media's thread wakes the loops of that
     thread's windows, and each window's next turn (its pass, the tick of its timers) compares the state with what
     it last told and calls the handlers before anything is rendered. No queue of events: a change is told by its
     transition, so two quick changes in one turn are told as where they ended.
  2. Order in one turn: it opened, it waits or not, it plays or not, it ended, it failed. Nothing but a failure is
     told before it opened; opening is not waiting. A media that failed stays failed (no retry on `Media`): a
     player's "Try again" opens a new media, whose failure is told again.
  3. A player without a source tells none of the five (what it shows is the app's, and a handler is not told what
     the app set). With a source, `on_playing_change` (which heard a press) hears the media stop and start by itself
     too; what a press told or the app set is not told again, and a signal bound to `playing` that follows the media
     at its end is not taken for the app pausing it.
  4. `AudioQueue` plays nothing: the player that plays its current item tells each end (`on_ended`), where the app
     goes on to the next.
  5. `PictureError` lives in `core` beside `MediaError`, its words in `MediaWords` (the media's where they fit, and
     three of its own). `Picture` was not in 0.62.0: no break.
  6. Tests drive a media with no thread (`media::drive_media`, `Media::driven`, hidden from the docs).
- **State.** Built on `claude/media-events-ab3678`; to be merged into `feat/design-system`.

### A lighter light theme: light fills, crisp lines

- **Question.** The light palette's greys were made darker on 2026-11-13 (`e820b1df`) because an all-white window
  told nothing apart; now there is too much grey. What reads as grey is the large fills (Canvas behind the page,
  Muted on the window's bar, tables' headers, a code block's bar, a tab list, Secondary buttons), not the lines.
- **Decision (Ivan, 2026-11-18, all three).** (1) Light fills: Canvas `#F7F7F8`, Muted / Secondary / Highlight
  `#F1F1F4`, SecondaryHover `#E6E6EB` (final values tuned by eye in the gallery, kept near these). (2) Crisp lines:
  Border `#D4D4DA`; Input stays `#9A9AA5` (a field's edge keeps 3:1). (3) The components that fill a large area with
  Muted move to `Background`/`Card` with a line (the window's bar, tables' headers, the code block's bar, tab lists,
  the sidebar, and what an audit finds); Muted stays for small things (chips, badges, inline code, slider tracks).
  The dark theme is not touched; the legacy theme keeps 0.62's look. First on the board (Ivan): the light
  variables and the light components frame (`Ovt1D`, "Components / light") changed and looked at before the code.
- **What the session that built it chose, where the decision left it open (2026-11-18).**
  1. **The values, tuned by eye in the gallery, are the decision's**: Canvas `#F7F7F8`, Muted / Secondary / Highlight
     `#F1F1F4`, SecondaryHover `#E6E6EB`, Border `#D4D4DA`. **Input is `#8E8E99`, not `#9A9AA5`**: `#9A9AA5` has 2.79:1
     on white (2.60 on the new Canvas), short of the 3:1 the decision meant it to keep; `#8E8E99` has 3.24 on white and
     3.03 on Canvas. MutedForeground `#52525B` has 6.86:1 on Muted, 7.22 on Canvas, 6.21 on SecondaryHover; Ring
     `#71717A` 4.83 on white and 4.51 on Canvas. Border (1.48:1 on white) is the only edge of no control that takes
     input: a field, a checkbox, a radio and an outline button are edged in Input; a read-only field keeps its Border
     edge on Muted, as before (1.86:1 then), its text being what shows it.
  2. **A role of its own, `Bar`, in place of `Background`.** The dark theme is not to be touched, and a bar moved to
     `Background` would turn from `#27272A` to `#09090B` in it. `PaletteColor::Bar` is the fill of a bar along an edge of
     a window or a panel: `#FFFFFF` in light, `#27272A` (Muted's) in dark, `#f1f5f9` (Muted's) in the legacy palette, so
     neither of those changes. The board has the variable `bar` with the same values.
  3. **What moved to `Bar`**: the window chrome's bar, the window's tab strip, document tabs' bar, the command bar's
     tabs, the dock layout's bar of tabs and its strip, the panel header and the floating panel's title bar, the
     sidebar, the status bar, the table's (and data grid's) header, the code block's bar, the contained list's header,
     the transfer's list headers, the rich text editor's toolbar, the property grid's header, Markdown's table header
     and details summary, and the gallery's props table header. **What keeps `Muted`**, being small or being told
     apart by its fill alone: the tabs' default rail and the segmented control's track (controls of 36, as shadcn's), a
     table's group row, summary row and row detail, the property grid's categories, Markdown's striped rows, chips,
     badges, tags, inline code, kbd, slider, progress, meter and gauge tracks, skeletons, a picture's and a teaching
     tip's placeholder, avatars, an empty state's mark, a read-only or disabled field, the banner and the callout (a
     notice is its fill), the drop zone (its dashed edge on a light fill), a card's `Filled` variant and the breadcrumb's
     path control.
  4. **A selected tab on a white bar is edged.** The window's selected tab was told apart by its white on the grey bar;
     it is now edged at its top and sides by a line of Border and stands over the bar's line, open to the content. The
     line under a bar with tabs is a part of its own (`WindowChromePart::Line`), drawn under the tabs, since a border of
     the bar is drawn over its children. A rich text tool that is on is the background's fill edged by a line of
     Border. In dark both lines are the bar's own color.
  5. **The engine's `components::WindowChrome`** keeps its default `Muted` bar: it is 0.62's, and the legacy look keeps
     it.
- **State.** Built on `claude/light-theme-9c28cb`, the board and the code together (the session was started before
  "the board first" was written here): the board's light variables and components are changed, for Ivan to look at
  before the code is merged into `feat/design-system`.

### A frame callback for an app: `ctx.on_frame`

- **Question.** lurq's own animations (transitions, keyframes, presence, part animations) get their frames from the
  runtime, which asks the shell for the next frame when something will change. An app's own code has no way to be
  called once per displayed frame in step with the display: an `Interval` timer is not on the vsync (it jitters or
  draws too much), and `GpuViewport`'s `wants_redraw` serves GPU drawing only. Springs, physics, a drag's inertia, a
  game or a simulation on a canvas, smooth following of a value need it.
- **Decision (Ivan).** A `requestAnimationFrame` of lurq's: `ctx.on_frame(|frame| ..)`, told the frame's time and the
  time since the one before, called once per presented frame in step with the display, returning whether it wants
  another; while any wants frames the runtime asks for them; paused while the window is minimized or occluded, as
  animations are; on the tree's clock in tests (`set_animation_clock`), never the wall clock.
- **What the session that built it chose, where the decision left it open.**
  1. The callback returns whether it wants another frame, and `ctx.on_frame` returns a handle too
     (`FrameCallback`: `cancel()`, `start()`, `is_active()`): a spring at rest returns `false`, and the handler that
     moves its end starts it again. Called in `create` it is the component's until it unmounts; called in a render it
     is the callback of that place, as a `ctx.future` is.
  2. It runs before the frame is rendered and laid out, as `requestAnimationFrame` does: what it sets is in the frame
     it ran for. What was dirty before it is rendered first, so a component the frame no longer has runs none.
  3. Outside a component it is `WindowHandle::on_frame`. `App` has none: frames are a window's, each with its
     display's rate.
  4. The first frame after a hidden window shows again has a delta of zero, as a first frame, not the hidden time
     and not a clamp of it; a frame late while the window is shown has its real delta, which the app clamps where
     its integration needs it.
  5. Reduced motion does not stop it: what it moves is the app's.
- **Keyframes stay the engine's (Ivan asked, 2026-11-18, whether they should be put on this system for
  consistency).** They are on one system already, from the other side: a running callback is a timeline to the
  runtime (`Tree::has_active_timeline`, `next_timeline_change`), so the shell's pacing, the pause of a hidden window
  and the rule that a frame is laid out and not drawn from the kept list are one path for both. Keyframes are not
  moved onto `on_frame`: the engine steps them in the layout and knows when they next change, so a `Steps` spinner
  draws its steps and a delay draws nothing, where a callback asks for every refresh.
- **State.** Built on `claude/on-frame-c75375` (`crates/lurq/src/app/ctx/frames.rs`), with tests on the tree's clock
  and a section of the gallery's Animation page; to be merged into `feat/design-system`.

## 2026-11-17

### A file upload does not take the same file twice

- **Question.** A `ui::FileUpload` took a file it already held as another: chosen twice, or dropped twice, it stood
  twice in the list and would be sent twice. Seen on the iPhone, where each choice is a copy of its own.
- **Decision (Ivan, 2026-11-18, at the phone: "два однакових файла можна загрузить").** A file of the same name
  and size as one the list holds, or as one before it among those given at once, is not taken: it is refused with
  `RejectReason::Duplicate`, as one too large is with its reason, and the app hears it through `on_reject`.
- **Why.** A person who gives the same file again means the one they gave; the app should not send it twice.
- **State.** In work on `feat/design-system`.

### A frame of three planes (I420) is drawn as it is

- **Question.** A decoder of software (dav1d, libhevc, as Parties has) gives its frames in memory as three planes,
  gray then the two of color apart (I420). lurq drew two planes, gray and the two of color side by side (NV12), and
  red, green and blue: an app put the colors side by side on the processor itself before each frame.
- **Decision (Ivan, 2026-11-18: "так давай l420 зробимо?").** An image of memory may be I420
  (`ImageData::streaming_i420`, `set_streaming_i420`, `take_streaming_i420_buffer`): the `wgpu` renderer uploads the
  three planes as they are and makes the colors of them in the shader, with no work on the processor, on every
  system. The `dx12` renderer draws none yet and says so once in the log; it is to be done on Windows.
- **Why.** A frame decoded in memory is uploaded once anyway; putting its planes together first is a pass more
  over every frame for nothing.
- **State.** Built on `feat/design-system` for `wgpu`: a test sends a picture to the card on Metal and reads its
  three planes back as they were; the gallery's Video Player page draws the same bars as a pixel buffer and as I420,
  pixel for pixel. Built for `dx12` on Windows on `claude/apple-to-pc-5b0fa9` (2026-11-18): three `R8` textures
  uploaded as they are, three SRVs under a root signature of three, and `image_nv12.hlsl` compiled with
  `THREE_PLANES`, which reads the two of color from two textures; a picture of three planes drawn whole is read
  back from both renderers on Windows and matches BT.709 of the video range (`app/yuv_readback_tests.rs`).

### On Apple's systems an app's own frames are drawn where it wrote them: red, green and blue too, and once its card is done

- **Question.** To write an app like Parties (`github.com/emcifuntik/parties`: voice, screen sharing, its own
  decoders) in Rust on lurq, its frames have to reach the screen without a copy on Apple's systems as they do on
  Windows (`CardFrames`, with its fence). `CvPixelBufferFrames` drew a decoder's buffers of gray and color alone, and
  showed a buffer at once, whatever the app's card was still writing in it.
- **Decision (Ivan, 2026-11-18: "а зірокопі можливість є малювати в текстуру як на віндовсі? щоб це було
  переписано на раст", then "будемо працювати над цим").** `CvPixelBufferFrames` draws a buffer of red, green and
  blue (`BGRA`, as ScreenCaptureKit and an app's own Metal give) where it is, as a picture of sRGB. And an app that
  writes a frame with its own Metal commands shows it with the `MTLSharedEvent` it signals
  (`CvPixelBufferFrames::show_after(buffer, &event, value)`): the frame is drawn from the first frame of the window
  after the event reached the value, the one before it until then; nothing waits. Frames of three planes (I420) and a
  picture elsewhere than the buffer's corner are left for later.
- **Why.** A screen shared is red, green and blue; an app's decoder or renderer works on its own queue, and a
  frame drawn before its card finished shows half of it. Waiting on the card in lurq's queue is not open to it
  (wgpu gives no Metal queue), and asking the event costs nothing.
- **State.** Built on `feat/design-system`, with its tests on Metal; no app of the kind is written on it yet.

### Undo takes back what was typed as the platform's own fields do

- **Question.** Undo in a text input took back one character at a time, on every system: each key typed was a step
  of its own. Seen on a Mac through MCP on 2026-11-18.
- **Decision (Ivan, 2026-11-18: "давай групування як на маці", then "нам треба шоб на маку як на маку працювало, а
  на віндовс як на віндовс").** What is typed is one step, as the platform's own text fields group it:
  - **macOS and iOS** (`NSTextView`, `UITextView`): a run of typing is one step until the caret is moved or the
    selection changed, or the run turns from typing to deleting or back. Backspace held down is one step, as is
    Delete.
  - **Windows** (WinUI's `TextBox`, Notepad, Word): a word typed, with the spaces after it, is a step; the next word
    begins the next one. Moving the caret and turning from typing to deleting end a step as on a Mac.
  - Elsewhere (Linux, a browser) as on a Mac.
  A paste, a cut, a deletion of a selection and what the app puts in are each a step of their own.
- **A pause ends a step (Ivan, the same day, at the Mac: "я пишу частину, потім ще через 2 сек і воно все
  зтирається", then "да - давай, і настройкою в App").** On every system, typing that comes more than a second after
  the typing before is a step of its own, as in a browser. The app sets how long
  (`App::set_typing_undo_pause(Some(duration))`), or none (`None`), when only the caret and the kind of edit end a
  step.
- **Why.** A person undoes what they typed as they would in any other app of the system.
- **State.** Built on `feat/design-system`. Windows' grouping by word was written from what is known of its
  fields; tried on a PC on 2026-11-18, Windows' fields group a run as a Mac's do, and Windows' part of this decision
  is replaced by "On Windows a run of typing is one step of undo, as on a Mac" (2026-11-18). The pause and the
  rest stand.

### On macOS every app has an Edit menu, which it can turn off or put in its own place

- **Question.** On a Mac the menu bar of an app of lurq's was winit's, or the app's own (`App::set_menu_bar`), and
  neither had an Edit menu: no Undo, Cut, Copy, Paste or Select All to be seen or clicked, no Emoji & Symbols, and
  its keys reached the window as keys alone. The gallery declares no menu.
- **Decision (Ivan, 2026-11-18, at the Mac: "давай додамо", then "і можна вимкнути або заоверрайдити").** lurq puts
  an Edit menu in every app's bar on a Mac: Undo, Redo, Cut, Copy, Paste, Select All with their keys, and the
  system's Emoji & Symbols, in the words of the app's locale. Each item runs what its key runs on the focused
  element (`WindowHandle::edit`, `EditCommand`). An app with no bar of its own gets the system's menu of the app
  and the Edit menu after it. An app turns it off (`App::set_edit_menu(false)`), or puts its own: a menu of its
  bar that has an item of editing (`MenuAction::edit(EditCommand::..)`, or the whole of `Menu::edit()` with items
  of its own added) is the Edit menu, where the app put it, and lurq adds none.
- **Why.** Every app on a Mac has one: it is where a person looks for Undo and Paste, and where the system puts
  emoji and dictation. As the menu takes Cmd+Z before the window, an app that wants the key for itself turns the
  menu off.
- **State.** Built on `feat/design-system`; Ivan tried it on a Mac.

### On macOS the system's lights are the window's controls, and the bar stays in full screen

- **Question.** On a Mac the gallery's bar drew three lights of its own: no menu of tilings under the pointer on the
  green one, no full screen at its press. In full screen the bar went away with its tabs, and after leaving full
  screen by the system's own keys the bar stayed hidden, since the shell never heard that the window came back.
- **Decision (Ivan, 2026-11-18, at the Mac: "на маку немає доку як на віндовсі … і фоулскрін не робить фулскрін",
  then "а чому вкладок в повноекранному нема").** Where an app's bar marks its controls (`.window_control(..)`), the
  window on macOS has a title bar that is not seen, its content under it, and the system's own lights are moved to
  the middles of the app's controls, which keep their room and are not drawn (`WindowInfo::native_controls`). In full
  screen the system shows its lights with its own bar at the top of the screen, and the app's bar stays, its tabs
  with it, as Safari and the Finder keep theirs; what is laid over the whole window (a video that fills the screen, a
  sheet) has all of it there. Whether the window is in full screen is read again every turn. Windows is as before.
- **In full screen the bar has no lights (Ivan, the same day, at the Mac: "дивно що маковськи три кнопки лишаються
  на фулл скріні, там же згори в фулскріні появляється смуг сама").** The system's bar that comes down at the top
  of the screen has the lights; the app's bar does not draw its own under it, and its tabs start where the lights
  were.
- **State.** Built on `feat/design-system`; looked at on a Mac out of full screen, in full screen to be looked at.

### A video plays on macOS and iOS, with the system's player

- **Question.** `lurq::media` played a sound on Apple's systems (`AVAudioPlayer`) and said of a video that this build shows none there. The checks of Apple's systems (`apple-checks.md`) had "a video does not play" as what to expect.
- **Decision (Ivan, 2026-11-17, at the Mac: "так давай додамо").** A video plays there. `Media::open` gives it to the system's `AVPlayer`, which decodes it, plays its sound, keeps the two together and reads an address by itself; the frames are taken from it (`AVPlayerItemVideoOutput`) as two planes of gray and color and copied into the image a `Video` draws, through memory once, as everywhere but Direct3D. Bytes are written to a file of the system's temporary folder for as long as the media lives, since the player opens files and addresses. `Media::sound` stays on `AVAudioPlayer`.
- **Why the system's player and not its reader of samples** (`AVAssetReader`, which is what Media Foundation's reader is on Windows): the reader opens no address, cannot be sent to a time without being made again, and leaves the sound, its output and its clock to be written; the player has all of these. What it costs: the player says what changed on the main queue, so the media's thread asks it where it is instead of being told.
- **Frames stay on the card (Ivan, the same day, at the phone: "нам треба зіро копі як на віндовсі").** Where the `wgpu` renderer draws on Metal, the buffer the decoder gives (`CVPixelBuffer`, of a kind Metal makes textures of) is shown as it is, through `images::CvPixelBufferFrames`: the renderer makes its two textures of the buffer shown now and draws them, and nothing is copied. The renderer says it draws such frames (`images::cv_pixel_buffer_frames_drawn`); where nothing does, a frame is copied through memory once, as before. The same type is an app's own decoder's way to the screen, as `CardFrames` is on Windows (`MacosCvPixelBuffer::nv12`, `write_nv12` for one that decodes in memory); the gallery's "Frames of a decoder of your own" shows it there.
- **State.** In work on `feat/design-system`: built, its checks pass on a Mac, looked at in the gallery on a Mac; on a phone to be looked at.

### `GpuViewport` has no limits that are defects

- **Question.** The guide to `GpuViewport` (`docs/src/content/docs/gpu-viewport.md`) listed eight limits an app had to
  work around: lurq did not re-export wgpu; the frame had no scale factor and only the unclipped rectangle; a
  viewport with no size was one pixel square; nothing checked a texture's format; a validation error of the app's own
  wgpu calls ended the program; the `dx12` renderer failed a frame with a viewport; each window had a device of its
  own, so a slot could not be shown in two, and DevTools' window ran the app's extensions; and only an extension heard
  of a lost device. Ivan: known defects are fixed, not documented.
- **Decision (the session given the task, as the task left each shape to it).**
  1. `lurq::wgpu` is wgpu itself, re-exported with the `wgpu` feature: an app writes `use lurq::wgpu;` and pins
     nothing.
  2. `WgpuFrameInfo::scale_factor` is the window's, and `WgpuFrameInfo::visible_viewport(id)` is the part of the
     viewport that is shown (cut by its parents' clips and the window's edges, `None` when none of it is), beside
     `viewport(id)`, the whole box. Both in physical pixels; the engine is told the scale with the frame
     (`RenderEngine::set_scale_factor`, called by the tree before each frame).
  3. A `GpuViewport` with no size is 300 by 150 logical pixels, as lurq's `Canvas` is and as a browser's `<canvas>`
     is: a surface the app draws on has one default size in lurq, and it is seen. Given one side, the other stays
     the canvas's, as a `Canvas`'s does: there is no picture whose shape it keeps, and a viewport that took a
     picture's would not grow with flex. Filling the container was not taken: a fill is nothing in a column as
     high as its content, and the viewport would vanish as it did at one pixel. A size required in `new` was not
     taken: most viewports are sized by flex and percentages, not numbers.
  4. `set_texture_view` returns `Result<u64, WgpuTextureError>` and logs what it refuses: a texture without
     `TEXTURE_BINDING`, not 2D, multisampled, or of a format lurq's image shader cannot read as filtered color
     (integer, depth and stencil, 32-bit float). Everything else is shown right: an sRGB format is decoded by the
     card, and a linear one (`Rgba8Unorm`, `Rgba16Float`) holds linear color, which is what a shader writes into it;
     both are encoded once for the window. A slot that refuses a view keeps the one it had.
  5. A wgpu error of the app's own calls ends nothing: what an extension's `initialize`, `prepare`, `device_lost` and
     `shutdown` cause is caught in error scopes lurq pushes around them, and what the app causes elsewhere (a thread
     of its own) reaches the device's handler outside lurq's own work. Each is logged and given to the extensions of
     the device's engines, `WgpuFrameExtension::error(&WgpuAppError)`. An error of lurq's own work ends the program as
     before. In a browser only the extension's calls can be told apart, the browser telling the rest later.
  6. The `dx12` renderer leaves a viewport's box empty, what is under it showing through, and says once in the log
     that it draws no `GpuViewport`. Sampling a wgpu texture there would need the texture shared between two APIs'
     devices; an app that draws its own GPU code takes the `wgpu` renderer.
  7. The `wgpu` engines of an app's windows share one device by default: an engine made without `with_device` takes
     the device of another engine on its thread with the same backends, features and limits, while one draws, and
     makes one otherwise. `WgpuRenderEngine::with_own_device()` keeps one apart. So one slot can be shown in two
     windows: both draw the slot's one view, each stretched to its box. lurq's DevTools window runs none of the app's
     extensions (`RenderEngine::draws_tooling`).
  8. A lost device is an app event: `AppEvent::GraphicsDeviceLost`, told to each window's `on_app_event` handler
     once its engine was replaced, after the extensions were told and before the window is drawn with the new device.
- **State.** Built on `claude/gpu-viewport-8dee02` (2026-11-17), to be merged.

### lurq's own Direct3D 12 renderer draws the frames of a card

- **Question.** `images::CardFrames` (a video's frames kept on the graphics card) were drawn by the `wgpu` renderer alone; under `Dx12RenderEngine` a video went through memory and an app's decoder was not drawn.
- **Decision (Ivan: "вливай гілку і давай за вулкан візьмемось", then "працюй далі" when this renderer was named as next after Vulkan).** The order of Windows' backends is Vulkan, then lurq's own `dx12` renderer, and OpenGL only if someone uses it. The `dx12` renderer draws `CardFrames` as `wgpu` does: on the card where its device opens them, through memory where the decoder is on another card.
- **State.** Built on `claude/ds-vulkan`, not merged; see `design/handoff/media-playback.md`.

### The `dx12` renderer's older shared NV12 textures go

- **Question.** `Dx12RenderEngine` had a way of its own to draw a video's frame from the card, older than `images::CardFrames`: `Dx12VideoSurfaceAllocator`, `Dx12Nv12Surface`, `Dx12Nv12Image`, `Dx12Nv12ImageSlot` and `NativeImageData::from_dx12_nv12`. With `CardFrames` drawn by that renderer there are two ways to do one thing.
- **Decision (Ivan, 2026-11-17: "треба прибрати і додати це в брейкінг ченжі").** The older way is removed, and the removal is listed under Breaking in the changelog. An app gives its frames as `CardFrames`.
- **State.** Built on `claude/ds-vulkan`, not merged.

### One NV12 texture under Vulkan on a card of NVIDIA's goes through memory, and that is written down

- **Question.** Under Vulkan on the RTX 5080 one NV12 texture of a decoder is not drawn where it is: given to Vulkan, its color is read from the wrong place and the device is lost (what was tried is in `design/handoff/media-playback.md`). A way round is a copy on the card: each frame drawn into a texture of red, green and blue that Vulkan does take.
- **Decision (Ivan, 2026-11-17: "давай просто тоді позначимо про цю ваду в доках і не будемо нічого переводити").** Nothing is converted. The fault is written in the docs: the handoff, the changelog, and what `images::card_frames_nv12()` and `CardSurface::Nv12` say. Such frames go through memory there, as before; a decoder gives another kind of surface, or the app draws through Direct3D 12.
- **State.** Done on `claude/ds-vulkan`: the docs written, the way round thrown away.

### A video in a browser is not built on a copy

- **Question.** In a browser build lurq decodes no video. What wgpu 29 has there is `Queue::copy_external_image_to_texture` from a `<video>` element or a `VideoFrame`: one copy on the card for each frame. WebGPU's way with no copy, an external texture (`importExternalTexture`), is `unimplemented!` in wgpu's backend for the web.
- **Decision (Ivan, 2026-11-17: "нє це повна параша").** Not built. A video in a browser waits for external textures in wgpu's web backend.
- **State.** Decided; nothing written.

### OpenGL is not worked on

- **Decision (Ivan, 2026-11-17: nobody uses it).** Frames on the card and a lost device are not done for wgpu's OpenGL backend on Windows. A video there goes through memory, as before.
- **State.** Decided.

### Back and forward in one place

- **Question.** Going back and forward is spread like undo was: `router.back()`/`forward()`, `on_history_swipe`
  (a trackpad's swipe, a finger from the edge), the stack's back button calling the navigator; the mouse's side
  buttons arrive as `MouseButton::Other(4/5)` and lead nowhere; Alt+←/→ (Cmd+[ / ] on a Mac), Android's back and a
  browser's back are not handled.
- **Decision (Ivan).** One command, as `EditCommand` is: `NavigationCommand::Back` / `Forward`, given by every
  source — the mouse's side buttons, Alt+← / Alt+→ (Cmd+[ / Cmd+] on Apple), a trackpad's history swipe, a finger
  from the edge, Android's back, a browser's back and forward. Element events **`on_history_back` /
  `on_history_forward`** (named after `on_history_swipe`, for consistency), dispatched from the focused element
  outward, a handler that does nothing passing it on (a dialog may close on back instead of leaving the page). With
  none taking it, the mounted router goes back or forward. `Ctx` runs the same from a menu or a button; MCP and the
  emulator too. `on_history_swipe` and `StackOutlet` move onto it.
- **State.** Built on `claude/history-nav-9fccc7` (2026-11-17), to be merged: `NavigationCommand` and
  `NavigationSource`, `KeyboardEvent::navigation_command` (Alt+←/→, Cmd+[/] on Apple by the bracket or its place,
  `BrowserBack`/`BrowserForward` for a keyboard's keys and Android's back), `MouseButton::Back`/`Forward` on their
  release, `on_history_back`/`on_history_forward` with `HistoryEvent::pass_on`, the router as the default (on the
  way, then elsewhere in the window, innermost first), `on_history_swipe`'s default, a browser's `popstate` to the
  entry beside through the command (put back where something else took it), `Ctx::history_back`/`history_forward`/
  `history_runner`, `WindowHandle::history_back`/`history_forward`, MCP's `history_back`/`history_forward` and
  buttons `back`/`forward`, the emulator's side buttons as the phone's back, and `StackOutlet`'s way back and edge
  from its top screen. Decided in the session where the decision left it open: a key and a request start at the
  focus, with nothing focused at the last element that hears the command; a pointer's source starts under the
  pointer, then the focus; Alt+arrows go back in a text input too (its word moves are Ctrl's; Apple's keys are
  Cmd's); in a browser the side buttons are the browser's. Not built: Android's back with nothing taking it does not
  leave the app (winit takes the key), and `ui`'s overlays do not close on going back by themselves (an app's
  handler does it).

### The gallery shows how to style

- **Question.** lurq styles at four levels — the theme's tokens, a component's recipe in the theme (parts by
  variant, size, state, density, breakpoint), one instance (`.style(part, ..)`, `.rules`, `.state`, `.force`), a
  renderer — and the gallery shows none of it: the props tables leave styling out, the playground has no view of a
  component's parts, Getting Started has no page on it.
- **Decision (Ivan).** A page of its own, **Styling**, in Getting Started: the four levels with live examples (a
  palette role changes everything, a recipe patched changes every button, `.style` one button, a renderer of one's
  own). And in every playground **a section of its own for styling**: the component's parts and states, a part
  picked and restyled live (color, radius, padding, shadow…), the code beside it for one instance
  (`.style(Part, ..)`) and for all of them (`theme.patch_recipe::<X>(..)`). Built once in
  `crates/gallery/src/playground.rs` from the parts and states each component already declares.
- **State.** Built (`claude/gallery-styling-578adc`, 2026-11-17): the Styling page (`/styling`), the playground's
  Styling tab beside its props on every component's page, and in `lurq::ui` `Themed::parts`/`states` (`Declared`)
  and the `Styled` trait that every builder implements.

### The gallery's Engine group replaces the demo

- **Question.** The gallery was to replace `crates/demo`, but it had no page for the engine's primitives (layout,
  scroll, paint, text, motion, the reactive model), which only the demo showed, as hand-made pages in a theme of
  their own.
- **Decision (Ivan).** The gallery gets a group of pages for the engine, built as its component pages are (a
  playground first, then sections with their code), with every demo page's content and the primitives the demo
  never showed (`Canvas`, `Svg`, `VirtualizedList`, `GpuViewport`, `ZoomScroll`, drag and drop, rich text, the
  router and outlets, the engine's own controls and overlays). Then `crates/demo` is removed; `crates/mobile_demo`
  stays. The group is named **Engine** (the session that built it chose, as the task left the name to it): its pages are not all primitives (signals, context, persistent storage and
  events are the app's model, not boxes), the demo called itself the engine's, and a group's name in the side bar is
  one short word, as Actions, Data and Desktop are. Its pages live under `/engine/…`, so a primitive and the design
  system's component of the same name (`/slider`, `/engine/controls`) never share a route. What the demo's binary did
  besides its pages moved: `--renderer dx12` and `--profile-log` to the gallery, the lifecycle window and the
  continuous video probe to `lurq`'s examples.
- **State.** Built (`claude/gallery-engine`, merged 2026-11-17); `crates/demo` is gone.

### Code highlighting is pluggable, its token names open, its colors the theme's

- **Question.** `ui::CodeBlock` and Markdown color code with lurq's own scanner, which knows Rust alone, into seven
  fixed kinds painted in the palette's roles; an app can neither bring its own tokenizer nor color code apart from
  the rest of its interface.
- **Decision (Ivan).**
  1. Token kinds are **open names with a dotted hierarchy** (`keyword`, `keyword.control`, `string.escape`,
     `function.macro`, …), as TextMate and tree-sitter name them; a color is looked up dropping the tail
     (`keyword.control.rust` → `keyword.control` → `keyword` → plain). `CodeTokenKind` as a closed enum goes.
  2. **A code palette in the theme**: a name → color map of its own, for the light and the dark scheme, defaulting to
     the roles used now (so a theme left alone looks as today); `set_code_color`, a whole scheme, and a block's own
     `.token_style(name, ..)` (color, weight, italic) override it.
  3. **A tokenizer is pluggable, one per language**: a `Tokenizer` trait (or any `Fn(&str) -> Vec<CodeToken>`) is
     registered for a language and its aliases (`App::register_tokenizer("sql", ..)`); registering one for a
     language that has one replaces it (lurq's own included). A language with none registered is plain text. A block
     may still take its own (`.tokenizer(..)`) over the registry. It is named a tokenizer, not a highlighter (Ivan):
     it splits code into named tokens and colors nothing. A tokenizer may keep state between renders and may
     be incremental (Ivan, the same day): by line, with an opaque state at each line's start so an edit re-tokenizes
     only from the changed line until the state agrees again; a handle to tell lurq its tokens are stale; and edit
     hints (tree-sitter's shape) for incremental parsers. All optional: a function stays one line. lurq's own becomes a
     table-driven scanner with a public language table (Rust, TOML, JSON, JS/TS, shell, Python, CSS, HTML/XML, YAML,
     SQL, Markdown), no dependency, wasm-safe. **No `syntect` or tree-sitter adapter in lurq**: the gallery crate
     shows one as an example of plugging a tokenizer in.
- **With it.** A cache of tokens by code, language and tokenizer; a long block draws the lines in view; `InlineCode`
  takes a language; the editable text's tokens use the same tokenizer and colors.
- **State.** Built on `claude/code-highlight-ef7166` (2026-11-17), to be merged: `lurq::code` (`CodeToken` with a
  dotted name, `Tokenizer`, `CodeTokenizer`, the registry and its cache, `CodeLanguage` and the scanner, thirteen
  languages), `CodeScheme` and `CodeStyle` in the theme, `CodeBlock::tokenizer`, `token_style` and `code_scheme`, a
  long block windowed past 400 lines (in its own room, or in the window where it has no greatest height),
  `InlineCode::language`, `TextInput::tokens_with` and `code_language`, and the gallery's `syntect` adapter
  (`crates/gallery/src/syntect_code.rs`). A tokenizer with state, the same day: `Tokenizer::tokens_from` with a
  `LineState` (any `Clone + PartialEq` value, kept by lurq at each line's start of a code field's text and of a
  mounted block's code, an edit re-tokenized from its line until the states agree; lurq's scanner and the gallery's
  syntect go by line), `TokenizerHandle::invalidate` (from `register_tokenizer` or `CodeTokenizer::handle`), and
  `tokens_in`, `edited` (a `CodeEdit` in tree-sitter's shape) and `dropped` for a code field's `BufferId`.

### Link Preview works from the link alone; every field overrides

- **Question.** Where does a link preview's site, title, description and picture come from?
- **Decision (Ivan).** `ui::LinkPreview::new(url)` fetches what the page says about itself and shows it;
  `.title`, `.description`, `.site`, `.image`, `.icon`, `.video` each override the fetched field; when every field it
  shows is given, nothing is fetched. The fetcher is lurq's own, behind `http` (the start of the page; `<title>`,
  OpenGraph, Twitter Card, the icon; a small scanner of the `<head>`, no HTML-parser crate; a cache by URL that an app
  can replace). Without `http` it shows what it was given and the address.
- **With it.** Read-only styled text holds inline elements in its line (the inline form "in a sentence"), built on
  the inline objects of the text input; `open_url` in the engine, by each platform's own call, which the preview uses
  unless `on_open` is given.
- **State.** Built (`claude/link-preview`, merged 2026-11-17), with `Ctx::focus_handle`, `.copy_text`, inline objects without `markdown` and the `/favicon.ico` fallback.

### Cargo features

- **Question.** 28 features: five pull no dependency, pictures spread over six, two localization systems, the `ui`
  charts behind `canvas`, `default = []`.
- **Decision (Ivan).** `form`, `router`, `query` stay features and go into `default`. `i18n` goes: `I18n` (an app's
  keyed translations) is always built, beside `core::Locale` (lurq's own words). `default` is the desktop set:
  `winit`, `wgpu`, `clipboard`, `file_dialog`, `image`, `canvas`, `form`, `router`, `query`. Two features for
  pictures and drawing: `image` (codecs) and `canvas` (drawing, with `svg` folded in); `raster` and `render`
  internal; `screenshot` uses `image`; one `png`. `resources` goes. `devtools` and `perf_profile` stay, their
  call-site cfgs behind macros. The browser keeps an explicit list. Tests: seven runs (A–G) in place of five feature
  combinations. SVG is always rasterized at the size it is drawn: the tessellation an `svg`-only build had is removed
  (Ivan, the same day), as no build has it once `canvas` brings `raster`.
- **State.** Built (the session "Clean up lurq's Cargo features", merged 2026-11-17 with the 0.62.0 release from
  `master`): 25 features, the desktop `default`, the runs A–G in `target/suites2.sh`.

### Editing commands in one place

- **Question.** Ctrl+Z on the signature pad did nothing after a stroke, Ctrl+Shift+Z did nothing, and nothing worked
  in a Ukrainian layout; every component matched keys of its own.
- **Decision (Ivan: "undo & redo in one place", then "undo, redo, copy, paste").** `app::events::EditCommand`
  (`Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`) and `KeyboardEvent::edit_command()` are the only code that
  knows those keys: Ctrl (Cmd on Apple), by the Latin letter or by the key's place in any layout. A focused text
  input does it first, then `on_undo` / `on_redo` / `on_select_all` / `on_copy` / `on_paste` from the focused element
  outward, a handler with nothing to do passing it on. `Ctx::edit`, `Ctx::edit_runner`, `WindowHandle::edit` run the
  same from a menu or a toolbar. A press that begins a drag focuses what the element `focuses`.
- **State.** Built, on `feat/design-system` (`77ffe27c`).

### The old `lurq::components` meet the design system

- **Question.** Plan decision D7 (form controls on the new components, `FormTheme` deprecated) was never done, and
  the old styled components live on beside their `ui` twins.
- **Decision (Ivan: "давай роби").** The form controls are rebuilt on `ui` and `FormTheme` deprecated. Every old
  type is an engine primitive (stays, its doc names the `ui` component an app wants), a styled duplicate (deprecated
  in favour of `ui::X`, as `components::Image` is for `ui::Picture`), or styled with hard-coded colors (`new()` /
  `dark()` become recipes). Each break in `migration-v0-63.md`.
- **State.** The old styled components: built (`claude/legacy-styled`, merged 2026-11-17: `Checkbox` and `ActivityIndicator` deprecated for `ui::Checkbox` and `ui::Spinner`; the phone pieces, `SelectStyle`, the stack's shades and the unstyled checkbox and slider nodes on recipes, the legacy theme keeping 0.62's look). The phone pieces as real `ui` components (Tab Bar, App Bar, Large Title, Bottom Sheet, Swipe Row, Pull To Refresh, as the board draws them) are not built. The form controls: built (`claude/legacy-form`, merged 2026-11-17: thin bindings over `ui::Field`, `Input`, `Textarea`, `Checkbox`, `Slider`, `Button`; `FormTheme` deprecated; the gallery's `/form` page), with an engine fix on the way: a mounted component as the root of an `in_theme` scope keeps the theme.

### Words lurq writes are in the locale

- **Decision (Ivan, `AGENTS.md`).** Every word a person sees or a tool reads is a field of `core::Locale`, in six
  languages; a component takes it from `ui::current_locale()`; a single-word method on the component wins.
- **State.** Built: `claude/ds-locale-words` and the thirteenth batch's words, merged.

## 2026-11-16

### Every family on the board is built

- **Decision (Ivan: "давай ці 6 доробимо").** The six families held for a request (Signature Pad, Angle Slider, QR
  Code, Watermark, Transfer, Cascader) were built. With them every family the board draws has a type in `lurq::ui`.
- **State.** Built; see the plan's 14.27.
