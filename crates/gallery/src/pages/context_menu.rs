//! The page of the context menu: its playground, the files of a list each with its menu, the edit bar over a field,
//! and the overflow menu of a button.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Card, ContextMenu, Input, Menu, MenuItem, MenuPart, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Spec, divider, flag},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// The files of the list, each with its own menu.
const FILES: [&str; 3] = ["report.pdf", "budget.xlsx", "notes.md"];

/// What a choice does on this page: the signal says which it was.
fn says(did: &Signal<String>, name: &str) -> impl Fn() + Send + Sync + use<> {
  let (did, name) = (did.clone(), name.to_owned());
  move || did.set(name.clone())
}

fn did(signal: &Signal<String>) -> String {
  match signal.get() {
    name if name.is_empty() => "nothing chosen yet".to_owned(),
    name => format!("chosen: {name}"),
  }
}

pub struct ContextMenuPage {
  frame: Frame,
  playground: Playground,
  bar: Signal<bool>,
  icons: Signal<bool>,
  shortcuts: Signal<bool>,
  submenu: Signal<bool>,
  disabled: Signal<bool>,
  play_open: Signal<bool>,
  play_sub_anchor: ElementRef,
  play_sub_open: Signal<bool>,
  // The files.
  files_open: [Signal<bool>; 3],
  files_did: Signal<String>,
  // The edit bar.
  text: Signal<String>,
  edit_open: Signal<bool>,
  edit_did: Signal<String>,
  // The overflow menu.
  more_anchor: ElementRef,
  more_open: Signal<bool>,
  more_did: Signal<String>,
}

impl ContextMenuPage {
  /// The playground: the board's context menu on a card, as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (bar, with_icons, shortcuts, submenu, disabled) = (
      self.bar.get(),
      self.icons.get(),
      self.shortcuts.get(),
      self.submenu.get(),
      self.disabled.get(),
    );
    let record = self.playground.recorder();
    let told = move |name: &str| -> Box<dyn Fn() + Send + Sync> {
      let (record, name) = (record.clone(), name.to_owned());
      Box::new(move || record(format!("on_select({name:?})")))
    };
    let menu = move |open: Signal<bool>,
                     tools: Option<(ElementRef, Signal<bool>)>,
                     told: &dyn Fn(&str) -> Box<dyn Fn() + Send + Sync>| {
      let target = Card::new()
        .title("report.pdf")
        .description("Right-click it, hold it with a finger, or focus it and press Shift+F10.")
        .width(300.0)
        .on_click(|_| {});
      let command = |label: &str, icon: &'static str, keys: &str| {
        let mut command = MenuItem::new(label).on_select(told(label));
        if with_icons {
          command = command.icon(icon);
        }
        if shortcuts && !keys.is_empty() {
          command = command.shortcut(keys);
        }
        command
      };
      let mut menu = ContextMenu::new(target, open)
        .bar(bar)
        .disabled(disabled)
        .item(command("Back", icons::ARROW_LEFT, "Alt+←"))
        .item(command("Reload", icons::REFRESH, "Ctrl+R"))
        .separator()
        .item(MenuItem::new("Show grid").checked(true).on_select(told("Show grid")));
      if let Some((anchor, open)) = tools {
        menu = menu.item(
          MenuItem::new("More tools").submenu(
            Menu::new(anchor, open)
              .item(MenuItem::new("Task manager").on_select(told("Task manager")))
              .item(MenuItem::new("Developer tools").on_select(told("Developer tools"))),
          ),
        );
      }
      menu.item(command("Inspect", icons::CODE, "F12")).separator().item(
        command("Delete", icons::TRASH, "Del")
          .destructive(true)
          .on_select(told("Delete")),
      )
    };
    let shown: Element = menu(
      self.play_open.clone(),
      submenu.then(|| (self.play_sub_anchor.clone(), self.play_sub_open.clone())),
      &told,
    )
    .into();
    let make = {
      let menu = menu.clone();
      move || -> Element {
        let quiet = |_: &str| -> Box<dyn Fn() + Send + Sync> { Box::new(|| {}) };
        let tools = submenu.then(|| (ElementRef::new(), Signal::new(false)));
        menu(Signal::new(false), tools, &quiet).into()
      }
    };

    let item_code = |label: &str, icon: &str, keys: &str| {
      let mut line = format!("    .item(MenuItem::new({label:?})");
      if with_icons {
        line.push_str(&format!(".icon(icons::{icon})"));
      }
      if shortcuts {
        line.push_str(&format!(".shortcut({keys:?})"));
      }
      line.push_str(".on_select(..))");
      line
    };
    let mut code = vec!["// open: Signal<bool>; the menu sets it when it is asked for".to_owned()];
    code.push("ContextMenu::new(card, open)".to_owned());
    if bar {
      code.push("    .bar(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push(item_code("Back", "ARROW_LEFT", "Alt+←"));
    code.push(item_code("Reload", "REFRESH", "Ctrl+R"));
    code.push("    .separator()".to_owned());
    code.push("    .item(MenuItem::new(\"Show grid\").checked(true))".to_owned());
    if submenu {
      code.push("    .item(MenuItem::new(\"More tools\").submenu(Menu::new(tools, tools_open).items(..)))".to_owned());
    }
    code.push(item_code("Inspect", "CODE", "F12"));
    code.push("    .separator()".to_owned());
    code.push(item_code("Delete", "TRASH", "Del").replace(".on_select", ".destructive(true).on_select"));

    let tree = if bar {
      vec![
        "card         the element, as it is".to_owned(),
        "└─ bar        padded 3, radius lg, at the pointer".to_owned(),
        "   └─ items   28 tall, padded 10, labels 13".to_owned(),
      ]
    } else {
      vec![
        "card         the element, as it is".to_owned(),
        "└─ menu       at least 220 wide, at the pointer".to_owned(),
        "   ├─ items   32 tall  icon 16 · label · shortcut".to_owned(),
        "   ├─ separator".to_owned(),
        "   ├─ item    \"Show grid\"  check 16".to_owned(),
        "   └─ item    \"Delete\"  destructive".to_owned(),
      ]
    };

    let flags = [
      self.bar.clone(),
      self.icons.clone(),
      self.shortcuts.clone(),
      self.submenu.clone(),
      self.disabled.clone(),
    ];
    let mut facts = vec![if bar { "bar" } else { "column" }.to_owned()];
    if disabled {
      facts.push("disabled".to_owned());
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Context Menu",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: facts.join(" · "),
        event: "on_select",
        code,
        tree,
        controls: vec![
          flag("bar", &self.bar),
          divider(),
          flag("icons", &self.icons),
          flag("shortcuts", &self.shortcuts),
          flag("submenu", &self.submenu),
          flag("disabled", &self.disabled),
        ],
        changed: [bar, !with_icons, !shortcuts, submenu, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          for (index, flag) in flags.iter().enumerate() {
            // The icons and the shortcuts are on to start with; the rest are off.
            flag.set(index == 1 || index == 2);
          }
        }),
      },
    )
  }
}

impl Component for ContextMenuPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      bar: ctx.signal(false),
      icons: ctx.signal(true),
      shortcuts: ctx.signal(true),
      submenu: ctx.signal(false),
      disabled: ctx.signal(false),
      play_open: ctx.signal(false),
      play_sub_anchor: ElementRef::new(),
      play_sub_open: ctx.signal(false),
      files_open: [ctx.signal(false), ctx.signal(false), ctx.signal(false)],
      files_did: ctx.signal(String::new()),
      text: ctx.signal("Select some of this text".to_owned()),
      edit_open: ctx.signal(false),
      edit_did: ctx.signal(String::new()),
      more_anchor: ElementRef::new(),
      more_open: ctx.signal(false),
      more_did: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // A list of files, each with a menu of its own: one is open at a time.
    let mut list = Column::new().width(280.0).spacing(2.0).align_items(Alignment::Stretch);
    for (file, open) in FILES.iter().zip(&self.files_open) {
      let did = |what: &str| says(&self.files_did, &format!("{what} {file}"));
      list = list.child(
        ContextMenu::new(
          Button::new(file)
            .variant(ButtonVariant::Ghost)
            .icon(icons::FILE)
            .full_width(true),
          open.clone(),
        )
        .item(MenuItem::new("Open").icon(icons::FOLDER_OPEN).on_select(did("Open")))
        .item(
          MenuItem::new("Rename")
            .icon(icons::PENCIL)
            .shortcut("F2")
            .on_select(did("Rename")),
        )
        .item(
          MenuItem::new("Duplicate")
            .icon(icons::DUPLICATE)
            .shortcut("Ctrl+D")
            .on_select(did("Duplicate")),
        )
        .separator()
        .item(
          MenuItem::new("Delete")
            .icon(icons::TRASH)
            .shortcut("Del")
            .destructive(true)
            .on_select(did("Delete")),
        ),
      );
    }
    let files = item(list, &did(&self.files_did));

    // The board's edit menu: a bar over a field.
    let edit = |name: &str| says(&self.edit_did, name);
    let field = item(
      ContextMenu::new(Input::new(self.text.clone()).full_width(true), self.edit_open.clone())
        .bar(true)
        .items(["Cut", "Copy", "Paste", "Select All"].map(|name| MenuItem::new(name).on_select(edit(name)))),
      &did(&self.edit_did),
    );

    // The board's overflow menu: a menu under a button of three dots.
    let more = |name: &str| says(&self.more_did, name);
    let toggle = self.more_open.clone();
    let overflow = item(
      Column::new()
        .child(
          Row::new().ref_element(self.more_anchor.clone()).child(
            Button::new("More")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::MORE_VERTICAL)
              .icon_only(true)
              .on_click(move |_| toggle.update(|open| *open = !*open)),
          ),
        )
        .child(
          Menu::new(self.more_anchor.clone(), self.more_open.clone())
            .style(MenuPart::Panel, |s| s.min_width(180.0))
            .item(MenuItem::new("Rename").on_select(more("Rename")))
            .item(MenuItem::new("Duplicate").on_select(more("Duplicate")))
            .item(MenuItem::new("Delete").destructive(true).on_select(more("Delete"))),
        ),
      &did(&self.more_did),
    );

    let rows = [
      (
        "ContextMenu::new(target, open)",
        "impl Into<Element>, Signal<bool>",
        "",
        "The element the menu is for, which lays out as without it, and whether the menu is open: the app's. \
         The presses and keys that ask for it set it; choosing, a press outside and Escape clear it.",
      ),
      (
        "item",
        "MenuItem",
        "",
        "A command, as a menu has it: icon, shortcut, check, submenu. items takes several.",
      ),
      (
        "separator",
        "",
        "",
        "A line that ends one group of items and starts the next.",
      ),
      ("label", "&str", "", "A text over the items after it."),
      (
        "bar",
        "bool",
        "false",
        "The commands in one row, as the board's edit menu over selected text has them.",
      ),
      ("disabled", "bool", "false", "The element has no menu now."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Context Menu"],
        title: "Context Menu",
        description: "A menu of what can be done with an element, opened where the element is pressed with the \
                      secondary button or held with a finger: a file in a list, a node of a canvas, selected text.",
        import: Some("use lurq::ui::{ContextMenu, MenuItem};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The props are controls on the right. Right-click the card, hold it with a finger, or Tab to it and press \
           the Menu key or Shift+F10: from the keyboard the menu opens under the card. It stays inside the window, \
           opening up or back from the pointer where there is no room.",
          playground,
        ),
        Section::new(
          "A list of files",
          "Each file has a menu of its own, open while its own signal is. One context menu is open at a time: asking \
           for another closes the first. The focus goes to the first item as it opens and back to the file as it \
           closes.",
          self.frame.demo(
            0,
            files,
            "ContextMenu::new(file, open).item(MenuItem::new(\"Rename\"))",
            &[
              "// open: Signal<bool>, one for each file",
              "ContextMenu::new(",
              "    Button::new(\"report.pdf\").variant(ButtonVariant::Ghost).icon(icons::FILE).full_width(true),",
              "    open,",
              ")",
              ".item(MenuItem::new(\"Open\").icon(icons::FOLDER_OPEN).on_select(..))",
              ".item(MenuItem::new(\"Rename\").icon(icons::PENCIL).shortcut(\"F2\").on_select(..))",
              ".item(MenuItem::new(\"Duplicate\").icon(icons::DUPLICATE).shortcut(\"Ctrl+D\").on_select(..))",
              ".separator()",
              ".item(MenuItem::new(\"Delete\").icon(icons::TRASH).shortcut(\"Del\").destructive(true).on_select(..))",
            ],
          ),
        ),
        Section::new(
          "Edit bar",
          "The board's edit menu: the commands of selected text in one row, at the pointer. Left and Right move \
           between them.",
          self.frame.demo(
            1,
            field,
            ".bar(true)",
            &[
              "// text: Signal<String>, open: Signal<bool>",
              "ContextMenu::new(Input::new(text).full_width(true), open)",
              "    .bar(true)",
              "    .items([\"Cut\", \"Copy\", \"Paste\", \"Select All\"].map(MenuItem::new))",
            ],
          ),
        ),
        Section::new(
          "Overflow menu",
          "The board's overflow menu is a menu under a button of three dots: a Menu, at least 180 wide, hung from a \
           ghost button that is its icon alone.",
          self.frame.demo(
            2,
            overflow,
            "Menu::new(anchor, open).style(MenuPart::Panel, |s| s.min_width(180.0))",
            &[
              "// anchor: ElementRef, open: Signal<bool>",
              "Row::new().ref_element(anchor.clone()).child(",
              "    Button::new(\"More\")",
              "        .variant(ButtonVariant::Ghost)",
              "        .size(ButtonSize::Sm)",
              "        .icon(icons::MORE_VERTICAL)",
              "        .icon_only(true)",
              "        .on_click(move |_| toggle.update(|open| *open = !*open)),",
              ")",
              "",
              "Menu::new(anchor, open)",
              "    .style(MenuPart::Panel, |s| s.min_width(180.0))",
              "    .item(MenuItem::new(\"Rename\").on_select(..))",
              "    .item(MenuItem::new(\"Duplicate\").on_select(..))",
              "    .item(MenuItem::new(\"Delete\").destructive(true).on_select(..))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a context menu is given; its items are a menu's.",
          props_table(&rows, &["ContextMenu::new(target, open)"], self.frame.room()),
        ),
      ],
    )
  }
}
