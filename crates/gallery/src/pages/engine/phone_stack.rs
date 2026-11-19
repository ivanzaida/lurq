//! A phone's stack of screens over the whole window, opened from the Router page: what a finger does to it is
//! tried where it does it on a phone. A swipe in from the left edge takes the top screen back with it; the list
//! of a screen is pulled down to load anew, and its rows are swiped aside for what can be done with them.

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Modal, Root},
    events::EdgePanEvent,
    theme::PaletteColor,
  },
  components::{
    Column, NavigationBar, NavigationItem, PullToRefresh, PullToRefreshProps, PullToRefreshStyle, Router, Row,
    StackOutlet, StackOutletProps, SwipeAction, SwipeRow, SwipeRowProps,
  },
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border, color::Color},
  router::{RouterHandle, Routes},
  ui::{Button, ButtonSize, ButtonVariant},
};

use crate::kit::{FILL, quiet, text};

/// The stack over the whole window while `open` says so, its router `stack`.
pub fn over_the_window(ctx: &mut Ctx, open: &Signal<bool>, stack: &RouterHandle) -> Element {
  let content = Column::new()
    .size(FILL, FILL)
    .background(PaletteColor::Background)
    // At the first screen there is nothing to go back to: a finger from the left edge goes nowhere, and not back in
    // the gallery's own history under the stack.
    .on_edge_pan(|event: EdgePanEvent| {
      event.prevent_default();
      event.stop_propagation();
    })
    .child(Router::mount(ctx, stack.clone()));
  Modal::new(content)
    .open(open.clone())
    .target(Root)
    .dismiss_on_escape(true)
    .takes_focus(true)
    .into()
}

/// The screens of the stack: a first one that says what to try and closes it, a list, and screens pushed deeper.
pub fn routes(open: Signal<bool>) -> Routes {
  Routes::new().layout(
    "/",
    |ctx| StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new())),
    move |routes| {
      let open = open.clone();
      routes
        .route("/", move |ctx| {
          let close = open.clone();
          ctx.navigation_item(
            NavigationItem::new("A phone's stack").trailing(
              Button::new("Close")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm)
                .on_click(move |_| close.set(false)),
            ),
          );
          let (list, deeper) = (
            push(ctx, "A list to pull and swipe", "/list"),
            push(ctx, "A screen, and one under it", "/deeper/1"),
          );
          screen(
            ctx,
            Column::new()
              .width(FILL)
              .spacing(12.0)
              .child(
                quiet(
                  "Push a screen, then swipe in from the left edge of the screen to take it back: it follows the \
                   finger, and goes back where it is let go far enough. The list is pulled down to load anew, and \
                   its rows are swiped aside.",
                  13.0,
                )
                .width(FILL),
              )
              .child(list)
              .child(deeper),
          )
        })
        .route("/list", |ctx| {
          ctx.navigation_item(NavigationItem::new("A list"));
          ctx.mount::<ListScreen>(())
        })
        .route("/deeper/:depth", |ctx| {
          let depth: usize = ctx.route_params().get_parsed("depth").unwrap_or(1);
          ctx.navigation_item(NavigationItem::new(format!("Screen {depth}")));
          let next = push(ctx, "One more", &format!("/deeper/{}", depth + 1));
          screen(
            ctx,
            Column::new()
              .width(FILL)
              .spacing(12.0)
              .child(quiet(&format!("{depth} screens over the first."), 13.0).width(FILL))
              .child(next),
          )
        })
    },
  )
}

/// A screen of the stack: its content inside the safe area, on the background, so that the screen under it does not
/// show through while it moves.
fn screen(ctx: &mut Ctx, content: impl Into<Element>) -> Element {
  let safe = ctx.safe_area();
  Column::new()
    .size(FILL, FILL)
    .padding(16.0)
    .padding_left(16.0 + safe.left)
    .padding_right(16.0 + safe.right)
    .padding_bottom(16.0 + safe.bottom)
    .background(PaletteColor::Background)
    .child(content)
    .into()
}

/// A button that pushes `to` on the stack the screen is in.
fn push(ctx: &mut Ctx, label: &str, to: &str) -> Element {
  let (navigator, to) = (ctx.navigator(), to.to_owned());
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .full_width(true)
    .on_click(move |_| {
      if let Some(navigator) = &navigator {
        navigator.push(to.clone());
      }
    })
    .into()
}

/// A list pulled down past its top to load anew, with rows swiped aside: left for Delete and Flag, right for Pin.
struct ListScreen {
  rows: Signal<Vec<String>>,
  refreshing: Signal<bool>,
  loads: Signal<u32>,
  did: Signal<String>,
}

impl Component for ListScreen {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      rows: ctx.signal((1..=14).map(|row| format!("Row {row}")).collect()),
      refreshing: ctx.signal(false),
      loads: ctx.signal(0),
      did: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let did = self.did.get();
    let safe = ctx.safe_area();
    let mut content = Column::new()
      .width(FILL)
      .padding_top(8.0)
      .padding_bottom(16.0 + safe.bottom)
      .child(
        Row::new()
          .padding_horizontal(16.0 + safe.left)
          .padding_bottom(12.0)
          .child(
            quiet(
              &format!(
                "Pull the list down to load it anew ({} so far). Swipe a row left for Delete and Flag, right for \
               Pin; all the way left deletes it.{}",
                self.loads.get(),
                if did.is_empty() {
                  String::new()
                } else {
                  format!(" Last: {did}.")
                }
              ),
              13.0,
            )
            .width(FILL),
          ),
      );
    for name in &self.rows.get() {
      let (delete, flag, pin) = (name.clone(), name.clone(), name.clone());
      let (all, did_delete, did_flag, did_pin) =
        (self.rows.clone(), self.did.clone(), self.did.clone(), self.did.clone());
      content = content.child(SwipeRow::mount_keyed(
        ctx,
        name,
        SwipeRowProps::new()
          .trailing([
            SwipeAction::destructive("Delete", move || {
              all.update(|rows| rows.retain(|row| row != &delete));
              did_delete.set(format!("deleted {delete}"));
            }),
            SwipeAction::new("Flag", move || did_flag.set(format!("flagged {flag}")))
              .background(Color::from_hex("#FF9500")),
          ])
          .leading([SwipeAction::new("Pin", move || did_pin.set(format!("pinned {pin}")))
            .background(Color::from_hex("#0A84FF"))]),
        Row::new()
          .width(FILL)
          .height(52.0)
          .align_items(Alignment::Center)
          .padding_horizontal(16.0 + safe.left)
          .background(PaletteColor::Card)
          .border_bottom(Border::inside(1.0, PaletteColor::Border))
          .child(text(name, 15.0, FontWeight::Normal)),
      ));
    }
    // The stack's own list, under the bar, with the spinner standing under the bar.
    let top = StackOutlet::bar_inset(ctx);
    let scroll = StackOutlet::screen_scroll(ctx).unwrap_or_else(lurq::layout::layout_kind::ScrollState::new);
    let list = StackOutlet::large_title_scroll(ctx, content)
      .size(FILL, FILL)
      .background(PaletteColor::Background);
    let (refreshing, loads, all) = (self.refreshing.clone(), self.loads.clone(), self.rows.clone());
    PullToRefresh::mount(
      ctx,
      PullToRefreshProps::new(self.refreshing.clone(), scroll)
        .top(top)
        .style(PullToRefreshStyle::new())
        .on_refresh(move || {
          // What an app would load: here, a wait and one more row on top.
          let (refreshing, loads, all) = (refreshing.clone(), loads.clone(), all.clone());
          std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            loads.update(|loads| *loads += 1);
            let number = loads.get_untracked();
            all.update(|rows| rows.insert(0, format!("Loaded {number}")));
            refreshing.set(false);
          });
        }),
      list,
    )
  }
}
