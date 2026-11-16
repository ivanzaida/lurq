//! The page of the teaching tip: its playground, the design's tip with its
//! picture, and the primary look. A tour of them has a page of its own.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Placement, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonVariant, Icon, TeachingTip, TeachingTipVariant, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The sides the playground offers.
const PLACEMENTS: [(&str, Placement); 6] = [
  ("Bottom", Placement::Bottom),
  ("BottomStart", Placement::BottomStart),
  ("Top", Placement::Top),
  ("TopStart", Placement::TopStart),
  ("Right", Placement::Right),
  ("Left", Placement::Left),
];

/// The looks the playground offers.
const VARIANTS: [(&str, TeachingTipVariant); 2] = [
  ("Plain", TeachingTipVariant::Plain),
  ("Primary", TeachingTipVariant::Primary),
];

/// The board's picture: sparkles of 24.
fn sparkles() -> Icon {
  Icon::new(icons::SPARKLES).size(24.0)
}

/// What a tip points at: an outlined button whose root is `anchor`.
fn target(label: &str, icon: &str, anchor: &ElementRef) -> Row {
  Row::new()
    .ref_element(anchor.clone())
    .child(Button::new(label).variant(ButtonVariant::Outline).icon(icon))
}

/// What turns a tip's signal: a ghost button.
fn toggle(label: &str, open: &Signal<bool>) -> Button {
  let turn = open.clone();
  Button::new(label)
    .variant(ButtonVariant::Ghost)
    .on_click(move |_| turn.update(|open| *open = !*open))
}

pub struct TeachingTipPage {
  frame: Frame,
  playground: Playground,
  anchor: ElementRef,
  open: Signal<bool>,
  title: Signal<String>,
  description: Signal<String>,
  media: Signal<bool>,
  step: Signal<bool>,
  secondary: Signal<bool>,
  close_button: Signal<bool>,
  /// The look, as its index in [`VARIANTS`].
  variant: Signal<usize>,
  /// The placement, as its index in [`PLACEMENTS`].
  placement: Signal<usize>,
  arrow: Signal<bool>,
  takes_focus: Signal<bool>,
  /// The design's tip and the primary one: what each points at, and whether it is open.
  design: (ElementRef, Signal<bool>),
  primary: (ElementRef, Signal<bool>),
}

impl TeachingTipPage {
  /// The playground: a button and the tip pointing at it as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (media, step, secondary, close_button) = (
      self.media.get(),
      self.step.get(),
      self.secondary.get(),
      self.close_button.get(),
    );
    let (chosen_variant, chosen_placement) = (self.variant.get(), self.placement.get());
    let (variant_name, variant) = VARIANTS[chosen_variant.min(VARIANTS.len() - 1)];
    let (placement_name, placement) = PLACEMENTS[chosen_placement.min(PLACEMENTS.len() - 1)];
    let (arrow, takes_focus, open) = (self.arrow.get(), self.takes_focus.get(), self.open.get());
    let record = self.playground.recorder();
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move |anchor: ElementRef, open: Signal<bool>| {
        let (on, back, dismissed) = (record.clone(), record.clone(), record.clone());
        let mut tip = TeachingTip::new(open, anchor)
          .title(&title)
          .description(&description)
          .primary(if step { "Next" } else { "Got it" }, move || {
            on("pressed: primary".to_owned())
          })
          .close_button(close_button)
          .variant(variant)
          .placement(placement)
          .arrow(arrow)
          .takes_focus(takes_focus)
          .on_dismiss(move || dismissed("dismissed".to_owned()));
        if media {
          tip = tip.media(sparkles());
        }
        if step {
          tip = tip.step(2, 4);
        }
        if secondary {
          tip = tip.secondary("Back", move || back("pressed: secondary".to_owned()));
        }
        tip
      }
    };
    let shown = Column::new()
      .align_items(Alignment::Center)
      .spacing(12.0)
      .child(target("Panels", icons::LAYERS, &self.anchor))
      .child(toggle(if open { "Hide the tip" } else { "Show the tip" }, &self.open))
      .child(plain(self.anchor.clone(), self.open.clone()).mount(ctx));

    let mut code = vec![
      "// open: Signal<bool>, anchor: ElementRef".to_owned(),
      "TeachingTip::new(open.clone(), anchor.clone())".to_owned(),
      format!("    .title({title:?})"),
      format!("    .description({description:?})"),
    ];
    if media {
      code.push("    .media(Icon::new(icons::SPARKLES).size(24.0))".to_owned());
    }
    if step {
      code.push("    .step(2, 4)".to_owned());
    }
    if secondary {
      code.push("    .secondary(\"Back\", || back())".to_owned());
    }
    code.push(format!(
      "    .primary({:?}, || next())",
      if step { "Next" } else { "Got it" }
    ));
    if !close_button {
      code.push("    .close_button(false)".to_owned());
    }
    if variant != TeachingTipVariant::Plain {
      code.push(format!("    .variant(TeachingTipVariant::{variant_name})"));
    }
    if placement != Placement::Bottom {
      code.push(format!("    .placement(Placement::{placement_name})"));
    }
    if !arrow {
      code.push("    .arrow(false)".to_owned());
    }
    if takes_focus {
      code.push("    .takes_focus(true)".to_owned());
    }
    code.push("    .on_dismiss(|| dismissed())".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let (width, padding) = if step { (280, 16) } else { (300, 14) };
    let mut tree = vec![
      "anchor    the button it points at".to_owned(),
      format!(
        "root      4 from it, {}   {}",
        placement_name.to_lowercase(),
        if open { "open" } else { "closed" }
      ),
      format!("├─ panel  {width} wide, {variant_name}, radius lg, shadow lg   a dialog"),
    ];
    if media {
      tree.push("│  ├─ media    96 high, muted".to_owned());
    }
    tree.push(format!("│  └─ content  {padding} padding"));
    tree.push(format!(
      "│     ├─ header       title 14{}",
      if close_button { ", close 20" } else { "" }
    ));
    tree.push("│     ├─ description  13, muted".to_owned());
    tree.push(format!(
      "│     └─ footer       {}{}",
      if step { "\"2 of 4\" mono 11, " } else { "" },
      if secondary {
        "Back, then the primary"
      } else {
        "the primary"
      }
    ));
    tree.push(format!(
      "└─ arrow  {}",
      if arrow {
        "18 × 9, the panel's fill and edge"
      } else {
        "none"
      }
    ));

    let (to_title, to_description, to_media, to_step) = (
      self.title.clone(),
      self.description.clone(),
      self.media.clone(),
      self.step.clone(),
    );
    let (to_secondary, to_close, to_variant, to_placement) = (
      self.secondary.clone(),
      self.close_button.clone(),
      self.variant.clone(),
      self.placement.clone(),
    );
    let (to_arrow, to_focus, to_open) = (self.arrow.clone(), self.takes_focus.clone(), self.open.clone());
    let (set_variant, set_placement) = (self.variant.clone(), self.placement.clone());
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let placements: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Teaching Tip",
        make: Arc::new(move || plain(ElementRef::new(), Signal::new(true)).into()),
        shown: Shown::of(shown),
        fill: false,
        forceable: false,
        facts: if open { "open".to_owned() } else { "closed".to_owned() },
        event: "on_dismiss",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "variant",
            "TeachingTipVariant",
            self
              .playground
              .select(0, &variants, &chosen_variant, move |index| set_variant.set(index)),
          ),
          prop(
            "placement",
            "Placement",
            self
              .playground
              .select(1, &placements, &chosen_placement, move |index| set_placement.set(index)),
          ),
          divider(),
          flag("media", &self.media),
          flag("step 2 of 4", &self.step),
          flag("secondary", &self.secondary),
          flag("close button", &self.close_button),
          flag("arrow", &self.arrow),
          flag("takes focus", &self.takes_focus),
          flag("open", &self.open),
        ],
        changed: [
          !media,
          step,
          secondary,
          !close_button,
          variant != TeachingTipVariant::Plain,
          placement != Placement::Bottom,
          !arrow,
          takes_focus,
          !open,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_title.set("Dock panels anywhere".to_owned());
          to_description.set("Drag a panel by its header and drop it on any edge of the window.".to_owned());
          to_media.set(true);
          to_step.set(false);
          to_secondary.set(false);
          to_close.set(true);
          to_variant.set(0);
          to_placement.set(0);
          to_arrow.set(true);
          to_focus.set(false);
          to_open.set(true);
        }),
      },
    )
  }
}

impl Component for TeachingTipPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      anchor: ElementRef::new(),
      open: ctx.signal(true),
      title: ctx.signal("Dock panels anywhere".to_owned()),
      description: ctx.signal("Drag a panel by its header and drop it on any edge of the window.".to_owned()),
      media: ctx.signal(true),
      step: ctx.signal(false),
      secondary: ctx.signal(false),
      close_button: ctx.signal(true),
      variant: ctx.signal(0),
      placement: ctx.signal(0),
      arrow: ctx.signal(true),
      takes_focus: ctx.signal(false),
      design: (ElementRef::new(), ctx.signal(false)),
      primary: (ElementRef::new(), ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (anchor, open) = &self.design;
    let done = open.clone();
    let design_tip = TeachingTip::new(open.clone(), anchor.clone())
      .media(sparkles())
      .title("Dock panels anywhere")
      .description("Drag a panel by its header and drop it on any edge of the window.")
      .primary("Got it", move || done.set(false));
    let design = Row::new()
      .width(FILL)
      .padding_top(32.0)
      .padding_bottom(280.0)
      .spacing(16.0)
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .child(target("Panels", icons::LAYERS, anchor))
      .child(toggle("Show the tip", open))
      .child(design_tip.mount(ctx));

    let (anchor, open) = &self.primary;
    let done = open.clone();
    let primary_tip = TeachingTip::new(open.clone(), anchor.clone())
      .title("Search everything")
      .description("Find a file, a setting or a command from one place.")
      .secondary("Not now", {
        let open = open.clone();
        move || open.set(false)
      })
      .primary("Try it", move || done.set(false))
      .variant(TeachingTipVariant::Primary)
      .placement(Placement::Right);
    let primary = Row::new()
      .width(FILL)
      .padding_vertical(80.0)
      .padding_left(24.0)
      .spacing(16.0)
      .align_items(Alignment::Center)
      .child(target("Search", icons::SEARCH, anchor))
      .child(toggle("Show the tip", open))
      .child(primary_tip.mount(ctx));

    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it shows. The app's: its close control and Escape set it to false.",
      ),
      (
        "anchor",
        "impl Into<ElementRef>",
        "",
        "The element it points at, kept from one render to the next.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts it in the app; the element it returns has no size and goes anywhere.",
      ),
      (
        "title",
        "&str",
        "",
        "What it points at, in a few words. What tooling calls it, unless its label says otherwise.",
      ),
      ("label", "&str", "", "What tooling calls it, in place of its title."),
      ("description", "&str", "", "What it is for, under the title."),
      (
        "media",
        "impl Into<Element>",
        "",
        "A picture or an icon at its top, 96 high on the muted fill.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Something it holds, under the description. children adds several.",
      ),
      (
        "step",
        "(usize, usize)",
        "",
        "That it is one step of several: \"2 of 4\" in its footer, in the design's tour look.",
      ),
      (
        "primary",
        "(&str, impl Fn())",
        "",
        "The button that goes on, and what a press of it does.",
      ),
      ("secondary", "(&str, impl Fn())", "", "The button before it, outlined."),
      (
        "tertiary",
        "(&str, impl Fn())",
        "",
        "A quiet button before the others: Skip, Remind me later.",
      ),
      (
        "on_dismiss",
        "impl Fn()",
        "",
        "Told after its close control or Escape closed it, the signal false already. A press elsewhere does not \
         close it.",
      ),
      ("dismiss_on_escape", "bool", "true", "Whether Escape closes it."),
      (
        "dismiss_on_outside_press",
        "bool",
        "false",
        "Whether a press outside it closes it: a press elsewhere is the person going on beside it.",
      ),
      (
        "close_button",
        "bool",
        "true",
        "Whether it has a close control at the end of its title.",
      ),
      (
        "variant",
        "TeachingTipVariant",
        "Plain",
        "Plain, in the popover's colors, or Primary.",
      ),
      (
        "placement",
        "Placement",
        "Bottom",
        "The side of what it points at it is on; the other side where there is no room.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "4",
        "How far it stands from what it points at, in place of what the theme gives it.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where there is no room on its side: to the other side, then moved into the window.",
      ),
      (
        "arrow",
        "bool",
        "true",
        "A triangle with the panel's edge pointing at what it is about.",
      ),
      (
        "takes_focus",
        "bool",
        "false",
        "Whether the focus goes into it as it opens. Without, Tab goes into it from what it points at.",
      ),
      (
        "width",
        "impl Into<Length>",
        "300",
        "How wide the panel is; 280 for a step.",
      ),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws its close control hovered, pressed or focused with nothing on it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Teaching Tip"],
        title: "Teaching Tip",
        description: "A panel that points at one thing in the app and says what it is for, without taking the focus \
                      from what a person is doing. A Tour shows several, one after another: it has its own page.",
        import: Some("use lurq::ui::TeachingTip;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TeachingTip is a control on the right. Its close control and Escape close it and the open \
           switch follows; a press elsewhere on the page does not.",
          playground,
        ),
        Section::new(
          "Teaching tip",
          "The design's: a picture, a title with a close control, a line, and a button to go on. The page holds \
           whether it is open, in a signal.",
          self.frame.demo(
            0,
            design,
            "TeachingTip::new(open, anchor).media(..).title(..).description(..).primary(..)",
            &[
              "// open: Signal<bool>, anchor: ElementRef",
              "TeachingTip::new(open.clone(), anchor.clone())",
              "    .media(Icon::new(icons::SPARKLES).size(24.0))",
              "    .title(\"Dock panels anywhere\")",
              "    .description(\"Drag a panel by its header and drop it on any edge of the window.\")",
              "    .primary(\"Got it\", move || open.set(false))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Primary",
          "In the primary color, for the one thing that is to be seen first: the button that goes on is the light \
           one.",
          self.frame.demo(
            1,
            primary,
            ".variant(TeachingTipVariant::Primary)",
            &[
              "TeachingTip::new(open.clone(), anchor.clone())",
              "    .title(\"Search everything\")",
              "    .description(\"Find a file, a setting or a command from one place.\")",
              "    .secondary(\"Not now\", || open.set(false))",
              "    .primary(\"Try it\", || open.set(false))",
              "    .variant(TeachingTipVariant::Primary)",
              "    .placement(Placement::Right)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a teaching tip is given.",
          props_table(&rows, &["open", "anchor", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
