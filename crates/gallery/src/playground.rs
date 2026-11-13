//! The playground of a component's page: the component on a stage, a control
//! for each of its props beside it, and under it the Rust that builds it,
//! what it did and what it is built of.
//!
//! A page keeps a [`Playground`] and says, each time it renders, what is on
//! the stage and what the panel has ([`Spec`]). The rest is the same on every
//! page: the switch that draws the component in a state with nothing pointing
//! at it, its measured size, the count of what it did, the tabs and Copy.

use std::sync::Arc;

use lurq::{
  app::{
    ctx::Ctx,
    theme::{BorderSize, Breakpoint, PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Rect, Row, TextInput},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
  ui::{Interaction, icons},
};

use crate::kit::{FILL, badge, code_block, icon, line, mono, name, quiet, segmented, select, switch, text, tool};

/// The states a component is drawn in with nothing pointing at it.
const FORCED: [(&str, Option<Interaction>); 4] = [
  ("Default", None),
  ("Hover", Some(Interaction::Hovered)),
  ("Pressed", Some(Interaction::Pressed)),
  ("Focus", Some(Interaction::FocusVisible)),
];

/// How many of the latest things a component did are kept.
const KEPT: usize = 6;

/// What a page keeps between renders for its playground.
pub struct Playground {
  forced: Signal<usize>,
  tab: Signal<usize>,
  count: Signal<u32>,
  /// What the component did, the latest first, a line each.
  events: Signal<String>,
  /// What holds the component on the stage, measured.
  anchor: ElementRef,
  measured: Signal<String>,
  /// The selects of the panel: what each hangs its list from, and whether it is open.
  selects: Vec<(ElementRef, Signal<bool>)>,
}

/// What a page says of its playground when it renders.
pub struct Spec {
  /// The component, as its props make it.
  pub shown: Element,
  /// Whether the component is as wide as the stage has room for.
  pub fill: bool,
  /// Whether the component can be drawn in a state with nothing pointing at it: the switch over the stage.
  pub forceable: bool,
  /// What is said after the measured size: "radius 6".
  pub facts: String,
  /// What the component does that is counted: "on_click".
  pub event: &'static str,
  /// The Rust that builds the component, a call to a line.
  pub code: Vec<String>,
  /// The parts the component is built of.
  pub tree: Vec<String>,
  /// The controls of the panel, one for each prop: [`prop`], [`flag`], [`divider`].
  pub controls: Vec<Element>,
  /// How many props are not what the component has when it says nothing.
  pub changed: usize,
  /// Puts every prop back to what the playground starts with.
  pub reset: Arc<dyn Fn() + Send + Sync>,
}

impl Playground {
  /// A playground whose panel has `selects` selects: call it where the page is created.
  pub fn new(ctx: &mut Ctx, selects: usize) -> Self {
    Self {
      forced: ctx.signal(0),
      tab: ctx.signal(0),
      count: ctx.signal(0),
      events: ctx.signal(String::new()),
      anchor: ElementRef::new(),
      measured: ctx.signal("0 × 0 px".to_owned()),
      selects: (0..selects).map(|_| (ElementRef::new(), ctx.signal(false))).collect(),
    }
  }

  /// The state the switch over the stage asks for: what the page gives its component's `force`.
  pub fn forced(&self) -> Option<Interaction> {
    FORCED[self.forced.get().min(FORCED.len() - 1)].1
  }

  /// The size of the component on the stage as it was last laid out: "144 × 36 px".
  pub fn measured(&self) -> String {
    self.measured.get()
  }

  /// What a handler of the component calls to say what it did: the line is
  /// counted, and goes to the top of the Events tab.
  pub fn recorder(&self) -> impl Fn(String) + Clone + Send + Sync + 'static {
    let (count, events) = (self.count.clone(), self.events.clone());
    move |what: String| {
      count.update(|count| *count += 1);
      let number = count.get();
      events.update(|log| {
        let earlier: Vec<&str> = log.lines().take(KEPT - 1).collect();
        *log = std::iter::once(format!("{what}  #{number}").as_str())
          .chain(earlier)
          .collect::<Vec<_>>()
          .join("\n");
      });
    }
  }

  /// A select of the panel, the `index`th of those the playground was made with.
  pub fn select<T: Clone + PartialEq + Send + Sync + 'static>(
    &self,
    index: usize,
    options: &[(&str, T)],
    chosen: &T,
    choose: impl Fn(T) + Send + Sync + 'static,
  ) -> Element {
    let (anchor, open) = &self.selects[index];
    select(anchor, open, options, chosen, choose).width(FILL).into()
  }

  /// The bar over the stage: the state the component is drawn in.
  fn toolbar(&self, forceable: bool) -> Row {
    let mut bar = Row::new()
      .width(FILL)
      .height(52.0)
      .padding_left(16.0)
      .padding_right(12.0)
      .align_items(Alignment::Center)
      .border_bottom(line());
    if forceable {
      let forced = self.forced.clone();
      bar = bar.child(
        Row::new()
          .spacing(10.0)
          .align_items(Alignment::Center)
          .child(quiet("state", 12.0))
          .child(segmented(&FORCED, &self.forced(), move |interaction| {
            forced.set(FORCED.iter().position(|(_, held)| *held == interaction).unwrap_or(0))
          })),
      );
    } else {
      bar = bar.child(quiet(
        "live: what is on the stage answers the pointer and the keyboard",
        12.0,
      ));
    }
    bar
  }

  /// The stage: the component on the canvas, with its measured size and the count of what it did under it.
  fn stage(&self, spec: &mut Spec) -> Column {
    let mut held = Row::new()
      .ref_element(self.anchor.clone())
      .child(std::mem::take(&mut spec.shown));
    if spec.fill {
      held = held.width(FILL);
    }
    let measured = if spec.facts.is_empty() {
      self.measured()
    } else {
      format!("{} · {}", self.measured(), spec.facts)
    };
    Column::new()
      .width(FILL)
      .height(418.0)
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
          .child(mono(&measured, 12.0))
          .child(
            Row::new()
              .spacing(6.0)
              .align_items(Alignment::Center)
              .child(icon(icons::HAND_POINTER, 14.0))
              .child(mono(&format!("{} × {}", spec.event, self.count.get()), 12.0)),
          ),
      )
  }

  /// The tabs under the stage, and what the chosen one shows.
  fn output(&self, spec: &Spec) -> Column {
    let events = self.events.get();
    let shown = self.tab.get();
    let mut tabs = Row::new().height(FILL).spacing(20.0);
    for (index, title) in ["Code", "Events", "Layout tree"].into_iter().enumerate() {
      let on = index == shown;
      let tab = self.tab.clone();
      let mut entry = Row::new()
        .height(FILL)
        .spacing(6.0)
        .align_items(Alignment::Center)
        .cursor(CursorIcon::Pointer)
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
      if index == 1 && self.count.get() > 0 {
        entry = entry.child(
          badge(&self.count.get().to_string())
            .height(18.0)
            .padding_horizontal(6.0),
        );
      }
      tabs = tabs.child(entry);
    }
    let copied = match shown {
      0 => spec.code.join("\n"),
      1 => events.clone(),
      _ => spec.tree.join("\n"),
    };
    let body: Element = match shown {
      0 => code_block(&spec.code).into(),
      1 if events.is_empty() => plain_lines(&["Nothing yet: use what is on the stage.".to_owned()]).into(),
      1 => plain_lines(&events.lines().map(str::to_owned).collect::<Vec<_>>()).into(),
      _ => plain_lines(&spec.tree).into(),
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

  /// The panel beside the stage: how many props were changed, Reset, and the controls.
  fn panel(&self, spec: &mut Spec) -> Column {
    let reset = spec.reset.clone();
    let mut controls = Column::new().width(FILL).padding(16.0).spacing(16.0);
    for control in std::mem::take(&mut spec.controls) {
      controls = controls.child(control);
    }
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
              .child(quiet(&format!("{} changed", spec.changed), 11.0)),
          )
          .child(tool("Reset", icons::REFRESH, move || reset())),
      )
      .child(controls)
  }

  /// The playground: the stage with its tabs, and the panel beside it, or under it in a narrow window.
  pub fn card(&self, ctx: &mut Ctx, mut spec: Spec) -> Element {
    // What is on the stage says how large it is whenever that changes.
    let measured = self.measured.clone();
    self.anchor.observe_rect(move |rect| {
      let size = format!("{} × {} px", rect.width.round(), rect.height.round());
      if measured.get() != size {
        measured.set(size);
      }
    });
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let preview = Column::new()
      .child(self.toolbar(spec.forceable))
      .child(self.stage(&mut spec))
      .child(self.output(&spec));
    let panel = self.panel(&mut spec);
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

/// A control of the panel under the name of its prop and its type.
pub fn prop(label: &str, kind: &str, control: impl Into<Element>) -> Element {
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
    .into()
}

/// A prop that is on or off: its name, and a switch.
pub fn flag(label: &str, value: &Signal<bool>) -> Element {
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
    .into()
}

/// A prop that is text: a field bound to it.
pub fn field(value: &Signal<String>) -> Element {
  let body = lurq::ui::current_theme().typography_style(TypographyStyle::Body);
  Row::new()
    .width(FILL)
    .height(36.0)
    .padding_horizontal(12.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
    .child(
      TextInput::styled(value.clone(), body)
        .flex(1.0)
        .height(FILL)
        .tab_index(0),
    )
    .into()
}

/// A line between the props that are chosen and those that are on or off.
pub fn divider() -> Element {
  Rect::new(FILL, 1.0).background(PaletteColor::Border).into()
}
