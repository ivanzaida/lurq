//! The page of style by hand: the theme's foreground and an element's own, disabled elements, focus rings, text
//! decorations and a limit of lines, transitions of what is only painted, and reduced motion. Its playground styles
//! a box and the text in it as its props say; its sections are the demo's Style page.
//!
//! The demo set the foreground on the app's theme. Here it is set on a theme of the section's own, so the gallery
//! around it keeps its text as it is.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  animation::Transition,
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Button, Column, Markdown, MarkdownProps, Row, ScrollVertical, Text, TextInput},
  core::Signal,
  layout::{
    Alignment,
    text_style::{FontWeight, TextStrikethrough, TextStyle, TextUnderline},
  },
  node::{BoxShadow, CursorIcon, Element, Outline, border::Border, transform::Transform2D},
  ui::{ButtonSize, Switch, Toggle},
};

use super::{
  kit::{controls, number, samples},
  theme::follow,
};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The roles the theme's foreground can be given here: the theme's own, and three others.
const INKS: [(&str, PaletteColor); 4] = [
  ("Foreground", PaletteColor::Foreground),
  ("MutedForeground", PaletteColor::MutedForeground),
  ("Primary", PaletteColor::Primary),
  ("Destructive", PaletteColor::Destructive),
];
const INK_CHOICES: [(&str, usize); 4] = [
  ("Foreground", 0),
  ("MutedForeground", 1),
  ("Primary", 2),
  ("Destructive", 3),
];
/// The foreground the playground's box is given of its own, or none.
const OWN: [(&str, Option<PaletteColor>); 4] = [
  ("none", None),
  ("Primary", Some(PaletteColor::Primary)),
  ("Destructive", Some(PaletteColor::Destructive)),
  ("MutedForeground", Some(PaletteColor::MutedForeground)),
];
const OWN_CHOICES: [(&str, usize); 4] = [("none", 0), ("Primary", 1), ("Destructive", 2), ("MutedForeground", 3)];
/// The most lines the playground's text takes, 0 for no limit.
const LIMITS: [(&str, usize); 4] = [("none", 0), ("1", 1), ("2", 2), ("3", 3)];
const LONG: &str = "The quick brown fox jumps over the lazy dog while seven wizards quietly box, and then the fox, \
                    having jumped, sits down to think about what the dog made of it all and whether it will ever \
                    jump again.";
/// How wide the boxes of the motion section are, and the farthest the second one moves.
const MOVER: f32 = 260.0;
const TRAVEL: f32 = 220.0;

/// A sheet the samples of a section stand on, of `fill`, with the theme's foreground on it.
fn sheet(fill: PaletteColor) -> Column {
  Column::new()
    .width(FILL)
    .padding(16.0)
    .spacing(8.0)
    .rounded(RadiusSize::Md)
    .background(fill)
    .foreground(PaletteColor::Foreground)
}

/// The engine's button, plain, with a ring that grows in round it when it has the focus from the keyboard.
fn focusable(caption: &str) -> Button {
  Button::new(caption)
    .padding_horizontal(14.0)
    .height(32.0)
    .background(PaletteColor::Background)
    .border(Border::inside(1.0, PaletteColor::Border))
    .rounded(6.0)
    .cursor(CursorIcon::Pointer)
    .tab_index(0)
    .transition(Transition::outline().duration_ms(140))
    .focus_visible(|style| style.outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0)))
}

/// What the playground's props are, at a render.
#[derive(Clone, Copy, PartialEq)]
struct Props {
  own: usize,
  underline: bool,
  strike: bool,
  limit: usize,
  ring: bool,
  off: bool,
}

pub struct StylePage {
  frame: Frame,
  playground: Playground,
  /// An index into [`OWN`].
  own: Signal<usize>,
  underline: Signal<bool>,
  strike: Signal<bool>,
  /// An index into [`LIMITS`].
  limit: Signal<usize>,
  ring: Signal<bool>,
  off: Signal<bool>,
  /// An index into [`INKS`]: the foreground of the theme of the first section.
  ink: Signal<usize>,
  /// The theme of the first section, whose foreground is set: the gallery's is not.
  paper: Theme,
  typed: Signal<String>,
  disabled: Signal<bool>,
  clicks: Signal<usize>,
  disabled_text: Signal<String>,
  lines: Signal<i32>,
  moved: Signal<bool>,
  /// The app's theme, whose reduced motion the last section sets, and whether it did: leaving the page gives the
  /// decision back to the system's setting.
  app: Theme,
  took_motion: Arc<AtomicBool>,
}

impl StylePage {
  fn props(&self) -> Props {
    Props {
      own: self.own.get().min(OWN.len() - 1),
      underline: self.underline.get(),
      strike: self.strike.get(),
      limit: self.limit.get().min(LIMITS.len() - 1),
      ring: self.ring.get(),
      off: self.off.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record = self.playground.recorder();
    let build = move || -> Element {
      let record = record.clone();
      let mut words = Text::new(LONG).width(FILL);
      if props.underline {
        words = words.underline(TextUnderline::new());
      }
      if props.strike {
        words = words.strikethrough(TextStrikethrough::new());
      }
      if props.limit > 0 {
        words = words.max_lines(props.limit as u32);
      }
      let mut shown = Column::new()
        .width(320.0)
        .max_width(FILL)
        .padding(16.0)
        .spacing(8.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Background)
        .border(Border::inside(1.0, PaletteColor::Border))
        .cursor(CursorIcon::Pointer)
        .transition(Transition::background_color().duration_ms(150))
        .hovered(|style| style.background(PaletteColor::Highlight))
        .disabled(props.off)
        .when_disabled(|style| style.opacity(0.45))
        .on_click(move |_| record("on_click".to_owned()))
        .child(text("A box", 14.0, FontWeight::SemiBold))
        .child(words);
      if let Some(role) = OWN[props.own].1.clone() {
        shown = shown.foreground(role);
      }
      if props.ring {
        shown = shown.outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0));
      }
      shown.into()
    };

    let mut code = vec![
      "Column::new()".to_owned(),
      "    .width(320.0)".to_owned(),
      "    .background(PaletteColor::Background)".to_owned(),
      "    .hovered(|style| style.background(PaletteColor::Highlight))".to_owned(),
    ];
    if props.own > 0 {
      code.push(format!("    .foreground(PaletteColor::{})", OWN[props.own].0));
    }
    if props.ring {
      code.push("    .outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0))".to_owned());
    }
    if props.off {
      code.push("    .disabled(true)".to_owned());
      code.push("    .when_disabled(|style| style.opacity(0.45))".to_owned());
    }
    code.push("    .on_click(|_| ..)".to_owned());
    code.push("    .child(Text::new(\"A box\"))".to_owned());
    let mut words = "    .child(Text::new(LONG)".to_owned();
    if props.underline {
      words.push_str(".underline(TextUnderline::new())");
    }
    if props.strike {
      words.push_str(".strikethrough(TextStrikethrough::new())");
    }
    if props.limit > 0 {
      words.push_str(&format!(".max_lines({})", props.limit));
    }
    words.push(')');
    code.push(words);
    let tree = vec![
      format!("column   {}", self.playground.measured()),
      "├─ text \"A box\"".to_owned(),
      format!(
        "└─ text   {}",
        if props.limit > 0 {
          format!("{} lines at most", props.limit)
        } else {
          "every line".to_owned()
        }
      ),
    ];

    let (set_own, set_limit) = (self.own.clone(), self.limit.clone());
    let reset = {
      let (own, underline, strike, limit, ring, off) = (
        self.own.clone(),
        self.underline.clone(),
        self.strike.clone(),
        self.limit.clone(),
        self.ring.clone(),
        self.off.clone(),
      );
      Arc::new(move || {
        own.set(0);
        underline.set(false);
        strike.set(false);
        limit.set(3);
        ring.set(false);
        off.set(false);
      })
    };
    let controls = vec![
      prop(
        "foreground",
        "impl Into<TextColor>",
        self
          .playground
          .select(0, &OWN_CHOICES, &props.own, move |value| set_own.set(value)),
      ),
      prop(
        "max_lines",
        "u32",
        self
          .playground
          .select(1, &LIMITS, &props.limit, move |value| set_limit.set(value)),
      ),
      divider(),
      flag("underline", &self.underline),
      flag("strikethrough", &self.strike),
      flag("outline", &self.ring),
      flag("disabled", &self.off),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Style",
        make: Arc::new(build.clone()),
        shown: Shown::built(move |_| build()),
        fill: true,
        forceable: false,
        facts: if props.off {
          "disabled".to_owned()
        } else {
          String::new()
        },
        event: "on_click",
        code,
        tree,
        controls,
        changed: [
          props.own != 0,
          props.underline,
          props.strike,
          props.limit != 3,
          props.ring,
          props.off,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The chooser of the theme's foreground, and a sheet in that theme with text that names no color, text that
  /// names one, a field and a Markdown document.
  fn inked(&self, ctx: &mut Ctx) -> Column {
    let ink = self.ink.get().min(INKS.len() - 1);
    let set = self.ink.clone();
    let typed = self.typed.clone();
    let paper = self.paper.clone();
    let sheet = ctx.in_theme(&paper, |ctx| {
      let theme = ctx.theme().clone();
      let mut hint = theme.typography_style(TypographyStyle::Body);
      hint.color = theme.palette_color(PaletteColor::MutedForeground);
      let plain = TextStyle {
        font_size: 15.0,
        ..TextStyle::default()
      };
      sheet(PaletteColor::Background)
        .border(Border::inside(1.0, PaletteColor::Border))
        .child(Text::new("A text with no color: it takes the theme's foreground.").width(FILL))
        .child(
          Text::styled(
            "A text given a style that keeps the default color: it takes it too.",
            plain,
          )
          .width(FILL),
        )
        .child(
          Text::new("A text that names its color keeps it.")
            .color(PaletteColor::Success)
            .width(FILL),
        )
        .child(
          TextInput::new(typed)
            .placeholder("Type here: the text and the caret take it")
            .placeholder_style(hint)
            .padding_horizontal(10.0)
            .size(360.0, 32.0)
            .max_width(FILL)
            .background(PaletteColor::Background)
            .rounded(6.0)
            .border(Border::inside(1.0, PaletteColor::Input))
            .tab_index(0),
        )
        .child(Markdown::mount(
          ctx,
          MarkdownProps::new("A **Markdown** document whose style names no color takes it, `code` and all."),
        ))
    });
    // A toggle for each role, on a line of their own where there is no room beside one another.
    let mut choices = Row::new()
      .width(FILL)
      .spacing(6.0)
      .wrap()
      .align_items(Alignment::Center);
    for (name, index) in INK_CHOICES {
      let set = set.clone();
      choices = choices.child(
        Toggle::new(name)
          .size(ButtonSize::Sm)
          .on(index == ink)
          .on_change(move |_| set.set(index)),
      );
    }
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(quiet("PaletteColor::Foreground", 12.0))
      .child(choices)
      .child(sheet)
  }

  /// A group that can be disabled whole: two buttons, a field and a list that still scrolls, with the count of the
  /// clicks the first button took.
  fn disabled(&self) -> Column {
    let off = self.disabled.get();
    let (set, clicks) = (self.disabled.clone(), self.clicks.clone());
    let mut lines = Column::new().width(FILL).padding(8.0);
    for index in 1..=12 {
      lines = lines.child(Text::new(&format!("Line {index} of a list that still scrolls")).height(22.0));
    }
    let group = sheet(PaletteColor::Muted)
      .disabled(off)
      .when_disabled(|style| style.opacity(0.45))
      .child(
        Row::new()
          .width(FILL)
          .spacing(10.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(focusable("A button").on_click(move |_| clicks.update(|count| *count += 1)))
          .child(focusable("Another"))
          .child(
            TextInput::new(self.disabled_text.clone())
              .placeholder("An input")
              .padding_horizontal(10.0)
              .size(180.0, 32.0)
              .max_width(FILL)
              .background(PaletteColor::Background)
              .rounded(6.0)
              .border(Border::inside(1.0, PaletteColor::Input))
              .tab_index(0),
          ),
      )
      .child(
        ScrollVertical::new(lines)
          .size(320.0, 88.0)
          .max_width(FILL)
          .background(PaletteColor::Background)
          .rounded(6.0),
      );
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(
            Switch::new("The group is disabled")
              .on(off)
              .on_change(move |on| set.set(on)),
          )
          .child(quiet(&format!("clicks on the button: {}", self.clicks.get()), 12.0)),
      )
      .child(group)
  }

  /// The lift that comes in under the pointer, the box that moves and fades when it is clicked, and the switch of
  /// the app's reduced motion.
  fn motion(&self, ctx: &mut Ctx) -> Column {
    let now = ctx.reduced_motion();
    let (app, took) = (ctx.theme().clone(), self.took_motion.clone());
    let moved = self.moved.get();
    let flip = self.moved.clone();
    // As far as the card has room for: a transform moves what is painted, and the card would cut it off.
    let room = self.frame.stage_room();
    let travel = if room > 0.0 {
      (room - MOVER).clamp(0.0, TRAVEL)
    } else {
      TRAVEL
    };
    let lift = Column::new()
      .size(MOVER, 64.0)
      .max_width(FILL)
      .padding(12.0)
      .rounded(10.0)
      .background(PaletteColor::Background)
      .foreground(PaletteColor::Foreground)
      .border(Border::inside(1.0, PaletteColor::Border))
      .cursor(CursorIcon::Pointer)
      .transition(Transition::box_shadow().duration_ms(250))
      .transition(Transition::outline().duration_ms(250))
      .hovered(|style| {
        style
          .box_shadow(BoxShadow::new(0.0, 12.0, 28.0, PaletteColor::Scrim))
          .outline(Outline::new(2.0, PaletteColor::Ring).offset(3.0))
      })
      .child(Text::new("Hover: a shadow and an outline come in").width(FILL));
    let slide = Column::new()
      .size(MOVER, 64.0)
      .max_width(FILL)
      .padding(12.0)
      .rounded(10.0)
      .background(PaletteColor::Primary)
      .foreground(PaletteColor::PrimaryForeground)
      .cursor(CursorIcon::Pointer)
      .opacity(if moved { 0.55 } else { 1.0 })
      .transform(Transform2D::translate(if moved { travel } else { 0.0 }, 0.0))
      .transition(Transition::transform().duration_ms(600))
      .transition(Transition::opacity().duration_ms(600))
      .on_click(move |_| flip.update(|moved| *moved = !*moved))
      .child(Text::new("Click: it moves and fades").width(FILL));
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(Switch::new("The app reduces motion").on(now).on_change(move |on| {
            took.store(true, Ordering::Relaxed);
            app.set_reduced_motion(Some(on));
          }))
          .child(quiet(
            if now {
              "in effect: motion is reduced"
            } else {
              "in effect: motion is not reduced"
            },
            12.0,
          )),
      )
      .child(Row::new().width(FILL).padding(16.0).overflow_visible().child(lift))
      .child(Row::new().width(FILL).padding(16.0).child(slide))
  }
}

/// A row in a foreground of its own, with a count that names its color; and a box whose one style under the
/// pointer recolors its fill and its label together.
fn own() -> Row {
  samples()
    .spacing(12.0)
    .child(
      Row::new()
        .spacing(10.0)
        .padding(14.0)
        .rounded(8.0)
        .align_items(Alignment::Center)
        .background(PaletteColor::Primary)
        .foreground(PaletteColor::PrimaryForeground)
        .child(Text::new("Inbox"))
        .child(Text::new("Unread"))
        .child(
          Row::new()
            .padding_horizontal(6.0)
            .rounded(RadiusSize::Full)
            .background(PaletteColor::PrimaryForeground)
            .child(Text::new("12").color(PaletteColor::Primary)),
        ),
    )
    .child(
      Column::new()
        .max_width(FILL)
        .padding(14.0)
        .rounded(8.0)
        .background(PaletteColor::Muted)
        .foreground(PaletteColor::MutedForeground)
        .cursor(CursorIcon::Pointer)
        .transition(Transition::foreground().duration_ms(200))
        .transition(Transition::background_color().duration_ms(200))
        .hovered(|style| {
          style
            .background(PaletteColor::Primary)
            .foreground(PaletteColor::PrimaryForeground)
        })
        .child(Text::new("Hover me: one style recolors the fill and the label")),
    )
}

/// Three buttons with room round them, and two at the edges of a container with none.
fn rings() -> Column {
  Column::new()
    .width(FILL)
    .spacing(16.0)
    .child(
      sheet(PaletteColor::Muted).child(
        Row::new()
          .width(FILL)
          .spacing(12.0)
          .wrap()
          .child(focusable("First"))
          .child(focusable("Second"))
          .child(focusable("Third")),
      ),
    )
    .child(
      // No padding: the buttons fill their container to its edges, which cut off what is in it.
      Row::new()
        .max_width(FILL)
        .wrap()
        .rounded(6.0)
        .background(PaletteColor::Muted)
        .child(focusable("At the edge"))
        .child(focusable("of its container")),
    )
}

impl Component for StylePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let app = ctx.theme().clone();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      own: ctx.signal(0),
      underline: ctx.signal(false),
      strike: ctx.signal(false),
      limit: ctx.signal(3),
      ring: ctx.signal(false),
      off: ctx.signal(false),
      ink: ctx.signal(0),
      paper: Theme::from_definition(app.definition()),
      typed: ctx.signal(String::new()),
      disabled: ctx.signal(false),
      clicks: ctx.signal(0),
      disabled_text: ctx.signal(String::new()),
      lines: ctx.signal(3),
      moved: ctx.signal(false),
      app,
      took_motion: Arc::default(),
    }
  }

  fn on_unmounted(&self) {
    if self.took_motion.load(Ordering::Relaxed) {
      self.app.set_reduced_motion(None);
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The first section's theme is the page's, with the foreground chosen: set only where it differs, so that
    // the render this asks for finds it as it was left.
    let page = ctx.theme().clone();
    follow(&self.paper, &page, true);
    let ink = page.palette_color(INKS[self.ink.get().min(INKS.len() - 1)].1.clone());
    if self.paper.palette_color(PaletteColor::Foreground) != ink {
      self.paper.set_palette_color(PaletteColor::Foreground, ink);
    }
    let rows = [
      (
        "foreground",
        "impl Into<TextColor>",
        "",
        "The color of the text in the element and under it, as CSS color. A text that names its own keeps it; one \
         with none above it takes the theme's Foreground.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Nothing in the element takes a click, a hover, the focus or Tab, and the focus in it is let go. What is in \
         it still scrolls.",
      ),
      (
        "when_disabled",
        "Fn(Style) -> Style",
        "",
        "How the element is painted while it is disabled: an opacity, a fill.",
      ),
      (
        "focus_visible",
        "Fn(Style) -> Style",
        "",
        "How the element is painted while it has the focus from the keyboard, not from a click: a ring.",
      ),
      (
        "outline",
        "Outline",
        "",
        "A line round the element, outside its box, that takes no room: Outline::new(width, color).offset(gap).",
      ),
      (
        "hovered",
        "Fn(Style) -> Style",
        "",
        "How the element is painted under the pointer. A state changes what is painted, never a size.",
      ),
      (
        "transition",
        "Transition",
        "",
        "Runs a change of one value from the old to the new: Transition::outline(), ::foreground(), ::transform().",
      ),
      (
        "Text::underline",
        "TextUnderline",
        "",
        "A line under the text, of the text's color or its own, and of a thickness of its own.",
      ),
      (
        "Text::strikethrough",
        "TextStrikethrough",
        "",
        "A line through the text.",
      ),
      (
        "Text::max_lines",
        "u32",
        "",
        "The most lines the text takes: the last ends in an ellipsis.",
      ),
      (
        "set_reduced_motion",
        "Option<bool>",
        "None",
        "On the theme: the app's decision; None follows the system's. Reduced, a change of place or size ends at \
         once and a fade still runs.",
      ),
    ];
    let playground = self.play(ctx);
    let inked = self.inked(ctx);
    let motion = self.motion(ctx);
    let destructive = page.palette_color(PaletteColor::Destructive);
    let decorations = Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        sheet(PaletteColor::Background)
          .border(Border::inside(1.0, PaletteColor::Border))
          .child(Text::new("An underlined text").underline(TextUnderline::new()))
          .child(
            Text::new("An underline of its own color and thickness")
              .underline(TextUnderline::new().color(destructive).thickness(2.0))
              .width(FILL),
          )
          .child(Text::new("A struck text").strikethrough(TextStrikethrough::new()))
          .child(
            Text::new(LONG)
              .underline(TextUnderline::new())
              .width(300.0)
              .max_width(FILL)
              .max_lines(2),
          ),
      )
      .child(number("Lines at most", &self.lines, 1, 6))
      .child(
        sheet(PaletteColor::Background)
          .border(Border::inside(1.0, PaletteColor::Border))
          .child(
            Text::new(LONG)
              .width(300.0)
              .max_width(FILL)
              .max_lines(self.lines.get().max(1) as u32),
          ),
      );
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Style"],
        title: "Style",
        description: "What an element is painted with in its states, by hand: the foreground its text takes, how it \
                      looks disabled and with the focus, the lines on and the lines of a text, and the changes that \
                      run as transitions, which reduced motion cuts short where they move.",
        import: Some("use lurq::{animation::Transition, node::Outline};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A box and the text in it, styled as the props say. Give the box a foreground of its own, a ring, a limit \
           of lines and lines on its text; hover it and click it, then disable it: the clicks stop.",
          playground,
        ),
        Section::new(
          "The theme's foreground",
          "The text of the sheet takes the theme's Foreground, as a window's does: choose another role for it. A \
           text with no color takes it, and so does one whose style keeps the default color, a field's text and \
           caret, and Markdown; a text that names its color keeps it. The role is set on a theme of this card's \
           own, so the rest of the gallery keeps its text as it is.",
          self.frame.demo(
            0,
            inked,
            "theme.set_palette_color(PaletteColor::Foreground, color)",
            &[
              "app.theme().set_palette_color(PaletteColor::Foreground, color);",
              "",
              "Text::new(\"A text with no color: it takes the theme's foreground.\")",
              "Text::new(\"A text that names its color keeps it.\").color(PaletteColor::Success)",
              "TextInput::new(typed).placeholder(\"Type here\")",
            ],
          ),
        ),
        Section::new(
          "An element's own foreground",
          "An element's foreground comes first, for everything in it: the row's words take it, and the count that \
           names its color keeps its own. One style under the pointer recolors a fill and its label together.",
          self.frame.demo(
            1,
            own(),
            ".foreground(PaletteColor::PrimaryForeground)",
            &[
              "Row::new()",
              "    .background(PaletteColor::Primary)",
              "    .foreground(PaletteColor::PrimaryForeground)",
              "    .child(Text::new(\"Inbox\"))",
              "    .child(Text::new(\"12\").color(PaletteColor::Primary))",
              "",
              "Column::new()",
              "    .foreground(PaletteColor::MutedForeground)",
              "    .transition(Transition::foreground().duration_ms(200))",
              "    .transition(Transition::background_color().duration_ms(200))",
              "    .hovered(|style| style.background(PaletteColor::Primary).foreground(PaletteColor::PrimaryForeground))",
            ],
          ),
        ),
        Section::new(
          "Disabled",
          "Disabled, nothing in the group takes a click, a hover, the focus or Tab, and the focus that was in it is \
           let go: the count of clicks stops. The list in it still scrolls, by the wheel and by its bar.",
          self.frame.demo(
            2,
            self.disabled(),
            ".disabled(true).when_disabled(|style| style.opacity(0.45))",
            &[
              "Column::new()",
              "    .disabled(off)",
              "    .when_disabled(|style| style.opacity(0.45))",
              "    .child(Button::new(\"A button\").tab_index(0).on_click(|_| ..))",
              "    .child(TextInput::new(text).tab_index(0))",
              "    .child(ScrollVertical::new(lines))",
            ],
          ),
        ),
        Section::new(
          "Focus rings",
          "Press Tab: a ring grows in round the button with the focus, outside its box, and moves nothing. Click a \
           button: no ring. The ring of a button at its container's edge is not cut off there.",
          self.frame.demo(
            3,
            rings(),
            ".focus_visible(|style| style.outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0)))",
            &[
              "Button::new(\"First\")",
              "    .tab_index(0)",
              "    .transition(Transition::outline().duration_ms(140))",
              "    .focus_visible(|style| style.outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0)))",
            ],
          ),
        ),
        Section::new(
          "Lines on a text, and a limit of lines",
          "An underline, one of its own color and thickness, and a line through. A text cut to the lines it may \
           take ends in an ellipsis; try a fractional display scale: it never paints a line more than its box holds.",
          self.frame.demo(
            4,
            decorations,
            ".max_lines(3)",
            &[
              "Text::new(\"An underlined text\").underline(TextUnderline::new())",
              "Text::new(\"..\").underline(TextUnderline::new().color(red).thickness(2.0))",
              "Text::new(\"A struck text\").strikethrough(TextStrikethrough::new())",
              "Text::new(LONG).width(300.0).max_lines(3)",
            ],
          ),
        ),
        Section::new(
          "Transitions of paint, and reduced motion",
          "A shadow and an outline come in under the pointer; a click moves the second box and fades it. With \
           motion reduced it is at its new place at once and still fades: the shadow and the outline are no \
           motion and come in either way. The switch sets the app's theme, as the gallery's settings do; leaving \
           the page gives the decision back to the system.",
          self.frame.demo(
            5,
            motion,
            "theme.set_reduced_motion(Some(true))",
            &[
              "app.theme().set_reduced_motion(Some(true));",
              "",
              "Column::new()",
              "    .transform(Transform2D::translate(if moved { 220.0 } else { 0.0 }, 0.0))",
              "    .opacity(if moved { 0.55 } else { 1.0 })",
              "    .transition(Transition::transform().duration_ms(600))",
              "    .transition(Transition::opacity().duration_ms(600))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an element is given for its states and its text, and the theme for its motion.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
