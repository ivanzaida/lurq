//! The page of the list and its items: its playground, the board's list of people, the contained list, settings
//! rows with their values and switches, rows with tags, an (i), details and what they say under themselves, the rows
//! under a finger, and the states of a row.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{Density, TypographyStyle},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Avatar, Badge, Button, ButtonSize, ButtonVariant, Interaction, Length, Link, List, ListItem, ListPart, Switch, Tag,
    icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, muted, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The rows of the playground: a title, its description, its icon, its value and its time.
const ROWS: [(&str, &str, &str, &str, &str); 3] = [
  ("Wi-Fi", "Connected, secured", icons::WIFI, "Home", "2m"),
  ("Bluetooth", "2 devices nearby", icons::BLUETOOTH, "On", "1h"),
  ("Airplane mode", "Turns every radio off", icons::AIRPLANE, "Off", "3h"),
];

/// What stands before a row's text.
const LEADING: [(&str, usize); 5] = [("none", 0), ("icon", 1), ("avatar", 2), ("checkbox", 3), ("radio", 4)];

/// What stands at a row's end.
const ENDS: [(&str, usize); 6] = [
  ("none", 0),
  ("value", 1),
  ("meta", 2),
  ("switch", 3),
  ("badge", 4),
  ("shortcut", 5),
];

/// The densities, as the code names them.
const DENSITIES: [(&str, Density); 4] = [
  ("compact", Density::Compact),
  ("default", Density::Default),
  ("comfortable", Density::Comfortable),
  ("touch", Density::Touch),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// The board's list: people and what they did, the second selected.
fn people() -> List {
  List::new()
    .label("Activity")
    .item(
      ListItem::new("Ivan Zaida")
        .description("Pushed 3 commits to feat/node-blur")
        .leading(Avatar::new("Ivan Zaida"))
        .meta("2m"),
    )
    .item(
      ListItem::new("Dependabot")
        .description("Bumped wgpu to 26.0")
        .leading(Avatar::new("Dependabot"))
        .meta("1h")
        .selected(true),
    )
    .item(
      ListItem::new("CI")
        .description("All 412 tests passed")
        .leading(Avatar::new("CI"))
        .meta("3h"),
    )
    .style(ListPart::Root, |s| s.width(340.0).max_width(Length::Fill))
}

pub struct ListPage {
  frame: Frame,
  playground: Playground,
  leading: Signal<usize>,
  end: Signal<usize>,
  density: Signal<usize>,
  description: Signal<bool>,
  chevron: Signal<bool>,
  pressed: Signal<bool>,
  dividers: Signal<bool>,
  contained: Signal<bool>,
  selected: Signal<bool>,
  disabled: Signal<bool>,
  /// The rows that are checked, or the one chosen, or on: what a press of a row or of its switch changes.
  checked: Signal<Vec<usize>>,
  dark: Signal<bool>,
  wifi: Signal<bool>,
}

impl ListPage {
  /// The playground: three settings as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (leading, end) = (self.leading.get(), self.end.get());
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let (description, chevron, pressed) = (self.description.get(), self.chevron.get(), self.pressed.get());
    let (dividers, contained, selected, disabled) = (
      self.dividers.get(),
      self.contained.get(),
      self.selected.get(),
      self.disabled.get(),
    );
    let checked = self.checked.get();
    let record = self.playground.recorder();

    // A row of the playground; `live` with the handlers that change the page and count, or still for a window.
    let build = {
      let (set, checked) = (self.checked.clone(), checked.clone());
      move |live: bool| -> List {
        let mut list = List::new()
          .density(density)
          .separators(dividers)
          .contained(contained)
          .label("Settings")
          .style(ListPart::Root, |s| s.width(340.0));
        if contained {
          list = list.title("Network");
        }
        // The lines start under the text where an icon or an avatar is before it.
        if dividers && leading != 0 {
          let inset = if density == Density::Touch { 60.0 } else { 40.0 };
          list = list.inset(inset);
        }
        for (index, (title, about, icon, value, meta)) in ROWS.into_iter().enumerate() {
          let on = checked.contains(&index);
          let mut row = ListItem::new(title)
            .chevron(chevron)
            .selected(selected && index == 1)
            .disabled(disabled && index == 2);
          if description {
            row = row.description(about);
          }
          row = match leading {
            1 => row.icon(icon),
            2 => row.leading(Avatar::new(title)),
            3 => row.checkbox(on),
            4 => row.radio(on),
            _ => row,
          };
          row = match end {
            1 => row.value(value),
            2 => row.meta(meta),
            3 => {
              let mut switch = Switch::new(title)
                .label_hidden(true)
                .on(on)
                .disabled(disabled && index == 2);
              if live {
                let (set, record) = (set.clone(), record.clone());
                switch = switch.on_change(move |now| {
                  set.update(|held| {
                    held.retain(|row| *row != index);
                    if now {
                      held.push(index);
                    }
                  });
                  record(format!("Switch::on_change({title:?}, {now})"));
                });
              }
              row.trailing(switch)
            }
            4 => row.trailing(Badge::new(value)),
            5 => row.shortcut(&format!("Mod+{}", index + 1)),
            _ => row,
          };
          if pressed && live {
            let (set, record) = (set.clone(), record.clone());
            row = row.on_click(move |_| {
              set.update(|held| match leading {
                // A radio: this one alone.
                4 => *held = vec![index],
                _ => match held.iter().position(|row| *row == index) {
                  Some(at) => {
                    held.remove(at);
                  }
                  None => held.push(index),
                },
              });
              record(format!("on_click({title:?})"));
            });
          } else if pressed {
            row = row.on_click(|_| {});
          }
          list = list.item(row);
        }
        list
      }
    };
    let shown = build(true).force(self.playground.forced());
    let make = {
      let build = build.clone();
      move || build(false).into()
    };

    let mut code = vec!["List::new()".to_owned()];
    if density != Density::Default {
      code.push(format!("    .density(Density::{density:?})"));
    }
    if contained {
      code.push("    .contained(true)".to_owned());
      code.push("    .title(\"Network\")".to_owned());
    }
    if dividers {
      code.push("    .separators(true)".to_owned());
      if leading != 0 {
        code.push(format!(
          "    .inset({})",
          if density == Density::Touch { 60 } else { 40 }
        ));
      }
    }
    code.push("    .item(".to_owned());
    code.push("        ListItem::new(\"Wi-Fi\")".to_owned());
    if description {
      code.push("            .description(\"Connected, secured\")".to_owned());
    }
    match leading {
      1 => code.push("            .icon(icons::WIFI)".to_owned()),
      2 => code.push("            .leading(Avatar::new(\"Wi-Fi\"))".to_owned()),
      3 => code.push("            .checkbox(on)".to_owned()),
      4 => code.push("            .radio(on)".to_owned()),
      _ => {}
    }
    match end {
      1 => code.push("            .value(\"Home\")".to_owned()),
      2 => code.push("            .meta(\"2m\")".to_owned()),
      3 => code.push("            .trailing(Switch::new(\"Wi-Fi\").label_hidden(true).on(on))".to_owned()),
      4 => code.push("            .trailing(Badge::new(\"Home\"))".to_owned()),
      5 => code.push("            .shortcut(\"Mod+1\")".to_owned()),
      _ => {}
    }
    if chevron {
      code.push("            .chevron(true)".to_owned());
    }
    if pressed {
      code.push("            .on_click(|_| /* open the setting */),".to_owned());
    }
    code.push("    )".to_owned());
    code.push("    .item(ListItem::new(\"Bluetooth\")..)".to_owned());
    code.push("    .item(ListItem::new(\"Airplane mode\")..)".to_owned());

    let height = match (density, description) {
      (Density::Compact, false) => 28,
      (Density::Compact, true) => 44,
      (Density::Comfortable, false) => 44,
      (Density::Comfortable, true) => 60,
      (Density::Touch, false) => 56,
      (Density::Touch, true) => 72,
      (_, false) => 36,
      (_, true) => 52,
    };
    let tree = vec![
      format!(
        "list        {}{}",
        self.playground.measured(),
        if contained { "   a box with a header of 32" } else { "" }
      ),
      format!(
        "└─ item     {height} high at {density_name}, {} in, 12 apart",
        if density == Density::Touch { 16 } else { 12 }
      ),
      "   ├─ leading      icon · avatar · checkbox · radio".to_owned(),
      format!(
        "   ├─ content      title{}",
        if description {
          " over its description, 2 apart"
        } else {
          ""
        }
      ),
      "   └─ trailing     value · meta · element · keys · chevron, 8 apart".to_owned(),
    ];

    let choose_leading = self.leading.clone();
    let choose_end = self.end.clone();
    let choose_density = self.density.clone();
    let flags = [
      self.description.clone(),
      self.chevron.clone(),
      self.dividers.clone(),
      self.contained.clone(),
      self.selected.clone(),
      self.disabled.clone(),
    ];
    let (to_leading, to_end, to_density, to_pressed, to_checked) = (
      self.leading.clone(),
      self.end.clone(),
      self.density.clone(),
      self.pressed.clone(),
      self.checked.clone(),
    );
    let densities: Vec<(&str, usize)> = DENSITIES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "List",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: true,
        facts: format!("{} rows", ROWS.len()),
        event: if pressed || end == 3 { "on_click" } else { "" },
        code,
        tree,
        controls: vec![
          prop(
            "leading",
            "icon / leading / checkbox / radio",
            segmented_fill(&LEADING, &leading, move |value| choose_leading.set(value)),
          ),
          prop(
            "trailing",
            "value / meta / trailing / shortcut",
            segmented_fill(&ENDS, &end, move |value| choose_end.set(value)),
          ),
          prop(
            "density",
            "Density",
            segmented_fill(&densities, &self.density.get(), move |value| choose_density.set(value)),
          ),
          divider(),
          flag("description", &self.description),
          flag("chevron", &self.chevron),
          flag("on_click", &self.pressed),
          flag("separators", &self.dividers),
          flag("contained", &self.contained),
          flag("a selected row", &self.selected),
          flag("a disabled row", &self.disabled),
        ],
        changed: [
          leading != 1,
          end != 1,
          density != Density::Default,
          description,
          !chevron,
          !pressed,
          dividers,
          contained,
          selected,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          flags[1].set(true);
          to_leading.set(1);
          to_end.set(1);
          to_density.set(1);
          to_pressed.set(true);
          to_checked.set(vec![0]);
        }),
      },
    )
  }
}

impl Component for ListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      leading: ctx.signal(1),
      end: ctx.signal(1),
      density: ctx.signal(1),
      description: ctx.signal(false),
      chevron: ctx.signal(true),
      pressed: ctx.signal(true),
      dividers: ctx.signal(false),
      contained: ctx.signal(false),
      selected: ctx.signal(false),
      disabled: ctx.signal(false),
      checked: ctx.signal(vec![0]),
      dark: ctx.signal(true),
      wifi: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let recent = List::new()
      .contained(true)
      .title("Recent")
      .action(Button::new("Clear").variant(ButtonVariant::Ghost).size(ButtonSize::Sm))
      .item(ListItem::new("theme.md").chevron(true).on_click(|_| {}))
      .item(ListItem::new("blur.md").chevron(true).on_click(|_| {}))
      .item(ListItem::new("canvas.md").chevron(true).on_click(|_| {}))
      .style(ListPart::Root, |s| s.width(240.0));

    let (dark, set_dark) = (self.dark.get(), self.dark.clone());
    let settings = List::new()
      .separators(true)
      .label("Startup")
      .item(
        ListItem::new("Open at login")
          .description("Start lurq when you sign in to this computer.")
          .trailing(
            Switch::new("Open at login")
              .label_hidden(true)
              .on(dark)
              .on_change(move |on| set_dark.set(on)),
          ),
      )
      .item(
        ListItem::new("Language")
          .description("The language of menus and messages.")
          .value("English")
          .chevron(true)
          .on_click(|_| {}),
      )
      .item(ListItem::new("New window").shortcut("Mod+Shift+N").on_click(|_| {}))
      .style(ListPart::Root, |s| s.width(420.0).max_width(Length::Fill));

    let (wifi, set_wifi) = (self.wifi.get(), self.wifi.clone());
    let finger = List::new()
      .density(Density::Touch)
      .contained(true)
      .inset(60.0)
      .label("Network")
      .item(
        ListItem::new("Wi-Fi")
          .icon(icons::WIFI)
          .value("Home")
          .chevron(true)
          .on_click(|_| {}),
      )
      .item(
        ListItem::new("Bluetooth")
          .icon(icons::BLUETOOTH)
          .value("On")
          .chevron(true)
          .on_click(|_| {}),
      )
      .item(
        ListItem::new("Airplane mode").icon(icons::AIRPLANE).trailing(
          Switch::new("Airplane mode")
            .label_hidden(true)
            .on(!wifi)
            .on_change(move |on| set_wifi.set(!on)),
        ),
      )
      .style(ListPart::Root, |s| {
        s.width(328.0)
          .max_width(Length::Fill)
          .radius(lurq::app::theme::RadiusSize::Xl)
      });

    let state = |forced: Option<Interaction>, selected: bool, disabled: bool| {
      List::new()
        .item(
          ListItem::new("Dependabot")
            .description("Bumped wgpu to 26.0")
            .leading(Avatar::new("Dependabot"))
            .meta("1h")
            .selected(selected)
            .disabled(disabled)
            .on_click(|_| {})
            .force(forced),
        )
        .style(ListPart::Root, |s| s.width(300.0).max_width(Length::Fill))
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(None, false, false), "at rest"))
      .child(item(state(None, true, false), "selected"))
      .child(item(
        state(Some(Interaction::Hovered), false, false),
        "under the pointer",
      ))
      .child(item(
        state(Some(Interaction::FocusVisible), false, false),
        "focused from the keyboard",
      ))
      .child(item(state(None, false, true), "disabled"));

    // Settings rows that say more by their title and under themselves.
    let said = List::new()
      .contained(true)
      .item(
        ListItem::new("Theme")
          .description("Light, dark, or as the system is")
          .tag(Tag::new("Default"))
          .info("The app follows the system unless a theme is chosen here.")
          .value("System"),
      )
      .item(
        ListItem::new("Access token")
          .description("Read and write on the repository")
          .detail(Link::new("What it can reach"))
          .below(muted(
            "The token expired on 18 November: pull requests are not opened.",
            TypographyStyle::Description,
          ))
          .trailing(
            Button::new("Renew")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          ),
      );

    // Settings rows whose words are still read.
    let reading = List::new()
      .contained(true)
      .item(
        ListItem::new("Default branch")
          .title_pending(true)
          .description_pending(true)
          .value("main"),
      )
      .item(
        ListItem::new("Pull requests")
          .description_pending(true)
          .trailing(Switch::new("Pull requests").label_hidden(true).on(true)),
      )
      .item(ListItem::new("Merge method").description("Squash, so each run is one commit"));

    let rows = [
      (
        "ListItem::new",
        "&str",
        "",
        "A row that says its title: `description`, `leading`, `icon`, `checkbox`, `radio`, `value`, `meta`, \
         `trailing`, `shortcut`, `chevron`.",
      ),
      (
        "tag / info / detail / below",
        "impl Into<Element> / &str",
        "",
        "Elements after the title, an (i) with its words in a tooltip after them, elements under the description, \
         and what the row says under itself, under its line, which a press of the row does not reach. A row with \
         something below is held with it in the part item (x-item); the row's id stays on the row.",
      ),
      (
        "title_pending / description_pending",
        "bool",
        "false",
        "A bone in place of the title's or the description's words while they are read, on a line of their height, \
         its size the recipe's title_skeleton and description_skeleton. The row says it is busy.",
      ),
      (
        "on_click",
        "impl Fn(MouseEvent)",
        "",
        "What a press of the row does: the row is pressed, a stop of Tab, and a button to tooling. A row without \
         it answers no pointer.",
      ),
      (
        "on_open_aside",
        "impl Fn(MouseEvent)",
        "",
        "Called in place of on_click for the middle button, or a press with Ctrl (Cmd on macOS): open it in a new tab.",
      ),
      (
        "tab_stop",
        "bool",
        "true",
        "false: Tab passes a pressed row by, and a press leaves the focus where it was.",
      ),
      (
        "href",
        "&str",
        "",
        "Where a press goes in the app's router: the row is a link.",
      ),
      (
        "selected / disabled",
        "bool",
        "false",
        "The selected row is highlighted; a disabled one is dimmed and takes nothing.",
      ),
      (
        "density",
        "Density",
        "the theme's",
        "How high the row is: 28, 36, 44 or 56, and 16 more with a description. Under a finger it is the mobile \
         row.",
      ),
      (
        "List::item / items",
        "ListItem",
        "",
        "The rows, one under the other: 2 apart, or touching with a line between each two.",
      ),
      (
        "separators / inset",
        "bool / f32",
        "false / 0",
        "A line between each two rows, and how far from the start it starts.",
      ),
      (
        "contained / title / action",
        "bool / &str / Element",
        "false",
        "The rows in a box, touching, under a header with a title in capitals and an action: the one, a second \
         takes its place.",
      ),
      (
        "style / rules",
        "ListItemPart / ListPart",
        "",
        "Root, Leading, Icon, Content, Title, Description, Trailing, Value, Meta, Chevron; Root, Header, \
         Title, Action, Items, Separator.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "List"],
        title: "List",
        description: "Rows one under the other: settings, people, files, notifications. A row has a title, a \
                      description under it, what stands before it and what stands at its end.",
        import: Some("use lurq::ui::{List, ListItem};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ListItem and List is a control on the right. The rows on the stage are live: a press of \
           a row with `on_click` is counted, as is a switch at a row's end.",
          playground,
        ),
        Section::new(
          "People",
          "The board's list: an avatar, a title over a description, and a time at the end. The second row is \
           selected.",
          self.frame.demo(
            0,
            people(),
            "ListItem::new(title).description(about).leading(avatar).meta(time)",
            &[
              "List::new()",
              "    .item(",
              "        ListItem::new(\"Ivan Zaida\")",
              "            .description(\"Pushed 3 commits to feat/node-blur\")",
              "            .leading(Avatar::new(\"Ivan Zaida\"))",
              "            .meta(\"2m\"),",
              "    )",
              "    .item(ListItem::new(\"Dependabot\")..selected(true))",
              "    .item(ListItem::new(\"CI\")..)",
            ],
          ),
        ),
        Section::new(
          "Contained list",
          "The rows in a box, touching, with a line between each two and a header over them: what was opened \
           lately, the members of a group.",
          self.frame.demo(
            1,
            recent,
            "List::new().contained(true).title(\"Recent\")",
            &[
              "List::new()",
              "    .contained(true)",
              "    .title(\"Recent\")",
              "    .action(Button::new(\"Clear\").variant(ButtonVariant::Ghost).size(ButtonSize::Sm))",
              "    .item(ListItem::new(\"theme.md\").chevron(true).on_click(|_| /* open it */))",
              "    .item(ListItem::new(\"blur.md\").chevron(true).on_click(|_| /* open it */))",
              "    .item(ListItem::new(\"canvas.md\").chevron(true).on_click(|_| /* open it */))",
            ],
          ),
        ),
        Section::new(
          "Settings",
          "A row that is not pressed holds a control that is: the switch. A row that leads somewhere has its \
           value and a chevron; one that does something, its keys.",
          self.frame.demo(
            2,
            settings,
            "ListItem::new(title).trailing(Switch::new(title))",
            &[
              "List::new()",
              "    .separators(true)",
              "    .item(",
              "        ListItem::new(\"Open at login\")",
              "            .description(\"Start lurq when you sign in to this computer.\")",
              "            .trailing(Switch::new(\"Open at login\").label_hidden(true).on(on)),",
              "    )",
              "    .item(ListItem::new(\"Language\").description(..).value(\"English\").chevron(true).on_click(..))",
              "    .item(ListItem::new(\"New window\").shortcut(\"Mod+Shift+N\").on_click(..))",
            ],
          ),
        ),
        Section::new(
          "Saying more",
          "Tags and an (i) after the title, a link under the description, and what the row says under itself: a \
           problem, a code to copy. It stands under the row's line, and a press of it is not the row's.",
          self.frame.demo(
            5,
            said,
            ".tag(..).info(..).detail(..).below(..)",
            &[
              "List::new()",
              "    .contained(true)",
              "    .item(",
              "        ListItem::new(\"Theme\")",
              "            .description(\"Light, dark, or as the system is\")",
              "            .tag(Tag::new(\"Default\"))",
              "            .info(\"The app follows the system unless a theme is chosen here.\")",
              "            .value(\"System\"),",
              "    )",
              "    .item(",
              "        ListItem::new(\"Access token\")",
              "            .description(\"Read and write on the repository\")",
              "            .detail(Link::new(\"What it can reach\"))",
              "            .below(muted(\"The token expired on 18 November: pull requests are not opened.\", ..))",
              "            .trailing(Button::new(\"Renew\").variant(ButtonVariant::Outline).size(ButtonSize::Sm)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "While its words are read",
          "A row whose title or description the app is still reading shows a bone in their place, on a line of \
           their height, so the words that come move nothing; the rest of the row is drawn as it is.",
          self.frame.demo(
            6,
            reading,
            ".title_pending(true).description_pending(true)",
            &[
              "List::new()",
              "    .contained(true)",
              "    .item(ListItem::new(\"Default branch\").title_pending(true).description_pending(true).value(\"main\"))",
              "    .item(ListItem::new(\"Pull requests\").description_pending(true).trailing(Switch::new(..)))",
              "    .item(ListItem::new(\"Merge method\").description(\"Squash, so each run is one commit\"))",
              "",
              "// A bone of the app's width, for one row:",
              "ListItem::new(\"Default branch\").title_pending(true).style(ListItemPart::TitleSkeleton, |s| s.width(96.0))",
            ],
          ),
        ),
        Section::new(
          "Under a finger",
          "At the touch density a row is the mobile one: 56 high, its icon on a tile, its title at 16. The lines \
           start under the text.",
          self.frame.demo(
            3,
            finger,
            "List::new().density(Density::Touch).contained(true).inset(60.0)",
            &[
              "List::new()",
              "    .density(Density::Touch)",
              "    .contained(true)",
              "    .inset(60.0)",
              "    .item(ListItem::new(\"Wi-Fi\").icon(icons::WIFI).value(\"Home\").chevron(true))",
              "    .item(ListItem::new(\"Bluetooth\").icon(icons::BLUETOOTH).value(\"On\").chevron(true))",
              "    .item(ListItem::new(\"Airplane mode\").icon(icons::AIRPLANE).trailing(Switch::new(..)))",
              "    .style(ListPart::Root, |s| s.radius(RadiusSize::Xl))",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws a row at rest and a selected one. A pressed row under the pointer and held down is \
           veiled, the focus from the keyboard is a ring inside it, and a disabled row is dimmed.",
          self.frame.demo(
            4,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "ListItem::new(\"Dependabot\")",
              "    .description(\"Bumped wgpu to 26.0\")",
              "    .leading(Avatar::new(\"Dependabot\"))",
              "    .meta(\"1h\")",
              "    .on_click(|_| {})",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "    .force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a row and a list are given.",
          props_table(&rows, &["ListItem::new"], self.frame.room()),
        ),
      ],
    )
  }
}
