//! The page of the spinner: its playground, the ring, the dots and the bars,
//! its sizes, a spinner beside the text that says what is being done, and the
//! overlay that lays one over a part of the page while it loads.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonVariant, LoadingOverlay, Skeleton, Spinner, SpinnerKind, SpinnerSize, SpinnerVariant},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

const KINDS: [(&str, SpinnerKind); 3] = [
  ("ring", SpinnerKind::Ring),
  ("dots", SpinnerKind::Dots),
  ("bars", SpinnerKind::Bars),
];

const SIZES: [(&str, SpinnerSize); 3] = [
  ("sm", SpinnerSize::Sm),
  ("md", SpinnerSize::Md),
  ("lg", SpinnerSize::Lg),
];

const VARIANTS: [(&str, SpinnerVariant); 3] = [
  ("current", SpinnerVariant::Current),
  ("muted", SpinnerVariant::Muted),
  ("primary", SpinnerVariant::Primary),
];

/// The names of a list with the index of each: what a control of the panel chooses between.
fn indexed<T>(list: &[(&'static str, T)]) -> Vec<(&'static str, usize)> {
  list
    .iter()
    .enumerate()
    .map(|(index, (name, _))| (*name, index))
    .collect()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// A card of the design's overlay, 220 by 120: four lines of a list that is loading.
fn activity() -> Column {
  card()
    .width(220.0)
    .height(120.0)
    .padding(16.0)
    .spacing(10.0)
    .child(text("Recent activity", 14.0, FontWeight::Medium))
    .child(
      Skeleton::line()
        .width(188.0)
        .height(10.0)
        .animation(lurq::ui::SkeletonAnimation::None),
    )
    .child(
      Skeleton::line()
        .width(120.0)
        .height(10.0)
        .animation(lurq::ui::SkeletonAnimation::None),
    )
    .child(
      Skeleton::line()
        .width(150.0)
        .height(10.0)
        .animation(lurq::ui::SkeletonAnimation::None),
    )
}

/// The props of the playground's spinner.
#[derive(Clone)]
struct Props {
  kind: SpinnerKind,
  size: SpinnerSize,
  variant: SpinnerVariant,
  label: String,
  label_shown: bool,
  overlay: bool,
  loading: bool,
}

impl Props {
  fn spinner(&self) -> Spinner {
    let mut spinner = Spinner::new().kind(self.kind).size(self.size).variant(self.variant);
    if !self.label.is_empty() {
      spinner = spinner.label(&self.label).label_hidden(!self.label_shown);
    }
    spinner
  }

  fn shown(&self) -> Element {
    if self.overlay {
      LoadingOverlay::new(self.loading, activity())
        .style("root", |s| s.radius(lurq::app::theme::RadiusSize::Lg))
        .spinner(self.spinner())
        .into()
    } else {
      self.spinner().into()
    }
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec!["Spinner::new()".to_owned()];
    if self.kind != SpinnerKind::Ring {
      code.push(format!("    .kind(SpinnerKind::{:?})", self.kind));
    }
    if self.size != SpinnerSize::Sm {
      code.push(format!("    .size(SpinnerSize::{:?})", self.size));
    }
    if self.variant != SpinnerVariant::Current {
      code.push(format!("    .variant(SpinnerVariant::{:?})", self.variant));
    }
    if !self.label.is_empty() {
      code.push(format!("    .label({:?})", self.label));
      if !self.label_shown {
        code.push("    .label_hidden(true)".to_owned());
      }
    }
    if self.overlay {
      let mut wrapped = vec![format!("LoadingOverlay::new({}, card)", self.loading)];
      wrapped.push("    .spinner(".to_owned());
      wrapped.extend(code.into_iter().map(|line| format!("        {line}")));
      wrapped.push("    )".to_owned());
      return wrapped;
    }
    code
  }

  fn tree(&self, measured: String) -> Vec<String> {
    let indicator = match self.kind {
      SpinnerKind::Ring => "indicator  the ring, turning".to_owned(),
      SpinnerKind::Dots => "indicator  └─ dot × 3, fading in turn".to_owned(),
      SpinnerKind::Bars => "indicator  └─ bar × 5, shrinking in turn".to_owned(),
    };
    let label = !self.label.is_empty() && self.label_shown;
    let mut tree = vec![
      format!(
        "root       {measured}   status {:?}",
        if self.label.is_empty() { "Loading" } else { &self.label }
      ),
      format!("{} {indicator}", if label { "├─" } else { "└─" }),
    ];
    if label {
      tree.push(format!("└─ label    {:?}", self.label));
    }
    if self.overlay {
      tree.insert(0, "overlay    content, and the veil over it".to_owned());
    }
    tree
  }
}

pub struct SpinnerPage {
  frame: Frame,
  playground: Playground,
  /// The indices of the kind, the size and the variant in their lists.
  kind: Signal<usize>,
  size: Signal<usize>,
  variant: Signal<usize>,
  label: Signal<String>,
  label_shown: Signal<bool>,
  overlay: Signal<bool>,
  loading: Signal<bool>,
  refreshing: Signal<bool>,
}

impl SpinnerPage {
  fn props(&self) -> Props {
    Props {
      kind: KINDS[self.kind.get()].1,
      size: SIZES[self.size.get()].1,
      variant: VARIANTS[self.variant.get()].1,
      label: self.label.get(),
      label_shown: self.label_shown.get(),
      overlay: self.overlay.get(),
      loading: self.loading.get(),
    }
  }

  /// The playground: the spinner as the panel's props make it, alone or in an overlay.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let (kind, size, variant) = (self.kind.clone(), self.size.clone(), self.variant.clone());
    let controls = vec![
      prop(
        "kind",
        "SpinnerKind",
        segmented_fill(&indexed(&KINDS), &self.kind.get(), move |chosen| kind.set(chosen)),
      ),
      prop(
        "size",
        "SpinnerSize",
        segmented_fill(&indexed(&SIZES), &self.size.get(), move |chosen| size.set(chosen)),
      ),
      prop(
        "variant",
        "SpinnerVariant",
        segmented_fill(&indexed(&VARIANTS), &self.variant.get(), move |chosen| {
          variant.set(chosen)
        }),
      ),
      prop("label", "&str", field(&self.label)),
      divider(),
      flag("label shown", &self.label_shown),
      flag("in a LoadingOverlay", &self.overlay),
      flag("loading", &self.loading),
    ];
    let changed = [
      props.kind != SpinnerKind::Ring,
      props.size != SpinnerSize::Sm,
      props.variant != SpinnerVariant::Current,
      !props.label.is_empty(),
      !props.label_shown,
      props.overlay,
      !props.loading,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let reset = {
      let (kind, size, variant) = (self.kind.clone(), self.size.clone(), self.variant.clone());
      let (label, label_shown) = (self.label.clone(), self.label_shown.clone());
      let (overlay, loading) = (self.overlay.clone(), self.loading.clone());
      Arc::new(move || {
        kind.set(0);
        size.set(0);
        variant.set(0);
        label.set(String::new());
        label_shown.set(true);
        overlay.set(false);
        loading.set(true);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Spinner",
        make: {
          let props = props.clone();
          Arc::new(move || props.shown())
        },
        shown: props.shown(),
        fill: false,
        forceable: false,
        facts: if props.overlay && !props.loading {
          "veil gone".to_owned()
        } else {
          "turning".to_owned()
        },
        event: "nothing",
        code: props.code(),
        tree: props.tree(self.playground.measured()),
        controls,
        changed,
        reset,
      },
    )
  }
}

impl Component for SpinnerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      kind: ctx.signal(0),
      size: ctx.signal(0),
      variant: ctx.signal(0),
      label: ctx.signal(String::new()),
      label_shown: ctx.signal(true),
      overlay: ctx.signal(false),
      loading: ctx.signal(true),
      refreshing: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let kinds = items()
      .child(item(Spinner::new(), "ring"))
      .child(item(Spinner::new().kind(SpinnerKind::Dots), "dots"))
      .child(item(Spinner::new().kind(SpinnerKind::Bars), "bars"));
    let sizes = items()
      .child(item(Spinner::new(), "sm · 16"))
      .child(item(Spinner::new().size(SpinnerSize::Md), "md · 24"))
      .child(item(Spinner::new().size(SpinnerSize::Lg), "lg · 32"));
    let colors = items()
      .child(item(
        Row::new().foreground(PaletteColor::Destructive).child(Spinner::new()),
        "the text's color around it",
      ))
      .child(item(
        Row::new()
          .foreground(PaletteColor::Destructive)
          .spacing(16.0)
          .align_items(Alignment::Center)
          .child(Spinner::new().kind(SpinnerKind::Dots))
          .child(Spinner::new().kind(SpinnerKind::Bars)),
        "dots and bars too",
      ))
      .child(item(
        Button::new("Loading").leading(Spinner::new().kind(SpinnerKind::Dots).decorative(true)),
        "in a button: its label's color",
      ))
      .child(item(Spinner::new().variant(SpinnerVariant::Muted), "muted"))
      .child(item(Spinner::new().variant(SpinnerVariant::Primary), "primary"));
    let inline = items()
      .child(item(Spinner::new().label("Loading results…"), "inline"))
      .child(item(Button::new("Saving…").loading(true), "button loading"))
      .child(item(
        Button::new("Loading").variant(ButtonVariant::Outline).loading(true),
        "button loading outline",
      ));
    let refreshing = self.refreshing.clone();
    let overlay = Column::new()
      .spacing(16.0)
      .child(
        LoadingOverlay::new(self.refreshing.get(), activity())
          .style("root", |s| s.radius(lurq::app::theme::RadiusSize::Lg)),
      )
      .child(
        Row::new().child(
          Button::new(if self.refreshing.get() {
            "Stop loading"
          } else {
            "Load again"
          })
          .variant(ButtonVariant::Outline)
          .on_click(move |_| refreshing.update(|on| *on = !*on)),
        ),
      );
    let rows = [
      (
        "kind",
        "SpinnerKind",
        "Ring",
        "A ring that turns, three dots that fade in turn, or five bars that shrink in turn.",
      ),
      (
        "size",
        "SpinnerSize",
        "Sm",
        "16, 24 or 32 for the ring; the dots and the bars grow by a half and by twice.",
      ),
      (
        "variant",
        "SpinnerVariant",
        "Current",
        "The color of the text around it, the muted foreground, or the primary color.",
      ),
      (
        "label",
        "&str",
        "",
        "What is being done, drawn 8 beside it, and what it is called. Without one it is called \"Loading\".",
      ),
      (
        "label_hidden",
        "bool",
        "false",
        "The label is its name and is not drawn.",
      ),
      (
        "decorative",
        "bool",
        "false",
        "No status of its own, where what it is in says already what is under way: a button that loads.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Spinner"],
        title: "Spinner",
        description: "Something is being done, and how far it is is not known: a ring that turns, dots or bars. \
                      A LoadingOverlay lays one over a part of the page while that part loads.",
        import: Some("use lurq::ui::{LoadingOverlay, Spinner};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Spinner is a control on the right, and the spinner can be put in a LoadingOverlay over a \
           card that is loading.",
          playground,
        ),
        Section::new(
          "Kinds",
          "The ring is the spinner of the design and of a button that loads. Dots and bars say the same in \
           another form. Each is a status to tooling, called \"Loading\" unless it has a label.",
          self.frame.demo(
            0,
            kinds,
            ".kind(SpinnerKind::Dots)",
            &[
              "Spinner::new()",
              "Spinner::new().kind(SpinnerKind::Dots)",
              "Spinner::new().kind(SpinnerKind::Bars)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "16 beside a line of text, 24 in a panel, 32 over a page.",
          self.frame.demo(
            1,
            sizes,
            ".size(SpinnerSize::Md)",
            &[
              "Spinner::new()",
              "Spinner::new().size(SpinnerSize::Md)",
              "Spinner::new().size(SpinnerSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Colors",
          "The ring, the dots and the bars are drawn in the color of the text around them, so a spinner in a \
           button is the color of its label. A variant gives it a color of its own.",
          self.frame.demo(
            2,
            colors,
            ".variant(SpinnerVariant::Muted)",
            &[
              "Row::new().foreground(PaletteColor::Destructive).child(Spinner::new())",
              "Row::new()",
              "    .foreground(PaletteColor::Destructive)",
              "    .child(Spinner::new().kind(SpinnerKind::Dots))",
              "    .child(Spinner::new().kind(SpinnerKind::Bars))",
              "Button::new(\"Loading\").leading(Spinner::new().kind(SpinnerKind::Dots).decorative(true))",
              "Spinner::new().variant(SpinnerVariant::Muted)",
              "Spinner::new().variant(SpinnerVariant::Primary)",
            ],
          ),
        ),
        Section::new(
          "Beside a text",
          "A label says what is being done, 8 beside the spinner. A button that is loading turns the same spinner \
           in the place of its icon.",
          self.frame.demo(
            3,
            inline,
            ".label(\"Loading results…\")",
            &[
              "Spinner::new().label(\"Loading results…\")",
              "Button::new(\"Saving…\").loading(true)",
              "Button::new(\"Loading\").variant(ButtonVariant::Outline).loading(true)",
            ],
          ),
        ),
        Section::new(
          "Loading overlay",
          "A veil with a spinner over a part of the page while it loads. Meanwhile what is under it takes no press \
           and Tab passes it by; the veil fades in and out.",
          self.frame.demo(
            4,
            overlay,
            "LoadingOverlay::new(loading.get(), card)",
            &[
              "// loading: Signal<bool>",
              "LoadingOverlay::new(loading.get(), card)",
              "    .style(LoadingOverlayPart::Root, |s| s.radius(RadiusSize::Lg))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a spinner is given. A LoadingOverlay is given whether it is loading, its content, and a spinner of \
           its own if the plain one will not do.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
