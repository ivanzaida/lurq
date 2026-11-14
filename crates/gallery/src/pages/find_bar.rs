//! The page of the find bar: its playground over a text that Ctrl+F searches,
//! the board's find and replace over an editor, and its controls in each state.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, events::KeyboardEvent},
  components::{Column, Row, Stack},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, FindBar, Interaction, Kbd},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The text the page searches.
const TEXT: [&str; 5] = [
  "A find bar searches what a view shows. Type in its field: the app finds the text and gives the bar the count.",
  "Enter goes to the next match and Shift+Enter to the one before; F3 and Shift+F3 do the same anywhere in the bar.",
  "Escape closes the bar, and the focus goes back to the text it searched. Ctrl+F opens it again.",
  "The options say how the app matches: the case of the text, whole words, or a regular expression.",
  "The replace row replaces the current match with its text, or every match at once.",
];

/// The counts the playground offers: the page's own, or one the app says.
const COUNTS: [(&str, usize); 3] = [("Found in the text", 0), ("3 of 12", 1), ("No results", 2)];

/// Where `query` is found in the page's text: the start of each match, with the case or not, whole words or not.
fn find(query: &str, case: bool, whole: bool) -> Vec<usize> {
  if query.is_empty() {
    return Vec::new();
  }
  let text = TEXT.join("\n");
  let (hay, needle) = if case {
    (text.clone(), query.to_owned())
  } else {
    (text.to_lowercase(), query.to_lowercase())
  };
  let word = |at: Option<char>| at.is_none_or(|c| !c.is_alphanumeric());
  hay
    .match_indices(&needle)
    .map(|(at, _)| at)
    .filter(|at| !whole || word(hay[..*at].chars().next_back()) && word(hay[at + needle.len()..].chars().next()))
    .collect()
}

/// What the page's bars are bound to.
#[derive(Clone)]
struct Search {
  open: Signal<bool>,
  query: Signal<String>,
  current: Signal<usize>,
  case: Signal<bool>,
  whole: Signal<bool>,
  regex: Signal<bool>,
  replace: Signal<String>,
  replace_open: Signal<bool>,
}

type Record = Arc<dyn Fn(String) + Send + Sync>;

impl Search {
  /// How many matches there are now, and which is current, counted from 1.
  fn matches(&self) -> (usize, usize) {
    let total = find(&self.query.get(), self.case.get(), self.whole.get()).len();
    let current = if total == 0 {
      0
    } else {
      self.current.get().min(total - 1) + 1
    };
    (current, total)
  }

  /// The bar over the page's text, telling `record` what it did.
  fn bar(&self, record: &Record) -> FindBar {
    let (next, previous) = (self.current.clone(), self.current.clone());
    let (told_next, told_previous, told_query) = (record.clone(), record.clone(), record.clone());
    let (told_dismiss, told_replace, told_all) = (record.clone(), record.clone(), record.clone());
    let (search, back) = (self.clone(), self.clone());
    let reset = self.current.clone();
    FindBar::new(self.query.clone())
      .open(self.open.clone())
      .on_query(move |text| {
        reset.set(0);
        told_query(format!("on_query({text:?})"));
      })
      .on_next(move || {
        let (_, total) = search.matches();
        if total > 0 {
          next.update(|at| *at = (*at + 1) % total);
        }
        told_next("on_next".to_owned());
      })
      .on_previous(move || {
        let (_, total) = back.matches();
        if total > 0 {
          previous.update(|at| *at = (*at + total - 1) % total);
        }
        told_previous("on_previous".to_owned());
      })
      .on_dismiss(move || told_dismiss("on_dismiss".to_owned()))
      .on_replace(move |text| told_replace(format!("on_replace({text:?})")))
      .on_replace_all(move |text| told_all(format!("on_replace_all({text:?})")))
  }
}

/// A view of the page's text with `bar` over it, `height` high; Ctrl+F opens the bar.
fn view(search: &Search, bar: Element, height: f32, text: &ElementRef) -> Element {
  let open = search.open.clone();
  let mut lines = Column::new()
    .size(FILL, height)
    .padding(16.0)
    .spacing(10.0)
    .tab_index(0)
    .ref_element(text.clone())
    .describe("a11y_name", "Document");
  for line in TEXT {
    lines = lines.child(quiet(line, 13.0));
  }
  Stack::new()
    .size(FILL, height)
    .on_key_down(move |event: KeyboardEvent| {
      if (event.ctrl || event.meta) && (event.code == "KeyF" || event.key.eq_ignore_ascii_case("f")) {
        event.prevent_default();
        open.set(true);
      }
    })
    .child(lines)
    .child(bar)
    .into()
}

pub struct FindBarPage {
  frame: Frame,
  playground: Playground,
  search: Search,
  /// The board's find and replace, open with its replace row.
  editor: Search,
  /// The text the bar gives the focus back to.
  text: ElementRef,
  editor_text: ElementRef,
  count: Signal<usize>,
  label: Signal<String>,
  options: Signal<bool>,
  replacing: Signal<bool>,
  disabled: Signal<bool>,
}

impl FindBarPage {
  /// The playground: the bar over the page's text, as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (count, label) = (self.count.get(), self.label.get());
    let (options, replacing, disabled) = (self.options.get(), self.replacing.get(), self.disabled.get());
    let record: Record = Arc::new(self.playground.recorder());
    let plain = {
      let (search, label, record) = (self.search.clone(), label.clone(), record.clone());
      let text = self.text.clone();
      move || {
        let mut bar = search
          .bar(&record)
          .label(&label)
          .disabled(disabled)
          .return_focus(text.clone());
        bar = match count {
          1 => bar.matches(3, 12),
          2 => bar.matches(0, 0),
          _ => {
            let (current, total) = search.matches();
            bar.matches(current, total)
          }
        };
        if options {
          bar = bar
            .match_case(search.case.clone())
            .whole_word(search.whole.clone())
            .regex(search.regex.clone());
        }
        if replacing {
          bar = bar
            .replace(search.replace.clone())
            .replace_open(search.replace_open.clone());
        }
        bar
      }
    };
    let bar = plain().force(self.playground.forced()).mount(ctx);
    let open = self.search.open.clone();
    let shown = Column::new()
      .width(FILL)
      .spacing(10.0)
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Button::new("Find")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| open.set(true)),
          )
          .child(Kbd::new("Ctrl+F"))
          .child(quiet("in the text opens it", 12.0)),
      )
      .child(view(&self.search, bar, 260.0, &self.text));

    let mut code = vec!["FindBar::new(query)".to_owned(), "    .open(open)".to_owned()];
    code.push(match count {
      1 => "    .matches(3, 12)".to_owned(),
      2 => "    .matches(0, 0)".to_owned(),
      _ => "    .matches(current, total)".to_owned(),
    });
    if label != "Find" {
      code.push(format!("    .label({label:?})"));
    }
    if options {
      code.push("    .match_case(case).whole_word(whole).regex(regex)".to_owned());
    }
    if replacing {
      code.push("    .replace(with).replace_open(replacing)".to_owned());
      code.push("    .on_replace(|text| ..).on_replace_all(|text| ..)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_query(|text| ..).on_next(|| ..).on_previous(|| ..)".to_owned());
    code.push("    .return_focus(text)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      "root        the view · hit test ContentOnly · padding 8".to_owned(),
      "└─ bar      440 · popover · border · shadow lg · radius lg · padding 6 · gap 6".to_owned(),
      "   ├─ find row    28 · gap 4".to_owned(),
      "   │  ├─ field    input 28 · text 13 · options 22 over its end".to_owned(),
      "   │  ├─ count    11 · muted · padding 6".to_owned(),
      "   │  └─ buttons  24 · arrow-up · arrow-down · close".to_owned(),
      "   └─ replace row indented 28 · field · replace · replace-all · pad 52".to_owned(),
    ];

    let (to_count, to_label, to_options, to_replacing, to_disabled) = (
      self.count.clone(),
      self.label.clone(),
      self.options.clone(),
      self.replacing.clone(),
      self.disabled.clone(),
    );
    let set_count = self.count.clone();
    let make = {
      let (plain, search, text) = (plain.clone(), self.search.clone(), self.text.clone());
      move || view(&search, plain().into(), 260.0, &text)
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Find Bar",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: true,
        forceable: true,
        facts: "440 wide, over its view".to_owned(),
        event: "on_next / on_query",
        code,
        tree,
        controls: vec![
          prop(
            "matches",
            "(usize, usize)",
            self
              .playground
              .select(0, &COUNTS, &count, move |count| set_count.set(count)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("options", &self.options),
          flag("replace", &self.replacing),
          flag("disabled", &self.disabled),
        ],
        changed: [count != 0, label != "Find", !options, !replacing, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_count.set(0);
          to_label.set("Find".to_owned());
          to_options.set(true);
          to_replacing.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }

  /// The board's find and replace: an editor with the bar open, its options and its replace row.
  fn editor(&self, ctx: &mut Ctx) -> Element {
    let record: Record = Arc::new(|_| {});
    let (current, total) = self.editor.matches();
    let bar = self
      .editor
      .bar(&record)
      .matches(current, total)
      .match_case(self.editor.case.clone())
      .whole_word(self.editor.whole.clone())
      .regex(self.editor.regex.clone())
      .replace(self.editor.replace.clone())
      .replace_open(self.editor.replace_open.clone())
      .return_focus(self.editor_text.clone())
      .mount(ctx);
    view(&self.editor, bar, 240.0, &self.editor_text)
  }

  /// The bar's buttons and options at rest and in each interaction, and a search that finds nothing.
  fn states(&self) -> Element {
    let specimen = |interaction: Option<Interaction>, caption: &str, total: usize| {
      let query = Signal::new(if total == 0 { "zzz" } else { "bar" }.to_owned());
      Column::new()
        .spacing(10.0)
        .child(
          Column::new().width(440.0).child(
            FindBar::new(query)
              .matches(if total == 0 { 0 } else { 2 }, total)
              .match_case(Signal::new(true))
              .whole_word(Signal::new(false))
              .regex(Signal::new(false))
              .force(interaction),
          ),
        )
        .child(quiet(caption, 12.0))
    };
    Column::new()
      .width(FILL)
      .spacing(20.0)
      .child(specimen(None, "idle", 12))
      .child(specimen(Some(Interaction::Hovered), "hover", 12))
      .child(specimen(Some(Interaction::FocusVisible), "focus-visible", 12))
      .child(specimen(None, "no results", 0))
      .into()
  }
}

impl Component for FindBarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let search = |ctx: &mut Ctx, open: bool, query: &str, replacing: bool| Search {
      open: ctx.signal(open),
      query: ctx.signal(query.to_owned()),
      current: ctx.signal(0),
      case: ctx.signal(false),
      whole: ctx.signal(false),
      regex: ctx.signal(false),
      replace: ctx.signal(String::new()),
      replace_open: ctx.signal(replacing),
    };
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      search: search(ctx, false, "the", false),
      editor: search(ctx, true, "bar", true),
      text: ElementRef::new(),
      editor_text: ElementRef::new(),
      count: ctx.signal(0),
      label: ctx.signal("Find".to_owned()),
      options: ctx.signal(true),
      replacing: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "new",
        "Signal<String>",
        "",
        "The bar of what is typed in the signal: the app finds it, and gives the bar the count.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the bar in the tree: the focus goes into its field as it opens, and back as it closes.",
      ),
      (
        "open",
        "Signal<bool>",
        "always",
        "Whether it is shown; Escape and the close button set it to false.",
      ),
      (
        "on_query",
        "impl Fn(&str)",
        "",
        "Called after each edit of the field, with what it holds.",
      ),
      (
        "on_next / on_previous",
        "impl Fn()",
        "",
        "Enter and Shift+Enter in the field, F3 and Shift+F3 in the bar, the arrows.",
      ),
      (
        "matches",
        "(usize, usize)",
        "none",
        "The current match, from 1, and how many there are: \"3 of 12\", or \"No results\".",
      ),
      (
        "count_text",
        "impl Fn(usize, usize) -> String",
        "",
        "What the bar says of the count, in place of \"3 of 12\".",
      ),
      (
        "match_case / whole_word / regex",
        "Signal<bool>",
        "not shown",
        "An option in the field, shown when given; each has _controlled(on, on_change) and on_<option>_change.",
      ),
      (
        "replace",
        "Signal<String>",
        "",
        "What the matches are replaced with: the replace row under the field.",
      ),
      (
        "replace_open",
        "Signal<bool>",
        "",
        "Whether the replace row is shown: the chevron before the field toggles it.",
      ),
      (
        "on_replace / on_replace_all",
        "impl Fn(&str)",
        "",
        "Its buttons, told the replace text; Enter in the replace field replaces one.",
      ),
      (
        "on_dismiss / on_open_change",
        "impl Fn() / impl Fn(bool)",
        "",
        "Called when a person closes it, by Escape or the close button.",
      ),
      (
        "return_focus",
        "ElementRef",
        "",
        "Where the focus goes as the bar closes with the focus in it: the view it searched.",
      ),
      ("disabled", "bool", "false", "Its controls take no input."),
      (
        "label, placeholder, replace_placeholder",
        "&str",
        "\"Find\", \"Find\", \"Replace\"",
        "What the bar and its fields are called and show.",
      ),
      (
        "<control>_label",
        "&str",
        "",
        "previous, next, close, match_case, whole_word, regex, replace, replace_all, toggle_replace.",
      ),
      (
        "style, rules, state, force, id, key",
        "",
        "",
        "As every themed builder has them.",
      ),
    ];
    let playground = self.play(ctx);
    let editor = self.editor(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Find Bar"],
        title: "Find Bar",
        description: "The search of what a view shows, hung from the top of the view over its content: a field, \
                      the count, the next and the previous match, the options of the search, and a row that \
                      replaces.",
        import: Some("use lurq::ui::FindBar;"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop is a control on the right. Press the text and Ctrl+F, or Find: the bar comes down with the \
           focus in its field. Type, press Enter and Shift+Enter, turn an option on; Escape closes it and the \
           focus goes back to the text. The page finds the text itself and gives the bar the count.",
          playground,
        ),
        Section::new(
          "Find and replace",
          "The board's find and replace over an editor: the options in the field, and the replace row under it, \
           which the chevron before the field hides.",
          self.frame.demo(
            0,
            editor,
            ".replace(with).replace_open(replacing)",
            &[
              "FindBar::new(query)",
              "    .open(open)",
              "    .matches(current, total)",
              "    .match_case(case).whole_word(whole).regex(regex)",
              "    .replace(with).replace_open(replacing)",
              "    .on_replace(|text| ..).on_replace_all(|text| ..)",
              "    .return_focus(editor)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "Its buttons and options at rest, under the pointer and focused from the keyboard, and a search that \
           finds nothing: the count in the destructive color.",
          self.frame.demo(
            1,
            self.states(),
            ".force(Some(Interaction::Hovered))",
            &["FindBar::new(query).matches(0, 0)"],
          ),
        ),
        Section::new(
          "Props",
          "What a find bar is given.",
          props_table(&rows, &["new", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
