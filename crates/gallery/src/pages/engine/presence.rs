//! The page of presence: what is removed stays on screen, painted only, until its exit animation ends. Its
//! playground shows and hides a row of a container as its props say; its sections are the demo's Presence page:
//! menus and a dialog that enter and leave, a list whose rows leave when they are removed, and a list in a menu.

use std::sync::Arc;

use lurq::{
  animation::{AnimatableProperty, Animation, KeyframeBuilder, Keyframes, KeyframesId},
  app::{
    Tree,
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle},
  },
  components::{Column, Modal, Popup, Rect, Root, Row, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, transform::Decomposed},
  ui::{Button, ButtonSize, ButtonVariant, Switch, Toggle},
};

use super::kit::{controls, number};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

// The keyframes the page plays, registered on the tree by `register_keyframes`: see `engine::prepare` for the
// numbers.
const MENU_IN: KeyframesId = KeyframesId::new(0x0D20);
const MENU_OUT: KeyframesId = KeyframesId::new(0x0D21);
const FADE_IN: KeyframesId = KeyframesId::new(0x0D22);
const FADE_OUT: KeyframesId = KeyframesId::new(0x0D23);
const PANEL_IN: KeyframesId = KeyframesId::new(0x0D24);
const PANEL_OUT: KeyframesId = KeyframesId::new(0x0D25);
/// A row that fades and then gives its room to the rows after it.
const ROW_CLOSE: KeyframesId = KeyframesId::new(0x0D26);
/// The same for an item of a menu, which is lower.
const ITEM_CLOSE: KeyframesId = KeyframesId::new(0x0D27);
const ROW_HEIGHT: f32 = 36.0;
const ITEM_HEIGHT: f32 = 30.0;

/// The keyframes of the page, told to a tree once: `engine::prepare` does.
pub fn register_keyframes(tree: &mut Tree) {
  let slide = |opacity: f32, y: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Opacity, opacity);
      frame.set(AnimatableProperty::OffsetY, y);
    }
  };
  let grow = |opacity: f32, scale: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Opacity, opacity);
      frame.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_scale(scale, scale),
      );
    }
  };
  let fade = |opacity: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Opacity, opacity);
    }
  };
  let row = |opacity: f32, height: f32| {
    move |frame: &mut KeyframeBuilder| {
      frame.set(AnimatableProperty::Opacity, opacity);
      frame.set(AnimatableProperty::Height, height);
    }
  };
  tree.register_keyframes(
    Keyframes::new(MENU_IN)
      .frame(0.0, slide(0.0, -10.0))
      .frame(1.0, slide(1.0, 0.0)),
  );
  tree.register_keyframes(
    Keyframes::new(MENU_OUT)
      .frame(0.0, slide(1.0, 0.0))
      .frame(1.0, slide(0.0, -10.0)),
  );
  tree.register_keyframes(Keyframes::new(FADE_IN).frame(0.0, fade(0.0)).frame(1.0, fade(1.0)));
  tree.register_keyframes(Keyframes::new(FADE_OUT).frame(0.0, fade(1.0)).frame(1.0, fade(0.0)));
  tree.register_keyframes(
    Keyframes::new(PANEL_IN)
      .frame(0.0, grow(0.0, 0.9))
      .frame(1.0, grow(1.0, 1.0)),
  );
  tree.register_keyframes(
    Keyframes::new(PANEL_OUT)
      .frame(0.0, grow(1.0, 1.0))
      .frame(1.0, grow(0.0, 0.9)),
  );
  for (keyframes, height) in [(ROW_CLOSE, ROW_HEIGHT), (ITEM_CLOSE, ITEM_HEIGHT)] {
    tree.register_keyframes(
      Keyframes::new(keyframes)
        .frame(0.0, row(1.0, height))
        .frame(0.5, row(0.0, height))
        .frame(1.0, row(0.0, 0.0)),
    );
  }
}

/// How the playground's row comes in: an index of it, the keyframes and their name in the code.
const ENTERS: [(&str, usize); 4] = [("none", 0), ("FADE_IN", 1), ("MENU_IN", 2), ("PANEL_IN", 3)];
const ENTER_KEYFRAMES: [Option<KeyframesId>; 4] = [None, Some(FADE_IN), Some(MENU_IN), Some(PANEL_IN)];
const EXITS: [(&str, usize); 5] = [
  ("none", 0),
  ("FADE_OUT", 1),
  ("MENU_OUT", 2),
  ("PANEL_OUT", 3),
  ("ROW_CLOSE", 4),
];
const EXIT_KEYFRAMES: [Option<KeyframesId>; 5] =
  [None, Some(FADE_OUT), Some(MENU_OUT), Some(PANEL_OUT), Some(ROW_CLOSE)];
/// What each exit runs, as the panel says it.
const EXIT_SAYS: [&str; 5] = [
  "gone with the render",
  "opacity",
  "opacity, offset y",
  "opacity, scale",
  "opacity, then height",
];
const DURATIONS: [(&str, i32); 4] = [("150 ms", 150), ("400 ms", 400), ("1000 ms", 1000), ("2000 ms", 2000)];

/// The tasks the list starts with, and the items of the menu of tools.
const TASKS: [&str; 5] = [
  "Write the plan",
  "Draw the icons",
  "Build the engine",
  "Check on a Mac",
  "Ship it",
];
const TOOLS: [&str; 5] = ["Format", "Lint", "Test", "Bench", "Publish"];

pub struct PresencePage {
  frame: Frame,
  playground: Playground,
  /// Whether the playground's row is there.
  shown: Signal<bool>,
  /// Indices into [`ENTER_KEYFRAMES`] and [`EXIT_KEYFRAMES`].
  enter: Signal<usize>,
  exit: Signal<usize>,
  play_duration: Signal<i32>,
  play_presence: Signal<bool>,

  file: Signal<bool>,
  edit: Signal<bool>,
  dialog: Signal<bool>,
  tools: Signal<bool>,
  page_clicks: Signal<i32>,
  /// How long an animation of the sections takes, in milliseconds: slow, to see a layer on its way.
  duration: Signal<i32>,
  /// The tasks of the list, each its own key: a task added is "Task" and a number.
  tasks: Signal<Vec<String>>,
  next_task: Signal<i32>,
  tools_items: Signal<Vec<String>>,
  /// Whether a removed row gives its room away as it leaves, or only fades.
  closes_up: Signal<bool>,
  with_presence: Signal<bool>,
  renders: Signal<i32>,
  file_anchor: ElementRef,
  edit_anchor: ElementRef,
  tools_anchor: ElementRef,
}

/// A row of a list, in the type of the page.
fn row_text(content: &str) -> lurq::components::Text {
  text(content, 13.0, FontWeight::Normal)
}

/// An item of a menu, lit under the pointer.
fn menu_item(caption: &str) -> Row {
  Row::new()
    .width(FILL)
    .height(ITEM_HEIGHT)
    .padding_horizontal(10.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Sm)
    .hovered(|style| {
      style
        .background(PaletteColor::Highlight)
        .foreground(PaletteColor::HighlightForeground)
    })
    .child(row_text(caption))
}

impl PresencePage {
  fn animate(&self, keyframes: KeyframesId) -> Animation {
    Animation::new(keyframes)
      .duration_ms(self.duration.get().max(1) as u64)
      .linear()
  }

  /// The panel of a menu: it slides down as it enters and up as it leaves.
  fn menu_panel(&self) -> Column {
    Column::new()
      .width(190.0)
      .padding(6.0)
      .spacing(2.0)
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Popover)
      .foreground(PaletteColor::PopoverForeground)
      .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      .box_shadow(ShadowStyle::Md)
      .animation(self.animate(MENU_IN))
      .exit_animation(self.animate(MENU_OUT))
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let (shown, enter, exit) = (self.shown.get(), self.enter.get(), self.exit.get());
    let (duration, presence) = (self.play_duration.get(), self.play_presence.get());
    let (enter, exit) = (enter.min(ENTERS.len() - 1), exit.min(EXITS.len() - 1));
    let scene = move |button: Element| -> Element {
      let mut list = Column::new().width(FILL);
      if presence {
        list = list.presence();
      }
      let line_row = |caption: &str| {
        Row::new()
          .width(FILL)
          .height(ROW_HEIGHT)
          .padding_horizontal(12.0)
          .align_items(Alignment::Center)
          .child(row_text(caption))
      };
      list = list.child(line_row("A row before it").key("before"));
      if shown {
        let mut row = Row::new()
          .key("shown")
          .width(FILL)
          .height(ROW_HEIGHT)
          .padding_horizontal(12.0)
          .align_items(Alignment::Center)
          .background(PaletteColor::Chart1)
          .child(row_text("The row that comes and goes").color(PaletteColor::Background));
        if let Some(keyframes) = ENTER_KEYFRAMES[enter] {
          row = row.animation(Animation::new(keyframes).duration_ms(duration as u64).linear());
        }
        if let Some(keyframes) = EXIT_KEYFRAMES[exit] {
          row = row.exit_animation(Animation::new(keyframes).duration_ms(duration as u64).linear());
        }
        list = list.child(row);
      }
      list = list.child(line_row("A row after it").key("after"));
      Column::new()
        .width(300.0)
        .max_width(FILL)
        .spacing(16.0)
        .align_items(Alignment::Start)
        .child(button)
        .child(
          Column::new()
            .width(FILL)
            .clip()
            .rounded(RadiusSize::Md)
            .background(PaletteColor::Background)
            .border(line())
            .child(list),
        )
        .into()
    };
    let button =
      |shown: bool| Button::new(if shown { "Remove the row" } else { "Put the row back" }).size(ButtonSize::Sm);
    let (record, set_shown) = (self.playground.recorder(), self.shown.clone());
    let pressed = button(shown).on_click(move |_| {
      set_shown.set(!shown);
      record(format!("on_click: {}", if shown { "removed" } else { "put back" }));
    });

    let mut code = vec![];
    if presence {
      code.push("let mut list = Column::new().presence();".to_owned());
    } else {
      code.push("let mut list = Column::new();".to_owned());
    }
    code.push("list = list.child(Row::new().key(\"before\").height(36.0));".to_owned());
    code.push("if shown {".to_owned());
    code.push("    list = list.child(".to_owned());
    code.push("        Row::new()".to_owned());
    code.push("            .key(\"shown\")".to_owned());
    code.push("            .height(36.0)".to_owned());
    if enter != 0 {
      code.push(format!(
        "            .animation(Animation::new({}).duration_ms({duration}).linear())",
        ENTERS[enter].0
      ));
    }
    if exit != 0 {
      code.push(format!(
        "            .exit_animation(Animation::new({}).duration_ms({duration}).linear())",
        EXITS[exit].0
      ));
    }
    code.push("    );".to_owned());
    code.push("}".to_owned());
    code.push("list = list.child(Row::new().key(\"after\").height(36.0));".to_owned());

    let mut tree = vec![format!(
      "column{}   {}",
      if presence { ".presence()" } else { "" },
      self.playground.measured()
    )];
    tree.push("├─ row \"before\"".to_owned());
    if shown {
      tree.push(format!(
        "├─ row \"shown\"   {}{}",
        if enter != 0 { "enters " } else { "" },
        if exit != 0 { "leaves" } else { "" }
      ));
    }
    tree.push("└─ row \"after\"".to_owned());

    let (set_enter, set_exit, set_duration) = (self.enter.clone(), self.exit.clone(), self.play_duration.clone());
    let reset = {
      let (shown, enter, exit, duration, presence) = (
        self.shown.clone(),
        self.enter.clone(),
        self.exit.clone(),
        self.play_duration.clone(),
        self.play_presence.clone(),
      );
      Arc::new(move || {
        shown.set(true);
        enter.set(1);
        exit.set(4);
        duration.set(1000);
        presence.set(true);
      })
    };
    let leaves = presence && exit != 0;
    self.playground.card(
      ctx,
      Spec {
        title: "Presence",
        make: Arc::new(move || scene(button(shown).into())),
        shown: Shown::built(move |_| scene(pressed.into())),
        fill: true,
        forceable: false,
        facts: if leaves {
          format!("leaves: {}", EXIT_SAYS[exit])
        } else {
          "gone with the render".to_owned()
        },
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop(
            "animation",
            "Animation",
            self
              .playground
              .select(0, &ENTERS, &enter, move |value| set_enter.set(value)),
          ),
          prop(
            "exit_animation",
            "Animation",
            self
              .playground
              .select(1, &EXITS, &exit, move |value| set_exit.set(value)),
          ),
          prop(
            "duration_ms",
            "u64",
            self
              .playground
              .select(2, &DURATIONS, &duration, move |value| set_duration.set(value)),
          ),
          divider(),
          flag("presence() on the column", &self.play_presence),
        ],
        changed: [!shown, enter != 1, exit != 4, duration != 1000, !presence]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// Two menus and a dialog, with the slider of their duration and an area that counts clicks.
  fn layers(&self) -> Column {
    let opener =
      |caption: &str, anchor: &ElementRef, open: &Signal<bool>, other: &Signal<bool>| {
        let (open, other) = (open.clone(), other.clone());
        // A row holds the anchor: the popup hangs from it.
        Row::new().ref_element(anchor.clone()).child(
          Toggle::new(caption)
            .size(ButtonSize::Sm)
            .on(open.get())
            .on_change(move |on| {
              other.set(false);
              open.set(on);
            }),
        )
      };
    let menu = |anchor: &ElementRef, open: &Signal<bool>, items: [&str; 4]| {
      let mut panel = self.menu_panel();
      for item in items {
        panel = panel.child(menu_item(item));
      }
      Popup::new(anchor.clone(), panel).open(open.clone()).offset(0.0, 6.0)
    };
    let open_dialog = self.dialog.clone();
    let clicks = self.page_clicks.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(number("duration, ms", &self.duration, 60, 2000))
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(opener("File", &self.file_anchor, &self.file, &self.edit))
          .child(opener("Edit", &self.edit_anchor, &self.edit, &self.file))
          .child(
            Button::new("Dialog")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| open_dialog.set(true)),
          )
          .child(
            Row::new()
              .spacing(8.0)
              .align_items(Alignment::Center)
              .child(quiet("clicks on the area below", 12.0))
              .child(mono(&self.page_clicks.get().to_string(), 13.0)),
          ),
      )
      .child(
        Column::new()
          .width(FILL)
          .height(150.0)
          .padding(14.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border).dashed())
          .cursor(CursorIcon::Pointer)
          .on_click(move |_| clicks.update(|count| *count += 1))
          .child(
            row_text("Click here. A click where a menu is still leaving counts: the menu takes nothing.").width(FILL),
          ),
      )
      .child(menu(&self.file_anchor, &self.file, ["New", "Open…", "Save", "Close"]))
      .child(menu(&self.edit_anchor, &self.edit, ["Undo", "Cut", "Copy", "Paste"]))
      .child(self.dialog_layer())
  }

  /// A dialog whose backdrop fades while its panel fades and grows, each with its own exit.
  fn dialog_layer(&self) -> Modal {
    let (close, dismiss) = (self.dialog.clone(), self.dialog.clone());
    let backdrop = Rect::new(FILL, FILL)
      .background(PaletteColor::Scrim)
      .animation(self.animate(FADE_IN))
      .exit_animation(self.animate(FADE_OUT))
      .on_click(move |_| dismiss.set(false));
    let panel = Column::new()
      .width(340.0)
      .max_width(FILL)
      .padding(24.0)
      .spacing(14.0)
      .rounded(RadiusSize::Xl)
      .background(PaletteColor::Popover)
      .foreground(PaletteColor::PopoverForeground)
      .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      .box_shadow(ShadowStyle::Lg)
      .animation(self.animate(PANEL_IN))
      .exit_animation(self.animate(PANEL_OUT))
      .child(text("A dialog", 18.0, FontWeight::SemiBold))
      .child(
        text(
          "Its backdrop fades. Its panel fades and grows. Each leaves with its own animation.",
          13.0,
          FontWeight::Normal,
        )
        .color(PaletteColor::MutedForeground)
        .width(FILL),
      )
      .child(
        Row::new().width(FILL).justify(Justify::End).child(
          Button::new("Close")
            .size(ButtonSize::Sm)
            .on_click(move |_| close.set(false)),
        ),
      );
    Modal::new(
      Stack::new()
        .size(FILL, FILL)
        .padding(16.0)
        .stack_align(StackAlignment::Center)
        .child(backdrop)
        .child(panel),
    )
    .open(self.dialog.clone())
    .target(Root)
    .takes_focus(true)
  }

  /// A row of the list of tasks: a press removes it.
  fn task_row(&self, title: &str) -> Row {
    let (tasks, removed) = (self.tasks.clone(), title.to_owned());
    let leave = if self.closes_up.get() { ROW_CLOSE } else { FADE_OUT };
    Row::new()
      .key(format!("task-{title}"))
      .width(FILL)
      .height(ROW_HEIGHT)
      .padding_horizontal(12.0)
      .align_items(Alignment::Center)
      .border_bottom(line())
      .cursor(CursorIcon::Pointer)
      .hovered(|style| style.background(PaletteColor::StateHover))
      .exit_animation(self.animate(leave))
      .on_click(move |_| tasks.update(|tasks| tasks.retain(|kept| *kept != removed)))
      .child(row_text(title))
  }

  /// The list of tasks, with what adds, removes and renders again, and the two switches.
  fn list(&self) -> Column {
    let mut list = Column::new().width(FILL);
    if self.with_presence.get() {
      list = list.presence();
    }
    for title in self.tasks.get() {
      list = list.child(self.task_row(&title));
    }
    let (tasks, next) = (self.tasks.clone(), self.next_task.clone());
    let add = move |_| {
      let number = next.get();
      next.set(number + 1);
      tasks.update(|tasks| tasks.insert(tasks.len().min(1), format!("Task {number}")));
    };
    let (first, all, renders) = (self.tasks.clone(), self.tasks.clone(), self.renders.clone());
    let action = |caption: &str| {
      Button::new(caption)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
    };
    let switch = |caption: &str, value: &Signal<bool>| {
      let set = value.clone();
      Switch::new(caption).on(value.get()).on_change(move |on| set.set(on))
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(action("Add a task").on_click(add))
          .child(action("Remove the first").on_click(move |_| {
            first.update(|tasks| {
              if !tasks.is_empty() {
                tasks.remove(0);
              }
            })
          }))
          .child(action("Remove every other").on_click(move |_| {
            all.update(|tasks| {
              let mut index = 0;
              tasks.retain(|_| {
                index += 1;
                index % 2 == 0
              });
            })
          }))
          .child(action("Render again").on_click(move |_| renders.update(|count| *count += 1)))
          .child(
            Row::new()
              .spacing(8.0)
              .align_items(Alignment::Center)
              .child(quiet("renders", 12.0))
              .child(mono(&self.renders.get().to_string(), 13.0)),
          ),
      )
      .child(
        controls()
          .child(switch("The list has presence", &self.with_presence))
          .child(switch("A row gives its room away", &self.closes_up)),
      )
      .child(
        Column::new()
          .width(360.0)
          .max_width(FILL)
          .clip()
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Background)
          .border(line())
          .child(list),
      )
  }

  /// A menu whose items leave when they are pressed: it gets shorter once one is gone.
  fn list_in_a_menu(&self) -> Column {
    let mut items = Column::new().presence().width(FILL);
    for item in self.tools_items.get() {
      let (kept, removed) = (self.tools_items.clone(), item.clone());
      items = items.child(
        menu_item(&item)
          .key(item.clone())
          .cursor(CursorIcon::Pointer)
          .exit_animation(self.animate(ITEM_CLOSE))
          .on_click(move |_| kept.update(|items| items.retain(|other| *other != removed))),
      );
    }
    let (open, restore) = (self.tools.clone(), self.tools_items.clone());
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Row::new().ref_element(self.tools_anchor.clone()).child(
              Toggle::new("Tools")
                .size(ButtonSize::Sm)
                .on(open.get())
                .on_change(move |on| open.set(on)),
            ),
          )
          .child(
            Button::new("Put the items back")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| restore.set(TOOLS.iter().map(|tool| (*tool).to_owned()).collect())),
          ),
      )
      .child(
        Popup::new(self.tools_anchor.clone(), self.menu_panel().child(items))
          .open(self.tools.clone())
          .offset(0.0, 6.0),
      )
  }
}

impl Component for PresencePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      shown: ctx.signal(true),
      enter: ctx.signal(1),
      exit: ctx.signal(4),
      play_duration: ctx.signal(1000),
      play_presence: ctx.signal(true),
      file: ctx.signal(false),
      edit: ctx.signal(false),
      dialog: ctx.signal(false),
      tools: ctx.signal(false),
      page_clicks: ctx.signal(0),
      duration: ctx.signal(400),
      tasks: ctx.signal(TASKS.iter().map(|task| (*task).to_owned()).collect()),
      next_task: ctx.signal(TASKS.len() as i32),
      tools_items: ctx.signal(TOOLS.iter().map(|tool| (*tool).to_owned()).collect()),
      closes_up: ctx.signal(true),
      with_presence: ctx.signal(true),
      renders: ctx.signal(0),
      file_anchor: ElementRef::new(),
      edit_anchor: ElementRef::new(),
      tools_anchor: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Read here, so that "Render again" renders the page.
    let _ = self.renders.get();
    let rows = [
      (
        "exit_animation",
        "Animation",
        "",
        "On an element: what it leaves with. In a layer that closes, or a child of a container with presence, it \
         stays on screen, painted only, until the longest exit in it ends.",
      ),
      (
        "animation",
        "Animation",
        "",
        "What it plays from when it is mounted: how it enters. A layer's content is mounted when the layer opens.",
      ),
      (
        "presence",
        "()",
        "",
        "On a container: a child a render removes stays where it was until its exit animation ends. Without it the \
         child is gone with the render.",
      ),
      (
        "key",
        "impl Into<Arc<str>>",
        "",
        "What tells a child of a container from the others: one with presence needs a key or an id on each child \
         that leaves. The same key added again while it leaves takes it back.",
      ),
      (
        "Popup::open",
        "impl Into<OpenState>",
        "",
        "A layer hung from an anchor, open while the signal is true: Escape and a press outside close it.",
      ),
      (
        "Modal::open",
        "impl Into<OpenState>",
        "",
        "A layer over what it targets (Root: the window), open while the signal is true.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Presence"],
        title: "Presence",
        description: "What leaves stays on screen until its exit animation ends: a layer that closes, and a child a \
                      container with presence lost. On its way it is only painted: it takes no press, no focus and \
                      no Tab.",
        import: Some("use lurq::{animation::Animation, components::{Column, Modal, Popup}};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A column of three rows, the middle one there or not. Choose how it enters and how it leaves, how long it \
           takes, and whether the column keeps a removed row until its exit ends; then remove the row and put it \
           back, also while it is on its way.",
          playground,
        ),
        Section::new(
          "Menus and a dialog",
          "Make the duration long and watch a layer on its way out. Open File, then press Edit: File leaves while \
           Edit enters. Close a menu with Escape or a press outside, and press Tab while one leaves: the focus does \
           not go into it. A click on the area while a menu leaves over it counts.",
          self.frame.demo(
            0,
            self.layers(),
            ".exit_animation(Animation::new(MENU_OUT).duration_ms(400))",
            &[
              "let menu = Column::new()",
              "    .width(190.0)",
              "    .animation(Animation::new(MENU_IN).duration_ms(400).linear())",
              "    .exit_animation(Animation::new(MENU_OUT).duration_ms(400).linear())",
              "    .child(item(\"New\"))",
              "    // ..",
              "Popup::new(anchor.clone(), menu).open(open.clone()).offset(0.0, 6.0)",
              "",
              "// A dialog: the backdrop fades, the panel fades and grows.",
              "Modal::new(",
              "    Stack::new()",
              "        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))",
              "        .stack_align(StackAlignment::Center)",
              "        .child(Rect::new(Dimension::Pct(100.0), Dimension::Pct(100.0))",
              "            .background(PaletteColor::Scrim)",
              "            .animation(Animation::new(FADE_IN).duration_ms(400))",
              "            .exit_animation(Animation::new(FADE_OUT).duration_ms(400)))",
              "        .child(panel",
              "            .animation(Animation::new(PANEL_IN).duration_ms(400))",
              "            .exit_animation(Animation::new(PANEL_OUT).duration_ms(400))),",
              ")",
              ".open(dialog.clone())",
              ".target(Root)",
            ],
          ),
        ),
        Section::new(
          "A list whose rows leave",
          "Press a task to remove it. A removed task stays where it was until its animation ends, and takes no press. \
           Remove several at once, add one while another leaves, and render again while one leaves: it goes on from \
           where it was. Without presence a removed task is gone at once.",
          self.frame.demo(
            1,
            self.list(),
            "Column::new().presence()",
            &[
              "// tasks: Signal<Vec<String>>",
              "let mut list = Column::new().presence();",
              "for title in tasks.get() {",
              "    list = list.child(",
              "        Row::new()",
              "            .key(format!(\"task-{title}\"))",
              "            .height(36.0)",
              "            // Fades, then gives its 36 to the rows after it.",
              "            .exit_animation(Animation::new(ROW_CLOSE).duration_ms(400).linear())",
              "            .on_click(move |_| tasks.update(|tasks| tasks.retain(|kept| *kept != title)))",
              "            .child(Text::new(&title)),",
              "    );",
              "}",
              "",
              "tree.register_keyframes(",
              "    Keyframes::new(ROW_CLOSE)",
              "        .frame(0.0, |f| { f.set(AnimatableProperty::Opacity, 1.0).set(AnimatableProperty::Height, 36.0); })",
              "        .frame(0.5, |f| { f.set(AnimatableProperty::Opacity, 0.0).set(AnimatableProperty::Height, 36.0); })",
              "        .frame(1.0, |f| { f.set(AnimatableProperty::Opacity, 0.0).set(AnimatableProperty::Height, 0.0); }),",
              ");",
            ],
          ),
        ),
        Section::new(
          "A list in a menu",
          "Open Tools and press an item: it leaves, and the menu gets shorter once it is gone, not before. Put the \
           items back to try again.",
          self.frame.demo(
            2,
            self.list_in_a_menu(),
            "Popup::new(anchor, panel.child(Column::new().presence()))",
            &[
              "let mut items = Column::new().presence();",
              "for item in tools.get() {",
              "    items = items.child(",
              "        Row::new()",
              "            .key(item.clone())",
              "            .height(30.0)",
              "            .exit_animation(Animation::new(ITEM_CLOSE).duration_ms(400).linear())",
              "            .on_click(move |_| tools.update(|items| items.retain(|other| *other != item))),",
              "    );",
              "}",
              "Popup::new(anchor.clone(), menu.child(items)).open(open.clone())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an element leaves with, what keeps a child that leaves, and the layers that close.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
