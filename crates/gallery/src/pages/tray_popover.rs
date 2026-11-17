//! The page of the tray popover: its playground, the board's panel, one hung from a tray's icon over the window,
//! and the tones of its status.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Progress, TrayAction, TrayPopover, TrayStatusVariant, TrayToggle, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The tones of the status, as the code names them.
const TONES: [(&str, TrayStatusVariant); 5] = [
  ("Neutral", TrayStatusVariant::Neutral),
  ("Info", TrayStatusVariant::Info),
  ("Success", TrayStatusVariant::Success),
  ("Warning", TrayStatusVariant::Warning),
  ("Destructive", TrayStatusVariant::Destructive),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(280.0)
    .max_width(FILL)
    .spacing(10.0)
    .overflow_visible()
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

pub struct TrayPopoverPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  status: Signal<String>,
  /// The tone, as an index of [`TONES`].
  tone: Signal<usize>,
  icon: Signal<bool>,
  toggles: Signal<bool>,
  body: Signal<bool>,
  disabled: Signal<bool>,
  /// The switches of the playground and of the sections: the page holds them.
  sync: Signal<bool>,
  paused: Signal<bool>,
  /// The tray of the section: its icon, and whether its popover is open.
  tray: (ElementRef, Signal<bool>),
  /// What the section's commands did last.
  ran: Signal<String>,
}

impl TrayPopoverPage {
  /// The board's popover, its switches the page's, its commands told to `ran`.
  fn board(&self, ran: impl Fn(&'static str) + Clone + Send + Sync + 'static) -> TrayPopover {
    let say = |what: &'static str| {
      let ran = ran.clone();
      move || ran(what)
    };
    TrayPopover::new("lurq")
      .icon(icons::BOX)
      .status("Synced just now")
      .item(TrayToggle::new("Sync in background").bind(self.sync.clone()))
      .item(TrayToggle::new("Pause notifications").bind(self.paused.clone()))
      .separator()
      .item(TrayAction::new("Open lurq").on_select(say("Open lurq")))
      .item(TrayAction::new("Preferences…").on_select(say("Preferences…")))
      .item(TrayAction::new("Quit").on_select(say("Quit")))
  }

  /// The playground: the panel as the props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, status) = (self.title.get(), self.status.get());
    let chosen = self.tone.get().min(TONES.len() - 1);
    let (tone_name, tone) = TONES[chosen];
    let (icon, toggles, body, disabled) = (
      self.icon.get(),
      self.toggles.get(),
      self.body.get(),
      self.disabled.get(),
    );
    let plain = {
      let (title, status) = (title.clone(), status.clone());
      move |sync: Signal<bool>, paused: Signal<bool>, record: Arc<dyn Fn(String) + Send + Sync>| {
        let mut popover = TrayPopover::new(&title).status_variant(tone);
        if icon {
          popover = popover.icon(icons::BOX);
        }
        if !status.is_empty() {
          popover = popover.status(&status);
        }
        if toggles {
          let (first, second) = (record.clone(), record.clone());
          popover = popover
            .item(
              TrayToggle::new("Sync in background")
                .bind(sync)
                .on_change(move |on| first(format!("on_change(Sync in background, {on})"))),
            )
            .item(
              TrayToggle::new("Pause notifications")
                .bind(paused)
                .disabled(disabled)
                .on_change(move |on| second(format!("on_change(Pause notifications, {on})"))),
            )
            .separator();
        }
        if body {
          popover = popover.child(Progress::new(0.4).label("Uploading 3 files")).separator();
        }
        for label in ["Open lurq", "Preferences…", "Quit"] {
          let said = record.clone();
          popover = popover.item(
            TrayAction::new(label)
              .disabled(disabled && label == "Preferences…")
              .on_select(move || said(format!("on_select({label})"))),
          );
        }
        popover
      }
    };
    let record = self.playground.recorder();
    let shown = plain(self.sync.clone(), self.paused.clone(), Arc::new(record)).force(self.playground.forced());

    let mut code = vec![format!("TrayPopover::new({title:?})")];
    if icon {
      code.push("    .icon(icons::BOX)".to_owned());
    }
    if !status.is_empty() {
      code.push(format!("    .status({status:?})"));
    }
    if tone != TrayStatusVariant::Success {
      code.push(format!("    .status_variant(TrayStatusVariant::{tone_name})"));
    }
    if toggles {
      code.push("    .item(TrayToggle::new(\"Sync in background\").bind(sync.clone()))".to_owned());
      code.push(format!(
        "    .item(TrayToggle::new(\"Pause notifications\").bind(paused.clone()){})",
        if disabled { ".disabled(true)" } else { "" }
      ));
      code.push("    .separator()".to_owned());
    }
    if body {
      code.push("    .child(Progress::new(0.4).label(\"Uploading 3 files\"))".to_owned());
      code.push("    .separator()".to_owned());
    }
    code.push("    .item(TrayAction::new(\"Open lurq\").on_select(|| show_window()))".to_owned());
    code.push(format!(
      "    .item(TrayAction::new(\"Preferences…\"){}.on_select(|| preferences()))",
      if disabled { ".disabled(true)" } else { "" }
    ));
    code.push("    .item(TrayAction::new(\"Quit\").on_select(|| quit()))".to_owned());

    let mut tree = vec![
      format!(
        "panel         {}   radius 12 · padded 10 · shadow lg",
        self.playground.measured()
      ),
      format!(
        "├─ header     {}title 13{}",
        if icon { "mark 16 on a tile of 28 · " } else { "" },
        if status.is_empty() {
          String::new()
        } else {
          format!(" · {} dot 6 · status 11", tone_name.to_lowercase())
        }
      ),
      "├─ separator  1 · 4 over and under".to_owned(),
    ];
    if toggles {
      tree.push("├─ toggle ×2  30 · words 13 · a switch at the end".to_owned());
      tree.push("├─ separator".to_owned());
    }
    if body {
      tree.push("├─ body       the app's own".to_owned());
      tree.push("├─ separator".to_owned());
    }
    tree.push("└─ item ×3    28 · radius 4 · highlight under the pointer".to_owned());

    let to = (self.title.clone(), self.status.clone(), self.tone.clone());
    let flags = [
      self.icon.clone(),
      self.toggles.clone(),
      self.body.clone(),
      self.disabled.clone(),
    ];
    let choose = self.tone.clone();
    let tones: Vec<(&str, usize)> = TONES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Tray Popover",
        make: Arc::new(move || plain(Signal::new(true), Signal::new(false), Arc::new(|_| ())).into()),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: true,
        facts: format!("{} · in the flow", tone_name.to_lowercase()),
        event: "on_select / on_change",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("status", "&str", field(&self.status)),
          prop(
            "status_variant",
            "TrayStatusVariant",
            self
              .playground
              .select(0, &tones, &chosen, move |index| choose.set(index)),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("toggles", &self.toggles),
          flag("child", &self.body),
          flag("disabled rows", &self.disabled),
        ],
        changed: [
          title != "lurq",
          status != "Synced just now",
          tone != TrayStatusVariant::Success,
          !icon,
          !toggles,
          body,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("lurq".to_owned());
          to.1.set("Synced just now".to_owned());
          to.2.set(2);
          flags[0].set(true);
          flags[1].set(true);
          flags[2].set(false);
          flags[3].set(false);
        }),
      },
    )
  }

  /// A picture of a tray, as the board draws one, whose last icon is the app's and opens the popover over the
  /// window.
  fn tray(&self) -> Element {
    let (anchor, open) = &self.tray;
    let turn = open.clone();
    let ran = self.ran.clone();
    let mark = |name: &str| {
      Button::new(name)
        .icon(name)
        .icon_only(true)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Xs)
        .disabled(true)
    };
    let strip = Row::new()
      .height(30.0)
      .padding_horizontal(8.0)
      .spacing(4.0)
      .align_items(Alignment::Center)
      .corner_radius(6.0)
      .background(PaletteColor::Muted)
      .child(mark(icons::CHEVRON_UP))
      .child(mark(icons::WIFI))
      .child(mark(icons::VOLUME))
      .child(
        Row::new().ref_element(anchor.clone()).child(
          Button::new("lurq")
            .icon(icons::BOX)
            .icon_only(true)
            .variant(if open.get() {
              ButtonVariant::Secondary
            } else {
              ButtonVariant::Ghost
            })
            .size(ButtonSize::Xs)
            .on_click(move |_| turn.update(|open| *open = !*open)),
        ),
      )
      .child(text("14:02", 12.0, lurq::layout::text_style::FontWeight::Normal));
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .align_items(Alignment::End)
      .overflow_visible()
      .child(strip)
      .child(quiet(
        &match self.ran.get().as_str() {
          "" => "the box opens it · a command closes it and runs".to_owned(),
          ran => format!("ran: {ran}"),
        },
        12.0,
      ))
      .child(
        self
          .board(move |what| ran.set(what.to_owned()))
          .anchored(open.clone(), anchor.clone()),
      )
      .into()
  }
}

impl Component for TrayPopoverPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("lurq".to_owned()),
      status: ctx.signal("Synced just now".to_owned()),
      tone: ctx.signal(2),
      icon: ctx.signal(true),
      toggles: ctx.signal(true),
      body: ctx.signal(false),
      disabled: ctx.signal(false),
      sync: ctx.signal(true),
      paused: ctx.signal(false),
      tray: (ElementRef::new(), ctx.signal(false)),
      ran: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items().child(item(self.board(|_| ()), "tray popover"));
    let tray = self.tray();
    let tones = items()
      .child(item(
        TrayPopover::new("lurq")
          .icon(icons::BOX)
          .status("Syncing 3 files")
          .status_variant(TrayStatusVariant::Info)
          .child(Progress::new(0.4).label("Uploading 3 files"))
          .separator()
          .item(TrayAction::new("Pause sync")),
        "info · child",
      ))
      .child(item(
        TrayPopover::new("lurq")
          .icon(icons::BOX)
          .status("Offline")
          .status_variant(TrayStatusVariant::Warning)
          .item(TrayAction::new("Retry now"))
          .item(TrayAction::new("Check for updates").disabled(true)),
        "warning · a disabled command",
      ));
    let table = [
      (
        "new",
        "&str",
        "",
        "What the app is called: the title, and what the panel is called.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "",
        "The app's mark as an icon of 16 on a tile of 28 in the primary color.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "The app's own mark in place of the tile, 28 by 28.",
      ),
      (
        "status",
        "&str",
        "",
        "How the app is doing, under the title after a dot.",
      ),
      (
        "status_variant",
        "TrayStatusVariant",
        "Success",
        "Neutral, Info, Success, Warning, Destructive: the color of the dot.",
      ),
      (
        "item / items",
        "impl Into<TrayItem>",
        "",
        "A row: TrayAction::new(label).on_select(..) runs a command, TrayToggle::new(label).bind(signal) or \
         .on(bool).on_change(..) has a switch. Both have disabled, id and key.",
      ),
      ("separator", "()", "", "A line between two groups of rows."),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Something of the app's own among the rows: a progress, an account.",
      ),
      (
        "anchored",
        "(Signal<bool>, impl Into<ElementRef>)",
        "",
        "Shows it over the window beside an element while the signal is true, in place of where it is declared.",
      ),
      (
        "placement",
        "Placement",
        "BottomEnd",
        "The side of the anchor an anchored one is on: TopEnd over a taskbar.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "6",
        "How far an anchored one is from its anchor.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What an anchored one does where there is no room on its side.",
      ),
      (
        "on_dismiss",
        "impl Fn()",
        "",
        "Told after Escape or a press outside closed an anchored one; not on a command.",
      ),
      (
        "dismiss_on_escape, dismiss_on_outside_press",
        "bool",
        "true",
        "Whether each closes an anchored one.",
      ),
      ("width", "impl Into<Length>", "280", "How wide the panel is."),
      ("label", "&str", "the title", "What it is called to tooling."),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws its first command hovered, pressed or focused with nothing on it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Tray Popover"],
        title: "Tray Popover",
        description: "The small panel a press of an app's icon in the system tray or the menu bar opens: who the app \
                      is and how it is doing, a few switches, a few commands. It is the panel alone: an app with a \
                      tray icon renders it in a window of its own at the icon, and inside a window it is anchored to \
                      an element as a popover is.",
        import: Some("use lurq::ui::{TrayAction, TrayPopover, TrayStatusVariant, TrayToggle};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The panel where it is declared, as a window of its own would hold it. Every command run and every switch \
           turned is counted.",
          playground,
        ),
        Section::new(
          "Tray popover",
          "The board's panel: the app's mark, its title over its status, two switches, a line, three commands. The \
           arrow keys move through the switches and the commands.",
          self.frame.demo(
            0,
            board,
            "TrayPopover::new(\"lurq\").icon(..).status(..).item(..).separator().item(..)",
            &[
              "// sync, paused: Signal<bool>",
              "TrayPopover::new(\"lurq\")",
              "    .icon(icons::BOX)",
              "    .status(\"Synced just now\")",
              "    .item(TrayToggle::new(\"Sync in background\").bind(sync.clone()))",
              "    .item(TrayToggle::new(\"Pause notifications\").bind(paused.clone()))",
              "    .separator()",
              "    .item(TrayAction::new(\"Open lurq\").on_select(|| show_window()))",
              "    .item(TrayAction::new(\"Preferences…\").on_select(|| preferences()))",
              "    .item(TrayAction::new(\"Quit\").on_select(|| quit()))",
            ],
          ),
        ),
        Section::new(
          "Anchored",
          "Hung from an element of the window: here a picture of a tray whose last icon is the app's. Escape and a \
           press outside close it; a command closes it and then runs. A real tray icon is the platform's: the app \
           opens a window without a frame at the icon and renders the panel in it.",
          self.frame.demo(
            1,
            tray,
            ".anchored(open, anchor)",
            &[
              "// open: Signal<bool>, anchor: ElementRef, on the app's own button",
              "Row::new().ref_element(anchor.clone()).child(",
              "    Button::new(\"lurq\").icon(icons::BOX).icon_only(true).on_click(move |_| open.update(|open| *open = !*open)),",
              ")",
              "",
              "TrayPopover::new(\"lurq\")",
              "    .item(TrayAction::new(\"Quit\").on_select(|| quit()))",
              "    .anchored(open.clone(), anchor.clone())",
            ],
          ),
        ),
        Section::new(
          "Status and content",
          "The dot of the status has the color of its tone. What the app puts among the rows stands as wide as the \
           panel, and a row that takes no input is faded.",
          self.frame.demo(
            2,
            tones,
            ".status_variant(TrayStatusVariant::Warning) · .child(..)",
            &[
              "TrayPopover::new(\"lurq\")",
              "    .icon(icons::BOX)",
              "    .status(\"Syncing 3 files\")",
              "    .status_variant(TrayStatusVariant::Info)",
              "    .child(Progress::new(0.4).label(\"Uploading 3 files\"))",
              "    .separator()",
              "    .item(TrayAction::new(\"Pause sync\"))",
              "",
              "TrayPopover::new(\"lurq\")",
              "    .icon(icons::BOX)",
              "    .status(\"Offline\")",
              "    .status_variant(TrayStatusVariant::Warning)",
              "    .item(TrayAction::new(\"Retry now\"))",
              "    .item(TrayAction::new(\"Check for updates\").disabled(true))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a tray popover is given. Its title is all it needs.",
          props_table(&table, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
