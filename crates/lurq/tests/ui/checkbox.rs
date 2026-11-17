//! A checkbox: its box and its mark, what a press of it or of its label asks
//! for, its states, and a theme's other anatomy and rules.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::QuadContent,
  node::color::Color,
  ui::{Case, Checkbox, CheckboxPart, CheckboxRenderer, CheckboxState, CheckboxView, Interaction, Opacity, icons},
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

/// The color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

#[test]
fn a_checkbox_is_a_box_of_16_with_a_check_or_a_dash_in_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Send crash reports").id("off"))
      .child(Checkbox::new("Sync settings").checked(true).id("on"))
      .child(Checkbox::new("Select all").indeterminate(true).id("mixed"))
      .into()
  });
  for id in ["off", "on", "mixed"] {
    let (_, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (16.0, 16.0), "{id}");
  }
  let radius = stage.app.theme().radius_value(RadiusSize::Sm);
  let off = stage.fill("off").unwrap();
  assert!(
    off.border_radius.is_some_and(|corners| corners.top_left == radius),
    "the small radius"
  );

  // Empty it is the background edged in the color of an input's edge; checked or indeterminate, the primary color.
  assert_eq!(stage.fill_color("off"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "off"), Some(light(PaletteColor::Input)));
  for id in ["on", "mixed"] {
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Primary)), "{id}");
    assert_eq!(edge(&mut stage, id), Some(light(PaletteColor::Primary)), "{id}");
  }

  // The mark is a glyph of 12 in the color on the primary one: a check, and a dash while it is indeterminate.
  let (check, minus) = (glyph(icons::CHECK), glyph(icons::MINUS));
  assert_eq!(stage.text(&check), (light(PaletteColor::PrimaryForeground), 12.0));
  assert_eq!(stage.text(&minus), (light(PaletteColor::PrimaryForeground), 12.0));
  let painted = |content: &str| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
      .count()
  };
  assert_eq!((painted(&check), painted(&minus)), (1, 1), "nothing in the empty box");

  // The label is 8 beside the box, in the body's type and the foreground.
  let (x, ..) = stage.bounds("on");
  let label = stage.text_quad("Sync settings").unwrap();
  assert_eq!(label.x - (x + 16.0), 8.0);
  assert_eq!(stage.text("Sync settings"), (light(PaletteColor::Foreground), 14.0));
}

#[test]
fn a_press_of_the_box_or_of_the_label_asks_for_the_other_state() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = Stage::new(move |_| {
    let tell = |id: &'static str| {
      let told = told.clone();
      move |next| told.lock().unwrap().push((id, next))
    };
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Checkbox::new("Sync").id("off").on_change(tell("off")))
      .child(Checkbox::new("Reports").checked(true).id("on").on_change(tell("on")))
      .child(
        Checkbox::new("Select all")
          .indeterminate(true)
          .id("mixed")
          .on_change(tell("mixed")),
      )
      .child(
        Checkbox::new("Managed")
          .disabled(true)
          .id("disabled")
          .on_change(tell("disabled")),
      )
      .child(Checkbox::new("Hidden").label_hidden(true).id("bare"))
      .into()
  });
  // The keyboard presses the box: Tab reaches the first checkbox and then the next, and Space or Enter
  // presses the one it is on. The ring is the box's.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  stage.key("Tab");
  stage.key("Enter");
  stage.key("Tab");
  stage.key(" ");
  // The fourth is disabled: Tab passes it by, to the fifth, which tells nobody.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(
    *asked.lock().unwrap(),
    [("off", true), ("on", false), ("mixed", true)],
    "an indeterminate one asks to be checked"
  );
  asked.lock().unwrap().clear();

  for id in ["off", "on", "mixed", "disabled"] {
    stage.click(id);
  }
  assert_eq!(*asked.lock().unwrap(), [("off", true), ("on", false), ("mixed", true)]);

  // A press of the label is a press of the checkbox.
  let label = stage.text_quad("Sync").unwrap();
  let (x, y) = (label.x + label.width / 2.0, label.y + label.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(asked.lock().unwrap().last(), Some(&("off", true)));
  assert_eq!(asked.lock().unwrap().len(), 4);

  assert!(!stage.paints("Hidden"), "the box alone");
  let (_, _, width, _) = stage.bounds("bare");
  assert_eq!(width, 16.0);
}

#[test]
fn an_invalid_checkbox_is_edged_in_the_destructive_color() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Accept the terms").invalid(true).id("empty"))
      .child(Checkbox::new("Accept").invalid(true).checked(true).id("checked"))
      .into()
  });
  assert_eq!(stage.fill_color("empty"), Some(light(PaletteColor::Background)));
  assert_eq!(stage.fill_color("checked"), Some(light(PaletteColor::Primary)));
  for id in ["empty", "checked"] {
    assert_eq!(edge(&mut stage, id), Some(light(PaletteColor::Destructive)), "{id}");
  }
}

#[test]
fn a_disabled_checkbox_is_dimmed_with_its_label_and_its_empty_box_muted() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(12.0)
      .child(Checkbox::new("Managed").disabled(true).id("empty"))
      .child(
        Checkbox::new("Managed by policy")
          .disabled(true)
          .checked(true)
          .id("checked"),
      )
      .child(Checkbox::new("Some").disabled(true).indeterminate(true).id("mixed"))
      .into()
  });
  assert_eq!(stage.fill_color("empty"), Some(light(PaletteColor::Muted)));
  assert_eq!(edge(&mut stage, "empty"), Some(light(PaletteColor::Input)));
  for id in ["checked", "mixed"] {
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Primary)), "{id}");
  }

  // The root, which holds the box and the label, is dimmed by the theme's opacity for what is disabled.
  // It is a layer of what is in it, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(CheckboxState::Disabled, true);
  let root = theme.resolve::<Checkbox>(&case, None).part(CheckboxPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<Checkbox>(&Case::new(), None)
    .part(CheckboxPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Checkbox::new("Sync settings").force(Some(Interaction::FocusVisible)))
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_themes_rule_restyles_the_box_and_its_mark() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app.theme().patch_recipe::<Checkbox>(|rules| {
    rules
      .state(CheckboxState::Checked, CheckboxPart::Box, |s| s.background(RED))
      .base(CheckboxPart::Indicator, |s| s.square(14.0))
  });
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(Checkbox::new("Sync settings").checked(true).id("box"))
      .into()
  });
  assert_eq!(stage.fill_color("box"), Some(RED));
  assert_eq!(stage.text(&glyph(icons::CHECK)).1, 14.0);
}

#[test]
fn a_theme_gives_the_checkbox_another_anatomy_and_it_still_checks() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // A checkbox that says what it is in words inside its box: other nodes, the same behavior.
  let renderer: CheckboxRenderer = Arc::new(|view: CheckboxView| {
    let word = if view.checked { "YES" } else { "NO" };
    view.control().child(view.part(CheckboxPart::Label).text(word)).into()
  });
  app.theme().set_renderer::<Checkbox>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    Row::new()
      .padding(8.0)
      .child(
        Checkbox::new("Sync settings")
          .id("box")
          .on_change(move |next| told.lock().unwrap().push(next)),
      )
      .into()
  });
  assert!(stage.paints("NO"));
  assert!(!stage.paints("Sync settings"));
  assert_eq!(
    stage.fill_color("box"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.click("box");
  assert_eq!(*asked.lock().unwrap(), [true]);
}

#[test]
fn a_group_of_checkboxes_is_bound_to_a_list() {
  use lurq::{
    core::Signal,
    ui::{CheckboxGroup, Choice},
  };
  let days = Signal::new(vec![2u8]);
  let read = days.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(CheckboxGroup::new(
        read.clone(),
        [
          Choice::new(1u8, "Monday"),
          Choice::new(2, "Tuesday"),
          Choice::new(3, "Wednesday").disabled(true),
        ],
      ))
      .into()
  });
  // One under the other, 12 apart, as radios are.
  let (monday, tuesday) = (stage.text_quad("Monday").unwrap(), stage.text_quad("Tuesday").unwrap());
  assert_eq!(monday.x, tuesday.x);
  assert!((tuesday.y - monday.y - (monday.height.max(16.0) + 12.0)).abs() < 0.01);

  // Each box is a stop of Tab: Space checks the first, and the second, checked, is unchecked by its own.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(days.get(), [2, 1]);
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(days.get(), [1]);
  // The disabled one is passed by: the focus is back on the first.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(days.get(), Vec::<u8>::new());
}

#[test]
fn a_bound_checkbox_turns_its_signal_and_then_tells_its_handlers() {
  use lurq::core::Signal;
  let checked = Signal::new(false);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (checked.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let (seen, told) = (read.clone(), told.clone());
    Row::new()
      .padding(8.0)
      .child(
        // A handler given before the signal is still told after the signal is set.
        Checkbox::new("Keep me signed in")
          .id("box")
          .on_change(move |next| told.lock().unwrap().push((next, seen.get_untracked())))
          .bind(read.clone()),
      )
      .into()
  });
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Background)));
  stage.click("box");
  assert!(checked.get());
  assert_eq!(*heard.lock().unwrap(), [(true, true)]);
  stage.pass();
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Primary)));
  // The app sets the signal: the box is drawn as it says, and no handler is told.
  checked.set(false);
  stage.pass();
  assert_eq!(stage.fill_color("box"), Some(light(PaletteColor::Background)));
  assert_eq!(heard.lock().unwrap().len(), 1);
}

#[test]
fn a_group_tells_its_handlers_the_whole_new_set_and_not_what_the_app_sets() {
  use lurq::{
    core::Signal,
    ui::{CheckboxGroup, Choice},
  };
  let days = Signal::new(vec![2u8]);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (days.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        CheckboxGroup::new(read.clone(), [Choice::new(1u8, "Monday"), Choice::new(2, "Tuesday")])
          .on_change(move |days: &[u8]| told.lock().unwrap().push(days.to_vec())),
      )
      .into()
  });
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(days.get(), [2, 1]);
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(days.get(), [1]);
  assert_eq!(
    *heard.lock().unwrap(),
    [vec![2, 1], vec![1]],
    "the whole set, after the signal"
  );
  days.set(vec![1, 2]);
  stage.pass();
  assert_eq!(heard.lock().unwrap().len(), 2, "the app's own set tells nobody");
}

#[test]
fn a_group_the_app_holds_shows_what_the_app_answers() {
  use lurq::{
    core::Signal,
    ui::{CheckboxGroup, Choice},
  };
  // The app holds the set, and takes only a set of at most one: a second box pressed is refused.
  let held = Signal::new(vec!["read"]);
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (held.clone(), asked.clone());
  let mut stage = Stage::new(move |_| {
    let (set, told) = (read.clone(), told.clone());
    Column::new()
      .padding(8.0)
      .child(CheckboxGroup::controlled(
        read.get(),
        [Choice::new("read", "Read"), Choice::new("write", "Write")],
        move |next: &[&'static str]| {
          told.lock().unwrap().push(next.to_vec());
          if next.len() <= 1 {
            set.set(next.to_vec());
          }
        },
      ))
      .into()
  });
  let boxes = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .filter(|quad| {
        quad.width == 16.0
          && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Primary))
      })
      .count()
  };
  assert_eq!(boxes(&stage), 1);
  // Write pressed asks for both: the app refuses, and one box is still checked.
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*asked.lock().unwrap(), [vec!["read", "write"]]);
  stage.pass();
  assert_eq!(boxes(&stage), 1, "what the app holds, not what was pressed");
  assert_eq!(held.get(), ["read"]);
  // Read pressed asks for none: the app takes it.
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
  stage.key(" ");
  stage.pass();
  assert_eq!(held.get(), Vec::<&str>::new());
  assert_eq!(boxes(&stage), 0);
}

#[test]
fn a_group_has_an_orientation_its_states_and_its_rules() {
  use lurq::ui::{CheckboxGroup, CheckboxGroupPart, Choice, Orientation, Recipe};
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(40.0)
      .child(CheckboxGroup::controlled(
        Vec::<u8>::new(),
        [Choice::new(1u8, "One"), Choice::new(2, "Two")],
        |_| {},
      ))
      .child(
        CheckboxGroup::controlled(Vec::<u8>::new(), [Choice::new(3u8, "Three"), Choice::new(4, "Four")], |_| {})
          .orientation(Orientation::Horizontal)
          // The app's state, which its own rule styles, and the rules of what holds the group.
          .state("roomy", true)
          .rules(&Recipe::new().state("roomy", CheckboxGroupPart::Root, |s| s.gap(40.0)))
          .force(Some(Interaction::FocusVisible)),
      )
      .into()
  });
  let quad = |label: &str| stage.text_quad(label).unwrap();
  // One under the other unless told: the vertical state.
  assert_eq!(quad("One").x, quad("Two").x);
  assert!(quad("Two").y > quad("One").y);
  // Side by side, with the room the app's rule gives in place of 24.
  assert_eq!(quad("Three").y, quad("Four").y);
  let gap = quad("Four").x - 16.0 - 8.0 - (quad("Three").x + quad("Three").width);
  assert!((gap - 40.0).abs() < 0.01, "{gap}");
  // A forced focus draws the ring of the first box.
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_names_the_group_without_its_type_and_styles_a_group_of_any() {
  use lurq::ui::{CheckboxGroup, CheckboxGroupPart, Choice};
  let app = App::new();
  app
    .theme()
    .patch_recipe::<CheckboxGroup>(|rules| rules.base(CheckboxGroupPart::Root, |s| s.gap(30.0)));
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(CheckboxGroup::controlled(
        Vec::<u8>::new(),
        [Choice::new(1u8, "Monday"), Choice::new(2, "Tuesday")],
        |_| {},
      ))
      .into()
  });
  let (monday, tuesday) = (stage.text_quad("Monday").unwrap(), stage.text_quad("Tuesday").unwrap());
  assert!((tuesday.y - monday.y - (monday.height.max(16.0) + 30.0)).abs() < 0.01);
}

#[cfg(feature = "form")]
mod in_a_form {
  use std::sync::{Arc, Mutex};

  use lurq::{
    app::{component::Component, ctx::Ctx},
    components::{Column, FormHandle, FormOptions, FormValues},
    core::Signal,
    node::Element,
    ui::{Button, Checkbox},
  };

  use super::Stage;

  #[derive(Clone, Default)]
  struct Sent(Arc<Mutex<Option<FormValues>>>);

  #[cfg(feature = "devtools")]
  impl lurq::app::component::DevtoolsInspectable for Sent {
    fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
  }

  impl PartialEq for Sent {
    fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
    }
  }

  struct Settings {
    form: FormHandle,
    news: Signal<bool>,
  }

  impl Component for Settings {
    type Props = Sent;

    fn create(ctx: &mut Ctx) -> Self {
      let sent = ctx.props::<Sent>().0.clone();
      let form = ctx
        .form(FormOptions::new())
        .on_submit(move |values| *sent.lock().unwrap() = Some(values));
      Self {
        form,
        news: ctx.signal(false),
      }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      ctx.form_view(self.form.clone(), |_| {
        Column::new()
          .padding(10.0)
          .spacing(8.0)
          .child(
            Checkbox::new("Send me news")
              .bind(self.news.clone())
              .name("news")
              .id("news"),
          )
          .child(Button::new("Save").submit(true).id("save"))
      })
    }
  }

  #[test]
  fn a_named_checkbox_gives_the_form_on_while_it_is_checked_and_nothing_else() {
    let sent = Sent::default();
    let probe = sent.clone();
    let mut stage = Stage::new(move |ctx| ctx.mount::<Settings>(probe.clone()));
    let submitted = |sent: &Sent| sent.0.lock().unwrap().clone().expect("submitted");
    stage.click("save");
    assert_eq!(
      submitted(&sent).get_string("news"),
      None,
      "not checked, it gives nothing"
    );
    stage.click("news");
    stage.click("save");
    assert_eq!(submitted(&sent).get_string("news"), Some("on"));
  }
}
