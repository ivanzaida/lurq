//! A command bar: its tabs and its groups against the board, a large and a
//! small command, a stack, a toggled command, the line after the last group,
//! the tabs selecting the commands, what does not fit and the secondary
//! commands behind the chevron, and a bar that takes no input.

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::Signal,
  node::color::Color,
  ui::{CommandBar, CommandBarCommand, CommandBarTab, Toggle, ToolbarCommand, ToolbarGroup, ToolbarItem, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// A command that counts its runs in `runs`.
fn command(label: &str, icon: &str, runs: &Arc<AtomicUsize>) -> CommandBarCommand {
  let runs = runs.clone();
  CommandBarCommand::new(label)
    .icon(icon)
    .id(label.to_lowercase().replace(' ', "-"))
    .on_select(move || {
      runs.fetch_add(1, Ordering::SeqCst);
    })
}

/// The board's Home tab: Clipboard (Paste, Cut over Copy), Format (Bold on,
/// Italic), Insert (Image, Table on), View (Sidebar, Zoom in over Zoom out).
fn home(runs: &Arc<AtomicUsize>, table: &Arc<AtomicBool>) -> CommandBarTab<usize> {
  let table = table.clone();
  CommandBarTab::new(0, "Home")
    .id("home")
    .item(
      ToolbarGroup::new()
        .label("Clipboard")
        .item(command("Paste", icons::PASTE, runs).large(true))
        .item(ToolbarItem::stack([
          command("Cut", icons::CUT, runs),
          command("Copy", icons::COPY, runs),
        ])),
    )
    .item(
      ToolbarGroup::new()
        .label("Format")
        .item(
          Toggle::new("Bold")
            .icon(icons::BOLD)
            .icon_only(true)
            .on(true)
            .id("bold"),
        )
        .item(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).id("italic")),
    )
    .item(
      ToolbarGroup::new()
        .label("Insert")
        .item(command("Image", icons::IMAGE, runs).large(true))
        .item(
          command("Table", icons::TABLE, runs)
            .large(true)
            .on(true)
            .on_change(move |on| table.store(on, Ordering::SeqCst)),
        ),
    )
    .item(
      ToolbarGroup::new()
        .label("View")
        .item(command("Sidebar", icons::PANEL_LEFT, runs).large(true))
        .item(ToolbarItem::stack([
          command("Zoom in", icons::ZOOM_IN, runs),
          command("Zoom out", icons::ZOOM_OUT, runs),
        ])),
    )
}

/// The bar of `tabs` bound to `tab`, `width` wide, in a window of 900 by 300.
fn stage(
  width: f32,
  tab: Signal<usize>,
  tabs: impl Fn() -> Vec<CommandBarTab<usize>> + Send + Sync + 'static,
  adjust: impl Fn(CommandBar<usize>) -> CommandBar<usize> + Send + Sync + 'static,
) -> Stage {
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(900.0, 300.0)
      .padding(20.0)
      .child(
        Column::new()
          .width(width)
          .child(adjust(CommandBar::new(tab.clone(), tabs()).id("cb")).mount(ctx)),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      900.0, 300.0,
    ))));
  stage.pass();
  stage
}

fn board(runs: &Arc<AtomicUsize>, table: &Arc<AtomicBool>, tab: Signal<usize>, width: f32) -> Stage {
  let (runs, table) = (runs.clone(), table.clone());
  stage(
    width,
    tab,
    move || vec![home(&runs, &table), CommandBarTab::new(1, "Insert").id("insert")],
    |bar| bar,
  )
}

#[test]
fn its_tabs_stand_over_its_groups_as_the_board_draws_them() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let mut stage = board(&runs, &table, Signal::new(0), 860.0);
  let (_, bar_y, bar_width, _) = stage.bounds("cb");
  assert_eq!(bar_width, 860.0);
  let (_, tabs_y, _, tabs_height) = stage.bounds("cb-tabs");
  assert_eq!(tabs_height, 32.0, "a row of 32");
  assert!(
    close(tabs_y, bar_y),
    "at the top of the frame, its edge drawn over them"
  );
  assert_eq!(stage.fill_color("cb-tabs"), Some(light(PaletteColor::Muted)));
  // The selected tab's label is in the foreground, the others in the muted foreground, all of 13.
  assert_eq!(stage.text("Home"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Insert").0, light(PaletteColor::MutedForeground));

  // A large command is 56 by 52; a small one is 24 high, the one under it 2 lower.
  let (paste_x, paste_y, paste_width, paste_height) = stage.bounds("paste");
  assert_eq!((paste_width, paste_height), (56.0, 52.0));
  let (_, bar_row_y, ..) = stage.bounds("cb-bar");
  assert!(close(paste_y - bar_row_y, 8.0), "padded 8 under the tabs");
  let (cut_x, cut_y, _, cut_height) = stage.bounds("cut");
  let (_, copy_y, ..) = stage.bounds("copy");
  assert_eq!(cut_height, 24.0);
  assert!(close(copy_y - (cut_y + 24.0), 2.0), "Copy 2 under Cut");
  assert!(close(cut_x - (paste_x + 56.0), 4.0), "4 after Paste");
  // Paste's label is 11 and Cut's 12, in the foreground; a group's label is 10 in the muted foreground.
  assert_eq!(stage.text("Paste"), (light(PaletteColor::Foreground), 11.0));
  assert_eq!(stage.text("Cut"), (light(PaletteColor::Foreground), 12.0));
  assert_eq!(stage.text("Clipboard"), (light(PaletteColor::MutedForeground), 10.0));
  // The icon-alone toggles are the bar's size: 28.
  assert_eq!(stage.bounds("bold").2, 28.0);
  // Everything fits at the board's width, and nothing is behind the chevron.
  assert_eq!(stage.bounds("zoom-out").2 > 0.0, true);
}

#[test]
fn a_line_stands_after_each_group_but_the_last() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let stage = board(&runs, &table, Signal::new(0), 860.0);
  let border = light(PaletteColor::Border);
  let ends = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.border.as_ref().is_some_and(|sides| {
        sides
          .right
          .as_ref()
          .is_some_and(|side| side.width > 0.0 && side.color == border)
          && sides.left.as_ref().is_none_or(|side| side.width == 0.0)
      })
    })
    .count();
  assert_eq!(ends, 3, "Clipboard, Format and Insert, not View");
}

#[test]
fn no_line_stands_before_the_chevron_where_the_groups_after_went() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let mut stage = board(&runs, &table, Signal::new(0), 340.0);
  stage.pass();
  assert_eq!(stage.bounds("zoom-in").2, 0.0);
  assert_eq!(stage.bounds("image").2, 0.0, "Insert went too");
  let border = light(PaletteColor::Border);
  let shown = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .filter(|quad| {
        quad.border.as_ref().is_some_and(|sides| {
          sides
            .right
            .as_ref()
            .is_some_and(|side| side.width > 0.0 && side.color == border)
            && sides.left.as_ref().is_none_or(|side| side.width == 0.0)
        })
      })
      .count()
  };
  let (_, _, image, _) = stage.bounds("image");
  let groups_left = if image > 0.0 { 3 } else { 2 };
  assert_eq!(
    shown(&stage),
    groups_left - 1,
    "a line between the groups shown, none after the last of them"
  );
}

#[test]
fn a_toggled_command_is_picked_out_and_a_press_tells_the_other_value() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let mut stage = board(&runs, &table, Signal::new(0), 860.0);
  assert_eq!(stage.fill_color("table"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("image"), None, "no fill at rest");
  // To what reads the tree it is pressed.
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    let pressed = stage
      .tree
      .get_element_by_id_mut("table")
      .and_then(|element| element.description("a11y_pressed"));
    assert_eq!(pressed.as_deref(), Some("true"));
  }
  stage.click("table");
  assert!(!table.load(Ordering::SeqCst), "told off");
  assert_eq!(runs.load(Ordering::SeqCst), 1, "and its on_select ran");
}

#[test]
fn a_tab_selected_shows_its_own_commands() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let tab = Signal::new(0usize);
  let (r, t) = (runs.clone(), table.clone());
  let mut stage = stage(
    860.0,
    tab.clone(),
    move || {
      vec![
        home(&r, &t),
        CommandBarTab::new(1, "Insert").id("insert").item(
          ToolbarGroup::new()
            .label("Media")
            .item(command("Video", icons::IMAGE, &r).large(true)),
        ),
      ]
    },
    |bar| bar,
  );
  assert!(stage.paints("Clipboard") && !stage.paints("Media"));
  stage.click("insert");
  assert_eq!(tab.get(), 1);
  assert!(stage.paints("Media") && !stage.paints("Clipboard"));
  stage.click("video");
  assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[test]
fn what_does_not_fit_goes_behind_the_chevron_a_group_whole_under_its_label() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let mut stage = board(&runs, &table, Signal::new(0), 420.0);
  assert_eq!(stage.bounds("zoom-in").2, 0.0, "View went into the menu");
  assert!(stage.bounds("paste").2 > 0.0, "Clipboard stayed");
  // The chevron is at the far end of the bar, the room of what went the spacer's.
  let (bar_x, _, bar_width, _) = stage.bounds("cb-bar");
  let (more_x, more_y, more_width, more_height) = stage.bounds("cb-bar-more");
  assert!(
    close(more_x + more_width, bar_x + bar_width - 4.0),
    "at the end, inside the padding"
  );
  // The chevron is as high as a row of large commands, as the board draws it.
  assert_eq!((more_width, more_height), (28.0, 52.0));
  let (_, paste_y, ..) = stage.bounds("paste");
  assert!(close(more_y, paste_y), "level with Paste");
  stage.click("cb-bar-more");
  assert!(stage.paints("View"), "under its label in the menu");
  stage.click("zoom-in-item");
  assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[test]
fn a_secondary_command_is_only_behind_the_chevron_which_is_then_always_there() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let customized = Arc::new(AtomicUsize::new(0));
  let held = customized.clone();
  let (r, t) = (runs.clone(), table.clone());
  let mut stage = stage(
    860.0,
    Signal::new(0),
    move || {
      let held = held.clone();
      vec![
        home(&r, &t).secondary_command(ToolbarCommand::new("Customize").id("customize").on_select(move || {
          held.fetch_add(1, Ordering::SeqCst);
        })),
      ]
    },
    |bar| bar,
  );
  assert!(!stage.paints("Customize"));
  assert!(
    stage.bounds("cb-bar-more").2 > 0.0,
    "the chevron, with everything fitting"
  );
  stage.click("cb-bar-more");
  assert!(stage.paints("Customize"));
  stage.click("customize-item");
  assert_eq!(customized.load(Ordering::SeqCst), 1);
}

#[test]
fn a_disabled_bar_runs_none_of_its_commands() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let (r, t) = (runs.clone(), table.clone());
  let mut stage = stage(
    860.0,
    Signal::new(0),
    move || vec![home(&r, &t)],
    |bar| bar.disabled(true),
  );
  stage.click("paste");
  stage.click("cut");
  assert_eq!(runs.load(Ordering::SeqCst), 0);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn it_is_a_group_of_tabs_and_a_toolbar_called_by_the_selected_tab() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  let (r, t) = (runs.clone(), table.clone());
  let mut stage = stage(
    860.0,
    Signal::new(0),
    move || vec![home(&r, &t)],
    |bar| bar.label("Ribbon"),
  );
  let describe = |stage: &mut Stage, id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(describe(&mut stage, "cb", "a11y_role").as_deref(), Some("group"));
  assert_eq!(describe(&mut stage, "cb", "a11y_name").as_deref(), Some("Ribbon"));
  assert_eq!(describe(&mut stage, "cb-bar", "a11y_role").as_deref(), Some("toolbar"));
  assert_eq!(describe(&mut stage, "cb-bar", "a11y_name").as_deref(), Some("Home"));
  assert_eq!(describe(&mut stage, "paste", "a11y_name").as_deref(), Some("Paste"));
  assert_eq!(
    describe(&mut stage, "cb-bar-more", "a11y_name").as_deref(),
    Some("More commands")
  );
}

/// The board's groups by the ids of their commands, and their labels.
const GROUPS: [(&str, &[&str]); 4] = [
  ("Clipboard", &["paste", "cut", "copy"]),
  ("Format", &["bold", "italic"]),
  ("Insert", &["image", "table"]),
  ("View", &["sidebar", "zoom-in", "zoom-out"]),
];

/// Where a group's commands are laid out, from its first start to its last end; `None` where it went into the menu.
fn group_span(stage: &mut Stage, ids: &[&str]) -> Option<(f32, f32)> {
  let rects: Vec<_> = ids.iter().map(|id| stage.bounds(id)).collect();
  let shown = rects.iter().filter(|rect| rect.2 > 0.0).count();
  assert!(
    shown == 0 || shown == rects.len(),
    "a group is in the bar or in the menu whole: {ids:?} {rects:?}"
  );
  (shown > 0).then(|| {
    let start = rects.iter().map(|rect| rect.0).fold(f32::MAX, f32::min);
    let end = rects.iter().map(|rect| rect.0 + rect.2).fold(f32::MIN, f32::max);
    (start, end)
  })
}

/// The board's bar in a frame whose width is `width`, as a person drags it.
fn resizable(runs: &Arc<AtomicUsize>, table: &Arc<AtomicBool>, width: Signal<f32>) -> Stage {
  let (runs, table) = (runs.clone(), table.clone());
  let mut stage = Stage::new(move |ctx| {
    let tabs = vec![home(&runs, &table), CommandBarTab::new(1, "Insert").id("insert")];
    Column::new()
      .size(900.0, 600.0)
      .padding(20.0)
      .child(
        Column::new()
          .width(width.get())
          .child(CommandBar::new(Signal::new(0usize), tabs).id("cb").mount(ctx)),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      900.0, 600.0,
    ))));
  stage.pass();
  stage
}

/// Checks the bar at `width`: the groups shown are the first ones, apart from each other and from the chevron, their
/// labels apart too, and the others are in the chevron's menu.
fn check_at(stage: &mut Stage, width: f32, how: &str) {
  let (bar_x, _, bar_width, _) = stage.bounds("cb-bar");
  let spans: Vec<_> = GROUPS
    .iter()
    .map(|(label, ids)| (*label, group_span(stage, ids)))
    .collect();
  let shown: Vec<_> = spans
    .iter()
    .filter_map(|(label, span)| span.map(|span| (*label, span)))
    .collect();
  let dropped: Vec<_> = spans
    .iter()
    .filter(|(_, span)| span.is_none())
    .map(|(label, _)| *label)
    .collect();
  assert!(
    spans
      .iter()
      .skip_while(|(_, span)| span.is_some())
      .all(|(_, span)| span.is_none()),
    "{how} at {width}: the last groups go first: {spans:?}"
  );
  for pair in shown.windows(2) {
    let ((first, (_, end)), (second, (start, _))) = (pair[0], pair[1]);
    assert!(
      end <= start + 0.5,
      "{how} at {width}: {first} ends at {end}, past the start of {second} at {start}: {spans:?}"
    );
  }
  if let Some((label, (_, end))) = shown.last() {
    assert!(
      *end <= bar_x + bar_width + 0.5,
      "{how} at {width}: {label} runs past the bar's end"
    );
  }
  // Their labels stand apart too: the lowest text of each name, under the tabs (a tab is called Insert too).
  let quads = stage.quads();
  let labels: Vec<_> = shown
    .iter()
    .map(|(label, _)| {
      let quad = quads
        .iter()
        .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == label))
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap_or_else(|| panic!("{how} at {width}: {label}'s label is drawn"));
      (*label, quad.x, quad.x + quad.width)
    })
    .collect();
  for pair in labels.windows(2) {
    assert!(
      pair[0].2 <= pair[1].1 + 0.5,
      "{how} at {width}: the labels {} and {} are drawn over each other: {labels:?}",
      pair[0].0,
      pair[1].0
    );
  }
  let (more_x, _, more_width, _) = stage.bounds("cb-bar-more");
  if dropped.is_empty() {
    assert_eq!(more_width, 0.0, "{how} at {width}: no chevron while all fit");
    return;
  }
  // The chevron stands after what is shown, and what went is in its menu, each under its label.
  assert_eq!(more_width, 28.0, "{how} at {width}: the chevron is there");
  if let Some((label, (_, end))) = shown.last() {
    assert!(*end <= more_x + 0.5, "{how} at {width}: {label} runs under the chevron");
  }
  stage.click("cb-bar-more");
  for label in &dropped {
    assert!(stage.paints(label), "{how} at {width}: {label} is in the menu");
  }
  stage.key("Escape");
  stage.pass();
}

#[test]
fn at_every_width_the_groups_shown_stand_apart_and_the_rest_are_in_the_menu() {
  let (runs, table) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicBool::new(true)));
  // Built at each width, and dragged from the full width down to the chevron alone and back, 20 at a time.
  let widths: Vec<f32> = (0..=41).map(|step| 860.0 - 20.0 * step as f32).collect();
  for &width in &widths {
    let mut stage = resizable(&runs, &table, Signal::new(width));
    for _ in 0..3 {
      stage.pass();
    }
    check_at(&mut stage, width, "built");
  }
  let width = Signal::new(860.0f32);
  let mut stage = resizable(&runs, &table, width.clone());
  for &at in widths.iter().chain(widths.iter().rev()) {
    width.set(at);
    for _ in 0..3 {
      stage.pass();
    }
    check_at(&mut stage, at, "dragged");
  }
}
