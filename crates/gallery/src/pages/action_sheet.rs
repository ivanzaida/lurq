//! The page of the action sheet: its playground, the sheet as the board draws it, and one over the window.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{ActionSheet, Button, ButtonVariant, MenuItem},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const COUNTS: [(&str, usize); 3] = [("2", 2), ("3", 3), ("5", 5)];
const NAMES: [&str; 5] = ["Save to library", "Duplicate", "Copy link", "Share…", "Move to album"];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  // No wider than the stage: what is shown gets narrower with it, and what is said under it wraps.
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).padding(2.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// The options of a sheet: `count` of them, the last one that destroys something where `destructive` says so.
fn options(count: usize, destructive: bool, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Vec<MenuItem> {
  let mut items: Vec<MenuItem> = NAMES
    .iter()
    .take(count)
    .map(|name| {
      let item = MenuItem::new(name);
      match &record {
        Some(record) => {
          let (record, name) = (record.clone(), name.to_string());
          item.on_select(move || record(format!("on_select({name})")))
        }
        None => item,
      }
    })
    .collect();
  if destructive {
    items.push(MenuItem::new("Delete photo").destructive(true));
  }
  items
}

pub struct ActionSheetPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  count: Signal<usize>,
  destructive: Signal<bool>,
  open: Signal<bool>,
}

impl ActionSheetPage {
  /// The playground: a sheet as the panel's props make it, drawn in place.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, count, destructive) = (self.title.get(), self.count.get(), self.destructive.get());
    let record = self.playground.recorder();
    let plain = {
      let title = title.clone();
      move |record: Option<Arc<dyn Fn(String) + Send + Sync>>, playground: Option<&Playground>| {
        let mut sheet = ActionSheet::new(Signal::new(true), options(count, destructive, record)).inline(true);
        if !title.is_empty() {
          sheet = sheet.title(&title);
        }
        if let Some(playground) = playground {
          sheet = playground.styled(sheet);
        }
        Column::new().width(360.0).max_width(FILL).child(sheet)
      }
    };
    let live: Element = plain(Some(Arc::new(move |line: String| record(line))), Some(&self.playground)).into();
    let still = move || -> Element { plain(None, None).into() };

    let mut code = vec!["ActionSheet::new(open, [".to_owned()];
    for name in NAMES.iter().take(count) {
      code.push(format!("    MenuItem::new({name:?}).on_select(..),"));
    }
    if destructive {
      code.push("    MenuItem::new(\"Delete photo\").destructive(true),".to_owned());
    }
    code.push("])".to_owned());
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    let tree = vec![
      "root".to_owned(),
      "├─ options    the title, and an option of 56 for each".to_owned(),
      "└─ cancel     a card of its own".to_owned(),
    ];

    let (to_title, to_count, to_destructive) = (self.title.clone(), self.count.clone(), self.destructive.clone());
    let pick_count = self.count.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "ActionSheet",
        make: Arc::new(still),
        shown: Shown::of(live),
        fill: false,
        forceable: false,
        facts: format!("{} options", count + usize::from(destructive)),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          divider(),
          prop(
            "items",
            "how many",
            self
              .playground
              .select(0, &COUNTS, &count, move |which| pick_count.set(which)),
          ),
          flag("destructive", &self.destructive),
        ],
        changed: [title != "Photo options", count != 2, !destructive]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_title.set("Photo options".to_owned());
          to_count.set(2);
          to_destructive.set(true);
        }),
      },
    )
  }
}

impl Component for ActionSheetPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("Photo options".to_owned()),
      count: ctx.signal(2),
      destructive: ctx.signal(true),
      open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Row::new().width(FILL).id("action-sheet-board").child(item(
      Column::new().width(344.0).max_width(FILL).child(
        ActionSheet::new(Signal::new(true), options(2, true, None))
          .title("Photo options")
          .inline(true),
      ),
      "a title, three options, the last one destroys · Cancel in a card of its own",
    ));

    let open = self.open.clone();
    let over = Row::new()
      .width(FILL)
      .id("action-sheet-over")
      .child(
        Button::new("Show the actions")
          .variant(ButtonVariant::Outline)
          .id("action-sheet-open")
          .on_click(move |_| open.set(true)),
      )
      .child(
        ActionSheet::new(self.open.clone(), options(2, true, None))
          .title("Photo options")
          .id("action-sheet-window"),
      );

    let rows = [
      (
        "new(open, items) / item",
        "Signal<bool>, MenuItem…",
        "",
        "Open while the signal says so, with its options: what an item says, whether it destroys something or is \
         disabled, and what choosing it does. An icon, a shortcut and a submenu are not shown.",
      ),
      ("title", "&str", "", "What the sheet is about, over its options."),
      ("cancel_label", "&str", "Cancel", "What Cancel says."),
      (
        "on_dismiss",
        "Fn()",
        "",
        "Told when it is closed with no option: Cancel, the scrim, Escape.",
      ),
      (
        "inline",
        "bool",
        "false",
        "The sheet where it is declared, without the scrim: for a gallery, a screenshot, a test.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Action Sheet"],
        title: "Action Sheet",
        description: "On a phone, the actions on something, risen from the bottom of the screen over a scrim, and \
                      Cancel under them. A long press of a picture brings its own.",
        import: Some("use lurq::ui::ActionSheet;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ActionSheet is a control on the right. The sheet is drawn in place: over the window it rises \
           from the bottom on a scrim.",
          playground,
        ),
        Section::new(
          "As the board draws it",
          "A title in the muted role, options of 56 with a line over each, the one that destroys in the \
           destructive role, and Cancel 8 under them.",
          self.frame.demo(
            0,
            board,
            "ActionSheet::new(open, items)",
            &[
              "ActionSheet::new(open, [",
              "    MenuItem::new(\"Save to library\").on_select(save),",
              "    MenuItem::new(\"Duplicate\").on_select(duplicate),",
              "    MenuItem::new(\"Delete photo\").destructive(true).on_select(delete),",
              "])",
              ".title(\"Photo options\")",
            ],
          ),
        ),
        Section::new(
          "Over the window",
          "Open while its signal says so. An option does what it does and closes it; Cancel, a press on the scrim \
           and Escape close it.",
          self.frame.demo(
            1,
            over,
            "ActionSheet::new(open, items)",
            &[
              "let open = ctx.signal(false);",
              "Button::new(\"Show the actions\").on_click(move |_| open.set(true));",
              "ActionSheet::new(open, items).title(\"Photo options\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an action sheet is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
