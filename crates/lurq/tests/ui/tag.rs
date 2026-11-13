//! A tag and a chip: what they paint against the board, the tag's remove
//! button from the pointer and the keyboard, a chip turned by a press and
//! bound to a signal, a group of chips, and what a theme changes of them.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Case, Chip, ChipGroup, ChipPart, ChipState, Interaction, Opacity, Tag, TagPart, TagRenderer, TagSize, TagState,
    TagVariant, TagView, Themed,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// Whether `a` and `b` are the same position, give or take what a sum of fractions loses.
fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.01
}

/// The color of the line inside the element with `id`, if it has a line of some width.
fn line_color(stage: &mut Stage, id: &str) -> Option<Color> {
  let quad = stage.border(id)?;
  let side = quad.border?.top?;
  (side.width > 0.0).then_some(side.color)
}

/// The box of the remove button of the tag with `id`: the square at its end, `end` in from its edge.
fn remove_box(stage: &mut Stage, id: &str, side: f32, end: f32) -> (f32, f32, f32, f32) {
  let (x, y, width, height) = stage.bounds(id);
  (x + width - end - side, y + (height - side) / 2.0, side, side)
}

/// The filled rectangle painted at `rect`, if there is one.
fn fill_at(stage: &Stage, (x, y, width, height): (f32, f32, f32, f32)) -> Option<Color> {
  stage.quads().into_iter().find_map(|quad: Quad| match quad.content {
    QuadContent::Rect { color, .. }
      if color.a() > 0
        && near(quad.x, x)
        && near(quad.y, y)
        && near(quad.width, width)
        && near(quad.height, height) =>
    {
      Some(color)
    }
    _ => None,
  })
}

fn press_at(stage: &mut Stage, (x, y, width, height): (f32, f32, f32, f32)) {
  let (x, y) = (x + width / 2.0, y + height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
}

#[test]
fn a_tag_is_its_text_in_the_secondary_fill_with_a_line_inside_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Row::new().child(Tag::new("design").id("plain")))
      .child(Row::new().child(Tag::new("desktop").id("removable").on_remove(|| {})))
      .into()
  });
  let (x, y, width, height) = stage.bounds("plain");
  assert_eq!(height, 24.0);
  assert_eq!(stage.fill_color("plain"), Some(light(PaletteColor::Secondary)));
  assert_eq!(line_color(&mut stage, "plain"), Some(light(PaletteColor::Border)));
  let corner = stage.fill("plain").unwrap().border_radius.unwrap().top_left;
  assert_eq!(corner, 4.0, "the corners of a small radius");
  // Its text is 12 and medium, in the ink of what is secondary, padded 8 at both ends.
  assert_eq!(stage.text("design"), (light(PaletteColor::SecondaryForeground), 12.0));
  let label = stage.text_quad("design").unwrap();
  assert!(near(label.x - x, 8.0), "{}", label.x - x);
  assert!(near(x + width - (label.x + label.width), 8.0));
  let middle = label.y + label.height / 2.0 - y;
  assert!((middle - 12.0).abs() < 0.5, "in the middle: {middle}");

  // A removable one is padded 4 at its end, where its button of 16 is, 4 after the text.
  let (x, _, width, height) = stage.bounds("removable");
  assert_eq!(height, 24.0);
  let label = stage.text_quad("desktop").unwrap();
  assert!(near(label.x - x, 8.0));
  assert!(near(x + width - (label.x + label.width), 4.0 + 16.0 + 4.0));
}

#[test]
fn a_tag_of_a_variant_is_in_the_muted_fill_of_its_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(Row::new().child(Tag::new("in review").variant(TagVariant::Info).id("info")))
      .child(Row::new().child(Tag::new("passing").variant(TagVariant::Success).id("success")))
      .child(Row::new().child(Tag::new("pending").variant(TagVariant::Warning).id("warning")))
      .child(Row::new().child(Tag::new("failing").variant(TagVariant::Destructive).id("destructive")))
      .into()
  });
  for (id, text, fill, ink) in [
    ("info", "in review", PaletteColor::InfoMuted, PaletteColor::Info),
    ("success", "passing", PaletteColor::SuccessMuted, PaletteColor::Success),
    ("warning", "pending", PaletteColor::WarningMuted, PaletteColor::Warning),
    (
      "destructive",
      "failing",
      PaletteColor::DestructiveMuted,
      PaletteColor::Destructive,
    ),
  ] {
    assert_eq!(stage.fill_color(id), Some(light(fill.clone())), "{id}");
    assert_eq!(stage.text(text).0, light(ink), "{id}");
  }
}

#[test]
fn a_tag_is_20_24_or_28_high_and_its_icon_is_before_its_text() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(Row::new().child(Tag::new("small").size(TagSize::Sm).id("sm")))
      .child(Row::new().child(Tag::new("middle").id("md")))
      .child(Row::new().child(Tag::new("large").size(TagSize::Lg).id("lg")))
      .child(Row::new().child(Tag::new("tagged").icon(lurq::ui::icons::TAG).id("icon")))
      .into()
  });
  for (id, text, height, font) in [
    ("sm", "small", 20.0, 11.0),
    ("md", "middle", 24.0, 12.0),
    ("lg", "large", 28.0, 13.0),
  ] {
    assert_eq!(stage.bounds(id).3, height, "{id}");
    assert_eq!(stage.text(text).1, font, "{id}");
  }
  // An icon of 12, 4 before the text.
  let (x, ..) = stage.bounds("icon");
  let label = stage.text_quad("tagged").unwrap();
  assert!(near(label.x - x, 8.0 + 12.0 + 4.0), "{}", label.x - x);
}

#[test]
fn a_pill_is_round_with_no_line_and_padded_10_and_6() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Row::new().child(Tag::new("React").pill(true).on_remove(|| {}).id("pill")))
      .into()
  });
  let (x, _, width, height) = stage.bounds("pill");
  assert_eq!(height, 24.0);
  assert_eq!(stage.fill_color("pill"), Some(light(PaletteColor::Secondary)));
  assert!(
    stage.fill("pill").unwrap().border_radius.unwrap().top_left >= 12.0,
    "round"
  );
  assert_eq!(line_color(&mut stage, "pill"), None, "no line");
  let label = stage.text_quad("React").unwrap();
  assert!(near(label.x - x, 10.0));
  // The cross of 12 is 4 after the text and 6 from the end.
  assert!(near(x + width - (label.x + label.width), 4.0 + 12.0 + 6.0));
}

#[test]
fn the_remove_button_takes_the_tag_away_from_the_pointer_and_the_keyboard() {
  let removed = Arc::new(Mutex::new(Vec::new()));
  let told = removed.clone();
  let mut stage = Stage::new(move |_| {
    let tag = |label: &'static str| {
      let told = told.clone();
      Tag::new(label)
        .id(label)
        .on_remove(move || told.lock().unwrap().push(label))
    };
    Row::new()
      .padding(8.0)
      .spacing(6.0)
      .child(tag("rust"))
      .child(tag("go"))
      .child(Tag::new("read only").id("read only"))
      .child(tag("locked").disabled(true))
      .child(tag("wasm"))
      .into()
  });
  // Backspace with the focus on nothing takes nothing away.
  stage.key("Backspace");
  assert!(removed.lock().unwrap().is_empty());

  // Tab reaches the first remove button, and draws its ring.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  stage.key("Tab");
  stage.key("Backspace");
  // Tab passes a tag with no button and a disabled one by.
  stage.key("Tab");
  stage.key("Delete");
  stage.key(" ");
  assert_eq!(*removed.lock().unwrap(), ["rust", "go", "wasm", "wasm"]);
  removed.lock().unwrap().clear();

  // A press of the button; one of the disabled tag's takes nothing, and a press of the text is no press of it.
  let rust = remove_box(&mut stage, "rust", 16.0, 4.0);
  press_at(&mut stage, rust);
  let locked = remove_box(&mut stage, "locked", 16.0, 4.0);
  press_at(&mut stage, locked);
  stage.click("read only");
  let label = stage.text_quad("go").unwrap();
  press_at(&mut stage, (label.x, label.y, label.width, label.height));
  assert_eq!(*removed.lock().unwrap(), ["rust"]);
}

#[test]
fn the_remove_button_is_filled_under_the_pointer() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(Row::new().child(Tag::new("rust").id("rust").on_remove(|| {})))
      .child(
        Row::new().child(
          Tag::new("forced")
            .id("forced")
            .on_remove(|| {})
            .force(Some(Interaction::Hovered)),
        ),
      )
      .into()
  });
  let rest = remove_box(&mut stage, "rust", 16.0, 4.0);
  assert_eq!(fill_at(&stage, rest), None, "nothing under it at rest");
  let forced = remove_box(&mut stage, "forced", 16.0, 4.0);
  assert_eq!(fill_at(&stage, forced), Some(light(PaletteColor::StateHover)));
  let (x, y) = (rest.0 + 8.0, rest.1 + 8.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert_eq!(fill_at(&stage, rest), Some(light(PaletteColor::StateHover)));
}

#[test]
fn a_disabled_tag_is_dimmed() {
  let stage = Stage::new(|_| Row::new().child(Tag::new("locked").disabled(true)).into());
  let theme = stage.app.theme().clone();
  let case = Case::new().state(TagState::Disabled, true);
  let root = theme.resolve::<Tag>(&case, None).part(TagPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme.resolve::<Tag>(&Case::new(), None).part(TagPart::Root).rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_theme_gives_the_tag_another_anatomy_and_its_button_still_takes_it_away() {
  let removed = Arc::new(Mutex::new(0));
  let told = removed.clone();
  let app = App::new();
  // The remove button alone, with the text in it.
  let renderer: TagRenderer = Arc::new(|view: TagView| {
    let label = view.part(TagPart::Label).text(&format!("× {}", view.label));
    view.remove().unwrap().id("remove").child(label).into()
  });
  app.theme().set_renderer::<Tag>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(Tag::new("rust").on_remove(move || *told.lock().unwrap() += 1))
      .into()
  });
  assert!(stage.paints("× rust"));
  stage.click("remove");
  assert_eq!(*removed.lock().unwrap(), 1);
}

#[test]
fn a_chip_not_chosen_is_outlined_and_a_chosen_one_is_filled_with_a_check() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(6.0)
      .child(Chip::new("Rust").selected(true).id("on"))
      .child(Chip::new("Swift").id("off"))
      .into()
  });
  let (x, _, _, height) = stage.bounds("off");
  assert_eq!(height, 28.0);
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Background)));
  assert_eq!(line_color(&mut stage, "off"), Some(light(PaletteColor::Input)));
  assert!(
    stage.fill("off").unwrap().border_radius.unwrap().top_left >= 14.0,
    "round"
  );
  assert_eq!(stage.text("Swift"), (light(PaletteColor::Foreground), 13.0));
  assert!(near(stage.text_quad("Swift").unwrap().x - x, 12.0));

  let (x, ..) = stage.bounds("on");
  assert_eq!(stage.fill_color("on"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.text("Rust"), (light(PaletteColor::PrimaryForeground), 13.0));
  // The check of 14 is 8 from the start and 6 before the text.
  assert!(near(stage.text_quad("Rust").unwrap().x - x, 8.0 + 14.0 + 6.0));
}

#[test]
fn a_chip_under_the_pointer_is_highlighted() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(6.0)
      .child(Chip::new("Swift").id("off"))
      .child(Chip::new("Rust").selected(true).id("on"))
      .into()
  });
  stage.hover("off");
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Highlight)));
  stage.hover("on");
  assert_eq!(stage.fill_color("on"), Some(light(PaletteColor::PrimaryHover)));
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Background)));
}

#[test]
fn a_press_of_a_chip_turns_it_and_a_disabled_one_takes_nothing() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let open = Signal::new(false);
  let (told, bound) = (asked.clone(), open.clone());
  let mut stage = Stage::new(move |_| {
    let (rust, gone) = (told.clone(), told.clone());
    Row::new()
      .padding(8.0)
      .spacing(6.0)
      .child(
        Chip::new("Rust")
          .id("rust")
          .on_change(move |next| rust.lock().unwrap().push(("rust", next))),
      )
      .child(
        Chip::new("Kotlin")
          .disabled(true)
          .id("kotlin")
          .on_change(move |next| gone.lock().unwrap().push(("kotlin", next))),
      )
      .child(Chip::new("Open").bind(bound.clone()).id("open"))
      .into()
  });
  // Tab reaches a chip and draws its ring; Space and Enter press it. Tab passes the disabled one by.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*asked.lock().unwrap(), [("rust", true)]);
  assert!(open.get(), "bound to the signal: the press turned it");
  assert_eq!(stage.fill_color("open"), Some(light(PaletteColor::Primary)));
  stage.key(" ");
  assert!(!open.get());

  stage.click("rust");
  stage.click("kotlin");
  stage.click("open");
  assert_eq!(*asked.lock().unwrap(), [("rust", true), ("rust", true)]);
  assert!(open.get());
}

#[test]
fn a_disabled_chip_is_dimmed() {
  let stage = Stage::new(|_| Row::new().child(Chip::new("Disabled").disabled(true)).into());
  let theme = stage.app.theme().clone();
  let case = Case::new().state(ChipState::Disabled, true);
  let root = theme.resolve::<Chip>(&case, None).part(ChipPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn the_arrow_keys_move_between_the_chips_of_a_group_and_space_turns_one() {
  let languages = Signal::new(vec!["rust"]);
  let kept = languages.clone();
  let after = Arc::new(Mutex::new(0));
  let pressed = after.clone();
  let mut stage = Stage::new(move |_| {
    let pressed = pressed.clone();
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(
        ChipGroup::multiple(
          kept.clone(),
          [("rust", "Rust"), ("swift", "Swift"), ("kotlin", "Kotlin")],
        )
        .id("group"),
      )
      .child(Chip::new("After").on_change(move |_| *pressed.lock().unwrap() += 1))
      .into()
  });
  // The chips are 6 apart.
  let rust = stage.text_quad("Rust").unwrap();
  let swift = stage.text_quad("Swift").unwrap();
  assert!(swift.x > rust.x + rust.width + 6.0);

  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(languages.get(), ["rust"], "moving the focus turns nothing");
  stage.key(" ");
  assert_eq!(languages.get(), ["rust", "swift"]);
  stage.key("ArrowLeft");
  stage.key("Enter");
  assert_eq!(languages.get(), ["swift"]);
  // The group is one stop: the next Tab is on what is after it.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*after.lock().unwrap(), 1);
}

#[test]
fn a_group_of_one_choice_sets_the_signal_to_the_chip_pressed() {
  let kind = Signal::new(0u8);
  let kept = kind.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(ChipGroup::new(
        kept.clone(),
        [(0u8, "All"), (1, "Issues"), (2, "Pull requests")],
      ))
      .into()
  });
  let issues = stage.text_quad("Issues").unwrap();
  press_at(&mut stage, (issues.x, issues.y, issues.width, issues.height));
  assert_eq!(kind.get(), 1);
  assert_eq!(stage.text("Issues").0, light(PaletteColor::PrimaryForeground));
  assert_eq!(stage.text("All").0, light(PaletteColor::Foreground));
}

#[test]
fn a_rule_of_a_theme_changes_a_chosen_chip() {
  let app = App::new();
  app.theme().set_recipe::<Chip>(
    <Chip as Themed>::recipe().state(ChipState::Selected, ChipPart::Root, |s| {
      s.background(PaletteColor::Success)
    }),
  );
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(Chip::new("Rust").selected(true).id("on"))
      .into()
  });
  assert_eq!(stage.fill_color("on"), Some(light(PaletteColor::Success)));
  assert_eq!(stage.bounds("on").3, 28.0, "a chip of the design in everything else");
}
