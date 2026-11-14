//! The page of the tabs: their playground, tabs bound to a signal with a
//! panel each, the three variants the board draws, the list down the side,
//! their states, and the overflowing form.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Interaction, Tab, Tabs, TabsOrientation, TabsOverflow, TabsVariant, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, flag, prop},
};

/// The variants, as the code names them.
const VARIANTS: [(TabsVariant, &str, &str); 3] = [
  (TabsVariant::Default, "default", "TabsVariant::Default"),
  (TabsVariant::Line, "line", "TabsVariant::Line"),
  (TabsVariant::Pill, "pill", "TabsVariant::Pill"),
];

/// The orientations, as the code names them.
const ORIENTATIONS: [(TabsOrientation, &str, &str); 2] = [
  (TabsOrientation::Horizontal, "horizontal", "TabsOrientation::Horizontal"),
  (TabsOrientation::Vertical, "vertical", "TabsOrientation::Vertical"),
];

/// The sections of the playground's tabs: their label, icon and count.
const SECTIONS: [(&str, &str, u32); 4] = [
  ("General", icons::SETTINGS, 12),
  ("Appearance", icons::BOX, 4),
  ("Shortcuts", icons::INBOX, 0),
  ("Advanced", icons::USERS, 2),
];

/// The tabs of the overflowing form, as the board draws them and past it.
const OVERFLOWING: [&str; 10] = [
  "Canvas", "Layout", "Text", "Theme", "Router", "Forms", "Motion", "Charts", "Tables", "Media",
];

/// What a panel shows: a title and a line under it.
fn panel(title: &str, about: &str) -> Element {
  Column::new()
    .spacing(4.0)
    .padding_vertical(4.0)
    .child(text(title, 14.0, FontWeight::Medium))
    .child(quiet(about, 13.0))
    .into()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, one under the other.
fn items() -> Column {
  Column::new().width(FILL).spacing(28.0)
}

pub struct TabsPage {
  frame: Frame,
  playground: Playground,
  variant: Signal<usize>,
  orientation: Signal<usize>,
  disabled: Signal<bool>,
  disabled_tab: Signal<bool>,
  with_icons: Signal<bool>,
  counts: Signal<bool>,
  manual: Signal<bool>,
  selected: Signal<usize>,
  section: Signal<&'static str>,
  memory: Signal<&'static str>,
  filter: Signal<usize>,
  side: Signal<usize>,
  canvas: Signal<usize>,
  overflow: TabsOverflow,
}

impl TabsPage {
  /// The playground: the tabs as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (orientation, orientation_name, orientation_code) =
      ORIENTATIONS[self.orientation.get().min(ORIENTATIONS.len() - 1)];
    let (disabled, disabled_tab) = (self.disabled.get(), self.disabled_tab.get());
    let (with_icons, counts, manual) = (self.with_icons.get(), self.counts.get(), self.manual.get());
    let selected = self.selected.get();
    let tabs = move || {
      SECTIONS.iter().enumerate().map(move |(index, (label, icon, count))| {
        let mut tab = Tab::new(index, label).panel(move || panel(label, "What this section holds."));
        if with_icons {
          tab = tab.icon(*icon);
        }
        if counts {
          tab = tab.count(count);
        }
        tab.disabled(disabled_tab && index == SECTIONS.len() - 1)
      })
    };
    let plain = move |selected: usize| {
      Tabs::controlled(selected, tabs(), |_| {})
        .variant(variant)
        .orientation(orientation)
        .disabled(disabled)
        .manual(manual)
        .label("Settings")
    };
    let (record, set) = (self.playground.recorder(), self.selected.clone());
    let shown = Tabs::controlled(selected, tabs(), move |index| {
      set.set(index);
      record(format!("select({index})"));
    })
    .variant(variant)
    .orientation(orientation)
    .disabled(disabled)
    .manual(manual)
    .label("Settings")
    .force(self.playground.forced());

    let mut code = vec![
      "// section: Signal<usize>".to_owned(),
      "Tabs::new(section, [".to_owned(),
    ];
    for (index, (label, icon, count)) in SECTIONS.iter().enumerate() {
      let mut line = format!("    Tab::new({index}, {label:?})");
      if with_icons {
        line.push_str(&format!(".icon(icons::{})", icon.to_uppercase().replace('-', "_")));
      }
      if counts {
        line.push_str(&format!(".count({count})"));
      }
      if disabled_tab && index == SECTIONS.len() - 1 {
        line.push_str(".disabled(true)");
      }
      line.push_str(".panel(|| ..),");
      code.push(line);
    }
    code.push("])".to_owned());
    code.push("    .label(\"Settings\")".to_owned());
    if variant != TabsVariant::Default {
      code.push(format!("    .variant({variant_code})"));
    }
    if orientation != TabsOrientation::Horizontal {
      code.push(format!("    .orientation({orientation_code})"));
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if manual {
      code.push("    .manual(true)".to_owned());
    }

    let list = match variant {
      TabsVariant::Default => "muted rail · radius 8 · padded 3",
      TabsVariant::Line => "a line of 1 under it",
      TabsVariant::Pill => "tabs 4 apart",
    };
    let tab = match variant {
      TabsVariant::Default => "28 px · radius 6",
      TabsVariant::Line if orientation == TabsOrientation::Vertical => "32 px · marked at its start",
      TabsVariant::Line => "44 px · underlined when selected",
      TabsVariant::Pill => "30 px · round",
    };
    let tree = vec![
      format!("root        {}   {orientation_name}", self.playground.measured()),
      format!("├─ list      {list}   tablist"),
      format!("│  └─ tab    {tab}   ×{}", SECTIONS.len()),
      format!(
        "│     └─ {}label{}",
        if with_icons { "icon · " } else { "" },
        if counts { " · count" } else { "" }
      ),
      format!(
        "└─ panel     of {:?}   tabpanel",
        SECTIONS[selected.min(SECTIONS.len() - 1)].0
      ),
    ];

    let to_variant = self.variant.clone();
    let to_orientation = self.orientation.clone();
    let reset = (self.variant.clone(), self.orientation.clone(), self.selected.clone());
    let flags = [
      self.disabled.clone(),
      self.disabled_tab.clone(),
      self.with_icons.clone(),
      self.counts.clone(),
      self.manual.clone(),
    ];
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let orientations: Vec<(&str, usize)> = ORIENTATIONS.iter().enumerate().map(|(index, o)| (o.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Tabs",
        make: Arc::new(move || plain(selected).into()),
        shown: shown.into(),
        fill: true,
        forceable: true,
        facts: format!("{variant_name} · {orientation_name}"),
        event: "select",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "TabsVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| to_variant.set(index)),
          ),
          prop(
            "orientation",
            "TabsOrientation",
            segmented_fill(&orientations, &self.orientation.get(), move |index| {
              to_orientation.set(index)
            }),
          ),
          divider(),
          flag("disabled", &self.disabled),
          flag("a disabled tab", &self.disabled_tab),
          flag("icons", &self.with_icons),
          flag("counts", &self.counts),
          flag("manual", &self.manual),
        ],
        changed: [
          variant != TabsVariant::Default,
          orientation != TabsOrientation::Horizontal,
          disabled,
          disabled_tab,
          with_icons,
          counts,
          manual,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(0);
          reset.2.set(0);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TabsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      variant: ctx.signal(0),
      orientation: ctx.signal(0),
      disabled: ctx.signal(false),
      disabled_tab: ctx.signal(false),
      with_icons: ctx.signal(false),
      counts: ctx.signal(false),
      manual: ctx.signal(false),
      selected: ctx.signal(0),
      section: ctx.signal("general"),
      memory: ctx.signal("memory"),
      filter: ctx.signal(0),
      side: ctx.signal(0),
      canvas: ctx.signal(1),
      overflow: TabsOverflow::new(ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |value: &str| format!("the signal has: {value:?}");
    let live = item(
      Tabs::new(
        self.section.clone(),
        [
          Tab::new("general", "General").panel(|| panel("General", "The name of the workspace and its language.")),
          Tab::new("appearance", "Appearance").panel(|| panel("Appearance", "The theme, the density, the font.")),
          Tab::new("shortcuts", "Shortcuts").panel(|| panel("Shortcuts", "What each key does.")),
        ],
      )
      .label("Settings"),
      &says(self.section.get()),
    );

    let line_tabs = [
      ("all", "All", 7),
      ("profiles", "Profiles", 4),
      ("mcp", "MCP servers", 0),
      ("instructions", "Instructions", 1),
      ("skills", "Skills", 1),
      ("tools", "Tools", 1),
      ("memory", "Memory", 9),
      ("archived", "Archived", 0),
    ];
    let variants = items()
      .child(item(
        Tabs::controlled(0, [(0, "General"), (1, "Appearance"), (2, "Shortcuts")], |_| {}),
        "default",
      ))
      .child(item(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "General").count(12),
            Tab::new(1, "Appearance").count(4),
            Tab::new(2, "Shortcuts").count(0),
          ],
          |_| {},
        ),
        "default with counts",
      ))
      .child(item(
        Tabs::new(
          self.memory.clone(),
          line_tabs.map(|(value, label, count)| Tab::new(value, label).count(count).separated(value == "archived")),
        )
        .variant(TabsVariant::Line),
        "line with counts, the archived set apart",
      ))
      .child(item(
        Tabs::new(self.filter.clone(), [(0, "Overview"), (1, "Activity"), (2, "Settings")]).variant(TabsVariant::Pill),
        "pill",
      ));

    let side = Tabs::new(
      self.side.clone(),
      ["General", "Appearance", "Keyboard", "Advanced"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| Tab::new(index, label).panel(move || panel(label, "What this section holds."))),
    )
    .variant(TabsVariant::Line)
    .orientation(TabsOrientation::Vertical)
    .label("Preferences");

    let states = items()
      .child(item(
        Tabs::controlled(0, [(0, "General"), (1, "Appearance"), (2, "Shortcuts")], |_| {})
          .force(Some(Interaction::Hovered)),
        "hover: the label in the foreground",
      ))
      .child(item(
        Tabs::controlled(0, [(0, "General"), (1, "Appearance"), (2, "Shortcuts")], |_| {})
          .force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "General"),
            Tab::new(1, "Appearance"),
            Tab::new(2, "Shortcuts").disabled(true),
          ],
          |_| {},
        )
        .variant(TabsVariant::Line),
        "a disabled tab",
      ))
      .child(item(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "Inbox").icon(icons::INBOX).count(3),
            Tab::new(1, "Team").icon(icons::USERS),
            Tab::new(2, "Settings").icon(icons::SETTINGS),
          ],
          |_| {},
        )
        .variant(TabsVariant::Pill),
        "icons and a count",
      ));

    let overflowing = Column::new().width(420.0).child(
      Tabs::new(
        self.canvas.clone(),
        OVERFLOWING.iter().enumerate().map(|(index, label)| (index, *label)),
      )
      .variant(TabsVariant::Line)
      .overflow(self.overflow.clone()),
    );

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "Which tab is selected: the one with the signal's value. Selecting another sets it. Tabs::controlled takes \
         the value and a handler in its place.",
      ),
      (
        "tabs",
        "impl IntoIterator<Item = impl Into<Tab<T>>>",
        "",
        "The tabs, in order: a (value, label) or a Tab with an icon, a count, disabled, separated and a panel.",
      ),
      (
        "variant",
        "TabsVariant",
        "Default",
        "Default: a muted rail. Line: a row over a line, with counts. Pill: round tabs.",
      ),
      (
        "orientation",
        "TabsOrientation",
        "Horizontal",
        "Along the top with the panel under it, or down the side with the panel beside it.",
      ),
      ("disabled", "bool", "false", "No tab can be selected; each is dimmed."),
      (
        "manual",
        "bool",
        "false",
        "The arrow keys move the focus alone, and Enter or Space selects. Otherwise the selection follows the focus.",
      ),
      ("label", "&str", "", "What the list is called, to what reads the tree."),
      (
        "overflow",
        "TabsOverflow",
        "",
        "The list scrolls between two scroll buttons, with a menu of every tab at its end.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Tabs"],
        title: "Tabs",
        description: "A list of tabs of which one is selected, and the panel of the selected one: the sections of a \
                      settings page, the views of a record, the filters over a list.",
        import: Some("use lurq::ui::{Tab, Tabs};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Tabs is a control on the right. The tabs on the stage are live: a press selects one, and the \
           arrow keys move the selection once Tab has reached the list.",
          playground,
        ),
        Section::new(
          "Tabs",
          "The page holds which tab is selected, in a signal. Tab reaches the list once, at the selected tab; Left \
           and Right select, Home and End go to the ends, and Tab goes on into the panel. Only the selected tab's \
           panel is built.",
          self.frame.demo(
            0,
            live,
            "Tabs::new(section, [..])",
            &[
              "// section: Signal<&'static str>",
              "Tabs::new(section, [",
              "    Tab::new(\"general\", \"General\").panel(|| general()),",
              "    Tab::new(\"appearance\", \"Appearance\").panel(|| appearance()),",
              "    Tab::new(\"shortcuts\", \"Shortcuts\").panel(|| shortcuts()),",
              "])",
              ".label(\"Settings\")",
            ],
          ),
        ),
        Section::new(
          "Variants",
          "A muted rail with the selected tab lifted out of it; a row over a line with counts, the selected tab \
           underlined; round tabs, the selected one in the primary fill.",
          self.frame.demo(
            1,
            variants,
            ".variant(TabsVariant::Line)",
            &[
              "Tabs::controlled(0, [(0, \"General\"), (1, \"Appearance\"), (2, \"Shortcuts\")], |_| {})",
              "",
              "Tabs::controlled(0, [Tab::new(0, \"General\").count(12), ..], |_| {})",
              "",
              "// memory: Signal<&'static str>",
              "Tabs::new(memory, [",
              "    Tab::new(\"all\", \"All\").count(7),",
              "    ..",
              "    Tab::new(\"memory\", \"Memory\").count(9),",
              "    Tab::new(\"archived\", \"Archived\").count(0).separated(true),",
              "])",
              ".variant(TabsVariant::Line)",
              "",
              "// filter: Signal<usize>",
              "Tabs::new(filter, [(0, \"Overview\"), (1, \"Activity\"), (2, \"Settings\")])",
              "    .variant(TabsVariant::Pill)",
            ],
          ),
        ),
        Section::new(
          "Down the side",
          "The list runs down the start side with the panel beside it; Up and Down select.",
          self.frame.demo(
            2,
            Row::new().width(FILL).align_items(Alignment::Start).child(side),
            ".orientation(TabsOrientation::Vertical)",
            &[
              "// side: Signal<usize>",
              "Tabs::new(side, [",
              "    Tab::new(0, \"General\").panel(|| general()),",
              "    Tab::new(1, \"Appearance\").panel(|| appearance()),",
              "    Tab::new(2, \"Keyboard\").panel(|| keyboard()),",
              "    Tab::new(3, \"Advanced\").panel(|| advanced()),",
              "])",
              ".variant(TabsVariant::Line)",
              ".orientation(TabsOrientation::Vertical)",
              ".label(\"Preferences\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "A tab under the pointer has its label in the foreground. The ring of the focus is the tab's. A disabled \
           tab is dimmed, and the arrow keys pass it by.",
          self.frame.demo(
            3,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "// Drawn as under the pointer, or focused from the keyboard, with nothing pointing at it.",
              "Tabs::controlled(0, [..], |_| {}).force(Some(Interaction::Hovered))",
              "Tabs::controlled(0, [..], |_| {}).force(Some(Interaction::FocusVisible))",
              "",
              "Tab::new(2, \"Shortcuts\").disabled(true)",
              "",
              "Tab::new(0, \"Inbox\").icon(icons::INBOX).count(3)",
            ],
          ),
        ),
        Section::new(
          "Overflow",
          "More tabs than the width holds: the list scrolls between two scroll buttons, and More opens a menu of every \
           tab with the selected one marked. The arrow keys scroll the list to the tab they select.",
          self.frame.demo(
            4,
            overflowing,
            ".overflow(TabsOverflow::new(open))",
            &[
              "// In create: overflow: TabsOverflow::new(ctx.signal(false))",
              "// canvas: Signal<usize>",
              "Tabs::new(canvas, [(0, \"Canvas\"), (1, \"Layout\"), (2, \"Text\"), ..])",
              "    .variant(TabsVariant::Line)",
              "    .overflow(overflow.clone())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What tabs are given.",
          props_table(&rows, &["value", "tabs"], self.frame.room()),
        ),
      ],
    )
  }
}
