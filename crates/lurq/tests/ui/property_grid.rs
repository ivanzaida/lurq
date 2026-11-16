//! A property grid: the board's header, categories and rows, an editor of each kind, a changed value and its reset,
//! the keys of a tree with editors in it, the divider, and its states.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{
    App,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row, SelectOption},
  core::Signal,
  node::color::Color,
  ui::{Property, PropertyCategory, PropertyGrid, PropertyGridRenderer, PropertyGridView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The board's grid: its transform and appearance, with the radius changed from its default.
#[derive(Clone)]
struct Values {
  x: Signal<f64>,
  y: Signal<f64>,
  rotation: Signal<f64>,
  radius: Signal<f64>,
  name: Signal<String>,
  blend: Signal<u8>,
  visible: Signal<bool>,
}

impl Values {
  fn new() -> Self {
    Self {
      x: Signal::new(120.0),
      y: Signal::new(64.0),
      rotation: Signal::new(0.0),
      radius: Signal::new(12.0),
      name: Signal::new("Card".to_owned()),
      blend: Signal::new(0),
      visible: Signal::new(true),
    }
  }

  fn grid(&self) -> PropertyGrid {
    PropertyGrid::new()
      .title("Properties")
      .id("grid")
      .category(
        PropertyCategory::new("Transform")
          .property(Property::xy("Position", self.x.clone(), self.y.clone()))
          .property(
            Property::number_with("Rotation", self.rotation.clone(), "°")
              .stepper(true)
              .default(0.0),
          ),
      )
      .category(
        PropertyCategory::new("Appearance")
          .property(Property::number("Radius", self.radius.clone()).default(8.0))
          .property(Property::text("Name", self.name.clone()))
          .property(Property::choice(
            "Blend",
            self.blend.clone(),
            [SelectOption::new(0u8, "Normal"), SelectOption::new(1u8, "Multiply")],
          ))
          .property(Property::toggle("Visible", self.visible.clone())),
      )
      .category(
        PropertyCategory::new("Effects")
          .default_open(false)
          .property(Property::toggle("Shadow", Signal::new(false))),
      )
  }
}

#[test]
fn the_grid_is_the_boards_header_categories_and_rows() {
  let values = Values::new();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .size(400.0, 500.0)
      .padding(8.0)
      .align_items(lurq::layout::Alignment::Start)
      .child(values.grid())
      .into()
  });
  let (x, y, width, _) = stage.bounds("grid");
  assert_eq!(width, 300.0);
  assert_eq!(
    stage.text("PROPERTIES").0,
    light(PaletteColor::MutedForeground),
    "the title in capitals"
  );
  let (cx, cy, cw, ch) = stage.bounds("grid-Transform");
  assert_eq!(
    (cx, cw, ch),
    (x, 300.0, 28.0),
    "a category of 28 across the grid, its edge drawn over it"
  );
  assert_eq!(cy, y + 30.0, "under the header of 30");
  assert_eq!(stage.fill_color("grid-Transform"), Some(light(PaletteColor::Muted)));
  assert_eq!(stage.text("Transform"), (light(PaletteColor::Foreground), 12.0));

  let (rx, ry, _, rh) = stage.bounds("grid-Position");
  assert_eq!((ry, rh), (cy + 28.0, 32.0), "a row of 32 under its category");
  assert_eq!(stage.text("Position"), (light(PaletteColor::MutedForeground), 12.0));
  let name = stage.text_quad("Position").unwrap();
  assert!((name.x - (rx + 12.0)).abs() < 1.0, "the name 12 in");
  let (ex, _, _, eh) = stage.bounds("grid-Position-editor");
  assert_eq!(
    ex,
    rx + 12.0 + 84.0 + 8.0,
    "the editors after the names' 84 and a gap of 8"
  );
  assert!(eh >= 24.0 && eh < 25.0, "an editor 24 high: {eh}");
  assert_eq!(stage.text("X").0, light(PaletteColor::Destructive));
  assert_eq!(stage.text("Y").0, light(PaletteColor::Success));
  assert!(
    stage.paints("Effects") && !stage.paints("Shadow"),
    "a closed category's rows are not built"
  );
}

#[test]
fn a_value_not_its_default_is_marked_and_its_reset_puts_it_back_and_tells() {
  let values = Values::new();
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (scene, heard) = (values.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    scene
      .grid()
      .on_change(move |id| heard.lock().unwrap().push(id.to_owned()))
      .into()
  });
  assert_eq!(
    stage.text("Radius"),
    (light(PaletteColor::Foreground), 12.0),
    "its name is the foreground's"
  );
  assert!(
    stage.tree.get_element_by_id_mut("grid-Rotation-reset").is_none(),
    "a default value has no reset"
  );
  stage.click("grid-Radius-reset");
  assert_eq!(values.radius.get(), 8.0);
  assert_eq!(*told.lock().unwrap(), ["Radius"]);
  assert!(
    stage.tree.get_element_by_id_mut("grid-Radius-reset").is_none(),
    "back at its default"
  );
  assert_eq!(stage.text("Radius").0, light(PaletteColor::MutedForeground));
}

#[test]
fn the_keys_go_between_rows_into_an_editor_and_back() {
  let values = Values::new();
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (scene, heard) = (values.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 500.0)
      .child(
        scene
          .grid()
          .on_change(move |id| heard.lock().unwrap().push(id.to_owned()))
          .mount(ctx),
      )
      .into()
  });
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("grid-Transform"),
    "Tab enters at the first row"
  );
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Rotation"));
  stage.key("Enter");
  assert_ne!(
    focused(&stage).as_deref(),
    Some("grid-Rotation"),
    "Enter gives the editor the focus"
  );
  stage.key("ArrowUp");
  assert_eq!(values.rotation.get(), 1.0, "the number input steps");
  assert_eq!(*told.lock().unwrap(), ["Rotation"]);
  stage.key("Escape");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("grid-Rotation"),
    "Escape gives it back to the row"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)));

  // Tab goes from the row into its editor.
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(values.rotation.get(), 0.0, "the number input's own Down steps it back");
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Rotation"));

  // Left goes to the category, and closes it; Right opens it.
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Transform"));
  stage.key("ArrowLeft");
  assert!(!stage.paints("Position"), "closed");
  stage.key("ArrowRight");
  assert!(stage.paints("Position"), "open again");
  // A press of a closed category opens it.
  stage.click("grid-Effects");
  assert!(stage.paints("Shadow"));
}

#[test]
fn a_text_is_set_before_the_grid_is_told() {
  let values = Values::new();
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (scene, heard, name) = (values.clone(), told.clone(), values.name.clone());
  let mut stage = Stage::new(move |ctx| {
    let (heard, name) = (heard.clone(), name.clone());
    Column::new()
      .size(400.0, 500.0)
      .child(
        scene
          .grid()
          .on_change(move |id| heard.lock().unwrap().push(format!("{id}: {}", name.get_untracked())))
          .mount(ctx),
      )
      .into()
  });
  stage.click("grid-Name");
  stage.tree.text_input_replace(0..0, "A ");
  stage.pass();
  assert_eq!(values.name.get(), "A Card");
  assert!(told.lock().unwrap().is_empty(), "a text is told as it is committed");
  stage.key("Enter");
  assert_eq!(
    *told.lock().unwrap(),
    ["Name: A Card"],
    "Enter commits it, after it is set"
  );
  stage.tree.text_input_replace(0..0, "B");
  stage.pass();
  stage.key("Tab");
  assert_eq!(
    *told.lock().unwrap(),
    ["Name: A Card", "Name: BA Card"],
    "the focus leaving commits it"
  );
  stage.key("Tab");
  assert_eq!(told.lock().unwrap().len(), 2, "nothing edited, nothing told");
}

#[test]
fn a_press_of_a_toggle_and_a_step_of_an_axis_tell_the_grid() {
  let values = Values::new();
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (scene, heard) = (values.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 500.0)
      .child(
        scene
          .grid()
          .on_change(move |id| heard.lock().unwrap().push(id.to_owned())),
      )
      .into()
  });
  stage.click("grid-Visible");
  stage.key(" ");
  assert!(!values.visible.get(), "Space turns the box the row gave the focus");
  assert_eq!(*told.lock().unwrap(), ["Visible"]);
  stage.click("grid-Position");
  stage.key("ArrowUp");
  assert_eq!(
    (values.x.get(), values.y.get()),
    (121.0, 64.0),
    "the first axis has the focus"
  );
}

#[test]
fn read_only_and_disabled_rows_change_nothing() {
  let (count, locked) = (Signal::new(3.0), Signal::new(4.0));
  let (a, b) = (count.clone(), locked.clone());
  let mut stage = Stage::new(move |_| {
    PropertyGrid::new()
      .id("grid")
      .category(
        PropertyCategory::new("Limits")
          .property(Property::number("Count", a.clone()).read_only(true))
          .property(Property::number("Locked", b.clone()).disabled(true)),
      )
      .into()
  });
  stage.click("grid-Count");
  stage.key("ArrowUp");
  stage.click("grid-Locked");
  stage.key("ArrowUp");
  assert_eq!((count.get(), locked.get()), (3.0, 4.0));
}

#[test]
fn a_long_name_is_cut_and_the_divider_moves_the_editors() {
  let value = Signal::new(1.0);
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        PropertyGrid::new()
          .id("grid")
          .category(
            PropertyCategory::new("Layout")
              .property(Property::number("The spacing between the columns", value.clone())),
          )
          .mount(ctx),
      )
      .into()
  });
  assert!(!stage.paints("The spacing between the columns"), "the name is cut");
  assert!(stage.cut_text_quad("The spacing between the columns").is_some());
  let (before, ..) = stage.bounds("grid-The spacing between the columns-editor");
  let (.., divider_height) = stage.bounds("grid-divider");
  let (.., grid_height) = stage.bounds("grid");
  assert!(
    divider_height > 0.0 && divider_height <= grid_height,
    "the divider is as high as the rows, in a grid as high as its rows: {divider_height} of {grid_height}"
  );
  stage.click("grid-divider");
  stage.key("ArrowRight");
  let (after, ..) = stage.bounds("grid-The spacing between the columns-editor");
  assert_eq!(after, before + 8.0, "Right moves the divider by 8");
}

#[test]
fn a_rows_height_follows_the_density_and_an_editor_keeps_its_own() {
  for (density, row) in [
    (Density::Compact, 24.0),
    (Density::Default, 32.0),
    (Density::Touch, 44.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let values = Values::new();
    let mut stage = Stage::in_app(app, move |_| values.grid().into());
    let (_, _, _, height) = stage.bounds("grid-Rotation");
    let (_, _, _, editor) = stage.bounds("grid-Rotation-editor");
    assert_eq!(height, row, "{density:?}");
    assert!(editor >= 24.0 && editor < 25.0, "{density:?}: {editor}");
  }
}

#[test]
fn a_theme_renderer_gives_another_anatomy() {
  let app = App::new();
  let renderer: PropertyGridRenderer = Arc::new(|view: PropertyGridView| {
    let mut column = Column::new();
    for category in &view.categories {
      for row in &category.rows {
        column = column.child(lurq::components::Text::new(&format!(
          "{}: {}",
          row.property.name(),
          row.property.value_text().unwrap_or_default()
        )));
      }
    }
    column.into()
  });
  app.theme().set_renderer::<PropertyGrid>(renderer);
  let values = Values::new();
  let stage = Stage::in_app(app, move |_| values.grid().into());
  assert!(stage.paints("Position: X 120, Y 64"));
  assert!(stage.paints("Blend: Normal"));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_treegrid_of_rows_that_say_their_values() {
  let values = Values::new();
  let mut stage = Stage::new(move |_| values.grid().into());
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("grid", "a11y_role").as_deref(), Some("treegrid"));
  assert_eq!(said("grid-Transform", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said("grid-Position", "a11y_name").as_deref(), Some("Position"));
  assert_eq!(said("grid-Position", "a11y_value").as_deref(), Some("X 120, Y 64"));
  assert_eq!(said("grid-Rotation", "a11y_value").as_deref(), Some("0°"));
  assert_eq!(said("grid-Radius-reset", "a11y_name").as_deref(), Some("Reset Radius"));
}

#[test]
fn escape_gives_the_row_the_focus_when_the_page_around_the_grid_renders_again_too() {
  let values = Values::new();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| {
    // The page shows the value, so a step renders it again with the grid.
    let shown = scene.rotation.get();
    Column::new()
      .size(400.0, 500.0)
      .child(lurq::components::Text::new(&format!("rotation {shown}")))
      .child(scene.grid().mount(ctx))
      .into()
  });
  stage.click("grid-Rotation");
  stage.key("ArrowUp");
  assert_eq!(values.rotation.get(), 1.0);
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Rotation"));
}

#[test]
fn after_escape_the_editor_paints_no_caret() {
  let values = Values::new();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| Column::new().size(400.0, 500.0).child(scene.grid().mount(ctx)).into());
  let carets = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .filter(|quad| quad.width == crate::support::caret_width() && quad.height > 8.0 && quad.border.is_none())
      .count()
  };
  let unfocused = carets(&stage);
  stage.click("grid-Radius");
  assert_eq!(carets(&stage), unfocused + 1, "the focused editor's caret");
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Radius"));
  assert_eq!(carets(&stage), unfocused, "no caret once the row has the focus");
}

#[test]
fn two_grids_of_the_same_values_paint_one_caret_and_none_after_escape() {
  let values = Values::new();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| {
    Row::new()
      .size(800.0, 500.0)
      .child(scene.grid().id("first").mount(ctx))
      .child(scene.grid().mount(ctx))
      .into()
  });
  let carets = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .filter(|quad| quad.width == crate::support::caret_width() && quad.height > 8.0 && quad.border.is_none())
      .count()
  };
  let unfocused = carets(&stage);
  stage.click("grid-Radius");
  assert_eq!(carets(&stage), unfocused + 1, "the focused editor's caret only");
  stage.key("Escape");
  assert_eq!(carets(&stage), unfocused, "no caret once the row has the focus");
}

#[test]
fn a_press_of_a_name_then_escape_leaves_no_caret() {
  let values = Values::new();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| Column::new().size(400.0, 500.0).child(scene.grid().mount(ctx)).into());
  let carets = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .filter(|quad| quad.width == crate::support::caret_width() && quad.height > 8.0 && quad.border.is_none())
      .count()
  };
  let unfocused = carets(&stage);
  let (x, y, _, height) = stage.bounds("grid-Radius");
  let (px, py) = (x + 20.0, y + height / 2.0);
  stage.tree.mouse_move(px, py);
  stage.tree.mouse_down(px, py, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(px, py, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_ne!(
    focused(&stage).as_deref(),
    Some("grid-Radius"),
    "the press of the name focused the editor"
  );
  assert_eq!(carets(&stage), unfocused + 1, "the editor's caret");
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Radius"));
  assert_eq!(carets(&stage), unfocused, "no caret once the row has the focus");
}

#[test]
fn the_color_row_is_the_boards_swatch_outside_its_hex_and_the_swatch_opens_the_picker() {
  let fill = Signal::new(Color::from_hex("#2563EB"));
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (bound, heard) = (fill.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 600.0)
      .child(
        PropertyGrid::new()
          .id("grid")
          .category(
            PropertyCategory::new("Appearance")
              .property(Property::color("Fill", bound.clone()))
              .property(Property::color("Stroke", bound.clone()).eyedropper(true)),
          )
          .on_change(move |id| heard.lock().unwrap().push(id.to_owned()))
          .mount(ctx),
      )
      .into()
  });
  let (ex, ..) = stage.bounds("grid-Fill-editor");
  let hex = stage.text_quad("#2563EB").expect("the hex in its box");
  assert!(hex.x > ex + 24.0 + 6.0, "the box after the swatch of 24 and a gap of 6");
  let eyedroppers = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "\u{E02D}"))
      .count()
  };
  assert_eq!(eyedroppers(&stage), 1, "the eyedropper only where asked");

  // A press of the swatch opens the picker: the panel and its fields are painted.
  let closed = stage.quads().len();
  let (x, y, _, h) = stage.bounds("grid-Fill-editor");
  stage.tree.mouse_move(x + 12.0, y + h / 2.0);
  stage
    .tree
    .mouse_down(x + 12.0, y + h / 2.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(x + 12.0, y + h / 2.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert!(
    stage.quads().len() > closed + 10,
    "the picker is open: {} quads, {closed} closed",
    stage.quads().len()
  );

  // Escape closes it; the hex typed and committed is the color.
  stage.key("Escape");
  assert!(stage.quads().len() <= closed + 2, "closed");
  stage.click("grid-Stroke-editor");
  stage.tree.text_input_replace(0..7, "#22C55E");
  stage.pass();
  stage.key("Enter");
  assert_eq!(fill.get(), Color::from_hex("#22C55E"));
  assert_eq!(*told.lock().unwrap(), ["Stroke"]);
}

#[test]
fn the_stepper_is_one_control_of_chevrons_whose_halves_step_the_number() {
  let rotation = Signal::new(0.0);
  let bound = rotation.clone();
  let mut stage = Stage::new(move |_| {
    PropertyGrid::new()
      .id("grid")
      .category(
        PropertyCategory::new("Transform")
          .property(Property::number_with("Rotation", bound.clone(), "°").stepper(true)),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("grid-Rotation-editor");
  let chevrons = stage.text_quad("\u{E01D}").expect("the chevrons");
  assert!(
    chevrons.x + chevrons.width <= x + width - 5.0,
    "inside the box, 6 from its end"
  );
  let at = (chevrons.x + chevrons.width / 2.0, y + height / 2.0);
  let press = |stage: &mut Stage, dy: f32| {
    let (px, py) = (at.0, at.1 + dy);
    stage.tree.mouse_move(px, py);
    stage.tree.mouse_down(px, py, lurq::app::events::MouseButton::Left);
    stage.tree.mouse_up(px, py, lurq::app::events::MouseButton::Left);
    stage.pass();
  };
  press(&mut stage, -3.0);
  assert_eq!(rotation.get(), 1.0, "the upper half steps up");
  press(&mut stage, 3.0);
  press(&mut stage, 3.0);
  assert_eq!(rotation.get(), -1.0, "the lower half steps down");
  assert!(!stage.paints("\u{E05A}"), "no number input's own buttons");
}

#[test]
fn a_long_inspector_builds_the_rows_in_view_and_the_keys_reach_every_row() {
  let values: Vec<Signal<f64>> = (0..2000).map(|index| Signal::new(index as f64)).collect();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| {
    let mut grid = PropertyGrid::new().id("grid").height(400.0);
    for category in 0..20 {
      let mut group = PropertyCategory::new(&format!("Group {category}"));
      for index in 0..100 {
        let at = category * 100 + index;
        group = group.property(Property::number(&format!("p{at}"), scene[at].clone()));
      }
      grid = grid.category(group);
    }
    Column::new().size(400.0, 500.0).child(grid.mount(ctx)).into()
  });
  let built = |stage: &mut Stage| {
    (0..2000)
      .filter(|index| stage.tree.get_element_by_id_mut(&format!("grid-p{index}")).is_some())
      .count()
  };
  let first = built(&mut stage);
  assert!(
    first > 5 && first < 60,
    "only the rows near the view are built: {first}"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Group 0"));
  stage.key("End");
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some("grid-p1999"),
    "End reaches the last row, built for it"
  );
  assert!(built(&mut stage) < 80, "still only the rows near the view");
  stage.key("Enter");
  stage.key("ArrowUp");
  assert_eq!(values[1999].get(), 2000.0, "its editor has the focus, and steps it");
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-p1999"));
  stage.key("ArrowLeft");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("grid-Group 19"),
    "Left goes to its category"
  );
  stage.key("ArrowLeft");
  stage.key("Home");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("grid-Group 0"));
  assert!(
    stage.tree.get_element_by_id_mut("grid-p1999").is_none(),
    "the closed category's rows are gone"
  );
}

#[test]
fn down_from_a_changed_row_goes_to_the_next_row_and_not_to_its_reset() {
  let values = Values::new();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| Column::new().size(400.0, 500.0).child(scene.grid().mount(ctx)).into());
  stage.click("grid-Radius");
  stage.key("Escape");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Radius"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Name"), "the next row");
}

#[test]
fn a_grid_that_scrolls_keeps_a_lane_for_its_bar_and_one_that_does_not_keeps_its_width() {
  let style = lurq::layout::scrollbar::ScrollBarStyle::default();
  let lane = style.width + style.resolved_edge_inset() * 2.0;
  let values: Vec<Signal<f64>> = (0..60).map(|index| Signal::new(index as f64)).collect();
  let (scene, few) = (values.clone(), values.clone());
  let grid = |values: &[Signal<f64>], count: usize, id: &str| {
    let mut category = PropertyCategory::new("Fields");
    for (index, value) in values.iter().take(count).enumerate() {
      let property = Property::number(&format!("f{index}"), value.clone());
      // The last row is changed, so that its reset is at the row's end too.
      category = category.property(if index + 1 == count {
        property.default(-1.0)
      } else {
        property
      });
    }
    PropertyGrid::new().id(id).category(category)
  };
  let mut stage = Stage::new(move |ctx| {
    Row::new()
      .size(800.0, 500.0)
      .align_items(lurq::layout::Alignment::Start)
      .child(grid(&scene, 60, "long").height(300.0).mount(ctx))
      .child(grid(&few, 3, "short").mount(ctx))
      .into()
  });
  let (gx, _, gw, _) = stage.bounds("long");
  let lane_starts = gx + gw - lane;
  for id in ["long-f0-editor", "long-f5-editor"] {
    let (x, _, w, _) = stage.bounds(id);
    assert!(
      x + w <= lane_starts + 0.5,
      "{id} ends at {} before the bar's lane at {lane_starts}",
      x + w
    );
  }
  // Under the pointer the bar is shown; nothing moves into its lane.
  let (cx, cy) = stage.center("long-f0");
  stage.tree.mouse_move(cx, cy);
  stage.pass();
  let (x, _, w, _) = stage.bounds("long-f0-editor");
  assert!(x + w <= lane_starts + 0.5, "with the bar shown");

  // The grid that does not scroll keeps its full width: its editor ends at its row's padding.
  let (sx, _, sw, _) = stage.bounds("short");
  let (x, _, w, _) = stage.bounds("short-f0-editor");
  assert!(
    (x + w - (sx + sw - 8.0)).abs() < 0.5,
    "no lane: the editor ends 8 from the edge"
  );
  // Its last row's reset ends at the row's padding, where a scrolling one keeps out of the lane.
  let (rx, _, rw, _) = stage.bounds("short-f2-reset");
  assert!(rx + rw <= sx + sw - 8.0 + 0.5);
}

/// Two categories of three rows: what is below the first moves as it opens and closes.
fn two_categories(height: Option<f32>) -> PropertyGrid {
  let values: Vec<Signal<f64>> = (0..6).map(|index| Signal::new(index as f64)).collect();
  let category = |title: &str, from: usize| {
    (from..from + 3).fold(PropertyCategory::new(title), |category, index| {
      category.property(Property::number(&format!("n{index}"), values[index].clone()))
    })
  };
  let grid = PropertyGrid::new()
    .id("grid")
    .category(category("First", 0))
    .category(category("Second", 3));
  match height {
    Some(height) => grid.height(height),
    None => grid,
  }
}

/// Presses a key with no pass after it: what it starts is caught on its way. The clock is set to the present first:
/// what the key starts runs from it, and not from the second the stage's last pass went on to.
fn press_key(stage: &mut Stage, key: &str) {
  stage.pass_after(Duration::ZERO);
  stage.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

fn top(stage: &mut Stage, id: &str) -> f32 {
  stage.bounds(id).1
}

/// The motion of a category in a grid of either form: it closes and opens with its height, what is below moving
/// through every place between, and turns round when it is pressed again on the way.
fn a_category_opens_and_closes_with_its_height(height: Option<f32>) {
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .child(two_categories(height).mount(ctx))
      .into()
  });
  let opened = top(&mut stage, "grid-Second");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("grid-First"));

  // Left closes it: at the start what is below has not moved, on the way it is between, at the end it is up.
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::ZERO);
  let start = top(&mut stage, "grid-Second");
  assert!(
    (start - opened).abs() < 1.0,
    "nothing jumps at the start: {opened} -> {start}"
  );
  stage.pass_after(Duration::from_millis(60));
  let on_the_way = top(&mut stage, "grid-Second");
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_secs(2));
  stage.pass();
  let closed = top(&mut stage, "grid-Second");
  assert!(closed < opened - 90.0, "three rows of 32 less: {opened} -> {closed}");
  assert!(
    on_the_way < opened - 0.5 && on_the_way > closed + 0.5,
    "on the way: {closed} < {on_the_way} < {opened}"
  );
  assert!(!stage.paints("n0"), "closed, its rows are gone");
  // Its rows are not reached once it has closed: Down goes on to the next category.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("grid-Second"));
  stage.key("ArrowUp");

  // Right opens it, and Left on the way turns it round from where it is. Rows built for it are given their height
  // on the turn of the loop after the one that laid them out at none: the timers of that turn are ticked.
  press_key(&mut stage, "ArrowRight");
  stage.pass_after(Duration::ZERO);
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_millis(1));
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let opening = top(&mut stage, "grid-Second");
  assert!(
    opening > closed + 0.5 && opening < opened - 0.5,
    "opening: {closed} < {opening} < {opened}"
  );
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::from_millis(60));
  let turned = top(&mut stage, "grid-Second");
  assert!(
    turned <= opening + 0.5,
    "turned round, it goes back up from where it was: {opening} -> {turned}"
  );
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_secs(2));
  stage.pass();
  assert!((top(&mut stage, "grid-Second") - closed).abs() < 1.0, "closed again");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("grid-First"),
    "the focus stayed on the category through it all"
  );

  // Opened to the end, its rows are there and reached again.
  stage.key("ArrowRight");
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_millis(1));
  stage.pass();
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_secs(2));
  stage.pass();
  assert!((top(&mut stage, "grid-Second") - opened).abs() < 1.0, "open again");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("grid-n0"));
}

#[test]
fn a_category_of_the_grid_that_builds_every_row_opens_and_closes_with_its_height() {
  a_category_opens_and_closes_with_its_height(None);
}

#[test]
fn a_category_of_the_grid_that_builds_the_rows_in_view_opens_and_closes_with_its_height() {
  a_category_opens_and_closes_with_its_height(Some(400.0));
}

#[test]
fn with_motion_reduced_a_category_opens_and_closes_at_once_in_either_form() {
  for height in [None, Some(400.0)] {
    let app = App::new();
    app.theme().set_reduced_motion(Some(true));
    let mut stage = Stage::in_app(app, move |ctx| {
      Column::new()
        .size(400.0, 500.0)
        .child(two_categories(height).mount(ctx))
        .into()
    });
    let opened = top(&mut stage, "grid-Second");
    stage.key("Tab");
    press_key(&mut stage, "ArrowLeft");
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::from_millis(10));
    let closed = top(&mut stage, "grid-Second");
    assert!(closed < opened - 90.0, "{height:?}: at once, {opened} -> {closed}");
    assert!(!stage.paints("n0"), "{height:?}: its rows gone at once");
  }
}

#[test]
fn the_chevron_turns_as_the_category_closes() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 500.0)
      .child(two_categories(None).mount(ctx))
      .into()
  });
  let chevron = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "\u{E019}"))
      .map(|quad| format!("{:?}", quad.transform))
  };
  let open = chevron(&stage);
  stage.key("Tab");
  press_key(&mut stage, "ArrowLeft");
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(60));
  let turning = chevron(&stage);
  stage.pass();
  let closed = chevron(&stage);
  assert_ne!(open, closed, "turned to the end");
  assert_ne!(turning, open);
  assert_ne!(turning, closed, "on its way round");
}

#[test]
fn as_a_long_category_opens_in_view_what_is_below_it_stays_painted_in_every_frame() {
  let values: Vec<Signal<f64>> = (0..200).map(|index| Signal::new(index as f64)).collect();
  let scene = values.clone();
  let mut stage = Stage::new(move |ctx| {
    let mut grid = PropertyGrid::new().id("grid").height(400.0);
    for category in 0..8 {
      let mut group = PropertyCategory::new(&format!("Group {category}")).default_open(category == 0);
      for index in 0..25 {
        let at = category * 25 + index;
        group = group.property(Property::number(&format!("p{at}"), scene[at].clone()));
      }
      grid = grid.category(group);
    }
    Column::new().size(400.0, 500.0).child(grid.mount(ctx)).into()
  });
  stage.key("Tab");
  stage.key("ArrowLeft");
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_secs(2));
  stage.pass();
  assert!(stage.paints("Group 1") && !stage.paints("p0"), "closed");
  press_key(&mut stage, "ArrowRight");
  stage.pass_after(Duration::ZERO);
  assert!(stage.paints("Group 1"), "the frame that builds its rows at no height");
  stage
    .tree
    .tick_timers_at(lurq::time::Instant::now() + Duration::from_millis(1));
  stage.pass_after(Duration::ZERO);
  assert!(
    stage.paints("Group 1"),
    "the frame that gives them their height to run to"
  );
  stage.pass_after(Duration::from_millis(30));
  assert!(stage.paints("p0"), "on the way, its first row shows");
}

/// The slider of a grid's only row, `width` wide, at `value` of 0 to 100: its track and its thumb, and the number
/// beside it.
fn slider_row(width: f32, value: f32, first: bool) -> Stage {
  let opacity = Signal::new(value);
  let scene = move |_: &mut lurq::app::ctx::Ctx| -> lurq::node::Element {
    Column::new()
      .size(width, 300.0)
      .child(
        PropertyGrid::new()
          .id("grid")
          .category(PropertyCategory::new("A").property(Property::slider("Opacity", opacity.clone(), 0.0, 100.0, "%"))),
      )
      .into()
  };
  if first {
    Stage::first_frame(width, 300.0, scene)
  } else {
    Stage::new(scene)
  }
}

/// The slider's track (the rectangle of 4 in the secondary fill), its thumb (the round one of 16 in the
/// background's) and the box of its number, by the text in it.
fn slider_parts(stage: &Stage, number: &str) -> ((f32, f32), (f32, f32), f32) {
  let quads = stage.quads();
  let rect = |color: Color, height: f32| {
    quads
      .iter()
      .find(|quad| {
        matches!(&quad.content, lurq::layout::quad::QuadContent::Rect { color: fill, .. } if *fill == color)
          && (quad.height - height).abs() < 0.01
      })
      .map(|quad| (quad.x, quad.width))
      .unwrap_or_else(|| panic!("a rectangle of {height} in {color:?}"))
  };
  let track = rect(light(PaletteColor::Secondary), 4.0);
  let thumb = rect(light(PaletteColor::Background), 16.0);
  let text = stage
    .text_quad(number)
    .unwrap_or_else(|| panic!("the number {number}"))
    .x;
  (track, thumb, text)
}

#[test]
fn a_rows_slider_keeps_its_thumb_on_its_track_and_out_of_the_number_at_any_width() {
  for width in [240.0, 300.0, 360.0, 480.0] {
    for value in [0.0, 50.0, 100.0] {
      let stage = slider_row(width, value, false);
      let number = format!("{}%", value as u32);
      let ((track_x, track_width), (thumb_x, thumb_width), text) = slider_parts(&stage, &number);
      assert!(
        thumb_x >= track_x - 0.01 && thumb_x + thumb_width <= track_x + track_width + 0.01,
        "{width} at {value}: the thumb {thumb_x}+{thumb_width} on the track {track_x}+{track_width}"
      );
      assert!(
        track_x + track_width + 8.0 <= text,
        "{width} at {value}: the track ends 8 or more before the number at {text}"
      );
      // The thumb's center is where the value is along the way it travels.
      let center = thumb_x + thumb_width / 2.0;
      let at = track_x + 8.0 + (track_width - 16.0) * value / 100.0;
      assert!(
        (center - at).abs() < 0.5,
        "{width} at {value}: the thumb at {center}, the value at {at}"
      );
    }
  }
}

#[test]
fn the_first_frame_of_a_rows_slider_is_the_settled_one() {
  for width in [240.0, 300.0, 480.0] {
    let mut stage = slider_row(width, 50.0, true);
    let first = stage.picture();
    stage.frame();
    assert_eq!(first, stage.picture(), "at {width}");
  }
}
