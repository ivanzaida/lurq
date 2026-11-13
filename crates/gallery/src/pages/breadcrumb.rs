//! The page of the breadcrumb: its playground, the board's breadcrumb, the
//! collapsed form with its menu, the separator and icons, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Breadcrumb, BreadcrumbSeparator, BreadcrumbStep, Interaction, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, one under the other.
fn items() -> Column {
  Column::new().width(FILL).spacing(24.0).align_items(Alignment::Start)
}

/// The way of the playground: a label and a route for each step, as deep as it goes.
const WAY: [(&str, &str); 6] = [
  ("Home", "/"),
  ("Projects", "/projects"),
  ("lurq", "/projects/lurq"),
  ("crates", "/projects/lurq/crates"),
  ("ui", "/projects/lurq/crates/ui"),
  ("breadcrumb.rs", "/projects/lurq/crates/ui/breadcrumb.rs"),
];

/// How many steps the playground's breadcrumb has.
const STEPS: [(&str, usize); 5] = [("2", 2), ("3", 3), ("4", 4), ("5", 5), ("6", 6)];

/// The most steps the playground's breadcrumb shows: all of them, or a number.
const MAX_ITEMS: [(&str, usize); 4] = [("None", 0), ("2", 2), ("3", 3), ("4", 4)];

/// What stands between the playground's steps.
const SEPARATORS: [(&str, usize); 2] = [("chevron", 0), ("slash", 1)];

/// The way to the board's page of a setting.
const SETTINGS: [(&str, &str); 3] = [
  ("Settings", "/settings"),
  ("Appearance", "/settings/appearance"),
  ("Theme", "/settings/appearance/theme"),
];

/// The way the collapsed demo hides the middle of.
const ACCOUNT: [(&str, &str); 5] = [
  ("Settings", "/settings"),
  ("Account", "/settings/account"),
  ("Security", "/settings/account/security"),
  ("Sessions", "/settings/account/security/sessions"),
  ("Theme", "/settings/account/security/sessions/theme"),
];

pub struct BreadcrumbPage {
  frame: Frame,
  playground: Playground,
  steps: Signal<usize>,
  max_items: Signal<usize>,
  separator: Signal<usize>,
  icon: Signal<bool>,
  disabled: Signal<bool>,
  play_anchor: ElementRef,
  play_open: Signal<bool>,
  /// The board's breadcrumb: how deep it is now, followed back by a press of a step.
  depth: Signal<usize>,
  /// The collapsed one: what was followed last, and its menu.
  followed: Signal<String>,
  collapsed_anchor: ElementRef,
  collapsed_open: Signal<bool>,
}

/// A breadcrumb of the first `count` steps of `way`, each step's value its route.
fn along(way: &[(&str, &str)], count: usize, icon: bool) -> Breadcrumb {
  way
    .iter()
    .take(count)
    .enumerate()
    .fold(Breadcrumb::new(), |breadcrumb, (index, (label, route))| {
      let mut step = BreadcrumbStep::new(label).value(route);
      if icon && index == 0 {
        step = step.icon(icons::HOME);
      }
      breadcrumb.step(step)
    })
}

impl BreadcrumbPage {
  /// The playground: the breadcrumb as the panel's props make it. Following a step goes back to it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let steps = self.steps.get().clamp(1, WAY.len());
    let (max_items, separator) = (self.max_items.get(), self.separator.get());
    let (icon, disabled) = (self.icon.get(), self.disabled.get());
    let plain = move || {
      let mut breadcrumb = along(&WAY, steps, icon).disabled(disabled);
      if max_items > 0 {
        breadcrumb = breadcrumb.max_items(max_items);
      }
      if separator == 1 {
        breadcrumb = breadcrumb.separator(BreadcrumbSeparator::slash());
      }
      breadcrumb
    };
    let (record, back) = (self.playground.recorder(), self.steps.clone());
    let shown = plain()
      .force(self.playground.forced())
      .menu(self.play_anchor.clone(), self.play_open.clone())
      .on_select(move |route| {
        record(format!("on_select({route:?})"));
        if let Some(index) = WAY.iter().position(|(_, held)| *held == route) {
          back.set(index + 1);
        }
      });

    let mut code = vec!["// anchor: ElementRef, open: Signal<bool>, for the menu of the hidden steps".to_owned()];
    code.push("Breadcrumb::new()".to_owned());
    for (index, (label, route)) in WAY.iter().take(steps).enumerate() {
      let icon = if icon && index == 0 { ".icon(icons::HOME)" } else { "" };
      code.push(format!(
        "    .step(BreadcrumbStep::new({label:?}){icon}.value({route:?}))"
      ));
    }
    if max_items > 0 {
      code.push(format!("    .max_items({max_items})"));
      code.push("    .menu(anchor, open)".to_owned());
    }
    if separator == 1 {
      code.push("    .separator(BreadcrumbSeparator::slash())".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_select(|route| go_to(route))".to_owned());

    let hidden = (max_items > 0 && steps > max_items.max(2)).then(|| steps - max_items.max(2));
    let shown_steps: Vec<usize> = match hidden {
      Some(count) => std::iter::once(0).chain(1 + count..steps).collect(),
      None => (0..steps).collect(),
    };
    let mut tree = vec![format!("root (navigation)   {}", self.playground.measured())];
    let between = if separator == 1 { "/ 14" } else { "chevron 14" };
    for (place, index) in shown_steps.iter().enumerate() {
      if place > 0 {
        tree.push(format!("├─ separator  {between}"));
      }
      let last = *index + 1 == steps;
      let what = if last { "page " } else { "link " };
      let icon = if icon && *index == 0 { "icon 14 · " } else { "" };
      tree.push(format!(
        "{} {what}      {icon}{:?}",
        if last { "└─" } else { "├─" },
        WAY[*index].0
      ));
      if place == 0
        && let Some(count) = hidden
      {
        tree.push(format!("├─ separator  {between}"));
        tree.push(format!("├─ ellipsis   24 × 24 · menu of {count}"));
      }
    }

    let (to_steps, to_max, to_separator) = (self.steps.clone(), self.max_items.clone(), self.separator.clone());
    let flags = [self.icon.clone(), self.disabled.clone()];
    let (pick_steps, pick_max, pick_separator) = (self.steps.clone(), self.max_items.clone(), self.separator.clone());
    let mut facts = vec![format!("{steps} steps")];
    if let Some(count) = hidden {
      facts.push(format!("{count} hidden"));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Breadcrumb",
        make: Arc::new(move || plain().into()),
        shown: shown.into(),
        fill: false,
        forceable: true,
        facts: facts.join(" · "),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop(
            "steps",
            "BreadcrumbStep",
            self
              .playground
              .select(0, &STEPS, &steps, move |count| pick_steps.set(count)),
          ),
          prop(
            "max_items",
            "usize",
            self
              .playground
              .select(1, &MAX_ITEMS, &max_items, move |max| pick_max.set(max)),
          ),
          prop(
            "separator",
            "BreadcrumbSeparator",
            self
              .playground
              .select(2, &SEPARATORS, &separator, move |which| pick_separator.set(which)),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("disabled", &self.disabled),
        ],
        changed: [steps != 4, max_items != 0, separator != 0, icon, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_steps.set(4);
          to_max.set(0);
          to_separator.set(0);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for BreadcrumbPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      steps: ctx.signal(4),
      max_items: ctx.signal(0),
      separator: ctx.signal(0),
      icon: ctx.signal(false),
      disabled: ctx.signal(false),
      play_anchor: ElementRef::new(),
      play_open: ctx.signal(false),
      depth: ctx.signal(SETTINGS.len()),
      followed: ctx.signal(String::new()),
      collapsed_anchor: ElementRef::new(),
      collapsed_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's breadcrumb: following a step goes back to it, and the reset puts the whole way back.
    let depth = self.depth.get().clamp(1, SETTINGS.len());
    let back = self.depth.clone();
    let board = along(&SETTINGS, depth, false).on_select(move |route| {
      if let Some(index) = SETTINGS.iter().position(|(_, held)| *held == route) {
        back.set(index + 1);
      }
    });
    let again = self.depth.clone();
    let live = items().child(item(
      Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(board)
        .child(lurq::ui::Link::new("Back to Theme").on_click(move |_| again.set(SETTINGS.len()))),
      &format!("at {}", SETTINGS[depth - 1].1),
    ));

    let told = self.followed.clone();
    let followed = self.followed.get();
    let collapsed = items().child(item(
      along(&ACCOUNT, ACCOUNT.len(), false)
        .max_items(2)
        .menu(self.collapsed_anchor.clone(), self.collapsed_open.clone())
        .on_select(move |route| told.set(route.to_owned())),
      &if followed.is_empty() {
        "the ellipsis opens the menu of the three hidden steps".to_owned()
      } else {
        format!("followed: {followed}")
      },
    ));

    let looks = items()
      .child(item(
        along(&SETTINGS, 3, false).separator(BreadcrumbSeparator::slash()),
        "a slash between the steps",
      ))
      .child(item(along(&WAY, 3, true), "an icon on the first step"));

    let states = items()
      .child(item(
        along(&SETTINGS, 3, false),
        "at rest: links muted, the page in the foreground",
      ))
      .child(item(
        along(&SETTINGS, 3, false).force(Some(Interaction::Hovered)),
        "hover: a link in the foreground",
      ))
      .child(item(
        along(&SETTINGS, 3, false).force(Some(Interaction::FocusVisible)),
        "focus-visible: the ring around a link",
      ))
      .child(item(along(&SETTINGS, 3, false).disabled(true), "disabled"));

    let rows = [
      (
        "step",
        "impl Into<BreadcrumbStep>",
        "",
        "A step, after those it has. The last is the page one is on: text, not a link. A &str is a step of that \
         label.",
      ),
      ("steps", "impl IntoIterator", "", "Steps, in order, after those it has."),
      (
        "BreadcrumbStep::value",
        "&str",
        "its label",
        "What the step stands for, told to on_select when it is followed: a route, a path.",
      ),
      (
        "BreadcrumbStep::icon",
        "Name",
        "",
        "An icon of 14 before the step's text.",
      ),
      (
        "BreadcrumbStep::on_select",
        "impl Fn()",
        "",
        "Called when that step is followed: a press, Enter or Space, or choosing it in the menu.",
      ),
      (
        "on_select",
        "impl Fn(&str)",
        "",
        "Called with the value of the step followed, after the step's own handlers.",
      ),
      (
        "max_items",
        "usize",
        "all",
        "The most steps shown. With more: the first, an ellipsis and the last max_items - 1. Never fewer than 2.",
      ),
      (
        "menu",
        "ElementRef, Signal<bool>",
        "",
        "What the menu of the hidden steps hangs from, and whether it is open: the app's. A press of the ellipsis \
         turns it.",
      ),
      (
        "separator",
        "BreadcrumbSeparator",
        "chevron",
        "What stands between two steps: an icon of the theme's set, or a text such as a slash.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; no step is followed and nothing is a stop of Tab.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Breadcrumb"],
        title: "Breadcrumb",
        description: "The way to where one is: a step for each place on it, each but the last a link back there, \
                      the last the page one is on.",
        import: Some("use lurq::ui::{Breadcrumb, BreadcrumbStep};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Breadcrumb is a control on the right. The breadcrumb on the stage is live: following a \
           step goes back to it, and with fewer steps shown than it has, the ellipsis opens the menu of the others.",
          playground,
        ),
        Section::new(
          "Breadcrumb",
          "The board's: two links and the page. Tab stops at each link, and Enter or Space follows it; the page is \
           not a stop. Following a link here goes back to it.",
          self.frame.demo(
            0,
            live,
            "Breadcrumb::new().step(..).on_select(..)",
            &[
              "Breadcrumb::new()",
              "    .step(BreadcrumbStep::new(\"Settings\").value(\"/settings\"))",
              "    .step(BreadcrumbStep::new(\"Appearance\").value(\"/settings/appearance\"))",
              "    .step(BreadcrumbStep::new(\"Theme\").value(\"/settings/appearance/theme\"))",
              "    .on_select(|route| go_to(route))",
            ],
          ),
        ),
        Section::new(
          "Collapsed",
          "With more steps than max_items, the first, an ellipsis and the last ones. The ellipsis opens a menu of \
           the steps it hides, and choosing one follows it. What the menu hangs from and whether it is open are the \
           app's, as for any menu.",
          self.frame.demo(
            1,
            collapsed,
            ".max_items(2).menu(anchor, open)",
            &[
              "// anchor: ElementRef, open: Signal<bool>, kept by the app",
              "Breadcrumb::new()",
              "    .steps([\"Settings\", \"Account\", \"Security\", \"Sessions\", \"Theme\"])",
              "    .max_items(2)",
              "    .menu(anchor.clone(), open.clone())",
              "    .on_select(|step| go_to(step))",
            ],
          ),
        ),
        Section::new(
          "Separator and icons",
          "A slash in place of the chevron, in the type of the steps; a theme can give the separator a renderer of \
           its own. A step can have an icon before its text.",
          self.frame.demo(
            2,
            looks,
            ".separator(BreadcrumbSeparator::slash())",
            &[
              "Breadcrumb::new()",
              "    .steps([\"Settings\", \"Appearance\", \"Theme\"])",
              "    .separator(BreadcrumbSeparator::slash())",
              "",
              "Breadcrumb::new()",
              "    .step(BreadcrumbStep::new(\"Home\").icon(icons::HOME))",
              "    .steps([\"Projects\", \"lurq\"])",
            ],
          ),
        ),
        Section::new(
          "States",
          "The links are in the muted foreground and turn to the foreground under the pointer; the ring of the \
           focus is a link's. A disabled breadcrumb is dimmed and follows nothing.",
          self.frame.demo(
            3,
            states,
            ".force(Some(Interaction::Hovered))",
            &[
              "let way = || Breadcrumb::new().steps([\"Settings\", \"Appearance\", \"Theme\"]);",
              "way()",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "way().force(Some(Interaction::Hovered))",
              "way().force(Some(Interaction::FocusVisible))",
              "",
              "way().disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a breadcrumb and its steps are given.",
          props_table(&rows, &["step"], self.frame.room()),
        ),
      ],
    )
  }
}
