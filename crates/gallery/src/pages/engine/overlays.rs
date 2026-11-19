//! The page of the engine's layers over the page: a popup hung from an anchor, a modal laid over a target, and a
//! sheet that comes up from the bottom of the window. Its playground is a popup whose props are the panel's
//! controls; its sections are the demo's modal, with a choice of what it covers, and a sheet.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{
    CollisionStrategy, Column, Modal, ModalTarget, OutsidePress, Parent, Placement, Popup, Rect, Root, Row, Sheet,
    SheetDetent, SheetProps, SheetStyle, Slider, Stack,
  },
  core::{ElementRef, Signal},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension},
  ui::{Button, ButtonSize, ButtonVariant, Switch},
};

use super::kit::{controls, sample, samples, tile, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, line, mono, phone, popover, props_table, quiet, segmented, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The anchors the playground's popup can hang from: the button that opens it, or a tile at the other end.
const ANCHORS: [(&str, usize); 2] = [("the button", 0), ("the tile", 1)];
const PLACEMENTS: [(&str, Placement); 12] = [
  ("BottomStart", Placement::BottomStart),
  ("Bottom", Placement::Bottom),
  ("BottomEnd", Placement::BottomEnd),
  ("TopStart", Placement::TopStart),
  ("Top", Placement::Top),
  ("TopEnd", Placement::TopEnd),
  ("RightStart", Placement::RightStart),
  ("Right", Placement::Right),
  ("RightEnd", Placement::RightEnd),
  ("LeftStart", Placement::LeftStart),
  ("Left", Placement::Left),
  ("LeftEnd", Placement::LeftEnd),
];
/// The choices of the placement's select: an index into [`PLACEMENTS`], which a signal keeps.
const PLACEMENT_CHOICES: [(&str, usize); 12] = [
  ("BottomStart", 0),
  ("Bottom", 1),
  ("BottomEnd", 2),
  ("TopStart", 3),
  ("Top", 4),
  ("TopEnd", 5),
  ("RightStart", 6),
  ("Right", 7),
  ("RightEnd", 8),
  ("LeftStart", 9),
  ("Left", 10),
  ("LeftEnd", 11),
];
const OFFSETS: [(&str, i32); 4] = [("0", 0), ("4", 4), ("8", 8), ("16", 16)];
const COLLISIONS: [(&str, CollisionStrategy); 4] = [
  ("FlipThenClamp", CollisionStrategy::FlipThenClamp),
  ("Flip", CollisionStrategy::Flip),
  ("Clamp", CollisionStrategy::Clamp),
  ("None", CollisionStrategy::None),
];
const COLLISION_CHOICES: [(&str, usize); 4] = [("FlipThenClamp", 0), ("Flip", 1), ("Clamp", 2), ("None", 3)];
const PRESSES: [(&str, OutsidePress); 2] = [
  ("Consume", OutsidePress::Consume),
  ("PassThrough", OutsidePress::PassThrough),
];
const PRESS_CHOICES: [(&str, usize); 2] = [("Consume", 0), ("PassThrough", 1)];

/// What the modal of the section covers: the box it is declared in, the window, or another box.
const TARGETS: [(&str, usize); 3] = [("Parent", 0), ("Root", 1), ("ElementRef", 2)];
/// The heights a sheet stands at, as the section offers them.
const DETENTS: [(&str, usize); 4] = [("Content", 0), ("Medium", 1), ("Large", 2), ("Medium + Large", 3)];

/// The detents of the `index`th choice of [`DETENTS`], and how the code names them.
fn detents(index: usize) -> (Vec<SheetDetent>, &'static str) {
  match index {
    1 => (vec![SheetDetent::Medium], "[SheetDetent::Medium]"),
    2 => (vec![SheetDetent::Large], "[SheetDetent::Large]"),
    3 => (
      vec![SheetDetent::Medium, SheetDetent::Large],
      "[SheetDetent::Medium, SheetDetent::Large]",
    ),
    _ => (vec![SheetDetent::Content], "[SheetDetent::Content]"),
  }
}

/// Whether a placement is above or under its anchor: its offset is then along y, away from the anchor.
fn vertical(placement: Placement) -> bool {
  !matches!(
    placement,
    Placement::LeftStart
      | Placement::Left
      | Placement::LeftEnd
      | Placement::RightStart
      | Placement::Right
      | Placement::RightEnd
  )
}

/// The popup's settings as the panel has them.
#[derive(Clone, Copy)]
struct PopupSetup {
  anchor: usize,
  placement: usize,
  offset: i32,
  collision: usize,
  press: usize,
  outside: bool,
  escape: bool,
  match_width: bool,
  takes_focus: bool,
}

pub struct OverlaysPage {
  frame: Frame,
  playground: Playground,
  /// The two things the playground's popup can hang from.
  button: ElementRef,
  tile: ElementRef,
  open: Signal<bool>,
  anchor: Signal<usize>,
  /// An index into [`PLACEMENTS`].
  placement: Signal<usize>,
  offset: Signal<i32>,
  /// An index into [`COLLISIONS`].
  collision: Signal<usize>,
  /// An index into [`PRESSES`].
  press: Signal<usize>,
  outside: Signal<bool>,
  escape: Signal<bool>,
  match_width: Signal<bool>,
  takes_focus: Signal<bool>,
  /// How many times the button under the anchors was pressed: what a press that passes through reaches.
  presses: Signal<i32>,
  /// The modal of the section: whether it is open, what it covers, the box it covers as an element, and its slider.
  modal_open: Signal<bool>,
  target: Signal<usize>,
  other_box: ElementRef,
  modal_focus: Signal<bool>,
  modal_value: Signal<i32>,
  dismissals: Signal<i32>,
  dismissed_by: Signal<String>,
  /// The sheet of the section.
  sheet_open: Signal<bool>,
  sheet_detents: Signal<usize>,
  sheet_at: Signal<usize>,
  grabber: Signal<bool>,
  dismissable: Signal<bool>,
  sheet_dismissals: Signal<i32>,
}

/// What the popup shows: a few words and a way to close it.
fn popup_content(open: &Signal<bool>) -> Column {
  let close = open.clone();
  popover()
    .padding(12.0)
    .spacing(8.0)
    .child(text("A popup", 14.0, FontWeight::SemiBold))
    .child(quiet("Placed beside its anchor.", 12.0))
    .child(
      Button::new("Close")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| close.set(false)),
    )
}

/// The popup over `anchor`, as the setup makes it.
fn popup(anchor: &ElementRef, open: &Signal<bool>, setup: PopupSetup) -> Popup {
  let placement = PLACEMENTS[setup.placement.min(PLACEMENTS.len() - 1)].1;
  let gap = setup.offset as f32;
  let (x, y) = if vertical(placement) { (0.0, gap) } else { (gap, 0.0) };
  Popup::new(anchor.clone(), popup_content(open))
    .open(open.clone())
    .placement(placement)
    .offset(x, y)
    .collision(COLLISIONS[setup.collision.min(COLLISIONS.len() - 1)].1)
    .outside_press(PRESSES[setup.press.min(PRESSES.len() - 1)].1)
    .dismiss_on_outside_click(setup.outside)
    .dismiss_on_escape(setup.escape)
    .match_anchor_width(setup.match_width)
    .takes_focus(setup.takes_focus)
}

/// The scene of the playground: the button that opens the popup, a tile at the other end it may hang from instead,
/// and a button under them that counts its presses, which a press that closes the popup reaches or not.
fn popup_scene(
  button: &ElementRef,
  tile_ref: &ElementRef,
  open: &Signal<bool>,
  presses: &Signal<i32>,
  shown: Popup,
  count: impl Fn() + Send + Sync + 'static,
) -> Column {
  let (toggle, pressed) = (open.clone(), presses.clone());
  Column::new()
    .width(FILL)
    .height(260.0)
    .padding(16.0)
    .spacing(32.0)
    .justify(Justify::Center)
    .child(
      Row::new()
        .width(FILL)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .child(
          Row::new().ref_element(button.clone()).child(
            Button::new("Open popup")
              .variant(ButtonVariant::Outline)
              .on_click(move |_| toggle.update(|open| *open = !*open)),
          ),
        )
        .child(tile("tile", 1, 88.0, 40.0).ref_element(tile_ref.clone())),
    )
    .child(
      Row::new().width(FILL).justify(Justify::Center).child(
        Button::new(&format!("Pressed {} times", presses.get()))
          .variant(ButtonVariant::Ghost)
          .on_click(move |_| {
            pressed.update(|count| *count += 1);
            count();
          }),
      ),
    )
    .child(shown)
}

impl OverlaysPage {
  fn setup(&self) -> PopupSetup {
    PopupSetup {
      anchor: self.anchor.get(),
      placement: self.placement.get(),
      offset: self.offset.get(),
      collision: self.collision.get(),
      press: self.press.get(),
      outside: self.outside.get(),
      escape: self.escape.get(),
      match_width: self.match_width.get(),
      takes_focus: self.takes_focus.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let setup = self.setup();
    let open = self.open.get();
    let anchor = if setup.anchor == 1 {
      self.tile.clone()
    } else {
      self.button.clone()
    };
    let record = self.playground.recorder();

    let shown = {
      let (button, tile_ref, open, presses) = (
        self.button.clone(),
        self.tile.clone(),
        self.open.clone(),
        self.presses.clone(),
      );
      let (dismissed, counted) = (record.clone(), record.clone());
      let anchor = anchor.clone();
      Shown::built(move |_| {
        let shown = popup(&anchor, &open, setup).on_dismiss(move || dismissed("on_dismiss()".to_owned()));
        popup_scene(&button, &tile_ref, &open, &presses, shown, move || {
          counted("on_click: the button under".to_owned())
        })
        .into()
      })
    };
    let make = move || -> Element {
      let (button, tile_ref, open, presses) =
        (ElementRef::new(), ElementRef::new(), Signal::new(false), Signal::new(0));
      let anchor = if setup.anchor == 1 {
        tile_ref.clone()
      } else {
        button.clone()
      };
      popup_scene(&button, &tile_ref, &open, &presses, popup(&anchor, &open, setup), || {}).into()
    };

    let (placement_name, placement) = PLACEMENTS[setup.placement.min(PLACEMENTS.len() - 1)];
    let collision_name = COLLISIONS[setup.collision.min(COLLISIONS.len() - 1)].0;
    let press_name = PRESSES[setup.press.min(PRESSES.len() - 1)].0;
    let mut code = vec![
      "// open: Signal<bool>, anchor: ElementRef on what it hangs from".to_owned(),
      "Popup::new(anchor.clone(), content)".to_owned(),
      "    .open(open.clone())".to_owned(),
    ];
    if placement != Placement::BottomStart {
      code.push(format!("    .placement(Placement::{placement_name})"));
    }
    if setup.offset != 0 {
      code.push(if vertical(placement) {
        format!("    .offset(0.0, {}.0)", setup.offset)
      } else {
        format!("    .offset({}.0, 0.0)", setup.offset)
      });
    }
    if setup.collision != 0 {
      code.push(format!("    .collision(CollisionStrategy::{collision_name})"));
    }
    if setup.press != 0 {
      code.push(format!("    .outside_press(OutsidePress::{press_name})"));
    }
    for (on, call) in [
      (!setup.outside, "    .dismiss_on_outside_click(false)"),
      (!setup.escape, "    .dismiss_on_escape(false)"),
      (setup.match_width, "    .match_anchor_width(true)"),
      (setup.takes_focus, "    .takes_focus(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push("    .on_dismiss(|| ..)".to_owned());
    code.push("// Popover is the same type: Popover::new(anchor, content)".to_owned());

    let tree = vec![
      format!("scene   {}", self.playground.measured()),
      format!(
        "├─ anchor   {}{}",
        ANCHORS[setup.anchor.min(1)].0,
        if setup.anchor == 0 {
          "   a row with the button in it"
        } else {
          ""
        }
      ),
      "├─ tile".to_owned(),
      "├─ button   counts its presses".to_owned(),
      format!(
        "└─ popup    a layer over the window, {placement_name}, {}",
        if open { "open" } else { "closed" }
      ),
      "   └─ popover column: title, line, Close".to_owned(),
    ];

    let (set_anchor, set_placement, set_offset, set_collision, set_press) = (
      self.anchor.clone(),
      self.placement.clone(),
      self.offset.clone(),
      self.collision.clone(),
      self.press.clone(),
    );
    let reset = {
      let (anchor, placement, offset, collision, press) = (
        self.anchor.clone(),
        self.placement.clone(),
        self.offset.clone(),
        self.collision.clone(),
        self.press.clone(),
      );
      let (outside, escape, match_width, takes_focus, open, presses) = (
        self.outside.clone(),
        self.escape.clone(),
        self.match_width.clone(),
        self.takes_focus.clone(),
        self.open.clone(),
        self.presses.clone(),
      );
      Arc::new(move || {
        anchor.set(0);
        placement.set(0);
        offset.set(4);
        collision.set(0);
        press.set(0);
        outside.set(true);
        escape.set(true);
        match_width.set(false);
        takes_focus.set(false);
        open.set(false);
        presses.set(0);
      })
    };
    let controls = vec![
      prop(
        "anchor",
        "impl Into<ElementRef>",
        self
          .playground
          .select(0, &ANCHORS, &setup.anchor, move |value| set_anchor.set(value)),
      ),
      prop(
        "placement",
        "Placement",
        self
          .playground
          .select(1, &PLACEMENT_CHOICES, &setup.placement, move |value| {
            set_placement.set(value)
          }),
      ),
      prop(
        "offset",
        "(f32, f32)",
        self
          .playground
          .select(2, &OFFSETS, &setup.offset, move |value| set_offset.set(value)),
      ),
      prop(
        "collision",
        "CollisionStrategy",
        self
          .playground
          .select(3, &COLLISION_CHOICES, &setup.collision, move |value| {
            set_collision.set(value)
          }),
      ),
      prop(
        "outside_press",
        "OutsidePress",
        self
          .playground
          .select(4, &PRESS_CHOICES, &setup.press, move |value| set_press.set(value)),
      ),
      divider(),
      flag("open", &self.open),
      flag("dismiss_on_outside_click", &self.outside),
      flag("dismiss_on_escape", &self.escape),
      flag("match_anchor_width", &self.match_width),
      flag("takes_focus", &self.takes_focus),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Popup",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{} · {placement_name}", if open { "open" } else { "closed" }),
        event: "open / on_dismiss / on_click",
        code,
        tree,
        controls,
        changed: [
          setup.anchor != 0,
          setup.placement != 0,
          setup.offset != 4,
          setup.collision != 0,
          setup.press != 0,
          !setup.outside,
          !setup.escape,
          setup.match_width,
          setup.takes_focus,
          open,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The demo's modal: a veil that closes it when pressed, and a panel with a slider and a Close button over the
  /// target the segmented control chooses.
  fn modal(&self) -> Column {
    let target_at = self.target.get().min(TARGETS.len() - 1);
    let open = self.modal_open.clone();
    let dismiss = {
      let (open, count, by) = (open.clone(), self.dismissals.clone(), self.dismissed_by.clone());
      Arc::new(move |how: &str| {
        open.set(false);
        count.update(|count| *count += 1);
        by.set(how.to_owned());
      })
    };
    let veil_dismiss = dismiss.clone();
    let close_dismiss = dismiss.clone();
    let escape_dismiss = {
      let (count, by) = (self.dismissals.clone(), self.dismissed_by.clone());
      move || {
        count.update(|count| *count += 1);
        by.set("Escape".to_owned());
      }
    };
    let value = self.modal_value.get();
    let content = Stack::new()
      .size(FILL, FILL)
      .stack_align(StackAlignment::Center)
      .child(
        Rect::new(FILL, FILL)
          .background(PaletteColor::Scrim)
          .on_click(move |_| veil_dismiss("the veil")),
      )
      .child(
        Column::new()
          .width(340.0)
          .max_width(Dimension::Pct(92.0))
          .padding(20.0)
          .spacing(12.0)
          .rounded(RadiusSize::Lg)
          .background(PaletteColor::Card)
          .foreground(PaletteColor::CardForeground)
          .border(line())
          .child(text("A modal", 18.0, FontWeight::SemiBold))
          .child(
            Column::new()
              .width(FILL)
              .child(quiet("Declared in the render flow, laid over its target.", 12.0).width(FILL)),
          )
          .child(
            Column::new()
              .width(FILL)
              .spacing(8.0)
              .child(
                Row::new()
                  .width(FILL)
                  .justify(Justify::SpaceBetween)
                  .child(text("Modal slider", 13.0, FontWeight::Medium))
                  .child(mono(&value.to_string(), 12.0)),
              )
              .child(
                Slider::new(self.modal_value.clone())
                  .range(0, 100)
                  .height(18.0)
                  .width(FILL),
              ),
          )
          .child(
            Row::new()
              .width(FILL)
              .justify(Justify::End)
              .child(Button::new("Close").on_click(move |_| close_dismiss("Close"))),
          ),
      );
    let target: ModalTarget = match target_at {
      1 => Root.into(),
      2 => self.other_box.clone().into(),
      _ => Parent.into(),
    };
    let modal = Modal::new(content)
      .open(open.clone())
      .target(target)
      .takes_focus(self.modal_focus.get())
      .on_dismiss(escape_dismiss);

    let set_target = self.target.clone();
    let focus = self.modal_focus.clone();
    let opener = open.clone();
    let count = self.dismissals.get();
    let said = if count == 0 {
      "Not dismissed yet".to_owned()
    } else {
      format!("Dismissed {count} times, the last by {}", self.dismissed_by.get())
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(segmented(&TARGETS, &target_at, move |value| set_target.set(value)))
          .child(
            Switch::new("takes_focus")
              .on(self.modal_focus.get())
              .on_change(move |on| focus.set(on)),
          ),
      )
      .child(
        samples()
          .child(sample(
            well()
              .width(280.0)
              .max_width(FILL)
              .height(240.0)
              .align_items(Alignment::Center)
              .justify(Justify::Center)
              .child(Button::new("Open modal").on_click(move |_| opener.set(true)))
              .child(modal),
            "the box it is declared in: Parent",
          ))
          .child(sample(
            well()
              .width(280.0)
              .max_width(FILL)
              .height(240.0)
              .align_items(Alignment::Center)
              .justify(Justify::Center)
              .ref_element(self.other_box.clone())
              .child(quiet("another box", 12.0)),
            "a box of its own: ElementRef",
          )),
      )
      .child(quiet(&said, 12.0))
  }

  /// A sheet that comes up from the bottom of the window, at the detents chosen.
  fn sheet(&self, ctx: &mut Ctx) -> Column {
    let choice = self.sheet_detents.get().min(DETENTS.len() - 1);
    let (heights, _) = detents(choice);
    let many = heights.len() > 1;
    let at = self.sheet_at.get();
    let open = self.sheet_open.clone();
    let close = open.clone();
    let mut content = Column::new()
      .width(FILL)
      .padding(24.0)
      .spacing(12.0)
      .child(text("A sheet", 18.0, FontWeight::SemiBold))
      .child(
        Column::new().width(FILL).child(
          quiet(
            "It comes up from the bottom of the window, over everything in it.",
            12.0,
          )
          .width(FILL),
        ),
      )
      .child(mono(&format!("detent {at}"), 12.0));
    if many {
      let (to_medium, to_large) = (self.sheet_at.clone(), self.sheet_at.clone());
      content = content.child(
        Row::new()
          .spacing(8.0)
          .child(
            Button::new("Medium")
              .variant(ButtonVariant::Outline)
              .on_click(move |_| to_medium.set(0)),
          )
          .child(
            Button::new("Large")
              .variant(ButtonVariant::Outline)
              .on_click(move |_| to_large.set(1)),
          ),
      );
    }
    content = content.child(Button::new("Close").on_click(move |_| close.set(false)));
    let dismissed = self.sheet_dismissals.clone();
    let sheet = Sheet::mount(
      ctx,
      SheetProps::new(open.clone())
        .detents(heights)
        .detent(self.sheet_at.clone())
        .dismissable(self.dismissable.get())
        .style(SheetStyle {
          max_width: 520.0,
          grabber: self.grabber.get(),
          ..SheetStyle::new()
        })
        .on_dismiss(move || dismissed.update(|count| *count += 1)),
      content,
    );

    let (set_detents, at_signal) = (self.sheet_detents.clone(), self.sheet_at.clone());
    let (grabber, dismissable) = (self.grabber.clone(), self.dismissable.clone());
    let opener = open.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          // Four choices are wider than a phone's card: they scroll sideways in it there.
          .child(fit(
            "sheet-detents",
            self.frame.stage_room(),
            350.0,
            segmented(&DETENTS, &choice, move |value| {
              // The detent is a place in the list: the first of a new list.
              at_signal.set(0);
              set_detents.set(value);
            }),
          ))
          .child(
            Switch::new("grabber")
              .on(self.grabber.get())
              .on_change(move |on| grabber.set(on)),
          )
          .child(
            Switch::new("dismissable")
              .on(self.dismissable.get())
              .on_change(move |on| dismissable.set(on)),
          ),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(16.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(Button::new("Open sheet").on_click(move |_| opener.set(true)))
          .child({
            let status = quiet(
              &format!(
                "detent {at} · put away by a person {} times",
                self.sheet_dismissals.get()
              ),
              12.0,
            );
            // As narrow as a phone it is on a line of its own, and goes on to the next where it is longer.
            if phone() { status.width(FILL) } else { status }
          }),
      )
      .child(sheet)
  }
}

impl Component for OverlaysPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 5);
    let open = ctx.signal(false);
    playground.watch(ctx, &open, |open| format!("open = {open}"));
    Self {
      frame: Frame::new(ctx),
      playground,
      button: ElementRef::new(),
      tile: ElementRef::new(),
      open,
      anchor: ctx.signal(0),
      placement: ctx.signal(0),
      offset: ctx.signal(4),
      collision: ctx.signal(0),
      press: ctx.signal(0),
      outside: ctx.signal(true),
      escape: ctx.signal(true),
      match_width: ctx.signal(false),
      takes_focus: ctx.signal(false),
      presses: ctx.signal(0),
      modal_open: ctx.signal(false),
      target: ctx.signal(0),
      other_box: ElementRef::new(),
      modal_focus: ctx.signal(true),
      modal_value: ctx.signal(64),
      dismissals: ctx.signal(0),
      dismissed_by: ctx.signal(String::new()),
      sheet_open: ctx.signal(false),
      sheet_detents: ctx.signal(0),
      sheet_at: ctx.signal(0),
      grabber: ctx.signal(true),
      dismissable: ctx.signal(true),
      sheet_dismissals: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "Popup::new",
        "(impl Into<ElementRef>, impl Into<Element>)",
        "",
        "A layer of the content beside the anchor, with no look of its own. Popover is the same type.",
      ),
      (
        "anchor",
        "impl Into<ElementRef>",
        "",
        "What it hangs from, in place of the one it was made with.",
      ),
      (
        "open",
        "impl Into<OpenState>",
        "open",
        "A Signal<bool> it is open while, which Escape and a press outside set to false; a bool is a state that \
         nothing closes.",
      ),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Which side of the anchor it stands on, and where along it. It goes to the other side where the window \
         has no room.",
      ),
      (
        "offset",
        "(f32, f32)",
        "0, 0",
        "How far from the anchor: along y above and under it, along x beside it.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where the window has no room for it: turn to the other side, keep inside the window, both, or \
         nothing.",
      ),
      ("match_anchor_width", "bool", "false", "As wide as its anchor."),
      (
        "outside_press",
        "OutsidePress",
        "Consume",
        "Whether the press that closes it also reaches what is under the pointer.",
      ),
      (
        "dismiss_on_outside_click",
        "bool",
        "true",
        "A left press outside it closes it.",
      ),
      ("dismiss_on_escape", "bool", "true", "Escape closes it."),
      (
        "takes_focus",
        "bool",
        "false",
        "The focus goes into it as it opens and back as it closes.",
      ),
      (
        "traps_tab",
        "bool",
        "false",
        "Tab goes round in it while the focus is in it, as in a modal.",
      ),
      (
        "on_dismiss",
        "Fn()",
        "",
        "Called after a press outside or Escape closed it; not when the app sets its signal.",
      ),
      (
        "at",
        "Option<(f32, f32)>",
        "None",
        "A point of the window it stands at in place of its anchor: a context menu's.",
      ),
      (
        "Modal::new",
        "impl Into<Element>",
        "",
        "A layer over its target, declared where it is in the tree and taking no room there.",
      ),
      (
        "target",
        "impl Into<ModalTarget>",
        "Parent",
        "What it covers: the box it is declared in, the window (Root), or an element by its ref.",
      ),
      (
        "Modal::takes_focus",
        "bool",
        "false",
        "The focus goes to its first stop of Tab as it opens, and back as it closes. Tab stays in it.",
      ),
      (
        "Modal::on_dismiss",
        "Fn()",
        "",
        "Called after Escape closed it; the app's own veil and close button call it too.",
      ),
      (
        "SheetProps::new",
        "Signal<bool>",
        "",
        "A sheet that is up while the signal is set, mounted with Sheet::mount(ctx, props, content).",
      ),
      (
        "detents",
        "impl IntoIterator<Item = SheetDetent>",
        "[Content]",
        "The heights it stands at: Content, Medium, Large, a fraction or a height. It comes up at the first.",
      ),
      (
        "detent",
        "Signal<usize>",
        "",
        "Which of the detents it is at, by its place in the list: it goes where the signal says.",
      ),
      (
        "style",
        "SheetStyle",
        "the theme's",
        "Its colors, corners, grabber, widest width and motion.",
      ),
      (
        "dismissable",
        "bool",
        "true",
        "A press beside it, a drag down and Escape put it away.",
      ),
      (
        "SheetProps::on_dismiss",
        "Fn()",
        "",
        "Called after a person put it away; not when the app clears the signal.",
      ),
    ];
    let playground = self.play(ctx);
    let modal = self.modal();
    let sheet = self.sheet(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Overlays"],
        title: "Overlays",
        description: "Popup, Modal and Sheet: the engine's layers over the page. A popup hangs beside an anchor, a \
                      modal covers a target, and a sheet comes up from the bottom of the window. They have no look \
                      of their own; the design system's popovers, dialogs and sheets are built on them.",
        import: Some("use lurq::components::{Modal, Popover, Popup, Sheet, SheetProps};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A popup hung from the button that opens it, or from the tile at the other end. Choose its side, its \
           offset and what it does where the window has no room; with the outside press passed through, the press \
           that closes it also counts on the button under it. Popover is the same type.",
          playground,
        ),
        Section::new(
          "Modal",
          "A modal declared in the render flow: a veil that closes it when pressed and a panel with a slider over \
           what its target says. Parent covers the box it is declared in, Root the window, an ElementRef another \
           box. Escape, the veil and Close each put it away.",
          self.frame.demo(
            0,
            modal,
            "Modal::new(content).open(open.clone()).target(Parent)",
            &[
              "// open: Signal<bool>",
              "Modal::new(",
              "    Stack::new()",
              "        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))",
              "        .stack_align(StackAlignment::Center)",
              "        // The veil: a press on it closes the modal.",
              "        .child(Rect::new(FILL, FILL).background(PaletteColor::Scrim).on_click(..))",
              "        .child(panel), // a title, a line, a Slider, Close",
              ")",
              ".open(open.clone())",
              ".target(Parent) // Root, or an ElementRef",
              ".takes_focus(true)",
              ".on_dismiss(|| ..)",
            ],
          ),
        ),
        Section::new(
          "Sheet",
          "A sheet comes up from the bottom of the window with a move, and the window behind dims. It stands at its \
           detents; with two of them the buttons in it move it between them. A press beside it and Escape put it \
           away where it is dismissable, and Close always does.",
          self.frame.demo(
            1,
            sheet,
            "Sheet::mount(ctx, SheetProps::new(open.clone()), content)",
            &[
              "// open: Signal<bool>, detent: Signal<usize>",
              "Sheet::mount(",
              "    ctx,",
              &format!(
                "    SheetProps::new(open.clone()).detents({}).detent(detent.clone())",
                detents(self.sheet_detents.get().min(DETENTS.len() - 1)).1
              ),
              "        .style(SheetStyle { max_width: 520.0, grabber: true, ..SheetStyle::new() })",
              "        .on_dismiss(|| ..),",
              "    content,",
              ")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a popup, a modal and a sheet are given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
