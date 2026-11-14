//! A pagination: which pages it draws wherever the current one is, its buttons against the board, what a press
//! and the keys do, the compact form with a select of how many to a page, and a theme's rule and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  node::color::Color,
  ui::{
    PageItem::{self, Ellipsis, Page},
    Pagination, PaginationPart, PaginationRenderer, PaginationView, Select,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn pages(numbers: impl IntoIterator<Item = usize>) -> Vec<PageItem> {
  numbers.into_iter().map(Page).collect()
}

/// A pagination of `count` pages bound to `page`, with the id `p`, on a stage.
fn stage_of(page: &Signal<usize>, count: usize) -> Stage {
  let page = page.clone();
  Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Pagination::new(page.clone(), count).id("p"))
      .into()
  })
}

#[test]
fn every_page_is_drawn_while_they_fit() {
  for current in [1, 2, 3] {
    assert_eq!(
      Pagination::pages_shown(current, 3, 1),
      pages(1..=3),
      "3 pages, on {current}"
    );
  }
  // Seven is as many as one sibling on each side makes room for: the first, the last, three and two ellipses.
  for current in [1, 4, 7] {
    assert_eq!(
      Pagination::pages_shown(current, 7, 1),
      pages(1..=7),
      "7 pages, on {current}"
    );
  }
  assert_eq!(Pagination::pages_shown(1, 1, 1), pages([1]));
  assert_eq!(Pagination::pages_shown(1, 0, 1), []);
  assert_eq!(Pagination::pages_shown(0, 0, 1), [], "no pages have no current one");
}

#[test]
fn of_many_pages_the_first_the_last_and_the_current_with_its_siblings_are_drawn() {
  assert_eq!(
    Pagination::pages_shown(1, 20, 1),
    [Page(1), Page(2), Page(3), Page(4), Page(5), Ellipsis, Page(20)],
    "on the first, the pages run on from it"
  );
  assert_eq!(
    Pagination::pages_shown(10, 20, 1),
    [Page(1), Ellipsis, Page(9), Page(10), Page(11), Ellipsis, Page(20)]
  );
  assert_eq!(
    Pagination::pages_shown(20, 20, 1),
    [Page(1), Ellipsis, Page(16), Page(17), Page(18), Page(19), Page(20)],
    "on the last, the pages run on to it"
  );
  // Two siblings: nine things.
  assert_eq!(
    Pagination::pages_shown(10, 20, 2),
    [
      Page(1),
      Ellipsis,
      Page(8),
      Page(9),
      Page(10),
      Page(11),
      Page(12),
      Ellipsis,
      Page(20)
    ]
  );
  // No siblings: the board's row, 1 2 3 … 12 on the first of twelve.
  assert_eq!(
    Pagination::pages_shown(1, 12, 0),
    [Page(1), Page(2), Page(3), Ellipsis, Page(12)]
  );
  // A current page past the ends is the end it is past.
  assert_eq!(Pagination::pages_shown(0, 20, 1), Pagination::pages_shown(1, 20, 1));
  assert_eq!(Pagination::pages_shown(99, 20, 1), Pagination::pages_shown(20, 20, 1));
}

#[test]
fn the_row_has_as_many_things_wherever_the_current_page_is_and_an_ellipsis_stands_for_two_pages_or_more() {
  for (count, siblings) in [(8, 1), (12, 0), (20, 1), (20, 2), (100, 1)] {
    for current in 1..=count {
      let shown = Pagination::pages_shown(current, count, siblings);
      assert_eq!(
        shown.len(),
        2 * siblings + 5,
        "{count} pages, {siblings} siblings, on {current}"
      );
      assert!(shown.contains(&Page(current)), "the current one is drawn");
      assert_eq!((shown[0], shown[shown.len() - 1]), (Page(1), Page(count)));
      for (index, item) in shown.iter().enumerate() {
        if *item == Ellipsis {
          let (Page(before), Page(after)) = (shown[index - 1], shown[index + 1]) else {
            panic!("an ellipsis is between two pages");
          };
          assert!(after - before > 2, "{before} … {after}: two pages or more left out");
        }
      }
    }
  }
}

#[test]
fn the_pages_are_quiet_buttons_of_36_four_apart_and_the_current_one_has_an_edge() {
  let page = Signal::new(10_usize);
  let mut stage = stage_of(&page, 20);
  for drawn in ["1", "9", "10", "11", "20"] {
    assert!(stage.paints(drawn), "{drawn} is drawn");
  }
  for left_out in ["2", "8", "12", "19"] {
    assert!(!stage.paints(left_out), "{left_out} is left out");
  }
  let (x, _, width, height) = stage.bounds("p-page-9");
  assert_eq!((width, height), (36.0, 36.0));
  assert_eq!(stage.bounds("p-page-10").0 - x, 40.0, "4 apart");
  assert_eq!(stage.bounds("p-previous").2, 36.0);
  // 1, an ellipsis of 36 and 9, each 4 from the next.
  assert_eq!(x - stage.bounds("p-page-1").0, 80.0);

  let edge = stage
    .border("p-page-10")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .expect("the current page has an edge");
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Input)));
  assert_eq!(stage.fill_color("p-page-10"), Some(light(PaletteColor::Background)));
  assert!(stage.border("p-page-11").is_none(), "another page has none");
  assert_eq!(stage.fill_color("p-page-11"), None, "and no fill");
  assert_eq!(stage.text("11"), (light(PaletteColor::Foreground), 13.0));

  stage.hover("p-page-11");
  assert_eq!(stage.fill_color("p-page-11"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn the_row_does_not_change_its_length_as_the_current_page_moves() {
  let page = Signal::new(1_usize);
  let mut stage = stage_of(&page, 20);
  let width = stage.bounds("p").2;
  // 7 pages and ellipses between previous and next, 36 each and 4 apart.
  assert_eq!(width, 9.0 * 36.0 + 8.0 * 4.0);
  for current in [4, 5, 10, 16, 17, 20] {
    page.set(current);
    stage.pass();
    assert_eq!(stage.bounds("p").2, width, "on {current}");
  }
}

#[test]
fn a_press_goes_to_its_page_and_previous_and_next_are_disabled_at_the_ends() {
  let page = Signal::new(1_usize);
  let mut stage = stage_of(&page, 20);
  stage.click("p-previous");
  assert_eq!(page.get(), 1, "there is no page before the first");
  stage.click("p-next");
  assert_eq!(page.get(), 2);
  stage.click("p-page-20");
  assert_eq!(page.get(), 20);
  assert!(stage.paints("16"));
  stage.click("p-next");
  assert_eq!(page.get(), 20, "there is no page after the last");
  stage.click("p-previous");
  assert_eq!(page.get(), 19);
  stage.click("p-page-1");
  assert_eq!(page.get(), 1);
}

#[test]
fn of_one_page_and_of_none_previous_and_next_take_nothing() {
  let page = Signal::new(1_usize);
  let mut stage = stage_of(&page, 1);
  assert!(stage.paints("1"));
  stage.click("p-next");
  stage.click("p-previous");
  assert_eq!(page.get(), 1);

  let none = Signal::new(1_usize);
  let mut stage = stage_of(&none, 0);
  assert!(!stage.paints("1"), "no pages are drawn");
  assert_eq!(stage.bounds("p").2, 2.0 * 36.0 + 4.0, "previous and next");
  stage.click("p-next");
  stage.click("p-previous");
  assert_eq!(none.get(), 1);
}

#[test]
fn tab_reaches_a_page_and_the_arrows_move_between_them() {
  let page = Signal::new(1_usize);
  let mut stage = stage_of(&page, 20);
  // Previous is disabled on the first page: Tab passes it by, to the current one.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowRight");
  stage.key("Enter");
  assert_eq!(page.get(), 2);
  stage.key("ArrowRight");
  stage.key(" ");
  assert_eq!(page.get(), 3);
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  stage.key("Enter");
  assert_eq!(page.get(), 1);
}

#[test]
fn the_focus_stays_in_the_pagination_when_the_button_it_is_on_is_disabled() {
  let page = Signal::new(2_usize);
  let mut stage = stage_of(&page, 3);
  // Tab reaches previous, and End the last stop, which is next.
  stage.key("Tab");
  stage.key("End");
  stage.key("Enter");
  assert_eq!(page.get(), 3);
  // Next is disabled on the last page. The focus is on the page before it, the third: Left and Enter choose
  // the second, where a focus that was lost would have had the keys do nothing.
  stage.pass();
  stage.key("ArrowLeft");
  stage.key("Enter");
  assert_eq!(page.get(), 2);
}

#[test]
fn a_disabled_pagination_takes_nothing() {
  let page = Signal::new(5_usize);
  let read = page.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Pagination::new(read.clone(), 20).disabled(true).id("p"))
      .into()
  });
  stage.click("p-next");
  stage.click("p-page-1");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(page.get(), 5);
}

#[test]
fn the_compact_form_says_which_items_are_shown_with_previous_and_next_edged() {
  let page = Signal::new(1_usize);
  let read = page.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(Pagination::for_items(read.clone(), 284, 20).compact(true).id("p"))
      .into()
  });
  assert!(stage.paints("1–20 of 284"));
  assert_eq!(stage.text("1–20 of 284"), (light(PaletteColor::Foreground), 13.0));
  assert!(!stage.paints("2"), "no pages");
  let (x, _, width, height) = stage.bounds("p-previous");
  assert_eq!((width, height), (30.0, 30.0));
  assert_eq!(stage.bounds("p-next").0 - x, 34.0, "4 apart");
  assert!(stage.border("p-next").is_some(), "with an edge");
  let summary = stage.text_quad("1–20 of 284").unwrap();
  assert!(
    (x - (summary.x + summary.width) - 12.0).abs() < 0.5,
    "12 after where the list is"
  );

  stage.click("p-next");
  assert_eq!(page.get(), 2);
  assert!(stage.paints("21–40 of 284"));
  page.set(15);
  stage.pass();
  assert!(stage.paints("281–284 of 284"), "the last page has what is left");
  stage.click("p-next");
  assert_eq!(page.get(), 15);

  // Of pages and not of items it says which page.
  let stage = Stage::new(|_| {
    Row::new()
      .child(Pagination::controlled(3, 12, |_| {}).compact(true))
      .into()
  });
  assert!(stage.paints("Page 3 of 12"));
}

#[test]
fn with_a_select_of_how_many_to_a_page_it_is_as_wide_as_what_holds_it() {
  let (page, size) = (Signal::new(1_usize), Signal::new(20_usize));
  let (read, sized) = (page.clone(), size.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Pagination::for_items(read.clone(), 284, sized.get())
          .compact(true)
          .page_size(
            "Rows per page",
            Select::new(sized.clone(), [(10, "10"), (20, "20"), (50, "50")]).id("size"),
          )
          .id("p"),
      )
      .into()
  });
  assert_eq!(stage.bounds("p").2, 400.0);
  let (_, _, width, height) = stage.bounds("size");
  assert_eq!((width, height), (72.0, 30.0), "the board's select");
  assert_eq!(
    stage.text("Rows per page"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let label = stage.text_quad("Rows per page").unwrap();
  assert!((stage.bounds("size").0 - (label.x + label.width) - 12.0).abs() < 0.5);
  let (x, _, width, _) = stage.bounds("p-next");
  assert_eq!(x + width, 400.0, "the buttons are at the other end");

  size.set(50);
  stage.pass();
  assert!(stage.paints("1–50 of 284"));
}

#[test]
fn a_themes_rule_restyles_the_pages() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Pagination>(|rules| rules.base(PaginationPart::Item, |s| s.min_width(44.0)));
  let page = Signal::new(1_usize);
  let read = page.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new().child(Pagination::new(read.clone(), 5).id("p")).into()
  });
  assert_eq!(stage.bounds("p-page-2").2, 44.0);
}

#[test]
fn a_theme_gives_the_pagination_another_anatomy_and_it_still_goes_to_pages() {
  let app = App::new();
  // Where the list is, in words, between previous and next.
  let renderer: PaginationRenderer = Arc::new(|view: PaginationView| {
    Row::new()
      .child(view.previous())
      .child(
        view
          .part(PaginationPart::Summary)
          .text(&format!("{} / {}", view.current, view.pages)),
      )
      .child(view.next())
      .into()
  });
  app.theme().set_renderer::<Pagination>(renderer);
  let page = Signal::new(3_usize);
  let read = page.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new().child(Pagination::new(read.clone(), 12).id("p")).into()
  });
  assert!(stage.paints("3 / 12"));
  assert!(!stage.paints("4"));
  stage.click("p-next");
  assert_eq!(page.get(), 4);
  assert!(stage.paints("4 / 12"));
}
