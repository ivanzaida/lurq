//! A popconfirm: its panel under what was pressed, its question and its two
//! buttons where the board has them, where the focus goes, what each button,
//! Escape and a press outside do and whom they tell, the destructive look, its
//! icon, where it stands, drawn in place, and a theme's rules.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::{CollisionStrategy, Ctx},
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Placement, Rect, Row},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Button, ButtonVariant, FontFamily, IconChoice, Popconfirm, PopconfirmPart, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// Counts what is pressed.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

fn count(counter: &Arc<AtomicUsize>) -> usize {
  counter.load(Ordering::SeqCst)
}

/// A page of 400 by 300 with a button of the app's at (20, 20) that the popconfirm hangs from, and something to
/// press outside it. The page has a size of its own, so the popup has room in it.
fn page(
  popconfirm: impl Fn(ElementRef) -> Popconfirm + Send + Sync + 'static,
) -> impl Fn(&mut Ctx) -> Element + Send + Sync + 'static {
  let anchor = ElementRef::new();
  move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding_left(20.0)
      .padding_top(20.0)
      .child(Row::new().child(popconfirm(anchor.clone())))
      .child(Rect::new(1.0, 180.0))
      .child(Rect::new(40.0, 20.0).id("outside"))
      .into()
  }
}

/// The board's popconfirm: delete this file, its trigger a button of the app's.
fn delete_file(
  anchor: ElementRef,
  open: Signal<bool>,
  confirmed: impl Fn() + Send + Sync + 'static,
  cancelled: impl Fn() + Send + Sync + 'static,
) -> Popconfirm {
  Popconfirm::new(open, anchor)
    .id("panel")
    .trigger(Button::new("Delete file").variant(ButtonVariant::Outline).id("trigger"))
    .title("Delete this file?")
    .description("It will be moved to the trash.")
    .confirm("Yes", confirmed)
    .cancel("No", cancelled)
}

/// The filled rectangles of `color` 28 high: the buttons.
fn buttons_of(stage: &Stage, color: Color) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color) && quad.height == 28.0
    })
    .count()
}

#[test]
fn an_open_popconfirm_is_the_panel_of_the_board_under_what_was_pressed() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| delete_file(anchor, held.clone(), || {}, || {})));
  let (_, trigger_y, _, trigger_height) = stage.bounds("trigger");
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!(width, 250.0);
  assert_eq!(x, 20.0, "at the start of what was pressed");
  assert_eq!(y, trigger_y + trigger_height + 4.0, "under it");
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Popover)));

  // The title of 13 and the description of 12 in the muted color, after the icon of 16 and 8, 12 from the edges.
  assert_eq!(stage.text("Delete this file?"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(
    stage.text("It will be moved to the trash."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let title = stage.text_quad("Delete this file?").unwrap();
  assert!((title.x - (x + 12.0 + 16.0 + 8.0)).abs() < 0.5, "{} {x}", title.x);

  // Two buttons of 28 at the end, No before Yes, their labels 13.
  assert_eq!(buttons_of(&stage, light(PaletteColor::Primary)), 1);
  let (no, yes) = (stage.text_quad("No").unwrap(), stage.text_quad("Yes").unwrap());
  assert!(no.x < yes.x);
  assert_eq!(stage.text("Yes").1, 13.0);
  assert!(
    yes.x + yes.width < x + width - 12.0 - 9.0,
    "padded 10 inside the panel's 12"
  );
  assert!(yes.x + yes.width > x + width - 12.0 - 11.0);
}

#[test]
fn the_trigger_opens_it_and_the_focus_goes_to_the_confirm_button() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  stage.click("trigger");
  assert!(open.get());
  // Space presses the focused button: the confirm one.
  stage.key(" ");
  assert_eq!((count(&confirmed), count(&cancelled)), (1, 0));
  assert!(!open.get());
}

#[test]
fn where_what_it_confirms_is_destructive_the_focus_goes_to_the_cancel_button() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone()).destructive(true)
  }));
  stage.click("trigger");
  assert!(open.get());
  // Space presses the focused button: the cancel one, the least destructive answer.
  stage.key(" ");
  assert_eq!((count(&confirmed), count(&cancelled)), (0, 1));
  assert!(!open.get());
}

#[test]
fn the_confirm_button_closes_it_and_confirms() {
  let open = Signal::new(true);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  let yes = stage.text_quad("Yes").unwrap();
  stage.tree.mouse_move(yes.x + 2.0, yes.y + 2.0);
  stage
    .tree
    .mouse_down(yes.x + 2.0, yes.y + 2.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(yes.x + 2.0, yes.y + 2.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!((count(&confirmed), count(&cancelled)), (1, 0));
  assert!(!open.get());
}

#[test]
fn escape_and_a_press_outside_cancel() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  // Opened by its trigger, it has the focus.
  stage.click("trigger");
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!((count(&confirmed), count(&cancelled)), (0, 1));

  // A press outside cancels too.
  open.set(true);
  stage.pass();
  stage.click("outside");
  assert!(!open.get());
  assert_eq!((count(&confirmed), count(&cancelled)), (0, 2));
}

#[test]
fn a_destructive_popconfirm_confirms_in_the_destructive_color() {
  let stage = Stage::new(page(|anchor| {
    delete_file(anchor, Signal::new(true), || {}, || {}).destructive(true)
  }));
  assert_eq!(buttons_of(&stage, light(PaletteColor::Destructive)), 1);
  assert_eq!(buttons_of(&stage, light(PaletteColor::Primary)), 0);
}

#[test]
fn a_theme_styles_its_buttons_apart_from_buttons_elsewhere() {
  let app = App::new();
  app.theme().patch_recipe::<Popconfirm>(|rules| {
    rules
      .base(PopconfirmPart::ConfirmLabel, |s| s.font_family(FontFamily::Mono))
      .base(PopconfirmPart::Panel, |s| s.width(300.0))
  });
  let mut stage = Stage::in_app(app, page(|anchor| delete_file(anchor, Signal::new(true), || {}, || {})));
  assert_eq!(stage.font_of("Yes"), "monospace");
  let system = stage.app.theme().sans_family().to_string();
  assert_eq!(stage.font_of("No"), system);
  assert_eq!(stage.font_of("Delete file"), system);
  assert_eq!(stage.bounds("panel").2, 300.0);
}

#[test]
fn a_title_and_a_description_too_long_for_a_line_go_on_to_the_next() {
  const TITLE: &str = "Delete this file and every copy of it in the workspace?";
  const DESCRIPTION: &str = "It will be moved to the trash, where it is kept for thirty days.";
  let long = Signal::new(true);
  let held = long.clone();
  let mut stage = Stage::new(page(move |anchor| {
    let (title, description) = if held.get() {
      (TITLE, DESCRIPTION)
    } else {
      ("Delete?", "Trash.")
    };
    Popconfirm::new(Signal::new(true), anchor)
      .id("panel")
      .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
      .title(title)
      .description(description)
  }));
  let (x, _, width, height) = stage.bounds("panel");
  for (text, line) in [(TITLE, 13.0), (DESCRIPTION, 12.0)] {
    let quad = stage
      .text_quad(text)
      .unwrap_or_else(|| panic!("{text:?} is painted whole"));
    assert!(
      quad.height > line * 2.0,
      "{text:?} on more than one line: {} high",
      quad.height
    );
    assert!(
      quad.x + quad.width <= x + width - 12.0 + 0.5,
      "{text:?} inside the panel"
    );
  }
  long.set(false);
  stage.pass();
  assert!(
    height > stage.bounds("panel").3 + 20.0,
    "the panel grows with its lines"
  );
}

#[test]
fn buttons_too_long_to_stand_side_by_side_go_one_under_the_other_inside_the_panel() {
  const CANCEL: &str = "Keep the file where it is";
  const CONFIRM: &str = "Move it to the trash now";
  let mut stage = Stage::new(page(move |anchor| {
    Popconfirm::new(Signal::new(true), anchor)
      .id("panel")
      .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
      .title("Delete this file?")
      .confirm(CONFIRM, || {})
      .cancel(CANCEL, || {})
  }));
  let (x, _, width, _) = stage.bounds("panel");
  let (cancel, confirm) = (stage.text_quad(CANCEL).unwrap(), stage.text_quad(CONFIRM).unwrap());
  for (text, quad) in [(CANCEL, &cancel), (CONFIRM, &confirm)] {
    assert!(
      quad.x >= x + 12.0 - 0.5 && quad.x + quad.width <= x + width - 12.0 + 0.5,
      "{text:?} inside the panel: {}..{} in {x}..{}",
      quad.x,
      quad.x + quad.width,
      x + width
    );
  }
  // Cancel first, the confirming button under it, both at the end.
  assert!(confirm.y > cancel.y + cancel.height);
  assert!((confirm.x + confirm.width - (cancel.x + cancel.width)).abs() < 1.0);
}

#[test]
fn a_button_too_long_for_the_panel_cuts_its_label_inside_it() {
  const CONFIRM: &str = "Move this file and every other file of its folder to the trash, now and for good";
  let mut stage = Stage::new(page(move |anchor| {
    Popconfirm::new(Signal::new(true), anchor)
      .id("panel")
      .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
      .title("Delete this file?")
      .confirm(CONFIRM, || {})
      .cancel("No", || {})
  }));
  let (x, _, width, _) = stage.bounds("panel");
  let inside = |left: f32, right: f32| left >= x - 0.5 && right <= x + width + 0.5;
  // The confirming button: its fill is the primary one, and nothing of it is past the panel's edge.
  let fill = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary))
        && quad.height == 28.0
    })
    .expect("the confirming button");
  assert!(
    inside(fill.x, fill.x + fill.width),
    "the button in the panel: {}..{} in {x}..{}",
    fill.x,
    fill.x + fill.width,
    x + width
  );
  // Its label is cut with an ellipsis, inside the button.
  let (text, label_x, label_width) = stage
    .quads()
    .into_iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } if text.starts_with("Move this") => Some((text.clone(), quad.x, quad.width)),
      _ => None,
    })
    .expect("the label");
  assert!(text.ends_with('…'), "cut with an ellipsis: {text}");
  assert!(label_x >= fill.x && label_x + label_width <= fill.x + fill.width + 0.5);
  assert!(stage.paints("No"), "the other button is whole");
}

/// A press of the text `content`: a button's label.
fn press(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"));
  let (x, y) = (quad.x + 2.0, quad.y + 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// What was told, in order.
type Told = Arc<Mutex<Vec<&'static str>>>;

/// A handler that writes `what` down in `told`.
fn tell(told: &Told, what: &'static str) -> impl Fn() + Send + Sync + Clone + 'static {
  let told = told.clone();
  move || told.lock().unwrap().push(what)
}

#[test]
fn a_button_closes_it_before_its_handler_runs() {
  let open = Signal::new(true);
  let seen = Arc::new(Mutex::new(Vec::new()));
  let (held, confirm_saw, cancel_saw) = (open.clone(), seen.clone(), seen.clone());
  let mut stage = Stage::new(page(move |anchor| {
    let (on_confirm, on_cancel) = (held.clone(), held.clone());
    let (confirm_saw, cancel_saw) = (confirm_saw.clone(), cancel_saw.clone());
    delete_file(
      anchor,
      held.clone(),
      move || confirm_saw.lock().unwrap().push(on_confirm.get()),
      move || cancel_saw.lock().unwrap().push(on_cancel.get()),
    )
  }));
  press(&mut stage, "Yes");
  open.set(true);
  stage.pass();
  press(&mut stage, "No");
  assert_eq!(*seen.lock().unwrap(), [false, false], "closed when either handler ran");
}

#[test]
fn every_handler_of_on_dismiss_is_told_after_cancel_when_a_person_closes_it_and_not_on_an_answer() {
  let open = Signal::new(false);
  let told: Told = Arc::default();
  let (held, record) = (open.clone(), told.clone());
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), tell(&record, "confirm"), tell(&record, "cancel"))
      .on_dismiss(tell(&record, "first"))
      .on_dismiss(tell(&record, "second"))
  }));
  stage.click("trigger");
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(*told.lock().unwrap(), ["cancel", "first", "second"], "Escape");

  told.lock().unwrap().clear();
  open.set(true);
  stage.pass();
  stage.click("outside");
  assert!(!open.get());
  assert_eq!(*told.lock().unwrap(), ["cancel", "first", "second"], "a press outside");

  told.lock().unwrap().clear();
  open.set(true);
  stage.pass();
  press(&mut stage, "No");
  open.set(true);
  stage.pass();
  press(&mut stage, "Yes");
  assert_eq!(
    *told.lock().unwrap(),
    ["cancel", "confirm"],
    "an answer is no dismissal"
  );
}

#[test]
fn escape_and_a_press_outside_can_each_be_kept_from_cancelling() {
  let open = Signal::new(false);
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), || {}, cancel.clone()).dismiss_on_escape(false)
  }));
  stage.click("trigger");
  stage.key("Escape");
  assert!(open.get(), "Escape leaves it open");
  assert_eq!(count(&cancelled), 0);
  stage.click("outside");
  assert!(!open.get(), "a press outside still cancels");
  assert_eq!(count(&cancelled), 1);

  let open = Signal::new(false);
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), || {}, cancel.clone()).dismiss_on_outside_press(false)
  }));
  stage.click("trigger");
  stage.click("outside");
  assert!(open.get(), "a press outside leaves it open");
  assert_eq!(count(&cancelled), 0);
  stage.key("Escape");
  assert!(!open.get(), "Escape still cancels");
  assert_eq!(count(&cancelled), 1);
}

#[test]
fn its_icon_is_the_warning_sign_another_one_or_none() {
  let title_from_edge = |icon: Option<IconChoice>| {
    let mut stage = Stage::new(page(move |anchor| {
      let popconfirm = delete_file(anchor, Signal::new(true), || {}, || {});
      match icon {
        Some(icon) => popconfirm.icon(icon),
        None => popconfirm,
      }
    }));
    let (x, ..) = stage.bounds("panel");
    stage.text_quad("Delete this file?").unwrap().x - x
  };
  let warning = title_from_edge(None);
  assert!((warning - (12.0 + 16.0 + 8.0)).abs() < 0.5, "after the icon: {warning}");
  let other = title_from_edge(Some(icons::TRASH.into()));
  assert!((other - warning).abs() < 0.5, "after another icon: {other}");
  let none = title_from_edge(Some(None.into()));
  assert!((none - 12.0).abs() < 0.5, "at the padding, with no icon: {none}");
}

#[test]
fn its_width_offset_and_collision_are_its_own() {
  let mut stage = Stage::new(page(|anchor| {
    delete_file(anchor, Signal::new(true), || {}, || {})
      .width(300.0)
      .offset(0.0, 10.0)
  }));
  let (_, trigger_y, _, trigger_height) = stage.bounds("trigger");
  let (_, y, width, _) = stage.bounds("panel");
  assert_eq!((width, y), (300.0, trigger_y + trigger_height + 10.0));

  // Above its trigger there is no room: by default it goes under it, and with no strategy it stays above.
  let above = |collision: Option<CollisionStrategy>| {
    let mut stage = Stage::new(page(move |anchor| {
      let popconfirm = delete_file(anchor, Signal::new(true), || {}, || {}).placement(Placement::TopStart);
      match collision {
        Some(collision) => popconfirm.collision(collision),
        None => popconfirm,
      }
    }));
    let (_, trigger_y, ..) = stage.bounds("trigger");
    let (_, y, _, height) = stage.bounds("panel");
    y + height <= trigger_y
  };
  assert!(!above(None), "flipped under it");
  assert!(above(Some(CollisionStrategy::None)), "left above it");
}

#[test]
fn an_inline_popconfirm_is_drawn_where_it_is_declared_and_still_answers() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), || {}).inline(true)
  }));
  assert!(stage.paints("Delete this file?"), "closed, it is drawn");
  assert!(!stage.paints("Delete file"), "without its trigger");
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!((x, y, width), (20.0, 20.0, 250.0));
  open.set(true);
  stage.pass();
  press(&mut stage, "Yes");
  assert_eq!(count(&confirmed), 1);
  assert!(!open.get());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn its_label_is_what_tooling_calls_it_in_place_of_its_title() {
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("panel")
      .and_then(|element| element.description(name))
  };
  let mut stage = Stage::new(page(|anchor| delete_file(anchor, Signal::new(true), || {}, || {})));
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("alertdialog"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Delete this file?"));
  let mut stage = Stage::new(page(|anchor| {
    delete_file(anchor, Signal::new(true), || {}, || {}).label("Delete the file")
  }));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Delete the file"));
}
