//! A message box: its panel, its icon by kind, its title, its message and its
//! footer where the board has them, the answers and where the focus goes,
//! what Enter, Escape and a press of the veil give, the checkbox, the detail,
//! and a theme's fonts for its two buttons and its other anatomy.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{
    Button, FontFamily, MessageBox, MessageBoxKind, MessageBoxPart, MessageBoxRenderer, MessageBoxRole, MessageBoxView,
    icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage whose window is 600 by 400, the size of its page: room for the panel of 400 and the veil's margin.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new().size(600.0, 400.0).child(scene(ctx)).into()
  });
  stage.tree.resize(600, 400);
  stage.pass();
  stage
}

/// Counts what is given.
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

/// A press of the left button at a point of the window.
fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// A press of the text `content`: of the button or the trigger it is the label of.
fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("no text {content:?} was painted"));
  press_at(stage, quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
}

/// The filled rectangles of `color` that are `width` by `height`.
fn fills(stage: &Stage, color: Color, width: f32, height: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width - width).abs() < 0.5
        && (quad.height - height).abs() < 0.5
    })
    .collect()
}

/// The board's message box: replace an existing file, with a checkbox and two answers.
fn replace(
  open: Signal<bool>,
  again: Signal<bool>,
  cancel: impl Fn() + Send + Sync + 'static,
  replace: impl Fn() + Send + Sync + 'static,
) -> MessageBox {
  MessageBox::new(open)
    .id("box")
    .kind(MessageBoxKind::Warning)
    .title("Replace existing file?")
    .message("theme.json already exists in this folder.")
    .checkbox("Don't ask again", again)
    .answer("Cancel", MessageBoxRole::Cancel, cancel)
    .answer("Replace", MessageBoxRole::Default, replace)
}

/// The board's destructive message box: delete projects.
fn delete(
  open: Signal<bool>,
  cancel: impl Fn() + Send + Sync + 'static,
  delete: impl Fn() + Send + Sync + 'static,
) -> MessageBox {
  MessageBox::new(open)
    .id("box")
    .kind(MessageBoxKind::Warning)
    .icon(icons::TRASH)
    .title("Delete 3 projects?")
    .message("This cannot be undone.")
    .answer("Cancel", MessageBoxRole::Cancel, cancel)
    .answer("Delete", MessageBoxRole::Destructive, delete)
}

#[test]
fn an_open_message_box_is_the_panel_of_the_board_over_a_veil() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| replace(open.clone(), Signal::new(false), || {}, || {}).into());

  // A panel of 400 in the middle of the window, in the popover's color, over the scrim.
  let (x, y, width, height) = stage.bounds("box");
  assert_eq!(width, 400.0);
  assert_eq!(x, 100.0);
  assert!((y + height / 2.0 - 200.0).abs() < 0.6, "in the middle: {y} {height}");
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Popover)));
  assert_eq!(
    fills(&stage, light(PaletteColor::Scrim), 600.0, 400.0).len(),
    1,
    "the veil"
  );

  // The well of 36 in the warning's muted color, 20 from the top and the start.
  let wells = fills(&stage, light(PaletteColor::WarningMuted), 36.0, 36.0);
  assert_eq!(wells.len(), 1, "one well of the warning's muted color");
  assert!((wells[0].x - (x + 20.0)).abs() < 0.5 && (wells[0].y - (y + 20.0)).abs() < 0.5);
  assert!(
    wells[0].border_radius.is_some_and(|radius| radius.top_left == 18.0),
    "round"
  );

  // The title of 15 and the message of 13 in the muted color, 14 after the well.
  assert_eq!(
    stage.text("Replace existing file?"),
    (light(PaletteColor::Foreground), 15.0)
  );
  assert_eq!(
    stage.text("theme.json already exists in this folder."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let title = stage.text_quad("Replace existing file?").unwrap();
  assert!((title.x - (x + 70.0)).abs() < 0.5, "{} {x}", title.x);
  let message = stage.text_quad("theme.json already exists in this folder.").unwrap();
  assert!(message.y > title.y + title.height - 0.5, "the message under the title");

  // The checkbox at the start of the footer, its label of 13 8 after its box of 16.
  let label = stage.text_quad("Don't ask again").unwrap();
  assert_eq!(stage.text("Don't ask again").1, 13.0);
  assert!((label.x - (x + 20.0 + 16.0 + 8.0)).abs() < 0.5, "{} {x}", label.x);

  // The buttons of 32 at the end, 20 from the edges, the cancel one first, 8 apart.
  let primary: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary))
        && quad.height == 32.0
    })
    .collect();
  assert_eq!(primary.len(), 1, "the default answer is the one filled button");
  let replace = &primary[0];
  assert!((replace.x + replace.width - (x + width - 20.0)).abs() < 0.5);
  assert!((replace.y + replace.height - (y + height - 20.0)).abs() < 0.5);
  let cancel = stage.text_quad("Cancel").unwrap();
  let replace_label = stage.text_quad("Replace").unwrap();
  assert!(cancel.x < replace_label.x, "the cancel answer before the default one");
  // Padded 14 on each side of its label.
  assert!((replace_label.x - (replace.x + 14.0)).abs() < 1.0);
}

#[test]
fn each_kind_has_its_colors_and_a_destructive_question_the_destructive_ones() {
  let cases = [
    (MessageBoxKind::Information, PaletteColor::InfoMuted),
    (MessageBoxKind::Warning, PaletteColor::WarningMuted),
    (MessageBoxKind::Error, PaletteColor::DestructiveMuted),
    (MessageBoxKind::Question, PaletteColor::Secondary),
  ];
  for (kind, well) in cases {
    let stage = stage(move |_| {
      MessageBox::new(Signal::new(true))
        .kind(kind)
        .title("Title")
        .answer("OK", MessageBoxRole::Default, || {})
        .into()
    });
    assert_eq!(fills(&stage, light(well), 36.0, 36.0).len(), 1, "{kind:?}");
  }
  let stage = stage(|_| delete(Signal::new(true), || {}, || {}).into());
  assert_eq!(
    fills(&stage, light(PaletteColor::DestructiveMuted), 36.0, 36.0).len(),
    1
  );
  assert!(fills(&stage, light(PaletteColor::WarningMuted), 36.0, 36.0).is_empty());
  let destructive = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Destructive))
      && quad.height == 32.0
  });
  assert!(destructive, "the destructive answer is filled in the destructive color");
}

#[test]
fn the_focus_goes_to_the_default_answer_and_an_answer_closes_the_box_then_runs() {
  let open = Signal::new(true);
  let held = open.clone();
  let (cancelled, cancel) = counter();
  let (replaced, replace_it) = counter();
  let seen = Arc::new(AtomicUsize::new(9));
  let was = seen.clone();
  let after = open.clone();
  let mut stage = stage(move |_| {
    let (replace_it, was, after) = (replace_it.clone(), was.clone(), after.clone());
    replace(held.clone(), Signal::new(false), cancel.clone(), move || {
      replace_it();
      // The box is closed by the time the answer runs.
      was.store(usize::from(after.get()), Ordering::SeqCst);
    })
    .into()
  });
  // Space presses only the focused button: the default one.
  stage.key(" ");
  assert_eq!((count(&cancelled), count(&replaced)), (0, 1));
  assert!(!open.get());
  assert_eq!(seen.load(Ordering::SeqCst), 0, "closed before the answer ran");
  assert!(!stage.paints("Replace existing file?"));
}

#[test]
fn the_focus_goes_to_the_cancel_answer_of_a_destructive_question() {
  let open = Signal::new(true);
  let (cancelled, cancel) = counter();
  let (deleted, delete_them) = counter();
  let mut stage = stage(move |_| delete(open.clone(), cancel.clone(), delete_them.clone()).into());
  stage.key(" ");
  assert_eq!((count(&cancelled), count(&deleted)), (1, 0));
}

#[test]
fn enter_gives_the_default_answer_unless_the_focus_is_on_another_button() {
  let open = Signal::new(true);
  let again = Signal::new(false);
  let (cancelled, cancel) = counter();
  let (replaced, replace_it) = counter();
  let (held, checked) = (open.clone(), again.clone());
  let mut stage = stage(move |_| replace(held.clone(), checked.clone(), cancel.clone(), replace_it.clone()).into());

  // From the default answer Tab goes round to the checkbox. Enter there gives the default answer and does not
  // check it.
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!((count(&cancelled), count(&replaced)), (0, 1));
  assert!(!again.get(), "Enter does not check the checkbox");
  assert!(!open.get());

  // Space checks it; on the cancel button Enter presses the cancel button.
  open.set(true);
  stage.pass();
  stage.key("Tab");
  stage.key(" ");
  assert!(again.get(), "Space checks the checkbox");
  assert!(open.get());
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!((count(&cancelled), count(&replaced)), (1, 1));
}

#[test]
fn escape_and_a_press_of_the_veil_give_the_cancel_answer() {
  let open = Signal::new(true);
  let (cancelled, cancel) = counter();
  let (replaced, replace_it) = counter();
  let held = open.clone();
  let mut stage = stage(move |_| replace(held.clone(), Signal::new(false), cancel.clone(), replace_it.clone()).into());
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!((count(&cancelled), count(&replaced)), (1, 0));

  open.set(true);
  stage.pass();
  stage.click("box");
  assert!(open.get(), "a press on the panel goes no further");
  press_at(&mut stage, 5.0, 5.0);
  assert!(!open.get());
  assert_eq!((count(&cancelled), count(&replaced)), (2, 0));
}

#[test]
fn a_message_box_with_no_cancel_answer_stays_on_escape_and_a_press_of_the_veil() {
  let open = Signal::new(true);
  let (oks, ok) = counter();
  let held = open.clone();
  let mut stage = stage(move |_| {
    let ok = ok.clone();
    MessageBox::new(held.clone())
      .title("The export is done.")
      .answer("OK", MessageBoxRole::Default, move || ok())
      .into()
  });
  stage.key("Escape");
  press_at(&mut stage, 5.0, 5.0);
  assert!(open.get());
  assert_eq!(count(&oks), 0);
  stage.key("Enter");
  assert!(!open.get());
  assert_eq!(count(&oks), 1);
}

#[test]
fn the_plain_answers_stand_at_the_start_and_the_cancel_one_before_the_others_at_the_end() {
  let mut stage = stage(|_| {
    MessageBox::new(Signal::new(true))
      .id("box")
      .title("Do you want to save the changes?")
      .answer("Save", MessageBoxRole::Default, || {})
      .answer("Don't Save", MessageBoxRole::Plain, || {})
      .answer("Cancel", MessageBoxRole::Cancel, || {})
      .into()
  });
  let panel = stage.bounds("box").0;
  let x = |content: &str| stage.text_quad(content).unwrap().x;
  let start = x("Don't Save");
  assert!(
    start < x("Cancel") && x("Cancel") < x("Save"),
    "Don't Save, Cancel, Save"
  );
  assert!(
    (start - (panel + 20.0 + 14.0)).abs() < 1.0,
    "at the start of the footer"
  );
}

#[test]
fn the_detail_opens_under_the_message() {
  let detail = Signal::new(false);
  let held = detail.clone();
  let mut stage = stage(move |_| {
    MessageBox::new(Signal::new(true))
      .kind(MessageBoxKind::Error)
      .title("The file could not be saved.")
      .message("The disk is full.")
      .detail("os error 28: no space left on device", held.clone())
      .answer("OK", MessageBoxRole::Default, || {})
      .into()
  });
  assert!(stage.paints("Details"));
  assert!(!stage.paints("os error 28: no space left on device"));
  press_text(&mut stage, "Details");
  assert!(detail.get());
  stage.pass_after(std::time::Duration::from_secs(1));
  let text = stage
    .text_quad("os error 28: no space left on device")
    .expect("the detail is open");
  assert_eq!(stage.text("os error 28: no space left on device").1, 11.0);
  let message = stage.text_quad("The disk is full.").unwrap();
  assert!(text.y > message.y, "under the message");
}

#[test]
fn a_theme_gives_the_two_buttons_of_a_message_box_fonts_of_their_own() {
  let app = App::new();
  app.theme().patch_recipe::<MessageBox>(|rules| {
    rules
      .base(MessageBoxPart::DefaultLabel, |s| s.font_family("Georgia"))
      .base(MessageBoxPart::CancelLabel, |s| s.font_family(FontFamily::Mono))
      .base(MessageBoxPart::DefaultButton, |s| s.background(PaletteColor::Success))
  });
  let stage = stage_in(app, |_| {
    Row::new()
      .child(Button::new("Elsewhere"))
      .child(replace(Signal::new(true), Signal::new(false), || {}, || {}))
      .into()
  });
  assert_eq!(stage.font_of("Replace"), "Georgia");
  assert_eq!(stage.font_of("Cancel"), "monospace");
  let system = stage.app.theme().sans_family().to_string();
  assert_eq!(
    stage.font_of("Elsewhere"),
    system,
    "a button elsewhere keeps the button's"
  );
  let success = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Success))
      && quad.height == 32.0
  });
  assert!(success, "the default answer is filled as the theme says");
}

#[test]
fn a_theme_gives_the_message_box_the_anatomy_of_a_mobile_alert() {
  // The board's mobile alert: the title and the message in the middle, and the answers as cells side by side,
  // the cancel one first. Other nodes, the same behavior.
  let app = App::new();
  let renderer: MessageBoxRenderer = Arc::new(|view: MessageBoxView| {
    let mut cells = Row::new();
    let cancel = view.find(MessageBoxRole::Cancel);
    let order = cancel
      .into_iter()
      .chain((0..view.answers.len()).filter(|index| Some(*index) != cancel));
    for index in order {
      let press = view.press(index);
      cells = cells.child(
        lurq::components::Button::new(&format!("[{}]", view.answers[index].label))
          .tab_index(0)
          .on_click(move |_| press()),
      );
    }
    let mut content = view.content();
    if let Some(title) = view.title() {
      content = content.child(title);
    }
    content = content.child(cells);
    view.dialog(content)
  });
  app.theme().set_renderer::<MessageBox>(renderer);
  let open = Signal::new(true);
  let (cancelled, cancel) = counter();
  let (deleted, delete_them) = counter();
  let held = open.clone();
  let mut stage = stage_in(app, move |_| {
    delete(held.clone(), cancel.clone(), delete_them.clone()).into()
  });
  assert!(stage.paints("[Cancel]") && stage.paints("[Delete]"));
  assert!(!stage.paints("This cannot be undone."));
  assert_eq!(
    stage.fill_color("box"),
    Some(light(PaletteColor::Popover)),
    "the dialog's panel"
  );
  press_text(&mut stage, "[Delete]");
  assert_eq!((count(&cancelled), count(&deleted)), (0, 1));
  assert!(!open.get());
  open.set(true);
  stage.pass();
  stage.key("Escape");
  assert_eq!((count(&cancelled), count(&deleted)), (1, 1));
}

#[test]
fn an_inline_message_box_is_its_panel_where_it_is_declared() {
  let (replaced, replace_it) = counter();
  let mut stage = stage(move |_| {
    replace(Signal::new(false), Signal::new(false), || {}, replace_it.clone())
      .inline(true)
      .into()
  });
  assert!(stage.paints("Replace existing file?"));
  assert!(fills(&stage, light(PaletteColor::Scrim), 600.0, 400.0).is_empty());
  // Enter with the focus nowhere is the page's, not the box's.
  stage.key("Enter");
  assert_eq!(count(&replaced), 0);
}
