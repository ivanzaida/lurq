//! A toast: the panel with its icon, text, action, close control and timer,
//! and what shows toasts from anywhere and takes them away after a while.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{ChromeTitleBar, Column, Row, WindowChrome, WindowChromeMode},
  layout::quad::QuadContent,
  node::color::Color,
  ui::{
    Button, ButtonSize, ButtonVariant, Case, Interaction, Toast, ToastPart, ToastPosition, ToastRenderer, ToastVariant,
    ToastView, Toasts, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// The filled rectangles of `color` and `height`, in the order they are painted.
fn bars(stage: &Stage, color: Color, height: f32) -> Vec<(f32, f32, f32)> {
  stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Rect { color: fill, .. } if fill == color && (quad.height - height).abs() < 0.01 => {
        Some((quad.x, quad.y, quad.width))
      }
      _ => None,
    })
    .collect()
}

/// A window of 400 by 300 that renders what `host` shows at `position`, and nothing else.
fn window(toasts: &Toasts, position: ToastPosition) -> Stage {
  let host = toasts.clone();
  // The page takes the whole window: a layer is painted where the page's root is, and it is as large as the
  // window.
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(host.host_at(ctx, position))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

#[test]
fn a_toast_is_a_panel_with_its_icon_its_text_and_its_close_control() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(40.0)
      .child(
        Toast::new("Settings saved")
          .description("Your theme applies to every open window.")
          .variant(ToastVariant::Success)
          .id("saved"),
      )
      .child(Toast::new("Link copied").id("copied"))
      .into()
  });
  let (x, y, width, _) = stage.bounds("saved");
  assert_eq!(width, 360.0);
  assert_eq!(stage.fill_color("saved"), Some(light(PaletteColor::Popover)));
  let edge = stage.border("saved").expect("an edge around the panel");
  assert_eq!(
    edge.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Border))
  );

  // The icon of its kind, 16 inside, in the color of success.
  let check = glyph(icons::CHECK_CIRCLE);
  assert_eq!(stage.text(&check), (light(PaletteColor::Success), 16.0));
  let icon = stage.text_quad(&check).unwrap();
  assert!(
    (icon.x - x - 16.0).abs() < 0.5 && (icon.y - y - 16.0).abs() < 0.5,
    "16 inside"
  );

  // The title 12 after the icon, in the body's size, over the description in a muted 13.
  let title = stage.text_quad("Settings saved").unwrap();
  assert!(
    (title.x - x - 44.0).abs() < 0.5,
    "16, the icon of 16, 12: {}",
    title.x - x
  );
  assert_eq!(
    stage.text("Settings saved"),
    (light(PaletteColor::PopoverForeground), 14.0)
  );
  assert_eq!(
    stage.text("Your theme applies to every open window."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let description = stage.text_quad("Your theme applies to every open window.").unwrap();
  assert!(description.y > title.y + title.height, "under the title");

  // The close control at the end, 16 from the edge, in the muted color.
  let close = glyph(icons::CLOSE);
  assert_eq!(stage.text(&close), (light(PaletteColor::MutedForeground), 16.0));
  let mark = stage.text_quad(&close).unwrap();
  assert!((x + width - (mark.x + mark.width) - 16.0).abs() < 0.5);

  // A neutral toast has no icon: its title is where the icon would be.
  let (x, ..) = stage.bounds("copied");
  let title = stage.text_quad("Link copied").unwrap();
  assert!((title.x - x - 16.0).abs() < 0.5);
}

#[test]
fn each_variant_has_its_icon_in_its_color() {
  let stage = Stage::new(|_| {
    let mut column = Column::new().spacing(4.0);
    for variant in [ToastVariant::Info, ToastVariant::Warning, ToastVariant::Error] {
      column = column.child(Toast::new(variant.as_str()).variant(variant).closable(false));
    }
    column.into()
  });
  assert_eq!(stage.text(&glyph(icons::INFO)).0, light(PaletteColor::Info));
  assert_eq!(stage.text(&glyph(icons::WARNING)).0, light(PaletteColor::Warning));
  assert_eq!(
    stage.text(&glyph(icons::CLOSE_CIRCLE)).0,
    light(PaletteColor::Destructive)
  );
  assert!(!stage.paints(&glyph(icons::CLOSE)), "not closable: no close control");

  // An icon of its own in place of the variant's, or none.
  let stage = Stage::new(|_| {
    Column::new()
      .child(Toast::new("Copied").icon(icons::COPY))
      .child(Toast::new("Failed").variant(ToastVariant::Error).icon_hidden(true))
      .into()
  });
  assert_eq!(
    stage.text(&glyph(icons::COPY)).0,
    light(PaletteColor::PopoverForeground)
  );
  assert!(!stage.paints(&glyph(icons::CLOSE_CIRCLE)));
}

#[test]
fn the_close_control_and_the_action_are_pressed_by_the_pointer_and_the_keyboard() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let (closed, undone) = (told.clone(), told.clone());
    Row::new()
      .padding(8.0)
      .child(
        Toast::new("File moved to trash")
          .description("theme-dark.json")
          .action(
            Button::new("Undo")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .id("undo")
              .on_click(move |_| undone.lock().unwrap().push("undo")),
          )
          .on_close(move || closed.lock().unwrap().push("close")),
      )
      .into()
  });
  // Tab reaches the action and then the close control; the ring is the close control's.
  stage.key("Tab");
  stage.key("Enter");
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(*asked.lock().unwrap(), ["undo", "close"]);

  stage.click("undo");
  let mark = stage.text_quad(&glyph(icons::CLOSE)).unwrap();
  let (x, y) = (mark.x + mark.width / 2.0, mark.y + mark.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(*asked.lock().unwrap(), ["undo", "close", "undo", "close"]);
}

#[test]
fn with_an_action_it_is_lower_and_centered_on_it_with_its_timer_under_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Toast::new("File moved to trash")
          .description("theme-dark.json")
          .variant(ToastVariant::Success)
          .action(
            Button::new("Undo")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .id("undo"),
          )
          .timer(true)
          .id("toast"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("toast");
  let (_, undo_y, _, undo_height) = stage.bounds("undo");
  // 14 above and under what is in it: the title and the description, which are taller than the button, start 14
  // from the top and end 14 over the timer.
  let title = stage.text_quad("File moved to trash").unwrap();
  let description = stage.text_quad("theme-dark.json").unwrap();
  let (top, bottom) = (title.y - y, y + height - 2.0 - (description.y + description.height));
  assert!(top >= 14.0 - 0.5 && top <= 16.0, "the text 14 from the top: {top}");
  assert!(bottom >= 12.0 && bottom <= 14.5, "the text 14 over the timer: {bottom}");
  // The title 2 over the description.
  assert!((description.y - (title.y + title.height) - 2.0).abs() < 2.0);
  // The action is centered on what is in it, between the top and the timer.
  let middle = y + (height - 2.0) / 2.0;
  assert!(
    (undo_y + undo_height / 2.0 - middle).abs() < 0.5,
    "centered: {} and {middle}",
    undo_y + undo_height / 2.0
  );
  assert!(
    undo_y - y > 14.0,
    "lower than the padding: the text is taller than the button"
  );
  // The timer is a rail of 2 along the bottom edge, all of it left while nothing counts it down.
  let rail = bars(&stage, light(PaletteColor::Secondary), 2.0);
  assert_eq!(rail.len(), 1);
  let left = bars(&stage, light(PaletteColor::Primary), 2.0);
  assert_eq!(left.len(), 1);
  assert!(
    (left[0].0 - x).abs() < 1.5 && left[0].2 >= width - 2.0,
    "all of it: {:?}",
    left[0]
  );
  assert!(rail[0].1 + 2.0 <= y + height + 0.01 && rail[0].1 >= undo_y + undo_height + 13.0);
}

#[test]
fn a_toast_shown_by_a_host_leaves_after_its_duration_or_on_its_close_control() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::default());
  assert!(!stage.paints("Settings saved"));

  toasts.show(
    Toast::new("Settings saved")
      .variant(ToastVariant::Success)
      .duration(Duration::from_secs(3)),
  );
  stage.pass();
  assert!(stage.paints("Settings saved"));
  // At the bottom right of the window, 16 from its edges.
  let title = stage.text_quad("Settings saved").unwrap();
  assert!((title.x - (400.0 - 16.0 - 360.0 + 44.0)).abs() < 0.5, "{}", title.x);
  assert!(title.y > 200.0);

  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  stage.pass();
  assert!(stage.paints("Settings saved"), "not yet");
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(4));
  stage.pass();
  assert!(!stage.paints("Settings saved"), "its time is up");
  assert!(toasts.is_empty());

  // One that stays until it is closed: Tab reaches its close control from the page.
  toasts.show(
    Toast::new("Could not connect")
      .variant(ToastVariant::Error)
      .persistent(true),
  );
  stage.pass();
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(60));
  stage.pass();
  assert!(stage.paints("Could not connect"));
  stage.key("Tab");
  stage.key("Enter");
  assert!(!stage.paints("Could not connect"));
  assert!(toasts.is_empty());

  // And one taken away by the app.
  let id = toasts.show(Toast::new("Syncing"));
  stage.pass();
  assert!(stage.paints("Syncing"));
  toasts.dismiss(id);
  stage.pass();
  assert!(!stage.paints("Syncing"));
}

#[test]
fn a_press_of_its_action_takes_a_toast_away() {
  let undone = Arc::new(Mutex::new(0));
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::default());
  let told = undone.clone();
  toasts.show(
    Toast::new("File moved to trash").action(
      Button::new("Undo")
        .id("undo")
        .on_click(move |_| *told.lock().unwrap() += 1),
    ),
  );
  stage.pass();
  stage.click("undo");
  assert_eq!(*undone.lock().unwrap(), 1);
  assert!(!stage.paints("File moved to trash"));
}

#[test]
fn several_are_stacked_and_laid_out_one_over_another_while_the_pointer_is_on_them() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::BottomRight);
  for title in ["First", "Second", "Third", "Fourth"] {
    toasts.show(Toast::new(title).duration(Duration::from_secs(3)));
  }
  stage.pass();
  // Stacked: the newest in front, the edges of two behind it. The fourth waits.
  assert!(stage.paints("Fourth"));
  for title in ["First", "Second", "Third"] {
    assert!(!stage.paints(title), "{title} is behind");
  }
  let edges = bars(&stage, light(PaletteColor::Popover), 24.0);
  assert_eq!(edges.len(), 2, "two edges behind: {edges:?}");
  let front = stage.text_quad("Fourth").unwrap();
  assert!(edges.iter().all(|edge| edge.1 < front.y), "above the front one");

  // The pointer on them: one over another, the newest at the bottom, and their time stands still.
  let (x, y) = (front.x + 4.0, front.y + 4.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  for title in ["Second", "Third", "Fourth"] {
    assert!(stage.paints(title), "{title}");
  }
  assert!(!stage.paints("First"), "three at once");
  let (second, fourth) = (stage.text_quad("Second").unwrap(), stage.text_quad("Fourth").unwrap());
  assert!(second.y < fourth.y);
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(10));
  stage.pass();
  assert_eq!(toasts.len(), 4, "none left while the pointer is on them");

  // The pointer gone: stacked again, and the time of the three shown runs on, those behind too.
  stage.tree.mouse_move(1.0, 1.0);
  stage.pass();
  // They are drawn while they move back behind the front one, and not once they are there.
  assert!(stage.paints("Third"));
  let (third, fourth) = (stage.text_quad("Third").unwrap(), stage.text_quad("Fourth").unwrap());
  assert!((third.y - fourth.y).abs() < 0.01, "back where the front one stands");
  stage.tree.tick_timers_at(Instant::now() + Duration::from_millis(500));
  stage.pass();
  assert!(!stage.paints("Third"));
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(4));
  stage.pass();
  assert_eq!(toasts.len(), 1, "the three shown have left");
  assert!(stage.paints("First"), "the one that waited comes to the front");
}

#[test]
fn a_toast_comes_in_and_leaves_by_animations_that_reduced_motion_goes_without() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::BottomRight);
  let id = toasts.show(Toast::new("Settings saved"));
  stage.pass();
  let resting = stage.text_quad("Settings saved").unwrap().y;
  // Taken away, it is painted on its way down until its animation ends.
  toasts.dismiss(id);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(80));
  let leaving = stage
    .text_quad("Settings saved")
    .expect("still painted while it leaves");
  assert!(leaving.y > resting, "on its way down: {} after {resting}", leaving.y);
  stage.pass();
  assert!(!stage.paints("Settings saved"));

  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let host = toasts.clone();
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new().size(400.0, 300.0).child(host.host(ctx)).into()
  });
  stage.tree.resize(400, 300);
  let id = toasts.show(Toast::new("At once"));
  stage.pass();
  toasts.dismiss(id);
  stage.pass_after(Duration::ZERO);
  assert!(!stage.paints("At once"));
}

#[test]
fn the_timer_stands_still_with_what_is_left_while_the_pointer_is_on_it() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::BottomRight);
  toasts.show(Toast::new("Moved").timer(true).duration(Duration::from_secs(60)));
  stage.pass();
  let full = bars(&stage, light(PaletteColor::Primary), 2.0)[0].2;
  assert_eq!(full, 360.0, "all of it, drawn first where it is");
  // Rendered again, it runs out; under the pointer it stands where it is, all but nothing gone of a minute.
  stage.tree.tick_timers_at(Instant::now() + Duration::from_millis(1));
  stage.pass();
  let title = stage.text_quad("Moved").unwrap();
  stage.tree.mouse_move(title.x + 2.0, title.y + 2.0);
  stage.pass();
  let held = bars(&stage, light(PaletteColor::Primary), 2.0)[0].2;
  assert!(held > 300.0 && held <= 360.0, "{held}");
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(120));
  stage.pass();
  assert_eq!(toasts.len(), 1);
}

#[test]
fn toasts_at_the_top_of_a_window_with_its_own_title_bar_stand_under_it() {
  let at = |chrome: bool| {
    let toasts = Toasts::new();
    let host = toasts.clone();
    let mut stage = Stage::new(move |ctx| {
      let page = Column::new()
        .size(400.0, 300.0)
        .child(host.host_at(ctx, ToastPosition::TopLeft));
      if chrome {
        WindowChrome::new()
          .mode(WindowChromeMode::AlwaysCustom)
          .title_bar(ChromeTitleBar::new().height(44.0))
          .content(page)
          .mount(ctx)
      } else {
        page.into()
      }
    });
    stage.tree.resize(400, 300);
    stage.pass();
    toasts.show(Toast::new("Up here"));
    stage.pass();
    stage.text_quad("Up here").unwrap().y
  };
  let (plain, under) = (at(false), at(true));
  assert!((under - plain - 44.0).abs() < 0.5, "{plain} {under}");
}

#[test]
fn toasts_at_the_top_are_in_the_top_corner() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::TopLeft);
  toasts.show(Toast::new("Up here"));
  stage.pass();
  let title = stage.text_quad("Up here").unwrap();
  assert!(
    (title.x - 32.0).abs() < 0.5 && title.y < 50.0,
    "{} {}",
    title.x,
    title.y
  );
}

#[test]
fn a_forced_focus_draws_the_ring_of_the_close_control() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Toast::new("Settings saved").force(Some(Interaction::FocusVisible)))
      .into()
  });
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_restyles_a_part_or_gives_the_toast_another_anatomy() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        Toast::new("Narrow")
          .style(ToastPart::Root, |s| s.width(300.0))
          .id("narrow"),
      )
      .into()
  });
  assert_eq!(stage.bounds("narrow").2, 300.0);

  let closed = Arc::new(Mutex::new(0));
  let told = closed.clone();
  let app = App::new();
  // A toast that is its title and its close control alone, in a row.
  let renderer: ToastRenderer = Arc::new(|view: ToastView| {
    let title = view.part(ToastPart::Title);
    Row::new()
      .child(title.text(&view.title.to_uppercase()))
      .child(view.close().id("close"))
      .into()
  });
  app.theme().set_renderer::<Toast>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(Toast::new("Saved").on_close(move || *told.lock().unwrap() += 1))
      .into()
  });
  assert!(stage.paints("SAVED"));
  stage.click("close");
  assert_eq!(*closed.lock().unwrap(), 1);

  // The variant is what the recipe is resolved for.
  let theme = stage.app.theme().clone();
  let case = Case::new().variant(ToastVariant::Error);
  let icon = theme.resolve::<Toast>(&case, None).part(ToastPart::Icon).rest();
  assert!(icon.foreground.is_some());
}

#[test]
fn a_toast_that_comes_while_another_is_shown_comes_in_over_it_and_moves_nothing() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::BottomRight);
  toasts.show(Toast::new("First").duration(Duration::from_secs(30)));
  stage.pass();
  stage.pass_after(Duration::from_secs(2));
  let first = stage.text_quad("First").unwrap();

  // The second comes in over the first: the first is still drawn, where the one in front stands (an edge's
  // peek lower than it stood alone, for the edge now behind), and not a toast's height away, where a list
  // would have put it while it left.
  toasts.show(Toast::new("Second").duration(Duration::from_secs(30)));
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(30));
  let under = stage
    .text_quad("First")
    .expect("the first is still drawn under the second");
  assert_eq!(under.x, first.x);
  assert!(
    (under.y - first.y).abs() <= 8.01,
    "the first moved by {}",
    under.y - first.y
  );
  let second = stage.text_quad("Second").unwrap();
  assert!(
    second.y >= under.y - 0.01,
    "the second comes in from under its place: {} {}",
    second.y,
    under.y
  );

  // In, it stands where the first stood. The first fades under it, its shadow with it, and is then an edge
  // behind it, with nothing else drawn of it.
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(1));
  stage.pass_after(Duration::from_secs(1));
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  stage.pass_after(Duration::from_secs(1));
  stage.pass();
  assert!(!stage.paints("First"));
  let second = stage.text_quad("Second").unwrap();
  assert!((second.y - under.y).abs() < 0.01);
  assert_eq!(
    bars(&stage, light(PaletteColor::Popover), 24.0).len(),
    1,
    "one edge behind"
  );
}

#[test]
fn the_front_toast_of_a_pile_closed_shows_the_next_in_its_place() {
  let toasts = Toasts::new();
  let mut stage = window(&toasts, ToastPosition::BottomRight);
  toasts.show(Toast::new("First").duration(Duration::from_secs(30)));
  let second = toasts.show(Toast::new("Second").duration(Duration::from_secs(30)));
  stage.pass();
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(1));
  stage.pass_after(Duration::from_secs(2));
  let front = stage.text_quad("Second").unwrap();
  assert!(!stage.paints("First"));

  toasts.dismiss(second);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_secs(2));
  stage.pass();
  assert!(!stage.paints("Second"));
  let first = stage.text_quad("First").expect("the next is in front");
  assert_eq!(first.x, front.x);
  // Alone it has no edge behind it: an edge's peek nearer the window's edge than the front of two stood.
  assert!((first.y - front.y).abs() <= 8.01);
  assert!(bars(&stage, light(PaletteColor::Popover), 24.0).is_empty());
}

#[test]
fn a_description_too_long_for_its_line_goes_on_to_the_next() {
  use lurq::{components::Column, ui::Button};
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Toast::new("Settings saved")
          .description("Your theme applies to every open window.")
          .action(Button::new("Undo").id("undo"))
          .id("toast"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("toast");
  let (undo, ..) = stage.bounds("undo");
  // Beside an action there is no room for it in one line: it is in two, each ending before the action, and
  // nothing of it is cut.
  let lines: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. }
        if "Your theme applies to every open window.".contains(text.trim()) && !text.trim().is_empty())
    })
    .collect();
  assert!(!lines.is_empty(), "the description is painted");
  let painted: String = lines
    .iter()
    .map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, .. } => text.trim().to_owned(),
      _ => String::new(),
    })
    .collect::<Vec<_>>()
    .join(" ");
  assert_eq!(painted, "Your theme applies to every open window.", "all of it");
  for line in &lines {
    assert!(
      line.x >= x && line.x + line.width <= undo + 0.01,
      "a line ends before the action"
    );
    assert!(
      !line.clip.active || line.clip.x + line.clip.width >= line.x + line.width - 0.01,
      "and is not cut"
    );
  }
  assert!(lines.len() >= 2 || lines[0].height > 30.0, "on two lines");
  assert_eq!(width, 360.0);
}
