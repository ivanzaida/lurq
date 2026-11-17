//! The page of the engine's router: routes a path is matched against, the screen of the route it matches in an
//! outlet, links that push a path, and a history to go back and forward through. Its playground is a small app of
//! the page's own, a router that is not the gallery's, with a layout, params, a nested layout, a guard and a
//! fallback, under a bar that says the path it is at; its sections are the patterns a path is matched by, a stack
//! of screens, going back and forward from every source (with a dialog that takes it), and tabs that are each a
//! router.
//!
//! The demo had no page of its own for these: its shell was a router, its sidebar links, and its pages an outlet.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{HistoryEvent, NavigationCommand, NavigationSource},
    theme::{PaletteColor, RadiusSize},
  },
  components::{
    Column, Link, NavigationBar, NavigationItem, Outlet, Router, Row, StackOutlet, StackOutletProps, TabBar, TabItem,
    TabOutlet, TabOutletProps, Tabs,
  },
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  router::{GuardAction, RouterHandle, Routes},
  ui::{Button, ButtonSize, ButtonVariant, Dialog, icons},
};

use super::kit::{named, samples, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, segmented, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The messages of the playground's inbox: `/inbox/1` is the first.
const MESSAGES: [(&str, &str); 3] = [
  ("Ada", "The engine's notes"),
  ("Grace", "A compiler for everyone"),
  ("Edsger", "On the cruelty of teaching"),
];

/// The books of the stack's library: `/book/1` is the first.
const BOOKS: [&str; 3] = [
  "The Analytical Engine",
  "Structured Programming",
  "The Art of Programming",
];

/// The keys of the tabs, in order, with what each is called.
const TAB_KEYS: [(&str, usize); 3] = [("Library", 0), ("Search", 1), ("Account", 2)];
const TAB_NAMES: [&str; 3] = ["library", "search", "account"];

/// The paths the section on patterns resolves, one under another.
const PROBES: [&str; 7] = [
  "/",
  "/users/settings",
  "/users/42",
  "/files/readme",
  "/docs/guide/routing/links",
  "/files/a/b",
  "/nowhere",
];

/// How many of the commands the section on going back heard it shows, the latest first.
const HEARD: usize = 4;

/// What a source of going back or forward is called in the section's log.
fn source_name(source: NavigationSource) -> &'static str {
  match source {
    NavigationSource::Key => "a key",
    NavigationSource::Mouse => "the mouse's button",
    NavigationSource::Swipe => "a trackpad's swipe",
    NavigationSource::Edge => "a finger from the edge",
    NavigationSource::Browser => "the browser",
    NavigationSource::App => "the app",
    _ => "another source",
  }
}

/// Writes `line` at the top of the log, which keeps the latest few.
fn heard_line(heard: &Signal<Vec<String>>, line: String) {
  heard.update(|lines| {
    lines.insert(0, line);
    lines.truncate(HEARD);
  });
}

/// The routes of the app of the section on going back: a shell with its own way back and forward and a dialog, and
/// three screens one after another.
fn travel_routes(heard: Signal<Vec<String>>, open: Signal<bool>) -> Routes {
  Routes::new().layout(
    "/",
    move |ctx| travel_shell(ctx, &heard, &open),
    |routes| {
      routes
        .route("/", |ctx| travel_screen(ctx, "Home", Some(("Photos", "/photos"))))
        .route("/photos", |ctx| {
          travel_screen(ctx, "Photos", Some(("A photo", "/photos/1")))
        })
        .route("/photos/1", |ctx| travel_screen(ctx, "A photo", None))
    },
  )
}

/// A screen of the section's app: its title, and a button to the next one.
fn travel_screen(ctx: &mut Ctx, title: &str, next: Option<(&str, &str)>) -> Element {
  let mut screen = Column::new()
    .width(FILL)
    .padding(16.0)
    .spacing(12.0)
    .child(text(title, 16.0, FontWeight::SemiBold));
  if let Some((label, to)) = next {
    screen = screen.child(push_button(ctx, label, to.to_owned()));
  } else {
    screen = screen.child(
      text(
        "The last screen. Go back: Alt+← (Cmd+[ on a Mac), the mouse's back button, a trackpad's swipe to the right.",
        13.0,
        FontWeight::Normal,
      )
      .width(FILL),
    );
  }
  screen.into()
}

/// The shell of the section's app: its way back and forward, which run the command as a key does, where it is,
/// a button that opens a dialog which closes on going back, and the screen. It hears every command on its way out
/// to the router, and passes it on.
fn travel_shell(ctx: &mut Ctx, heard: &Signal<Vec<String>>, open: &Signal<bool>) -> Element {
  let path = ctx.route_path();
  let (can_back, can_forward) = ctx
    .navigator()
    .map(|navigator| (navigator.can_back(), navigator.can_forward()))
    .unwrap_or_default();
  let history = ctx.history_runner();
  let (back, forward) = (history.clone(), history);
  let opens = open.clone();
  let bar = Row::new()
    .width(FILL)
    .height(44.0)
    .padding_horizontal(8.0)
    .spacing(4.0)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(
      Button::new("Back")
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .icon(icons::ARROW_LEFT)
        .icon_only(true)
        .disabled(!can_back)
        .on_click(move |_| back.back()),
    )
    .child(
      Button::new("Forward")
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .icon(icons::ARROW_RIGHT)
        .icon_only(true)
        .disabled(!can_forward)
        .on_click(move |_| forward.forward()),
    )
    .child(Row::new().flex(1.0).min_width(0.0).child(mono(&path, 12.0).nowrap()))
    .child(
      Button::new("Dialog")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| opens.set(true)),
    );
  let hears = |name: &'static str| {
    let heard = heard.clone();
    move |event: HistoryEvent| {
      heard_line(
        &heard,
        format!("on_history_{name} from {}", source_name(event.source())),
      );
      event.pass_on();
    }
  };
  let (closes, shuts, told) = (open.clone(), open.clone(), heard.clone());
  let dialog = Dialog::new(open.clone())
    .title("A dialog that takes going back")
    .close_button(false)
    .child(
      Column::new()
        .width(FILL)
        .spacing(16.0)
        .on_history_back(move |event: HistoryEvent| {
          closes.set(false);
          heard_line(
            &told,
            format!("the dialog closed on going back from {}", source_name(event.source())),
          );
        })
        .child(
          text(
            "Go back now: the dialog closes and the page stays where it is. Escape closes it too.",
            14.0,
            FontWeight::Normal,
          )
          .width(FILL),
        )
        .child(
          Row::new()
            .width(FILL)
            .justify(lurq::layout::layout_kind::Justify::End)
            .child(
              Button::new("Close")
                .size(ButtonSize::Sm)
                .on_click(move |_| shuts.set(false)),
            ),
        ),
    );
  Column::new()
    .size(FILL, FILL)
    .on_history_back(hears("back"))
    .on_history_forward(hears("forward"))
    .child(bar)
    .child(Column::new().width(FILL).flex(1.0).child(Outlet::mount(ctx)))
    .child(dialog)
    .into()
}

/// A screen's title and what it says under it.
fn screen(title: &str, body: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(6.0)
    .child(text(title, 16.0, FontWeight::SemiBold))
    .child(
      text(body, 13.0, FontWeight::Normal)
        .color(PaletteColor::MutedForeground)
        .width(FILL),
    )
}

/// A link of the engine dressed in the theme's colors: it has no look of its own. The one whose place the router is
/// at is marked.
fn link(ctx: &mut Ctx, label: &str, to: &str, current: bool) -> Element {
  let label = text(
    label,
    13.0,
    if current {
      FontWeight::SemiBold
    } else {
      FontWeight::Medium
    },
  )
  .color(if current {
    PaletteColor::Foreground
  } else {
    PaletteColor::Primary
  });
  let mut link = Link::build_empty(ctx, to)
    .padding_horizontal(10.0)
    .padding_vertical(5.0)
    .rounded(RadiusSize::Sm)
    .child(label);
  if current {
    link = link.background(PaletteColor::Muted);
  }
  link.into()
}

/// The playground's routes: a shell around every screen, with an inbox whose messages are params, a nested layout
/// of settings, an account behind a guard, and a fallback. `signed_in` is what the guard reads.
fn app_routes(signed_in: Signal<bool>) -> Routes {
  let guarded = signed_in.clone();
  Routes::new().layout("/", shell, move |routes| {
    routes
      .route("/", |_| {
        screen(
          "Home",
          "The route at /. A link above pushes its path on the router; Back and Forward move through what it pushed.",
        )
        .into()
      })
      .route("/inbox", |ctx| {
        let mut list = Column::new()
          .width(FILL)
          .spacing(4.0)
          .child(text("Inbox", 16.0, FontWeight::SemiBold));
        for (index, (from, subject)) in MESSAGES.iter().enumerate() {
          let to = format!("/inbox/{}", index + 1);
          list = list.child(
            Link::build_empty(ctx, to).width(FILL).padding_vertical(4.0).child(
              Row::new()
                .width(FILL)
                .spacing(8.0)
                .child(text(from, 13.0, FontWeight::SemiBold).flex_shrink(0.0))
                .child(
                  Column::new()
                    .flex(1.0)
                    .min_width(0.0)
                    .child(text(subject, 13.0, FontWeight::Normal).color(PaletteColor::Primary)),
                ),
            ),
          );
        }
        list.into()
      })
      .route("/inbox/:id", |ctx| {
        let params = ctx.route_params();
        let id = params.get("id").unwrap_or("").to_owned();
        let found = params
          .get_parsed::<usize>("id")
          .and_then(|number| number.checked_sub(1))
          .and_then(|index| MESSAGES.get(index));
        let body = match found {
          Some((from, subject)) => format!("From {from}: {subject}. The id {id} is a param of /inbox/:id."),
          None => {
            format!("No message has the id {id}: the route matched, and the screen read a param it has no message for.")
          }
        };
        Column::new()
          .width(FILL)
          .spacing(10.0)
          .child(screen(&format!("Message {id}"), &body))
          .child(link(ctx, "All messages", "/inbox", false))
          .into()
      })
      .layout("/settings", settings, |routes| {
        routes
          .route("/", |_| {
            screen("General", "The nested layout's own index: /settings.").into()
          })
          .route("/profile", |_| {
            screen(
              "Profile",
              "/settings/profile: the settings' shell stays, and its outlet shows this.",
            )
            .into()
          })
      })
      .route("/account", |_| {
        screen(
          "Account",
          "The guard let the router in: the switch says you are signed in.",
        )
        .into()
      })
      .guard(move |_| {
        if guarded.get_untracked() {
          GuardAction::Allow
        } else {
          GuardAction::Redirect("/login".to_owned())
        }
      })
      .route("/login", move |ctx| {
        let (signed, navigator) = (signed_in.clone(), ctx.navigator());
        Column::new()
          .width(FILL)
          .spacing(10.0)
          .child(screen(
            "Sign in",
            "The guard of /account sent the router here, in place of where the link went.",
          ))
          .child(
            Row::new().child(Button::new("Sign in").size(ButtonSize::Sm).on_click(move |_| {
              signed.set(true);
              if let Some(navigator) = &navigator {
                navigator.replace("/account");
              }
            })),
          )
          .into()
      })
      .fallback(|ctx| {
        let path = ctx.route_path();
        Column::new()
          .width(FILL)
          .spacing(10.0)
          .child(screen(
            "Not found",
            &format!("No route matches {path}: the fallback shows this, in the shell."),
          ))
          .child(link(ctx, "Home", "/", false))
          .into()
      })
  })
}

/// The shell of the playground's app: its links, and the outlet the matched route is shown in.
fn shell(ctx: &mut Ctx) -> Element {
  let path = ctx.route_path();
  let at = |to: &str| {
    if to == "/" {
      path == "/"
    } else {
      path == to || path.starts_with(&format!("{to}/"))
    }
  };
  let mut links = Row::new()
    .width(FILL)
    .spacing(4.0)
    .wrap()
    .align_items(Alignment::Center);
  for (label, to) in [
    ("Home", "/"),
    ("Inbox", "/inbox"),
    ("Settings", "/settings"),
    ("Account", "/account"),
    ("Lost", "/lost"),
  ] {
    let current = at(to);
    links = links.child(link(ctx, label, to, current));
  }
  Column::new()
    .size(FILL, FILL)
    .child(
      Column::new()
        .width(FILL)
        .padding(8.0)
        .border_bottom(line())
        .child(links),
    )
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .padding(16.0)
        .child(Outlet::mount(ctx)),
    )
    .into()
}

/// The layout of the settings, inside the shell's: links of its own, and an outlet of its own.
fn settings(ctx: &mut Ctx) -> Element {
  let path = ctx.route_path();
  let general = link(ctx, "General", "/settings", path == "/settings");
  let profile = link(ctx, "Profile", "/settings/profile", path == "/settings/profile");
  Column::new()
    .width(FILL)
    .spacing(12.0)
    .child(
      Row::new()
        .width(FILL)
        .spacing(4.0)
        .wrap()
        .align_items(Alignment::Center)
        .child(quiet("settings /", 12.0))
        .child(general)
        .child(profile),
    )
    .child(
      Column::new()
        .width(FILL)
        .padding_left(12.0)
        .border_left(line())
        .child(Outlet::mount(ctx)),
    )
    .into()
}

/// What the patterns section resolves paths against: one route of each kind of pattern, and a fallback.
fn pattern_routes() -> Routes {
  let nothing = |_: &mut Ctx| -> Element { Element::new() };
  Routes::new()
    .route("/", nothing)
    .route("/users/settings", nothing)
    .route("/users/:id", nothing)
    .route("/files/*", nothing)
    .route("/docs/**rest", nothing)
    .fallback(nothing)
}

/// What a path resolves to: the pattern of the last route matched, and its params.
fn resolved(routes: &Routes, path: &str) -> String {
  let chain = routes.resolve(path);
  let Some(last) = chain.last() else {
    return "nothing".to_owned();
  };
  let pattern = match last.pattern_raw() {
    "**" => "fallback".to_owned(),
    pattern => pattern.to_owned(),
  };
  let params: Vec<String> = last
    .params()
    .entries()
    .map(|(name, value)| format!("{name} = {value:?}"))
    .collect();
  if params.is_empty() {
    pattern
  } else {
    format!("{pattern}   {}", params.join(", "))
  }
}

/// A screen of a stack: what the bar says over it, and its content, on the background so that the screen under it
/// does not show through while it moves.
fn stack_screen(ctx: &mut Ctx, title: &str, content: impl Into<Element>) -> Element {
  ctx.navigation_item(NavigationItem::new(title));
  let bottom = ctx.safe_area().bottom;
  Column::new()
    .size(FILL, FILL)
    .padding(16.0)
    .padding_bottom(16.0 + bottom)
    .spacing(12.0)
    .background(PaletteColor::Background)
    .child(content)
    .into()
}

/// A button of a screen that pushes `to` on the router the screen is in.
fn push_button(ctx: &mut Ctx, label: &str, to: String) -> Element {
  let navigator = ctx.navigator();
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .size(ButtonSize::Sm)
    .full_width(true)
    .on_click(move |_| {
      if let Some(navigator) = &navigator {
        navigator.push(to.clone());
      }
    })
    .into()
}

/// The routes of a library that is a stack of screens: the books, a book, its notes.
fn library_routes() -> Routes {
  Routes::new().layout(
    "/",
    |ctx| StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new())),
    |routes| {
      routes
        .route("/", |ctx| {
          let mut books = Column::new().width(FILL).spacing(8.0);
          for (index, title) in BOOKS.iter().enumerate() {
            books = books.child(push_button(ctx, title, format!("/book/{}", index + 1)));
          }
          stack_screen(ctx, "Library", books)
        })
        .route("/book/:id", |ctx| {
          let id: usize = ctx.route_params().get_parsed("id").unwrap_or(1);
          let title = BOOKS.get(id.saturating_sub(1)).copied().unwrap_or("A book");
          let notes = push_button(ctx, "Its notes", format!("/book/{id}/notes"));
          stack_screen(
            ctx,
            &format!("Book {id}"),
            Column::new()
              .width(FILL)
              .spacing(12.0)
              .child(text(title, 15.0, FontWeight::SemiBold).width(FILL))
              .child(notes),
          )
        })
        .route("/book/:id/notes", |ctx| {
          let id = ctx.route_params().get("id").unwrap_or("").to_owned();
          stack_screen(
            ctx,
            "Notes",
            text(
              &format!("The notes of book {id}: three screens deep. The bar's way back goes to the one under."),
              13.0,
              FontWeight::Normal,
            )
            .width(FILL),
          )
        })
    },
  )
}

/// The routes of the search tab: a plain outlet, a list of links, and a result.
fn search_routes() -> Routes {
  Routes::new()
    .route("/", |ctx| {
      let mut list =
        Column::new()
          .size(FILL, FILL)
          .padding(16.0)
          .spacing(4.0)
          .child(text("Search", 16.0, FontWeight::SemiBold));
      for term in ["routers", "outlets", "tabs"] {
        list = list.child(link(ctx, term, &format!("/result/{term}"), false));
      }
      list.into()
    })
    .route("/result/:term", |ctx| {
      let term = ctx.route_params().get("term").unwrap_or("").to_owned();
      let back = ctx.navigator();
      Column::new()
        .size(FILL, FILL)
        .padding(16.0)
        .spacing(10.0)
        .child(screen(
          &format!("Found: {term}"),
          "This tab's router is at /result/:term; the others stay where they are.",
        ))
        .child(
          Row::new().child(
            Button::new("Back")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| {
                if let Some(back) = &back {
                  back.back();
                }
              }),
          ),
        )
        .into()
    })
}

pub struct RouterPage {
  frame: Frame,
  playground: Playground,
  /// The playground's router: the page's own, never the gallery's.
  router: RouterHandle,
  /// What the playground's routes resolve to, for the tree under the stage.
  routes: Arc<Routes>,
  /// What the guard of /account reads.
  signed_in: Signal<bool>,
  /// The path the panel pushes or puts in place.
  typed: Signal<String>,
  /// The path the section on patterns resolves.
  probe: Signal<String>,
  patterns: Arc<Routes>,
  /// The router of the stack of screens.
  stack: RouterHandle,
  /// The tabs, each a router.
  tabs: Tabs,
  /// The router of the section on going back and forward, what its app heard, and whether its dialog is open.
  travel: RouterHandle,
  heard: Signal<Vec<String>>,
  dialog: Signal<bool>,
}

impl RouterPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    // Read first: the history is not a signal, and the path says when it changed.
    let path = self.router.path().get();
    let (can_back, can_forward) = (self.router.can_back(), self.router.can_forward());
    let signed_in = self.signed_in.get();
    let router = self.router.clone();
    let build = {
      let router = router.clone();
      let path = path.clone();
      move |ctx: &mut Ctx| -> Element {
        let (back, forward) = (router.clone(), router.clone());
        let bar = Row::new()
          .width(FILL)
          .height(44.0)
          .padding_horizontal(8.0)
          .spacing(4.0)
          .align_items(Alignment::Center)
          .border_bottom(line())
          .child(
            Button::new("Back")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::ARROW_LEFT)
              .icon_only(true)
              .disabled(!can_back)
              .on_click(move |_| {
                back.back();
              }),
          )
          .child(
            Button::new("Forward")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::ARROW_RIGHT)
              .icon_only(true)
              .disabled(!can_forward)
              .on_click(move |_| {
                forward.forward();
              }),
          )
          .child(
            Row::new()
              .flex(1.0)
              .min_width(0.0)
              .height(28.0)
              .padding_horizontal(10.0)
              .align_items(Alignment::Center)
              .rounded(RadiusSize::Md)
              .background(PaletteColor::Muted)
              .child(mono(&path, 12.0).nowrap()),
          );
        Column::new()
          .width(FILL)
          .height(320.0)
          .clip()
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Background)
          .border(Border::inside(1.0, PaletteColor::Border))
          .child(bar)
          .child(
            Column::new()
              .width(FILL)
              .flex(1.0)
              .child(Router::mount(ctx, router.clone())),
          )
          .into()
      }
    };
    {
      let build = build.clone();
      self.playground.window_shows(move |ctx| build(ctx));
    }
    let make = {
      let path = path.clone();
      move || -> Element {
        well()
          .padding(16.0)
          .child(quiet(
            &format!("The router is at {path}: it is shown on the page's stage."),
            12.0,
          ))
          .into()
      }
    };

    let code = vec![
      "// in create: the router of the app, at its first path".to_owned(),
      "let router = ctx.router(".to_owned(),
      "    Routes::new().layout(\"/\", shell, |routes| {".to_owned(),
      "        routes".to_owned(),
      "            .route(\"/\", home)".to_owned(),
      "            .route(\"/inbox\", inbox)".to_owned(),
      "            .route(\"/inbox/:id\", message) // ctx.route_params().get(\"id\")".to_owned(),
      "            .layout(\"/settings\", settings, |routes| routes.route(\"/\", general).route(\"/profile\", profile))"
        .to_owned(),
      "            .route(\"/account\", account)".to_owned(),
      format!(
        "            .guard(move |_| match signed_in.get_untracked() {{ // {}",
        if signed_in { "lets in" } else { "redirects" }
      ),
      "                true => GuardAction::Allow,".to_owned(),
      "                false => GuardAction::Redirect(\"/login\".to_owned()),".to_owned(),
      "            })".to_owned(),
      "            .route(\"/login\", login)".to_owned(),
      "            .fallback(not_found)".to_owned(),
      "    }),".to_owned(),
      ");".to_owned(),
      "router.replace(\"/\");".to_owned(),
      "".to_owned(),
      "// in render".to_owned(),
      "Router::mount(ctx, self.router.clone())".to_owned(),
      "".to_owned(),
      "// the shell: links, and where the route's screen goes".to_owned(),
      "fn shell(ctx: &mut Ctx) -> Element {".to_owned(),
      "    Column::new()".to_owned(),
      "        .child(Link::build(ctx, \"Inbox\", \"/inbox\"))".to_owned(),
      "        .child(Outlet::mount(ctx))".to_owned(),
      "        .into()".to_owned(),
      "}".to_owned(),
    ];

    let chain = self.routes.resolve(&path);
    let mut tree = vec![format!("router   {}   at {path}", self.playground.measured())];
    for (depth, matched) in chain.iter().enumerate() {
      let pattern = match matched.pattern_raw() {
        "**" => "fallback".to_owned(),
        pattern => format!("{pattern:?}"),
      };
      let params: Vec<String> = matched
        .params()
        .entries()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect();
      let kind = if depth + 1 < chain.len() { "layout" } else { "route" };
      tree.push(format!(
        "{}└─ {kind} {pattern}{}",
        "   ".repeat(depth),
        if params.is_empty() {
          String::new()
        } else {
          format!("   {}", params.join(", "))
        }
      ));
    }

    let typed = self.typed.get();
    let (push, replace) = (router.clone(), router.clone());
    let (pushed, replaced) = (typed.clone(), typed.clone());
    let reset = {
      let (router, signed_in, typed) = (router.clone(), self.signed_in.clone(), self.typed.clone());
      Arc::new(move || {
        // Back to the first entry of the history, which is put at the start.
        while router.can_back() {
          router.back();
        }
        router.replace("/");
        signed_in.set(false);
        typed.set("/inbox/2".to_owned());
      })
    };
    let controls = vec![
      prop(
        "push / replace",
        "impl Into<String>",
        Column::new().width(FILL).spacing(8.0).child(field(&self.typed)).child(
          Row::new()
            .width(FILL)
            .spacing(8.0)
            .child(
              Button::new("Push")
                .size(ButtonSize::Sm)
                .on_click(move |_| push.push(pushed.clone())),
            )
            .child(
              Button::new("Replace")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .on_click(move |_| replace.replace(replaced.clone())),
            ),
        ),
      ),
      divider(),
      flag("signed in: the guard of /account", &self.signed_in),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Router",
        make: Arc::new(make),
        shown: Shown::built(move |ctx| build(ctx)),
        fill: true,
        forceable: false,
        facts: format!(
          "back {} · forward {}",
          if can_back { "yes" } else { "no" },
          if can_forward { "yes" } else { "no" }
        ),
        event: "path",
        code,
        tree,
        controls,
        changed: [path != "/", can_back || can_forward, signed_in, typed != "/inbox/2"]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// The section on patterns: a path typed, and the paths of each kind, with the route each resolves to.
  fn patterns(&self) -> Element {
    let probe = self.probe.get();
    let mut list = Column::new().width(FILL).spacing(8.0);
    for path in PROBES {
      list = list.child(named(path, mono(&resolved(&self.patterns, path), 12.0).width(FILL)));
    }
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Column::new()
          .width(320.0)
          .max_width(FILL)
          .spacing(6.0)
          .child(quiet("a path", 12.0))
          .child(field(&self.probe))
          .child(mono(&format!("→ {}", resolved(&self.patterns, &probe)), 12.0).width(FILL)),
      )
      .child(well().padding(12.0).child(list))
      .into()
  }

  /// The section on a stack of screens: its path, a way back, and the stack in a box as large as a phone's screen.
  fn stacked(&self, ctx: &mut Ctx) -> Element {
    let path = self.stack.path().get();
    let can_back = self.stack.can_back();
    let (back, first) = (self.stack.clone(), self.stack.clone());
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(quiet("path", 12.0))
          .child(mono(&path, 12.0))
          .child(
            Button::new("Back")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(!can_back)
              .on_click(move |_| {
                back.back();
              }),
          )
          .child(
            Button::new("Back to the first")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(!can_back)
              .on_click(move |_| {
                while first.can_back() {
                  first.back();
                }
              }),
          ),
      )
      .child(
        Column::new()
          .width(360.0)
          .max_width(FILL)
          .height(380.0)
          .clip()
          .rounded(RadiusSize::Lg)
          .background(PaletteColor::Background)
          .border(Border::inside(1.0, PaletteColor::Border))
          .child(Router::mount(ctx, self.stack.clone())),
      )
      .into()
  }

  /// The section on going back and forward: the app in a box, and what it heard.
  fn traveled(&self, ctx: &mut Ctx) -> Element {
    let heard = self.heard.get();
    let mut log = Column::new().width(FILL).spacing(4.0).child(quiet("Events", 12.0));
    if heard.is_empty() {
      log = log.child(quiet("Nothing yet: go back or forward over the box.", 12.0));
    }
    for line in &heard {
      log = log.child(mono(line, 12.0).width(FILL));
    }
    let keys = [NavigationCommand::Back, NavigationCommand::Forward]
      .map(|command| {
        let accelerator = command.accelerator();
        let m = accelerator.modifiers;
        let held = if m.meta {
          "Cmd+"
        } else if m.alt {
          "Alt+"
        } else {
          ""
        };
        let key = match &*accelerator.key {
          "ArrowLeft" => "←",
          "ArrowRight" => "→",
          key => key,
        };
        format!("{held}{key}")
      })
      .join(" and ");
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(quiet("keys here", 12.0))
          .child(mono(&keys, 12.0)),
      )
      .child(
        Column::new()
          .width(360.0)
          .max_width(FILL)
          .height(260.0)
          .clip()
          .rounded(RadiusSize::Lg)
          .background(PaletteColor::Background)
          .border(Border::inside(1.0, PaletteColor::Border))
          .child(Router::mount(ctx, self.travel.clone())),
      )
      .child(well().padding(12.0).child(log))
      .into()
  }

  /// The section on tabs: which is shown, chosen here or in the bar, and the path of its router.
  fn tabbed(&self, ctx: &mut Ctx) -> Element {
    let active = self.tabs.active();
    let index = TAB_NAMES.iter().position(|name| *name == active).unwrap_or(0);
    let path = self
      .tabs
      .router(&active)
      .map(|router| router.path().get())
      .unwrap_or_default();
    let tabs = self.tabs.clone();
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        samples()
          .align_items(Alignment::Center)
          .child(segmented(&TAB_KEYS, &index, move |index| tabs.select(TAB_NAMES[index])))
          .child(mono(&format!("{active} at {path}"), 12.0)),
      )
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
      .into()
  }
}

impl Component for RouterPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 0);
    let signed_in = ctx.signal(false);
    let router = ctx.router(app_routes(signed_in.clone()));
    router.replace("/");
    playground.watch(ctx, &router.path(), |path| format!("path = {path:?}"));

    let stack = ctx.router(library_routes());
    stack.replace("/");

    let library = ctx.router(library_routes());
    library.replace("/");
    let search = ctx.router(search_routes());
    search.replace("/");
    let account = ctx.router(Routes::new().route("/", |ctx| {
      let bottom = ctx.safe_area().bottom;
      Column::new()
        .size(FILL, FILL)
        .padding(16.0)
        .padding_bottom(16.0 + bottom)
        .child(screen(
          "Account",
          "A tab of one route. Its badge is set with Tabs::set_badge or the item's badge.",
        ))
        .into()
    }));
    account.replace("/");
    let heard = ctx.signal(Vec::<String>::new());
    let dialog = ctx.signal(false);
    let travel = ctx.router(travel_routes(heard.clone(), dialog.clone()));
    travel.replace("/");

    let tabs = Tabs::new();
    tabs
      .add(TAB_NAMES[0], TabItem::new("Library"), library)
      .add(TAB_NAMES[1], TabItem::new("Search"), search)
      .add(TAB_NAMES[2], TabItem::new("Account").badge("2"), account);

    Self {
      frame: Frame::new(ctx),
      playground,
      routes: Arc::new(app_routes(signed_in.clone())),
      router,
      signed_in,
      typed: ctx.signal("/inbox/2".to_owned()),
      probe: ctx.signal("/users/7".to_owned()),
      patterns: Arc::new(pattern_routes()),
      stack,
      tabs,
      travel,
      heard,
      dialog,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "Routes::route",
        "(&str, Fn(&mut Ctx) -> Element)",
        "",
        "A path's pattern and the screen it renders: static segments, :name for a param, * for any one segment, \
         **name for the rest. The most specific pattern wins.",
      ),
      (
        "Routes::layout",
        "(&str, Fn, FnOnce(Routes) -> Routes)",
        "",
        "A shell its child routes are shown in, where it mounts an Outlet. The children match what is left of \
         the path after the layout's own pattern.",
      ),
      (
        "Routes::fallback",
        "Fn(&mut Ctx) -> Element",
        "",
        "What a path no route matches shows. Inside a layout it is shown in the layout's shell.",
      ),
      (
        "Routes::guard",
        "Fn(&RouteMatch) -> GuardAction",
        "",
        "On the route before it: Allow, Deny (the router stays where it is) or Redirect(path), asked at each \
         navigation to it.",
      ),
      (
        "ctx.router",
        "Routes -> RouterHandle",
        "",
        "A router over the routes, at no path until replace or push gives it one. Made once, in create.",
      ),
      (
        "Router::mount",
        "(ctx, RouterHandle)",
        "",
        "Renders the routes the router's path matches, and gives what is in them its navigator.",
      ),
      (
        "Outlet::mount",
        "(ctx)",
        "",
        "In a layout's shell: where the next route of the match is shown.",
      ),
      (
        "Link::build",
        "(ctx, &str, impl Into<String>)",
        "",
        "A label that pushes a path on the router around it when pressed. build_empty takes children of the \
         app's own. It has no look: ui::Link is the design system's.",
      ),
      (
        "push / replace",
        "impl Into<String>",
        "",
        "On the handle or ctx.navigator(): a new entry of the history, or the current one put at another path.",
      ),
      (
        "back / back_by / forward",
        "() / usize / ()",
        "",
        "Moves through the history; can_back and can_forward say whether there is an entry to go to.",
      ),
      (
        "ctx.route_params",
        "Params",
        "",
        "In a route's screen: the params its pattern captured, by name; get_parsed reads one as a number.",
      ),
      (
        "ctx.route_path",
        "String",
        "",
        "The router's path; a render that reads it is run again when the path changes.",
      ),
      (
        "StackOutlet::mount_with",
        "(ctx, StackOutletProps)",
        "",
        "In place of an Outlet: every entry of the history a screen of its own, the one before kept under the \
         one pushed. bar(NavigationBar) draws the bar with the way back.",
      ),
      (
        "on_history_back / on_history_forward",
        "Fn(HistoryEvent)",
        "",
        "Going back or forward on the focused element or one in it, or under the pointer: Alt+← and Alt+→ (Cmd+[ \
         and Cmd+] on a Mac), the mouse's side buttons, a swipe, Android's back, a browser's buttons. A handler with \
         nothing to do calls event.pass_on(); with nothing taking it, the router goes back.",
      ),
      (
        "ctx.history_back / history_forward",
        "()",
        "",
        "Goes back or forward as a key does, from what has the focus: a button of the app's own. \
         ctx.history_runner() keeps it for a handler; WindowHandle has the same.",
      ),
      (
        "Tabs::add",
        "(key, TabItem, RouterHandle)",
        "",
        "A tab of a set, a router with a history of its own. select shows one; TabOutlet::mount shows the active \
         one, with a TabBar if it is given one.",
      ),
    ];
    let playground = self.play(ctx);
    let patterns = self.patterns();
    let stacked = self.stacked(ctx);
    let traveled = self.traveled(ctx);
    let tabbed = self.tabbed(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Router and Outlets"],
        title: "Router and Outlets",
        description: "A router matches its path against routes and shows the screen of the one it matches in an \
                      outlet. Links push a path, and a history goes back and forward. A stack outlet keeps the \
                      screens of the history one over another, and tabs are routers side by side.",
        import: Some(
          "use lurq::{components::{Link, Outlet, Router, StackOutlet, TabOutlet, Tabs}, router::{RouterHandle, \
           Routes}};",
        ),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A small app with a router of its own, under a bar with the way back and forward and the path it is at. \
           Follow the links, open a message (its id is a param), go into the settings, whose layout is in the \
           shell's, and try Account with the guard's switch off and on. Lost goes to a path no route matches.",
          playground,
        ),
        Section::new(
          "Which route a path matches",
          "Each path is resolved against a route of each kind of pattern and a fallback. A static segment wins over \
           a param, a param over *, and ** takes the rest of the path. Type a path to resolve it.",
          self.frame.demo(
            0,
            patterns,
            "routes.resolve(\"/users/42\")",
            &[
              "Routes::new()",
              "    .route(\"/\", home)",
              "    .route(\"/users/settings\", settings) // wins over /users/:id",
              "    .route(\"/users/:id\", user)          // ctx.route_params().get(\"id\")",
              "    .route(\"/files/*\", file)            // one segment, not captured",
              "    .route(\"/docs/**rest\", docs)        // the rest of the path",
              "    .fallback(not_found)",
            ],
          ),
        ),
        Section::new(
          "A stack of screens",
          "A router whose layout is a stack outlet. A push brings the next screen in from the right over the one \
           before, which stays mounted under it; the bar's way back, or Back here, takes the top one out. Back to \
           the first goes back through every entry.",
          self.frame.demo(
            1,
            stacked,
            "StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new()))",
            &[
              "let router = ctx.router(Routes::new().layout(",
              "    \"/\",",
              "    |ctx| StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new())),",
              "    |routes| routes.route(\"/\", library).route(\"/book/:id\", book).route(\"/book/:id/notes\", notes),",
              "));",
              "router.replace(\"/\"); // the screen at the bottom of the stack",
              "",
              "// a screen says what the bar shows over it",
              "ctx.navigation_item(NavigationItem::new(\"Library\"));",
            ],
          ),
        ),
        Section::new(
          "Going back and forward",
          "One command for every way back: Alt+← (Cmd+[ on a Mac), the mouse's back button, a trackpad's swipe or a \
           finger from the left edge, Android's back, a browser's back. It goes from the focused element, or from what \
           is under the pointer, out to the router: the app's shell hears it on the way and passes it on, and the \
           router goes back. Open the dialog and go back: the dialog takes it and the page stays. The bar's arrows \
           run the same command from a handler.",
          self.frame.demo(
            3,
            traveled,
            ".on_history_back(|event: HistoryEvent| event.pass_on())",
            &[
              "// the shell: its own way back runs the command, as Alt+← does",
              "let history = ctx.history_runner();",
              "Button::new(\"Back\").on_click(move |_| history.back())",
              "",
              "// heard on its way out to the router, and passed on",
              "Column::new()",
              "    .on_history_back(|event: HistoryEvent| {",
              "        log(event.source());",
              "        event.pass_on();",
              "    })",
              "    .child(Outlet::mount(ctx))",
              "",
              "// a dialog takes it instead of leaving the page",
              "Dialog::new(open.clone()).child(",
              "    Column::new()",
              "        .on_history_back(move |_| open.set(false))",
              "        .child(content),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Tabs bound to routes",
          "Three tabs, each a router with a history of its own: the library is a stack, the search an outlet of two \
           routes. Go into one, choose another and come back: it is where it was left. A press on the tab that is \
           shown takes it back to its first screen; pressed again there it asks the page under the middle of the \
           window to scroll to its top, which here is the gallery's page.",
          self.frame.demo(
            2,
            tabbed,
            "TabOutlet::mount(ctx, TabOutletProps::new(tabs.clone()).bar(TabBar::new()))",
            &[
              "// in create: a router for each tab",
              "let library = ctx.router(library_routes());",
              "library.replace(\"/\");",
              "let search = ctx.router(search_routes());",
              "search.replace(\"/\");",
              "let tabs = Tabs::new();",
              "tabs",
              "    .add(\"library\", TabItem::new(\"Library\"), library)",
              "    .add(\"search\", TabItem::new(\"Search\"), search)",
              "    .add(\"account\", TabItem::new(\"Account\").badge(\"2\"), account);",
              "",
              "// in render",
              "TabOutlet::mount(ctx, TabOutletProps::new(self.tabs.clone()).bar(TabBar::new()))",
              "tabs.select(\"search\"); // what the segments above do",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What routes are made of, what a router, an outlet and a link are given, and how a screen reads where the \
           router is.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
