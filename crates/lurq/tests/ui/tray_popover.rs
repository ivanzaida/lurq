//! A tray popover: the board's panel, its header and status, a row with a switch bound and not, a command from
//! the pointer and the keyboard, a disabled row, the arrows, an anchored one that closes on a command, a text
//! longer than its room, the density of its rows, what it is to tooling, and a theme's rule and renderer.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{Density, PaletteColor, ShadowStyle, Theme, ThemePalette},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  node::{BoxShadowValue, color::Color},
  ui::{
    Button, Case, TrayAction, TrayPopover, TrayPopoverPart, TrayPopoverRenderer, TrayPopoverView, TrayStatusVariant,
    TrayToggle, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.75
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A filled quad `width` by `height` in `color`.
fn filled(stage: &Stage, width: f32, height: f32, color: Color) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
      && close(quad.width, width)
      && close(quad.height, height)
  })
}

/// The board's popover, its switches bound to `sync` and `paused` and its commands said in `ran`.
fn board(sync: &Signal<bool>, paused: &Signal<bool>, ran: &Arc<Mutex<Vec<&'static str>>>) -> TrayPopover {
  let say = |what: &'static str| {
    let ran = ran.clone();
    move || ran.lock().unwrap().push(what)
  };
  TrayPopover::new("lurq")
    .icon(icons::BOX)
    .status("Synced just now")
    .item(TrayToggle::new("Sync in background").bind(sync.clone()).id("sync"))
    .item(TrayToggle::new("Pause notifications").bind(paused.clone()).id("pause"))
    .separator()
    .item(TrayAction::new("Open lurq").on_select(say("open")).id("open"))
    .item(
      TrayAction::new("Preferences…")
        .on_select(say("preferences"))
        .id("preferences"),
    )
    .item(TrayAction::new("Quit").on_select(say("quit")).id("quit"))
    .id("tray")
}

/// The board's popover where it is declared.
fn stage_of(sync: &Signal<bool>, paused: &Signal<bool>, ran: &Arc<Mutex<Vec<&'static str>>>) -> Stage {
  let (sync, paused, ran) = (sync.clone(), paused.clone(), ran.clone());
  Stage::first_frame(400.0, 400.0, move |_| {
    Column::new()
      .size(400.0, 400.0)
      .padding(8.0)
      .child(board(&sync, &paused, &ran))
      .into()
  })
}

fn signals() -> (Signal<bool>, Signal<bool>, Arc<Mutex<Vec<&'static str>>>) {
  (Signal::new(true), Signal::new(false), Arc::new(Mutex::new(Vec::new())))
}

#[test]
fn a_tray_popover_is_the_boards_panel() {
  let (sync, paused, ran) = signals();
  let mut stage = stage_of(&sync, &paused, &ran);
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture(), "the first frame is the settled one");
  stage.pass();
  let (x, y, width, height) = stage.bounds("tray");
  assert_eq!(width, 280.0);
  assert!(close(height, 272.0), "the board's Popover is 272 high: {height}");
  assert_eq!(stage.fill_color("tray"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("tray")
    .and_then(|quad| quad.border)
    .and_then(|b| b.top)
    .unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Border)));
  assert_eq!(
    stage.fill("tray").and_then(|quad| quad.border_radius).unwrap().top_left,
    12.0
  );
  let rest = Theme::new()
    .resolve::<TrayPopover>(&Case::new(), None)
    .part(TrayPopoverPart::Panel)
    .rest();
  assert!(matches!(rest.shadow, Some(BoxShadowValue::Theme(ShadowStyle::Lg))));

  // The header: the app's mark of 16 on a tile of 28 in the primary color, 14 from the panel's start and 13 from
  // its top, in the middle of the text's 30; the title at 13 10 after it over the status, a dot of 6 in the success
  // color before words of 11.
  let tile = filled(&stage, 28.0, 28.0, light(PaletteColor::Primary)).expect("the tile");
  assert!(
    close(tile.x - x, 14.0) && close(tile.y - y, 13.0),
    "{} {}",
    tile.x - x,
    tile.y - y
  );
  assert_eq!(
    stage.text(&glyph(icons::BOX)),
    (light(PaletteColor::PrimaryForeground), 16.0)
  );
  assert_eq!(stage.text("lurq"), (light(PaletteColor::Foreground), 13.0));
  let title = stage.text_quad("lurq").unwrap();
  assert!(close(title.x - (tile.x + tile.width), 10.0));
  assert_eq!(
    stage.text("Synced just now"),
    (light(PaletteColor::MutedForeground), 11.0)
  );
  let (dot, status) = (
    filled(&stage, 6.0, 6.0, light(PaletteColor::Success)).expect("the dot"),
    stage.text_quad("Synced just now").unwrap(),
  );
  assert!(close(dot.x, title.x) && close(status.x - (dot.x + dot.width), 5.0));
  assert!(status.y >= title.y + title.height);

  // Two lines of 1 in the border's color, 10 from the panel's sides: under the header and between the groups.
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && close(quad.height, 1.0)
        && close(quad.width, 260.0)
    })
    .collect();
  assert_eq!(lines.len(), 2);
  assert!(close(lines[0].x - x, 10.0));

  // The rows: a switch's is 30, a command's 28, 8 apart, their words at 13 14 from the panel's start.
  let open = stage.bounds("open");
  let preferences = stage.bounds("preferences");
  assert_eq!((open.2, open.3), (260.0, 28.0));
  assert!(close(preferences.1 - (open.1 + open.3), 8.0));
  assert_eq!(stage.text("Open lurq"), (light(PaletteColor::Foreground), 13.0));
  assert!(close(stage.text_quad("Open lurq").unwrap().x - x, 14.0));
  let words = stage.text_quad("Sync in background").unwrap();
  assert_eq!(
    stage.text("Sync in background"),
    (light(PaletteColor::Foreground), 13.0)
  );
  assert!(close(words.x - x, 14.0));
  // The switch at the row's end, 14 from the panel's end, in the middle of the row's 30.
  let (sx, sy, sw, sh) = stage.bounds("sync");
  assert!(close(x + width - (sx + sw), 14.0), "{}", x + width - (sx + sw));
  assert!(close(sy + sh / 2.0, words.y + words.height / 2.0));
  let (_, py, _, ph) = stage.bounds("pause");
  assert!(close((py + ph / 2.0) - (sy + sh / 2.0), 38.0), "rows of 30, 8 apart");
  let quit = stage.bounds("quit");
  assert!(close(y + height - (quit.1 + quit.3), 10.0), "10 under the last row");
}

#[test]
fn a_command_under_the_pointer_is_in_the_highlight_and_a_press_runs_it() {
  let (sync, paused, ran) = signals();
  let mut stage = stage_of(&sync, &paused, &ran);
  stage.pass();
  assert_eq!(stage.fill_color("open"), None);
  stage.hover("open");
  assert_eq!(stage.fill_color("open"), Some(light(PaletteColor::Highlight)));
  assert_eq!(
    stage.fill("open").and_then(|quad| quad.border_radius).unwrap().top_left,
    4.0
  );
  stage.click("open");
  stage.click("quit");
  assert_eq!(*ran.lock().unwrap(), ["open", "quit"]);
}

#[test]
fn a_press_of_a_row_or_of_its_switch_turns_the_switch_once() {
  let (sync, paused, ran) = signals();
  let told = Arc::new(Mutex::new(Vec::new()));
  let held = Arc::new(Mutex::new(Vec::new()));
  let mut stage = {
    let (sync, paused, ran, told, held) = (sync.clone(), paused.clone(), ran.clone(), told.clone(), held.clone());
    Stage::new(move |_| {
      let (told, seen, held) = (told.clone(), paused.clone(), held.clone());
      Column::new()
        .size(400.0, 400.0)
        .child(
          board(&sync, &paused, &ran)
            .item(
              TrayToggle::new("Start at login")
                .bind(paused.clone())
                .on_change(move |on| told.lock().unwrap().push((on, seen.get_untracked())))
                .id("bound"),
            )
            .item(
              TrayToggle::new("Held by the app")
                .on(true)
                .on_change(move |on| held.lock().unwrap().push(on))
                .id("held"),
            ),
        )
        .into()
    })
  };
  // The switch itself.
  stage.click("sync");
  assert!(!sync.get(), "a press of the switch turns it");
  // The words of the row.
  let words = stage.text_quad("Sync in background").unwrap();
  let (wx, wy) = (words.x + 4.0, words.y + words.height / 2.0);
  stage.tree.mouse_move(wx, wy);
  stage.tree.mouse_down(wx, wy, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(wx, wy, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert!(sync.get(), "a press of the row turns it back, once");

  // Bound: the signal is set, then the handler is told the new value, and nothing when the app sets it.
  stage.click("bound");
  assert_eq!(*told.lock().unwrap(), [(true, true)]);
  paused.set(false);
  stage.pass();
  assert_eq!(told.lock().unwrap().len(), 1, "the app's own change tells nothing");

  // Not bound: told what a press asks for, and it stays as the app holds it.
  stage.click("held");
  stage.click("held");
  assert_eq!(*held.lock().unwrap(), [false, false]);
}

#[test]
fn the_keyboard_reaches_every_row_and_the_arrows_move_through_them() {
  let (sync, paused, ran) = signals();
  let mut stage = stage_of(&sync, &paused, &ran);
  stage.pass();
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("sync"));
  stage.key(" ");
  assert!(!sync.get(), "Space turns the switch");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("pause"));
  stage.key("ArrowDown");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("open"),
    "past the line, to the commands"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring from the keyboard");
  stage.key("Enter");
  assert_eq!(*ran.lock().unwrap(), ["open"]);
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("quit"), "no further than the last");
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("preferences"));
}

#[test]
fn a_disabled_row_is_faded_and_takes_nothing() {
  let ran = Arc::new(AtomicUsize::new(0));
  let turned = Arc::new(AtomicUsize::new(0));
  let mut stage = {
    let (ran, turned) = (ran.clone(), turned.clone());
    Stage::new(move |_| {
      let (ran, turned) = (ran.clone(), turned.clone());
      Column::new()
        .size(400.0, 300.0)
        .child(
          TrayPopover::new("lurq")
            .item(
              TrayToggle::new("Sync in background")
                .disabled(true)
                .on_change(move |_| {
                  turned.fetch_add(1, Ordering::SeqCst);
                })
                .id("sync"),
            )
            .item(
              TrayAction::new("Check for updates")
                .disabled(true)
                .on_select(move || {
                  ran.fetch_add(1, Ordering::SeqCst);
                })
                .id("check"),
            )
            .item(TrayAction::new("Quit").id("quit")),
        )
        .into()
    })
  };
  let words = stage.text_quad("Sync in background").unwrap();
  let (wx, wy) = (words.x + 4.0, words.y + words.height / 2.0);
  stage.tree.mouse_move(wx, wy);
  stage.tree.mouse_down(wx, wy, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(wx, wy, lurq::app::events::MouseButton::Left);
  stage.pass();
  stage.click("sync");
  stage.click("check");
  assert_eq!((turned.load(Ordering::SeqCst), ran.load(Ordering::SeqCst)), (0, 0));
  stage.hover("check");
  assert_eq!(stage.fill_color("check"), None, "nothing under the pointer");
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("quit"),
    "Tab passes what takes nothing"
  );
}

#[test]
fn anchored_it_opens_beside_its_anchor_and_a_command_closes_it_before_it_runs() {
  let open = Signal::new(false);
  let seen = Arc::new(Mutex::new(Vec::new()));
  let dismissed = Arc::new(AtomicUsize::new(0));
  let mut stage = {
    let (open, seen, dismissed, anchor) = (open.clone(), seen.clone(), dismissed.clone(), ElementRef::new());
    Stage::new(move |_| {
      let (toggle, read, seen, dismissed) = (open.clone(), open.clone(), seen.clone(), dismissed.clone());
      Column::new()
        .size(400.0, 300.0)
        .padding(8.0)
        .child(
          Row::new()
            .width(384.0)
            .child(Button::new("Elsewhere").id("elsewhere"))
            .child(Row::new().flex(1.0))
            .child(
              Row::new().ref_element(anchor.clone()).child(
                Button::new("lurq")
                  .id("icon")
                  .on_click(move |_| toggle.update(|open| *open = !*open)),
              ),
            ),
        )
        .child(
          TrayPopover::new("lurq")
            .status("Offline")
            .status_variant(TrayStatusVariant::Warning)
            .item(TrayToggle::new("Sync in background").id("sync"))
            .item(
              TrayAction::new("Open lurq")
                .on_select(move || seen.lock().unwrap().push(read.get_untracked()))
                .id("open"),
            )
            .on_dismiss(move || {
              dismissed.fetch_add(1, Ordering::SeqCst);
            })
            .anchored(open.clone(), anchor.clone())
            .id("tray"),
        )
        .into()
    })
  };
  assert!(!stage.paints("Offline"), "closed until its signal says so");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("icon"));
  stage.key("Enter");
  stage.pass();
  assert!(open.get());
  let (ix, iy, iw, ih) = stage.bounds("icon");
  let (x, y, width, _) = stage.bounds("tray");
  assert_eq!(width, 280.0);
  assert!(close(x + width, ix + iw), "its end at the anchor's end");
  assert!(close(y - (iy + ih), 6.0), "6 under it: {}", y - (iy + ih));
  assert!(
    filled(&stage, 6.0, 6.0, light(PaletteColor::Warning)).is_some(),
    "the dot of its tone"
  );
  assert_eq!(focused(&stage).as_deref(), Some("sync"), "the focus goes into it");

  // A command closes it, then runs: the handler reads the signal closed, and the focus is back.
  stage.key("ArrowDown");
  stage.key("Enter");
  stage.pass();
  assert_eq!(*seen.lock().unwrap(), [false]);
  assert!(!open.get());
  assert_eq!(dismissed.load(Ordering::SeqCst), 0, "a command is no dismissal");
  assert_eq!(focused(&stage).as_deref(), Some("icon"));

  // Escape and a press outside dismiss it.
  open.set(true);
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert!(!open.get());
  assert_eq!(dismissed.load(Ordering::SeqCst), 1);
  open.set(true);
  stage.pass();
  stage.click("elsewhere");
  stage.pass();
  assert!(!open.get());
  assert_eq!(dismissed.load(Ordering::SeqCst), 2);
}

#[test]
fn it_is_no_wider_than_a_narrow_room_and_long_words_are_cut() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(200.0).child(
          TrayPopover::new("lurq design system gallery")
            .icon(icons::BOX)
            .status("Synced a moment ago with the remote workspace")
            .item(TrayToggle::new("Sync in the background while on battery").id("sync"))
            .item(TrayAction::new("Open the lurq design system gallery window").id("open"))
            .id("tray"),
        ),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("tray");
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "{width} wide in {room_width}"
  );
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} runs past the room"
      );
    }
  }
  for words in [
    "lurq design system gallery",
    "Synced a moment ago with the remote workspace",
    "Sync in the background while on battery",
    "Open the lurq design system gallery window",
  ] {
    assert!(
      stage.cut_text_quad(words).is_some() && !stage.paints(words),
      "{words:?} ends in an ellipsis"
    );
  }
  // The switch keeps its room.
  let (sx, _, sw, _) = stage.bounds("sync");
  assert!(sw >= 30.0 && sx + sw <= room + room_width - 13.0);
}

#[test]
fn its_rows_follow_the_density() {
  for (density, toggle, command) in [
    (Density::Compact, 22.0, 20.0),
    (Density::Default, 30.0, 28.0),
    (Density::Comfortable, 38.0, 36.0),
    (Density::Touch, 42.0, 40.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, |_| {
      Column::new()
        .size(400.0, 400.0)
        .child(
          TrayPopover::new("lurq")
            .item(TrayAction::new("Open").id("open"))
            .item(TrayToggle::new("Sync").id("sync"))
            .item(TrayAction::new("Quit").id("quit")),
        )
        .into()
    });
    let (open, quit) = (stage.bounds("open"), stage.bounds("quit"));
    assert_eq!((open.3, quit.3), (command, command), "{density:?}");
    // The row of the switch is between the two commands, 8 from each.
    assert!(
      close(quit.1 - (open.1 + open.3), 8.0 + toggle + 8.0),
      "{density:?}: {}",
      quit.1 - (open.1 + open.3)
    );
  }
}

#[test]
fn a_themes_rule_and_renderer_change_it_and_media_stands_in_place_of_the_tile() {
  let app = App::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let renderer: TrayPopoverRenderer = {
    let seen = seen.clone();
    Arc::new(move |view: TrayPopoverView| {
      seen.lock().unwrap().push((
        view.title.to_string(),
        view.status.as_deref().map(str::to_owned),
        view.anchored,
      ));
      view.render()
    })
  };
  app.theme().set_renderer::<TrayPopover>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        TrayPopover::new("lurq")
          .media(Row::new().id("mark").size(28.0, 28.0))
          .status("Idle")
          .status_variant(TrayStatusVariant::Neutral)
          .child(Row::new().id("own").size(40.0, 12.0))
          .width(240.0)
          .style(TrayPopoverPart::Panel, |s| s.background(PaletteColor::Secondary))
          .id("tray"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("tray"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.bounds("tray").2, 240.0);
  assert_eq!(
    seen.lock().unwrap().last().cloned(),
    Some(("lurq".to_owned(), Some("Idle".to_owned()), false))
  );
  assert!(
    filled(&stage, 28.0, 28.0, light(PaletteColor::Primary)).is_none(),
    "the app's own mark, and no tile"
  );
  let (tx, ..) = stage.bounds("tray");
  assert!(close(stage.bounds("mark").0 - tx, 14.0));
  assert!(
    filled(&stage, 6.0, 6.0, light(PaletteColor::MutedForeground)).is_some(),
    "a neutral status has the muted dot"
  );
  assert_eq!(stage.bounds("own").3, 12.0, "what the app puts among the rows");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn what_a_tray_popover_is_to_tooling() {
  let (sync, paused, ran) = signals();
  let mut stage = {
    let (sync, paused, ran) = (sync.clone(), paused.clone(), ran.clone());
    Stage::new(move |_| {
      Column::new()
        .size(400.0, 400.0)
        .child(board(&sync, &paused, &ran).label("lurq tray"))
        .into()
    })
  };
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("tray", "a11y_role").as_deref(), Some("group"));
  assert_eq!(said("tray", "a11y_name").as_deref(), Some("lurq tray"));
  assert_eq!(said("open", "a11y_role").as_deref(), Some("menuitem"));
  assert_eq!(said("open", "a11y_name").as_deref(), Some("Open lurq"));
  assert_eq!(said("sync", "a11y_name").as_deref(), Some("Sync in background"));
  assert_eq!(said("sync", "a11y_checked").as_deref(), Some("true"));
}
