//! The page of the engine's drag and drop: `Draggable`, which moves what it holds under the pointer, `DropZone`,
//! which hears what is let go over it, and `DragContainer`, which keeps what is dragged in its bounds. Its
//! playground is a card in a container whose props are the panel's controls; its sections are the demo's Drag &
//! Drop page (two cards, one that stays where it is let go and one that goes back, and two zones that say what
//! was dropped), a drop that reads what was carried, and cards that the app's state places.
//!
//! This is not the gallery's Drop Zone page, which is the design system's zone for files from the system.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, DropEvent, DropResult, MouseButtonMask},
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{
    Column, DragBounds, DragContainer, DragContainerProps, DragMovement, DragOverridePolicy, Draggable, DraggableProps,
    DropMissBehavior, DropZone, DropZoneProps, Row, Stack,
  },
  core::{ElementRect, ElementRefMut, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{Button, ButtonSize, ButtonVariant, icons},
};

use super::kit::{controls, tint};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const BOUNDS: [(&str, DragBounds); 2] = [("SelfBounds", DragBounds::SelfBounds), ("None", DragBounds::None)];
const MOVEMENTS: [(&str, DragMovement); 2] = [("Automatic", DragMovement::Automatic), ("Manual", DragMovement::Manual)];
const MISSES: [(&str, DropMissBehavior); 2] = [
  ("KeepPosition", DropMissBehavior::KeepPosition),
  ("RevertToDragStart", DropMissBehavior::RevertToDragStart),
];
const POLICIES: [(&str, DragOverridePolicy); 2] =
  [("Keep", DragOverridePolicy::Keep), ("Clear", DragOverridePolicy::Clear)];
const BUTTONS: [(&str, MouseButtonMask); 3] = [
  ("ANY", MouseButtonMask::ANY),
  ("LEFT", MouseButtonMask::LEFT),
  ("RIGHT", MouseButtonMask::RIGHT),
];
/// The choices of the playground's selects: an index into the arrays above, which a signal keeps.
const BOUNDS_CHOICES: [(&str, usize); 2] = [("SelfBounds", 0), ("None", 1)];
const MOVEMENT_CHOICES: [(&str, usize); 2] = [("Automatic", 0), ("Manual: a grid of 20", 1)];
const MISS_CHOICES: [(&str, usize); 2] = [("KeepPosition", 0), ("RevertToDragStart", 1)];
const POLICY_CHOICES: [(&str, usize); 2] = [("Keep", 0), ("Clear", 1)];
const BUTTON_CHOICES: [(&str, usize); 3] = [("ANY", 0), ("LEFT", 1), ("RIGHT", 2)];

/// The playground's container, and where the card and the zone stand in it.
const PLAY_WIDTH: f32 = 400.0;
const PLAY_HEIGHT: f32 = 260.0;
const CARD_AT: (f32, f32) = (24.0, 64.0);
const CARD_SIZE: (f32, f32) = (120.0, 80.0);
const ZONE_AT: (f32, f32) = (236.0, 48.0);
const ZONE_SIZE: (f32, f32) = (140.0, 120.0);
/// The step a card moved by hand snaps to.
const GRID: f32 = 20.0;

/// The demo's container: its size, and its two cards and two zones.
const DEMO_WIDTH: f32 = 400.0;
const DEMO_HEIGHT: f32 = 280.0;

/// The cards of the section whose places are the app's state: their names, which a drop carries.
const TASKS: [&str; 3] = ["Draft", "Review", "Ship"];
/// The two columns they stand in, and how wide each is.
const COLUMNS: [&str; 2] = ["To do", "Done"];
const COLUMN_WIDTH: f32 = 180.0;
const COLUMN_GAP: f32 = 16.0;
const TASK_HEIGHT: f32 = 36.0;

/// A card to drag, in the `index`th tint, with `label` in its middle.
fn card(label: &str, index: usize, width: f32, height: f32) -> Row {
  Row::new()
    .size(width, height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .background(tint(index))
    .child(
      text(label, 13.0, FontWeight::SemiBold)
        .color(PaletteColor::Background)
        .nowrap(),
    )
}

/// A zone a card is let go over: a dashed box with `label` in its middle, its edge in the primary color while
/// something is being dragged.
fn zone(label: &str, width: f32, height: f32, dragging: bool) -> Row {
  let edge = if dragging {
    Border::inside(BorderSize::Md, PaletteColor::Primary).dashed()
  } else {
    Border::inside(BorderSize::Sm, PaletteColor::Border).dashed()
  };
  Row::new()
    .size(width, height)
    .padding(8.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(edge)
    .child(mono(label, 12.0).color(PaletteColor::Foreground))
}

/// The box a scene is laid out in: what is dragged moves inside it, and its bounds are what a container keeps a
/// card in.
fn ground(width: f32, height: f32) -> Stack {
  Stack::new()
    .size(width, height)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
}

/// What a drag's end says of the drop.
fn drop_result(event: &DragEvent) -> &'static str {
  match event.drop_result {
    Some(DropResult::Accepted { .. }) => "Accepted",
    Some(DropResult::Missed) => "Missed",
    None => "None",
  }
}

/// A handler's record of what happened: the playground's recorder, shared by the handlers of the scene.
type Record = Arc<dyn Fn(String) + Send + Sync>;

/// What the playground's props are, as indices into their arrays.
#[derive(Clone, Copy)]
struct Setup {
  bounds: usize,
  movement: usize,
  miss: usize,
  policy: usize,
  buttons: usize,
  follower: bool,
}

/// The refs of the playground's card and of what follows it, and what says the card was dropped into the zone.
#[derive(Clone)]
struct Played {
  card: ElementRefMut,
  follower: ElementRefMut,
  /// The card was dropped into the zone while the drag's overrides were cleared at its end: the app moved it there.
  docked: Signal<bool>,
  dragging: Signal<bool>,
}

/// The playground's scene: a card in a container, a zone beside it, and a tile that follows the card where it is
/// asked for.
fn play_scene(ctx: &mut Ctx, setup: Setup, played: &Played, record: Record) -> Element {
  let clear = POLICIES[setup.policy].1 == DragOverridePolicy::Clear;
  // Where a drag started, for a card moved by hand: it is placed at the step of the grid nearest to the
  // pointer's way from there.
  let start = Arc::new(Mutex::new(ElementRect::default()));
  let mut props = DraggableProps::new()
    .element_ref(played.card.clone())
    .payload("card")
    .start_drag_buttons(BUTTONS[setup.buttons].1)
    .drop_miss_behavior(MISSES[setup.miss].1)
    .override_policy(POLICIES[setup.policy].1)
    .movement(MOVEMENTS[setup.movement].1)
    .on_drag_start({
      let (record, card, start, dragging) = (
        record.clone(),
        played.card.clone(),
        start.clone(),
        played.dragging.clone(),
      );
      move |_| {
        *start.lock().unwrap() = card.bounds();
        dragging.set(true);
        record("on_drag_start".to_owned());
      }
    })
    .on_drag_end({
      let (record, dragging) = (record.clone(), played.dragging.clone());
      move |event| {
        dragging.set(false);
        record(format!("on_drag_end({})", drop_result(event)));
      }
    });
  if MOVEMENTS[setup.movement].1 == DragMovement::Manual {
    let (card, follower, start) = (played.card.clone(), played.follower.clone(), start.clone());
    props = props.on_drag_move(move |event: &DragEvent| {
      let from = *start.lock().unwrap();
      let snap = |value: f32| (value / GRID).round() * GRID;
      let now = card.bounds();
      let (x, y) = (
        snap(from.relative_x + event.total_delta_x),
        snap(from.relative_y + event.total_delta_y),
      );
      let (dx, dy) = (x - now.relative_x, y - now.relative_y);
      card.set_relative_bounds(x, y, now.width, now.height);
      // By hand, what follows is moved by the app too.
      if follower.is_attached() {
        let at = follower.bounds();
        follower.set_relative_bounds(at.relative_x + dx, at.relative_y + dy, at.width, at.height);
      }
    });
  }
  if setup.follower {
    props = props.follower(played.follower.clone());
  }
  let docked = played.docked.get();
  let (x, y) = if docked {
    (ZONE_AT.0 + 10.0, ZONE_AT.1 + 20.0)
  } else {
    CARD_AT
  };
  let dragged = Draggable::mount(
    ctx,
    props,
    card("card", 0, CARD_SIZE.0, CARD_SIZE.1).absolute_position(x, y),
  );
  let zone = DropZone::mount(
    ctx,
    DropZoneProps::new().on_drop({
      let docked = played.docked.clone();
      move |event: &DropEvent| {
        let carried = event.payload::<&str>().copied().unwrap_or("nothing");
        record(format!("on_drop(payload {carried:?})"));
        // With the overrides cleared at the drag's end, the card stands where the app's state says: the drop
        // puts it in the zone.
        if clear {
          docked.set(true);
        }
      }
    }),
    zone("DropZone", ZONE_SIZE.0, ZONE_SIZE.1, played.dragging.get()).absolute_position(ZONE_AT.0, ZONE_AT.1),
  );
  let mut children: Vec<Element> = vec![
    quiet(&format!("DragContainer {PLAY_WIDTH} × {PLAY_HEIGHT}"), 11.0)
      .absolute_position(8.0, 8.0)
      .into(),
    zone,
    dragged,
  ];
  if setup.follower {
    children.push(
      card("follower", 1, CARD_SIZE.0, 28.0)
        .ref_element(played.follower.clone())
        .absolute_position(x, y + CARD_SIZE.1 + 8.0)
        .into(),
    );
  }
  DragContainer::mount(
    ctx,
    DragContainerProps::new().bounds(BOUNDS[setup.bounds].1),
    // Without bounds the card can be taken past the container's edge, and is drawn there.
    ground(PLAY_WIDTH, PLAY_HEIGHT)
      .overflow_visible()
      .with_children(children),
  )
}

/// The demo's container: a card that stays where it is let go and one that goes back where it started when it
/// misses, and two zones that say what happened last.
fn demo_scene(ctx: &mut Ctx, status: &Signal<String>, keep: &ElementRefMut, revert: &ElementRefMut) -> Element {
  let said = status.get();
  let said_by = |what: &'static str| {
    let status = status.clone();
    move |_: &DropEvent| status.set(what.to_owned())
  };
  let started_by = |what: &'static str| {
    let status = status.clone();
    move |_: &DragEvent| status.set(what.to_owned())
  };
  let top = DropZone::mount(
    ctx,
    DropZoneProps::new().on_drop(said_by("Dropped top")),
    zone(&said, 140.0, 82.0, false).absolute_position(220.0, 60.0),
  );
  let bottom = DropZone::mount(
    ctx,
    DropZoneProps::new().on_drop(said_by("Dropped bottom")),
    zone(&said, 140.0, 82.0, false).absolute_position(220.0, 160.0),
  );
  let kept = Draggable::mount(
    ctx,
    DraggableProps::new()
      .element_ref(keep.clone())
      .on_drag_start(started_by("Dragging keep")),
    card("keep", 0, 120.0, 80.0).absolute_position(20.0, 40.0),
  );
  let reverted = Draggable::mount(
    ctx,
    DraggableProps::new()
      .element_ref(revert.clone())
      .drop_miss_behavior(DropMissBehavior::RevertToDragStart)
      .on_drag_start(started_by("Dragging revert")),
    card("revert", 3, 120.0, 80.0).absolute_position(20.0, 140.0),
  );
  DragContainer::mount(
    ctx,
    DragContainerProps::new(),
    ground(DEMO_WIDTH, DEMO_HEIGHT).with_children([
      quiet(&format!("DnD ({DEMO_WIDTH}x{DEMO_HEIGHT})"), 11.0)
        .absolute_position(8.0, 8.0)
        .into(),
      top,
      bottom,
      kept,
      reverted,
    ]),
  )
}

/// Where the `slot`th card of a column stands in the board.
fn task_at(column: usize, slot: usize) -> (f32, f32) {
  (
    column as f32 * (COLUMN_WIDTH + COLUMN_GAP) + 12.0,
    40.0 + slot as f32 * (TASK_HEIGHT + 8.0),
  )
}

/// Two columns of cards, each card where the app's state says: a drop on a column reads the card it carries and
/// moves it there; the drag's overrides are cleared at its end, so a card let go anywhere else goes back.
fn board_scene(ctx: &mut Ctx, places: &Signal<Vec<usize>>, moves: &Signal<i32>) -> Element {
  let now = places.get();
  let height = 40.0 + TASKS.len() as f32 * (TASK_HEIGHT + 8.0) + 8.0;
  let width = COLUMNS.len() as f32 * (COLUMN_WIDTH + COLUMN_GAP) - COLUMN_GAP;
  let mut children: Vec<Element> = Vec::new();
  for (column, name) in COLUMNS.iter().enumerate() {
    let (places, moves) = (places.clone(), moves.clone());
    children.push(DropZone::mount(
      ctx,
      DropZoneProps::new().on_drop(move |event: &DropEvent| {
        let Some(task) = event.payload::<usize>().copied() else {
          return;
        };
        if places.get_untracked().get(task) != Some(&column) {
          places.update(|places| places[task] = column);
          moves.update(|moves| *moves += 1);
        }
      }),
      Column::new()
        .size(COLUMN_WIDTH, height)
        .padding(10.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Background)
        .border(Border::inside(BorderSize::Sm, PaletteColor::Border).dashed())
        .child(mono(name, 12.0).color(PaletteColor::MutedForeground))
        .absolute_position(column as f32 * (COLUMN_WIDTH + COLUMN_GAP), 0.0),
    ));
  }
  for (task, name) in TASKS.iter().enumerate() {
    let column = now.get(task).copied().unwrap_or(0);
    let slot = (0..task)
      .filter(|earlier| now.get(*earlier).copied().unwrap_or(0) == column)
      .count();
    let (x, y) = task_at(column, slot);
    children.push(Draggable::mount(
      ctx,
      DraggableProps::new()
        .payload(task)
        .override_policy(DragOverridePolicy::Clear),
      card(name, task + 1, COLUMN_WIDTH - 24.0, TASK_HEIGHT).absolute_position(x, y),
    ));
  }
  DragContainer::mount(
    ctx,
    DragContainerProps::new(),
    Stack::new().size(width, height).with_children(children),
  )
}

pub struct DragAndDropPage {
  frame: Frame,
  playground: Playground,
  /// Indices into [`BOUNDS`], [`MOVEMENTS`], [`MISSES`], [`POLICIES`] and [`BUTTONS`].
  bounds: Signal<usize>,
  movement: Signal<usize>,
  miss: Signal<usize>,
  policy: Signal<usize>,
  buttons: Signal<usize>,
  follower: Signal<bool>,
  played: Played,
  /// What the demo's zones say: "Drop target", "Dragging keep", "Dropped top".
  status: Signal<String>,
  keep: ElementRefMut,
  revert: ElementRefMut,
  /// The column each card of the board is in, by the card's place in [`TASKS`], and how many times one moved.
  places: Signal<Vec<usize>>,
  moves: Signal<i32>,
}

impl DragAndDropPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let setup = Setup {
      bounds: self.bounds.get().min(BOUNDS.len() - 1),
      movement: self.movement.get().min(MOVEMENTS.len() - 1),
      miss: self.miss.get().min(MISSES.len() - 1),
      policy: self.policy.get().min(POLICIES.len() - 1),
      buttons: self.buttons.get().min(BUTTONS.len() - 1),
      follower: self.follower.get(),
    };
    let record: Record = Arc::new(self.playground.recorder());
    let played = self.played.clone();
    // The card and what follows it go back where they were laid out, and out of the zone.
    let put_back = {
      let played = played.clone();
      move || {
        played.card.clear_bounds_override();
        played.follower.clear_bounds_override();
        played.docked.set(false);
      }
    };
    let shown = {
      let (played, record, put_back) = (played.clone(), record.clone(), put_back.clone());
      Shown::built(move |ctx| {
        Column::new()
          .spacing(12.0)
          .child(
            Button::new("Put the card back")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon(icons::REFRESH)
              .on_click(move |_| put_back()),
          )
          .child(play_scene(ctx, setup, &played, record))
          .into()
      })
    };
    // A window of its own has a card of its own, with refs of its own: a ref is on one element. Whether the card
    // is in the zone is the page's, and the window shows it as the stage does.
    self.playground.window_shows({
      let (record, docked, dragging) = (record.clone(), played.docked.clone(), played.dragging.clone());
      move |ctx| {
        let played = Played {
          card: ctx.element_ref_mut(),
          follower: ctx.element_ref_mut(),
          docked: docked.clone(),
          dragging: dragging.clone(),
        };
        play_scene(ctx, setup, &played, record.clone())
      }
    });
    let make = || -> Element { quiet("The scene is mounted: it needs the window's context.", 12.0).into() };

    let mut code = vec![
      "Draggable::mount(".to_owned(),
      "    ctx,".to_owned(),
      "    DraggableProps::new()".to_owned(),
    ];
    if setup.buttons != 0 {
      code.push(format!(
        "        .start_drag_buttons(MouseButtonMask::{})",
        BUTTONS[setup.buttons].0
      ));
    }
    if setup.miss != 0 {
      code.push(format!(
        "        .drop_miss_behavior(DropMissBehavior::{})",
        MISSES[setup.miss].0
      ));
    }
    if setup.policy != 0 {
      code.push(format!(
        "        .override_policy(DragOverridePolicy::{})",
        POLICIES[setup.policy].0
      ));
    }
    if setup.movement != 0 {
      code.push("        .movement(DragMovement::Manual)".to_owned());
      code.push("        .on_drag_move(move |event| card_ref.set_relative_bounds(snapped(event), ..))".to_owned());
    }
    if setup.follower {
      code.push("        .follower(follower_ref.clone())".to_owned());
    }
    code.push("        .payload(\"card\")".to_owned());
    code.push("        .on_drag_end(|event| log(event.drop_result)),".to_owned());
    code.push("    Row::new().size(120.0, 80.0).absolute_position(24.0, 64.0),".to_owned());
    code.push(")".to_owned());
    code.push(String::new());
    code.push(
      "DropZone::mount(ctx, DropZoneProps::new().on_drop(|event| log(event.payload::<&str>())), zone)".to_owned(),
    );
    code.push(String::new());
    if setup.bounds == 0 {
      code.push("DragContainer::mount(ctx, DragContainerProps::new(), Stack::new().size(400.0, 260.0)..)".to_owned());
    } else {
      code.push(
        "DragContainer::mount(ctx, DragContainerProps::new().bounds(DragBounds::None), Stack::new()..)".to_owned(),
      );
    }

    let mut tree = vec![
      format!("drag container   {}", self.playground.measured()),
      format!("└─ stack   {PLAY_WIDTH} × {PLAY_HEIGHT}"),
      format!("   ├─ drop zone   {} × {}", ZONE_SIZE.0, ZONE_SIZE.1),
    ];
    if setup.follower {
      tree.push(format!("   ├─ draggable   {} × {}", CARD_SIZE.0, CARD_SIZE.1));
      tree.push(format!("   └─ follower   {} × 28", CARD_SIZE.0));
    } else {
      tree.push(format!("   └─ draggable   {} × {}", CARD_SIZE.0, CARD_SIZE.1));
    }

    let (set_bounds, set_movement, set_miss, set_policy, set_buttons) = (
      self.bounds.clone(),
      self.movement.clone(),
      self.miss.clone(),
      self.policy.clone(),
      self.buttons.clone(),
    );
    let reset = {
      let (bounds, movement, miss, policy, buttons, follower) = (
        self.bounds.clone(),
        self.movement.clone(),
        self.miss.clone(),
        self.policy.clone(),
        self.buttons.clone(),
        self.follower.clone(),
      );
      Arc::new(move || {
        bounds.set(0);
        movement.set(0);
        miss.set(0);
        policy.set(0);
        buttons.set(0);
        follower.set(false);
        put_back();
      })
    };
    let controls = vec![
      prop(
        "bounds",
        "DragBounds, of the container",
        self
          .playground
          .select(0, &BOUNDS_CHOICES, &setup.bounds, move |value| set_bounds.set(value)),
      ),
      prop(
        "movement",
        "DragMovement",
        self
          .playground
          .select(1, &MOVEMENT_CHOICES, &setup.movement, move |value| {
            set_movement.set(value)
          }),
      ),
      prop(
        "drop_miss_behavior",
        "DropMissBehavior",
        self
          .playground
          .select(2, &MISS_CHOICES, &setup.miss, move |value| set_miss.set(value)),
      ),
      prop(
        "override_policy",
        "DragOverridePolicy",
        self
          .playground
          .select(3, &POLICY_CHOICES, &setup.policy, move |value| set_policy.set(value)),
      ),
      prop(
        "start_drag_buttons",
        "MouseButtonMask",
        self
          .playground
          .select(4, &BUTTON_CHOICES, &setup.buttons, move |value| set_buttons.set(value)),
      ),
      divider(),
      flag("follower", &self.follower),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Draggable",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: if played.docked.get() {
          "the card is in the zone".to_owned()
        } else {
          "the card is where it was laid out".to_owned()
        },
        event: "on_drag_start · on_drag_end · on_drop",
        code,
        tree,
        controls,
        changed: [
          setup.bounds != 0,
          setup.movement != 0,
          setup.miss != 0,
          setup.policy != 0,
          setup.buttons != 0,
          setup.follower,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The demo's page: its container with the two cards and the two zones, and what puts the cards back.
  fn container_drop(&self, ctx: &mut Ctx) -> Element {
    let (status, keep, revert) = (self.status.clone(), self.keep.clone(), self.revert.clone());
    let scene = demo_scene(ctx, &self.status, &self.keep, &self.revert);
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Button::new("Put the cards back")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon(icons::REFRESH)
              .on_click(move |_| {
                keep.clear_bounds_override();
                revert.clear_bounds_override();
                status.set("Drop target".to_owned());
              }),
          )
          .child(quiet(&format!("status: {}", self.status.get()), 12.0)),
      )
      .child(fit("drag-demo", self.frame.stage_room(), DEMO_WIDTH, scene))
      .into()
  }

  /// Cards whose places are the app's: a drop on a column moves the card it carries there.
  fn board(&self, ctx: &mut Ctx) -> Element {
    let width = COLUMNS.len() as f32 * (COLUMN_WIDTH + COLUMN_GAP) - COLUMN_GAP;
    let scene = board_scene(ctx, &self.places, &self.moves);
    let places = self.places.get();
    let done: Vec<&str> = TASKS
      .iter()
      .enumerate()
      .filter(|(task, _)| places.get(*task) == Some(&1))
      .map(|(_, name)| *name)
      .collect();
    let (reset_places, reset_moves) = (self.places.clone(), self.moves.clone());
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Button::new("Start again")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon(icons::REFRESH)
              .on_click(move |_| {
                reset_places.set(vec![0; TASKS.len()]);
                reset_moves.set(0);
              }),
          )
          .child(quiet(
            &format!(
              "done: {} · moved {} times",
              if done.is_empty() {
                "nothing".to_owned()
              } else {
                done.join(", ")
              },
              self.moves.get()
            ),
            12.0,
          )),
      )
      .child(fit("drag-board", self.frame.stage_room(), width, scene))
      .into()
  }
}

impl Component for DragAndDropPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      bounds: ctx.signal(0),
      movement: ctx.signal(0),
      miss: ctx.signal(0),
      policy: ctx.signal(0),
      buttons: ctx.signal(0),
      follower: ctx.signal(false),
      played: Played {
        card: ElementRefMut::new(),
        follower: ElementRefMut::new(),
        docked: ctx.signal(false),
        dragging: ctx.signal(false),
      },
      status: ctx.signal("Drop target".to_owned()),
      keep: ElementRefMut::new(),
      revert: ElementRefMut::new(),
      places: ctx.signal(vec![0; TASKS.len()]),
      moves: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "on_drag_start / on_drag_move / on_drag_end",
        "Fn(&DragEvent)",
        "",
        "Draggable: told where the pointer is and how far it went from the start. The end says whether a zone \
         took the drop: drop_result is Accepted or Missed.",
      ),
      (
        "movement",
        "DragMovement",
        "Automatic",
        "Draggable: Automatic moves the element by each step of the pointer; Manual leaves it where it is, for \
         on_drag_move to place it, snapped to a grid or held on a line.",
      ),
      (
        "drop_miss_behavior",
        "DropMissBehavior",
        "KeepPosition",
        "Draggable: what a drop on no zone does. KeepPosition leaves the element where it was let go; \
         RevertToDragStart puts it back where the drag started.",
      ),
      (
        "override_policy",
        "DragOverridePolicy",
        "Keep",
        "Draggable: Keep leaves the element where the drag put it; Clear gives its place back to layout at the \
         end, for an element the app places from its state, which a drop changes.",
      ),
      (
        "start_drag_buttons",
        "MouseButtonMask",
        "ANY",
        "Draggable: the buttons of the mouse a drag starts with.",
      ),
      (
        "element_ref",
        "ElementRefMut",
        "",
        "Draggable: a ref of the app's for the dragged element, to read its bounds or put it back \
         (clear_bounds_override). Without it the draggable makes one of its own.",
      ),
      (
        "follower / followers",
        "ElementRefMut",
        "",
        "Draggable: elements that move with the dragged one, go back with it, and are cleared with it.",
      ),
      (
        "payload",
        "impl Any + Send + Sync",
        "",
        "Draggable: what the drag carries, which a zone reads as event.payload::<T>().",
      ),
      (
        "on_drop",
        "Fn(&DropEvent)",
        "",
        "DropZone: called when a drag is let go over the zone's element, with where, from which element and \
         what it carried.",
      ),
      (
        "bounds",
        "DragBounds",
        "SelfBounds",
        "DragContainer: SelfBounds keeps every draggable in it inside its box; None lets them go anywhere.",
      ),
    ];
    let playground = self.play(ctx);
    let container_drop = self.container_drop(ctx);
    let board = self.board(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Drag and Drop"],
        title: "Drag and Drop",
        description: "Draggable, DropZone and DragContainer: an element that moves under the pointer, one that \
                      hears what is let go over it, and a box that keeps what is dragged inside it. The design \
                      system's sortable lists and splitters are built on them.",
        import: Some(
          "use lurq::components::{DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, \
           DropZoneProps};",
        ),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A card in a container, and a zone beside it. Drag the card, let it go on the zone and beside it, and \
           choose what a miss does, whether the drag's place is kept, and whether the container holds it. With \
           Clear, a drop on the zone moves the card there by the app's state.",
          playground,
        ),
        Section::new(
          "Container drop",
          "Two cards in a container of 400 by 280: keep stays where it is let go, revert goes back where it started \
           when it misses a zone. The zones say what happened last. Neither card leaves the container.",
          self.frame.demo(
            0,
            container_drop,
            "DraggableProps::new().drop_miss_behavior(DropMissBehavior::RevertToDragStart)",
            &[
              "let top = DropZone::mount(",
              "    ctx,",
              "    DropZoneProps::new().on_drop(move |_| status.set(\"Dropped top\".into())),",
              "    zone(&status.get()).absolute_position(220.0, 60.0),",
              ");",
              "let revert = Draggable::mount(",
              "    ctx,",
              "    DraggableProps::new()",
              "        .drop_miss_behavior(DropMissBehavior::RevertToDragStart)",
              "        .on_drag_start(move |_| status.set(\"Dragging revert\".into())),",
              "    card(\"revert\").absolute_position(20.0, 140.0),",
              ");",
              "DragContainer::mount(",
              "    ctx,",
              "    DragContainerProps::new(),",
              "    Stack::new().size(400.0, 280.0).with_children([top, bottom, keep, revert]),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Placed by the app's state",
          "Each card carries its number as a payload. A drop on a column reads it and moves the card there in the \
           app's state; the drag's place is cleared at its end, so a card let go anywhere else goes back to where \
           the state has it.",
          self.frame.demo(
            1,
            board,
            "DraggableProps::new().payload(task).override_policy(DragOverridePolicy::Clear)",
            &[
              "// places: Signal<Vec<usize>>, the column of each card",
              "DropZone::mount(",
              "    ctx,",
              "    DropZoneProps::new().on_drop(move |event| {",
              "        if let Some(task) = event.payload::<usize>() {",
              "            places.update(|places| places[*task] = column);",
              "        }",
              "    }),",
              "    column_box,",
              ")",
              "",
              "Draggable::mount(",
              "    ctx,",
              "    DraggableProps::new().payload(task).override_policy(DragOverridePolicy::Clear),",
              "    card(name).absolute_position(x_of(places[task]), y),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a draggable, a drop zone and a drag container are given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
