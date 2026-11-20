//! The page of the window tabs: their playground in a title bar, the board's
//! strip, each state of a tab, a window with more tabs than its bar holds,
//! and the tabs of an app's title bar: a home tab, marks after a label, the
//! dot beside the close button, and the tabs that do not fit in a list; and the list of every open tab, with tabs
//! as wide as what they show, 2 apart.

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
  ui::{
    Badge, BadgeSize, BadgeVariant, Interaction, Recipe, Tooltip, WindowTab, WindowTabs, WindowTabsDirtyMark,
    WindowTabsMenu, WindowTabsOverflow, WindowTabsPart, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

/// The names of the files the tabs hold, by their id.
/// The folder of each of [`FILES`], which a row of the list of every tab says after the file's name.
const FOLDERS: [&str; 12] = [
  "ui", "ui", "canvas", "layout", "router", "text", "animation", "form", "charts", "ui", "media", "app",
];

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

/// The value of the home tab, which is no file's.
const HOME: usize = usize::MAX;

/// The mark that something in a tab waits for the person: an amber hand with how many.
fn hand(count: usize) -> Badge {
  Badge::count(count)
    .icon(icons::HAND)
    .variant(BadgeVariant::Warning)
    .size(BadgeSize::Sm)
}

/// [`hand`] with a tooltip that says what waits.
fn waiting(ctx: &mut Ctx, count: usize) -> Element {
  Tooltip::new(&format!("{count} runs wait for you")).mount(ctx, hand(count))
}

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
  home: Signal<bool>,
  marks: Signal<bool>,
  capped: Signal<bool>,
  after_label: Signal<bool>,
  listed: Signal<bool>,
  every: Signal<bool>,
  list_shortcut: Signal<bool>,
  fit: Signal<bool>,
  tab_stop: Signal<bool>,
  many: Files,
  app: Files,
  all: Files,
  /// Whether the list of every open tab is open: the page's signal, which its button sets too.
  all_listed: Signal<bool>,
}

/// The rules of a strip whose tabs are 2 apart, and 2 before the add button, with no line between them.
fn gapped() -> Recipe {
  Recipe::new()
    .base(WindowTabsPart::List, |s| s.gap(2.0))
    .base(WindowTabsPart::Root, |s| s.gap(2.0))
    .base(WindowTabsPart::Separator, |s| s.width(0.0).height(0.0))
}

impl WindowTabsPage {
  /// The playground: the tabs in a title bar, as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (with_icons, dirty, close_buttons) = (self.icons.get(), self.dirty.get(), self.close_buttons.get());
    let (add, shortcuts, disabled) = (self.add.get(), self.shortcuts.get(), self.disabled.get());
    let (disabled_tab, long) = (self.disabled_tab.get(), self.long.get());
    let (home, marks, capped) = (self.home.get(), self.marks.get(), self.capped.get());
    let (after_label, listed, tab_stop) = (self.after_label.get(), self.listed.get(), self.tab_stop.get());
    let (every, list_shortcut, fit) = (self.every.get(), self.list_shortcut.get(), self.fit.get());
    let menu = if every {
      WindowTabsMenu::All
    } else {
      WindowTabsMenu::Hidden
    };
    let open = self.play.open.get();
    let selected = self.play.selected.get();
    let mark = marks.then(|| waiting(ctx, 2));
    let tabs = move |open: &[usize], mark: Option<Element>| -> Vec<WindowTab<usize>> {
      let home = home.then(|| {
        WindowTab::new(HOME, "Home")
          .icon(icons::HOME)
          .icon_only(true)
          .closable(false)
          .describe("route", "/")
      });
      let files = open.iter().map(|file| {
        let name = if long && *file == 0 { LONG } else { FILES[*file] };
        let mut tab = WindowTab::new(*file, name)
          .dirty(dirty && *file == 1)
          .close_button(close_buttons)
          .disabled(disabled_tab && *file == 2)
          .describe("route", format!("/files/{}", FILES[*file]));
        if with_icons {
          tab = tab.icon(icons::FILE_CODE);
        }
        if capped {
          tab = tab.max_width(110.0);
        }
        if fit {
          tab = tab.fit_content(200.0);
        }
        if let (Some(mark), 2) = (&mark, *file) {
          tab = tab.trailing(mark.clone());
        }
        tab
      });
      home.into_iter().chain(files).collect()
    };
    let overflow = if listed {
      WindowTabsOverflow::List
    } else {
      WindowTabsOverflow::Scroll
    };
    let dirty_mark = if after_label {
      WindowTabsDirtyMark::AfterLabel
    } else {
      WindowTabsDirtyMark::InCloseSlot
    };
    let plain = {
      let open = open.clone();
      move || {
        let mark = marks.then(|| Element::from(hand(2)));
        let mut strip = WindowTabs::controlled(selected, tabs(&open, mark), |_| {})
          .label("Open files")
          .shortcuts(false)
          .disabled(disabled)
          .overflow(overflow)
          .menu(menu)
          .list_shortcut(list_shortcut)
          .dirty_mark(dirty_mark)
          .tab_stop(tab_stop);
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
      .styled(WindowTabs::new(self.play.selected.clone(), tabs(&open, mark)))
      .label("Open files")
      .shortcuts(shortcuts)
      .disabled(disabled)
      .overflow(overflow)
      .menu(menu)
      .list_shortcut(list_shortcut)
      .dirty_mark(dirty_mark)
      .tab_stop(tab_stop)
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

    let mut code = vec!["// open: Signal<usize>, files: the app's list".to_owned()];
    if home {
      code.push(
        "let home = WindowTab::new(HOME, \"Home\").icon(icons::HOME).icon_only(true).closable(false);".to_owned(),
      );
      code.push("WindowTabs::new(open, std::iter::once(home).chain(files.iter().map(|file| {".to_owned());
    } else {
      code.push("WindowTabs::new(open, files.iter().map(|file| {".to_owned());
    }
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
    if capped {
      tab.push_str(".max_width(110.0)");
    }
    if fit {
      tab.push_str(".fit_content(200.0)");
    }
    tab.push_str(".describe(\"route\", &file.route)");
    code.push(tab);
    if marks {
      code.push("        // A badge in a tooltip, after the label of the file whose runs wait.".to_owned());
      code.push(
        "        .trailing(Tooltip::new(\"2 runs wait for you\").mount(ctx, Badge::count(2).icon(icons::HAND)))"
          .to_owned(),
      );
    }
    code.push(if home { "})))" } else { "}))" }.to_owned());
    code.push("    .label(\"Open files\")".to_owned());
    if listed {
      code.push("    .overflow(WindowTabsOverflow::List)".to_owned());
    }
    if every {
      code.push("    .menu(WindowTabsMenu::All)".to_owned());
    }
    if list_shortcut {
      code.push("    .list_shortcut(true)".to_owned());
    }
    if after_label {
      code.push("    .dirty_mark(WindowTabsDirtyMark::AfterLabel)".to_owned());
    }
    if !tab_stop {
      code.push("    .tab_stop(false)".to_owned());
    }
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

    let mut tree = vec![
      format!("root        {}   32 high", self.playground.measured()),
      format!(
        "├─ list      tabs abutting   tablist   ×{}",
        open.len() + usize::from(home)
      ),
    ];
    if home {
      tree.push("│  ├─ tab    28 × 32 · icon_only · the home icon 14 in its middle · not closable".to_owned());
    }
    tree.push(format!(
      "│  └─ tab    {} × 32 · radius 8 8 0 0 · least 72",
      if capped { "110" } else { "128" }
    ));
    tree.push(format!(
      "│     ├─ {}title: label 12{}{}",
      if with_icons { "icon 14 · " } else { "" },
      if after_label { " · dot 7, 6 after it" } else { "" },
      if marks { " · marks (6 after)" } else { "" },
    ));
    tree.push(
      if close_buttons {
        if after_label {
          "│     └─ close 18 (x 12, always)"
        } else {
          "│     └─ close 18 (x 12, or a dot of 7)"
        }
      } else {
        "│     └─ (no close button: closed by the middle button and Delete)"
      }
      .to_owned(),
    );
    tree.push("├─ separator 1 × 16 · between tabs not selected".to_owned());
    if listed {
      tree.push("├─ more      28 high · the count 12 · chevron 12 · while tabs left the strip".to_owned());
    }
    tree.push(
      if add {
        "└─ add       30 × 28 · plus 14"
      } else {
        "└─ (no add button: on_add not given)"
      }
      .to_owned(),
    );

    let defaults = [
      (self.icons.clone(), true),
      (self.dirty.clone(), true),
      (self.close_buttons.clone(), true),
      (self.add.clone(), true),
      (self.shortcuts.clone(), true),
      (self.disabled.clone(), false),
      (self.disabled_tab.clone(), false),
      (self.long.clone(), false),
      (self.home.clone(), false),
      (self.marks.clone(), false),
      (self.capped.clone(), false),
      (self.after_label.clone(), false),
      (self.listed.clone(), false),
      (self.every.clone(), false),
      (self.list_shortcut.clone(), false),
      (self.fit.clone(), false),
      (self.tab_stop.clone(), true),
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
        facts: format!(
          "{} tabs{}{}",
          open.len() + usize::from(home),
          if add { " · add" } else { "" },
          if listed { " · list" } else { "" }
        ),
        event: "on_change",
        code,
        tree,
        controls: vec![
          flag("icon", &self.icons),
          flag("dirty (theme.rs)", &self.dirty),
          flag("close_button", &self.close_buttons),
          flag("a long label (blur.rs)", &self.long),
          flag("a disabled tab (canvas.rs)", &self.disabled_tab),
          flag("a home tab (icon_only, closable(false))", &self.home),
          flag("trailing: a mark (canvas.rs)", &self.marks),
          flag("max_width(110)", &self.capped),
          flag("fit_content(200)", &self.fit),
          divider(),
          flag("on_add", &self.add),
          flag("overflow: List", &self.listed),
          flag("menu: All", &self.every),
          flag("list_shortcut", &self.list_shortcut),
          flag("dirty_mark: AfterLabel", &self.after_label),
          flag("tab_stop", &self.tab_stop),
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
      home: ctx.signal(false),
      marks: ctx.signal(false),
      capped: ctx.signal(false),
      after_label: ctx.signal(false),
      listed: ctx.signal(false),
      every: ctx.signal(false),
      list_shortcut: ctx.signal(false),
      fit: ctx.signal(false),
      tab_stop: ctx.signal(true),
      many: Files {
        open: ctx.signal((0..9).collect()),
        selected: ctx.signal(4),
      },
      app: Files {
        open: ctx.signal((0..9).collect()),
        selected: ctx.signal(HOME),
      },
      all: Files {
        open: ctx.signal((0..12).collect()),
        selected: ctx.signal(0),
      },
      all_listed: ctx.signal(false),
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

    // An app's title bar: a home tab of its icon alone, a mark after a label, the dot beside the close button, and
    // the tabs that do not fit in the list of the More button.
    let app = {
      let (close, add, reorder) = (self.app.clone(), self.app.clone(), self.app.clone());
      let open = self.app.open.get();
      let mark = waiting(ctx, 3);
      let home = WindowTab::new(HOME, "Home")
        .icon(icons::HOME)
        .icon_only(true)
        .closable(false);
      let files = open.iter().map(|id| {
        let tab = file(*id).dirty(*id == 1).max_width(140.0);
        if *id == 2 { tab.trailing(mark.clone()) } else { tab }
      });
      WindowTabs::new(self.app.selected.clone(), std::iter::once(home).chain(files))
        .label("Open files")
        .overflow(WindowTabsOverflow::List)
        .dirty_mark(WindowTabsDirtyMark::AfterLabel)
        .on_close(move |file| close.close(*file))
        .on_add(move || {
          add.add();
        })
        // The home tab stays first: the files are the tabs after it.
        .on_reorder(move |from, to| {
          if from > 0 {
            reorder.reorder(from - 1, to.saturating_sub(1));
          }
        })
        .mount(ctx)
    };
    let app = window(app, 520.0, &format!("{} files open", self.app.open.get().len()));

    // Every open tab in the list of the More button, tabs as wide as what they show, 2 apart.
    let all = {
      let (close, add, reorder) = (self.all.clone(), self.all.clone(), self.all.clone());
      let open = self.all.open.get();
      WindowTabs::new(
        self.all.selected.clone(),
        open
          .iter()
          .map(|id| file(*id).fit_content(160.0).list_detail(FOLDERS[*id])),
      )
      .label("Open files")
      .rules(&gapped())
      .overflow(WindowTabsOverflow::List)
      .menu(WindowTabsMenu::All)
      .list_shortcut(true)
      .menu_open(self.all_listed.clone())
      .on_close(move |file| close.close(*file))
      .on_add(move || {
        add.add();
      })
      .on_reorder(move |from, to| reorder.reorder(from, to))
      .mount(ctx)
    };
    let listed = self.all_listed.clone();
    let all = Column::new()
      .width(FILL)
      .spacing(12.0)
      .align_items(Alignment::Start)
      .child(window(all, 520.0, &format!("{} files open", self.all.open.get().len())))
      // The app opens the list by its signal, whether a tab is hidden or not: close tabs until all fit to see it
      // hang from the tabs.
      .child(lurq::ui::Button::new("Show every open tab").on_click(move |_| listed.set(true)));

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
        "The tabs, in order: a (value, label) or a WindowTab with an icon, dirty, close_button, closable, \
         on_close, disabled, icon_only, width, max_width, trailing, describe, id and key.",
      ),
      (
        "WindowTab::closable",
        "bool",
        "true",
        "Whether a person may close the tab. One that may not (a home tab) shows no close button and is closed by \
         nothing: no middle press, no Delete, no Ctrl+W.",
      ),
      (
        "WindowTab::close_button",
        "bool",
        "true",
        "Whether the close button shows. A closable tab without one is still closed by the middle button, Delete \
         and Ctrl+W.",
      ),
      (
        "WindowTab::icon_only",
        "bool",
        "false",
        "The icon alone, 28 wide, no label and no close slot: a home tab. The label is still its name to tooling.",
      ),
      (
        "WindowTab::width / max_width",
        "f32",
        "",
        "A width of its own, which neither grows nor gives way; or a cap under the recipe's most, the label cut \
         there.",
      ),
      (
        "WindowTab::fit_content",
        "f32",
        "",
        "As wide as what it shows, its icon, label, marks and close slot, up to this cap, the label cut there. In \
         the list mode it keeps that width and leaves the strip whole.",
      ),
      (
        "WindowTab::trailing",
        "impl Into<Element>",
        "",
        "The app's element after the label, in the tab's marks part: a count, a mark that something waits, with \
         its tooltip. It keeps its width; the label gives way first.",
      ),
      (
        "WindowTab::list_label / list_detail / list_trailing",
        "&str, &str, impl Into<Element>",
        "",
        "Of the list of every tab: what the tab's row says in place of its label, a quiet text after it (both \
         searched), and the app's elements in the row, apart from trailing and with ids of their own.",
      ),
      (
        "WindowTab::describe",
        "(name, value)",
        "",
        "Said of the tab to tooling (the DevTools, the MCP tree) before what the strip says: its route, its state.",
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
        "overflow",
        "WindowTabsOverflow",
        "Scroll",
        "Scroll: the tabs share the room down to 72 and then the strip scrolls. List: they keep their width, and \
         those that do not fit leave the strip, the last first and never the selected one, into the menu of a \
         button at the end of the tabs that shows how many left. Mounted only.",
      ),
      (
        "menu",
        "WindowTabsMenu",
        "Hidden",
        "What the More button of the list mode opens. Hidden: a menu of the tabs that left the strip. All: a list of \
         every open tab, with a search field once it holds more than eight, marks on the selected tab and on those \
         not in the strip, and a close button on each row; Ctrl+W (Cmd+W) on a row closes its tab.",
      ),
      (
        "list_shortcut",
        "bool",
        "false",
        "Ctrl+Shift+A (Cmd+Shift+A on macOS) opens and closes the More button's menu from anywhere in the window. \
         The button's tooltip says the keys.",
      ),
      (
        "menu_open / on_menu_open_change",
        "Signal<bool>, impl Fn(bool)",
        "",
        "Whether the More button's menu is open, as the app's signal, which the strip sets and the app may set; and \
         what hears a person open or close it.",
      ),
      (
        "list_clear_of / list_inset",
        "impl Into<ElementRef>, f32",
        "the More button, 8",
        "The list of every tab stands 4 clear of that element's box (the title bar), lined up with the button, and \
         stays that far inside the window's edges, never lower than that above its bottom.",
      ),
      (
        "more_tooltip",
        "impl Fn(usize) -> String",
        "\"3 hidden tabs\"",
        "What the More button's tooltip says for how many tabs are hidden, and what it says of itself to tooling.",
      ),
      (
        "dirty_mark",
        "WindowTabsDirtyMark",
        "InCloseSlot",
        "InCloseSlot: the dot in the close slot, the x in its place under the pointer. AfterLabel: the dot right \
         after the label, and the close button in its slot, both shown.",
      ),
      (
        "tab_stop",
        "bool",
        "true",
        "false: Tab passes the tabs, the add button and the More button by, and a press leaves the focus where it \
         was. The window's keys still work.",
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
        "more_label",
        "&str",
        "\"Show the hidden tabs\"",
        "What the More button of the list mode and its menu are called, to what reads the tree.",
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
          "An app's title bar",
          "A home tab of its icon alone that nobody closes, a mark after a label with its tooltip, the dot of \
           unsaved changes right after the label beside a close button that stays, and tabs that keep their width: \
           those that do not fit leave the bar for the menu of the button after the tabs, which says how many. \
           Choose one there and it comes back as the selected tab. A middle press closes any tab but the home tab.",
          self.frame.demo(
            3,
            app,
            ".overflow(WindowTabsOverflow::List)",
            &[
              "let home = WindowTab::new(HOME, \"Home\").icon(icons::HOME).icon_only(true).closable(false);",
              "let files = files.iter().map(|file| {",
              "    let tab = WindowTab::new(file.id, &file.name).dirty(file.dirty).max_width(140.0);",
              "    match file.waiting {",
              "        0 => tab,",
              "        n => tab.trailing(Tooltip::new(&format!(\"{n} runs wait for you\")).mount(ctx, hand(n))),",
              "    }",
              "});",
              "WindowTabs::new(open, std::iter::once(home).chain(files))",
              "    .overflow(WindowTabsOverflow::List)",
              "    .dirty_mark(WindowTabsDirtyMark::AfterLabel)",
              "    .on_close(..)",
              "    .on_add(..)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Every open tab in a list",
          "Tabs as wide as what they show up to 160, 2 apart with no line between them (the gap of the list part, \
           which the strip counts as it fits them). The More button opens the list of every open tab: a search \
           field over twelve, a check on the selected tab, a mark on each tab not in the bar, and a close button \
           on each row, which says the file's folder after its name. Ctrl+Shift+A (Cmd+Shift+A) opens it; Up and \
           Down move through it, Enter chooses, Ctrl+W closes the row's tab, Escape empties the search and then \
           closes it. While it has the focus its keys are its own, and the selected tab's row is kept in view. The \
           shortcut and the app's signal (menu_open, the button under the window) open it with every tab in the bar \
           too: the More button stays hidden, and the list hangs from the tabs.",
          self.frame.demo(
            4,
            all,
            ".menu(WindowTabsMenu::All)",
            &[
              "WindowTabs::new(open, files.iter().map(|file| {",
              "    WindowTab::new(file.id, &file.name)",
              "        .icon(icons::FILE_CODE)",
              "        .fit_content(160.0)",
              "        // The row of the list says the folder after the name, quieter, and a search finds it.",
              "        .list_detail(&file.folder)",
              "}))",
              "    // 2 between tabs and before the add button, no line between them.",
              "    .rules(",
              "        &Recipe::new()",
              "            .base(WindowTabsPart::List, |s| s.gap(2.0))",
              "            .base(WindowTabsPart::Root, |s| s.gap(2.0))",
              "            .base(WindowTabsPart::Separator, |s| s.width(0.0).height(0.0)),",
              "    )",
              "    .overflow(WindowTabsOverflow::List)",
              "    .menu(WindowTabsMenu::All)",
              "    .list_shortcut(true)",
              "    // The app's signal: setting it opens the list, whether a tab is hidden or not.",
              "    .menu_open(listed)",
              "    .on_close(..)",
              "    .on_add(..)",
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
