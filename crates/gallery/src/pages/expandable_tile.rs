//! The page of the expandable tile: its playground, the board's tile open and closed, a tile that always shows
//! something, tiles side by side, and one bound to a signal.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, TypographyStyle},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Badge, BadgeVariant, BulletedList, Button, ButtonSize, ButtonVariant, ExpandableTile, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, field, flag, prop},
};

/// The board's words (`slE1t`).
const TITLE: &str = "Renderer";
const DETAIL: &str = "wgpu 26 on DirectX 12. Frame time 6.9 ms median.";

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// A figure a tile always shows.
fn figure(value: &str, unit: &str) -> Row {
  Row::new()
    .spacing(6.0)
    .align_items(Alignment::End)
    .child(Text::new(value).variant(TypographyStyle::Title))
    .child(
      Text::new(unit)
        .variant(TypographyStyle::Caption)
        .color(PaletteColor::MutedForeground),
    )
}

pub struct ExpandableTilePage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  detail: Signal<String>,
  open: Signal<bool>,
  body: Signal<bool>,
  element: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  plus: Signal<bool>,
  /// The tiles of the sections, each open or not.
  closed: Signal<bool>,
  fps: Signal<bool>,
  memory: Signal<bool>,
  plan: Signal<bool>,
}

impl ExpandableTilePage {
  /// The playground: the tile as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, detail, open) = (self.title.get(), self.detail.get(), self.open.get());
    let (body, element, disabled) = (self.body.get(), self.element.get(), self.disabled.get());
    let (full_width, plus) = (self.full_width.get(), self.plus.get());
    let forced = self.playground.forced();
    let record = self.playground.recorder();
    let build = {
      let (title, detail) = (title.clone(), detail.clone());
      move || -> ExpandableTile {
        let mut tile = if element {
          ExpandableTile::new(
            &title,
            Column::new()
              .width(FILL)
              .spacing(10.0)
              .child(BulletedList::new(["DirectX 12", "Frame time 6.9 ms median"]))
              .child(
                Button::new("Copy report")
                  .variant(ButtonVariant::Outline)
                  .size(ButtonSize::Sm),
              ),
          )
        } else {
          ExpandableTile::text(&title, &detail)
        };
        if body {
          tile = tile.child(figure("144", "frames per second"));
        }
        if plus {
          tile = tile.icon(icons::PLUS).open_icon(icons::MINUS);
        }
        tile.disabled(disabled).full_width(full_width)
      }
    };
    let make = {
      let build = build.clone();
      move || -> Element { build().open(open).into() }
    };
    let shown = {
      let (bound, record) = (self.open.clone(), record.clone());
      build()
        .bind(bound)
        .force(forced)
        .on_open_change(move |open| record(format!("on_open_change({open})")))
    };
    let mut code = vec![if element {
      format!(
        "ExpandableTile::new({title:?}, Column::new().child(BulletedList::new(..)).child(Button::new(\"Copy report\")))"
      )
    } else {
      format!("ExpandableTile::text({title:?}, {detail:?})")
    }];
    if body {
      code.push("    .child(figure(\"144\", \"frames per second\"))".to_owned());
    }
    code.push("    .bind(open.clone())".to_owned());
    if plus {
      code.push("    .icon(icons::PLUS)".to_owned());
      code.push("    .open_icon(icons::MINUS)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if full_width {
      code.push("    .full_width(true)".to_owned());
    }
    code.push("    .on_open_change(|open| ..)".to_owned());
    let mut tree = vec![
      format!(
        "root       {}   pressed · padded 16 · radius 8 · edge Border",
        self.playground.measured()
      ),
      "├─ header  title 14 medium · chevron 16 MutedForeground, 8 after it".to_owned(),
    ];
    if body {
      tree.push("├─ body    always shown · 10 under the header".to_owned());
    }
    tree.push(format!(
      "└─ panel   {} · its height runs over the theme's base duration",
      if open {
        "as high as its detail, 10 under the header"
      } else {
        "of no height"
      }
    ));
    let reset = {
      let (title, detail, open, body) = (
        self.title.clone(),
        self.detail.clone(),
        self.open.clone(),
        self.body.clone(),
      );
      let (element, disabled, full_width, plus) = (
        self.element.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
        self.plus.clone(),
      );
      Arc::new(move || {
        title.set(TITLE.to_owned());
        detail.set(DETAIL.to_owned());
        open.set(true);
        body.set(false);
        element.set(false);
        disabled.set(false);
        full_width.set(false);
        plus.set(false);
      })
    };
    let mut controls = vec![prop("new / text", "&str", field(&self.title))];
    if !element {
      controls.push(prop("text", "&str", field(&self.detail)));
    }
    controls.extend([
      flag("new(.., element)", &self.element),
      flag("child", &self.body),
      flag("open", &self.open),
      flag("icon · open_icon", &self.plus),
      flag("disabled", &self.disabled),
      flag("full_width", &self.full_width),
    ]);
    self.playground.card(
      ctx,
      Spec {
        title: "Expandable Tile",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(shown)),
        fill: full_width,
        forceable: true,
        facts: if open { "open".to_owned() } else { "closed".to_owned() },
        event: "on_open_change",
        code,
        tree,
        controls,
        changed: [
          title != TITLE,
          !element && detail != DETAIL,
          element,
          body,
          !open,
          plus,
          disabled,
          full_width,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

impl Component for ExpandableTilePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal(TITLE.to_owned()),
      detail: ctx.signal(DETAIL.to_owned()),
      open: ctx.signal(true),
      body: ctx.signal(false),
      element: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      plus: ctx.signal(false),
      closed: ctx.signal(false),
      fps: ctx.signal(false),
      memory: ctx.signal(true),
      plan: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(
        ExpandableTile::text(TITLE, DETAIL).open(true),
        "the board's tile · open(true)",
      ))
      .child(item(
        ExpandableTile::text(TITLE, DETAIL).bind(self.closed.clone()),
        "closed · bind(signal)",
      ))
      .child(item(
        ExpandableTile::text(TITLE, DETAIL).disabled(true),
        "disabled(true)",
      ));
    let always = Row::new()
      .width(FILL)
      .spacing(16.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(
        ExpandableTile::text(
          "Frame rate",
          "Measured over the last 600 frames on the primary monitor, vsync on.",
        )
        .child(figure("144", "frames per second"))
        .bind(self.fps.clone()),
      )
      .child(
        ExpandableTile::text(
          "Memory",
          "Textures 212 MB, glyph atlas 18 MB, layout tree 6 MB. The rest is the app's own.",
        )
        .child(figure("284", "MB resident"))
        .bind(self.memory.clone()),
      );
    let plan = item(
      ExpandableTile::new(
        "Team",
        Column::new()
          .width(FILL)
          .spacing(12.0)
          .child(BulletedList::new(["Unlimited seats", "Single sign-on", "Audit log"]).marker(icons::CHECK))
          .child(
            Button::new("Compare plans")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          ),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(figure("$12", "a seat, monthly"))
          .child(Badge::new("Popular").variant(BadgeVariant::Info)),
      )
      .icon(icons::PLUS)
      .open_icon(icons::MINUS)
      .bind(self.plan.clone()),
      "new(title, element) · child · icon · open_icon",
    );
    let rows = [
      (
        "new",
        "&str, impl Into<Element>",
        "",
        "Its title, and what it shows while it is open. A press of that element is the element's own.",
      ),
      (
        "text",
        "&str, &str",
        "",
        "Its title, and a text it shows while it is open, 13 on lines of 1.5 in the muted color.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "What it always shows, 10 under its header: a figure, a line, a chart.",
      ),
      ("open", "bool", "false", "Whether what it opens is shown."),
      (
        "bind",
        "Signal<bool>",
        "",
        "Open while the signal is true, which a press of the tile turns.",
      ),
      (
        "on_open_change",
        "Fn(bool)",
        "",
        "Told of a press, with what the tile is to be; after the signal is set where it is bound.",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "chevron-down",
        "The icon of the chevron, or None for no chevron.",
      ),
      (
        "open_icon",
        "impl Into<IconChoice>",
        "chevron-up",
        "The icon of the chevron while it is open; its icon where it was given one and no open one.",
      ),
      ("disabled", "bool", "false", "Whether it takes no input."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of the design's 260.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling, in place of its title.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Expandable Tile"],
        title: "Expandable Tile",
        description: "A tile that is pressed to show more of itself: a fact with its details, a figure with where it \
                      comes from, a plan with what it includes. The whole tile is pressed, and what it opens opens \
                      with its height, as a collapsible's does.",
        import: Some("use lurq::ui::ExpandableTile;"),
        status: Some("Beta"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ExpandableTile is a control on the right; the switch over the stage draws it under the \
           pointer or focused.",
          playground,
        ),
        Section::new(
          "Expandable Tile",
          "The board's tile, open as it is drawn, and closed: a title, a chevron that says which, and under them \
           what it opens.",
          self.frame.demo(
            0,
            board,
            "ExpandableTile::text(..)",
            &[
              "ExpandableTile::text(\"Renderer\", \"wgpu 26 on DirectX 12. Frame time 6.9 ms median.\").open(true)",
              "",
              "// Open while the signal is true, which a press turns.",
              "ExpandableTile::text(\"Renderer\", \"wgpu 26 on DirectX 12. ..\").bind(open.clone())",
              "",
              "ExpandableTile::text(\"Renderer\", \"wgpu 26 on DirectX 12. ..\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "What it always shows",
          "A tile shows more of itself: what child gives it is there open and closed, and what it opens comes \
           under it. Tiles side by side open each on their own.",
          self.frame.demo(
            1,
            always,
            ".child(..)",
            &[
              "ExpandableTile::text(\"Frame rate\", \"Measured over the last 600 frames ..\")",
              "    .child(figure(\"144\", \"frames per second\"))",
              "    .bind(fps.clone())",
              "",
              "ExpandableTile::text(\"Memory\", \"Textures 212 MB, glyph atlas 18 MB ..\")",
              "    .child(figure(\"284\", \"MB resident\"))",
              "    .bind(memory.clone())",
            ],
          ),
        ),
        Section::new(
          "Elements in it",
          "What it opens can be anything: a list, a button. A press of an element the app put in the tile is that \
           element's alone, and Tab goes from the tile to the controls in it.",
          self.frame.demo(
            2,
            plan,
            "ExpandableTile::new(..)",
            &[
              "ExpandableTile::new(",
              "  \"Team\",",
              "  Column::new()",
              "    .child(BulletedList::new([\"Unlimited seats\", \"Single sign-on\", \"Audit log\"]).marker(icons::CHECK))",
              "    .child(Button::new(\"Compare plans\").variant(ButtonVariant::Outline).size(ButtonSize::Sm)),",
              ")",
              ".child(Row::new().child(figure(\"$12\", \"a seat, monthly\")).child(Badge::new(\"Popular\")))",
              ".icon(icons::PLUS)",
              ".open_icon(icons::MINUS)",
              ".bind(open.clone())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an expandable tile is given.",
          props_table(&rows, &["new", "text"], self.frame.room()),
        ),
      ],
    )
  }
}
