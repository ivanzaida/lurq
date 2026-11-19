//! The page of the window tabs: their playground in a title bar, the board's
//! strip, each state of a tab, and a window with more tabs than its bar holds.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Interaction, WindowTab, WindowTabs, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

/// The names of the files the tabs hold, by their id.
const FILES: [&str; 12] = [
  "blur.rs",
  "theme.rs",
  "canvas.rs",
  "layout.rs",
  "router.rs",
  "text.rs",
  "motion.rs",
  "forms.rs",
  "charts.rs",
  "tables.rs",
  "media.rs",
  "window.rs",
];

/// What a long label is, where the playground is asked for one.
const LONG: &str = "a_file_whose_name_is_longer_than_its_tab.rs";

/// The window a title bar of tabs is drawn in, as wide as its room up to `width`: the bar, `Muted`, 40 high
/// with the tabs on its bottom edge and the room after them the window is dragged by, over the content the
/// selected tab joins.
fn window(tabs: impl Into<Element>, width: f32, content: &str) -> Column {
  Column::new()
    .width(FILL)
    .max_width(width)
    .rounded(RadiusSize::Lg)
    .border(line())
    .background(PaletteColor::Background)
    .child(
      Row::new()
        .width(FILL)
        .height(40.0)
        .padding_left(8.0)
        .align_items(Alignment::End)
        .background(PaletteColor::Muted)
        .child(tabs)
        // The drag region: what is left of the bar.
        .child(Row::new().flex(1.0).min_width(24.0)),
    )
    .child(
      Column::new()
        .width(FILL)
        .height(90.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(quiet(content, 11.0)),
    )
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(
      Row::new()
        .padding_left(8.0)
        .padding_top(8.0)
        .background(PaletteColor::Muted)
        .child(shown),
    )
    .child(quiet(caption, 12.0))
}

/// What the tabs of `open` do to it and to `selected`: a close takes the tab out and selects the one beside it
/// where it was the selected one, the add button opens the first file not open, a move moves it.
#[derive(Clone)]
struct Files {
  open: Signal<Vec<usize>>,
  selected: Signal<usize>,
}

impl Files {
  fn close(&self, file: usize) {
    let open = self.open.get_untracked();
    let Some(at) = open.iter().position(|open| *open == file) else {
      return;
    };
    let mut rest = open;
    rest.remove(at);
    if self.selected.get_untracked() == file {
      if let Some(next) = rest.get(at.min(rest.len().saturating_sub(1))) {
        self.selected.set(*next);
      }
    }
    self.open.set(rest);
  }

  fn add(&self) -> Option<usize> {
    let open = self.open.get_untracked();
    let file = (0..FILES.len()).find(|file| !open.contains(file))?;
    self.open.update(|open| open.push(file));
    self.selected.set(file);
    Some(file)
  }

  fn reorder(&self, from: usize, to: usize) {
    self.open.update(|open| {
      if from < open.len() && to < open.len() {
        let file = open.remove(from);
        open.insert(to, file);
      }
    });
  }
}

pub struct WindowTabsPage {
  frame: Frame,
  playground: Playground,
  play: Files,
  icons: Signal<bool>,
  dirty: Signal<bool>,
  close_buttons: Signal<bool>,
  add: Signal<bool>,
  shortcuts: Signal<bool>,
  disabled: Signal<bool>,
  disabled_tab: Signal<bool>,
  long: Signal<bool>,
  many: Files,
}

impl WindowTabsPage {
  /// The playground: the tabs in a title bar, as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (with_icons, dirty, close_buttons) = (self.icons.get(), self.dirty.get(), self.close_buttons.get());
    let (add, shortcuts, disabled) = (self.add.get(), self.shortcuts.get(), self.disabled.get());
    let (disabled_tab, long) = (self.disabled_tab.get(), self.long.get());
    let open = self.play.open.get();
    let selected = self.play.selected.get();
    let tabs = move |open: &[usize]| -> Vec<WindowTab<usize>> {
      open
        .iter()
        .map(|file| {
          let name = if long && *file == 0 { LONG } else { FILES[*file] };
          let mut tab = WindowTab::new(*file, name)
            .dirty(dirty && *file == 1)
            .close_button(close_buttons)
            .disabled(disabled_tab && *file == 2);
          if with_icons {
            tab = tab.icon(icons::FILE_CODE);
          }
          tab
        })
        .collect()
    };
    let plain = {
      let open = open.clone();
      move || {
        let mut strip = WindowTabs::controlled(selected, tabs(&open), |_| {})
          .label("Open files")
          .shortcuts(false)
          .disabled(disabled);
        if add {
          strip = strip.on_add(|| {});
        }
        strip
      }
    };

    let record = self.playground.recorder();
    let files = self.play.clone();
    let (on_change, on_close, on_add, on_reorder, on_detach) = (
      (record.clone(), files.clone()),
      (record.clone(), files.clone()),
      (record.clone(), files.clone()),
      (record.clone(), files.clone()),
      (record, files),
    );
    let mut strip = self
      .playground
      .styled(WindowTabs::new(self.play.selected.clone(), tabs(&open)))
      .label("Open files")
      .shortcuts(shortcuts)
      .disabled(disabled)
      .force(self.playground.forced())
      .on_change(move |file| on_change.0(format!("on_change({file})")))
      .on_close(move |file| {
        on_close.0(format!("on_close({file})"));
        on_close.1.close(*file);
      })
      .on_reorder(move |from, to| {
        on_reorder.0(format!("on_reorder({from}, {to})"));
        on_reorder.1.reorder(from, to);
      })
      .on_detach(move |file| {
        on_detach.0(format!("on_detach({file})"));
        on_detach.1.close(*file);
      });
    if add {
      strip = strip.on_add(move || {
        on_add.0("on_add()".to_owned());
        on_add.1.add();
      });
    }

    let mut code = vec![
      "// open: Signal<usize>, files: the app's list".to_owned(),
      "WindowTabs::new(open, files.iter().map(|file| {".to_owned(),
    ];
    let mut tab = "    WindowTab::new(file.id, &file.name)".to_owned();
    if with_icons {
      tab.push_str(".icon(icons::FILE_CODE)");
    }
    if dirty {
      tab.push_str(".dirty(file.dirty)");
    }
    if !close_buttons {
      tab.push_str(".close_button(false)");
    }
    if disabled_tab {
      tab.push_str(".disabled(file.locked)");
    }
    code.push(tab);
    code.push("}))".to_owned());
    code.push("    .label(\"Open files\")".to_owned());
    if !shortcuts {
      code.push("    .shortcuts(false)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_close(|file| /* take it out of the list */)".to_owned());
    if add {
      code.push("    .on_add(|| /* open a new file */)".to_owned());
    }
    code.push("    .on_reorder(|from, to| /* move it in the list */)".to_owned());
    code.push("    .on_detach(|file| /* open it in a window of its own */)".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root        {}   32 high", self.playground.measured()),
      format!("├─ list      tabs abutting   tablist   ×{}", open.len()),
      "│  └─ tab    128 × 32 · radius 8 8 0 0 · least 72".to_owned(),
      format!(
        "│     └─ {}label 12{}",
        if with_icons { "icon 14 · " } else { "" },
        if close_buttons {
          " · close 18 (x 12, or a dot of 7)"
        } else {
          ""
        }
      ),
      "├─ separator 1 × 16 · between tabs not selected".to_owned(),
      if add {
        "└─ add       30 × 28 · plus 14"
      } else {
        "└─ (no add button: on_add not given)"
      }
      .to_owned(),
    ];

    let defaults = [
      (self.icons.clone(), true),
      (self.dirty.clone(), true),
      (self.close_buttons.clone(), true),
      (self.add.clone(), true),
      (self.shortcuts.clone(), true),
      (self.disabled.clone(), false),
      (self.disabled_tab.clone(), false),
      (self.long.clone(), false),
    ];
    let changed = defaults.iter().filter(|(flag, default)| flag.get() != *default).count()
      + usize::from(open != [0, 1, 2] || selected != 0);
    let reset = {
      let files = self.play.clone();
      Arc::new(move || {
        for (flag, default) in &defaults {
          flag.set(*default);
        }
        files.open.set(vec![0, 1, 2]);
        files.selected.set(0);
      })
    };
    // The row that is measured gives way with the strip in it, in a bar narrower than its tabs.
    let measured = self
      .playground
      .measure(strip.mount(ctx))
      .flex_shrink(1.0)
      .min_width(0.0);
    self.playground.card(
      ctx,
      Spec {
        title: "Window Tabs",
        make: Arc::new(move || window(plain(), 560.0, "Window content").into()),
        shown: Shown::of(window(measured, 560.0, "Window content")).measured(),
        fill: true,
        forceable: true,
        facts: format!("{} tabs{}", open.len(), if add { " · add" } else { "" }),
        event: "on_change",
        code,
        tree,
        controls: vec![
          flag("icon", &self.icons),
          flag("dirty (theme.rs)", &self.dirty),
          flag("close_button", &self.close_buttons),
          flag("a long label (blur.rs)", &self.long),
          flag("a disabled tab (canvas.rs)", &self.disabled_tab),
          divider(),
          flag("on_add", &self.add),
          flag("shortcuts", &self.shortcuts),
          flag("disabled", &self.disabled),
        ],
        changed,
        reset,
      },
    )
  }
}

impl Component for WindowTabsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      play: Files {
        open: ctx.signal(vec![0, 1, 2]),
        selected: ctx.signal(0),
      },
      icons: ctx.signal(true),
      dirty: ctx.signal(true),
      close_buttons: ctx.signal(true),
      add: ctx.signal(true),
      shortcuts: ctx.signal(true),
      disabled: ctx.signal(false),
      disabled_tab: ctx.signal(false),
      long: ctx.signal(false),
      many: Files {
        open: ctx.signal((0..9).collect()),
        selected: ctx.signal(4),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let file = |id: usize| WindowTab::new(id, FILES[id]).icon(icons::FILE_CODE);
    let board = window(
      WindowTabs::controlled(0, [file(0), file(1).dirty(true), file(2)], |_| {}).on_add(|| {}),
      560.0,
      "Window content",
    );

    // A strip whose value is no tab's has its first tab as the one Tab stops on: what is forced on it.
    let none = usize::MAX;
    // What does not fit beside the others in a row goes on to the next line.
    let states = Column::new()
      .spacing(24.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(32.0)
          .wrap()
          .align_items(Alignment::Start)
          .child(item(WindowTabs::controlled(0, [file(0)], |_| {}), "selected"))
          .child(item(
            WindowTabs::controlled(none, [file(2)], |_| {}),
            "idle: the x at 0.6",
          ))
          .child(item(
            WindowTabs::controlled(none, [file(2)], |_| {}).force(Some(Interaction::Hovered)),
            "under the pointer",
          )),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(32.0)
          .wrap()
          .align_items(Alignment::Start)
          .child(item(
            WindowTabs::controlled(none, [file(1).dirty(true)], |_| {}),
            "dirty: a dot",
          ))
          .child(item(
            WindowTabs::controlled(none, [file(1).dirty(true)], |_| {}).force(Some(Interaction::Hovered)),
            "dirty under the pointer: the x",
          ))
          .child(item(
            WindowTabs::controlled(0, [file(0)], |_| {}).force(Some(Interaction::FocusVisible)),
            "focus-visible",
          )),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(32.0)
          .wrap()
          .align_items(Alignment::Start)
          .child(item(
            WindowTabs::controlled(0, [file(0), file(2).disabled(true)], |_| {}),
            "a disabled tab",
          ))
          .child(item(
            WindowTabs::controlled(0, [file(0), file(3).close_button(false)], |_| {}).on_add(|| {}),
            "no close button · the add button",
          )),
      );

    let many = {
      let (close, add, reorder, detach) = (
        self.many.clone(),
        self.many.clone(),
        self.many.clone(),
        self.many.clone(),
      );
      let open = self.many.open.get();
      WindowTabs::new(
        self.many.selected.clone(),
        open.iter().map(|id| file(*id).dirty(*id == 5)),
      )
      .label("Open files")
      .on_close(move |file| close.close(*file))
      .on_add(move || {
        add.add();
      })
      .on_reorder(move |from, to| reorder.reorder(from, to))
      .on_detach(move |file| detach.close(*file))
      .mount(ctx)
    };
    let many = window(many, 520.0, &format!("{} files open", self.many.open.get().len()));

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "Which tab is selected: the one with the signal's value. Selecting another sets it. WindowTabs::controlled \
         takes the value and an on_change in its place.",
      ),
      (
        "tabs",
        "impl IntoIterator<Item = impl Into<WindowTab<T>>>",
        "",
        "The tabs, in order: a (value, label) or a WindowTab with an icon, dirty, close_button, on_close, \
         disabled, id and key.",
      ),
      (
        "on_change",
        "impl Fn(&T)",
        "",
        "Told the value of the tab a person selects, by a press, a drag, the arrows or the window's keys, after \
         the signal is set: not what the app sets, nor a press of the selected tab.",
      ),
      (
        "on_close",
        "impl Fn(&T)",
        "",
        "Told the tab a person closes: its close button, Delete on it, Ctrl+W (Cmd+W on macOS) on the selected \
         one. After the tab's own on_close. The app removes it.",
      ),
      (
        "on_add",
        "impl Fn()",
        "",
        "Told a press of the add button, which is shown once this is given.",
      ),
      (
        "on_reorder",
        "impl Fn(usize, usize)",
        "",
        "Told a tab moved, from its place to its new one: dragged along the strip, or Ctrl+Shift+PageDown and \
         PageUp. The app reorders its list.",
      ),
      (
        "on_detach",
        "impl Fn(&T)",
        "",
        "Told a tab dragged out of the strip and let go more than the strip's height above or below it, or past \
         the window's top or start edge.",
      ),
      (
        "shortcuts",
        "bool",
        "true",
        "Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+PageDown and PageUp select; Ctrl+W closes; Ctrl+Shift+PageDown and \
         PageUp move: from anywhere in the window.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Nothing in the strip can be selected, closed, dragged or added.",
      ),
      (
        "label",
        "&str",
        "",
        "What the list of tabs is called, to what reads the tree.",
      ),
      (
        "add_label",
        "&str",
        "\"New tab\"",
        "What the add button is called, to what reads the tree.",
      ),
      (
        "close_label",
        "&str",
        "\"Close\"",
        "The word before a tab's label that names its close button: \"Close blur.rs\".",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Kept between renders: the dragged tab follows the pointer, the selected one is kept in view, the focus \
         goes to the selected tab when the keys close the one it was on.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Window Tabs"],
        title: "Window Tabs",
        description: "The tabs of a desktop app's title bar, as a browser or an editor has them: one is selected, a \
                      person closes them, asks for a new one, drags one along the bar or out of it.",
        import: Some("use lurq::ui::{WindowTab, WindowTabs};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of WindowTabs is a control on the right. The tabs stand in a title bar, on its bottom edge, \
           the selected one joining the content. A press selects a tab, its x closes it, the plus opens another; \
           drag one along the bar to move it, or down out of it to detach it. Ctrl+Tab and Ctrl+W work from \
           anywhere on the page.",
          playground,
        ),
        Section::new(
          "The board's strip",
          "Three tabs abutting, 128 by 32: blur.rs selected, theme.rs with unsaved changes, canvas.rs idle, and the \
           add button. A separator stands between two tabs that are not selected and before the add button, never \
           beside the selected tab.",
          self.frame.demo(
            0,
            board,
            "WindowTabs::controlled(0, [..], |_| {}).on_add(..)",
            &[
              "WindowTabs::controlled(",
              "    0,",
              "    [",
              "        WindowTab::new(0, \"blur.rs\").icon(icons::FILE_CODE),",
              "        WindowTab::new(1, \"theme.rs\").icon(icons::FILE_CODE).dirty(true),",
              "        WindowTab::new(2, \"canvas.rs\").icon(icons::FILE_CODE),",
              "    ],",
              "    |file| /* the tab a person selected */,",
              ")",
              ".on_add(|| /* open a new file */)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The selected tab is the content's background. Another has no fill and its x dimmed; under the pointer it \
           is the secondary hover and its x whole. A tab with unsaved changes shows a dot in place of its x, and \
           the x under the pointer. The ring of the focus is inside the tab.",
          self.frame.demo(
            1,
            states,
            ".force(Some(Interaction::Hovered))",
            &[
              "WindowTab::new(1, \"theme.rs\").dirty(true)",
              "WindowTab::new(2, \"canvas.rs\").disabled(true)",
              "WindowTab::new(3, \"layout.rs\").close_button(false)",
              "",
              "// Drawn as under the pointer, or focused from the keyboard, with nothing pointing at it.",
              "WindowTabs::controlled(..).force(Some(Interaction::Hovered))",
              "WindowTabs::controlled(..).force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "More tabs than the bar holds",
          "The tabs share the room the bar gives them down to 72 each, the icon and the close slot, and past that \
           the list scrolls sideways with the selected tab kept in view. The add button stays at the end.",
          self.frame.demo(
            2,
            many,
            ".mount(ctx)",
            &[
              "// open: Signal<usize>, files: the app's list of nine",
              "WindowTabs::new(open, files.iter().map(|file| WindowTab::new(file.id, &file.name)))",
              "    .label(\"Open files\")",
              "    .on_close(..)",
              "    .on_add(..)",
              "    .on_reorder(..)",
              "    .on_detach(..)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What window tabs are given.",
          props_table(&rows, &["value", "tabs"], self.frame.room()),
        ),
      ],
    )
  }
}
