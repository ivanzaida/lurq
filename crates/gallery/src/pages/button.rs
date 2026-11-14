//! `ui::Button`: a playground with every prop of it, then its variants,
//! sizes, states, icons and shapes, and the table of its props.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Rect, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, RadiusValue},
  ui::{Button, ButtonPart, ButtonShape, ButtonSize, ButtonState, ButtonVariant, Case, Fab, Interaction, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, line, name, props_table, quiet, segmented, segmented_fill, styled},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// A variant: what it is, what the code calls it, and what it is for.
const VARIANTS: [(ButtonVariant, &str, &str, &str); 5] = [
  (
    ButtonVariant::Primary,
    "Primary",
    "primary",
    "The main action of a view",
  ),
  (
    ButtonVariant::Secondary,
    "Secondary",
    "secondary",
    "An alternative beside the primary one",
  ),
  (
    ButtonVariant::Outline,
    "Outline",
    "outline",
    "A lesser action: dismiss, go back",
  ),
  (
    ButtonVariant::Ghost,
    "Ghost",
    "ghost",
    "Toolbars and actions in a line of text",
  ),
  (
    ButtonVariant::Destructive,
    "Destructive",
    "destructive",
    "What removes or cannot be undone",
  ),
];

/// A size as the design's recipe has it: its name in code, its name on the
/// page, its height, the padding beside its content, its label and its icon.
struct Sized {
  size: ButtonSize,
  code: &'static str,
  name: &'static str,
  height: f32,
  padding: f32,
  label: f32,
  icon: f32,
}

const SIZES: [Sized; 5] = [
  Sized {
    size: ButtonSize::Xs,
    code: "Xs",
    name: "xs",
    height: 28.0,
    padding: 10.0,
    label: 12.0,
    icon: 14.0,
  },
  Sized {
    size: ButtonSize::Sm,
    code: "Sm",
    name: "sm",
    height: 32.0,
    padding: 12.0,
    label: 13.0,
    icon: 16.0,
  },
  Sized {
    size: ButtonSize::Md,
    code: "Md",
    name: "md",
    height: 36.0,
    padding: 16.0,
    label: 13.0,
    icon: 16.0,
  },
  Sized {
    size: ButtonSize::Lg,
    code: "Lg",
    name: "lg",
    height: 40.0,
    padding: 20.0,
    label: 13.0,
    icon: 18.0,
  },
  Sized {
    size: ButtonSize::Xl,
    code: "Xl",
    name: "xl",
    height: 44.0,
    padding: 24.0,
    label: 15.0,
    icon: 20.0,
  },
];
/// The size a button has unless it says otherwise: `Md`.
const MD: usize = 2;

/// The icons the playground offers: a name for the select, the icon, and its constant in `ui::icons`.
const ICONS: [(&str, Option<&str>, &str); 6] = [
  ("None", None, ""),
  ("Plus", Some(icons::PLUS), "PLUS"),
  ("Download", Some(icons::DOWNLOAD), "DOWNLOAD"),
  ("Trash", Some(icons::TRASH), "TRASH"),
  ("Send", Some(icons::SEND), "SEND"),
  ("Settings", Some(icons::SETTINGS), "SETTINGS"),
];
/// The icon the playground starts with: a plus.
const PLUS: usize = 1;

const LABEL: &str = "Save changes";

/// What the playground's button is: every prop of it.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Props {
  variant: usize,
  size: usize,
  pill: bool,
  label: String,
  icon: usize,
  icon_only: bool,
  loading: bool,
  disabled: bool,
  full_width: bool,
}

impl Props {
  /// How many props differ from what a button has when it says nothing: its label is its own.
  fn changed(&self) -> usize {
    [
      self.variant != 0,
      self.size != MD,
      self.pill,
      self.icon != 0,
      self.icon_only,
      self.loading,
      self.disabled,
      self.full_width,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count()
  }

  /// The button these props make, with nothing listening to it yet.
  fn button(&self) -> Button {
    let mut button = Button::new(&self.label)
      .variant(VARIANTS[self.variant].0)
      .size(SIZES[self.size].size)
      .shape(if self.pill {
        ButtonShape::Pill
      } else {
        ButtonShape::Default
      })
      .icon_only(self.icon_only)
      .loading(self.loading)
      .disabled(self.disabled)
      .full_width(self.full_width);
    if let Some(name) = ICONS[self.icon].1 {
      button = button.icon(name);
    }
    button
  }

  /// The Rust that builds the button, a call to a line.
  fn code(&self) -> Vec<String> {
    let mut lines = vec![
      format!("Button::new({:?})", self.label),
      format!("    .variant(ButtonVariant::{})", VARIANTS[self.variant].1),
      format!("    .size(ButtonSize::{})", SIZES[self.size].code),
    ];
    if self.pill {
      lines.push("    .shape(ButtonShape::Pill)".to_owned());
    }
    if ICONS[self.icon].1.is_some() {
      lines.push(format!("    .icon(icons::{})", ICONS[self.icon].2));
    }
    for (on, call) in [
      (self.icon_only, "icon_only"),
      (self.loading, "loading"),
      (self.disabled, "disabled"),
      (self.full_width, "full_width"),
    ] {
      if on {
        lines.push(format!("    .{call}(true)"));
      }
    }
    lines.push("    .on_click(|_| save())".to_owned());
    lines
  }

  /// The case of the recipe these props are.
  fn case(&self) -> Case {
    Case::new()
      .variant(VARIANTS[self.variant].0)
      .size(SIZES[self.size].size)
      .state(ButtonState::Pill, self.pill)
      .state(ButtonState::IconOnly, self.icon_only)
      .state(ButtonState::Loading, self.loading)
      .state(ButtonState::Disabled, self.disabled)
      .state(ButtonState::FullWidth, self.full_width)
  }
}

pub struct ButtonPage {
  frame: Frame,
  variant: Signal<usize>,
  size: Signal<usize>,
  pill: Signal<bool>,
  label: Signal<String>,
  icon: Signal<usize>,
  icon_only: Signal<bool>,
  loading: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  playground: Playground,
  matrix_size: Signal<usize>,
}

impl ButtonPage {
  fn props(&self) -> Props {
    Props {
      variant: self.variant.get(),
      size: self.size.get(),
      pill: self.pill.get(),
      label: self.label.get(),
      icon: self.icon.get(),
      icon_only: self.icon_only.get(),
      loading: self.loading.get(),
      disabled: self.disabled.get(),
      full_width: self.full_width.get(),
    }
  }

  /// Puts every prop back to what the playground starts with.
  fn reset(&self) -> impl Fn() + Send + Sync + 'static {
    let (variant, size, pill, label, icon) = (
      self.variant.clone(),
      self.size.clone(),
      self.pill.clone(),
      self.label.clone(),
      self.icon.clone(),
    );
    let switches = [
      self.icon_only.clone(),
      self.loading.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    move || {
      variant.set(0);
      size.set(MD);
      pill.set(false);
      label.set(LABEL.to_owned());
      icon.set(PLUS);
      for switch in &switches {
        switch.set(false);
      }
    }
  }

  /// The parts the button is built of, as the recipe has them for these props.
  fn tree(&self, props: &Props) -> Vec<String> {
    let size = &SIZES[props.size];
    let padding = if props.icon_only { 0.0 } else { size.padding };
    let mut lines = vec![format!(
      "root      {}   padding 0 {padding}   gap 8",
      self.playground.measured()
    )];
    let glyph = if props.loading {
      Some("spinner")
    } else {
      ICONS[props.icon].1.map(|_| "icon   ")
    };
    let mut parts = Vec::new();
    if let Some(part) = glyph {
      parts.push(format!("{part}   {0} × {0}", size.icon));
    }
    if !props.icon_only {
      parts.push(format!("label     {:?}   {}/500", props.label, size.label));
    }
    let last = parts.len().saturating_sub(1);
    for (index, part) in parts.into_iter().enumerate() {
      lines.push(format!("{} {part}", if index == last { "└─" } else { "├─" }));
    }
    lines
  }

  /// The playground: the button these props make, and a control for each of them.
  fn playground(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let theme = ctx.theme().clone();
    let record = self.playground.recorder();
    let shown = props
      .button()
      .force(self.playground.forced())
      .on_click(move |_| record("on_click".to_owned()))
      .into();
    let radius = match theme
      .resolve::<Button>(&props.case(), None)
      .part(ButtonPart::Root)
      .rest()
      .radius
      // Every corner of a button is the same: the first says it.
      .map(|radius| radius.top_left)
    {
      Some(RadiusValue::Theme(RadiusSize::Full)) => "full".to_owned(),
      Some(RadiusValue::Theme(size)) => theme.radius_value(size).to_string(),
      Some(RadiusValue::Px(radius)) => radius.to_string(),
      None => "0".to_owned(),
    };
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, variant)| (variant.1, index))
      .collect();
    let glyphs: Vec<(&str, usize)> = ICONS
      .iter()
      .enumerate()
      .map(|(index, glyph)| (glyph.0, index))
      .collect();
    let sizes: Vec<(&str, usize)> = SIZES
      .iter()
      .enumerate()
      .map(|(index, size)| (size.name, index))
      .collect();
    let (variant, size, pill, glyph) = (
      self.variant.clone(),
      self.size.clone(),
      self.pill.clone(),
      self.icon.clone(),
    );
    let controls = vec![
      prop(
        "variant",
        "ButtonVariant",
        self
          .playground
          .select(0, &variants, &props.variant, move |index| variant.set(index)),
      ),
      prop(
        "size",
        "ButtonSize",
        segmented_fill(&sizes, &props.size, move |index| size.set(index)),
      ),
      prop(
        "shape",
        "ButtonShape",
        segmented_fill(&[("default", false), ("pill", true)], &props.pill, move |on| {
          pill.set(on)
        }),
      ),
      prop("label", "&str", field(&self.label)),
      prop(
        "icon",
        "impl Into<Name>",
        self
          .playground
          .select(1, &glyphs, &props.icon, move |index| glyph.set(index)),
      ),
      divider(),
      flag("icon_only", &self.icon_only),
      flag("loading", &self.loading),
      flag("disabled", &self.disabled),
      flag("full_width", &self.full_width),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Button",
        shown,
        make: {
          let props = props.clone();
          Arc::new(move || props.button().into())
        },
        fill: props.full_width,
        forceable: true,
        facts: format!("radius {radius}"),
        event: "on_click",
        code: props.code(),
        tree: self.tree(&props),
        controls,
        changed: props.changed(),
        reset: Arc::new(self.reset()),
      },
    )
  }

  fn variants(&self) -> Element {
    let mut stage = Row::new().width(FILL).spacing(16.0).align_items(Alignment::Start);
    for (variant, label, code, purpose) in VARIANTS {
      stage = stage.child(
        Column::new()
          .flex(1.0)
          .spacing(12.0)
          .child(Row::new().child(Button::new(label).variant(variant)))
          .child(
            Column::new().width(FILL).spacing(4.0).child(name(code, 12.0)).child(
              styled(purpose, TypographyStyle::Body, |style| {
                style.font_size = 12.0;
                style.line_height = 1.4;
              })
              .color(PaletteColor::MutedForeground)
              .width(FILL),
            ),
          ),
      );
    }
    // Five columns, each as wide as its button at least: in a narrower card they scroll sideways. The
    // stage of a card is 32 from its edges.
    let stage = fit("variants", self.frame.room() - 64.0, 720.0, stage);
    self.frame.demo(
      0,
      stage,
      ".variant(ButtonVariant::Primary)",
      &[
        "Button::new(\"Primary\")",
        "Button::new(\"Secondary\").variant(ButtonVariant::Secondary)",
        "Button::new(\"Outline\").variant(ButtonVariant::Outline)",
        "Button::new(\"Ghost\").variant(ButtonVariant::Ghost)",
        "Button::new(\"Destructive\").variant(ButtonVariant::Destructive)",
      ],
    )
  }

  fn sizes(&self) -> Element {
    let mut stage = Row::new().width(FILL).spacing(32.0).wrap().align_items(Alignment::End);
    for size in &SIZES {
      stage = stage.child(
        Column::new()
          .spacing(12.0)
          .child(Row::new().child(Button::new(&format!("Size {}", size.code)).size(size.size)))
          .child(quiet(&format!("{} · {} px", size.name, size.height), 12.0)),
      );
    }
    self.frame.demo(
      1,
      stage,
      ".size(ButtonSize::Md)",
      &[
        "Button::new(\"Size Xs\").size(ButtonSize::Xs)",
        "Button::new(\"Size Sm\").size(ButtonSize::Sm)",
        "Button::new(\"Size Md\")",
        "Button::new(\"Size Lg\").size(ButtonSize::Lg)",
        "Button::new(\"Size Xl\").size(ButtonSize::Xl)",
      ],
    )
  }

  /// Every variant in every state, at the size picked above it. The columns
  /// are live: a button in the default column answers the pointer.
  fn states(&self) -> Element {
    let sizes: Vec<(&str, usize)> = SIZES
      .iter()
      .enumerate()
      .map(|(index, size)| (size.name, index))
      .collect();
    let picked = self.matrix_size.get();
    let pick = self.matrix_size.clone();
    let columns = ["default", "hover", "pressed", "focus-visible", "disabled", "loading"];
    let mut header = Row::new()
      .width(FILL)
      .height(36.0)
      .align_items(Alignment::Center)
      .child(Rect::new(110.0, 1.0));
    for column in columns {
      header = header.child(Row::new().flex(1.0).child(quiet(column, 11.0)));
    }
    let mut matrix = Column::new()
      .width(FILL)
      .padding_top(8.0)
      .padding_horizontal(24.0)
      .padding_bottom(16.0)
      .child(header);
    for (variant, label, code, _) in VARIANTS {
      let button = || Button::new(label).variant(variant).size(SIZES[picked].size);
      let cells = [
        button(),
        button().force(Some(Interaction::Hovered)),
        button().force(Some(Interaction::Pressed)),
        button().force(Some(Interaction::FocusVisible)),
        button().disabled(true),
        button().loading(true),
      ];
      let mut row = Row::new()
        .width(FILL)
        .height(60.0)
        .align_items(Alignment::Center)
        .border_top(line())
        .child(Row::new().width(110.0).child(name(code, 12.0)));
      for cell in cells {
        row = row.child(Row::new().flex(1.0).child(cell));
      }
      matrix = matrix.child(row);
    }
    Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .foreground(PaletteColor::CardForeground)
      .border(line())
      .child(
        Row::new()
          .width(FILL)
          .height(52.0)
          .padding_horizontal(16.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_bottom(line())
          .child(
            Row::new()
              .spacing(10.0)
              .align_items(Alignment::Center)
              .child(quiet("size", 12.0))
              .child(segmented(&sizes, &picked, move |index| pick.set(index))),
          )
          .child(quiet("5 variants × 6 states", 12.0)),
      )
      // Six columns of buttons of this size need this much: in a narrower card the matrix scrolls sideways,
      // and no button is squeezed.
      .child(fit("states", self.frame.room(), MATRIX_WIDTH[picked.min(4)], matrix))
      .into()
  }

  fn icons_and_shapes(&self) -> Element {
    let item = |button: Button, caption: &str| {
      Column::new()
        .spacing(9.0)
        .child(Row::new().child(button))
        .child(quiet(caption, 12.0))
    };
    let group = |label: &str, items: Row| {
      Row::new()
        .width(FILL)
        .align_items(Alignment::Center)
        .child(Row::new().width(120.0).child(name(label, 12.0)))
        .child(items.flex(1.0))
    };
    // What does not fit beside the others goes on to the next line.
    let items = || Row::new().spacing(28.0).wrap().align_items(Alignment::End);
    let outline = |label: &str| Button::new(label).variant(ButtonVariant::Outline);

    let with_icon = items()
      .child(item(outline("New window").icon(icons::PLUS), "leading"))
      .child(item(Button::new("Next").trailing_icon(icons::ARROW_RIGHT), "trailing"))
      .child(item(
        outline("Download").icon(icons::DOWNLOAD).shape(ButtonShape::Pill),
        "pill",
      ))
      .child(item(
        Button::new("Delete")
          .variant(ButtonVariant::Destructive)
          .icon(icons::TRASH),
        "destructive",
      ));
    let mut alone = items();
    for size in &SIZES {
      alone = alone.child(item(
        outline("Settings")
          .size(size.size)
          .icon(icons::SETTINGS)
          .icon_only(true),
        &format!("{} · {}", size.name, size.height),
      ));
    }
    alone = alone
      .child(item(
        Button::new("Add")
          .variant(ButtonVariant::Secondary)
          .icon(icons::PLUS)
          .icon_only(true)
          .shape(ButtonShape::Pill),
        "round",
      ))
      .child(item(
        Button::new("More")
          .variant(ButtonVariant::Ghost)
          .icon(icons::MORE_HORIZONTAL)
          .icon_only(true),
        "ghost",
      ));
    let full = Column::new()
      .flex(1.0)
      .spacing(9.0)
      .child(Button::new("Continue").full_width(true))
      .child(quiet("full_width(true)", 12.0));
    let stage = Column::new()
      .width(FILL)
      .spacing(28.0)
      .child(group("with icon", with_icon))
      .child(group("icon only", alone))
      .child(
        Row::new()
          .width(FILL)
          .align_items(Alignment::Center)
          .child(Row::new().width(120.0).child(name("full width", 12.0)))
          .child(full),
      );
    self.frame.demo(
      2,
      stage,
      ".icon(icons::PLUS).icon_only(true).shape(ButtonShape::Pill)",
      &[
        "// With an icon.",
        "Button::new(\"New window\").variant(ButtonVariant::Outline).icon(icons::PLUS)",
        "Button::new(\"Next\").trailing_icon(icons::ARROW_RIGHT)",
        "Button::new(\"Download\")",
        "    .variant(ButtonVariant::Outline)",
        "    .icon(icons::DOWNLOAD)",
        "    .shape(ButtonShape::Pill)",
        "Button::new(\"Delete\").variant(ButtonVariant::Destructive).icon(icons::TRASH)",
        "",
        "// An icon alone, at each size: a square, still called by its label.",
        "for size in [ButtonSize::Xs, ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg, ButtonSize::Xl] {",
        "    row = row.child(",
        "        Button::new(\"Settings\")",
        "            .variant(ButtonVariant::Outline)",
        "            .size(size)",
        "            .icon(icons::SETTINGS)",
        "            .icon_only(true),",
        "    );",
        "}",
        "// In a pill it is a circle.",
        "Button::new(\"Add\")",
        "    .variant(ButtonVariant::Secondary)",
        "    .icon(icons::PLUS)",
        "    .icon_only(true)",
        "    .shape(ButtonShape::Pill)",
        "Button::new(\"More\").variant(ButtonVariant::Ghost).icon(icons::MORE_HORIZONTAL).icon_only(true)",
        "",
        "Button::new(\"Continue\").full_width(true)",
      ],
    )
  }

  /// A button with a second line, and the button that floats over a view.
  fn more(&self) -> Element {
    // A floating button is raised, and a container clips what is in it: what holds one here shows what
    // is outside it, so that its shadow has the room of the stage.
    let captioned = |shown: Element, caption: &str| {
      Column::new()
        .spacing(9.0)
        .overflow_visible()
        .child(shown)
        .child(quiet(caption, 12.0))
    };
    let raised = |fab: Fab| Element::from(Row::new().overflow_visible().child(fab));
    let stage = Row::new()
      .spacing(28.0)
      .overflow_visible()
      .align_items(Alignment::End)
      .child(captioned(
        Button::new("Schedule")
          .variant(ButtonVariant::Outline)
          .icon(icons::CALENDAR_PLUS)
          .description("Pick a date and time")
          .into(),
        "description(..)",
      ))
      .child(captioned(
        Button::new("Deploy")
          .icon(icons::SEND)
          .description("To production, at once")
          .into(),
        "on a filled button",
      ))
      .child(captioned(raised(Fab::new("Compose", icons::PLUS)), "Fab::new(..)"))
      .child(captioned(
        raised(Fab::new("Compose", icons::COMPOSE).extended(true)),
        "extended(true)",
      ));
    self.frame.demo(
      3,
      stage,
      "Button::new(\"Schedule\").description(\"Pick a date and time\")",
      &[
        "Button::new(\"Schedule\")",
        "    .variant(ButtonVariant::Outline)",
        "    .icon(icons::CALENDAR_PLUS)",
        "    .description(\"Pick a date and time\")",
        "Button::new(\"Deploy\").icon(icons::SEND).description(\"To production, at once\")",
        "",
        "// A floating button is raised, and a container clips what is in it:",
        "// what holds one shows what is outside it, for the shadow.",
        "Row::new().overflow_visible().child(Fab::new(\"Compose\", icons::PLUS))",
        "Row::new().overflow_visible().child(Fab::new(\"Compose\", icons::COMPOSE).extended(true))",
      ],
    )
  }

  fn table(&self) -> Element {
    let rows: [(&str, &str, &str, &str); 13] = [
      (
        "label",
        "&str",
        "",
        "The text in the button. A button that is an icon alone does not draw it, and is still called by it.",
      ),
      (
        "variant",
        "ButtonVariant",
        "Primary",
        "How much it stands out: Primary, Secondary, Outline, Ghost or Destructive, or a variant the theme names.",
      ),
      (
        "size",
        "ButtonSize",
        "Md",
        "Its height: Xs 28, Sm 32, Md 36, Lg 40, Xl 44. Xs and Xl have a label of their own size.",
      ),
      (
        "shape",
        "ButtonShape",
        "Default",
        "Default is the theme's radius for a control, Pill the full radius.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "none",
        "An icon of the theme's set before the label: 16, 14 at Xs, 18 at Lg and 20 at Xl.",
      ),
      (
        "trailing_icon",
        "impl Into<Name>",
        "none",
        "An icon after the label: a chevron, an arrow.",
      ),
      (
        "icon_only",
        "bool",
        "false",
        "The icon alone: a square as wide as the button is high.",
      ),
      (
        "description",
        "&str",
        "none",
        "A second line under the label: the button is as high as the two lines.",
      ),
      (
        "loading",
        "bool",
        "false",
        "A spinner in the place of the icon, three quarters of the opacity, and no press until it is over.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it leaves the order of Tab and takes no input.",
      ),
      ("full_width", "bool", "false", "As wide as what it is in."),
      (
        "on_click",
        "impl Fn(MouseEvent)",
        "",
        "Called on a click, and on Enter and Space while the button is focused.",
      ),
      (
        "style, rules, force",
        "",
        "",
        "One instance over the theme; the rules of the place it is in; a state drawn with nothing pointing at it.",
      ),
    ];
    props_table(&rows, &["label"], self.frame.room())
  }
}

/// How wide the matrix of states has to be for six columns of buttons of each size: the widest button of
/// the size, the destructive one while it loads, fits its column.
const MATRIX_WIDTH: [f32; 5] = [800.0, 840.0, 878.0, 990.0, 1080.0];

impl Component for ButtonPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      variant: ctx.signal(0),
      size: ctx.signal(MD),
      pill: ctx.signal(false),
      label: ctx.signal(LABEL.to_owned()),
      icon: ctx.signal(PLUS),
      icon_only: ctx.signal(false),
      loading: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      playground: Playground::new(ctx, 2),
      matrix_size: ctx.signal(MD),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sections = vec![
      Section::new(
        "Playground",
        "Every prop of Button is a control on the right. The button, its measured size and the Rust that builds it \
         follow each change; the state switch draws it hovered, pressed and focused with nothing pointing at it.",
        self.playground(ctx),
      ),
      Section::new(
        "Variants",
        "Five levels of emphasis. Keep one primary action in a view and let the others step back.",
        self.variants(),
      ),
      Section::new(
        "Sizes",
        "Md is the default. Sm, Md and Lg are the steps of the theme's control sizes; Xs and Xl have a label and an \
         icon of their own size.",
        self.sizes(),
      ),
      Section::new(
        "States",
        "Every variant in every state. The buttons are live, hover and press them, and the matrix follows the size \
         picked above it.",
        self.states(),
      ),
      Section::new(
        "Icons & shapes",
        "An icon is 8 before the label, in its color. A button that is an icon alone is a square at every size, and \
         any button can be a pill or fill what it is in.",
        self.icons_and_shapes(),
      ),
      Section::new(
        "Two lines, and floating",
        "A description under the label makes the button as high as the two lines, with a larger icon beside \
         them. A floating action button is a button that is an icon alone, round, larger than a control and \
         raised: the one thing a view is for.",
        self.more(),
      ),
      Section::new(
        "Props",
        "What a button is given, in the order of its builder.",
        self.table(),
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Button"],
        title: "Button",
        description: "Starts an action when it is pressed. Five variants, five sizes and the whole set of states: \
                      change any prop in the playground and the button and its code follow.",
        import: Some("use lurq::ui::Button;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      sections,
    )
  }
}
