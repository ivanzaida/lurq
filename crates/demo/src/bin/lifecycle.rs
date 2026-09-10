//! Run with `cargo run -p demo --bin lifecycle --features mcp`.
use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    Accelerator, App, ApplicationMenu, CloseRequest, Menu, MenuAction, MenuBar, Tree, component::Component, ctx::Ctx,
    wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow,
  },
  components::{
    Button, Checkbox, ChromeTitleBar, Column, Modal, Rect, Root, Row, Stack, Text, WindowChrome, WindowControls,
  },
  core::Signal,
  node::{Element, dimension::Dimension},
};

struct Lifecycle {
  dirty: Signal<bool>,
  dialog: Signal<bool>,
  last: Signal<String>,
  pending: Arc<Mutex<Option<CloseRequest>>>,
}
impl Component for Lifecycle {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    let dirty = ctx.signal(false);
    let dialog = ctx.signal(false);
    let last = ctx.signal("None".to_owned());
    let pending: Arc<Mutex<Option<CloseRequest>>> = Arc::new(Mutex::new(None));
    let (dirty_close, dialog_close, pending_close) = (dirty.clone(), dialog.clone(), pending.clone());
    ctx.window().on_close_requested(move |request| {
      if dirty_close.get_untracked() {
        *pending_close.lock().unwrap() = Some(request);
        dialog_close.set(true);
      } else {
        request.proceed();
      }
    });
    let menus = ctx.app_ref().menu_controller();
    let menu_dirty = dirty.clone();
    ctx.on_effect(move || {
      menus.set(MenuBar {
        application: ApplicationMenu {
          name: "lurq lifecycle".into(),
          about: Some(MenuAction::new("about", "About lurq lifecycle")),
          preferences: Some(MenuAction::new("preferences", "Preferences…").accelerator(Accelerator::command(","))),
          ..Default::default()
        },
        menus: vec![
          Menu {
            title: "File".into(),
            items: vec![
              MenuAction::new("new", "New")
                .accelerator(Accelerator::command("n"))
                .into(),
              MenuAction::new("save", "Save")
                .accelerator(Accelerator::command("s"))
                .enabled(menu_dirty.get())
                .into(),
              MenuAction::new("close", "Close Window")
                .accelerator(Accelerator::command("w"))
                .into(),
            ],
          },
          Menu {
            title: "Edit".into(),
            items: vec![MenuAction::new("dirty", "Mark Dirty").into()],
          },
          Menu {
            title: "Help".into(),
            items: vec![MenuAction::new("help", "Lifecycle Help").into()],
          },
        ],
      });
    });
    let (window, opener, menu_last, dirty_action) = (ctx.window(), ctx.window_opener(), last.clone(), dirty.clone());
    ctx.app_ref().on_menu_activate(move |id| {
      menu_last.set(id.to_owned());
      match id {
        "quit" | "close" => window.request_close(),
        "save" => dirty_action.set(false),
        "dirty" => dirty_action.set(true),
        "preferences" => opener.open("Preferences", 420, 220, |app, tree| {
          tree.mount_root::<Preferences>(app, ());
        }),
        _ => {}
      }
    });
    Self {
      dirty,
      dialog,
      last,
      pending,
    }
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let window = ctx.window();
    let drawn_close = window.clone();
    let opener = ctx.window_opener();
    let (cancel_pending, cancel_dialog) = (self.pending.clone(), self.dialog.clone());
    let (confirm_pending, confirm_dialog) = (self.pending.clone(), self.dialog.clone());
    let content = Stack::new()
      .child(
        Column::new()
          .padding(28.0)
          .spacing(20.0)
          .child(Text::new("Window lifecycle and native menu demo"))
          .child(
            Row::new()
              .spacing(12.0)
              .child(Checkbox::new(self.dirty.clone()).id("dirty"))
              .child(Text::new(
                "Unsynced changes (Save is enabled in the macOS menu while checked)",
              )),
          )
          .child(Text::new(&format!("Last menu activation: {}", self.last.get())).id("last-menu"))
          .child(
            Button::new("Request close")
              .id("request-close")
              .padding(12.0)
              .on_click(move |_| window.request_close()),
          )
          .child(
            Button::new("Open Preferences")
              .id("open-preferences")
              .padding(12.0)
              .on_click(move |_| {
                opener.open("Preferences", 420, 220, |app, tree| {
                  tree.mount_root::<Preferences>(app, ());
                });
              }),
          )
          .child(Text::new(
            "Try Alt+F4 / taskbar Close, or macOS ⌘Q / Dock Quit. Cancel keeps this window.",
          )),
      )
      .child(
        Modal::new(
          Stack::new()
            .width(Dimension::Pct(100.0))
            .height(Dimension::Pct(100.0))
            .stack_align(lurq::layout::StackAlignment::Center)
            .child(
              Rect::new(Dimension::Pct(100.0), Dimension::Pct(100.0))
                .background("#101827")
                .opacity(0.5),
            )
            .child(
              Column::new()
                .id("dialog-close-window")
                .width(420.0)
                .rounded(12.0)
                .padding(32.0)
                .spacing(20.0)
                .background("#ffffff")
                .child(Text::new("Close with unsynced changes?"))
                .child(
                  Button::new("Cancel")
                    .background("#edf1f8")
                    .rounded(6.0)
                    .id("dialog-cancel")
                    .padding(12.0)
                    .on_click(move |_| {
                      if let Some(request) = cancel_pending.lock().unwrap().take() {
                        request.cancel();
                      }
                      cancel_dialog.set(false);
                    }),
                )
                .child(
                  Button::new("Close anyway")
                    .background("#fdd8d8")
                    .rounded(6.0)
                    .id("dialog-confirm")
                    .padding(12.0)
                    .on_click(move |_| {
                      if let Some(request) = confirm_pending.lock().unwrap().take() {
                        request.proceed();
                      }
                      confirm_dialog.set(false);
                    }),
                ),
            ),
        )
        .open(self.dialog.clone())
        .target(Root),
      )
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .background("#f4f6fb");
    WindowChrome::new()
      .title_bar(
        ChromeTitleBar::new()
          .title(Text::styled(
            "lurq lifecycle",
            lurq::layout::text_style::TextStyle {
              color: lurq::node::color::Color::from_hex("#f0f3ff"),
              ..Default::default()
            },
          ))
          .controls(WindowControls::new().on_close(move || drawn_close.request_close())),
      )
      .content(content)
      .mount(ctx)
  }
}
struct Preferences;
impl Component for Preferences {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    ctx.window().on_close_requested(CloseRequest::proceed);
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let window = ctx.window();
    Column::new()
      .padding(24.0)
      .spacing(16.0)
      .background("#f4f6fb")
      .child(Text::new("Preferences has no unsynced state."))
      .child(
        Button::new("Close Preferences")
          .id("close-preferences")
          .padding(12.0)
          .on_click(move |_| window.request_close()),
      )
  }
}
fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  lurq::app::devtools::load_fonts(&mut app);
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  #[cfg(feature = "mcp")]
  let _mcp = tree.enable_mcp(
    lurq::mcp::McpConfig::new()
      .app_name("lurq-lifecycle")
      .scopes([lurq::mcp::Scope::Observe, lurq::mcp::Scope::Interact]),
  );
  tree.mount_root::<Lifecycle>(&mut app, ());
  WinitWindow::new(app, tree)
    .with_title("lurq lifecycle")
    .with_size(820, 450)
    .with_decorations(false)
    .run();
}
