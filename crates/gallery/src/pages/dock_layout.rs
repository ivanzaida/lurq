//! The page of the dock layout: its playground, the board's window as an app builds it, a layout stored and
//! restored, a panel floating, the keys, and an empty area of documents.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, ScrollVertical},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, DockLayout, DockNode, DockPanel, DockPlace, DockSide, DockState, EmptyState,
    EmptyStateSize, Kbd, Orientation, TreeItem, TreeView, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, icon, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag},
};

type Id = &'static str;

/// What a handler of the playground calls to say what the layout did.
type Record = Arc<dyn Fn(String) + Send + Sync>;

/// How high the playground's layout is: the stage less the room above and under it.
const PLAY_HEIGHT: f32 = 360.0;
/// How high the board's window is.
const BOARD_HEIGHT: f32 = 440.0;
/// How high the smaller layouts of the page are.
const SMALL_HEIGHT: f32 = 300.0;

/// What `blur.rs` holds.
const BLUR: [&str; 8] = [
  "use lurq::core::Signal;",
  "",
  "/// A frosted backdrop.",
  "pub struct Blur {",
  "  radius: f32,",
  "}",
  "",
  "pub fn blur(radius: f32) -> Blur {",
];

/// What `theme.rs` holds.
const THEME: [&str; 6] = [
  "pub struct Theme {",
  "  palette: Palette,",
  "  radius: RadiusScale,",
  "}",
  "",
  "impl Theme {",
];

/// What the terminal shows.
const TERMINAL: [&str; 4] = [
  "$ cargo run -p demo",
  "   Compiling lurq v0.62.0",
  "    Finished `dev` in 4.21s",
  "     Running `target/debug/demo`",
];

/// The problems: what, and where.
const PROBLEMS: [(&str, &str); 3] = [
  ("unused variable `tint`", "blur.rs 12:9"),
  ("unused import `Signal`", "blur.rs 1:5"),
  ("field `radius` is never read", "theme.rs 3:3"),
];

/// What the outline lists of `blur.rs`.
const OUTLINE: [&str; 3] = ["struct Blur", "fn blur", "impl Default for Blur"];

/// The keys of a dock layout and what each does.
const KEYS: [(&[&str], &str); 6] = [
  (&["F6"], "The focus goes to the next group."),
  (&["Shift+F6"], "The focus goes to the group before."),
  (
    &["Shift+F10", "Menu"],
    "On a tab or a header: its menu. Move to each edge, Float, Hide, Maximize or Restore, Close.",
  ),
  (&["Left", "Right"], "On a tab: the tab before or after it."),
  (&["Enter", "Space"], "On a tab: its panel is shown."),
  (
    &["Escape"],
    "Gives up a drag going on; hides a panel shown from its strip.",
  ),
];

/// The board's arrangement: the explorer at the start, two documents, the terminal's group of three along the
/// bottom, and the outline hidden in a strip at the right. With `empty` the area of documents holds nothing.
fn board(empty: bool) -> DockState {
  let documents = if empty {
    DockNode::documents(None)
  } else {
    DockNode::documents(DockNode::group(["blur.rs", "theme.rs"]))
  };
  let mut state = DockState::new(
    DockNode::row([
      DockNode::group(["explorer"]),
      DockNode::column([documents, DockNode::group(["terminal", "problems", "output"])]).shares([0.7, 0.3]),
    ])
    .shares([0.25, 0.75]),
  );
  state.show("outline", DockPlace::Hidden(DockSide::Right));
  state
}

/// The explorer and a document, and the outline floating over them.
fn floating() -> DockState {
  let mut state = DockState::new(
    DockNode::row([
      DockNode::group(["explorer"]),
      DockNode::documents(DockNode::group(["blur.rs", "theme.rs"])),
    ])
    .shares([0.3, 0.7]),
  );
  state.show(
    "outline",
    DockPlace::Float {
      position: (360.0, 48.0),
      size: Some((220.0, 170.0)),
    },
  );
  state
}

/// The explorer, and an area of documents with nothing in it.
fn empty_documents() -> DockState {
  DockState::new(DockNode::row([DockNode::group(["explorer"]), DockNode::documents(None)]).shares([0.3, 0.7]))
}

/// The board's file tree.
fn files() -> Vec<TreeItem<Id>> {
  vec![
    TreeItem::folder("crates", "crates").items([
      TreeItem::folder("lurq", "lurq").items([
        TreeItem::file("blur.rs", "blur.rs"),
        TreeItem::file("theme.rs", "theme.rs"),
      ]),
      TreeItem::folder("demo", "demo").item(TreeItem::file("main.rs", "main.rs")),
    ]),
    TreeItem::file("Cargo.toml", "Cargo.toml"),
  ]
}

/// The explorer's tree, bound to what is selected and open in it.
fn tree(selected: &Signal<Option<Id>>, open: &Signal<Vec<Id>>) -> Element {
  TreeView::new(selected.clone(), files())
    .label("Files")
    .open(open.clone())
    .into()
}

/// The explorer of a smaller layout: the files as rows.
fn file_rows() -> Element {
  let rows = [
    (icons::FOLDER, "crates"),
    (icons::FILE_CODE, "blur.rs"),
    (icons::FILE_CODE, "theme.rs"),
    (icons::FILE, "Cargo.toml"),
  ];
  rows
    .iter()
    .fold(
      Column::new().width(FILL).padding(8.0).spacing(2.0),
      |column, (glyph, name)| {
        column.child(
          Row::new()
            .height(26.0)
            .padding_horizontal(8.0)
            .spacing(8.0)
            .align_items(Alignment::Center)
            .child(icon(glyph, 14.0).color(PaletteColor::MutedForeground))
            .child(text(name, 13.0, FontWeight::Normal)),
        )
      },
    )
    .into()
}

/// What a panel holds, scrolled where it is higher than the panel.
fn scrolled(content: impl Into<Element>) -> Element {
  ScrollVertical::new(content)
    .width(FILL)
    .flex_full(1.0, 1.0, None)
    .into()
}

/// Lines of code, or of a terminal, in mono.
fn code(lines: &[&str]) -> Element {
  scrolled(
    lines
      .iter()
      .fold(Column::new().width(FILL).padding(12.0).spacing(2.0), |column, line| {
        column.child(mono(if line.is_empty() { " " } else { line }, 12.0))
      }),
  )
}

/// The problems, a row each.
fn problems() -> Element {
  scrolled(PROBLEMS.iter().fold(
    Column::new().width(FILL).padding(8.0).spacing(2.0),
    |column, (what, place)| {
      column.child(
        Row::new()
          .width(FILL)
          .height(24.0)
          .padding_horizontal(4.0)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(icon(icons::ALERT_CIRCLE, 14.0).color(PaletteColor::Warning))
          .child(text(what, 12.0, FontWeight::Normal))
          .child(quiet(place, 11.0)),
      )
    },
  ))
}

/// What the output shows.
fn output() -> Element {
  scrolled(
    Column::new()
      .width(FILL)
      .padding(12.0)
      .child(quiet("[rust-analyzer] indexing done: 412 files", 12.0)),
  )
}

/// The outline of `blur.rs`.
fn outline() -> Element {
  scrolled(
    OUTLINE
      .iter()
      .fold(Column::new().width(FILL).padding(8.0).spacing(2.0), |column, name| {
        column.child(
          Row::new()
            .height(24.0)
            .padding_horizontal(4.0)
            .spacing(8.0)
            .align_items(Alignment::Center)
            .child(icon(icons::CODE, 14.0).color(PaletteColor::MutedForeground))
            .child(mono(name, 12.0)),
        )
      }),
  )
}

/// What the playground changes of the board's panels: the terminal's props, and `theme.rs` with changes not saved
/// and its close refused.
#[derive(Clone, Copy)]
struct Shape {
  close_button: bool,
  floatable: bool,
  hideable: bool,
  movable: bool,
  dirty: bool,
  refuse: bool,
}

/// The board's panels as a panel is unless told.
const PLAIN: Shape = Shape {
  close_button: true,
  floatable: true,
  hideable: true,
  movable: true,
  dirty: false,
  refuse: false,
};

/// The board's seven panels, the explorer holding `files`; `record` hears each close.
fn panels(files: impl Into<Element>, shape: Shape, record: Option<&Record>) -> Vec<DockPanel> {
  let mut theme = DockPanel::new("theme.rs", "theme.rs")
    .icon(icons::FILE_CODE)
    .dirty(shape.dirty)
    .child(code(&THEME));
  if shape.refuse {
    theme = theme.on_close(|event| event.prevent_default());
  }
  let panels = vec![
    DockPanel::new("explorer", "Explorer")
      .icon(icons::FOLDER)
      .min_size((160.0, 80.0))
      .child(scrolled(files)),
    DockPanel::new("blur.rs", "blur.rs")
      .icon(icons::FILE_CODE)
      .child(code(&BLUR)),
    theme,
    DockPanel::new("terminal", "Terminal")
      .icon(icons::TERMINAL)
      .close_button(shape.close_button)
      .floatable(shape.floatable)
      .hideable(shape.hideable)
      .movable(shape.movable)
      .child(code(&TERMINAL)),
    DockPanel::new("problems", "Problems")
      .icon(icons::ALERT_CIRCLE)
      .count(3)
      .child(problems()),
    DockPanel::new("output", "Output").icon(icons::LIST).child(output()),
    DockPanel::new("outline", "Outline")
      .icon(icons::LIST_TREE)
      .child(outline()),
  ];
  match record {
    Some(record) => panels
      .into_iter()
      .map(|panel| {
        let record = record.clone();
        panel.on_close(move |event| {
          let refused = if event.default_prevented() { ", refused" } else { "" };
          record(format!("on_close({:?}{refused})", event.panel()));
        })
      })
      .collect(),
    None => panels,
  }
}

/// A side as a person says it.
fn side_name(side: DockSide) -> &'static str {
  match side {
    DockSide::Left => "left",
    DockSide::Right => "right",
    DockSide::Top => "top",
    DockSide::Bottom => "bottom",
  }
}

/// Where `panel` is in `state`, as a person says it.
fn place(state: &DockState, panel: &str) -> String {
  if !state.contains(panel) {
    "closed".to_owned()
  } else if let Some(side) = state.is_hidden(panel) {
    format!("hidden {}", side_name(side))
  } else if state.is_floating(panel) {
    "floating".to_owned()
  } else if state.in_documents(panel) {
    "in the documents".to_owned()
  } else {
    "docked".to_owned()
  }
}

/// The panels of the group that holds `panel`, in order.
fn group_panels(state: &DockState, panel: &str) -> Vec<String> {
  state
    .group_of(panel)
    .map(|group| group.panels.clone())
    .unwrap_or_default()
}

/// What a change from `before` to `after` did, said short: the panel whose place changed, a maximize, a panel
/// moved to another group, a tab shown, or the splits resized.
fn change(before: &DockState, after: &DockState) -> String {
  let mut names = before.panels();
  for panel in after.panels() {
    if !names.contains(&panel) {
      names.push(panel);
    }
  }
  for panel in &names {
    let now = place(after, panel);
    if place(before, panel) != now {
      return format!("{panel:?} {now}");
    }
  }
  if before.maximized != after.maximized {
    return match &after.maximized {
      Some(panel) => format!("{panel:?} maximized"),
      None => "restored".to_owned(),
    };
  }
  let groups: Vec<(Vec<String>, Vec<String>)> = names
    .iter()
    .map(|panel| (group_panels(before, panel), group_panels(after, panel)))
    .collect();
  let kept = |(was, now): &(Vec<String>, Vec<String>)| was.iter().filter(|other| now.contains(other)).count();
  // A panel moved onto another group keeps none of the panels it was with and is with others it was not: first
  // one that left others behind, then one that was alone.
  for left in [true, false] {
    for (panel, moved) in names.iter().zip(&groups) {
      let gained = moved.1.iter().any(|other| !moved.0.contains(other));
      let lost = moved.0.iter().any(|other| !moved.1.contains(other));
      if kept(moved) == 1 && gained && lost == left {
        return format!("{panel:?} moved");
      }
    }
  }
  // A panel moved beside a group is alone in a new one.
  for (panel, moved) in names.iter().zip(&groups) {
    if moved.1.len() == 1 && moved.0.len() > 1 {
      return format!("{panel:?} moved");
    }
  }
  if groups.iter().any(|(was, now)| was != now) {
    return "tabs reordered".to_owned();
  }
  for panel in &names {
    let shown = |state: &DockState| state.group_of(panel).and_then(|group| group.shown.clone());
    if shown(before) != shown(after) && shown(after).as_deref() == Some(panel.as_str()) {
      return format!("{panel:?} shown");
    }
  }
  if before.floating != after.floating {
    return "a floating group moved".to_owned();
  }
  match (&before.root, &after.root) {
    (Some(was), Some(now)) if skeleton(was) == skeleton(now) => "resized".to_owned(),
    _ => "rearranged".to_owned(),
  }
}

/// A node of the tree without its shares: what tells a resize from a group moved.
fn skeleton(node: &DockNode) -> String {
  match node {
    DockNode::Split(split) => {
      let children: Vec<String> = split.children.iter().map(skeleton).collect();
      format!("{:?}({})", split.orientation, children.join(" "))
    }
    DockNode::Group(group) => format!("group({})", group.panels.join(" ")),
    DockNode::Documents(documents) => format!(
      "documents({})",
      documents.node.as_deref().map(skeleton).unwrap_or_default()
    ),
  }
}

/// How many panels are docked, hidden and floating, and the one maximized.
fn facts(state: &DockState) -> String {
  let hidden = state.hidden().len();
  let floating: usize = state.floating.iter().map(|float| float.group.panels.len()).sum();
  let docked = state.panels().len().saturating_sub(hidden + floating);
  let mut facts = format!("{docked} docked · {hidden} hidden · {floating} floating");
  if let Some(panel) = &state.maximized {
    facts.push_str(&format!(" · {panel} maximized"));
  }
  facts
}

/// The lines of a node of the tree, `depth` in, with its share of the split that holds it.
fn node_lines(node: &DockNode, depth: usize, share: Option<f32>, lines: &mut Vec<String>) {
  let pad = "   ".repeat(depth);
  let share = share.map(|share| format!(" · {share:.2}")).unwrap_or_default();
  match node {
    DockNode::Split(split) => {
      let kind = match split.orientation {
        Orientation::Horizontal => "row",
        _ => "column",
      };
      lines.push(format!("{pad}{kind}{share}"));
      for (child, share) in split.children.iter().zip(&split.shares) {
        node_lines(child, depth + 1, Some(*share), lines);
      }
    }
    DockNode::Group(group) => {
      let mut said = format!("{pad}group  {}{share}", group.panels.join(", "));
      for hidden in &group.hidden {
        said.push_str(&format!(" · {} hidden {}", hidden.panel, side_name(hidden.side)));
      }
      lines.push(said);
    }
    DockNode::Documents(documents) => match &documents.node {
      Some(node) => {
        lines.push(format!("{pad}documents{share}"));
        node_lines(node, depth + 1, None, lines);
      }
      None => lines.push(format!("{pad}documents{share} · empty")),
    },
  }
}

/// What a state is built of, a line for each node, then what floats and what is maximized.
fn state_lines(state: &DockState) -> Vec<String> {
  let mut lines = Vec::new();
  match &state.root {
    Some(root) => node_lines(root, 0, None, &mut lines),
    None => lines.push("nothing docked".to_owned()),
  }
  for float in &state.floating {
    let (x, y) = float.position;
    lines.push(format!("float  {} at {x}, {y}", float.group.panels.join(", ")));
  }
  if let Some(panel) = &state.maximized {
    lines.push(format!("maximized  {panel}"));
  }
  lines
}

/// The keys, a row each: the keys drawn, and what they do.
fn keys_list() -> Column {
  KEYS
    .iter()
    .fold(Column::new().width(FILL).spacing(10.0), |list, (keys, does)| {
      let chips = keys.iter().enumerate().fold(
        Row::new().spacing(6.0).align_items(Alignment::Center),
        |chips, (index, key)| {
          let chips = if index > 0 {
            chips.child(quiet("or", 12.0))
          } else {
            chips
          };
          chips.child(Kbd::shortcut(key))
        },
      );
      list.child(
        Row::new()
          .width(FILL)
          .align_items(Alignment::Center)
          .child(Row::new().width(220.0).child(chips))
          .child(Column::new().flex(1.0).child(quiet(does, 13.0))),
      )
    })
}

/// A layout on a demo's stage: as wide as the stage, `height` high.
fn room(layout: impl Into<Element>, height: f32) -> Column {
  Column::new()
    .width(FILL)
    .height(height)
    .align_items(Alignment::Stretch)
    .child(layout)
}

pub struct DockLayoutPage {
  frame: Frame,
  playground: Playground,
  /// The playground's arrangement.
  state: Signal<DockState>,
  close_button: Signal<bool>,
  floatable: Signal<bool>,
  hideable: Signal<bool>,
  movable: Signal<bool>,
  dirty: Signal<bool>,
  refuse: Signal<bool>,
  /// Whether the playground starts with nothing in its area of documents.
  empty: Signal<bool>,
  /// What is selected and open in the playground's explorer.
  play_selected: Signal<Option<Id>>,
  play_open: Signal<Vec<Id>>,
  /// The board's window: its arrangement, and what is selected and open in its explorer.
  window: Signal<DockState>,
  selected: Signal<Option<Id>>,
  open: Signal<Vec<Id>>,
  /// The layout that is stored, and the text it was last stored as.
  stored: Signal<DockState>,
  saved: Signal<Option<String>>,
  /// The layout with a panel floating.
  floated: Signal<DockState>,
  /// The layout whose area of documents is empty.
  emptied: Signal<DockState>,
}

impl DockLayoutPage {
  /// The shape of the board's panels the playground's controls make.
  fn shape(&self) -> Shape {
    Shape {
      close_button: self.close_button.get(),
      floatable: self.floatable.get(),
      hideable: self.hideable.get(),
      movable: self.movable.get(),
      dirty: self.dirty.get(),
      refuse: self.refuse.get(),
    }
  }

  /// The playground: the board's layout as the controls make its panels, live on the stage.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let shape = self.shape();
    let empty = self.empty.get();
    let before = self.state.get();
    let record: Record = Arc::new(self.playground.recorder());
    let (selected, open) = (self.play_selected.clone(), self.play_open.clone());
    let plain = {
      let (state, selected, open) = (self.state.clone(), selected.clone(), open.clone());
      move |record: Option<&Record>| {
        let mut layout = DockLayout::new(state.clone(), panels(tree(&selected, &open), shape, record))
          .id("play-dock")
          .key("play-dock");
        if empty {
          layout = layout.empty(
            EmptyState::new("No file open")
              .description("Open one from the explorer.")
              .icon(icons::FILE_CODE)
              .size(EmptyStateSize::Sm),
          );
        }
        layout
      }
    };
    let changed = record.clone();
    let detached = record.clone();
    let layout = plain(Some(&record))
      .on_change(move |after| {
        changed(format!(
          "on_change: {} · {} chars",
          change(&before, after),
          after.to_text().len()
        ))
      })
      .on_detach(move |panel| detached(format!("on_detach({panel:?})")));
    let shown = Shown::built(move |ctx| room(layout.mount(ctx), PLAY_HEIGHT).into());

    let mut code = vec![
      "let layout = ctx.signal({".to_owned(),
      "    let mut state = DockState::new(".to_owned(),
      "        DockNode::row([".to_owned(),
      "            DockNode::group([\"explorer\"]),".to_owned(),
      "            DockNode::column([".to_owned(),
      if empty {
        "                DockNode::documents(None),".to_owned()
      } else {
        "                DockNode::documents(DockNode::group([\"blur.rs\", \"theme.rs\"])),".to_owned()
      },
      "                DockNode::group([\"terminal\", \"problems\", \"output\"]),".to_owned(),
      "            ])".to_owned(),
      "            .shares([0.7, 0.3]),".to_owned(),
      "        ])".to_owned(),
      "        .shares([0.25, 0.75]),".to_owned(),
      "    );".to_owned(),
      "    state.show(\"outline\", DockPlace::Hidden(DockSide::Right));".to_owned(),
      "    state".to_owned(),
      "});".to_owned(),
      "DockLayout::new(layout, [".to_owned(),
      "    DockPanel::new(\"explorer\", \"Explorer\").icon(icons::FOLDER).min_size((160.0, 80.0)).child(files),"
        .to_owned(),
      "    DockPanel::new(\"blur.rs\", \"blur.rs\").icon(icons::FILE_CODE).child(blur),".to_owned(),
    ];
    let mut theme = "    DockPanel::new(\"theme.rs\", \"theme.rs\").icon(icons::FILE_CODE)".to_owned();
    if shape.dirty {
      theme.push_str(".dirty(true)");
    }
    if shape.refuse {
      theme.push_str(".on_close(|event| event.prevent_default())");
    }
    theme.push_str(".child(theme),");
    code.push(theme);
    let mut terminal = "    DockPanel::new(\"terminal\", \"Terminal\").icon(icons::TERMINAL)".to_owned();
    for (on, call) in [
      (!shape.close_button, ".close_button(false)"),
      (!shape.floatable, ".floatable(false)"),
      (!shape.hideable, ".hideable(false)"),
      (!shape.movable, ".movable(false)"),
    ] {
      if on {
        terminal.push_str(call);
      }
    }
    terminal.push_str(".child(terminal),");
    code.push(terminal);
    code.push(
      "    DockPanel::new(\"problems\", \"Problems\").icon(icons::ALERT_CIRCLE).count(3).child(problems),".to_owned(),
    );
    code.push("    DockPanel::new(\"output\", \"Output\").icon(icons::LIST).child(output),".to_owned());
    code.push("    DockPanel::new(\"outline\", \"Outline\").icon(icons::LIST_TREE).child(outline),".to_owned());
    code.push("])".to_owned());
    if empty {
      for line in [
        "    .empty(",
        "        EmptyState::new(\"No file open\")",
        "            .description(\"Open one from the explorer.\")",
        "            .icon(icons::FILE_CODE)",
        "            .size(EmptyStateSize::Sm),",
        "    )",
      ] {
        code.push(line.to_owned());
      }
    }
    code.push("    .on_change(|state| store(state.to_text()))".to_owned());
    code.push("    .on_detach(|panel| open_in_a_window(panel))".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree_lines = state_lines(&self.state.get());

    let state = self.state.clone();
    let flags = [
      self.close_button.clone(),
      self.floatable.clone(),
      self.hideable.clone(),
      self.movable.clone(),
    ];
    let offs = [self.dirty.clone(), self.refuse.clone(), self.empty.clone()];
    let controls = vec![
      quiet("the Terminal panel", 11.0).into(),
      flag("close_button", &self.close_button),
      flag("floatable", &self.floatable),
      flag("hideable", &self.hideable),
      flag("movable", &self.movable),
      divider(),
      quiet("the theme.rs document", 11.0).into(),
      flag("dirty", &self.dirty),
      flag("on_close refuses", &self.refuse),
      divider(),
      quiet("the layout", 11.0).into(),
      flag("empty documents", &self.empty),
    ];
    let make = {
      let plain = plain.clone();
      Arc::new(move || room(plain(None), PLAY_HEIGHT).into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Dock Layout",
        make,
        shown,
        fill: true,
        forceable: false,
        facts: facts(&self.state.get()),
        event: "on_change / on_close / on_detach",
        code,
        tree: tree_lines,
        controls,
        changed: [
          !shape.close_button,
          !shape.floatable,
          !shape.hideable,
          !shape.movable,
          shape.dirty,
          shape.refuse,
          empty,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(true);
          }
          for flag in &offs {
            flag.set(false);
          }
          state.set(board(false));
        }),
      },
    )
  }
}

impl Component for DockLayoutPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let state = ctx.signal(board(false));
    let empty = ctx.signal(false);
    // The flag says how the layout starts: the board's again, with its documents or with none.
    let first = state.clone();
    ctx.watch(&empty, move |empty| first.set(board(*empty)));
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      state,
      close_button: ctx.signal(true),
      floatable: ctx.signal(true),
      hideable: ctx.signal(true),
      movable: ctx.signal(true),
      dirty: ctx.signal(false),
      refuse: ctx.signal(false),
      empty,
      play_selected: ctx.signal(Some("blur.rs")),
      play_open: ctx.signal(vec!["crates", "lurq"]),
      window: ctx.signal(board(false)),
      selected: ctx.signal(Some("blur.rs")),
      open: ctx.signal(vec!["crates", "lurq"]),
      stored: ctx.signal(board(false)),
      saved: ctx.signal(None),
      floated: ctx.signal(floating()),
      emptied: ctx.signal(empty_documents()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);

    let window = room(
      DockLayout::new(
        self.window.clone(),
        panels(tree(&self.selected, &self.open), PLAIN, None),
      )
      .id("board-dock")
      .key("board-dock")
      .mount(ctx),
      BOARD_HEIGHT,
    );

    let stored_text = self.stored.get().to_text();
    let saved = self.saved.get();
    let (store, restore, first) = (
      (self.stored.clone(), self.saved.clone()),
      (self.stored.clone(), self.saved.clone()),
      self.stored.clone(),
    );
    let storing = Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(room(
        DockLayout::new(self.stored.clone(), panels(file_rows(), PLAIN, None))
          .id("stored-dock")
          .key("stored-dock")
          .mount(ctx),
        SMALL_HEIGHT,
      ))
      .child(
        Column::new()
          .width(FILL)
          .padding(12.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .border(line())
          .child(mono(&stored_text, 12.0).color(PaletteColor::MutedForeground)),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Button::new("Store")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| store.1.set(Some(store.0.get().to_text()))),
          )
          .child(
            Button::new("Restore")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(saved.is_none())
              .on_click(move |_| {
                if let Some(state) = restore.1.get().as_deref().and_then(DockState::parse) {
                  restore.0.set(state);
                }
              }),
          )
          .child(
            Button::new("First layout")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::REFRESH)
              .on_click(move |_| first.set(board(false))),
          )
          .child(quiet(
            &match &saved {
              Some(text) => format!("stored: {} chars", text.len()),
              None => "nothing stored yet".to_owned(),
            },
            12.0,
          )),
      );

    let floated = room(
      DockLayout::new(self.floated.clone(), panels(file_rows(), PLAIN, None))
        .id("floating-dock")
        .key("floating-dock")
        .mount(ctx),
      SMALL_HEIGHT,
    );

    let opened = self.emptied.clone();
    let emptied = room(
      DockLayout::new(self.emptied.clone(), panels(file_rows(), PLAIN, None))
        .id("empty-dock")
        .key("empty-dock")
        .empty(
          EmptyState::new("No file open")
            .description("Open a file from the explorer, or drag a panel here.")
            .icon(icons::FILE_CODE)
            .size(EmptyStateSize::Sm)
            .action(
              Button::new("Open blur.rs")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .on_click(move |_| {
                  opened.update(|state| {
                    state.show("blur.rs", DockPlace::Documents);
                  })
                }),
            ),
        )
        .mount(ctx),
      SMALL_HEIGHT,
    );

    let layout_rows = [
      (
        "new",
        "Signal<DockState>, panels",
        "",
        "A layout of the panels arranged as the signal says; what a person changes is set in it.",
      ),
      (
        "controlled",
        "DockState, panels, Fn(&DockState)",
        "",
        "A layout arranged as the app's value says; a change is told and shown once the app gives it back.",
      ),
      (
        "panel, panels",
        "DockPanel",
        "",
        "Panels after those it has. A panel the state does not name is not shown until the app shows it.",
      ),
      (
        "on_change",
        "Fn(&DockState)",
        "",
        "Called with the new arrangement when a person changes it, after the signal is set; not when the app sets \
         it.",
      ),
      (
        "on_detach",
        "Fn(&str)",
        "",
        "Called with the id of a panel let go outside the window; the layout changes nothing by itself.",
      ),
      (
        "empty",
        "impl Into<Element>",
        "nothing",
        "What the area of documents shows while no document is in it.",
      ),
      (
        "label",
        "&str",
        "\"Dock layout\"",
        "What the layout is called to tooling.",
      ),
      (
        "maximize_label, restore_label, close_label, more_label, float_label, hide_label, put_back_label",
        "&str",
        "\"Maximize\", …",
        "The names of its buttons and its menu's items, in the app's words.",
      ),
      (
        "move_left_label, move_right_label, move_top_label, move_bottom_label, strip_label",
        "&str",
        "\"Move to the left\", …",
        "The menu's moves to each edge, and what a strip of hidden panels is called.",
      ),
      (
        "id",
        "&str",
        "",
        "The id of its root; its parts are found by ids after it: {id}-tab-{panel}, {id}-group-{panel}, \
         {id}-strip-{panel}, {id}-split-{path}.",
      ),
      (
        "key",
        "&str",
        "\"dock-layout\"",
        "What tells it apart from another layout mounted beside it.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Mounts it: it keeps what each panel holds while it moves, what is dragged, and what is shown from a strip. \
         Unmounted it is a still layout.",
      ),
    ];
    let panel_rows = [
      (
        "new",
        "&str, &str",
        "",
        "A panel by its id, which the state names it by, and its title.",
      ),
      (
        "child, children",
        "impl Into<Element>",
        "",
        "What it holds. It is kept while the panel moves: what is typed, scrolled or open in it.",
      ),
      (
        "icon",
        "icon",
        "",
        "On its button in a strip, and before its title in the documents' tabs.",
      ),
      (
        "count",
        "impl Into<Option<u32>>",
        "None",
        "A count after its title in a tab of capitals (Problems 3).",
      ),
      (
        "dirty",
        "bool",
        "false",
        "Changes not saved: a dot on its tab in the area of documents.",
      ),
      (
        "close_button",
        "bool",
        "true",
        "Its close control, and its menu's Close.",
      ),
      (
        "floatable",
        "bool",
        "true",
        "Whether it may float: let go over nothing, or from its menu.",
      ),
      (
        "hideable",
        "bool",
        "true",
        "Whether it may be hidden into a strip at an edge.",
      ),
      (
        "movable",
        "bool",
        "true",
        "Whether a person may move it at all: drag it, float it, move it from its menu.",
      ),
      (
        "min_size",
        "(f32, f32)",
        "120 × 80",
        "The least size the splits that hold it keep.",
      ),
      (
        "on_close",
        "Fn(&DockCloseEvent)",
        "",
        "Called when its close is pressed, before it is taken out of the state; event.prevent_default() keeps it.",
      ),
      ("label", "&str", "its title", "What it is called to tooling."),
    ];
    let state_rows = [
      (
        "DockState::new",
        "DockNode",
        "",
        "A state of a tree docked, nothing floating and nothing maximized. DockState::empty is nothing.",
      ),
      (
        "DockNode::row, column",
        "nodes",
        "equal shares",
        "A split of its children side by side, or one above another; shares gives each its share.",
      ),
      (
        "DockNode::group",
        "ids",
        "the first shown",
        "Panels shown as tabs; shown(id) shows another.",
      ),
      (
        "DockNode::documents",
        "Option<DockNode>",
        "",
        "The area of documents, whose groups have the documents' tabs; None leaves it empty.",
      ),
      (
        "show, move_panel",
        "&str, DockPlace",
        "",
        "Puts a panel at a place: Tab { of }, Beside { of, side }, Edge(side), Documents, Float { position, size }, \
         Hidden(side).",
      ),
      (
        "close, select, put_back, maximize, restore",
        "&str",
        "",
        "Takes a panel out, shows it in its group, puts a hidden one back, fills the layout with its group, and \
         ends that.",
      ),
      (
        "contains, is_hidden, is_floating, in_documents, group_of, panels, hidden",
        "&str",
        "",
        "What the state says of a panel, and every panel it names.",
      ),
      (
        "to_text, parse",
        "String, &str",
        "",
        "The state as text and back, to store it: parse gives None for text that is not the form.",
      ),
      (
        "valid, make_valid",
        "Fn(&str) -> bool",
        "",
        "Brings a state to the rules with the panels the app knows; the layout does this itself.",
      ),
    ];

    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Dock Layout"],
        title: "Dock Layout",
        description: "The panels of an editor's window in groups of tabs and splits, which a person rearranges by \
                      dragging a tab or a header onto a group or an edge; panels float, hide into a strip, maximize \
                      and close. Its arrangement is a value the app holds, stores and restores.",
        import: Some("use lurq::ui::{DockLayout, DockNode, DockPanel, DockPlace, DockSide, DockState};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The board's layout on the stage, live: drag a tab or a header onto the middle or a side of a group, or onto \
           an edge of the layout; let it go over nothing to float it. Each tab's menu has the rest. The controls \
           change the Terminal panel and the theme.rs document.",
          playground,
        ),
        Section::new(
          "The board's window",
          "An explorer with the files, two documents, the terminal's group of three along the bottom with a count on \
           Problems, and the outline hidden in the strip at the right: a press of its button shows it over the \
           layout.",
          self.frame.demo(
            0,
            window,
            "DockLayout::new(layout, panels).mount(ctx)",
            &[
              "let layout = ctx.signal({",
              "    let mut state = DockState::new(",
              "        DockNode::row([",
              "            DockNode::group([\"explorer\"]),",
              "            DockNode::column([",
              "                DockNode::documents(DockNode::group([\"blur.rs\", \"theme.rs\"])),",
              "                DockNode::group([\"terminal\", \"problems\", \"output\"]),",
              "            ])",
              "            .shares([0.7, 0.3]),",
              "        ])",
              "        .shares([0.25, 0.75]),",
              "    );",
              "    state.show(\"outline\", DockPlace::Hidden(DockSide::Right));",
              "    state",
              "});",
              "DockLayout::new(layout, [",
              "    DockPanel::new(\"explorer\", \"Explorer\")",
              "        .icon(icons::FOLDER)",
              "        .min_size((160.0, 80.0))",
              "        .child(TreeView::new(selected, files).label(\"Files\").open(open)),",
              "    DockPanel::new(\"blur.rs\", \"blur.rs\").icon(icons::FILE_CODE).child(blur),",
              "    DockPanel::new(\"theme.rs\", \"theme.rs\").icon(icons::FILE_CODE).child(theme),",
              "    DockPanel::new(\"terminal\", \"Terminal\").icon(icons::TERMINAL).child(terminal),",
              "    DockPanel::new(\"problems\", \"Problems\").icon(icons::ALERT_CIRCLE).count(3).child(problems),",
              "    DockPanel::new(\"output\", \"Output\").icon(icons::LIST).child(output),",
              "    DockPanel::new(\"outline\", \"Outline\").icon(icons::LIST_TREE).child(outline),",
              "])",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Storing and restoring",
          "The arrangement is plain data in the app's signal. Its text form, under the layout, changes as the layout \
           does: Store keeps it, Restore reads it back with DockState::parse, and First layout puts the board's \
           back. A state read back is made valid for the panels the app gives.",
          self.frame.demo(
            1,
            storing,
            "DockState::parse(&state.to_text())",
            &[
              "let layout = ctx.signal(board());",
              "Column::new()",
              "    .child(DockLayout::new(layout.clone(), panels).mount(ctx))",
              "    .child(mono(&layout.get().to_text()))",
              "    .child(Button::new(\"Store\").on_click(move |_| saved.set(Some(layout.get().to_text()))))",
              "    .child(Button::new(\"Restore\").on_click(move |_| {",
              "        if let Some(state) = saved.get().as_deref().and_then(DockState::parse) {",
              "            layout.set(state);",
              "        }",
              "    }))",
              "    .child(Button::new(\"First layout\").icon(icons::REFRESH).on_click(move |_| layout.set(board())))",
            ],
          ),
        ),
        Section::new(
          "Floating",
          "A panel placed with DockPlace::Float floats over the layout in a window of its own: its title bar moves it, \
           its edges resize it, and dragged back onto a group it docks again.",
          self.frame.demo(
            2,
            floated,
            "state.show(\"outline\", DockPlace::Float { .. })",
            &[
              "let layout = ctx.signal({",
              "    let mut state = DockState::new(",
              "        DockNode::row([",
              "            DockNode::group([\"explorer\"]),",
              "            DockNode::documents(DockNode::group([\"blur.rs\", \"theme.rs\"])),",
              "        ])",
              "        .shares([0.3, 0.7]),",
              "    );",
              "    state.show(\"outline\", DockPlace::Float { position: (360.0, 48.0), size: Some((220.0, 170.0)) });",
              "    state",
              "});",
              "// The panels the state does not name (the terminal's three) are not shown.",
              "DockLayout::new(layout, panels).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "The keys",
          "Everything a drag does has a key: the focus goes from group to group, and a tab's menu moves its panel.",
          keys_list(),
        ),
        Section::new(
          "The empty area of documents",
          "DockNode::documents(None) is an area of documents with nothing in it, where the layout shows what empty \
           gives. Closing the last document leaves it so; a document opened puts it back.",
          self.frame.demo(
            3,
            emptied,
            "DockLayout::new(layout, panels).empty(EmptyState::new(\"No file open\"))",
            &[
              "let layout = ctx.signal(DockState::new(",
              "    DockNode::row([DockNode::group([\"explorer\"]), DockNode::documents(None)]).shares([0.3, 0.7]),",
              "));",
              "let opened = layout.clone();",
              "DockLayout::new(layout, panels)",
              "    .empty(",
              "        EmptyState::new(\"No file open\")",
              "            .description(\"Open a file from the explorer, or drag a panel here.\")",
              "            .icon(icons::FILE_CODE)",
              "            .size(EmptyStateSize::Sm)",
              "            .action(Button::new(\"Open blur.rs\").variant(ButtonVariant::Outline).on_click(move |_| {",
              "                opened.update(|state| {",
              "                    state.show(\"blur.rs\", DockPlace::Documents);",
              "                })",
              "            })),",
              "    )",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "DockLayout props",
          "What a layout is given.",
          props_table(&layout_rows, &["new"], self.frame.room()),
        ),
        Section::new(
          "DockPanel props",
          "What a panel is given.",
          props_table(&panel_rows, &["new"], self.frame.room()),
        ),
        Section::new(
          "DockState",
          "What builds an arrangement, changes it and reads it.",
          props_table(&state_rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
