//! Glass over a page with something to bend: bands of colour, large words and a checkerboard scroll under
//! glass that stays where it is. Over a plain page a glass is only its blur; its lens shows where an edge of
//! something passes under its own edge.

use lurq::{
  app::ctx::Ctx,
  components::{Column, Rect, Row, ScrollVertical, Stack},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, Glass, HitTestBehavior, color::Color},
};

use crate::ui::{FILL, text};

/// The colours of the page's bands and tiles.
const COLORS: [&str; 6] = ["#ef4444", "#f59e0b", "#22c55e", "#06b6d4", "#3b82f6", "#a855f7"];

pub(crate) fn band(index: usize, words: &str) -> Column {
  let light = index % 3 == 2;
  Column::new()
    .width(FILL)
    .height(150.0)
    .padding(18.0)
    .justify(Justify::Center)
    .background(if light { "#f8fafc" } else { COLORS[index % COLORS.len()] })
    .child(text(
      words,
      30.0,
      FontWeight::Bold,
      if light { "#0f172a" } else { "#ffffff" },
    ))
}

/// A board of small tiles: where a lens is seen best.
pub(crate) fn board() -> Column {
  let mut board = Column::new().width(FILL);
  for row in 0..9 {
    board = board.child(Row::new().with_children((0..22).map(|column| {
      Rect::new(20.0, 20.0).background(if (row + column) % 2 == 0 {
        COLORS[(row * 2 + column) % COLORS.len()]
      } else {
        "#f8fafc"
      })
    })));
  }
  board
}

/// Words that name no colour: the glass they are on gives them one, and changes it as it turns.
fn words_on_glass(words: &str, size: f32) -> lurq::components::Text {
  lurq::components::Text::styled(
    words,
    lurq::layout::text_style::TextStyle {
      font_size: size,
      weight: FontWeight::SemiBold,
      ..Default::default()
    },
  )
  .nowrap()
}

fn round(symbol: &str, glass: Glass) -> Row {
  Row::new()
    .size(56.0, 56.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(28.0)
    .glass(glass)
    .child(words_on_glass(symbol, 22.0))
}

fn pill(words: &str, glass: Glass) -> Row {
  Row::new()
    .height(52.0)
    .padding_horizontal(22.0)
    .align_items(Alignment::Center)
    .rounded(26.0)
    .glass(glass)
    .child(words_on_glass(words, 16.0))
}

pub(crate) fn glass(ctx: &mut Ctx) -> Element {
  let safe_area = ctx.safe_area();
  let mut page = Column::new().width(FILL).padding_top(safe_area.top + 44.0);
  for (index, words) in [
    "Scroll me under the glass",
    "Its edge is a lens",
    "It turns dark over dark",
    "and light over light",
    "A press lights it",
  ]
  .into_iter()
  .enumerate()
  {
    page = page.child(band(index, words));
    if index % 2 == 0 {
      page = page.child(board());
    }
  }
  page = page.child(Rect::new(10.0, 260.0));

  // What stays where it is over the page: a row of clear and regular glass, a tinted one, and two that are
  // one glass.
  let controls = Column::new()
    .size(FILL, FILL)
    .hit_test(HitTestBehavior::ContentOnly)
    .padding_top(safe_area.top + 70.0)
    .padding_horizontal(18.0)
    .spacing(22.0)
    .overflow_visible()
    .child(
      Row::new()
        .width(FILL)
        .spacing(14.0)
        .overflow_visible()
        .child(pill("Regular", Glass::regular().interactive(true)))
        .child(pill("Clear", Glass::clear().interactive(true))),
    )
    .child(
      Row::new()
        .width(FILL)
        .spacing(14.0)
        .overflow_visible()
        .child(
          Row::new()
            .size(132.0, 132.0)
            .rounded(40.0)
            .glass(Glass::clear().interactive(true)),
        )
        .child(
          Column::new()
            .spacing(14.0)
            .overflow_visible()
            .child(pill(
              "Tinted",
              Glass::regular().tint(Color::new(59, 130, 246, 190)).interactive(true),
            ))
            .child(
              Row::new()
                .glass_container(22.0)
                .spacing(6.0)
                .overflow_visible()
                .child(round("−", Glass::regular().interactive(true)))
                .child(round("+", Glass::regular().interactive(true))),
            ),
        ),
    );

  Stack::new()
    .size(FILL, FILL)
    .background("#0f172a")
    .child(ScrollVertical::new(page).key("glass-page").size(FILL, FILL))
    .child(controls)
    .into()
}
