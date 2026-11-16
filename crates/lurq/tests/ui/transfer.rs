//! A transfer: its lists, their headers and the buttons between them against the board, checking by a press, a
//! press with Shift, the header's checkbox and the keys, moving both ways, what the handlers are told and when, a
//! disabled item and a disabled transfer, the density, a long list that builds only what is near its view, a text
//! longer than its room, its narrow form chosen by its own room, the first frame, and tooling.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, SelectOption},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontWeight},
  node::{Element, color::Color},
  ui::{Transfer, TransferPart},
};

use super::stage::Stage;

type Id = &'static str;

const CHECK: &str = "\u{E017}";
const CHEVRON_DOWN: &str = "\u{E019}";
const CHEVRON_RIGHT: &str = "\u{E01B}";
const MINUS: &str = "\u{E050}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close_to(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's features: canvas, svg and router available; wgpu and winit enabled.
fn features() -> Vec<SelectOption<Id>> {
  ["canvas", "svg", "router", "wgpu", "winit"]
    .into_iter()
    .map(|id| SelectOption::new(id, id))
    .collect()
}

type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn weight_of(stage: &Stage, content: &str) -> FontWeight {
  stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if text == content => Some(style.weight),
      _ => None,
    })
    .unwrap_or_else(|| panic!("no text {content:?} was painted"))
}

/// How many glyphs `glyph` are painted.
fn glyphs(stage: &Stage, glyph: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == glyph))
    .count()
}

fn press_with_shift(stage: &mut Stage, id: &str) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, true, false, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, true, false, false);
  stage.pass();
}

fn key_with_shift(stage: &mut Stage, key: &str) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), true, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), true, false, false);
  stage.pass();
}

/// A window of 640 by 400 that renders `scene` 8 in from its corner.
fn wide(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  wide_in(App::new(), scene)
}

fn wide_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame_in(app, 640.0, 400.0, move |ctx| {
    Column::new().size(640.0, 400.0).padding(8.0).child(scene(ctx)).into()
  });
  stage.pass();
  stage
}

/// A mounted transfer of the board's features bound to `value`, which tells `told` what its handlers hear.
fn bound(value: Vec<Id>) -> (Stage, Signal<Vec<Id>>, Told) {
  let value = Signal::new(value);
  let told: Told = Arc::default();
  let (held, heard) = (value.clone(), told.clone());
  let stage = wide(move |ctx| {
    let (change, check) = (heard.clone(), heard.clone());
    let seen = held.clone();
    Transfer::new(held.clone(), features())
      .source_title("Available")
      .target_title("Enabled")
      .label("Features")
      .id("transfer")
      .on_change(move |value| {
        change
          .lock()
          .unwrap()
          .push(format!("change {value:?} (the signal has {:?})", seen.get_untracked()))
      })
      .on_checked_change(move |checked| check.lock().unwrap().push(format!("checked {checked:?}")))
      .mount(ctx)
  });
  (stage, value, told)
}

#[test]
fn the_lists_and_the_buttons_are_the_boards() {
  let (mut stage, ..) = bound(vec!["wgpu", "winit"]);
  stage.click("transfer-source-0");
  // Two lists of 190, 12 from the buttons between them, which are 32 square and 8 apart.
  let (sx, sy, sw, sh) = stage.bounds("transfer-source");
  let (ax, ay, aw, ah) = stage.bounds("transfer-add");
  let (rx, ry, rw, rh) = stage.bounds("transfer-remove");
  let (tx, ty, tw, th) = stage.bounds("transfer-target");
  // The ids are of the lists' items; each list is its header of 36 over them.
  assert_eq!((sw, aw, ah, rw, rh, tw), (190.0, 32.0, 32.0, 32.0, 32.0, 190.0));
  assert!(close_to(ax, sx + sw + 12.0) && close_to(rx, ax) && close_to(ry, ay + 32.0 + 8.0));
  assert!(close_to(tx, ax + 32.0 + 12.0));
  // The lists in the middle of each other's height: the right one, of two items, is shorter.
  assert_eq!((sh, th), (3.0 * 32.0 + 8.0, 2.0 * 32.0 + 8.0));
  assert!(close_to((sy - 36.0 + sy + sh) / 2.0, (ty - 36.0 + ty + th) / 2.0));
  // The header: 36 on the muted fill, its title at 13 and 500, its count in the mono type at 11, muted.
  let title = stage.text_quad("Available").unwrap();
  assert_eq!(stage.text("Available"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(weight_of(&stage, "Available"), FontWeight::Medium);
  assert_eq!(stage.text("1/3"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.font_of("1/3").to_lowercase().contains("mono"));
  assert!(stage.paints("0/2"));
  let header_fill = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted))
        && close_to(quad.height, 36.0)
        && close_to(quad.width, 190.0)
    })
    .expect("the header on the muted fill");
  assert!(title.y > header_fill.y && title.y + title.height < header_fill.y + 36.0);
  // The items: 32 high, 4 under the header, a checkbox 12 in and the label 8 after it, at 13.
  let (ix, iy, iw, ih) = stage.bounds("transfer-source-0");
  assert_eq!(ih, 32.0);
  assert!(close_to(iy, header_fill.y + 36.0 + 4.0), "{iy} {}", header_fill.y);
  assert!(close_to(iw, 190.0) && close_to(ix, sx));
  let label = stage.text_quad("canvas").unwrap();
  assert!(close_to(label.x, ix + 12.0 + 16.0 + 8.0), "{} {ix}", label.x);
  assert_eq!(stage.text("canvas"), (light(PaletteColor::Foreground), 13.0));
  // A checked item: its checkbox filled in the primary color with a check, its label not heavier, no highlight.
  assert_eq!(weight_of(&stage, "canvas"), FontWeight::Normal);
  stage.tree.mouse_move(630.0, 390.0);
  stage.pass();
  assert_eq!(stage.fill_color("transfer-source-0"), None);
  assert!(stage.quads().iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary))
      && close_to(quad.width, 16.0)
      && close_to(quad.x, ix + 12.0)
  }));
  assert_eq!(glyphs(&stage, CHECK), 1);
  // The buttons: the primary one with a chevron to the right, enabled; the outline one disabled.
  assert_eq!(stage.fill_color("transfer-add"), Some(light(PaletteColor::Primary)));
  assert_eq!(glyphs(&stage, CHEVRON_RIGHT), 1);
}

#[test]
fn a_press_checks_and_the_button_moves_what_is_checked() {
  let (mut stage, value, told) = bound(vec!["wgpu", "winit"]);
  stage.click("transfer-source-0");
  assert_eq!(said(&told), [r#"checked ["canvas"]"#]);
  assert!(stage.paints("1/3"));
  // A second press unchecks it, a press of another checks that one too.
  stage.click("transfer-source-0");
  stage.click("transfer-source-2");
  stage.click("transfer-source-1");
  assert_eq!(
    said(&told),
    ["checked []", r#"checked ["router"]"#, r#"checked ["router", "svg"]"#]
  );
  // The button moves them to the end of the right, in the order of the items; the value is set, then told, then
  // what is checked is emptied.
  stage.click("transfer-add");
  assert_eq!(value.get(), ["wgpu", "winit", "svg", "router"]);
  assert_eq!(
    said(&told),
    [
      r#"change ["wgpu", "winit", "svg", "router"] (the signal has ["wgpu", "winit", "svg", "router"])"#,
      "checked []"
    ]
  );
  assert!(stage.paints("0/1") && stage.paints("0/4"));
  // And back.
  stage.click("transfer-target-0");
  stage.click("transfer-remove");
  assert_eq!(value.get(), ["winit", "svg", "router"]);
  assert!(stage.paints("0/2") && stage.paints("0/3"));
  // What the app sets is shown, and told to nobody.
  said(&told);
  value.set(vec![]);
  stage.pass();
  assert!(stage.paints("0/5") && stage.paints("No items"));
  assert!(said(&told).is_empty());
}

#[test]
fn the_buttons_are_disabled_while_their_list_has_nothing_checked() {
  let (mut stage, value, told) = bound(vec!["wgpu"]);
  stage.click("transfer-add");
  stage.click("transfer-remove");
  assert_eq!(value.get(), ["wgpu"]);
  assert!(said(&told).is_empty());
  // Something checked on the right: the button that moves back works, the other still does nothing.
  stage.click("transfer-target-0");
  stage.click("transfer-add");
  assert_eq!(value.get(), ["wgpu"]);
  stage.click("transfer-remove");
  assert!(value.get().is_empty());
}

#[test]
fn a_press_with_shift_checks_a_range_and_keeps_what_else_is_checked() {
  let (mut stage, _, told) = bound(vec![]);
  stage.click("transfer-source-4");
  stage.click("transfer-source-0");
  press_with_shift(&mut stage, "transfer-source-2");
  let last = said(&told).pop().unwrap();
  assert_eq!(last, r#"checked ["winit", "svg", "router", "canvas"]"#);
}

#[test]
fn the_headers_checkbox_checks_all_that_can_be_and_is_half_checked_while_some_are() {
  let value = Signal::new(Vec::<Id>::new());
  let held = value.clone();
  let mut stage = wide(move |ctx| {
    let mut items = features();
    items[1] = SelectOption::new("svg", "svg").disabled(true);
    Transfer::new(held.clone(), items).id("transfer").mount(ctx)
  });
  stage.click("transfer-source-0");
  // Some are: the dash.
  assert_eq!(glyphs(&stage, MINUS), 1);
  stage.click("transfer-source-all");
  assert!(stage.paints("4/5"), "every item but the disabled one");
  assert_eq!(glyphs(&stage, MINUS), 0);
  // The disabled item is not checked by a press either, and does not move.
  stage.click("transfer-source-1");
  assert!(stage.paints("4/5"));
  stage.click("transfer-add");
  assert_eq!(value.get(), ["canvas", "router", "wgpu", "winit"]);
  assert!(stage.paints("svg"));
  // Unchecks them all.
  stage.click("transfer-target-all");
  stage.click("transfer-target-all");
  assert!(stage.paints("0/4"));
}

#[test]
fn the_keys_move_through_a_list_check_and_move() {
  let (mut stage, value, _) = bound(vec!["wgpu"]);
  // Tab: the header's checkbox, then the list at its first item.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("transfer-source-all"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("transfer-source-0"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("transfer-source-1"));
  // Moving the focus checks nothing; Space does.
  assert!(stage.paints("0/4"));
  stage.key(" ");
  assert!(stage.paints("1/4"));
  key_with_shift(&mut stage, "ArrowDown");
  assert!(stage.paints("2/4"));
  // On to the button, which Enter presses.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("transfer-add"));
  stage.key("Enter");
  assert_eq!(value.get(), ["wgpu", "svg", "router"]);
  // The focus stays on the button, which does nothing more and is disabled once the focus leaves it.
  assert_eq!(focused(&stage).as_deref(), Some("transfer-add"));
  stage.key("Enter");
  assert_eq!(value.get(), ["wgpu", "svg", "router"]);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("transfer-target-all"));
  key_with_shift(&mut stage, "Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("transfer-source-0"),
    "the button is passed by"
  );
}

#[test]
fn a_controlled_transfer_shows_what_the_app_answers() {
  let told: Told = Arc::default();
  let heard = told.clone();
  let mut stage = wide(move |ctx| {
    let heard = heard.clone();
    Transfer::controlled(["wgpu"], features(), move |value| {
      heard.lock().unwrap().push(format!("{value:?}"))
    })
    .id("transfer")
    .mount(ctx)
  });
  stage.click("transfer-source-0");
  stage.click("transfer-add");
  assert_eq!(said(&told), [r#"["wgpu", "canvas"]"#]);
  // The app kept its value: canvas is still on the left.
  assert!(stage.paints("0/4") && stage.paints("0/1"));
}

#[test]
fn a_disabled_transfer_takes_nothing() {
  let value = Signal::new(vec!["wgpu"]);
  let held = value.clone();
  let mut stage = wide(move |ctx| {
    Transfer::new(held.clone(), features())
      .disabled(true)
      .id("transfer")
      .mount(ctx)
  });
  stage.click("transfer-source-0");
  stage.click("transfer-source-all");
  stage.click("transfer-add");
  assert!(stage.paints("0/4"));
  assert_eq!(value.get(), ["wgpu"]);
}

#[test]
fn an_items_height_follows_the_density() {
  for (density, height) in [
    (Density::Compact, 24.0),
    (Density::Default, 32.0),
    (Density::Comfortable, 40.0),
    (Density::Touch, 44.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = wide_in(app, |_| {
      Transfer::controlled(["wgpu"], features(), |_| {}).id("transfer").into()
    });
    assert_eq!(stage.bounds("transfer-source-0").3, height, "{density:?}");
    // The header keeps its 36.
    let (_, y, ..) = stage.bounds("transfer-source-0");
    let (_, all, _, side) = stage.bounds("transfer-source-all");
    assert!(close_to(y - 4.0 - (all + side / 2.0), 18.0), "{density:?}");
  }
}

#[test]
fn a_long_list_scrolls_and_builds_only_what_is_near_its_view() {
  let items: Vec<SelectOption<u32>> = (0..1000)
    .map(|at| SelectOption::new(at, format!("item {at}")))
    .collect();
  let mut stage = wide(move |ctx| {
    Transfer::new(Signal::new(Vec::new()), items.clone())
      .id("transfer")
      .mount(ctx)
  });
  // The list is as high as eight items and the room around them, and holds the first of them.
  let (_, _, _, height) = stage.bounds("transfer-source");
  assert!(close_to(height, 264.0), "{height}");
  assert!(stage.paints("item 0"));
  assert!(!stage.paints("item 999"));
  assert!(stage.paints("0/1000"));
  // End goes to the last, which the list builds and brings into view.
  stage.click("transfer-source-0");
  stage.key("End");
  assert!(stage.paints("item 999"));
}

#[test]
fn a_title_and_a_label_longer_than_their_room_end_in_an_ellipsis() {
  let mut stage = wide(|_| {
    Transfer::controlled(
      [],
      [("long", "An item whose label is far longer than the list is wide")],
      |_: &[&str]| {},
    )
    .source_title("Features that can be turned on for this build")
    .id("transfer")
    .into()
  });
  let (x, _, width, _) = stage.bounds("transfer-source");
  for text in [
    "Features that can be turned on for this build",
    "An item whose label is far longer than the list is wide",
  ] {
    let quad = stage.cut_text_quad(text).expect(text);
    assert!(
      quad.x + quad.width <= x + width - 12.0 + 0.5,
      "{text} runs past its room"
    );
  }
  // The count keeps its room.
  let count = stage.text_quad("0/1").expect("the count");
  assert!(count.x + count.width <= x + width - 12.0 + 0.5);
}

#[test]
fn in_a_narrow_room_the_lists_are_one_over_the_other_from_the_first_frame() {
  let scene = |width: f32| {
    move |ctx: &mut Ctx| -> Element {
      Column::new()
        .size(width, 600.0)
        .child(
          Transfer::new(Signal::new(vec!["wgpu"]), features())
            .id("transfer")
            .mount(ctx),
        )
        .into()
    }
  };
  let mut stage = Stage::first_frame(300.0, 600.0, scene(300.0));
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture(), "the first frame is the settled one");
  // The lists as wide as the room, one over the other, the buttons side by side between them, chevrons down and up.
  let (_, sy, sw, sh) = stage.bounds("transfer-source");
  let (ax, ay, ..) = stage.bounds("transfer-add");
  let (rx, ry, ..) = stage.bounds("transfer-remove");
  let (_, ty, tw, _) = stage.bounds("transfer-target");
  assert_eq!((sw, tw), (300.0, 300.0));
  assert!(ay > sy + sh && ty > ay + 32.0);
  assert!(close_to(ry, ay) && close_to(rx, ax + 32.0 + 8.0));
  assert_eq!(glyphs(&stage, CHEVRON_DOWN), 1);
  // Wide again, side by side.
  let mut wide = Stage::first_frame(640.0, 600.0, scene(640.0));
  wide.frame();
  assert_eq!(wide.bounds("transfer-source").2, 190.0);
}

#[test]
fn a_theme_restyles_its_parts() {
  let mut stage = wide(|_| {
    Transfer::controlled(["wgpu"], features(), |_| {})
      .style(TransferPart::List, |s| s.width(240.0))
      .style(TransferPart::Item, |s| s.height(40.0))
      .id("transfer")
      .into()
  });
  assert_eq!(stage.bounds("transfer-source").2, 240.0);
  assert_eq!(stage.bounds("transfer-source-0").3, 40.0);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_group_of_two_listboxes_of_checked_options_and_two_buttons() {
  let (mut stage, ..) = bound(vec!["wgpu"]);
  stage.click("transfer-source-0");
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("transfer", "a11y_role").as_deref(), Some("group"));
  assert_eq!(said("transfer", "a11y_name").as_deref(), Some("Features"));
  assert_eq!(said("transfer-source", "a11y_role").as_deref(), Some("listbox"));
  assert_eq!(said("transfer-source", "a11y_name").as_deref(), Some("Available"));
  assert_eq!(said("transfer-target", "a11y_name").as_deref(), Some("Enabled"));
  assert_eq!(said("transfer-source", "a11y_multiselectable").as_deref(), Some("true"));
  assert_eq!(said("transfer-source-0", "a11y_checked").as_deref(), Some("true"));
  assert_eq!(said("transfer-source-1", "a11y_checked").as_deref(), Some("false"));
  assert_eq!(said("transfer-source-all", "a11y_checked").as_deref(), Some("mixed"));
  assert_eq!(said("transfer-add", "a11y_name").as_deref(), Some("Add"));
  assert_eq!(said("transfer-remove", "a11y_name").as_deref(), Some("Remove"));
}
