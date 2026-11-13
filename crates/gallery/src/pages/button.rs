//! `ui::Button`: a playground with every prop of it, then its variants,
//! sizes, states, icons and shapes, and the table of its props.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, Breakpoint, PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Rect, Row, Text, TextInput},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, RadiusValue, border::Border},
  ui::{Button, ButtonPart, ButtonShape, ButtonSize, ButtonState, ButtonVariant, Case, Interaction, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{
    FILL, badge, card_footer, code_block, icon, mono, segmented, segmented_fill, select, styled, switch, text, tool,
  },
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

const FORCED: [(&str, Option<Interaction>); 4] = [
  ("Default", None),
  ("Hover", Some(Interaction::Hovered)),
  ("Pressed", Some(Interaction::Pressed)),
  ("Focus", Some(Interaction::FocusVisible)),
];

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

/// What the stage of the playground is given: the button's props, how it is
/// forced, and what the page keeps of it.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct StageProps {
  props: Props,
  forced: usize,
  measured: String,
  clicks: u32,
  #[devtools_ignore]
  press: Arc<dyn Fn() + Send + Sync>,
  #[devtools_ignore]
  anchor: ElementRef,
}

impl PartialEq for StageProps {
  fn eq(&self, other: &Self) -> bool {
    self.props == other.props
      && self.forced == other.forced
      && self.measured == other.measured
      && self.clicks == other.clicks
      && Arc::ptr_eq(&self.press, &other.press)
  }
}

/// The stage of the playground: the button on the canvas, with its measured
/// size and how often it was pressed under it.
struct Stage;

impl Component for Stage {
  type Props = StageProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let given = ctx.props::<StageProps>().clone();
    let theme = ctx.theme().clone();
    let press = given.press.clone();
    let button = given
      .props
      .button()
      .force(FORCED[given.forced].1)
      .on_click(move |_| press());
    let mut held = Row::new().ref_element(given.anchor.clone()).child(button);
    if given.props.full_width {
      held = held.width(FILL);
    }
    let radius = match theme
      .resolve::<Button>(&given.props.case(), None)
      .part(ButtonPart::Root)
      .rest()
      .radius
    {
      Some(RadiusValue::Theme(RadiusSize::Full)) => "full".to_owned(),
      Some(RadiusValue::Theme(size)) => theme.radius_value(size).to_string(),
      Some(RadiusValue::Px(radius)) => radius.to_string(),
      None => "0".to_owned(),
    };
    Column::new()
      .size(FILL, FILL)
      .background(PaletteColor::Canvas)
      .foreground(PaletteColor::Foreground)
      .child(
        Row::new()
          .width(FILL)
          .flex(1.0)
          .padding_horizontal(32.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .child(held),
      )
      .child(
        Row::new()
          .width(FILL)
          .height(36.0)
          .padding_horizontal(16.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .foreground(PaletteColor::MutedForeground)
          .child(mono(&format!("{} · radius {radius}", given.measured), 12.0))
          .child(
            Row::new()
              .spacing(6.0)
              .align_items(Alignment::Center)
              .child(icon("hand-pointer", 14.0))
              .child(mono(&format!("on_click × {}", given.clicks), 12.0)),
          ),
      )
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
  forced: Signal<usize>,
  clicks: Signal<u32>,
  /// What the button did, the latest first, a line each.
  events: Signal<String>,
  tab: Signal<usize>,
  /// What holds the playground's button, measured.
  anchor: ElementRef,
  measured: Signal<String>,
  matrix_size: Signal<usize>,
  variant_anchor: ElementRef,
  variant_open: Signal<bool>,
  icon_anchor: ElementRef,
  icon_open: Signal<bool>,
}

/// A line under a row or beside a column.
fn line() -> Border {
  Border::inside(BorderSize::Sm, PaletteColor::Border)
}

/// A label in the family of code, quiet: the name of a control, a caption.
fn quiet(content: &str, size: f32) -> Text {
  mono(content, size).color(PaletteColor::MutedForeground)
}

/// A name as the code has it, in the weight of a name.
fn name(content: &str, size: f32) -> Text {
  let theme = lurq::ui::current_theme();
  let mut style = theme.typography_style(TypographyStyle::Mono);
  style.font_size = theme.scaled(size);
  style.weight = FontWeight::Medium;
  Text::styled(content, style)
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

  /// The bar over the stage: the state the button is drawn in.
  fn toolbar(&self) -> Row {
    let forced = self.forced.clone();
    Row::new()
      .width(FILL)
      .height(52.0)
      .padding_left(16.0)
      .padding_right(12.0)
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .border_bottom(line())
      .child(
        Row::new()
          .spacing(10.0)
          .align_items(Alignment::Center)
          .child(quiet("state", 12.0))
          .child(segmented(&FORCED, &FORCED[self.forced.get()].1, move |interaction| {
            forced.set(FORCED.iter().position(|(_, held)| *held == interaction).unwrap_or(0))
          })),
      )
  }

  /// The stage, with what a press of its button does.
  fn stage(&self, ctx: &mut Ctx, props: &Props) -> Element {
    let (clicks, events) = (self.clicks.clone(), self.events.clone());
    let press: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
      clicks.update(|count| *count += 1);
      let count = clicks.get();
      events.update(|log| {
        let earlier: Vec<&str> = log.lines().take(5).collect();
        *log = std::iter::once(format!("on_click  #{count}").as_str())
          .chain(earlier)
          .collect::<Vec<_>>()
          .join("\n");
      });
    });
    let given = StageProps {
      props: props.clone(),
      forced: self.forced.get(),
      measured: self.measured.get(),
      clicks: self.clicks.get(),
      press,
      anchor: self.anchor.clone(),
    };
    ctx.mount::<Stage>(given)
  }

  /// The tabs under the stage, and what the chosen one shows.
  fn output(&self, props: &Props) -> Column {
    let code = props.code();
    let events = self.events.get();
    let tree = self.tree(props);
    let shown = self.tab.get();
    let mut tabs = Row::new().height(FILL).spacing(20.0);
    for (index, title) in ["Code", "Events", "Layout tree"].into_iter().enumerate() {
      let on = index == shown;
      let tab = self.tab.clone();
      let mut entry = Row::new()
        .height(FILL)
        .spacing(6.0)
        .align_items(Alignment::Center)
        .cursor(lurq::node::CursorIcon::Pointer)
        .foreground(if on {
          PaletteColor::Foreground
        } else {
          PaletteColor::MutedForeground
        })
        .on_click(move |_| tab.set(index))
        .child(text(title, 13.0, FontWeight::Medium));
      if on {
        entry = entry.border_bottom(Border::inside(2.0, PaletteColor::Primary));
      } else {
        entry = entry.hovered(|style| style.foreground(PaletteColor::Foreground));
      }
      if index == 1 && self.clicks.get() > 0 {
        entry = entry.child(
          badge(&self.clicks.get().to_string())
            .height(18.0)
            .padding_horizontal(6.0),
        );
      }
      tabs = tabs.child(entry);
    }
    let copied = match shown {
      0 => code.join("\n"),
      1 => events.clone(),
      _ => tree.join("\n"),
    };
    let body: Element = match shown {
      0 => code_block(&code).into(),
      1 if events.is_empty() => plain_lines(&["No event yet: press the button on the stage.".to_owned()]).into(),
      1 => plain_lines(&events.lines().map(str::to_owned).collect::<Vec<_>>()).into(),
      _ => plain_lines(&tree).into(),
    };
    Column::new()
      .width(FILL)
      .child(
        Row::new()
          .width(FILL)
          .height(40.0)
          .padding_left(16.0)
          .padding_right(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_top(line())
          .border_bottom(line())
          .child(tabs)
          .child(tool("Copy", icons::COPY, move || {
            lurq::clipboard::copy_to_clipboard(&copied);
          })),
      )
      .child(Column::new().width(FILL).min_height(150.0).child(body))
  }

  /// The parts the button is built of, as the recipe has them for these props.
  fn tree(&self, props: &Props) -> Vec<String> {
    let size = &SIZES[props.size];
    let padding = if props.icon_only { 0.0 } else { size.padding };
    let mut lines = vec![format!(
      "root      {}   padding 0 {padding}   gap 8",
      self.measured.get()
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

  /// The panel beside the stage: a control for each prop.
  fn panel(&self, props: &Props) -> Column {
    let prop = |label: &str, kind: &str, control: Element| {
      Column::new()
        .width(FILL)
        .spacing(6.0)
        .child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .justify(Justify::SpaceBetween)
            .child(name(label, 12.0))
            .child(quiet(kind, 11.0)),
        )
        .child(control)
    };
    let flag = |label: &str, value: &Signal<bool>| {
      let set = value.clone();
      Row::new()
        .width(FILL)
        .height(24.0)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .child(
          Row::new()
            .spacing(8.0)
            .align_items(Alignment::Center)
            .child(name(label, 12.0))
            .child(quiet("bool", 11.0)),
        )
        .child(switch(value.get(), move |on| set.set(on)))
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
    let theme = lurq::ui::current_theme();
    let body = theme.typography_style(TypographyStyle::Body);
    let (variant, size, pill, glyph) = (
      self.variant.clone(),
      self.size.clone(),
      self.pill.clone(),
      self.icon.clone(),
    );
    let changed = props.changed();
    Column::new()
      .child(
        Row::new()
          .width(FILL)
          .height(52.0)
          .padding_left(16.0)
          .padding_right(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_bottom(line())
          .child(
            Row::new()
              .spacing(8.0)
              .align_items(Alignment::Center)
              .child(text("Props", 14.0, FontWeight::SemiBold))
              .child(quiet(&format!("{changed} changed"), 11.0)),
          )
          .child(tool("Reset", icons::REFRESH, self.reset())),
      )
      .child(
        Column::new()
          .width(FILL)
          .padding(16.0)
          .spacing(16.0)
          .child(prop(
            "variant",
            "ButtonVariant",
            select(
              &self.variant_anchor,
              &self.variant_open,
              &variants,
              &props.variant,
              move |index| variant.set(index),
            )
            .width(FILL)
            .into(),
          ))
          .child(prop(
            "size",
            "ButtonSize",
            segmented_fill(&sizes, &props.size, move |index| size.set(index)).into(),
          ))
          .child(prop(
            "shape",
            "ButtonShape",
            segmented_fill(&[("default", false), ("pill", true)], &props.pill, move |on| {
              pill.set(on)
            })
            .into(),
          ))
          .child(prop(
            "label",
            "&str",
            Row::new()
              .width(FILL)
              .height(36.0)
              .padding_horizontal(12.0)
              .align_items(Alignment::Center)
              .rounded(RadiusSize::Md)
              .background(PaletteColor::Background)
              .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
              .child(
                TextInput::styled(self.label.clone(), body)
                  .flex(1.0)
                  .height(FILL)
                  .tab_index(0),
              )
              .into(),
          ))
          .child(prop(
            "icon",
            "impl Into<Name>",
            select(&self.icon_anchor, &self.icon_open, &glyphs, &props.icon, move |index| {
              glyph.set(index)
            })
            .width(FILL)
            .into(),
          ))
          .child(Rect::new(FILL, 1.0).background(PaletteColor::Border))
          .child(flag("icon_only", &self.icon_only))
          .child(flag("loading", &self.loading))
          .child(flag("disabled", &self.disabled))
          .child(flag("full_width", &self.full_width)),
      )
  }

  fn playground(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let preview = Column::new()
      .child(self.toolbar())
      .child(Column::new().width(FILL).height(418.0).child(self.stage(ctx, &props)))
      .child(self.output(&props));
    let panel = self.panel(&props);
    let card = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .foreground(PaletteColor::CardForeground)
      .border(line());
    if wide {
      card
        .child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Stretch)
            .child(preview.flex(1.0))
            .child(panel.width(320.0).border_left(line())),
        )
        .into()
    } else {
      card
        .child(preview.width(FILL))
        .child(panel.width(FILL).border_top(line()))
        .into()
    }
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
    demo(stage, ".variant(ButtonVariant::Primary)")
  }

  fn sizes(&self) -> Element {
    let mut stage = Row::new().spacing(32.0).align_items(Alignment::End);
    for size in &SIZES {
      stage = stage.child(
        Column::new()
          .spacing(12.0)
          .child(Row::new().child(Button::new(&format!("Size {}", size.code)).size(size.size)))
          .child(quiet(&format!("{} · {} px", size.name, size.height), 12.0)),
      );
    }
    demo(stage, ".size(ButtonSize::Md)")
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
      .child(matrix)
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
    let items = || Row::new().spacing(28.0).align_items(Alignment::End);
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
    demo(stage, ".icon(icons::PLUS).icon_only(true).shape(ButtonShape::Pill)")
  }

  fn table(&self) -> Element {
    let rows: [(&str, &str, &str, &str); 12] = [
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
    let head = |title: &str| text(title, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground);
    let mut table = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .foreground(PaletteColor::CardForeground)
      .border(line())
      .child(
        Row::new()
          .width(FILL)
          .height(40.0)
          .padding_horizontal(16.0)
          .align_items(Alignment::Center)
          .background(PaletteColor::Muted)
          .child(Row::new().width(150.0).child(head("Prop")))
          .child(Row::new().width(180.0).child(head("Type")))
          .child(Row::new().width(110.0).child(head("Default")))
          .child(Row::new().flex(1.0).child(head("Description"))),
      );
    for (prop, kind, default, about) in rows {
      let mut named = Row::new()
        .width(150.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(name(prop, 13.0));
      if prop == "label" {
        named = named.child(
          Row::new()
            .height(18.0)
            .padding_horizontal(6.0)
            .align_items(Alignment::Center)
            .rounded(RadiusSize::Full)
            .background(PaletteColor::WarningMuted)
            .foreground(PaletteColor::Warning)
            .child(text("required", 11.0, FontWeight::Medium)),
        );
      }
      let mut typed = Row::new().width(180.0);
      if !kind.is_empty() {
        typed = typed.child(
          Row::new()
            .padding_horizontal(5.0)
            .padding_vertical(1.0)
            .rounded(RadiusSize::Sm)
            .background(PaletteColor::Muted)
            .child(mono(kind, 12.0)),
        );
      }
      table = table.child(
        Row::new()
          .width(FILL)
          .padding_horizontal(16.0)
          .padding_vertical(12.0)
          .align_items(Alignment::Center)
          .border_top(line())
          .child(named)
          .child(typed)
          .child(Row::new().width(110.0).child(if default.is_empty() {
            quiet("—", 13.0)
          } else {
            mono(default, 13.0)
          }))
          .child(
            styled(about, TypographyStyle::Body, |style| {
              style.font_size = 13.0;
              style.line_height = 1.5;
            })
            .color(PaletteColor::MutedForeground)
            .flex(1.0),
          ),
      );
    }
    table.into()
  }
}

/// Lines of plain text in the family of code: a log, a tree.
fn plain_lines(lines: &[String]) -> Column {
  let mut block = Column::new()
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_top(16.0)
    .padding_bottom(18.0)
    .spacing(4.0);
  for line in lines {
    block = block.child(mono(line, 13.0).height(20.0));
  }
  block
}

/// A demo card: the live buttons on the canvas, and under them the call that makes one.
fn demo(stage: impl Into<Element>, call: &str) -> Element {
  Column::new()
    .width(FILL)
    .clip()
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(line())
    .child(Column::new().width(FILL).padding(32.0).child(stage))
    .child(card_footer(call))
    .into()
}

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
      forced: ctx.signal(0),
      clicks: ctx.signal(0),
      events: ctx.signal(String::new()),
      tab: ctx.signal(0),
      anchor: ElementRef::new(),
      measured: ctx.signal("0 × 0 px".to_owned()),
      matrix_size: ctx.signal(MD),
      variant_anchor: ElementRef::new(),
      variant_open: ctx.signal(false),
      icon_anchor: ElementRef::new(),
      icon_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The button says how large it is whenever that changes.
    let measured = self.measured.clone();
    self.anchor.observe_rect(move |rect| {
      let size = format!("{} × {} px", rect.width.round(), rect.height.round());
      if measured.get() != size {
        measured.set(size);
      }
    });
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
