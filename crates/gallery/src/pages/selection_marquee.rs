//! The page of the selection marquee: its playground, the board's surface of tiles, a surface that scrolls under
//! the drag, and what the rectangle tells the app.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Button, ScrollVertical, Stack},
  core::Signal,
  layout::layout_kind::ScrollState,
  node::{Element, border::Border},
  ui::{Marquee, SelectionMarquee},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Shown, Spec, flag},
};

/// Where the tile `index` is on a surface of `columns` across, as the board lays its six out: 58 by 38, 14 from the
/// surface's edge, 72 across and 50 down from one to the next.
fn tile(index: usize, columns: usize) -> (f32, f32, f32, f32) {
  (
    14.0 + 72.0 * (index % columns) as f32,
    14.0 + 50.0 * (index / columns) as f32,
    58.0,
    38.0,
  )
}

/// How large a surface of `count` tiles, `columns` across, is.
fn size(count: usize, columns: usize) -> (f32, f32) {
  (
    72.0 * columns as f32 + 14.0,
    50.0 * count.div_ceil(columns) as f32 + 20.0,
  )
}

/// The tiles the rectangle touches.
fn touched(marquee: &Marquee, count: usize, columns: usize) -> Vec<usize> {
  (0..count)
    .filter(|index| {
      let (x, y, width, height) = tile(*index, columns);
      marquee.touches(x, y, width, height)
    })
    .collect()
}

/// What a surface of tiles keeps: what is selected, and what was when the marquee under way began.
#[derive(Clone)]
struct Tiles {
  selected: Signal<Vec<usize>>,
  before: Signal<Vec<usize>>,
}

impl Tiles {
  fn new(ctx: &mut Ctx, selected: &[usize]) -> Self {
    Self {
      selected: ctx.signal(selected.to_vec()),
      before: ctx.signal(Vec::new()),
    }
  }

  /// The tiles on their surface: a press of one selects it, with Ctrl in or out, and a press of the surface's own
  /// room selects none.
  fn surface(&self, count: usize, columns: usize, framed: bool) -> Stack {
    let selected = self.selected.get();
    let (width, height) = size(count, columns);
    let clear = self.selected.clone();
    let mut surface = Stack::new()
      .size(width, height)
      .background(PaletteColor::Background)
      .on_click(move |_: MouseEvent| {
        if !clear.get_untracked().is_empty() {
          clear.set(Vec::new());
        }
      });
    if framed {
      surface = surface
        .clip()
        .rounded(RadiusSize::Lg)
        .border(Border::inside(BorderSize::Sm, PaletteColor::Border));
    }
    for index in 0..count {
      let (x, y, width, height) = tile(index, columns);
      let on = selected.contains(&index);
      let pick = self.selected.clone();
      surface = surface.child(
        Button::empty()
          .absolute(x, y, width, height)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .border(if on {
            Border::inside(BorderSize::Md, PaletteColor::Info)
          } else {
            Border::inside(BorderSize::Sm, PaletteColor::Border)
          })
          .tab_index(0)
          .describe("a11y_name", format!("Tile {}", index + 1))
          .describe("a11y_selected", if on { "true" } else { "false" })
          .on_click(move |event: MouseEvent| {
            // The press is the tile's: the surface under it does not clear the selection.
            event.stop_propagation();
            pick.update(|selected| {
              if event.ctrl || event.meta {
                match selected.iter().position(|held| *held == index) {
                  Some(at) => {
                    selected.remove(at);
                  }
                  None => selected.push(index),
                }
              } else {
                *selected = vec![index];
              }
            });
          }),
      );
    }
    surface
  }

  /// The surface in its marquee, which selects the tiles it touches as the keys held at its press say.
  fn marquee(&self, count: usize, columns: usize, framed: bool) -> SelectionMarquee {
    let (held, pick, back) = (self.selected.clone(), self.selected.clone(), self.selected.clone());
    let (kept, read, undo) = (self.before.clone(), self.before.clone(), self.before.clone());
    SelectionMarquee::new(self.surface(count, columns, framed))
      .label("Tiles")
      .on_start(move |_| kept.set(held.get_untracked()))
      .on_change(move |marquee| {
        let next = marquee.select(&read.get_untracked(), touched(marquee, count, columns));
        if pick.get_untracked() != next {
          pick.set(next);
        }
      })
      .on_cancel(move || back.set(undo.get_untracked()))
  }
}

pub struct SelectionMarqueePage {
  frame: Frame,
  playground: Playground,
  disabled: Signal<bool>,
  shown: Signal<bool>,
  played: Tiles,
  board: Tiles,
  scrolled: Tiles,
  scroll: ScrollState,
}

impl SelectionMarqueePage {
  /// The playground: the board's surface in a marquee as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (disabled, shown) = (self.disabled.get(), self.shown.get());
    let record = self.playground.recorder();
    let tiles = self.played.clone();
    let selected = tiles.selected.get();
    let build = {
      let tiles = tiles.clone();
      move |record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let mut marquee = tiles.marquee(6, 3, true).disabled(disabled);
        if shown {
          marquee = marquee.rect(76.0, 6.0, 146.0, 82.0);
        }
        if let Some(record) = record {
          let (start, end, cancel) = (record.clone(), record.clone(), record);
          let say = |marquee: &Marquee| {
            format!(
              "{}, {}, {} by {}, {}",
              marquee.x.round(),
              marquee.y.round(),
              marquee.width.round(),
              marquee.height.round(),
              marquee.mode.as_str()
            )
          };
          marquee = marquee
            .on_start(move |marquee| start(format!("on_start({})", say(marquee))))
            .on_change_end(move |marquee| end(format!("on_change_end({})", say(marquee))))
            .on_cancel(move || cancel("on_cancel()".to_owned()));
        }
        marquee
      }
    };
    let shown_on_stage = {
      let build = build.clone();
      let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
      Shown::built(move |ctx| build(Some(record)).mount(ctx))
    };
    {
      let build = build.clone();
      self.playground.window_shows(move |ctx| build(None).mount(ctx));
    }
    let make = {
      let build = build.clone();
      move || -> Element { build(None).into() }
    };

    let mut code = vec![
      "// selected, before: Signal<Vec<usize>>".to_owned(),
      "SelectionMarquee::new(tiles)".to_owned(),
      ".label(\"Tiles\")".to_owned(),
    ];
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    if shown {
      code.push(".rect(76.0, 6.0, 146.0, 82.0)".to_owned());
    }
    code.extend(
      [
        ".on_start(|_| before.set(selected.get_untracked()))",
        ".on_change(|marquee| selected.set(marquee.select(&before.get_untracked(), touched(marquee))))",
        ".on_cancel(|| selected.set(before.get_untracked()))",
        ".mount(ctx)",
      ]
      .map(str::to_owned),
    );

    let tree = vec![
      format!(
        "root        {}   the app's surface, as large as it is",
        self.playground.measured()
      ),
      "├─ (the app's content)".to_owned(),
      "└─ marquee  the muted info fill in an edge of 1 in the info color, at 0.55: while it is dragged".to_owned(),
    ];

    let (reset_disabled, reset_shown, reset_selected) =
      (self.disabled.clone(), self.shown.clone(), self.played.selected.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Selection Marquee",
        make: Arc::new(make),
        shown: shown_on_stage,
        fill: false,
        forceable: false,
        facts: format!("{} selected", selected.len()),
        event: "on_change_end",
        code,
        tree,
        controls: vec![flag("disabled", &self.disabled), flag("rect", &self.shown)],
        changed: [disabled, shown].into_iter().filter(|changed| *changed).count(),
        reset: Arc::new(move || {
          reset_disabled.set(false);
          reset_shown.set(false);
          reset_selected.set(Vec::new());
        }),
      },
    )
  }
}

impl Component for SelectionMarqueePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      disabled: ctx.signal(false),
      shown: ctx.signal(false),
      played: Tiles::new(ctx, &[]),
      board: Tiles::new(ctx, &[1, 2, 4, 5]),
      scrolled: Tiles::new(ctx, &[]),
      scroll: ScrollState::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = self
      .board
      .marquee(6, 3, true)
      .rect(76.0, 6.0, 146.0, 82.0)
      .key("board")
      .mount(ctx);
    let (_, high) = size(36, 3);
    let scrolled = ScrollVertical::new(
      self
        .scrolled
        .marquee(36, 3, false)
        .scroll(&self.scroll)
        .key("scrolled")
        .mount(ctx),
    )
    .with_scroll_state(self.scroll.clone())
    .key("scroll")
    .width(230.0)
    .height(170.0_f32.min(high))
    .rounded(RadiusSize::Lg)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border));

    let rows = [
      (
        "child",
        "impl Into<Element>",
        "",
        "The app's surface, the argument of `new`: what the rectangle is dragged over.",
      ),
      (
        "on_start",
        "impl Fn(&Marquee)",
        "",
        "Told once as a drag becomes a marquee: where the app keeps what was selected before it.",
      ),
      (
        "on_change",
        "impl Fn(&Marquee)",
        "",
        "Told the rectangle each time it changes: `x`, `y`, `width`, `height` in the surface's own coordinates, \
         and `mode`, what the keys held at the press ask for (`Replace`, `Add` with Shift, `Toggle` with Ctrl).",
      ),
      (
        "on_change_end",
        "impl Fn(&Marquee)",
        "",
        "Told the last rectangle when the button comes up.",
      ),
      (
        "on_cancel",
        "impl Fn()",
        "",
        "Told when Escape ends a marquee: where the app puts back what was selected before it.",
      ),
      (
        "scroll",
        "&ScrollState",
        "",
        "The state of the scroll container the surface is in: the rectangle keeps its start on the content while \
         it scrolls under the drag.",
      ),
      ("disabled", "bool", "false", "The surface takes no drag."),
      (
        "rect",
        "f32, f32, f32, f32",
        "",
        "Draws the rectangle at a place with nothing dragging it: for a gallery, a test.",
      ),
      (
        "label",
        "&str",
        "",
        "What the surface is called, to what reads the window's tree.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps a drag from one render to the next: the rectangle is drawn as it is dragged.",
      ),
      ("style / rules", "SelectionMarqueePart", "", "Root, Marquee."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Selection Marquee"],
        title: "Selection Marquee",
        description: "The rectangle a person drags over a surface to select what it touches: the tiles of a \
                      grid, the files of a folder, the nodes of a canvas.",
        import: Some("use lurq::ui::{Marquee, MarqueeMode, SelectionMarquee};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The surface on the stage is live: press on it and drag, and the tiles the rectangle touches are \
           selected. With Shift they are added to what was selected, with Ctrl put in or taken out, and Escape \
           puts back what was selected before. A press of a tile selects it; a press of the surface's own room \
           selects none.",
          playground,
        ),
        Section::new(
          "Tiles",
          "The board's surface, with the rectangle drawn as the board has it. The marquee tells the app the \
           rectangle; the app reckons what it touches and keeps the selection, here in a `Signal<Vec<usize>>`.",
          self.frame.demo(
            0,
            board,
            "SelectionMarquee::new(tiles).on_change(|marquee| ..).mount(ctx)",
            &[
              "// selected, before: Signal<Vec<usize>>",
              "SelectionMarquee::new(tiles)",
              "    .on_start(move |_| before.set(selected.get_untracked()))",
              "    .on_change(move |marquee| {",
              "        // The tiles the rectangle shares some room with.",
              "        let touched = (0..6).filter(|tile| {",
              "            let (x, y, width, height) = place(*tile);",
              "            marquee.touches(x, y, width, height)",
              "        });",
              "        // Alone, added, or put in and taken out, as the keys held at the press say.",
              "        selected.set(marquee.select(&before.get_untracked(), touched));",
              "    })",
              "    .on_cancel(move || selected.set(before.get_untracked()))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A surface that scrolls",
          "The marquee wraps what scrolls, and is given the container's state. A drag held at the container's \
           edge scrolls it, faster the further the pointer is past the edge, and the rectangle keeps its start on \
           the content: it grows over tiles that were out of sight when the drag began.",
          self.frame.demo(
            1,
            scrolled,
            "ScrollVertical::new(SelectionMarquee::new(tiles).scroll(&state).mount(ctx))",
            &[
              "// state: ScrollState, kept by the component",
              "ScrollVertical::new(",
              "    SelectionMarquee::new(tiles)",
              "        .scroll(&state)",
              "        .on_change(|marquee| ..)",
              "        .mount(ctx),",
              ")",
              ".with_scroll_state(state.clone())",
              ".height(170.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a selection marquee is given.",
          props_table(&rows, &["child"], self.frame.room()),
        ),
      ],
    )
  }
}
