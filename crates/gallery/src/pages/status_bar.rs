//! The page of the status bar: its playground, the board's bar of an editor, the tones of the bar and of an item,
//! and a narrow bar whose items leave it in the order the app says.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::{Element, dimension::Dimension},
  ui::{StatusBar, StatusBarVariant, StatusItem, StatusItemVariant, StatusRegion, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The tones the playground offers the bar, as the code names them.
const VARIANTS: [(&str, StatusBarVariant); 6] = [
  ("Neutral", StatusBarVariant::Neutral),
  ("Primary", StatusBarVariant::Primary),
  ("Info", StatusBarVariant::Info),
  ("Success", StatusBarVariant::Success),
  ("Warning", StatusBarVariant::Warning),
  ("Destructive", StatusBarVariant::Destructive),
];

/// The widths the playground offers the bar: the stage's, and narrower ones its items do not fit.
const WIDTHS: [(&str, Dimension); 4] = [
  ("the stage's", FILL),
  ("600", Dimension::Px(600.0)),
  ("420", Dimension::Px(420.0)),
  ("280", Dimension::Px(280.0)),
];

/// The ends that give way first, as the code names them.
const REGIONS: [(&str, StatusRegion); 2] = [("Trailing", StatusRegion::Trailing), ("Leading", StatusRegion::Leading)];

/// The board's bar, its pressed items telling `record`, the indexing at `done`.
fn editor(record: impl Fn(String) + Clone + Send + Sync + 'static, done: f64, errors: &str) -> StatusBar {
  let press = |what: &'static str| {
    let record = record.clone();
    move || record(format!("on_select(\"{what}\")"))
  };
  let errors_tone = if errors == "0" {
    StatusItemVariant::Neutral
  } else {
    StatusItemVariant::Destructive
  };
  StatusBar::new()
    .label("Editor status")
    .leading([
      StatusItem::new("feat/design-system")
        .icon(icons::BRANCH)
        .variant(StatusItemVariant::Primary)
        .tooltip("The branch checked out")
        .priority(2)
        .on_select(press("branch")),
      StatusItem::new("2↓ 1↑")
        .icon(icons::REFRESH)
        .tooltip("2 to pull, 1 to push")
        .on_select(press("sync")),
      StatusItem::new(errors)
        .icon(icons::CLOSE_CIRCLE)
        .variant(errors_tone)
        .tooltip("Errors")
        .priority(1)
        .on_select(press("errors")),
      StatusItem::new("3")
        .icon(icons::WARNING)
        .tooltip("Warnings")
        .priority(1)
        .on_select(press("warnings")),
      StatusItem::new("Indexing").progress(done),
    ])
    .trailing([
      StatusItem::new("Ln 42, Col 7")
        .priority(1)
        .on_select(press("go to line")),
      StatusItem::new("UTF-8").on_select(press("encoding")),
      StatusItem::new("Rust").on_select(press("language")),
      StatusItem::new("Notifications")
        .icon(icons::BELL)
        .icon_only(true)
        .tooltip("Notifications")
        .priority(3)
        .on_select(press("notifications")),
    ])
}

pub struct StatusBarPage {
  frame: Frame,
  playground: Playground,
  variant: Signal<usize>,
  width: Signal<usize>,
  region: Signal<usize>,
  errors: Signal<String>,
  progress: Signal<bool>,
  label: Signal<String>,
}

impl StatusBarPage {
  /// The playground: the board's bar as the panel's props make it, as wide as the width picked.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (variant_name, variant) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (width_name, width) = WIDTHS[self.width.get().min(WIDTHS.len() - 1)];
    let (region_name, region) = REGIONS[self.region.get().min(REGIONS.len() - 1)];
    let (errors, progress, label) = (self.errors.get(), self.progress.get(), self.label.get());
    let record = self.playground.recorder();
    let build = {
      let (errors, label) = (errors.clone(), label.clone());
      move || {
        let done = if progress { 0.55 } else { 1.0 };
        editor(record.clone(), done, &errors)
          .variant(variant)
          .drops_first(region)
          .label(&label)
      }
    };
    let shown = {
      let (build, playground) = (build.clone(), &self.playground);
      Shown::built(move |ctx| {
        Column::new()
          .width(width)
          .overflow_visible()
          .child(playground.measure(playground.styled(build()).mount(ctx)).width(FILL))
          .into()
      })
      .measured()
    };
    let mut code = vec![
      "StatusBar::new()".to_owned(),
      format!("    .label({label:?})"),
      "    .leading([".to_owned(),
      "        StatusItem::new(\"feat/design-system\").icon(icons::BRANCH).variant(StatusItemVariant::Primary),"
        .to_owned(),
      "        StatusItem::new(\"2↓ 1↑\").icon(icons::REFRESH).on_select(|| sync()),".to_owned(),
      format!("        StatusItem::new({errors:?}).icon(icons::CLOSE_CIRCLE).on_select(|| problems()),"),
      "        StatusItem::new(\"3\").icon(icons::WARNING).on_select(|| problems()),".to_owned(),
      format!(
        "        StatusItem::new(\"Indexing\").progress({}),",
        if progress { "0.55" } else { "1.0" }
      ),
      "    ])".to_owned(),
      "    .trailing([\"Ln 42, Col 7\", \"UTF-8\", \"Rust\"])".to_owned(),
    ];
    if variant != StatusBarVariant::Neutral {
      code.push(format!("    .variant(StatusBarVariant::{variant_name})"));
    }
    if region != StatusRegion::Trailing {
      code.push(format!("    .drops_first(StatusRegion::{region_name})"));
    }
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root        {} · muted · line on top", self.playground.measured()),
      "├─ item      padding 8 · gap 5 · icon 13 · text 12".to_owned(),
      "├─ item      primary · its fill and foreground".to_owned(),
      "├─ item      progress · muted text · rail 72 × 4".to_owned(),
      "├─ (room)".to_owned(),
      "└─ item …    trailing".to_owned(),
    ];

    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let widths: Vec<(&str, usize)> = WIDTHS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let regions: Vec<(&str, usize)> = REGIONS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let (set_variant, set_width, set_region) = (self.variant.clone(), self.width.clone(), self.region.clone());
    let (to_variant, to_width, to_region) = (self.variant.clone(), self.width.clone(), self.region.clone());
    let (to_errors, to_progress, to_label) = (self.errors.clone(), self.progress.clone(), self.label.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Status Bar",
        make: Arc::new(move || Column::new().width(width).child(build()).into()),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{width_name} wide"),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "StatusBarVariant",
            self
              .playground
              .select(0, &variants, &self.variant.get(), move |index| set_variant.set(index)),
          ),
          prop("label", "&str", field(&self.label)),
          prop("errors", "the item's text", field(&self.errors)),
          flag("progress", &self.progress),
          divider(),
          prop(
            "width",
            "of what holds it",
            self
              .playground
              .select(1, &widths, &self.width.get(), move |index| set_width.set(index)),
          ),
          prop(
            "drops_first",
            "StatusRegion",
            self
              .playground
              .select(2, &regions, &self.region.get(), move |index| set_region.set(index)),
          ),
        ],
        changed: [
          variant != StatusBarVariant::Neutral,
          self.width.get() != 0,
          region != StatusRegion::Trailing,
          errors != "0",
          !progress,
          label != "Editor status",
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_variant.set(0);
          to_width.set(0);
          to_region.set(0);
          to_errors.set("0".to_owned());
          to_progress.set(true);
          to_label.set("Editor status".to_owned());
        }),
      },
    )
  }
}

impl Component for StatusBarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      variant: ctx.signal(0),
      width: ctx.signal(0),
      region: ctx.signal(0),
      errors: ctx.signal("0".to_owned()),
      progress: ctx.signal(true),
      label: ctx.signal("Editor status".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let quiet_record = |_: String| {};
    let board = Column::new()
      .width(FILL)
      .child(editor(quiet_record, 0.55, "0").mount(ctx));
    let tones = Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        StatusBar::new()
          .leading([
            StatusItem::new("main").icon(icons::BRANCH),
            StatusItem::new("2 errors")
              .icon(icons::CLOSE_CIRCLE)
              .variant(StatusItemVariant::Destructive),
            StatusItem::new("1 warning")
              .icon(icons::WARNING)
              .variant(StatusItemVariant::Warning),
          ])
          .trailing([StatusItem::new("Tests passed")
            .icon(icons::CHECK)
            .variant(StatusItemVariant::Success)]),
      )
      .child(
        StatusBar::new()
          .variant(StatusBarVariant::Warning)
          .leading([StatusItem::new("Debugging").icon(icons::BUG)])
          .trailing(["Paused on a breakpoint"]),
      )
      .child(
        StatusBar::new()
          .variant(StatusBarVariant::Info)
          .leading([StatusItem::new("Connecting").progress(0.3)])
          .trailing(["Offline"]),
      );
    let mut narrow = Column::new().spacing(16.0);
    for width in [600.0, 420.0, 280.0] {
      narrow = narrow
        .child(
          Column::new()
            .width(width)
            .child(editor(quiet_record, 0.55, "0").mount(ctx)),
        )
        .child(quiet(&format!("{width} wide"), 12.0));
    }

    let rows = [
      ("new", "StatusBar", "", "An empty bar in the neutral tone."),
      (
        "leading, trailing",
        "impl IntoIterator<Item = impl Into<StatusItem>>",
        "",
        "Items at its start and at its end, after those it has there. A &str is an item of that text.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the bar in the tree with the tooltips of its items. Converted on its own it has none.",
      ),
      (
        "variant",
        "StatusBarVariant",
        "Neutral",
        "The tone of the whole bar: Primary, Info, Success, Warning, Destructive. Each item is in it unless it \
         has its own.",
      ),
      (
        "drops_first",
        "StatusRegion",
        "Trailing",
        "The end that gives way first where the bar is too narrow, each from its end.",
      ),
      (
        "label",
        "&str",
        "",
        "What the bar is called, to what reads the tree: a status.",
      ),
      (
        "StatusItem::new",
        "&str",
        "",
        "An item of that text, which is its name to what reads the tree.",
      ),
      (
        "StatusItem::icon, icon_only",
        "impl Into<Name>, bool",
        "",
        "An icon before the text, or alone.",
      ),
      (
        "StatusItem::on_select",
        "Fn()",
        "",
        "Pressed by a click, Space or Enter: the items pressed are one stop of Tab, whose arrows move between them.",
      ),
      (
        "StatusItem::variant",
        "StatusItemVariant",
        "the bar's",
        "Its tone: the muted fill and the color of it.",
      ),
      (
        "StatusItem::tooltip",
        "&str",
        "",
        "What a tooltip over it says, of a mounted bar.",
      ),
      (
        "StatusItem::progress",
        "impl Into<f64>",
        "",
        "How far it has gone, from 0 to 1: a rail after its text.",
      ),
      (
        "StatusItem::priority",
        "i32",
        "0",
        "How long it stays where the bar is too narrow: the lower leave first.",
      ),
      (
        "StatusItem::disabled, id, key",
        "",
        "",
        "A pressed item that takes nothing; its id and key.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Status Bar"],
        title: "Status Bar",
        description: "The bar along the bottom of a window, which says what state the app is in: the branch, the \
                      problems, what is being done at its start; where one is in a document at its end.",
        import: Some("use lurq::ui::{StatusBar, StatusItem, StatusItemVariant};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of StatusBar is a control on the right. Press an item, or Tab into the bar and move with the \
           arrow keys: the items that are pressed are one stop. Pick a narrow width to see the items leave the bar, \
           those of the lowest priority first, the end drops_first says before the other.",
          playground,
        ),
        Section::new(
          "An editor's bar",
          "The board's bar: the branch in the primary fill, sync, errors and warnings that are pressed, what is \
           being indexed with its rail; at the end the place in the file, the encoding, the language and the \
           notifications.",
          self.frame.demo(
            0,
            board,
            "StatusBar::new().leading(..).trailing(..)",
            &[
              "StatusBar::new()",
              "    .label(\"Editor status\")",
              "    .leading([",
              "        StatusItem::new(\"feat/design-system\").icon(icons::BRANCH).variant(StatusItemVariant::Primary),",
              "        StatusItem::new(\"3\").icon(icons::WARNING).tooltip(\"Warnings\").on_select(|| problems()),",
              "        StatusItem::new(\"Indexing\").progress(0.55),",
              "    ])",
              "    .trailing([\"Ln 42, Col 7\", \"UTF-8\", \"Rust\"])",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Tones",
          "An item in a tone has its muted fill and its color: the errors of a build. A bar in a tone colors every \
           item that has none of its own: debugging, offline.",
          self.frame.demo(
            1,
            tones,
            ".variant(StatusBarVariant::Warning)",
            &[
              "StatusItem::new(\"2 errors\").variant(StatusItemVariant::Destructive)",
              "StatusBar::new().variant(StatusBarVariant::Warning)",
            ],
          ),
        ),
        Section::new(
          "What does not fit",
          "The board's bar at three widths. The items of the lowest priority leave first, the trailing end before \
           the leading, each from its end; the branch and the notifications stay longest.",
          self.frame.demo(
            2,
            // The bars are as wide as they say: a card narrower than the widest scrolls them sideways.
            fit("narrow", self.frame.stage_room(), 600.0, narrow),
            "StatusItem::priority(2)",
            &[
              "StatusItem::new(\"feat/design-system\").priority(2)",
              "StatusItem::new(\"Notifications\").icon(icons::BELL).icon_only(true).priority(3)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a status bar is given, and what its items say of themselves.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
