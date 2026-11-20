//! The page of the menu: its playground, a dropdown from a button, the board's
//! menu with checked, disabled and grouped items, the split button's menu, and
//! items with a detail, a reason and an avatar, and a menu of the app's own rows.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Placement, Rect, Row, Text},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonGroup, ButtonGroupVariant, ButtonVariant, Menu, MenuItem, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// Where the playground's menu opens beside its button.
const PLACEMENTS: [(&str, Placement); 4] = [
  ("BottomStart", Placement::BottomStart),
  ("BottomEnd", Placement::BottomEnd),
  ("TopStart", Placement::TopStart),
  ("RightStart", Placement::RightStart),
];

/// How a split button merges, as its menu offers.
const STRATEGIES: [&str; 3] = ["Create a merge commit", "Squash and merge", "Rebase and merge"];

pub struct MenuPage {
  frame: Frame,
  playground: Playground,
  // The playground: the item its panel shapes, and the menu around it.
  label: Signal<String>,
  shortcut: Signal<String>,
  icon: Signal<bool>,
  checked: Signal<bool>,
  radio: Signal<bool>,
  submenu: Signal<bool>,
  destructive: Signal<bool>,
  disabled: Signal<bool>,
  inline: Signal<bool>,
  placement: Signal<usize>,
  play_anchor: ElementRef,
  play_open: Signal<bool>,
  play_sub_anchor: ElementRef,
  play_sub_open: Signal<bool>,
  // The dropdown under a button.
  account_anchor: ElementRef,
  account_open: Signal<bool>,
  account_chosen: Signal<String>,
  // The board's menu.
  grid: Signal<bool>,
  snap: Signal<bool>,
  zoom_chosen: Signal<String>,
  zoom: Signal<usize>,
  export_anchor: ElementRef,
  export_open: Signal<bool>,
  // The split button.
  merge_anchor: ElementRef,
  merge_open: Signal<bool>,
  strategy: Signal<usize>,
  merged: Signal<String>,
}

impl MenuPage {
  /// The playground: a menu whose second item is as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (label, shortcut) = (self.label.get(), self.shortcut.get());
    let (icon, checked, destructive, disabled) = (
      self.icon.get(),
      self.checked.get(),
      self.destructive.get(),
      self.disabled.get(),
    );
    let (radio, submenu) = (self.radio.get(), self.submenu.get());
    let (inline, placement) = (self.inline.get(), self.placement.get().min(PLACEMENTS.len() - 1));
    let record = self.playground.recorder();
    let told = |name: &str| {
      let (record, name) = (record.clone(), name.to_owned());
      move || record(format!("on_select({name:?})"))
    };
    // The formats a submenu of the shaped item offers, each telling `told` when it is chosen.
    let formats = |anchor: ElementRef, open: Signal<bool>, told: &dyn Fn(&str) -> Box<dyn Fn() + Send + Sync>| {
      ["PNG", "SVG", "PDF"]
        .into_iter()
        .fold(Menu::new(open, anchor), |menu, format| {
          menu.item(MenuItem::new(format).on_select(told(format)))
        })
    };
    let shaped = {
      let (label, shortcut) = (label.clone(), shortcut.clone());
      move |formats: Option<Menu>| {
        let mut shaped = MenuItem::new(&label).destructive(destructive).disabled(disabled);
        shaped = if radio {
          shaped.radio(checked)
        } else {
          shaped.checked(checked)
        };
        if icon {
          shaped = shaped.icon(icons::PENCIL);
        }
        if !shortcut.is_empty() {
          shaped = shaped.shortcut(&shortcut);
        }
        match formats {
          Some(formats) => shaped.submenu(formats),
          None => shaped,
        }
      }
    };
    let menu = move |anchor: ElementRef, open: Signal<bool>, shaped: MenuItem, undo: MenuItem, delete: MenuItem| {
      Menu::new(open, anchor)
        .placement(PLACEMENTS[placement].1)
        .inline(inline)
        .heading("Edit")
        .item(undo)
        .item(shaped)
        .separator()
        .item(delete)
    };
    let undo = || MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z");
    let delete = || MenuItem::new("Delete").icon(icons::TRASH).destructive(true);
    let shown = self.playground.styled(menu(
      self.play_anchor.clone(),
      self.play_open.clone(),
      shaped(submenu.then(|| {
        formats(
          self.play_sub_anchor.clone(),
          self.play_sub_open.clone(),
          &|format: &str| -> Box<dyn Fn() + Send + Sync> { Box::new(told(format)) },
        )
      }))
      .force(self.playground.forced())
      .on_select(told(&label)),
      undo().on_select(told("Undo")),
      delete().on_select(told("Delete")),
    ));
    let shown: Element = if inline {
      shown.into()
    } else {
      let toggle = self.play_open.clone();
      Column::new()
        .child(
          Row::new().ref_element(self.play_anchor.clone()).child(
            Button::new("Edit")
              .variant(ButtonVariant::Outline)
              .trailing_icon(icons::CHEVRON_DOWN)
              .on_click(move |_| toggle.update(|open| *open = !*open)),
          ),
        )
        .child(shown)
        .into()
    };
    let make = {
      let shaped = shaped.clone();
      move || -> Element {
        let quiet = |_: &str| -> Box<dyn Fn() + Send + Sync> { Box::new(|| {}) };
        let sub = submenu.then(|| formats(ElementRef::new(), Signal::new(false), &quiet));
        menu(ElementRef::new(), Signal::new(true), shaped(sub), undo(), delete())
          .inline(true)
          .into()
      }
    };

    let mut item_code = vec![format!("        MenuItem::new({label:?})")];
    if icon {
      item_code.push("            .icon(icons::PENCIL)".to_owned());
    }
    if !shortcut.is_empty() {
      item_code.push(format!("            .shortcut({shortcut:?})"));
    }
    if radio {
      item_code.push(format!("            .radio({checked})"));
    }
    for (on, call) in [
      (checked && !radio, ".checked(true)"),
      (destructive, ".destructive(true)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        item_code.push(format!("            {call}"));
      }
    }
    if submenu {
      item_code.push("            .submenu(".to_owned());
      item_code.push("                Menu::new(formats_open, formats_anchor)".to_owned());
      item_code.push("                    .items([\"PNG\", \"SVG\", \"PDF\"].map(MenuItem::new)),".to_owned());
      item_code.push("            ),".to_owned());
    } else {
      item_code.push("            .on_select(|| rename()),".to_owned());
    }
    let mut code = vec!["// open: Signal<bool>, set by a press of the button the menu hangs from".to_owned()];
    code.push("Menu::new(open, anchor)".to_owned());
    if placement != 0 {
      code.push(format!("    .placement(Placement::{})", PLACEMENTS[placement].0));
    }
    if inline {
      code.push("    .inline(true)".to_owned());
    }
    code.push("    .heading(\"Edit\")".to_owned());
    code.push("    .item(MenuItem::new(\"Undo\").icon(icons::UNDO).shortcut(\"Ctrl+Z\"))".to_owned());
    code.push("    .item(".to_owned());
    code.extend(item_code);
    code.push("    )".to_owned());
    code.push("    .separator()".to_owned());
    code.push("    .item(MenuItem::new(\"Delete\").icon(icons::TRASH).destructive(true))".to_owned());

    let mut parts = Vec::new();
    if icon {
      parts.push("icon 16");
    }
    parts.push("label");
    if !shortcut.is_empty() {
      parts.push("shortcut");
    }
    if checked {
      parts.push(if radio { "dot 16" } else { "check 16" });
    }
    if submenu {
      parts.push("chevron 16");
    }
    let tree = vec![
      format!("panel        {}", self.playground.measured()),
      "├─ label      \"Edit\"   11/600".to_owned(),
      "├─ item       32 tall  \"Undo\"".to_owned(),
      format!("├─ item       32 tall  {}", parts.join(" · ")),
      "├─ separator  1 + 4 above and below".to_owned(),
      "└─ item       32 tall  \"Delete\"  destructive".to_owned(),
    ];

    let (to_label, to_shortcut, to_placement, to_inline) = (
      self.label.clone(),
      self.shortcut.clone(),
      self.placement.clone(),
      self.inline.clone(),
    );
    let pick = self.placement.clone();
    let flags = [
      self.icon.clone(),
      self.checked.clone(),
      self.radio.clone(),
      self.submenu.clone(),
      self.destructive.clone(),
      self.disabled.clone(),
    ];
    let mut facts = vec![PLACEMENTS[placement].0.to_owned()];
    if inline {
      facts.push("inline".to_owned());
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Menu",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: facts.join(" · "),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop("shortcut", "&str", field(&self.shortcut)),
          prop(
            "placement",
            "Placement",
            self.playground.select(
              0,
              &PLACEMENTS.map(|(name, _)| (name, name)),
              &PLACEMENTS[placement].0,
              move |name| pick.set(PLACEMENTS.iter().position(|(held, _)| *held == name).unwrap_or(0)),
            ),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("checked", &self.checked),
          flag("radio", &self.radio),
          flag("submenu", &self.submenu),
          flag("destructive", &self.destructive),
          flag("disabled", &self.disabled),
          flag("inline", &self.inline),
        ],
        changed: [
          label != "Rename",
          shortcut != "F2",
          placement != 0,
          !icon,
          checked,
          radio,
          submenu,
          destructive,
          disabled,
          !inline,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_label.set("Rename".to_owned());
          to_shortcut.set("F2".to_owned());
          to_placement.set(0);
          to_inline.set(true);
          for (index, flag) in flags.iter().enumerate() {
            // The icon is on to start with; the rest are off.
            flag.set(index == 0);
          }
        }),
      },
    )
  }

  /// A button that opens and closes `open`, with the menu hung from it: the button is inside the anchor.
  fn trigger(anchor: &ElementRef, open: &Signal<bool>, button: Button) -> Row {
    let toggle = open.clone();
    Row::new()
      .ref_element(anchor.clone())
      .child(button.on_click(move |_| toggle.update(|open| *open = !*open)))
  }

  /// What a choice does on this page: the signal says which it was.
  fn says(chosen: &Signal<String>, name: &str) -> impl Fn() + Send + Sync + use<> {
    let (chosen, name) = (chosen.clone(), name.to_owned());
    move || chosen.set(name.clone())
  }
}

impl Component for MenuPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      label: ctx.signal("Rename".to_owned()),
      shortcut: ctx.signal("F2".to_owned()),
      icon: ctx.signal(true),
      checked: ctx.signal(false),
      radio: ctx.signal(false),
      submenu: ctx.signal(false),
      destructive: ctx.signal(false),
      disabled: ctx.signal(false),
      inline: ctx.signal(true),
      placement: ctx.signal(0),
      play_anchor: ElementRef::new(),
      play_open: ctx.signal(false),
      play_sub_anchor: ElementRef::new(),
      play_sub_open: ctx.signal(false),
      account_anchor: ElementRef::new(),
      account_open: ctx.signal(false),
      account_chosen: ctx.signal(String::new()),
      grid: ctx.signal(true),
      snap: ctx.signal(false),
      zoom_chosen: ctx.signal(String::new()),
      zoom: ctx.signal(1),
      export_anchor: ElementRef::new(),
      export_open: ctx.signal(false),
      merge_anchor: ElementRef::new(),
      merge_open: ctx.signal(false),
      strategy: ctx.signal(0),
      merged: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let chosen = |signal: &Signal<String>| match signal.get() {
      name if name.is_empty() => "nothing chosen yet".to_owned(),
      name => format!("chosen: {name}"),
    };

    // The board's dropdown, under a button.
    let says = |name: &str| Self::says(&self.account_chosen, name);
    let account = Menu::new(self.account_open.clone(), self.account_anchor.clone())
      .heading("Workspace")
      .item(
        MenuItem::new("Profile")
          .icon(icons::USER)
          .shortcut("Ctrl+P")
          .on_select(says("Profile")),
      )
      .item(
        MenuItem::new("Settings")
          .icon(icons::SETTINGS)
          .shortcut("Ctrl+,")
          .on_select(says("Settings")),
      )
      .item(
        MenuItem::new("Keyboard shortcuts")
          .icon(icons::KEYBOARD)
          .on_select(says("Keyboard shortcuts")),
      )
      .separator()
      .item(
        MenuItem::new("Log out")
          .icon(icons::SIGN_OUT)
          .destructive(true)
          .on_select(says("Log out")),
      );
    let dropdown = items().child(item(
      Column::new()
        .child(Self::trigger(
          &self.account_anchor,
          &self.account_open,
          Button::new("Account")
            .variant(ButtonVariant::Outline)
            .icon(icons::USER)
            .trailing_icon(icons::CHEVRON_DOWN),
        ))
        .child(account),
      &chosen(&self.account_chosen),
    ));

    // The board's menu, drawn in place: checked, disabled, grouped, radios, and a submenu.
    let (grid, snap) = (self.grid.clone(), self.snap.clone());
    let (show_grid, snap_on) = (grid.get(), snap.get());
    let zoom = |name: &str| Self::says(&self.zoom_chosen, name);
    let board = Menu::controlled(true, ElementRef::new(), |_| {})
      .inline(true)
      .item(MenuItem::new("Undo").shortcut("Ctrl+Z").on_select(zoom("Undo")))
      .item(MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true))
      .separator()
      .item(
        MenuItem::new("Show grid")
          .checked(show_grid)
          .on_select(move || grid.set(!show_grid)),
      )
      .item(
        MenuItem::new("Snap to guides")
          .checked(snap_on)
          .on_select(move || snap.set(!snap_on)),
      )
      .separator()
      .heading("Zoom")
      .items(["Fit to window", "100%"].into_iter().enumerate().map(|(index, name)| {
        let pick = self.zoom.clone();
        MenuItem::new(name)
          .radio(self.zoom.get() == index)
          .on_select(move || pick.set(index))
      }))
      .separator()
      .item(
        MenuItem::new("Export as").submenu(
          Menu::new(self.export_open.clone(), self.export_anchor.clone())
            .items(["PNG", "SVG", "PDF"].map(|format| MenuItem::new(format).on_select(zoom(format)))),
        ),
      );
    let states = items().child(item(board, &chosen(&self.zoom_chosen)));

    // The split button: the action, and beside it the menu of how to do it.
    let strategy = self.strategy.get().min(STRATEGIES.len() - 1);
    let mut how = Menu::new(self.merge_open.clone(), self.merge_anchor.clone()).placement(Placement::BottomEnd);
    for (index, name) in STRATEGIES.iter().enumerate() {
      let pick = self.strategy.clone();
      how = how.item(
        MenuItem::new(name)
          .checked(index == strategy)
          .on_select(move || pick.set(index)),
      );
    }
    let (toggle, merged, done) = (self.merge_open.clone(), self.merged.clone(), STRATEGIES[strategy]);
    let split = Column::new()
      .child(
        Row::new().ref_element(self.merge_anchor.clone()).child(
          ButtonGroup::new([])
            .variant(ButtonGroupVariant::Primary)
            .button(Button::new("Merge").on_click(move |_| merged.set(done.to_owned())))
            .button(
              Button::new("How to merge")
                .icon(icons::CHEVRON_DOWN)
                .icon_only(true)
                .on_click(move |_| toggle.update(|open| *open = !*open)),
            ),
        ),
      )
      .child(how);
    let split = items().child(item(
      split,
      &match self.merged.get() {
        merged if merged.is_empty() => format!("will: {}", STRATEGIES[strategy]),
        merged => format!("did: {merged}"),
      },
    ));

    // Items that say more than their label: a detail, why one cannot be chosen, an element before the label.
    let agents = Menu::controlled(true, ElementRef::new(), |_| {})
      .inline(true)
      .heading("Run with")
      .item(
        MenuItem::new("Builder")
          .leading(
            Rect::new(20.0, 20.0)
              .rounded(10.0)
              .background(lurq::app::theme::PaletteColor::Primary),
          )
          .detail("gpt-5 · high effort")
          .checked(true),
      )
      .item(
        MenuItem::new("Reviewer")
          .leading(
            Rect::new(20.0, 20.0)
              .rounded(10.0)
              .background(lurq::app::theme::PaletteColor::Secondary),
          )
          .detail("claude-opus · medium effort"),
      )
      .separator()
      .item(
        MenuItem::new("Publish the release")
          .icon(icons::UPLOAD)
          .unavailable("The run has not finished"),
      );
    let described = items().child(item(agents, "a detail, a reason, an avatar"));

    // Rows the app draws itself, a detail on the label's line, that are still the menu's items; and a foot of its own.
    let row = |label: &str, detail: &str| {
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(Text::new(label))
        .child(quiet(detail, 12.0))
    };
    let models = Menu::controlled(true, ElementRef::new(), |_| {})
      .inline(true)
      .item(MenuItem::new("Opus").content(row("Opus", "the deepest")).checked(true))
      .item(
        MenuItem::new("Sonnet")
          .content(row("Sonnet", "balanced"))
          .unavailable("Not on this plan"),
      )
      .item(MenuItem::new("Haiku").content(row("Haiku", "the fastest")))
      .separator()
      .child(
        Row::new()
          .padding(8.0)
          .child(quiet("Sonnet is not on this plan.", 12.0)),
      );
    let own = items().child(item(models, "content(row) on each item, a child foot"));

    let rows = [
      (
        "Menu::new(open, anchor)",
        "Signal<bool>, ElementRef",
        "",
        "Whether the menu is open, the app's signal, and what it hangs from. Choosing an item, a press outside, \
         Escape and Tab set it to false.",
      ),
      (
        "Menu::controlled(open, anchor, on_open_change)",
        "bool, ElementRef, Fn(bool)",
        "",
        "A menu the app holds open itself: a person closing it is told with false, and the app's next open is \
         what is drawn.",
      ),
      (
        "item",
        "MenuItem",
        "",
        "A command, after what the menu holds already. items takes several.",
      ),
      (
        "separator",
        "",
        "",
        "A line that ends one group of items and starts the next.",
      ),
      (
        "heading",
        "&str",
        "",
        "A heading over the items after it: what they are about.",
      ),
      ("label", "&str", "", "The menu's name to what reads the tree."),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Where the panel opens beside its anchor, 4 from it.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "4",
        "How far the panel is from its anchor, along the side it opens on.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What the panel does where the window has no room on its side.",
      ),
      (
        "on_dismiss",
        "Fn()",
        "",
        "A person closed it other than by a choice: Escape, a press outside, Tab. Adds.",
      ),
      (
        "on_open_change",
        "Fn(bool)",
        "",
        "Told false when a person closes it, by a choice too. Adds.",
      ),
      ("dismiss_on_escape", "bool", "true", "Escape closes it."),
      (
        "dismiss_on_outside_press",
        "bool",
        "true",
        "A press outside it closes it.",
      ),
      (
        "inline",
        "bool",
        "false",
        "The panel drawn where the menu is in the tree, open or not.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "An element of the app's among the entries: a foot, a section, a field. No item; children takes several.",
      ),
      (
        "within",
        "ElementRef",
        "",
        "Kept inside that element's box besides the window.",
      ),
      (
        "placed_by",
        "ElementRef",
        "",
        "Placed by that element's box in place of its anchor's; the anchor still opens it.",
      ),
      (
        "at",
        "Option<(f32, f32)>",
        "None",
        "A point of the window it stands at in place of its anchor.",
      ),
      ("MenuItem::new(label)", "&str", "", "A command, called by its label."),
      ("icon", "impl Into<Name>", "", "An icon of 16 before the label, muted."),
      (
        "shortcut",
        "&str",
        "",
        "The keys that do the same, at the end. The app binds them.",
      ),
      (
        "checked",
        "bool",
        "",
        "Whether what it turns on is on: a check at its end.",
      ),
      (
        "radio",
        "bool",
        "",
        "One of several of which one is selected: a dot at the end of the selected one.",
      ),
      (
        "submenu",
        "Menu",
        "",
        "A menu the item opens beside it, under the pointer or on a press, with a chevron at its end. Choosing in \
         it closes both.",
      ),
      (
        "destructive",
        "bool",
        "false",
        "It removes or cannot be undone: the destructive color, icon and all.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; it cannot be chosen and the arrows pass it by.",
      ),
      (
        "detail",
        "&str",
        "",
        "A second line under the label that says more of the item.",
      ),
      (
        "unavailable",
        "&str",
        "",
        "It cannot be chosen, for this reason: dimmed, a lock at its end, the reason under its label.",
      ),
      (
        "leading",
        "impl Into<Element>",
        "",
        "Any element before the label, in place of the icon: an avatar, a mark.",
      ),
      (
        "content",
        "impl Into<Element>",
        "",
        "All the item shows, the app's own row; it is still the menu's item.",
      ),
      (
        "on_select",
        "impl Fn()",
        "",
        "Called when it is chosen, after its menu closed. Adds.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Menu"],
        title: "Menu",
        description: "A list of commands in a panel over the window, opened from a button: the actions of a file, \
                      the account's menu, how a split button does what it does.",
        import: Some("use lurq::ui::{Menu, MenuItem};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The props of the second item and of the menu are controls on the right. Inline, the panel stands on the \
           stage; turn inline off and it hangs from the button, which opens and closes it.",
          playground,
        ),
        Section::new(
          "Dropdown",
          "The page holds whether the menu is open, in a signal the button turns. Choosing an item closes it, and so \
           do a press outside and Escape. While it is open Tab stops in it once, Up and Down move between its items, \
           Home and End go to the ends, and Enter or Space chooses one.",
          self.frame.demo(
            0,
            dropdown,
            "Menu::new(open, anchor).item(MenuItem::new(\"Profile\"))",
            &[
              "// anchor: ElementRef, open: Signal<bool>",
              "let toggle = open.clone();",
              "Row::new()",
              "    .ref_element(anchor.clone())",
              "    .child(",
              "        Button::new(\"Account\")",
              "            .variant(ButtonVariant::Outline)",
              "            .icon(icons::USER)",
              "            .trailing_icon(icons::CHEVRON_DOWN)",
              "            .on_click(move |_| toggle.update(|open| *open = !*open)),",
              "    )",
              "",
              "Menu::new(open, anchor)",
              "    .heading(\"Workspace\")",
              "    .item(MenuItem::new(\"Profile\").icon(icons::USER).shortcut(\"Ctrl+P\").on_select(..))",
              "    .item(MenuItem::new(\"Settings\").icon(icons::SETTINGS).shortcut(\"Ctrl+,\").on_select(..))",
              "    .item(MenuItem::new(\"Keyboard shortcuts\").icon(icons::KEYBOARD).on_select(..))",
              "    .separator()",
              "    .item(MenuItem::new(\"Log out\").icon(icons::SIGN_OUT).destructive(true).on_select(..))",
            ],
          ),
        ),
        Section::new(
          "Checked, radio, disabled, submenu",
          "The board's menu, drawn in place. A checked item has a check at its end and the page turns it; of the \
           radios one is chosen, with a dot. A disabled one is dimmed and the arrows pass it by, a separator ends a \
           group, and a label says what the next one is about. Export as opens its formats beside it, under the \
           pointer or on a press, and stays highlighted while they are open.",
          self.frame.demo(
            1,
            states,
            ".checked(grid.get())",
            &[
              "Menu::new(open, anchor)",
              "    .item(MenuItem::new(\"Undo\").shortcut(\"Ctrl+Z\"))",
              "    .item(MenuItem::new(\"Redo\").shortcut(\"Ctrl+Y\").disabled(true))",
              "    .separator()",
              "    .item(MenuItem::new(\"Show grid\").checked(grid.get()).on_select(..))",
              "    .item(MenuItem::new(\"Snap to guides\").checked(snap.get()).on_select(..))",
              "    .separator()",
              "    .heading(\"Zoom\")",
              "    .item(MenuItem::new(\"Fit to window\").radio(zoom.get() == 0).on_select(..))",
              "    .item(MenuItem::new(\"100%\").radio(zoom.get() == 1).on_select(..))",
              "    .separator()",
              "    .item(",
              "        MenuItem::new(\"Export as\").submenu(",
              "            // export: ElementRef, exporting: Signal<bool>",
              "            Menu::new(exporting, export)",
              "                .item(MenuItem::new(\"PNG\").on_select(..))",
              "                .item(MenuItem::new(\"SVG\").on_select(..))",
              "                .item(MenuItem::new(\"PDF\").on_select(..)),",
              "        ),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Split button",
          "A filled group of two: the action, and beside it the menu of how to do it, opened under the group from \
           its end. The group is the anchor: a button in a group takes no element ref of its own.",
          self.frame.demo(
            2,
            split,
            ".placement(Placement::BottomEnd)",
            &[
              "// anchor: ElementRef, open: Signal<bool>, strategy: Signal<usize>",
              "Row::new().ref_element(anchor.clone()).child(",
              "    ButtonGroup::new([])",
              "        .variant(ButtonGroupVariant::Primary)",
              "        .button(Button::new(\"Merge\").on_click(..))",
              "        .button(",
              "            Button::new(\"How to merge\")",
              "                .icon(icons::CHEVRON_DOWN)",
              "                .icon_only(true)",
              "                .on_click(move |_| toggle.update(|open| *open = !*open)),",
              "        ),",
              ")",
              "",
              "Menu::new(open, anchor)",
              "    .placement(Placement::BottomEnd)",
              "    .item(MenuItem::new(\"Create a merge commit\").checked(strategy.get() == 0).on_select(..))",
              "    .item(MenuItem::new(\"Squash and merge\").checked(strategy.get() == 1).on_select(..))",
              "    .item(MenuItem::new(\"Rebase and merge\").checked(strategy.get() == 2).on_select(..))",
            ],
          ),
        ),
        Section::new(
          "Details, reasons and avatars",
          "An item may say more under its label, say why it cannot be chosen (with a lock at its end, and the \
           reason read by what reads the tree), and stand an element of the app's before its label.",
          self.frame.demo(
            3,
            described,
            ".detail(..) .unavailable(..) .leading(..)",
            &[
              "MenuItem::new(\"Builder\").leading(avatar).detail(\"gpt-5 · high effort\").checked(true)",
              "MenuItem::new(\"Publish the release\").icon(icons::UPLOAD).unavailable(\"The run has not finished\")",
            ],
          ),
        ),
        Section::new(
          "The app's own rows",
          "An item given content shows the app's element and nothing of its own, and is still the menu's item: the \
           arrows reach it and typing its label, it is checked or unavailable to what reads the tree, and choosing \
           it closes the menu. A child is an element of the app's among the entries, a foot here, that is no item.",
          self.frame.demo(
            4,
            own,
            ".content(row) .child(foot)",
            &[
              "Menu::new(open, anchor)",
              "    .item(MenuItem::new(\"Opus\").content(row(\"Opus\", \"the deepest\")).checked(true))",
              "    .item(MenuItem::new(\"Sonnet\").content(row(\"Sonnet\", \"balanced\")).unavailable(\"Not on this plan\"))",
              "    .item(MenuItem::new(\"Haiku\").content(row(\"Haiku\", \"the fastest\")))",
              "    .separator()",
              "    .child(foot(\"Sonnet is not on this plan.\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a menu and its items are given.",
          props_table(
            &rows,
            &["Menu::new(open, anchor)", "MenuItem::new(label)"],
            self.frame.room(),
          ),
        ),
      ],
    )
  }
}
