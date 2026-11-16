//! The page of the sortable list: its playground, the board's list, a selection dragged together, rows dragged by
//! their handle alone, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Interaction, Length, SortableItem, SortableList, SortableListPart, Switch, icons, move_rows},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

type Id = &'static str;

/// The width of the board's list.
const WIDE: f32 = 300.0;

/// The board's rows, with what each is about.
const SECTIONS: [(Id, &str); 5] = [
  ("Buttons", "Button, toggle, split button"),
  ("Inputs", "Text, number, password"),
  ("Tabs", "Line, pill, segmented"),
  ("Toggles", "Switch, checkbox, radio"),
  ("Menus", "Menu, context menu, menubar"),
];

fn about(name: Id) -> &'static str {
  SECTIONS
    .iter()
    .find(|(section, _)| *section == name)
    .map_or("", |(_, about)| about)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What reorders a list of the page as the list on a stage tells it.
fn reorders(order: &Signal<Vec<Id>>) -> impl Fn(usize, usize) + Send + Sync + 'static {
  let order = order.clone();
  move |from, to| {
    order.update(|rows| {
      let row = rows.remove(from);
      rows.insert(to, row);
    })
  }
}

pub struct SortableListPage {
  frame: Frame,
  playground: Playground,
  handle: Signal<bool>,
  handle_only: Signal<bool>,
  selects: Signal<bool>,
  descriptions: Signal<bool>,
  disabled_row: Signal<bool>,
  disabled: Signal<bool>,
  played: Signal<Vec<Id>>,
  played_selected: Signal<Vec<Id>>,
  order: Signal<Vec<Id>>,
  many: Signal<Vec<Id>>,
  many_selected: Signal<Vec<Id>>,
  switches: Signal<Vec<Id>>,
  on: Signal<Vec<Id>>,
}

impl SortableListPage {
  /// The playground: the list as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (handle, handle_only, selects) = (self.handle.get(), self.handle_only.get(), self.selects.get());
    let (descriptions, disabled_row, disabled) =
      (self.descriptions.get(), self.disabled_row.get(), self.disabled.get());
    let (order, selected) = (self.played.get(), self.played_selected.get());
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let rows = move |order: &[Id]| -> Vec<SortableItem<Id>> {
      order
        .iter()
        .map(|name| {
          let mut row = SortableItem::new(*name, name).disabled(disabled_row && *name == "Toggles");
          if descriptions {
            row = row.description(about(name));
          }
          row
        })
        .collect()
    };
    let build = {
      let (set, set_selected, record) = (self.played.clone(), self.played_selected.clone(), record.clone());
      move |ctx: &mut Ctx, order: Vec<Id>, selected: Vec<Id>| -> Element {
        let (one, many) = (set.clone(), set.clone());
        let (told_one, told_many, told_selected) = (record.clone(), record.clone(), record.clone());
        let set_selected = set_selected.clone();
        let mut list = SortableList::new(rows(&order))
          .label("Sections")
          .handle(handle)
          .handle_only(handle_only)
          .disabled(disabled)
          .force(forced)
          .style(SortableListPart::Root, |s| s.width(WIDE).max_width(Length::Fill))
          .on_reorder(move |from, to| {
            told_one(format!("on_reorder({from}, {to})"));
            one.update(|rows| {
              let row = rows.remove(from);
              rows.insert(to, row);
            });
          })
          .on_reorder_many(move |rows, to| {
            told_many(format!("on_reorder_many({rows:?}, {to})"));
            many.update(|order| move_rows(order, rows, to));
          });
        if selects {
          list = list.selected_controlled(selected, move |next| {
            told_selected(format!("on_selected_change({next:?})"));
            set_selected.set(next.to_vec());
          });
        }
        list.mount(ctx)
      }
    };
    let shown = {
      let (build, order, selected) = (build.clone(), order.clone(), selected.clone());
      Shown::built(move |ctx| build(ctx, order, selected))
    };
    {
      let (build, order, selected) = (build.clone(), self.played.clone(), self.played_selected.clone());
      self
        .playground
        .window_shows(move |ctx| build(ctx, order.get(), selected.get()));
    }
    let make = {
      let (order, selected) = (order.clone(), selected.clone());
      move || -> Element {
        let mut list = SortableList::new(rows(&order))
          .handle(handle)
          .handle_only(handle_only)
          .disabled(disabled)
          .style(SortableListPart::Root, |s| s.width(WIDE).max_width(Length::Fill));
        if selects {
          list = list.selected_controlled(selected.clone(), |_| {});
        }
        list.into()
      }
    };

    let mut code = vec![
      "// sections: Signal<Vec<&str>>, the rows in the app's order".to_owned(),
      if descriptions {
        "SortableList::new(sections.get().into_iter().map(|name| SortableItem::new(name, name).description(..)))"
          .to_owned()
      } else {
        "SortableList::new(sections.get().into_iter().map(|name| (name, name)))".to_owned()
      },
      ".label(\"Sections\")".to_owned(),
    ];
    for (on, call) in [
      (!handle && !handle_only, ".handle(false)"),
      (handle_only, ".handle_only(true)"),
      (selects, ".selected(selected)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push(
      ".on_reorder(|from, to| sections.update(|rows| { let row = rows.remove(from); rows.insert(to, row); }))"
        .to_owned(),
    );
    if selects {
      code.push(".on_reorder_many(|rows, to| sections.update(|order| move_rows(order, rows, to)))".to_owned());
    }
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("root        {}   rows 4 apart", self.playground.measured()),
      "└─ item     a list item: 36 high on the card, an edge of 1, 6 round".to_owned(),
      format!(
        "   ├─ handle   14{}",
        if handle || handle_only {
          "   the grip, 6 from the start"
        } else {
          "   none"
        }
      ),
      "   ├─ title    body at 13".to_owned(),
      "   └─ description  12, muted, where the row has one".to_owned(),
      "while rows are dragged".to_owned(),
      "├─ ghost    in the place of each row dragged".to_owned(),
      "├─ slot     the room between two rows, which opens round the line: 4, the line of 2, 4".to_owned(),
      "├─ line     2 in the info color, with a dot of 6: where they would go".to_owned(),
      "├─ lifted   the row under the pointer, 40 narrower than the list".to_owned(),
      "└─ count    20, on the corner of the row under the pointer, where they are several".to_owned(),
    ];

    let flags = [
      self.handle_only.clone(),
      self.selects.clone(),
      self.descriptions.clone(),
      self.disabled_row.clone(),
      self.disabled.clone(),
    ];
    let (reset_handle, reset_order, reset_selected) =
      (self.handle.clone(), self.played.clone(), self.played_selected.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Sortable List",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: true,
        facts: if selects {
          format!("{} selected", selected.len())
        } else {
          String::new()
        },
        event: "on_reorder",
        code,
        tree,
        controls: vec![
          flag("handle", &self.handle),
          flag("handle_only", &self.handle_only),
          flag("a selection", &self.selects),
          divider(),
          flag("descriptions", &self.descriptions),
          flag("a disabled row", &self.disabled_row),
          flag("disabled", &self.disabled),
        ],
        changed: [!handle, handle_only, selects, descriptions, disabled_row, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          reset_handle.set(true);
          reset_order.set(SECTIONS.iter().map(|(name, _)| *name).collect());
          reset_selected.set(Vec::new());
        }),
      },
    )
  }
}

impl Component for SortableListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let all = || SECTIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      handle: ctx.signal(true),
      handle_only: ctx.signal(false),
      selects: ctx.signal(false),
      descriptions: ctx.signal(false),
      disabled_row: ctx.signal(false),
      disabled: ctx.signal(false),
      played: ctx.signal(all()),
      played_selected: ctx.signal(Vec::new()),
      order: ctx.signal(vec!["Buttons", "Inputs", "Tabs", "Toggles"]),
      many: ctx.signal(all()),
      many_selected: ctx.signal(vec!["Inputs", "Tabs", "Toggles"]),
      switches: ctx.signal(vec!["Wi-Fi", "Bluetooth", "Notifications"]),
      on: ctx.signal(vec!["Wi-Fi"]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = SortableList::new(self.order.get().into_iter().map(|name| (name, name)))
      .label("Sections")
      .style(SortableListPart::Root, |s| s.width(WIDE).max_width(Length::Fill))
      .on_reorder(reorders(&self.order))
      .key("board")
      .mount(ctx);
    let many_order = self.many.clone();
    let many = SortableList::new(self.many.get().into_iter().map(|name| (name, name)))
      .label("Sections")
      .selected(self.many_selected.clone())
      .style(SortableListPart::Root, |s| s.width(WIDE).max_width(Length::Fill))
      .on_reorder(reorders(&self.many))
      .on_reorder_many(move |rows, to| many_order.update(|order| move_rows(order, rows, to)))
      .key("many")
      .mount(ctx);
    let on = self.on.get();
    let switches = SortableList::new(self.switches.get().into_iter().map(|name| {
      let set = self.on.clone();
      SortableItem::new(name, name).trailing(Switch::new(name).label_hidden(true).on(on.contains(&name)).on_change(
        move |is| {
          set.update(|on| {
            on.retain(|held| *held != name);
            if is {
              on.push(name);
            }
          })
        },
      ))
    }))
    .label("Settings")
    .handle_only(true)
    .style(SortableListPart::Root, |s| s.width(WIDE).max_width(Length::Fill))
    .on_reorder(reorders(&self.switches))
    .key("switches")
    .mount(ctx);
    let state = |list: SortableList<Id>| list.style(SortableListPart::Root, |s| s.width(200.0).max_width(Length::Fill));
    let two = || {
      SortableList::new([
        SortableItem::new("Buttons", "Buttons"),
        SortableItem::new("Inputs", "Inputs").icon(icons::FILE),
      ])
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(two()), "at rest; an icon after the grip"))
      .child(item(
        state(two().force(Some(Interaction::Hovered))),
        "under the pointer",
      ))
      .child(item(
        state(two().force(Some(Interaction::FocusVisible))),
        "focused from the keyboard",
      ))
      .child(item(state(two().selected_controlled(["Buttons"], |_| {})), "selected"))
      .child(item(state(two().disabled(true)), "disabled"));

    let rows = [
      (
        "items",
        "impl IntoIterator<Item = impl Into<SortableItem<T>>>",
        "",
        "The rows in the app's order: `(value, title)` or `SortableItem::new(value, title)` with `description`, \
         `icon`, `trailing`, `disabled`, `id` and `key`.",
      ),
      (
        "on_reorder",
        "impl Fn(usize, usize)",
        "",
        "Told where a row was and where it goes, as it is taken out and put back: \
         `rows.insert(to, rows.remove(from))`.",
      ),
      (
        "on_reorder_many",
        "impl Fn(&[usize], usize)",
        "",
        "Told where several selected rows were and where the first of them goes in the list without them: \
         `move_rows(&mut rows, from, to)`.",
      ),
      (
        "selected",
        "Signal<Vec<T>>",
        "no selection",
        "The rows selected, which are dragged together. `selected_controlled(values, on_selected_change)` is the \
         same held by the app; `on_selected_change(&[T])` hears each change.",
      ),
      ("handle", "bool", "true", "Whether each row has its grip at its start."),
      (
        "handle_only",
        "bool",
        "false",
        "A row is dragged by its grip alone: for rows that hold what is pressed itself.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "No row is moved or selected. A row has its own too.",
      ),
      (
        "label",
        "&str",
        "",
        "What the list is called, to what reads the window's tree.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps a drag from one render to the next: the row follows the pointer, its place is a ghost, and the rows \
         make room for a line where it would go.",
      ),
      (
        "style / rules",
        "SortableListPart",
        "",
        "Root, Item, Handle, Icon, Title, Description, Ghost, Slot, Line, Cap, Count, Lifted.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Sortable List"],
        title: "Sortable List",
        description: "A list whose rows a person puts in another order, by a drag of a row or of its handle and \
                      by the keys.",
        import: Some("use lurq::ui::{SortableItem, SortableList};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SortableList is a control on the right. The list on the stage is live: drag a row, or \
           focus one and press Alt with Up or Down.",
          playground,
        ),
        Section::new(
          "Rows in another order",
          "The board's list. A press that moves past a few pixels is a drag: the row follows the pointer, a ghost \
           stands in its place, and the rows it would go between move apart round a line. Escape puts it back. Tab reaches \
           the list; Up and Down move the focus, Alt with Up or Down moves the focused row one place, and Alt with \
           Home or End to the first or the last.",
          self.frame.demo(
            0,
            board,
            "SortableList::new(rows).on_reorder(|from, to| ..).mount(ctx)",
            &[
              "// sections: Signal<Vec<&str>>",
              "let order = sections.clone();",
              "SortableList::new(sections.get().into_iter().map(|name| (name, name)))",
              "    .label(\"Sections\")",
              "    .on_reorder(move |from, to| {",
              "        order.update(|rows| {",
              "            let row = rows.remove(from);",
              "            rows.insert(to, row);",
              "        })",
              "    })",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A selection dragged together",
          "With `selected` a press selects a row, with Ctrl one in or out and with Shift a range, Space adds the \
           focused row or takes it out and Ctrl+A selects them all. A drag of a selected row, or Alt with a key on \
           it, moves every selected row: the row under the pointer counts them on its corner, each leaves a ghost, \
           and the app is told by `on_reorder_many`.",
          self.frame.demo(
            1,
            many,
            ".selected(selected).on_reorder_many(|rows, to| ..)",
            &[
              "// sections, selected: Signal<Vec<&str>>",
              "SortableList::new(rows)",
              "    .selected(selected)",
              "    .on_reorder(|from, to| ..)",
              "    .on_reorder_many(move |rows, to| sections.update(|order| move_rows(order, rows, to)))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Dragged by the handle",
          "Rows that hold what is pressed itself, a switch at their end, are dragged by their grip alone: a press \
           anywhere else on the row is the row's own.",
          self.frame.demo(
            2,
            switches,
            ".handle_only(true)",
            &[
              "SortableList::new(settings.into_iter().map(|name| {",
              "    SortableItem::new(name, name).trailing(Switch::new(name).label_hidden(true))",
              "}))",
              ".handle_only(true)",
              ".on_reorder(|from, to| ..)",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws a row at rest, the row under the pointer of a drag, the ghost and the line. Under the \
           pointer and held down a row is veiled, the focus from the keyboard is a ring inside it, a selected row \
           is in the highlight, and a disabled one is dimmed and stays where it is.",
          self.frame.demo(
            3,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "SortableList::new([",
              "    SortableItem::new(\"Buttons\", \"Buttons\"),",
              "    SortableItem::new(\"Inputs\", \"Inputs\").icon(icons::FILE),",
              "])",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              ".force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a sortable list is given.",
          props_table(&rows, &["items"], self.frame.room()),
        ),
      ],
    )
  }
}
