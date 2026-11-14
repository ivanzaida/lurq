//! A floating panel: its title bar and body as the board draws them, a drag of its title bar and of its edges and
//! corner, kept inside its room, the keys that move and resize it, collapsing, closing and Escape, the panel pressed
//! brought to the front of a layer, the app's own place, what tooling reads, and the header of a panel on its own.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{FloatingPanel, FloatingPanelPart, FloatingPanels, MenuItem, PanelHeader, PanelPosition, PanelSize},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// A window of 600 by 400 whose content is a stack, the panel `make` builds mounted over it.
fn scene(make: impl Fn() -> FloatingPanel + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Stack::new()
      .size(600.0, 400.0)
      .child(Column::new().size(600.0, 400.0).id("content"))
      .child(make().mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      600.0, 400.0,
    ))));
  stage.tree.resize(600, 400);
  stage.pass();
  stage
}

/// A body of the board's rows.
fn rows() -> Column {
  Column::new()
    .child(Row::new().height(28.0).width(200.0))
    .child(Row::new().height(28.0).width(200.0))
}

fn drag(stage: &mut Stage, from: (f32, f32), by: (f32, f32)) {
  let (x, y) = from;
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  for step in 1..=4 {
    let t = step as f32 / 4.0;
    stage.tree.mouse_move(x + by.0 * t, y + by.1 * t);
  }
  stage.tree.mouse_up(x + by.0, y + by.1, MouseButton::Left);
  stage.pass();
}

#[test]
fn at_rest_it_is_the_boards_panel_with_its_title_bar() {
  let mut stage = scene(|| {
    FloatingPanel::new("Layers")
      .id("panel")
      .default_position((40.0, 30.0))
      .on_dismiss(|| ())
      .child(rows())
  });
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!((x, y, width), (40.0, 30.0, 240.0));
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Popover)));
  // The title bar: 30 high, its title at 12, muted until the panel is active and in the foreground once a press in
  // it makes it so (the board draws the panel in front).
  let title = stage.text_quad("Layers").unwrap();
  assert!(title.y > y && title.y < y + 30.0);
  assert_eq!(stage.text("Layers"), (light(PaletteColor::MutedForeground), 12.0));
  stage.click("panel");
  assert_eq!(stage.text("Layers"), (light(PaletteColor::Foreground), 12.0));
  let (_, _, collapse_width, collapse_height) = stage.bounds("panel-collapse");
  assert_eq!((collapse_width, collapse_height), (22.0, 22.0));
  assert!(
    stage.bounds("panel-close").0 > stage.bounds("panel-collapse").0,
    "close after collapse"
  );
  // The large shadow.
  let shadows: Vec<_> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::BoxShadow(shadow) if (quad.x, quad.y) == (x, y) => Some((shadow.offset_y, shadow.blur)),
      _ => None,
    })
    .collect();
  assert!(
    shadows.contains(&(12.0, 24.0)) && shadows.contains(&(3.0, 6.0)),
    "{shadows:?}"
  );
  // The corner grip at the bottom, its row 22 high.
  let (gx, gy, gw, gh) = stage.bounds("panel-resize");
  let (_, _, _, height) = stage.bounds("panel");
  assert!(
    close(gx + gw, x + width - 3.0) && close(gy + gh, y + height - 3.0),
    "3 in from the corner"
  );
}

#[test]
fn a_drag_of_the_title_bar_moves_it_and_keeps_it_inside_its_room() {
  let place = Signal::new((40.0, 30.0));
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (place.clone(), heard.clone());
  let mut stage = scene(move || {
    let told = told.clone();
    FloatingPanel::new("Layers")
      .id("panel")
      .position(read.clone())
      .on_position_change(move |at: PanelPosition| told.lock().unwrap().push(at))
      .child(rows())
  });
  let title = stage.text_quad("Layers").unwrap();
  drag(&mut stage, (title.x + 4.0, title.y + 4.0), (100.0, 50.0));
  assert_eq!(place.get(), (140.0, 80.0));
  assert_eq!(heard.lock().unwrap().last(), Some(&(140.0, 80.0)));
  let (x, y, ..) = stage.bounds("panel");
  assert_eq!((x, y), (140.0, 80.0));
  // Far past the window's edge: it stops at the edge.
  let title = stage.text_quad("Layers").unwrap();
  drag(&mut stage, (title.x + 4.0, title.y + 4.0), (900.0, -500.0));
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!((x + width, y), (600.0, 0.0));
  // The app setting the signal tells nobody.
  let told = heard.lock().unwrap().len();
  place.set((10.0, 10.0));
  stage.pass();
  assert_eq!(heard.lock().unwrap().len(), told);
  assert_eq!(stage.bounds("panel").0, 10.0);
}

#[test]
fn its_corner_and_its_edges_resize_it_no_smaller_than_its_least_size() {
  let size: Signal<Option<PanelSize>> = Signal::new(None);
  let read = size.clone();
  let mut stage = scene(move || {
    FloatingPanel::new("Layers")
      .id("panel")
      .default_position((100.0, 40.0))
      .size(read.clone())
      .child(rows())
  });
  let (_, _, width, height) = stage.bounds("panel");
  let (gx, gy, gw, gh) = stage.bounds("panel-resize");
  drag(&mut stage, (gx + gw / 2.0, gy + gh / 2.0), (60.0, 40.0));
  assert_eq!(size.get(), Some((width + 60.0, height + 40.0)));
  let (_, _, now_width, now_height) = stage.bounds("panel");
  assert_eq!((now_width, now_height), (width + 60.0, height + 40.0));
  // The start edge: its place and its width.
  let (x, y, ..) = stage.bounds("panel");
  drag(&mut stage, (x + 2.0, y + 60.0), (-30.0, 0.0));
  let (moved_x, _, moved_width, _) = stage.bounds("panel");
  assert_eq!((moved_x, moved_width), (x - 30.0, now_width + 30.0));
  // No smaller than 160 by 80.
  let (gx, gy, gw, gh) = stage.bounds("panel-resize");
  drag(&mut stage, (gx + gw / 2.0, gy + gh / 2.0), (-500.0, -500.0));
  assert_eq!(size.get().map(|(width, height)| (width, height)), Some((160.0, 80.0)));
}

#[test]
fn the_keys_move_it_by_its_grip_and_resize_it_by_its_corner() {
  let place = Signal::new((40.0, 30.0));
  let size: Signal<Option<PanelSize>> = Signal::new(Some((240.0, 200.0)));
  let (at, sized) = (place.clone(), size.clone());
  let mut stage = scene(move || {
    FloatingPanel::new("Layers")
      .id("panel")
      .position(at.clone())
      .size(sized.clone())
      .child(rows())
  });
  stage.key("Tab");
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("panel-grip")
  );
  stage.key("ArrowRight");
  stage
    .tree
    .key_down("ArrowDown".to_owned(), "ArrowDown".to_owned(), true, false, false);
  stage.pass();
  assert_eq!(place.get(), (50.0, 31.0), "10, and 1 with Shift");
  // On to the corner grip, past the collapse button.
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("panel-resize")
  );
  stage.key("ArrowLeft");
  assert_eq!(size.get(), Some((230.0, 200.0)));
}

#[test]
fn collapsed_it_is_its_title_bar_and_close_and_escape_dismiss_it() {
  let open = Signal::new(true);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told, closed) = (open.clone(), heard.clone(), heard.clone());
  let mut stage = scene(move || {
    let (told, closed) = (told.clone(), closed.clone());
    FloatingPanel::new("Layers")
      .id("panel")
      .open(read.clone())
      .on_open_change(move |open| told.lock().unwrap().push(format!("open {open}")))
      .on_dismiss(move || closed.lock().unwrap().push("dismiss".to_owned()))
      .child(Row::new().size(200.0, 60.0).id("body"))
  });
  stage.click("panel-collapse");
  assert!(!open.get());
  let (_, _, _, height) = stage.bounds("panel");
  assert_eq!(height, 30.0, "the title bar alone");
  assert!(stage.tree.get_element_by_id_mut("body").is_none());
  stage.click("panel-collapse");
  assert!(open.get());
  stage.click("panel-close");
  // Escape with the focus in it.
  stage.key("Tab");
  stage.key("Escape");
  assert_eq!(
    *heard.lock().unwrap(),
    ["open false", "open true", "dismiss", "dismiss"]
  );
}

#[test]
fn a_press_brings_a_panel_of_a_layer_to_the_front() {
  let mut stage = Stage::new(|ctx: &mut Ctx| -> Element {
    Stack::new()
      .size(600.0, 400.0)
      .child(Column::new().size(600.0, 400.0))
      .child(
        FloatingPanels::new()
          .panel(
            FloatingPanel::new("Layers")
              .id("layers")
              .default_position((40.0, 40.0))
              .child(rows()),
          )
          .panel(
            FloatingPanel::new("Color")
              .id("color")
              .default_position((100.0, 60.0))
              .child(rows()),
          )
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      600.0, 400.0,
    ))));
  stage.tree.resize(600, 400);
  stage.pass();
  // The last given is in front; neither is active until a press or the focus makes one so, and both titles are muted.
  assert_eq!(stage.text("Color").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  let fill_at = |stage: &Stage, id_y: f32| {
    stage
      .quads()
      .iter()
      .rposition(|quad| matches!(quad.content, QuadContent::Text { .. }) && close(quad.y, id_y))
  };
  // A press on the one behind, where the other does not cover it, brings it to the front.
  let (lx, ly, ..) = stage.bounds("layers");
  stage.tree.mouse_move(lx + 10.0, ly + 10.0);
  stage.tree.mouse_down(lx + 10.0, ly + 10.0, MouseButton::Left);
  stage.tree.mouse_up(lx + 10.0, ly + 10.0, MouseButton::Left);
  stage.pass();
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Color").0, light(PaletteColor::MutedForeground));
  // Painted after the other: in front.
  let layers = stage
    .quads()
    .iter()
    .rposition(|quad| quad.x == lx && quad.y == ly)
    .unwrap();
  let (cx, cy, ..) = stage.bounds("color");
  let color = stage
    .quads()
    .iter()
    .rposition(|quad| quad.x == cx && quad.y == cy)
    .unwrap();
  assert!(layers > color, "Layers is painted over Color");
  let _ = fill_at;
}

#[test]
fn a_place_the_app_holds_is_where_it_stands() {
  let place = Signal::new((40.0, 30.0));
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (place.clone(), asked.clone());
  let mut stage = scene(move || {
    let (set, told) = (read.clone(), told.clone());
    // The app keeps the panel on a grid of 20.
    FloatingPanel::new("Layers")
      .id("panel")
      .position_controlled(read.get(), move |(x, y)| {
        told.lock().unwrap().push((x, y));
        set.set(((x / 20.0).round() * 20.0, (y / 20.0).round() * 20.0));
      })
      .child(rows())
  });
  let title = stage.text_quad("Layers").unwrap();
  drag(&mut stage, (title.x + 4.0, title.y + 4.0), (33.0, 0.0));
  assert!(!asked.lock().unwrap().is_empty());
  assert_eq!(stage.bounds("panel").0, 80.0, "where the app put it: 73 to the grid");
}

#[test]
fn a_panel_header_on_its_own_is_the_boards() {
  let more = Signal::new(false);
  let closed = Arc::new(Mutex::new(0));
  let (open, told) = (more.clone(), closed.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .size(400.0, 300.0)
      .child(
        Column::new().width(240.0).child(
          PanelHeader::new("Explorer")
            .id("header")
            .menu(open.clone(), || [MenuItem::new("Collapse all")])
            .on_close(move || *told.lock().unwrap() += 1),
        ),
      )
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  let (_, _, width, height) = stage.bounds("header");
  assert_eq!((width, height), (240.0, 30.0));
  assert_eq!(stage.fill_color("header"), Some(light(PaletteColor::Muted)));
  assert_eq!(stage.text("EXPLORER"), (light(PaletteColor::MutedForeground), 11.0));
  let (x, ..) = stage.bounds("header");
  assert!(close(stage.text_quad("EXPLORER").unwrap().x - x, 10.0), "10 in");
}

#[test]
fn a_theme_restyles_its_parts() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<FloatingPanel>(|rules| rules.base(FloatingPanelPart::Root, |s| s.width(300.0)));
  let mut stage = Stage::in_app(app, |ctx| {
    Stack::new()
      .size(600.0, 400.0)
      .child(FloatingPanel::new("Layers").id("panel").child(rows()).mount(ctx))
      .into()
  });
  stage.tree.resize(600, 400);
  stage.pass();
  assert_eq!(stage.bounds("panel").2, 300.0);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_dialog_with_a_grip_and_buttons() {
  let mut stage = scene(|| FloatingPanel::new("Layers").id("panel").on_dismiss(|| ()).child(rows()));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "panel", "a11y_role").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "panel", "a11y_name").as_deref(), Some("Layers"));
  assert_eq!(
    said(&mut stage, "panel-grip", "a11y_name").as_deref(),
    Some("Move Layers")
  );
  assert_eq!(
    said(&mut stage, "panel-collapse", "a11y_expanded").as_deref(),
    Some("true")
  );
  assert_eq!(said(&mut stage, "panel-close", "a11y_name").as_deref(), Some("Close"));
}

#[test]
fn over_a_stack_as_high_as_its_content_a_drag_moves_it_down_too() {
  // What holds the panel has no height of its own: a stack as high as the canvas in it.
  let place = Signal::new((24.0, 24.0));
  let read = place.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(600.0, 400.0)
      .child(
        Stack::new().width(560.0).child(Column::new().size(560.0, 340.0)).child(
          FloatingPanel::new("Layers")
            .id("panel")
            .position(read.clone())
            .child(rows())
            .mount(ctx),
        ),
      )
      .into()
  });
  stage.tree.resize(600, 400);
  stage.pass();
  let title = stage.text_quad("Layers").unwrap();
  drag(&mut stage, (title.x + 4.0, title.y + 4.0), (100.0, 60.0));
  assert_eq!(place.get(), (124.0, 84.0));
}

/// A window of 600 by 400: a button over the content, then a layer of two panels, Layers under Color, each telling
/// `told` of the active panel.
fn two_panels(told: &Arc<Mutex<Vec<Option<String>>>>) -> Stage {
  let heard = told.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let heard = heard.clone();
    Stack::new()
      .size(600.0, 400.0)
      .child(
        Column::new()
          .size(600.0, 400.0)
          .id("content")
          .child(Row::new().child(lurq::components::Button::new("Before").id("before").tab_index(0))),
      )
      .child(
        FloatingPanels::new()
          .panel(
            FloatingPanel::new("Layers")
              .id("layers")
              .default_position((40.0, 60.0))
              .child(rows()),
          )
          .panel(
            FloatingPanel::new("Color")
              .id("color")
              .default_position((300.0, 60.0))
              .child(rows()),
          )
          .on_active_change(move |key| heard.lock().unwrap().push(key.map(str::to_owned)))
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      600.0, 400.0,
    ))));
  stage.tree.resize(600, 400);
  stage.pass();
  stage
}

fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn a_press_on_the_content_leaves_no_panel_active_and_a_press_in_one_makes_it_active_again() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = two_panels(&told);
  let (lx, ly, ..) = stage.bounds("layers");
  press_at(&mut stage, lx + 60.0, ly + 50.0);
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Color").0, light(PaletteColor::MutedForeground));
  // A press on the content under the panels: no panel is active, every title muted.
  press_at(&mut stage, 300.0, 350.0);
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text("Color").0, light(PaletteColor::MutedForeground));
  // The order is as it was: Layers, pressed last, is still painted after Color.
  let painted = |stage: &Stage, title: &str| {
    stage
      .quads()
      .iter()
      .rposition(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if &**text == title))
      .unwrap()
  };
  assert!(painted(&stage, "Layers") > painted(&stage, "Color"));
  // A press in a panel makes it active again; one in the other moves the activity to it.
  press_at(&mut stage, lx + 60.0, ly + 50.0);
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  let (cx, cy, ..) = stage.bounds("color");
  press_at(&mut stage, cx + 60.0, cy + 50.0);
  assert_eq!(stage.text("Color").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  assert_eq!(
    *told.lock().unwrap(),
    [
      Some("Layers".to_owned()),
      None,
      Some("Layers".to_owned()),
      Some("Color".to_owned())
    ]
  );
}

#[test]
fn the_focus_going_outside_every_panel_leaves_none_active_and_tab_into_one_makes_it_active() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = two_panels(&told);
  let tab = |stage: &mut Stage, shift: bool| {
    stage
      .tree
      .key_down("Tab".to_owned(), "Tab".to_owned(), shift, false, false);
    stage
      .tree
      .key_up("Tab".to_owned(), "Tab".to_owned(), shift, false, false);
    stage.pass();
  };
  tab(&mut stage, false);
  assert_eq!(focused(&stage).as_deref(), Some("before"));
  assert!(
    told.lock().unwrap().is_empty(),
    "the focus on the content makes no panel active"
  );
  // Tab into Layers, at its grip: it is active.
  tab(&mut stage, false);
  assert_eq!(focused(&stage).as_deref(), Some("layers-grip"));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  // Through it and out of it to the content, its last stop being the window's last: none is active.
  tab(&mut stage, false);
  tab(&mut stage, false);
  assert_eq!(focused(&stage).as_deref(), Some("layers-resize"));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  tab(&mut stage, false);
  assert_eq!(focused(&stage).as_deref(), Some("before"));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text("Color").0, light(PaletteColor::MutedForeground));
  // Shift+Tab back into it: active again.
  tab(&mut stage, true);
  assert_eq!(focused(&stage).as_deref(), Some("layers-resize"));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  assert_eq!(
    *told.lock().unwrap(),
    [Some("Layers".to_owned()), None, Some("Layers".to_owned())]
  );
}

#[test]
fn a_panel_on_its_own_is_active_by_a_press_in_it_and_not_by_one_outside_it() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = scene(move || {
    let heard = heard.clone();
    FloatingPanel::new("Layers")
      .id("panel")
      .default_position((40.0, 30.0))
      .on_active_change(move |active| heard.lock().unwrap().push(active))
      .child(rows())
  });
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  stage.click("panel");
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
  press_at(&mut stage, 500.0, 350.0);
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  assert_eq!(*told.lock().unwrap(), [true, false]);
}

#[test]
fn the_active_panel_the_app_holds_is_shown_and_set() {
  let active = Signal::new(Some(Arc::<str>::from("Color")));
  let held = active.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Stack::new()
      .size(600.0, 400.0)
      .child(Column::new().size(600.0, 400.0))
      .child(
        FloatingPanels::new()
          .panel(FloatingPanel::new("Layers").id("layers").default_position((40.0, 60.0)))
          .panel(FloatingPanel::new("Color").id("color").default_position((300.0, 60.0)))
          .active(held.clone())
          .mount(ctx),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::loose(lurq::layout::Size::new(
      600.0, 400.0,
    ))));
  stage.tree.resize(600, 400);
  stage.pass();
  assert_eq!(stage.text("Color").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Layers").0, light(PaletteColor::MutedForeground));
  press_at(&mut stage, 300.0, 350.0);
  assert_eq!(active.get_untracked(), None);
  active.set(Some(Arc::from("Layers")));
  stage.pass();
  assert_eq!(stage.text("Layers").0, light(PaletteColor::Foreground));
}
