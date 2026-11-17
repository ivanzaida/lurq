//! The page of the pointer's events: what an element of the engine is told when the pointer presses it, presses
//! it twice, moves over it, comes in and goes out. Its playground is a box whose handlers are its props, and the
//! Events tab is the log of what it was told; its sections are the demo's Events page: two buttons and their log,
//! the states an element is painted in, and a box that follows the pointer. Then where a press goes when one
//! element is in another.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEvent},
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element},
  ui::Switch,
};

use super::kit::{sample, samples, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The pointers the playground's box may show, by the name the code gives them.
const CURSORS: [(&str, CursorIcon); 5] = [
  ("Pointer", CursorIcon::Pointer),
  ("Crosshair", CursorIcon::Crosshair),
  ("Grab", CursorIcon::Grab),
  ("Text", CursorIcon::Text),
  ("Default", CursorIcon::Default),
];
/// The choices of the playground's select: an index into [`CURSORS`], which a signal keeps.
const CURSOR_CHOICES: [(&str, usize); 5] = [
  ("Pointer", 0),
  ("Crosshair", 1),
  ("Grab", 2),
  ("Text", 3),
  ("Default", 4),
];
/// How many lines a log of a section keeps: the latest first.
const LOG: usize = 4;

/// What a button of the mouse is called in a log.
fn button_name(button: MouseButton) -> &'static str {
  match button {
    MouseButton::Left => "Left",
    MouseButton::Right => "Right",
    MouseButton::Middle => "Middle",
    MouseButton::Back => "Back",
    MouseButton::Forward => "Forward",
    MouseButton::Other(_) => "Other",
  }
}

/// Where a press was, and the keys held with it: "(142, 87) shift".
fn at(event: &MouseEvent) -> String {
  let mut said = format!("({:.0}, {:.0})", event.x, event.y);
  for (held, name) in [
    (event.shift, "shift"),
    (event.ctrl, "ctrl"),
    (event.alt, "alt"),
    (event.meta, "meta"),
  ] {
    if held {
      said.push(' ');
      said.push_str(name);
    }
  }
  said
}

/// Puts `line` at the top of a log that keeps its last [`LOG`] lines.
fn push(log: &Signal<Vec<String>>, line: String) {
  log.update(|lines| {
    lines.insert(0, line);
    lines.truncate(LOG);
  });
}

/// The lines of a log in a well, the latest first; `empty` while there are none.
fn log_view(lines: &[String], empty: &str) -> Column {
  let mut column = well().padding(12.0).spacing(4.0).min_height(120.0);
  if lines.is_empty() {
    column = column.child(quiet(empty, 12.0));
  }
  for line in lines {
    column = column.child(mono(line, 12.0).width(FILL));
  }
  column
}

/// A box of the engine dressed as a button: the primary fill, a lighter one under the pointer, and a dimmer one
/// while it is pressed. What it does is the handlers it is given.
fn pressable(label: &str) -> Row {
  Row::new()
    .height(40.0)
    .padding_horizontal(16.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Primary)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.background(PaletteColor::PrimaryHover))
    .active(|style| style.background(PaletteColor::PrimaryHover).opacity(0.8))
    .child(
      text(label, 14.0, FontWeight::SemiBold)
        .color(PaletteColor::PrimaryForeground)
        .nowrap(),
    )
}

pub struct EventsPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`CURSORS`].
  cursor: Signal<usize>,
  click: Signal<bool>,
  dblclick: Signal<bool>,
  down_up: Signal<bool>,
  enter_leave: Signal<bool>,
  moves: Signal<bool>,
  states: Signal<bool>,
  /// The log of the two buttons of the first section, the latest first.
  log: Signal<Vec<String>>,
  /// What the box of the states section was told: whether the pointer is over it, and whether it is pressed.
  over: Signal<bool>,
  pressed: Signal<bool>,
  /// The box that follows the pointer, and where the pointer was in it and in the window.
  area: ElementRef,
  inside: Signal<bool>,
  window_x: Signal<f32>,
  window_y: Signal<f32>,
  box_x: Signal<f32>,
  box_y: Signal<f32>,
  /// The section of a box in a box: whether the inner one stops the press, and what each was told.
  stop: Signal<bool>,
  nested: Signal<Vec<String>>,
}

impl EventsPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let cursor_at = self.cursor.get();
    let (cursor_name, cursor) = CURSORS[cursor_at.min(CURSORS.len() - 1)];
    let (click, dblclick, down_up) = (self.click.get(), self.dblclick.get(), self.down_up.get());
    let (enter_leave, moves, states) = (self.enter_leave.get(), self.moves.get(), self.states.get());
    let record = self.playground.recorder();
    let build = move || -> Element {
      let mut target = Row::new()
        .width(260.0)
        .max_width(FILL)
        .height(120.0)
        .padding(16.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .rounded(RadiusSize::Lg)
        .background(PaletteColor::Primary)
        .cursor(cursor)
        .child(
          text("Press me, twice, or move over me", 14.0, FontWeight::Medium).color(PaletteColor::PrimaryForeground),
        );
      if states {
        target = target
          .hovered(|style| style.background(PaletteColor::PrimaryHover))
          .active(|style| style.background(PaletteColor::PrimaryHover).opacity(0.8));
      }
      if click {
        let record = record.clone();
        target = target.on_click(move |event: MouseEvent| record(format!("on_click at {}", at(&event))));
      }
      if dblclick {
        let record = record.clone();
        target = target.on_dblclick(move |event: MouseEvent| record(format!("on_dblclick at {}", at(&event))));
      }
      if down_up {
        let down = record.clone();
        let up = record.clone();
        target = target
          .on_mouse_down(move |event: MouseEvent| down(format!("on_mouse_down {}", button_name(event.button))))
          .on_mouse_up(move |event: MouseEvent| up(format!("on_mouse_up {}", button_name(event.button))));
      }
      if enter_leave {
        let enter = record.clone();
        let leave = record.clone();
        target = target
          .on_mouse_enter(move || enter("on_mouse_enter".to_owned()))
          .on_mouse_leave(move || leave("on_mouse_leave".to_owned()));
      }
      if moves {
        let record = record.clone();
        target = target.on_mouse_move(move |event: MouseEvent| record(format!("on_mouse_move to {}", at(&event))));
      }
      target.into()
    };

    let mut code = vec![
      "Row::new()".to_owned(),
      "    .size(260.0, 120.0)".to_owned(),
      "    .background(PaletteColor::Primary)".to_owned(),
    ];
    if cursor != CursorIcon::Default {
      code.push(format!("    .cursor(CursorIcon::{cursor_name})"));
    }
    if states {
      code.push("    .hovered(|style| style.background(PaletteColor::PrimaryHover))".to_owned());
      code.push("    .active(|style| style.background(PaletteColor::PrimaryHover).opacity(0.8))".to_owned());
    }
    if click {
      code.push("    .on_click(|event: MouseEvent| log(event.x, event.y))".to_owned());
    }
    if dblclick {
      code.push("    .on_dblclick(|event: MouseEvent| log(event.x, event.y))".to_owned());
    }
    if down_up {
      code.push("    .on_mouse_down(|event: MouseEvent| log(event.button))".to_owned());
      code.push("    .on_mouse_up(|event: MouseEvent| log(event.button))".to_owned());
    }
    if enter_leave {
      code.push("    .on_mouse_enter(|| log(\"enter\"))".to_owned());
      code.push("    .on_mouse_leave(|| log(\"leave\"))".to_owned());
    }
    if moves {
      code.push("    .on_mouse_move(|event: MouseEvent| log(event.x, event.y))".to_owned());
    }
    code.push("    .child(Text::new(\"Press me, twice, or move over me\"))".to_owned());

    let handlers = [click, dblclick, down_up, down_up, enter_leave, enter_leave, moves]
      .into_iter()
      .filter(|on| *on)
      .count();
    let tree = vec![
      format!("row   {}   {handlers} handlers", self.playground.measured()),
      "└─ text   \"Press me, twice, or move over me\"".to_owned(),
    ];

    let set_cursor = self.cursor.clone();
    let reset = {
      let (cursor, click, dblclick, down_up) = (
        self.cursor.clone(),
        self.click.clone(),
        self.dblclick.clone(),
        self.down_up.clone(),
      );
      let (enter_leave, moves, states) = (self.enter_leave.clone(), self.moves.clone(), self.states.clone());
      Arc::new(move || {
        cursor.set(0);
        click.set(true);
        dblclick.set(true);
        down_up.set(false);
        enter_leave.set(false);
        moves.set(false);
        states.set(true);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Events",
        make: Arc::new(build.clone()),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("{handlers} handlers"),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop(
            "cursor",
            "CursorIcon",
            self
              .playground
              .select(0, &CURSOR_CHOICES, &cursor_at, move |value| set_cursor.set(value)),
          ),
          divider(),
          flag("on_click", &self.click),
          flag("on_dblclick", &self.dblclick),
          flag("on_mouse_down, on_mouse_up", &self.down_up),
          flag("on_mouse_enter, on_mouse_leave", &self.enter_leave),
          flag("on_mouse_move", &self.moves),
          // Not handlers: the styles of the states the engine keeps for the box, which change how it looks and
          // tell the Events tab nothing.
          divider(),
          flag("hovered(..), active(..) styles", &self.states),
        ],
        changed: [cursor_at != 0, !click, !dblclick, down_up, enter_leave, moves, !states]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// Two buttons and the log of what they were told: the first hears a press, its button going down and coming
  /// up; the second a press and a press twice.
  fn clicks(&self) -> Column {
    let log = &self.log;
    let once = {
      let (click, down, up) = (log.clone(), log.clone(), log.clone());
      pressable("Click me")
        .on_click(move |event: MouseEvent| push(&click, format!("click at {}", at(&event))))
        .on_mouse_down(move |event: MouseEvent| push(&down, format!("mouse_down {}", button_name(event.button))))
        .on_mouse_up(move |event: MouseEvent| push(&up, format!("mouse_up {}", button_name(event.button))))
    };
    let twice = {
      let (click, double) = (log.clone(), log.clone());
      pressable("Double-click me")
        .on_click(move |event: MouseEvent| push(&click, format!("click at {}", at(&event))))
        .on_dblclick(move |event: MouseEvent| push(&double, format!("dblclick at {}", at(&event))))
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(Row::new().width(FILL).spacing(12.0).wrap().child(once).child(twice))
      .child(
        Column::new()
          .width(FILL)
          .spacing(6.0)
          .child(quiet("Event log, the latest first", 12.0))
          .child(log_view(&log.get(), "Nothing yet: press a button.")),
      )
  }

  /// The three states a box is painted in, each drawn as it is and each live, and a fourth that says what its
  /// handlers were told.
  fn states(&self) -> Row {
    let block = |label: &str| {
      Row::new()
        .width(140.0)
        .height(44.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .rounded(RadiusSize::Md)
        .cursor(CursorIcon::Pointer)
        .hovered(|style| style.background(PaletteColor::PrimaryHover))
        .active(|style| style.background(PaletteColor::PrimaryHover).opacity(0.8))
        .child(
          text(label, 14.0, FontWeight::SemiBold)
            .color(PaletteColor::PrimaryForeground)
            .nowrap(),
        )
    };
    let (over, pressed) = (self.over.get(), self.pressed.get());
    let told = if pressed {
      "pressed"
    } else if over {
      "hovered"
    } else {
      "at rest"
    };
    let (enter, leave, down, up) = (
      self.over.clone(),
      self.over.clone(),
      self.pressed.clone(),
      self.pressed.clone(),
    );
    let telling = block("Button")
      .background(PaletteColor::Primary)
      .on_mouse_enter(move || enter.set(true))
      .on_mouse_leave({
        let pressed = self.pressed.clone();
        move || {
          leave.set(false);
          pressed.set(false);
        }
      })
      .on_mouse_down(move |_| down.set(true))
      .on_mouse_up(move |_| up.set(false));
    samples()
      .child(sample(block("Button").background(PaletteColor::Primary), "at rest"))
      .child(sample(
        block("Button").background(PaletteColor::PrimaryHover),
        "hovered",
      ))
      .child(sample(
        block("Button").background(PaletteColor::PrimaryHover).opacity(0.8),
        "active",
      ))
      .child(sample(telling, &format!("its handlers say: {told}")))
  }

  /// A box that follows the pointer: where it is in the window and in the box, and whether it is in.
  fn tracking(&self) -> Column {
    let (inside, window_x, window_y) = (self.inside.get(), self.window_x.get(), self.window_y.get());
    let (box_x, box_y) = (self.box_x.get(), self.box_y.get());
    let said = if inside || window_x != 0.0 || window_y != 0.0 {
      format!("x: {window_x:.0}  y: {window_y:.0}  |  entered: {inside}")
    } else {
      "Move the pointer over this box".to_owned()
    };
    let area = self.area.clone();
    let (to_x, to_y, to_box_x, to_box_y, to_inside) = (
      self.window_x.clone(),
      self.window_y.clone(),
      self.box_x.clone(),
      self.box_y.clone(),
      self.inside.clone(),
    );
    let (enter, leave) = (self.inside.clone(), self.inside.clone());
    Column::new()
      .width(FILL)
      .spacing(8.0)
      .child(
        Column::new()
          .width(FILL)
          .height(96.0)
          .padding(12.0)
          .spacing(4.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .border(super::kit::well_line())
          .cursor(CursorIcon::Crosshair)
          .ref_element(self.area.clone())
          .on_mouse_move(move |event: MouseEvent| {
            to_x.set(event.x);
            to_y.set(event.y);
            to_box_x.set(event.x - area.x());
            to_box_y.set(event.y - area.y());
            if !to_inside.get() {
              to_inside.set(true);
            }
          })
          .on_mouse_enter(move || enter.set(true))
          .on_mouse_leave(move || leave.set(false))
          .child(mono(&said, 14.0))
          .child(quiet(&format!("in the box: {box_x:.0}, {box_y:.0}"), 12.0)),
      )
      .child(quiet(
        "on_mouse_move gives the pointer in the window; less the box's own place, it is in the box.",
        12.0,
      ))
  }

  /// A box in a box, each with a handler of the press: the inner one may stop it.
  fn nested(&self) -> Column {
    let stop = self.stop.get();
    let (outer_log, inner_log, set_stop) = (self.nested.clone(), self.nested.clone(), self.stop.clone());
    let inner = pressable("Inner")
      .width(160.0)
      .max_width(FILL)
      .on_click(move |event: MouseEvent| {
        if stop {
          event.stop_propagation();
          push(&inner_log, "inner: click, stopped".to_owned());
        } else {
          push(&inner_log, "inner: click".to_owned());
        }
      });
    let outer = Column::new()
      .width(320.0)
      .max_width(FILL)
      .padding(24.0)
      .spacing(10.0)
      .align_items(Alignment::Center)
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Muted)
      .border(super::kit::well_line())
      .cursor(CursorIcon::Pointer)
      .on_click(move |_| push(&outer_log, "outer: click".to_owned()))
      .child(quiet("Outer", 12.0))
      .child(inner);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Switch::new("stop_propagation in the inner box")
          .on(stop)
          .on_change(move |on| set_stop.set(on)),
      )
      .child(outer)
      .child(log_view(
        &self.nested.get(),
        "Nothing yet: press the inner box, then the outer.",
      ))
  }
}

impl Component for EventsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      cursor: ctx.signal(0),
      click: ctx.signal(true),
      dblclick: ctx.signal(true),
      down_up: ctx.signal(false),
      enter_leave: ctx.signal(false),
      moves: ctx.signal(false),
      states: ctx.signal(true),
      log: ctx.signal(Vec::new()),
      over: ctx.signal(false),
      pressed: ctx.signal(false),
      area: ElementRef::new(),
      inside: ctx.signal(false),
      window_x: ctx.signal(0.0),
      window_y: ctx.signal(0.0),
      box_x: ctx.signal(0.0),
      box_y: ctx.signal(0.0),
      stop: ctx.signal(false),
      nested: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "on_click",
        "Fn(MouseEvent)",
        "",
        "A press and a release on the element: the button, where, and the keys held. A key that presses a \
         focused control is a click too, from_keyboard.",
      ),
      (
        "on_dblclick",
        "Fn(MouseEvent)",
        "",
        "A second press soon after the first, near it. Each of the two presses is a click as well.",
      ),
      (
        "on_mouse_down",
        "Fn(MouseEvent)",
        "",
        "A button going down on the element, any button: event.button says which.",
      ),
      (
        "on_mouse_up",
        "Fn(MouseEvent)",
        "",
        "A button coming up on the element.",
      ),
      (
        "on_mouse_move",
        "Fn(MouseEvent)",
        "",
        "Each move of the pointer over the element, with where it is in the window.",
      ),
      ("on_mouse_enter", "Fn()", "", "The pointer comes in over the element."),
      ("on_mouse_leave", "Fn()", "", "The pointer goes out of the element."),
      (
        "hovered",
        "FnOnce(Style) -> Style",
        "",
        "What changes in the element's look while the pointer is over it. It paints; it changes no size.",
      ),
      (
        "active",
        "FnOnce(Style) -> Style",
        "",
        "What changes while it is pressed, over hovered.",
      ),
      (
        "cursor",
        "CursorIcon",
        "Default",
        "The pointer's shape over the element.",
      ),
      (
        "stop_propagation",
        "MouseEvent",
        "",
        "Called in a handler: the elements the element is in are not told of the event.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Events"],
        title: "Events",
        description: "What an element is told when the pointer presses it, presses it twice, moves over it, comes \
                      in and goes out. A handler is given to the element as it is built, and is called with what \
                      happened: where, which button, which keys were held.",
        import: Some("use lurq::{app::events::MouseEvent, components::Row};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A box whose handlers are chosen beside it. What it is told goes to the top of the Events tab under the \
           stage. Turn on on_mouse_move and move over it: each move is told.",
          playground,
        ),
        Section::new(
          "Click and double click",
          "The first button hears a press and its button going down and coming up; the second a press and a \
           press twice. A double click is two clicks first. Try the right button and a key held.",
          self.frame.demo(
            0,
            self.clicks(),
            ".on_dblclick(|event| ..)",
            &[
              "Row::new()",
              "    .on_click(move |event: MouseEvent| log.push(format!(\"click at ({}, {})\", event.x, event.y)))",
              "    .on_mouse_down(move |event: MouseEvent| log.push(format!(\"mouse_down {:?}\", event.button)))",
              "    .on_mouse_up(move |event: MouseEvent| log.push(format!(\"mouse_up {:?}\", event.button)))",
              "    .child(Text::new(\"Click me\"))",
              "",
              "Row::new()",
              "    .on_click(move |event: MouseEvent| log.push(\"click\"))",
              "    .on_dblclick(move |event: MouseEvent| log.push(\"dblclick\"))",
              "    .child(Text::new(\"Double-click me\"))",
            ],
          ),
        ),
        Section::new(
          "Hover and pressed",
          "An element is painted in another way while the pointer is over it and while it is pressed, with no \
           handler: the first three are drawn in each state, and each is live. The last tells its state with \
           its handlers, as an app that must know of it does.",
          self.frame.demo(
            1,
            self.states(),
            ".hovered(|style| ..).active(|style| ..)",
            &[
              "Row::new()",
              "    .size(140.0, 44.0)",
              "    .background(PaletteColor::Primary)",
              "    .cursor(CursorIcon::Pointer)",
              "    .hovered(|style| style.background(PaletteColor::PrimaryHover))",
              "    .active(|style| style.background(PaletteColor::PrimaryHover).opacity(0.8))",
              "",
              "// What the app is told of it:",
              "    .on_mouse_enter(move || over.set(true))",
              "    .on_mouse_leave(move || over.set(false))",
              "    .on_mouse_down(move |_| pressed.set(true))",
              "    .on_mouse_up(move |_| pressed.set(false))",
            ],
          ),
        ),
        Section::new(
          "Mouse tracking",
          "A box that follows the pointer with on_mouse_move, and knows whether it is in with on_mouse_enter and \
           on_mouse_leave.",
          self.frame.demo(
            2,
            self.tracking(),
            ".on_mouse_move(|event| ..)",
            &[
              "let area = ElementRef::new();",
              "Column::new()",
              "    .height(96.0)",
              "    .cursor(CursorIcon::Crosshair)",
              "    .ref_element(area.clone())",
              "    .on_mouse_move(move |event: MouseEvent| {",
              "        at.set((event.x, event.y));",
              "        in_box.set((event.x - area.x(), event.y - area.y()));",
              "    })",
              "    .on_mouse_enter(move || entered.set(true))",
              "    .on_mouse_leave(move || entered.set(false))",
            ],
          ),
        ),
        Section::new(
          "A box in a box",
          "A press of the inner box is told to it and then to the outer one, which holds it. A handler that \
           calls stop_propagation keeps it from the outer.",
          self.frame.demo(
            3,
            self.nested(),
            "event.stop_propagation()",
            &[
              "Column::new()",
              "    .on_click(move |_| log.push(\"outer: click\"))",
              "    .child(",
              "        Row::new().on_click(move |event: MouseEvent| {",
              "            event.stop_propagation();",
              "            log.push(\"inner: click, stopped\");",
              "        }),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "The handlers of the pointer an element of the engine takes, and what it is painted with while it is \
           under the pointer.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
