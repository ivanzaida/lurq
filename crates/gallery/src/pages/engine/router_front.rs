//! The Router page's "The screen in front": two tabs, the first a stack of two screens, each screen reading
//! whether it is the one in front (`Ctx::is_in_front`), with what each last read beside them. The first screen
//! of the stack plays the gallery's clip while it is in front: its time stands still while its screen is not.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
    theme::{PaletteColor, RadiusSize},
  },
  components::{
    Column, NavigationBar, NavigationItem, Row, StackOutlet, StackOutletProps, TabBar, TabItem, TabOutlet,
    TabOutletProps, Tabs, Text,
  },
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  router::{RouterHandle, Routes},
  ui::{Button, ButtonSize, ButtonVariant},
};

use crate::kit::{FILL, quiet, text};

/// The screens of the demo, by the name each is told under.
const SCREENS: [(&str, &str); 3] = [
  ("watch", "Watch, the stack's first screen"),
  ("over", "The screen pushed over it"),
  ("other", "The other tab's screen"),
];

/// What the screens last read.
#[derive(Default)]
struct Read {
  /// Each screen that has rendered, and whether it read that it is in front.
  front: Vec<(&'static str, bool)>,
}

type Kept = Arc<Mutex<Read>>;

fn held(kept: &Kept) -> std::sync::MutexGuard<'_, Read> {
  kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Clone)]
struct Named {
  name: &'static str,
  kept: Kept,
}

impl PartialEq for Named {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Named {
  fn inspect(&self, formatter: &mut lurq::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(&self.name);
  }
}

/// A screen that reads whether it is in front, says so, and keeps what it read for the list beside the demo.
struct Screen {
  muted: Signal<bool>,
}

impl Component for Screen {
  type Props = Named;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      muted: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let named = ctx.props::<Named>().clone();
    let front = ctx.is_in_front();
    {
      let mut read = held(&named.kept);
      match read.front.iter_mut().find(|(name, _)| *name == named.name) {
        Some(said) => said.1 = front,
        None => read.front.push((named.name, front)),
      }
    }
    let title = match named.name {
      "watch" => "Watch",
      "over" => "Over it",
      _ => "Other",
    };
    ctx.navigation_item(NavigationItem::new(title));
    let bottom = ctx.safe_area().bottom;
    let content = Column::new()
      .size(FILL, FILL)
      .padding(16.0)
      .padding_bottom(16.0 + bottom)
      .spacing(12.0)
      .background(PaletteColor::Background)
      .child(text(
        if front {
          "This screen is in front."
        } else {
          "This screen is not in front."
        },
        14.0,
        FontWeight::SemiBold,
      ));
    // It plays while its screen is in front, and is where it was when the screen is in front again.
    #[cfg(not(target_arch = "wasm32"))]
    let content = if named.name == "watch" {
      content.child(
        lurq::ui::VideoPlayer::new()
          .source(&crate::pages::video_player::clip())
          .playing_controlled(front, |_| {})
          .muted(self.muted.clone())
          .looped(true)
          .width(328.0)
          .max_width(lurq::ui::Length::Fill)
          .id("front-clip")
          .key("front-clip")
          .mount(ctx),
      )
    } else {
      content
    };
    let _ = &self.muted;
    content
  }
}

/// The demo: its tabs and their routers, the controls that move them, and what the screens last read.
pub struct FrontDemo {
  tabs: Tabs,
  stack: RouterHandle,
  kept: Kept,
  /// Counted on by a clock when what the screens read has changed.
  beat: Signal<u64>,
  _clock: Interval,
}

impl Component for FrontDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let kept = Kept::default();
    let screen = |name: &'static str| {
      let kept = kept.clone();
      move |ctx: &mut Ctx| {
        ctx.mount::<Screen>(Named {
          name,
          kept: kept.clone(),
        })
      }
    };
    let stack = ctx.router(Routes::new().layout(
      "/",
      |ctx| StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new())),
      |routes| routes.route("/", screen("watch")).route("/over", screen("over")),
    ));
    stack.replace("/");
    let other = ctx.router(Routes::new().route("/", screen("other")));
    other.replace("/");
    let tabs = Tabs::new();
    tabs
      .add("watch", TabItem::new("Watch"), stack.clone())
      .add("other", TabItem::new("Other"), other);
    let beat = ctx.signal(0_u64);
    let clock = ctx.create_interval(std::time::Duration::from_millis(200), {
      let (kept, beat) = (kept.clone(), beat.clone());
      let seen = Mutex::new(Vec::new());
      move || {
        let now = held(&kept).front.clone();
        let mut seen = seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *seen != now {
          *seen = now;
          beat.update(|count| *count += 1);
        }
      }
    });
    clock.start();
    Self {
      tabs,
      stack,
      kept,
      beat,
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.beat.get();
    let active = self.tabs.active();
    let pushed = self.stack.path().get() == "/over";
    let button = |label: &str, id: &str| {
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .id(id)
    };
    let tab = |label: &str, key: &'static str| {
      let tabs = self.tabs.clone();
      button(label, &format!("front-tab-{key}"))
        .variant(if active == key {
          ButtonVariant::Secondary
        } else {
          ButtonVariant::Outline
        })
        .on_click(move |_| tabs.select(key))
    };
    let stack = self.stack.clone();
    let push = button(if pushed { "Take it off" } else { "Push a screen" }, "front-push").on_click(move |_| {
      if stack.path().get_untracked() == "/over" {
        stack.back();
      } else {
        stack.push("/over");
      }
    });
    let read = held(&self.kept);
    let mut said = Column::new().width(280.0).max_width(FILL).spacing(10.0);
    for (name, label) in SCREENS {
      let front = read.front.iter().find(|(of, _)| *of == name).map(|(_, front)| *front);
      said = said.child(
        Column::new().width(FILL).spacing(2.0).child(quiet(label, 12.0)).child(
          Text::new(match front {
            Some(true) => "in front",
            Some(false) => "not in front",
            None => "not shown yet",
          })
          .id(format!("front-{name}")),
        ),
      );
    }
    drop(read);
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .child(tab("Watch", "watch"))
          .child(tab("Other", "other"))
          .child(push),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(24.0)
          .wrap()
          .align_items(Alignment::Start)
          .child(
            Column::new()
              .width(360.0)
              .max_width(FILL)
              .height(420.0)
              .clip()
              .rounded(RadiusSize::Lg)
              .background(PaletteColor::Background)
              .border(Border::inside(1.0, PaletteColor::Border))
              .child(TabOutlet::mount(
                ctx,
                TabOutletProps::new(self.tabs.clone()).bar(TabBar::new()),
              )),
          )
          .child(said),
      )
  }
}
