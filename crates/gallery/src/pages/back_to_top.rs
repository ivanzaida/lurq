//! The page of the back to top button: its playground over a list that
//! scrolls, the button as the board draws it, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment, layout_kind::ScrollState, text_style::FontWeight},
  node::Element,
  ui::{BackToTop, Interaction, ScrollArea, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, props_table, quiet, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// How far down the list is scrolled before the playground's button shows.
const THRESHOLDS: [(&str, f32); 3] = [("0", 0.0), ("120", 120.0), ("300", 300.0)];

/// The rows of a changelog long enough to scroll.
fn entries() -> Vec<Element> {
  (1..=24)
    .rev()
    .map(|release| {
      Column::new()
        .width(FILL)
        .padding_horizontal(16.0)
        .padding_vertical(10.0)
        .spacing(2.0)
        .child(text(&format!("0.{release}.0"), 14.0, FontWeight::Medium))
        .child(
          text(
            "Fixes, and what the batch before it left open.",
            12.0,
            FontWeight::Normal,
          )
          .color(PaletteColor::MutedForeground),
        )
        .into()
    })
    .collect()
}

/// A card of 320 by 240 whose list scrolls by `scroll`, with `button` 12 in from its bottom end corner.
fn reader(scroll: &ScrollState, button: impl Into<Element>) -> Column {
  card().width(320.0).max_width(FILL).height(240.0).child(
    Stack::new()
      .size(FILL, FILL)
      .stack_align(StackAlignment::BottomEnd)
      .child(
        ScrollArea::new()
          .scroll_state(scroll.clone())
          .label("Releases")
          .width(lurq::ui::Length::Fill)
          .height(238.0)
          .children(entries()),
      )
      .child(Column::new().padding(12.0).overflow_visible().child(button)),
  )
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .align_items(Alignment::Center)
    .child(Row::new().padding(8.0).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct BackToTopPage {
  frame: Frame,
  playground: Playground,
  /// What the playground's list scrolls by, and the demo's.
  scroll: ScrollState,
  demo: ScrollState,
  /// What the still buttons of the last section would scroll: nothing in the tree.
  still: ScrollState,
  label: Signal<String>,
  /// The index of the threshold in [`THRESHOLDS`].
  threshold: Signal<usize>,
  forced: Signal<bool>,
  disabled: Signal<bool>,
}

impl BackToTopPage {
  /// The playground: the button as the panel's props make it, over a list that scrolls.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let label = self.label.get();
    let threshold = self.threshold.get().min(THRESHOLDS.len() - 1);
    let (forced, disabled) = (self.forced.get(), self.disabled.get());
    let record = self.playground.recorder();
    let scroll = self.scroll.clone();
    let make = {
      let (label, scroll) = (label.clone(), scroll.clone());
      move || {
        let record = record.clone();
        let mut button = BackToTop::new(&scroll)
          .threshold(THRESHOLDS[threshold].1)
          .disabled(disabled)
          .on_click(move |_| record("on_click".to_owned()));
        if label != "Back to top" {
          button = button.label(&label);
        }
        if forced {
          button = button.force_shown(Some(true));
        }
        button
      }
    };

    let mut code = vec!["BackToTop::new(&scroll)".to_owned()];
    if label != "Back to top" {
      code.push(format!("    .label({label:?})"));
    }
    if threshold != 2 {
      code.push(format!("    .threshold({:.1})", THRESHOLDS[threshold].1));
    }
    if forced {
      code.push("    .force_shown(Some(true))".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_click(|_| pressed())".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!(
        "scene        {}   the list that scrolls, the button over its corner",
        self.playground.measured()
      ),
      format!(
        "host         {}",
        if forced {
          "shown wherever the list is scrolled to".to_owned()
        } else {
          format!("shows past {} of scroll", THRESHOLDS[threshold].0)
        }
      ),
      "└─ root      fades in and out".to_owned(),
      format!("   └─ button 40 × 40 px   button {label:?}"),
      "      └─ icon arrow up, 16".to_owned(),
    ];
    let set_threshold = self.threshold.clone();
    let choices: Vec<(&str, usize)> = THRESHOLDS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let controls = vec![
      prop("label", "&str", field(&self.label)),
      prop(
        "threshold",
        "impl Into<f32>",
        segmented_fill(&choices, &threshold, move |chosen| set_threshold.set(chosen)),
      ),
      divider(),
      flag("force shown", &self.forced),
      flag("disabled", &self.disabled),
    ];
    let changed = [label != "Back to top", threshold != 1, forced, disabled]
      .into_iter()
      .filter(|changed| *changed)
      .count();
    let reset = {
      let (to_label, to_threshold) = (self.label.clone(), self.threshold.clone());
      let (to_forced, to_disabled, scroll) = (self.forced.clone(), self.disabled.clone(), scroll.clone());
      Arc::new(move || {
        to_label.set("Back to top".to_owned());
        to_threshold.set(1);
        to_forced.set(false);
        to_disabled.set(false);
        scroll.set_scroll(0.0, 0.0);
      })
    };
    {
      let (make, scroll) = (make.clone(), scroll.clone());
      self
        .playground
        .window_shows(move |ctx| reader(&scroll, make().mount(ctx)).into());
    }
    let shown = {
      let (make, scroll) = (make.clone(), scroll.clone());
      // The size under the stage is the scene's: the button is gone while the list is at its top, and what is
      // gone has no size to say.
      Shown::built(move |ctx| reader(&scroll, make().mount(ctx)).into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Back To Top",
        make: {
          let scroll = scroll.clone();
          Arc::new(move || reader(&scroll, make()).into())
        },
        shown,
        fill: false,
        forceable: false,
        facts: if forced {
          "the button shown".to_owned()
        } else {
          format!("the button shows past {}", THRESHOLDS[threshold].0)
        },
        event: "on_click",
        code,
        tree,
        controls,
        changed,
        reset,
      },
    )
  }
}

impl Component for BackToTopPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      scroll: ScrollState::new(),
      demo: ScrollState::new(),
      still: ScrollState::new(),
      label: ctx.signal("Back to top".to_owned()),
      threshold: ctx.signal(1),
      forced: ctx.signal(false),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let page = reader(&self.demo, BackToTop::new(&self.demo).threshold(120.0).mount(ctx));
    let states = Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::End)
      .overflow_visible()
      .child(item(BackToTop::new(&self.still), "back to top"))
      .child(item(
        BackToTop::new(&self.still).force(Some(Interaction::Hovered)),
        "under the pointer",
      ))
      .child(item(
        BackToTop::new(&self.still).force(Some(Interaction::FocusVisible)),
        "focused from the keyboard",
      ))
      .child(item(BackToTop::new(&self.still).disabled(true), "disabled"))
      .child(item(
        BackToTop::new(&self.still).icon(icons::CHEVRON_UP),
        "another icon",
      ));
    let rows = [
      (
        "new",
        "&ScrollState",
        "",
        "The state of what it scrolls, which the app gives the scroll container too.",
      ),
      (
        "threshold",
        "impl Into<f32>",
        "300",
        "How far down the content is scrolled before the button shows.",
      ),
      (
        "label",
        "&str",
        "\"Back to top\"",
        "What the button is called to tooling.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "arrow-up",
        "The icon of the button, by its name in the theme's set.",
      ),
      (
        "on_click",
        "impl IntoMouseEventHandler",
        "",
        "Hears the press before the content is scrolled; one that prevents the event's default keeps the \
         content where it is.",
      ),
      (
        "force_shown",
        "Option<bool>",
        "None",
        "Shown wherever the content is scrolled to, or never.",
      ),
      ("disabled", "bool", "false", "The button takes no input."),
      (
        "mount",
        "&mut Ctx",
        "",
        "Puts the button where it goes: it shows and goes with the scroll. Converted into an element, it is \
         always shown and its press takes the content to its top at once.",
      ),
      (
        "style, rules, state, force",
        "",
        "",
        "Over the theme's recipe: BackToTopPart::Root (what fades in and out), Button and Icon, the last two laid \
         over a button's root and icon.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Back To Top"],
        title: "Back To Top",
        description: "A round button that shows once a page is scrolled down, and takes it back to its top.",
        import: Some("use lurq::ui::BackToTop;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Scroll the list on the stage: the button shows once it is scrolled past the threshold, and a press \
           takes the list back to its top. Every prop is a control on the right.",
          playground,
        ),
        Section::new(
          "On a page",
          "The button is given the state the page scrolls by, and the app puts it in a corner of a stack over \
           the page. It fades in past the threshold, here 120, and out as the page comes back above it. A press \
           scrolls to the top as a wheel does, and at once while motion is reduced.",
          self.frame.demo(
            0,
            page,
            "BackToTop::new(&scroll).mount(ctx)",
            &[
              "// scroll: ScrollState, kept by the page",
              "Stack::new()",
              "    .stack_align(StackAlignment::BottomEnd)",
              "    .child(ScrollArea::new().scroll_state(scroll.clone()).children(entries))",
              "    .child(",
              "        Column::new()",
              "            .padding(12.0)",
              "            .overflow_visible()",
              "            .child(BackToTop::new(&scroll).threshold(120.0).mount(ctx)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "The button",
          "A circle of 40 on the background in an edge of the border's color, with the medium shadow and an \
           arrow of 16. Under the pointer and focused it is the outlined button it is made of. These are its \
           still form, which is always shown.",
          self.frame.demo(
            1,
            states,
            "BackToTop::new(&scroll)",
            &[
              "BackToTop::new(&scroll)",
              "BackToTop::new(&scroll).force(Some(Interaction::Hovered))",
              "BackToTop::new(&scroll).force(Some(Interaction::FocusVisible))",
              "BackToTop::new(&scroll).disabled(true)",
              "BackToTop::new(&scroll).icon(icons::CHEVRON_UP)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a back to top button is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
