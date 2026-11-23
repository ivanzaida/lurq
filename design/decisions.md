# Decisions

What was decided about lurq, by whom and why, newest first. A decision is written here the day it is taken, before
the work on it starts, so that the work, its review and a later session read the same thing. The design system's
first decisions (D1–D13: where components live, palette roles, recipes, breakpoints) are in
[design-system-plan.md](design-system-plan.md#3-decisions) and are not repeated; a decision here that changes one of
them says so. Decisions about phones and tablets are in [mobile-decisions.md](mobile-decisions.md).

Each entry: **the question**, **the decision**, **why**, and its **state** (decided, in work on a branch, built,
dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-23

### Mutations and cache writes for `lurq::query`

- **Question.** `lurq::query` caches reads and has nothing for writes: every screen keeps its own pending flag and
  error, invalidates by hand and rolls its own optimistic update (issue #207, Ivan: "work on this now"). The issue
  gives the API (`#[lurq::mutation]`, `ctx.mutation`, `on_success`, `optimistic`, `on_error`, `set_data`,
  `update_data`, `get_data`) and leaves three things open.
- **Decision (Ivan, by the issue).** The API as the issue writes it.
- **Decided by Claude in the work, to be confirmed.**
  - *A run asked for while one of the same handle is pending waits its turn* (`MutationConcurrency::Queue`, the
    default), with `Parallel` and `Reject` to choose. The issue says "queued or rejected (an option)". Queued by
    default because it is the one that loses no write and lets none overtake an earlier one: a handle shared by
    the rows of a list must not drop the second row's write, as a rejecting default would.
  - *A fetch on its way when a value is written is dropped*, and the written value is fresh. The issue allows
    this or marking the query stale and fetching again. Dropped, because an optimistic write followed at once by
    a fetch would show the server's old value under a pending write; what wants the server's value invalidates
    (a mutation's `invalidates`, or `on_success`).
  - *A run belongs to the client*, not to the component: it is polled by the client's tick, so it finishes and
    has its effects with its component gone, as the issue asks. It is never cancelled or sent twice; one whose
    Tokio runtime shut down under it ends with neither a result nor an error.
  - *The handle is `MutationHandle<Args, Data, Error, Snapshot = ()>`*, named by plain types, not by the macro's
    hidden definition: it can be a prop. Effects live in the handle value, so `optimistic` must come before
    `on_error` in the chain (the other order panics in render), and a clone runs with the effects it was cloned
    with.
  - *Cache writes are synchronous*, under the client's driver lock, which became reentrant so that an effect
    called by the tick can write. `get_data` after `set_data` reads what was written.
  - *`QueryClient::mutate(rename_task(id, title))`* runs a mutation with no component, which is what the
    descriptor the macro's function returns is for.
  - `lurq_macros` is 0.1.4, with the new attribute.
- **Why.** A write needs the same three things on every screen; kept by hand they drift between screens.
- **State.** Built, on `feat/query-mutations`. Out of scope, as the issue says: persisting the cache, prefetch.

## 2026-11-22

### What 0.63 deprecated is removed

- **Question.** 0.63 left the old API beside the new one, deprecated, so an app could move at its own pace: 24
  `#[deprecated]` items on master. Ivan: "давай тепер приберемо все що ми депрекейтнули з кода на мастері".
- **Decision (Ivan, 2026-11-22).** Every item that carries `#[deprecated]` goes, with what only it used:
  - `WinitWindow::on_frame` (`on_paint`);
  - `FormTheme` and its parts, `Theme::form`, `Theme::set_form`, `ThemeDefinition::form` (the recipes of the
    `ui` controls);
  - the fifteen palette roles of 0.61 (`Accent`, `SurfaceBase`, `TextMuted`, ...) and their fields of
    `ThemePalette` (the roles that replaced them);
  - `dark()` and `for_appearance(..)` of `TabBar`, `SheetStyle` and `NavigationBar`, and
    `PullToRefreshStyle::dark()` (the theme's appearance);
  - `components::ActivityIndicator` (`ui::Spinner`), `components::Checkbox` (`ui::Checkbox`) and
    `components::Image` (`ui::Picture`), as far as the engine does not need them (see below).
- **Decided by Claude in the work, to be confirmed.**
  - What is not marked `#[deprecated]` stays, the legacy theme with it (`ThemeDefinition::legacy`,
    `ThemePalette::legacy`). Its palette keeps 0.61's colors in the roles that replaced the old ones; where two
    became one it has one color (`TextSecondary`'s and `TextPrimary`'s own colors are in no role). Nothing the
    engine draws read those two, so nothing it draws in the legacy theme changes.
  - `PaletteColor::replacement` went with the roles: it answered only for them.
  - `components::ActivityIndicator` is no longer exported and loses its mark: the pull to refresh draws it.
    `ActivityIndicatorProps` and its builders are untouched (Ivan: builders that are not deprecated stay).
  - **`components::Image` and `components::Checkbox` are not removed**, and stay deprecated and exported. The
    engine draws with both (`ui::Picture`, Markdown, the bars' symbols, the form's checkbox), `Checkbox` is the
    key of the recipe of the engine's checkbox node (`EngineLook::in_theme::<Checkbox>`), and about seventeen
    test files check the engine's own nodes through them: decoding from bytes with a placeholder and an error
    view, fit, the checkbox's own styles. `ui::Picture` and `ui::Checkbox` are other components (`ui::Checkbox`
    is built on a button), so moving those tests to them would test something else, and `Node::image` /
    `Node::checkbox` do not reach all of it. Open: either they are engine primitives and lose the mark, or the
    node API first gains what the wrappers have.
- **State.** Built on `chore/remove-deprecated`, but for `components::Image` and `components::Checkbox`: open.

## 2026-11-21

### A video draws the window for each of its frames, and the system's player reads ahead only so far

- **Question.** An app with a video on screen (Erome's feed, on an iPhone 17) had the window drawn at every refresh
  of the display, 120 times a second, whether the video played or stood: a frame of a video was an image that
  "changes by itself", and the shell presented directly from every turn of the loop, polling, while one was drawn.
  Every such frame drew all that lay under and over the video again, blurs too, and kept the graphics card busy
  70–87% of the time; the phone got hot. The system's player (`AVPlayer`) on macOS and iOS also read ahead with no
  bound: a playing video to its end as fast as the host gave it, and paused players opened ahead read on too.
- **Decision (Ivan, 2026-11-21: "давай щас попрацюємо над оптимізацією під іос", "давай роби все на маку").** A
  picture whose frames come by themselves (a video's frames on the card, `CvPixelBufferFrames` and `CardFrames`, and
  a streaming image) wakes the loops of the windows that draw it with each new frame (`FrameWakers`), and a window
  draws it again only when its frame is another than the one it drew: the shell waits between frames instead of
  polling, and presents directly only a frame or a redraw still to be presented. An animated picture (a GIF) keeps
  its pacing. On macOS and iOS the system's player fetches 1 s ahead until it is first told to play and 10 s once it
  plays (`preferredForwardBufferDuration`), as the loader reads ahead on Windows.
- **Why.** Measured on a Mac (M4) with Erome: drawn 100 times a second for a video of 30 frames before, 33 after;
  106 times a second for a paused video before, none after. The same scenario of swipes through the feed read 23 MB
  before the bound and 14 MB after, and a video opened ahead was open in 0.2–0.6 s where it took up to 4 s beside
  a playing video that took the host's bytes.
- **State.** In work on `perf/video-redraw-on-frame` (2026-11-21), from `f76a69cc`: checked on macOS and on an
  iPhone 17 (the graphics card's power impact over 26 s of Erome's feed with a playing video: 37.6 before, 4.6 after);
  Windows (`CardFrames`) not run.

### A blurred layer that holds still is not painted again

- **Question.** An element with `.blur(...)` was painted into its layer and blurred in every frame the window drew,
  however still it held. Under a playing video (Erome's feed: the video's poster, blurred, fills the page around it)
  that is a full-window layer and its blur passes 30 times a second, for pixels that never change.
- **Decision (Ivan, 2026-11-21: "давай кешуй блюр").** The wgpu renderer keeps a blurred layer's finished blur as it
  keeps a backdrop blur's (`LayerBlurInputs`, a second `BackdropCache` by the layers' targets): while what is painted
  into the layer, its place and its blur are byte for byte what they were, the kept texture is composited and the
  layer, with the layers inside it, is not painted. A layer with an image of its owner's pixels or a backdrop blur
  inside it has no known input and is blurred every frame. The DX12 renderer is left as it is.
- **Why.** Measured on a Mac (M4) with Erome's feed: the graphics card's time a frame went from 4.07 ms to 3.29 ms,
  the layer's own pass gone; what is left of blur passes is the glass of the buttons over the video, which a moving
  video under it blurs again in every frame.
- **State.** In work on `perf/video-redraw-on-frame` (2026-11-21); checked on macOS, and on an iPhone 17 over 26 s
  of one video in Erome, the same build with and without the cache: the graphics card busy 51% → 42%, its power
  impact 11.2 → 7.3.

### A breadcrumb that ends with a link, and a step that is no link

- **Question.** Orchester's breadcrumbs do not all end with the page one is on: the Files header and Edit task end
  their trail with a link (the task's key, back to the task), and the Sharing upload and join crumbs start with a
  step that is plain text (the local project's name, which goes nowhere). lurq's breadcrumb makes every step but the
  last a link, and the last the current page. Can a step say what it is?
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").** `BreadcrumbStep::link(bool)`: whether the step
  is a link, over what its place makes it. `link(true)` on the last step makes it a link, and the trail has no
  current page; `link(false)` on a step before it makes it plain text, in a part of its own (`BreadcrumbPart::Text`,
  the muted foreground as a link has it), no stop of Tab, not followed, and disabled in the menu of the hidden steps.
  `Breadcrumb::current(false)` says the same of the whole: the last step is a link like the others, unless it is told
  `link(false)`. A breadcrumb that overflows never drops its last step, link or page.
- **Why.** What Orchester's kit (`ui/breadcrumb.rs`, `Crumb::to` and `Crumb::here` in any place) needs to move onto
  `ui::Breadcrumb`; a trail that ends at a parent of the page (an editor's form under its record) and one that starts
  at a place that cannot be opened are common to apps.
- **State.** Built on `claude/k7-a-navigation` (2026-11-21): `BreadcrumbStep::link`, `Breadcrumb::current`,
  `BreadcrumbView::is_link`, the part `BreadcrumbPart::Text`; the gallery's "Links and plain steps".

### A tab's own state, its tooltip, and a press of the selected tab

- **Question.** Orchester's page tabs paint a count in `warning` when it counts what waits for the person, give an
  openable tab a tooltip (an explanation beside its lock), and open the current tab's page again when it is pressed
  (a page whose current tab is a place of its own re-opens it). `ui::Tabs` has no state on one tab a recipe sees,
  shows a tooltip only for a disabled tab's reason, and a press of the selected tab does nothing.
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").** `Tab::state(name, on)`: the tab alone in a
  state the theme's recipe names, seen by every part of it (its label, count, badge, dot), as a breadcrumb step's
  state is. `Tab::tooltip(Tooltip)` and `Tab::hint(&str)`: a tooltip on the tab, open or disabled, in place of the
  reason's tooltip (the reason stays its description to tooling). `Tabs::reselect(true)`: a press of the selected tab,
  and choosing it in the menu of the overflowing form, tells `on_change` its value again (the signal is not set, being
  the same); off unless told.
- **Why.** What Orchester's kit (`ui/tabs.rs`, `PageTab::count(_, waits)`, `explanation`, `PageTab::to(.., current,
  ..)`) needs to move onto `ui::Tabs`.
- **State.** Built on `claude/k7-a-navigation` (2026-11-21): `Tab::state`, `Tab::tooltip`, `Tab::hint`,
  `Tabs::reselect`, `TabEntry::states`; the gallery's "A tab's state, its tooltip, a press of the selected tab".

### The list of every open tab opens with no tab hidden

- **Question.** The docs of `WindowTabs::list_shortcut` and `menu_open` say the list of every open tab
  (`WindowTabsMenu::All`) opens at any time, but it hangs from the More button, which is laid out only while a tab
  has left the strip: with every tab in the strip the shortcut and the app's signal opened nothing on screen.
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").** The list opens with no tab hidden, as the docs
  say. The More button stays hidden then, and the list hangs from the tabs: 4 under them (or under what
  `list_clear_of` names), its end at the end of the tabs, inside the window's edges as before. A press on a tab while
  it so hangs is not outside it.
- **Why.** Orchester's title bar opens the list by Ctrl/Cmd+Shift+A and from its own button whatever the strip shows.
- **State.** Built on `claude/k7-a-navigation` (2026-11-21): the list hangs from the list of tabs (`BottomEnd`) while
  none is hidden; the gallery's "Every open tab in a list" opens it by `menu_open` from a button.
### A menu's section that scrolls, an item that leaves the menu open, submenus opened by a press, and popovers placed as Orchester's are

- **Question.** Orchester's kit cannot move its last menus and popovers onto `ui::Menu` and `ui::Popover`: the slash
  menu's skills, the @mention files, a choice field's options, the base branch and the browser's overflow menu are
  rows in a section that scrolls, which lurq's menu does not reach (the kit draws them as standalone items with keys
  of its own); a filter that chooses several is a menu whose rows must not close it; a card's *Move to*, *Priority*
  and *Tags* open their lists on a press, not under the pointer; a menu at the pointer over a row and a popover the
  app opens (the kanban question) must not take a press of their anchor for a press of a trigger; and the status
  bar's popover stands 12 from the bar's end and a chip's popover inside its column, which `ui::Popover` cannot say
  (one offset, no inset of `within`). Should lurq's menu and popover do these?
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").**
  1. `MenuEntry::Scroll { entries, max_height }` and `Menu::scrolling(entries, max_height)`: a section of the menu
     that scrolls past `max_height` (the part `scroll`, `x-scroll`), whose items stay the menu's: they close it, are
     named after it, are reached by the arrows and by typing their label, and the focused one is scrolled into
     view; under `keeps_focus` they are counted among the items the keys move, and the active one is scrolled into
     view. `Menu::entry(MenuEntry)` adds any entry.
  2. `MenuItem::keeps_open(true)`: choosing the item calls its handlers and leaves the menu open, the focus where it
     was (or, under `keeps_focus`, the active item where it was): a multi-select filter's option, checked or not.
  3. `Menu::submenus(SubmenuOpen::Press)`: the submenus of the menu's items open on a press of their item (and Enter,
     Space and Right), and a press of an open one's item closes it; the pointer opens and closes none.
     `SubmenuOpen::Hover`, the default, is today's.
  4. `Menu::switch_on_anchor_press(false)` and `Popover::switch_on_anchor_press(false)`: the anchor is no trigger of
     a dropdown, as the engine's `Popup` already says.
  5. `Popover::cross_offset(value)`: how far the popover is moved along the edge it stands on, toward the end (or
     down, beside its anchor), besides `offset` away from it. `Popover::within_inset(element, Padding)` and the
     engine's `Overlay::within_inset` / `Popup::within_inset`: kept inside the box of `element` less the inset on each
     side.
  6. The suspected bug, a popover over its control with an offset of -6 standing 6 into the control: checked with a
     test first, fixed if the engine is wrong.
- **Why.** What Orchester's kit needs to drop its `standalone` items and `held_panel`, its `popup_by_app`, and the
  arithmetic of `bar_popover` and `popover_inside`. None of it is Orchester's alone: long menus scroll, filters choose
  several, many apps open submenus on a click.
- **State.** Built on `claude/k7-b-menus` (2026-11-21): `MenuEntry::Scroll`, `Menu::scrolling`, `Menu::entry`, the part
  `MenuPart::Scroll`; `MenuItem::keeps_open`; `Menu::submenus(SubmenuOpen)`; `Menu::switch_on_anchor_press`,
  `Popover::switch_on_anchor_press`; `Popover::cross_offset`, `Popover::within_inset`, `Overlay::within_inset`,
  `Popup::within_inset`. The suspected bug is no fault: a popover's or a menu's offset is away from its anchor on every
  side, over it too, so -6 over a control stands 6 into it as documented (`popover_place` pins it); Orchester's kit
  passes `-gap` for `PopoverSide::Above` and should pass `gap`. The gallery's Menu page has "A section that scrolls, a
  filter, submenus on a press"; the Popover page's bar popover uses the cross offset.
### A list item's detail between its title and its description

- **Question.** A list item's `detail(element)` stands under the description. Orchester's setting rows (Settings ›
  Browser › Program path) say the label, then the value on a line of its own (the program's path), then the hint,
  and a link under the hint as well: one row with an element over its description and another under it. Can a list
  item put an element between its title and its description?
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").** `ListItem::subtitle(element)`: an element of
  the app's on a line of its own under the title's line and over the description, after those given before, in a
  part of its own, `subtitle` (`x-subtitle`, `x-subtitle-2`), whose presses are its own as a detail's are. The
  details stay under the description, so a row has both.
- **Why.** What Orchester's kit needs to build its `SettingRow` on `ui::ListItem` (its `detail` and its `link`). A
  place of its own, not a switch of where every detail goes, since the row has one of each.
- **State.** Built on `claude/k7-c-rows` (2026-11-21): `ListItem::subtitle`, the part `ListItemPart::Subtitle`,
  `ListItemView::subtitles`; the gallery's List page says a program's path in "Saying more".

### A checkbox's, a radio's and a switch's label over several lines

- **Question.** `ui::Checkbox`, `ui::Radio` and `ui::Switch` keep their label on one line. Orchester's kit has a
  checkbox whose label may be long (one that names a folder) and wraps within the parent's width, the box beside
  its first line, and draws it itself with lurq's box alone. Can lurq's controls wrap their label?
- **Decision (Ivan, 2026-11-21: "давай додай можливості в лурк ці").** `Checkbox::label_wraps(true)`, and the same
  on `Radio` and `Switch`, which draw their label alike: the label goes on to more lines within the width the
  control is given, and the box (the mark, the track) stands beside its first line, centered on it. Not unless
  told: a label on one line stays the default, as a control that grows to its parent's width would move layouts
  that have one now.
- **Why.** What Orchester's kit needs to drop its own wrapping checkbox (`Toggle::wrap_label`). The three controls
  draw their label the same way, so they gain it together.
- **State.** Built on `claude/k7-c-rows` (2026-11-21): `Checkbox::label_wraps`, `Radio::label_wraps`,
  `Switch::label_wraps`, and the field `label_wraps` of their views; a mark taller than the first line (the switch's
  track) has the line in its middle. The gallery's Checkbox, Radio and Switch pages show "A label over several lines".

## 2026-11-20

### The list of every open tab: the app's to know, its keys first, its rows, its place and its words

- **Question.** Orchester's title bar moved onto `ui::WindowTabs` in its list mode with the list of every open tab
  (`WindowTabsMenu::All`), and had to work round what the list could not do: the app could not know whether the list
  was open; its keys (Ctrl/Cmd+W on a row, Enter) reached the page first, so the window's Ctrl+W closed the active
  tab as well and a question's Enter under the list confirmed it; a row said one text, where Orchester's says the page
  and, quieter, the project, with its mark that something waits; the panel stood under the More button, 4 below it,
  where Orchester's stands 4 under the title bar, 8 inside the window's edge and never lower than 48 above its
  bottom; a search moved no row into view, and the rows under the search field stood 2 apart where the design has
  them abut; the More button's tooltip said lurq's words. Should lurq's list do these?
- **Decision (Ivan, 2026-11-20: "роби", on the K6 list).**
  1. `WindowTabs::menu_open(Signal<bool>)`: whether the More button's menu is open is the app's signal, which the
     strip sets as it opens and closes and the app may set; `on_menu_open_change(bool)` hears each change a person
     makes. An element built with `keys_first(true)` (the engine's, `NodeUpdate::keys_first`) hears a key before the
     rest of the tree while the focus is in it, and what it stops (`stop_propagation`) the rest of the tree does not
     hear; what the window does by default with the key still happens unless it is prevented. The list is one: the
     keys it acts on while the focus is in it (Ctrl/Cmd+W, Enter, Escape, the arrows, and every key without Ctrl,
     Alt or Cmd) are its own.
  2. `WindowTab::list_label(&str)` and `list_detail(&str)`: what the tab's row says in place of its label, and a
     quiet text after it (the part `item_detail`), both searched; `WindowTab::list_trailing(element)`: the app's
     elements in the row, after the mark of a tab not in the strip (the part `item_marks`). They are elements of
     their own, apart from the tab's `trailing`, so their ids do not collide with the tab's.
  3. `WindowTabs::list_clear_of(element)`: the list stands clear of that element's box (the title bar) on the side
     it opens on, not of the More button's, lined up with the button along it; `list_inset(f32)`: how far it stays
     inside the window's edges (8 unless told), which it is never lower than either, its rows scrolling.
  4. While the list is open, the row of the selected tab, or the first that matches the search, is scrolled into
     view, after each change of the search too. A state of the strip, `searchable`, is on while the list leads with
     its search field; the default recipe makes the rows abut then.
  5. `WindowTabs::more_tooltip(Fn(usize) -> String)`: what the More button's tooltip says for the count of hidden
     tabs, and what it says of itself to what reads the tree; the locale's words unless told.
  6. A `within` rule of a part in no interaction group answers to the innermost button around it, as to a group:
     `within_hovered(ButtonPart::Label, ..)` paints the label of a hovered `ui::Button`, and so does a rule of the
     More button's label. Where a group is around the part, it answers to the group as before.
- **Why.** What Orchester's kit needs to drop its workarounds (`navigation/input.rs`'s check of a layer over the page,
  the question's check of where the focus is) and its own sizing of the list, and to keep the design's rows. None of
  it is Orchester's alone: a browser's tab search says a page and its site, stands under its bar, and takes its keys.
- **State.** Built on `claude/k6-a-tabs-list` (2026-11-20): `WindowTabs::menu_open`, `on_menu_open_change`,
  `list_clear_of`, `list_inset`, `more_tooltip`; `WindowTab::list_label`, `list_detail`, `list_trailing`; the parts
  `ItemDetail` and `ItemMarks` and the state `Searchable`; the engine's `keys_first`; a `within` rule in no group
  answering to its button. The gallery's "Every open tab in a list" says each file's folder.
### A checkbox's and a switch's element ref, and a mark alone for a row pressed as a whole

- **Question.** Orchester's toggles take an element ref, so a screen can move the focus to one; `ui::Radio` has one,
  `ui::Checkbox` and `ui::Switch` do not. And its rows pressed as a whole (a filter's option, a choice of an export)
  draw a checkbox's or a radio's mark alone (`check_mark`, `radio_mark` of its kit): no press, no label and no focus
  of its own, the row saying that it is checked. Can lurq's give both?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `Checkbox::element_ref` and
  `Switch::element_ref`, as `Radio::element_ref`: the ref is on what is pressed and focused (the box, the track, or
  the whole where it is `whole`). The mark alone is a form of the components that already draw it, not a new
  component: `Checkbox::mark_only(true)` and `Radio::mark_only(true)` draw the box with its check or dash, the round
  mark with its dot, in their recipes' parts and states, with no press, no focus, no stop of Tab and no label; the
  mark is hidden from what reads the tree, for the row around it is the control and says it is checked. Its id is
  on the box or the mark, its check `x-indicator`, its dot `x-dot`; its element ref, where it has one, on the box.
- **Why.** What Orchester's kit needs to drop its own toggle; a mark of the same recipe keeps one look for the
  checkbox and the row that only shows one, which a new component would have had to copy.
- **State.** Built on `claude/k6-c-controls` (2026-11-20): `Checkbox::element_ref`, `Switch::element_ref`, `Checkbox::mark_only`, `Radio::mark_only`, `CheckboxView::mark_alone()`, `RadioView::mark_alone()`; the gallery's "Its mark alone" on the Checkbox and Radio pages.

### The page a breadcrumb is on says its whole name while it is cut, and the ids of its separators

- **Question.** K5 gave a breadcrumb's links a tooltip with their whole name while they are cut
  (`Breadcrumb::link_max_width`). The page one is on, its last step, is cut too where the row is short, and says
  nothing. And Orchester's kit numbers its separators from `-separator-1`, where lurq's part-ids rule names them
  `x-separator`, `x-separator-2`. Should the page say its name, and which ids do the separators keep?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** The page's name has a tooltip that
  shows only while it is cut, in every breadcrumb, as a link's has with `link_max_width`: the page is cut in any of
  them. The separators keep lurq's ids, the part-ids rule's (`x-separator`, `x-separator-2`, ...): a repeated part
  with no key of its own is numbered from its second node, in every component, and the app reads those. This is
  written in the breadcrumb's page.
- **Why.** What Orchester's kit needs; one rule for the ids of repeated parts is what lets a test or an agent guess
  them, and a breadcrumb that numbered from 1 would be the only one.
- **State.** Built on `claude/k6-c-controls` (2026-11-20): the page's tooltip in `BreadcrumbView::step`, the refs kept by a mounted breadcrumb for every step; the ids written in `Breadcrumb::id` and the breadcrumb's page, held by a test.

### A list item whose title and description are still read, and the ids of a row that says something under it

- **Question.** Orchester's settings rows show a skeleton's bone in place of their label or their hint while it is
  read (`label_reading`, `hint_reading`). And they have two ids: `<id>-row`, the frame of the row and of what it
  says under itself, and `<id>-open`, the part that is pressed. `ui::ListItem` has neither the bones nor a second
  id. What does lurq give, and how does the app map its ids?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `ListItem::title_pending(true)` and
  `ListItem::description_pending(true)` put a bone in place of the title's words or the description's, as long as
  the recipe's parts `title_skeleton` and `description_skeleton` say (a line of the title's height, a share of the
  room), the row saying to tooling that it is busy and still called by its title. Ids keep the part-ids rule: the
  row that is pressed is the list item's root and has its id `x`, as every pressed component of lurq has its id on
  what is pressed; a row that says something under itself is held with it in a part `item`, `x-item`. The app maps
  its `<id>-open` to `<id>`, and its `<id>-row` to `<id>-item` where the row has something below, and to `<id>`
  where it has not (the row is the whole item then). A second id for one node (a `-press` beside the root's) would
  be two names for it.
- **Why.** What Orchester's kit needs to move its settings rows onto `ui::ListItem`; a row of a list that loads its
  words is common.
- **State.** Built on `claude/k6-c-controls` (2026-11-20): `ListItem::title_pending`, `description_pending`, `ListItemView::bone`; parts `title_skeleton`, `description_skeleton` and `item`. A description still read draws the row as one of two lines.

### A badge's and a status label's word say themselves whole while they are cut

- **Question.** Orchester's cut badges and status chips put their whole word in a tooltip while it is cut. A
  `ui::Badge` that gives way (`truncate`) and a `ui::StatusLabel` cut their word and say nothing. Should they?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** Both: `tooltip_when_cut(true)` puts
  the whole word in a tooltip that shows only while the word is cut, as `Tooltip::when_cut` does; and
  `word_ref(ElementRef)` refers to the word's text, so that an app can put a tooltip of its own on it
  (`Tooltip::when_cut(&word)`, with a hint), or read whether it is cut. Off unless told.
- **Why.** What Orchester's kit needs; the built-in one is what most apps want, and the ref what the rest need.
- **State.** Built on `claude/k6-c-controls` (2026-11-20): `Badge::tooltip_when_cut`, `Badge::word_ref`, `StatusLabel::tooltip_when_cut`, `StatusLabel::word_ref`. A badge on an element (`on`) has no such tooltip.

### How far a skeleton's pulse dims

- **Question.** A skeleton pulses from whole to half its opacity and back. Orchester's design dims to 0.55. Can an
  app or a theme say how far?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** A part of `SkeletonMotion`:
  `SkeletonMotion::dim_to(0.55)` is the opacity the pulse dims to at its middle, 0.5 unless told, in hundredths
  between 0 and 1; a theme gives it with the rest of the motion (`Theme::set_skeleton_motion`), a skeleton its own
  with `Skeleton::motion`. Skeletons that dim alike still pulse in one phase.
- **Why.** What Orchester's kit needs; how far a placeholder dims is part of its motion, which a theme already says.
- **State.** Built on `claude/k6-c-controls` (2026-11-20): `SkeletonMotion::dim_to`, `dimmed`; a pulse's keyframes for each hundredth, lurq's ids `0xFF9B` to `0xFFFF` (`SKELETON_PULSE` stays the one of half).
### A menu that leaves the focus in the app's field

- **Question.** Orchester's slash commands, @mentions and filter fields open a menu under a text field while the
  person goes on typing in it: the arrow keys move a highlighted row, Enter chooses it, Escape closes the menu, and the
  focus never leaves the field (a combobox's `aria-activedescendant`). `ui::Menu` takes the focus as it opens, and its
  rows are reached by moving the focus. Can a menu leave the focus where it is?
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `Menu::keeps_focus(&MenuKeys)`: the panel
  opens without taking the focus, its items take none (a press of one leaves it in the field), and one item is the
  active one (state `active`, `a11y_selected`), the first that can be chosen, again whenever what the menu holds
  changes. `MenuKeys::new(field)` is made once by the app, as a `DateMemory` is, with the element the focus stays in:
  while that element has the focus the panel reads the keys, Up and Down move the active item round the list, Enter
  chooses it, Ctrl+Enter (Cmd+Enter on macOS) asks for it aside, Escape closes the menu, and the rest go on to the
  field. A field whose own keys come first (Enter sends) forwards the key with `MenuKeys::key_down(&event)`, which says
  whether it took it; the panel then leaves that key alone. The pointer on an item makes it the active one.
- **Why.** What Orchester's kit needs to put its slash and mention menus on `ui::Menu`; every app with a search field
  over a list of choices needs the same.
- **State.** Built on `claude/k6-b-menus-calendar` (2026-11-20): `Menu::keeps_focus`, `MenuKeys` (`new`, `field`,
  `active`, `key_down`), `MenuItemState::Active`; and `Popover::keeps_focus(bool)`, a popover that leaves the focus
  where it is. A menu that keeps the focus does not tell the field an `a11y_activedescendant`: the field is the app's,
  and the active item says `a11y_selected`.

### A menu item asked for aside

- **Question.** `Button`, `ListItem`, `Tab` and the sidebar's items have `on_open_aside`, the middle button and a press
  with Ctrl (Cmd) that opens what they stand for beside what is open. Orchester's menu items open places by the same
  rules. `ui::MenuItem` has only `on_select`.
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `MenuItem::on_open_aside(handler)`, as
  `Button`'s: the middle button, a press with Ctrl (Cmd on macOS), and Ctrl+Enter (Cmd+Enter) on the focused item, or
  on the active item of a menu that keeps the focus elsewhere. Such a press is the handler's alone (`on_select` is not
  called) and the menu closes, as after a choice. An item that opens a submenu or a page has none.
- **Why.** What Orchester's kit needs (its `MenuItem::opens`); the same press means the same thing on every control.
- **State.** Built on `claude/k6-b-menus-calendar` (2026-11-20): `MenuItem::on_open_aside`.

### A calendar's heading with its buttons at the end, and its keys read without the focus

- **Question.** Orchester's date picker puts the month first and both buttons after it at the heading's end, and reads
  the calendar's keys at its panel while the focus stays where the app keeps it (the panel, a field): a ring marks the
  day the keys are on, Enter picks it and the picker closes. `ui::Calendar` has the month between its buttons, and its
  keys move the focus from day to day.
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `Calendar::heading(CalendarHeading::MonthThenNav)`
  lays a heading out as the month, then the button before and the button after at its end; `CalendarHeading::Centered`,
  the month between them, stays the default. `Calendar::keys(CalendarKeys::Panel)`: the days take no focus, a day is
  the one the keys are on (state `cursor`, the chosen one or today to begin with), and the panel reads the keys
  wherever the focus is while the calendar is shown: the arrows, PageUp and PageDown (a year with Shift) move that day,
  showing its month, and Enter chooses it, which tells `Calendar::on_choose` (now the app's too, told of every choice,
  of what is chosen already as well) so that a picker closes. A key a handler before it took (`prevent_default`) is
  left alone, and so is Enter where the focus is on what takes Enter itself (a button of the footer). `CalendarKeys::Days`
  stays the default. The day the keys are on is kept in a `DateMemory` or by a mounted calendar.
- **Why.** What Orchester's kit needs to put its picker's grid on `ui::Calendar`.
- **State.** Built on `claude/k6-b-menus-calendar` (2026-11-20): `CalendarHeading`, `CalendarKeys`,
  `Calendar::heading`, `Calendar::keys`, `Calendar::on_choose`, states `cursor` and `nav_at_end`,
  `CalendarView::cursor_day`. Of several months, both buttons stand at the end of the last month's heading.

### Page tabs that keep their places, an icon after a label, and tabs at the line's end

- **Question.** Orchester's page tabs keep the room of the selected label's heavier weight in every tab, so that
  switching moves no tab (as a browser's tab bar does); a tab not built yet ends in a lock after its words; and a
  page's Settings tab stands at the far end of the line after its lists. `ui::Tabs` widens a tab as it is selected, has
  an icon only before the label, and puts only the app's element (`Tabs::end`) at the end.
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `Tabs::reserve_selected_width(true)`: each
  label holds the room it takes in the selected tab's style, so a switch moves nothing; off unless told, so what an app
  sees does not change. `Tab::trailing_icon(name)`: an icon after the label and its count, badge and dot (part
  `trailing_icon`). `Tab::at_end(true)`: the tab stands at the end of the line, after the note and the app's element,
  with the tabs so placed; it is still a tab of the list, reached by its arrow keys and in the More menu in its order.
- **Why.** What Orchester's kit needs to put its page tabs on `ui::Tabs`.
- **State.** Built on `claude/k6-b-menus-calendar` (2026-11-20): `Tabs::reserve_selected_width`, `Tab::trailing_icon`
  (part `trailing_icon`), `Tab::at_end`; mounted tabs count the tabs at the end in what the list needs to stand whole.

### A dialog's first focus, key caps in the app's words, and a note's elements that wrap

- **Question.** Orchester's dialogs that Enter confirms (delete, save) give the focus to their main action, or to no
  control, so that Enter does what the note says; `ui::Dialog` gives it to its first stop of Tab (Cancel), and Enter
  presses Cancel. Their note writes the keys in the app's own words ("Enter", "Esc", translated) and the caps have ids;
  `note_key` writes the platform's names and glyphs, and its caps have none. A long reason in the note
  (`note_child`) ran out of the note in place of wrapping.
- **Decision (Ivan, 2026-11-20: "роби", after the K6 list was put to him).** `Button::autofocus(true)`: the focus goes to
  the button as the dialog, popover or menu it is in opens (the engine's `autofocus`, which `ui::Button` did not pass
  on). `Dialog::initial_focus(DialogFocus::Panel)`: the panel itself takes the focus, a stop of Tab before its first
  control, so Enter confirms and presses no button; `DialogFocus::FirstControl` stays the default.
  `Dialog::note_key_worded(cap, words)`: a cap that says the app's word as it is. Every cap of the note is named by
  the new part `note_key_cap` (`x-note_key_cap`, `x-note_key_cap-2`). An element `note_child` puts in the note gives
  way where the note is short and is never wider than it, so a text in it wraps.
- **Why.** What Orchester's kit needs to put its dialogs on `ui::Dialog`.
- **State.** Built on `claude/k6-b-menus-calendar` (2026-11-20).

### Window tabs that keep a gap between them, and fit by it

- **Question.** Orchester's design puts 2 between its title-bar tabs and before the add button and the More button.
  `ui::WindowTabs` counted the strip's width from its tabs, its separators and its add button, never a gap, and in
  the list mode it counted the separator of a tab only while the tab was in the strip: a tab that left for the More
  button's menu needed less room to come back than it took once back, and with a gap it never came back at all.
  Should the strip keep a gap, and how does it fit?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** The gap of the `list` part is the room between two tabs
  and before the More button, the gap of the `root` part the room before the add button; both are counted wherever
  the strip reckons its width, and a tab leaves the strip with its gap and its separator, so it comes back as soon as
  the room it took is there again. A separator stands by the tabs on either side of it whether they are in the strip
  or not, and the separator before the add button keeps its room in the list mode whether it shows or not. A separator
  the recipe makes 0 wide is not laid out. The default recipe has no gap: what an app sees does not change.
- **Why.** What Orchester's kit needs: its strip is lurq's, with the design's gap, and a tab that left comes back when
  the window grows.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20): the gaps of `list` and `root` counted, the list mode's strip as wide as its tabs are laid out (the list in a column, so the strip's line takes room from it and does not drop it whole), separators by the tabs on either side whatever is hidden, the one before the add button keeping its room in the list mode.

### The More button's menu may list every open tab

- **Question.** Orchester's tab list (its old `chrome/tab_list`) is the list of all open tabs, not only the hidden
  ones: a search field once there are more than eight, a close button on each row (and Ctrl/Cmd+W on the row the keys
  are on), marks for the current tab and for the tabs not in the strip, Ctrl+Shift+A (Cmd+Shift+A) to open it, a
  tooltip on its button with the count of hidden tabs, and its button pressed with an up-chevron while it is open.
  `WindowTabs`' list mode lists only the hidden tabs in a plain menu. Should lurq's list do Orchester's?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** Yes, as an option of the list mode:
  `WindowTabs::menu(WindowTabsMenu::All)` makes the More button's menu a panel of every tab in the strip's order,
  with all of the above; `WindowTabsMenu::Hidden`, the menu of the hidden tabs, stays the default.
  `WindowTabs::list_shortcut(bool)` is the optional Ctrl+Shift+A (Cmd+Shift+A on macOS) that opens and closes it,
  apart from the window's other shortcuts. Every word is in the locale; the More button says while it is open with
  a state of the strip (`menu_open`) a recipe styles, and its chevron points up then.
- **Why.** What Orchester's kit needs to drop its own list and keep its behaviour; a browser's tab search is the same
  thing, none of it Orchester's alone.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20): `WindowTabsMenu`, `WindowTabs::menu` and `list_shortcut`, the state `MenuOpen`, the parts of the list, four words of the locale; the gallery's "Every open tab in a list".

### A window tab as wide as what it shows, up to a cap

- **Question.** Orchester measures each tab's label after a layout (`tab_measure`, `tab_fit`) to give a tab the
  width of what it shows, capped. Can lurq size a tab to its content?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `WindowTab::fit_content(max)`: the tab is as wide as
  its icon, label, marks and close slot, up to `max`, where its label ends in an ellipsis; in the scroll mode it
  still gives way down to the recipe's least width, and in the list mode it keeps that width and leaves the strip
  whole. The strip is as wide as such tabs are laid out, with no measuring of the app's.
- **Why.** What Orchester's kit needs to drop `tab_measure` and `tab_fit`.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20). In the scroll mode the strip is reckoned with such a tab at its cap, a label's width being unknown before the layout.

### A tag's remove handler is given the event

- **Question.** Orchester's chips sit in rows a press opens; the press on a chip's remove button must not reach the
  row. `Tag::on_remove` is given nothing. Should it be given the event?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** Additively: `Tag::on_remove_event(|event| ..)` is given a
  `TagRemoveEvent`, the press or the key that took the tag away, which the handler stops (`stop_propagation`) so
  nothing around the tag hears it. `on_remove(|| ..)` stays as it was.
- **Why.** What Orchester's kit needs; breaking `on_remove` for it would break every app that has one.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20).

### A badge that cuts its word where it has less room

- **Question.** Orchester's badges stand in rows that narrow, and cut their word with an ellipsis there. A
  `ui::Badge` keeps its width. Should it give way?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Badge::truncate(true)`: the badge gives way where it
  has less room than its word, down to its dot and icon and its padding, and its word ends in an ellipsis; what
  reads the tree is still told the whole word. Off unless told.
- **Why.** What Orchester's kit needs.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20).

### A part's foreground wins over the color of its typography role

- **Question.** lurq colored a part's text by the part's `foreground` only when the text's style had the default
  color, so an app whose typography roles carry colors (Orchester's) saw a recipe's `foreground` ignored: a primary
  button's label in the role's color and not on the accent. Orchester works around it with a twin of each role in no
  color. Which wins?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** A part style that sets a `foreground`, at rest or in any
  of its states, colors its own text even where its typography role names a color: the role's color is dropped from
  the text of that part, which takes the part's foreground. Where the part sets a foreground only in a state, the
  role's color is the text's at rest. A role's color still applies where the part sets no foreground at all.
- **Why.** A recipe says what a part looks like; its `foreground` is the more particular of the two, and the role's
  color is a default for text that names nothing else.
- **State.** Built on `claude/k5-a-tabs-small` (2026-11-20), in `ResolvedPart::text`, the text every renderer draws a part's words with. A text a renderer resolves from `TextPatch::resolve` itself (a highlight's runs, a watermark) is as it was.
### Tabs with a note at the end, a dot, a count still read, and a tab a press leaves alone

- **Question.** Orchester's page tabs (`ui/tabs.rs` of its kit) have quiet words at the strip's end (why a tab cannot
  be opened), a rule between groups of tabs, a dot after a tab's count in the tone of what it counts (a service that
  crashed), a quiet count apart from the badge of what waits for the person, a count's bone while the count is read,
  and the tab of the page shown that a press leaves alone (it keeps the panel it shows). `ui::Tabs` has the rule
  (`Tab::separated`) and the quiet count (`Tab::count`, apart from `Tab::badge`), and none of the rest. Can it host
  them (K5, package C)?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Tabs::note(&str)` puts quiet words at the end of the
  list's line (part `note`), and `Tabs::end(element)` an element of the app's there (part `end`); both give way first
  where the line is short. `Tab::dot(PaletteColor)` draws a dot after the count and the badge in a role of the
  palette (part `dot`, its size the recipe's). `Tab::count_pending(true)` shows a skeleton in the count's place,
  as wide as one digit (part `count`, its bone `count_skeleton`), and says the tab is busy to tooling.
  `Tab::inert(true)` is a tab a press leaves alone: it is drawn as it is, selected or not and not dimmed, but a
  press, its item in the menu of the overflowing form and the arrow keys neither select nor focus it (state `inert`,
  no pointer cursor). The rule and the quiet count are already lurq's.
- **Why.** Each is what a page's tabs show in many apps; with them the kit's page tabs are `ui::Tabs` dressed by the
  app's recipe.
- **State.** Built on `claude/k5-c-rows-forms` (2026-11-20): `Tabs::note`, `Tabs::end`, `Tab::dot`,
  `Tab::count_pending`, `Tab::inert`; parts `dot`, `count_skeleton`, `note`, `end`, state `inert`;
  `TabsView::count_bone`, `end_of_line`. The note and the element are not counted in what the list needs to stand
  whole, so a list that just fits keeps its form and the note gives way.

### A list item with tags and an (i) after its title, elements under its words, and parts under the row

- **Question.** Orchester's grouped rows and settings rows (`ui/row_group.rs`, `ui/setting_rows.rs`) have tags after
  the label (*Default*, *Changed*), an (i) after it with the rest of what the setting does in its tooltip, a link
  under the hint, and what the row says under itself (a problem, a code to copy) inside the row's box under it.
  `ui::ListItem` has a title and a description of text. Can it hold them, and which ids does the app read?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `ListItem::tag(element)` puts the app's elements after the
  title, on its line (part `tags`); `ListItem::info(&str)` the mark of an `InfoLabel` after the title and the tags,
  its words in a tooltip (part `info`) that shows wherever the row is rendered; `ListItem::detail(element)` an element
  of the app's under the description in the text's column (part `detail`): a link, a path; `ListItem::below(element)`
  what the row says under itself, across the row under its line, in a part `below` that a press of the row does not
  reach. The ids stay lurq's part names: `x-title` and `x-description`, not the kit's `x-label` and `x-hint`. The app
  reads lurq's (its tests and its MCP readers move to `-title` and `-description`); a lurq part is named after what it
  is in every component, and a second name for one part would be two ids for one node.
- **Why.** A settings row and a grouped row are list items with a few more things around the title; with them the
  kit's rows are `ui::ListItem` in a `ui::List`.
- **State.** Built on `claude/k5-c-rows-forms` (2026-11-20): `ListItem::tag`, `info`, `detail`, `below`; parts `tags`,
  `info`, `detail`, `below`; the title's node now has its id `x-title`. A row with something below it is a column
  of the row (its id) and the part `below`, the key on the column.

### A field without a label, and rows under its description

- **Question.** Orchester's form fields (`ui/field/` of its kit) are often a control alone with its hint, named by the
  control (a search box, a field of a table's row), and its controls add error rows of their own under the field.
  A `ui::Field` with an empty label draws none, but gives some controls an empty name over their own, and has
  nothing under its description. Can it?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Field::unlabeled(control)` is a field with no label: the
  control keeps the name it has of its own (its hidden label, its own `label`), and no control takes an empty name
  from a field (each control's `in_field` leaves its name alone where the field's label is empty). A field beside
  its label (`Horizontal`) keeps the label's column, so its control stands in line with the others of the form.
  `Field::below(element)` adds the app's elements under the description, one under another (part `below`): error
  rows, a count, a link.
- **Why.** The kit's fields are lurq's only if a field can be a control alone, and if what the app says under it has a
  place that is the field's.
- **State.** Built on `claude/k5-c-rows-forms` (2026-11-20): `Field::unlabeled`, `Field::below` (part `below`); the
  controls that took an empty name from a field (a radio and a switch with their labels hidden, a select, a
  listbox, the groups, a rating and others) keep theirs, and a drop zone keeps a label of its own.

### Key caps in a dialog's note, and a veil that leaves a band of the app's at the top

- **Question.** Orchester's confirmations say at their footer's start what Enter does and that Esc cancels, each with
  its key cap (`Keys` in `08e · History · Delete confirmation`), and its scrim stops under its title bar. A
  `ui::Dialog`'s note is words alone; its veil starts under the title bar a `WindowChrome` says
  (`WindowHandle::title_bar_inset`), and nothing else. Can a note hold key caps, and can the veil leave more of the
  top?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Dialog::note_key(keys, words)` adds a key cap (a
  `ui::Kbd` of `keys`, as `Kbd::shortcut` writes them for the platform; part `note_key`) and its words in the note's
  type to the note, after its text, in the order given; `Dialog::note_child(element)` adds any element of the app's
  there. A note of keys alone is a note. `Dialog::inset_top(f32)` leaves that much of the top of what the veil covers
  to what is under it, which takes the pointer there (a bar the app draws itself, not a `WindowChrome`); the panel is
  centred in the rest. The title bar of a `WindowChrome` is left without it, as before.
- **Why.** What a key does is said with its key cap in many apps' footers; and a bar that is not the chrome's is still
  the app's to keep in reach while a dialog asks.
- **State.** Built on `claude/k5-c-rows-forms` (2026-11-20): `Dialog::note_key`, `note_child`, `inset_top`; part
  `note_key` (its id `x-note_key`, `x-note_key-2`). A note with key caps wraps them onto more lines where it is
  short, and cuts a pair's words where one pair is wider than the note.

### A button whose label takes its room, whose description wraps, and that tells its focus

- **Question.** Orchester's select-like buttons put their chevron at the button's end with the label taking the room
  before it; its stacked answers say what a press leads to under the label in several lines; and an answer tells the
  app when the keyboard focuses it (its supporting copy changes). A `ui::Button` centres its content, cuts its
  description to one line, and has no focus handler. Can it?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Button::fill_label(true)` (state `fill_label`): the
  label (with its description) takes the room the button has, its content starts at the button's start, and what is
  after the label (a trailing icon, an element) stands at the end. `Button::description_wraps(true)` (state
  `description_wraps`): the description goes on to the next lines in the room the button has, in place of one line
  cut with an ellipsis. `Button::on_focus(handler)` is called when the button takes the focus, from the keyboard or
  from the app; a second handler is called after the first.
- **Why.** A select's trigger, a stacked answer and a focus that changes what is said beside it are buttons in the
  kit; with these they are `ui::Button`, and the kit's own frame goes.
- **State.** Built on `claude/k5-c-rows-forms` (2026-11-20): `Button::fill_label`, `description_wraps`, `on_focus`;
  states `fill_label` and `description_wraps`. The description's id stays `x-description` (the kit's stacked answer
  has `-consequence`: the app reads lurq's).
### A menu whose rows are the app's, and panels placed by another frame

- **Question.** Orchester's menus (`ui/menu.rs` of its kit, and the menus of its pickers, its slash and mention
  lists, its context meter) are a panel the app builds itself: its own rows (a label cut to its room with its words in
  a tooltip, a detail after the label on one line or under it, a hint at the end, a row 40 high for an agent and its
  model, a row of an unavailable choice only faded whose reason a foot under the rows says once), separators, a foot
  of words, sections. `ui::Menu` owns its item list and draws each item itself: the kit could take its look,
  keyboard, focus and dismissal only by giving up its rows. Its popovers and menus also open by a frame that is not
  their trigger (the status bar's popover above the bar and at its end while the bar's item stays the trigger) or at
  a point. Should a lurq menu take the app's rows, and a popup be placed by another element?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** Yes:
  1. A `MenuItem` takes the app's element as all it shows (`MenuItem::content(element)`), in place of its icon, label,
     detail and end; the item keeps everything else: its root is the item that is pressed, reached by the arrows,
     found by typing its label, called by its label to tooling, checked, disabled or unavailable for a reason
     (`a11y_description`), destructive, and choosing it closes the menu and calls its handlers. The app's recipe of
     `menu_item` gives the root its height, its padding and its fill under the pointer. So a row of the app's is a
     menu's row and the keyboard works, with no role of the menu's for the app to put on an element of its own.
  2. A menu holds the app's elements among its entries (`Menu::child`, `Menu::children`, `MenuEntry::Element`): a
     foot, a section of its own, a field. Drawn as given; one that takes the focus is reached by the arrows as an item
     is, and choosing it does what the app wires.
  3. A popup may be placed by another element than its anchor (`Overlay::placed_by`, `Popup::placed_by`,
     `Menu::placed_by`, `Popover::placed_by`): it stands at its placement of that element's box, flips and is clamped
     as it would by its anchor, and the anchor stays what it hangs from (its trigger, where the focus goes back, what
     hides it out of view). A menu and a popover may stand at a point of the window too (`Menu::at`, `Popover::at`),
     as a context menu does.
- **Why.** The kit's menus keep their rows, which are its design, and get lurq's menu: the roving focus, the typing,
  Escape, a press outside, Tab, the focus given back, flipping and clamping, `within`. A popover by the status bar no
  longer reckons its offset from where two elements were last laid out.
- **State.** Built on `claude/k5-d-overlays` (2026-11-20).

### A hover card the pointer passes through

- **Question.** The cards of Orchester's status bar show what an item of the bar is (the cloud, the MCP server, the
  runner) while the pointer rests on it: they say words, hold nothing to press, let the pointer through to what is
  under them, and go as soon as the pointer leaves the item. `ui::HoverCard` takes the pointer, stays while it is on
  the card and goes after a delay to cross to it. Can a hover card do both?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `HoverCard::pass_through(true)`: the card takes no
  pointer (what is under it is hovered and pressed through it), and it goes as the pointer leaves the element it is
  about, with no delay. The focus and Escape do what they did. Staying open while the card is hovered stays the
  default.
- **Why.** A card of words over a bar the pointer moves along must not hide the next item from the pointer, nor
  linger over it.
- **State.** Built on `claude/k5-d-overlays` (2026-11-20).

### Tooltips placed by a frame, held by the app, and worded for a cut text

- **Question.** Orchester's tooltips do three things `ui::Tooltip` does not: a skill chip's tooltip opens over the
  composer box the chip sits in, clear of the box's top, and not over the chip; a message's actions show their
  tooltips themselves, the app saying which one shows ("hung" tooltips, with no hover of the tooltip's own); and a
  browser tab's label says one thing in its tooltip while it fits (the page's title) and another while it is cut.
  Can lurq's tooltip do them?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").**
  1. `Tooltip::over(&frame)`: the tooltip keeps clear of `frame`, an element around the one it describes, on the side
     it opens on: its distance is measured from the frame's edge and it lines up with its element along it
     (`Overlay::clear_of`). It flips as it would.
  2. `Tooltip::hang(anchor, open)`: a tooltip the app shows and hides itself beside an element of its own, open while
     `open` is (a `bool` or a `Signal<bool>`): the pointer, the focus, a press and Escape do nothing to it, it takes no
     pointer, and it is an element the app puts among the children of any container.
  3. `Tooltip::hint(text)` with `when_cut`: what the tooltip says while the text is whole, in place of showing
     nothing; while the text is cut it says its own text, as before.
- **Why.** Each is how the kit's tooltips behave today, and none is Orchester's alone: a tooltip over a field's box, a
  toolbar that shows its tooltips itself, a tab's title.
- **State.** Built on `claude/k5-d-overlays` (2026-11-20).
### Copies that go where the app says, not to the system's clipboard

- **Question.** Orchester's copy buttons (its code box, the commands and links it offers to copy) write through a
  copier the app provides once: a QA copy of the app and its gallery copy to a clipboard of their own and never
  touch the person's. A copy of lurq's (a `ui::CodeBlock`'s button, a Markdown code block's, a table's, Ctrl+C in a
  field, `WindowHandle::copy_to_clipboard`) goes to the system's clipboard in a window, and only a test's `Tree`
  can put another writer in its place. Can an app say once where lurq's copies go?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** Yes, once for the app: `App::set_clipboard_writer` (and
  `App::set_clipboard_reader` for what a paste reads), which every copy and paste of every window of the app goes
  through, the button of a code block and the keys alike, in place of the system's clipboard; a test's writer on its
  `Tree` stays over the app's. A code block's button says it copied, or could not, by what the writer answers, as
  it says by what the system's clipboard answers.
- **Why.** What a kit's copier is for is the whole app's, not one button's: a QA copy that writes the person's
  clipboard from one place is as wrong as from another, so the hook is the app's and lurq's own copies keep their
  behaviour.
- **State.** Built on `claude/k5-e-misc` (2026-11-20).

### A calendar that marks the other end of a range, in the app's own words

- **Question.** Orchester picks the start and the end of a range in two pickers of one date each, and the calendar
  of one marks the date the other holds (the other end) in a tint of its own while the person picks. And it names
  its months and weekdays in its own words. A `ui::Calendar` of one date marks no other day, and does it take the
  app's words?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Calendar::other_end(Option<Date>)` marks the day of the
  other end of a range the calendar's date is one end of: a state `other_end` of the day, which the design's recipe
  tints as the days inside a range are, and which tooling reads (`range_other_end`). The words were already the
  app's: a calendar takes its month names, its weekdays' heads and their names from `Locale::dates` (or
  `App::set_date_locale`), whose fields an app may change one by one; this is documented and tested.
- **Why.** The kit's date picker moves onto `ui::Calendar` only with the mark it draws, and its words must be
  Orchester's own.
- **State.** Built on `claude/k5-e-misc` (2026-11-20).

### Skeletons that pulse in steps, and bones laid out as the app chains them

- **Question.** Orchester's skeletons pulse together, in 16 steps a half pulse rather than smoothly, so the window
  draws a few frames a second while one is shown, and stand still where motion is reduced. Its bones (~114 call
  sites) are columns that it sizes, rounds and flexes with a column's builder. `ui::Skeleton` pulses smoothly, and
  takes a width and a height only. Can lurq's do both?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** A `SkeletonMotion` says how skeletons move:
  `SkeletonMotion::SMOOTH`, as before, or `SkeletonMotion::stepped(per_second)`, each pose held until the next and
  no frame asked for between, with its `period`; a theme gives the one of every skeleton not told its own
  (`Theme::set_skeleton_motion`), and `Skeleton::motion` one skeleton's. Every skeleton still pulses in one phase,
  and stands still where motion is reduced, as before. A skeleton takes the layout an app chains on a bone:
  `radius`, `min_width`, `max_width`, `min_height`, `max_height`, `flex` and `flex_shrink`, beside its `width` and
  `height`. lurq has no margins, so there are none to take.
- **Why.** The kit's bones and its pulse move onto `ui::Skeleton` only where the bones keep their shapes and the
  pulse its cadence.
- **State.** Built on `claude/k5-e-misc` (2026-11-20).

### A radio pressed as a whole, its id on the whole, and a press kept from a row around it

- **Question.** As for a switch and a checkbox (the entry of K5-B below): Orchester's radio is one control of its
  mark and its label, found by its id as a whole and pressed inside rows. A `ui::Radio` has its id on the mark, and
  a press of it reaches a row around it.
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Radio::whole(true)` and `RadioGroup::whole(true)`
  make the mark and its label one control: what is pressed, focused and ringed is the whole of them, one stop of
  Tab (one stop for the group, as before), its id on the whole and the mark, the dot, the label and the count named
  after it (`x-mark`, `x-dot`, `x-label`, `x-count`). A press of a radio, whole or not, its label too, stops there:
  a row around it is not pressed.
- **Why.** The same as for the switch and the checkbox, for the third of the kit's toggles.
- **State.** Built on `claude/k5-e-misc` (2026-11-20).

### Window tabs that hold Orchester's title-bar tabs

- **Question.** Orchester draws the tabs of its title bar itself: a Home tab of an icon alone, tabs capped at a width
  of the app's, an attention mark after a label (an amber hand with a count, or a red alert, with a tooltip), a
  middle press that closes any tab a person may close, tabs that do not fit put in a list rather than scrolled, a
  strip Tab passes by, the dot of unsaved changes beside a close button that stays, and a bar 36 high. `ui::WindowTabs`
  and `ui::WindowChrome` could do none of it. Should lurq's window tabs host them, and how?
- **Decision (Ivan, 2026-11-20: "так, давай обидва показувати одночасно - фікси ці всі пройоби в луркі").** Yes,
  all of it, in lurq:
  1. A tab takes the app's elements after its label (`WindowTab::trailing`), in a part of their own.
  2. A tab takes `describe(name, value)`, said on the tab's node before what the strip says of it, as `Styled::describe`
     is on a component.
  3. The middle button closes any tab a person may close (Ivan: "закриваємо будь яку"), whether its close button is
     shown or not; whether a person may close a tab is `WindowTab::closable(bool)`, apart from whether its close
     button shows (`close_button(bool)`). A tab that is not closable is closed by nothing a person does.
  4. A second overflow mode (Ivan: "треба окремим режимом табок"): the tabs that do not fit leave the strip, last
     first and never the selected one, into the menu of a button at the strip's end that shows how many left;
     choosing one there selects it. Shrinking and then scrolling stays the default.
  5. `tab_stop(false)` on the strip, by the mechanism of 0.63.2's pressables: Tab passes its tabs, its add button
     and its list button by, and a press leaves the focus where it was.
  6. A mode of the dot of unsaved changes where it stands right after the label and the close button keeps its slot
     and shows, with no swap under the pointer. The swap stays the default.
  7. A tab's own width and most width, and a tab of its icon alone (a Home tab, 28 wide).
  8. The height of a bar of `ui::WindowChrome` with tabs is its recipe's, as a bar without tabs is: the default stays
     40, and an app's recipe makes it 36.
- **Why.** Orchester's title bar is to be lurq's component, not a copy of it in the app; each of these is what a
  browser's or an editor's tabs do, and none is Orchester's alone.
- **State.** Built on `claude/window-tabs` (2026-11-20).

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

### A card that clips to its corners and gives its height to its body

- **Question.** Orchester's cards (`ui/card.rs` of its kit) clip what is in them to their rounded corners (a table, a
  picture, a header's fill reach the edge), and a card of a fixed height or a share of a row (`flex(1.0)`, the usage
  figure cards 118 high) gives that height to its body, so that what is in it with `flex` fills it. A `ui::Card` lets
  what is in it paint over its corners, and its content and body are as high as what is in them, whatever the card is.
  Can lurq's card do both?
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** What is in a card is clipped to its rounded corners by
  default: every element of lurq clips what is in it to its own box and corners unless told `overflow_visible`, a
  card's surface too, so no option is added; a test holds it. A card given a height of its own (`height`, or
  its recipe's) gives what its header leaves to its body and what its footer leaves to its content, so children with
  `flex` fill it and the footer stays at the bottom. A card as high as what is in it, or given only a least height, is
  as before: a column that grows takes all the room it may in lurq's layout, so the body grows only where the card's
  height is its own (`CardView::has_height`). A card that is a share of a column takes `height(Length::Fill)`.
- **Why.** The kit's card is a column that clips; a card that could do neither kept the kit on its own.
- **State.** Built on `claude/k5-b-surfaces` (2026-11-20).

### A data list's term at the top of a value that wraps, and its id kept by a theme's renderer

- **Question.** Orchester's detail rows (`ui/detail_row.rs`) put the term at the top of its value, where the value
  wraps onto several lines or is an element of several lines. A `ui::DataList` centers the term on the value's height.
  And a theme's renderer of a data list could not put the list's id on its root: the view kept it to itself.
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `DataList::align(Alignment)` says how a term and its
  value stand to each other across: `Center` as before, or `Start`, the term at the top of the value. The view gives a
  renderer the id and the key (`DataListView::id`, `key`), and a root a renderer builds with no id of its own is given
  the list's.
- **Why.** The kit's detail rows move onto `DataList` only with their term where the design draws it, and a list whose
  id a theme drops cannot be found by the tests that read it.
- **State.** Built on `claude/k5-b-surfaces` (2026-11-20).

### An alert at the top of its first line, with a note, without a title, and in the inspector's layout

- **Question.** Orchester's notices (`ui/notice.rs`, `ui/setting_notice.rs`) put the icon and the actions at the top
  of the first line where the actions stand at the end, say a quieter note after the actions (Settings), may have no
  title (Settings), and in an inspector of 340 have the icon beside the title alone, with the words, the parts and the
  actions under them across the whole width. A `ui::Alert` centers what is in it on its actions, has a title always,
  and no note and no such layout. Its kit's ids are `<id>-icon`, `-text`, `-title`, `-body`, `-actions`, `-note`.
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Alert::align_top(true)` (state `align_top`): the icon,
  the words, the actions and the close control stand at the top, the actions on the first line. `Alert::note(&str)`:
  a quieter line after the actions (part `note`, state `note`). An empty title draws no title part
  (`Alert::untitled()`), and the alert is then called by its description. `Alert::compact(true)` (state `compact`): the
  icon beside the title alone, the description, the body and the actions under them across the alert, the actions
  under the words. `Alert::actions_end(true)` (state `actions_end`): actions under the words stand at the far end.
  Ids keep the part-ids rule: the kit's `-body` is lurq's `-description` (the kit's words under the title), its
  `-actions` lurq's `-action` (the part's name, kept so a theme's rule on it holds), `-icon`, `-text`, `-title` and
  `-note` are the same; the app renames the two it reads.
- **Why.** The kit's notices are built on `ui::Alert` only where it draws them as the design does.
- **State.** Built on `claude/k5-b-surfaces` (2026-11-20).

### A breadcrumb's links cut at a width, with their whole name in a tooltip while cut

- **Question.** Orchester's breadcrumb (`ui/breadcrumb.rs`) cuts an ancestor's name at 160 with an ellipsis and shows
  it whole in a tooltip while it is cut; ancestors keep their room and the page gives way. A `ui::Breadcrumb`'s links
  are as wide as their names.
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Breadcrumb::link_max_width(length)`: each link is at
  most that wide and keeps its room, its name cut with an ellipsis, and a tooltip says the name whole while it is cut
  (`Tooltip::when_cut`).
- **Why.** A trail of long names (folders, projects) fits its bar only if each of them gives up room, and a name cut
  must still be readable.
- **State.** Built on `claude/k5-b-surfaces` (2026-11-20).

### A switch and a checkbox pressed as a whole, their id on the whole, and a press kept from a row around them

- **Question.** Orchester's toggles (`ui/toggle.rs`) are one control: the mark and the label pressed together, one
  ring around both, the id on the whole and its parts `<id>-mark`, `<id>-knob`, `<id>-label`; a press of one does
  not reach the row it stands in. A `ui::Switch` and a `ui::Checkbox` put the id on the track or the box alone, and a
  press of them goes on to a pressable row around them.
- **Decision (Ivan, 2026-11-20: "так давай роби в луркі").** `Switch::whole(true)` and `Checkbox::whole(true)`
  (state `whole`): the root is what is pressed, focused and ringed, with the track or the box and the label in it; the
  id is on the root and the parts are named after it. lurq keeps its part names: the kit's `-mark` is a switch's
  `-track` and a checkbox's `-box`, its `-knob` a switch's `-thumb`, and `-label` is the same. A press of a switch or
  a checkbox, whole or not, its label too, stops there: a row around it is not pressed.
- **Why.** The kit's toggle is found by its id as a whole and pressed inside rows; with the id on the box alone and a
  press that also pressed the row, it could not move onto lurq.
- **State.** Built on `claude/k5-b-surfaces` (2026-11-20).

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
