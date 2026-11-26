//! A rich text editor's link triggers and wiki links: a trigger typed opens a list of links under it, narrowed by
//! what is typed after it, whose choice is a link in place of what was typed; a wiki link reads, is pressed and is
//! written back as one.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{events::ClipboardContent, theme::PaletteColor},
  components::{Column, RichText, WikiLink},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  ui::{LinkOffer, LinkTrigger, RichTextController, RichTextEditor},
};

use super::stage::Stage;

const CONTENT: &str = "editor-content";

fn pages() -> Vec<LinkOffer> {
  vec![
    LinkOffer::new("Roadmap 2027", "/roadmap").id("roadmap"),
    LinkOffer::new("Onboarding", "/onboarding")
      .description("Where to start")
      .id("onboarding"),
    LinkOffer::new("Release notes", "/releases").id("releases"),
  ]
}

fn title(target: &str) -> Option<String> {
  pages()
    .into_iter()
    .find(|page| page.target() == target)
    .map(|page| page.label().to_owned())
}

/// An editor with a `[[` trigger of pages, as wiki links, and a `KON-` trigger of tasks found by their number.
struct Editor {
  stage: Stage,
  value: Signal<RichText>,
  editor: RichTextController,
  opened: Arc<Mutex<Vec<String>>>,
}

impl Editor {
  fn new(document: RichText) -> Self {
    Self::with(document, |editor| editor)
  }

  fn with(document: RichText, adjust: impl Fn(RichTextEditor) -> RichTextEditor + Send + Sync + 'static) -> Self {
    let value = Signal::new(document);
    let editor = RichTextController::new();
    let opened = Arc::new(Mutex::new(Vec::new()));
    let (held, handle, heard) = (value.clone(), editor.clone(), opened.clone());
    let mut stage = Stage::first_frame_in(lurq::app::App::new(), 640.0, 480.0, move |ctx| {
      let heard = heard.clone();
      let editor = RichTextEditor::new(held.clone())
        .id("editor")
        .controller(&handle)
        .wiki_links(title)
        .link_trigger(LinkTrigger::new("[[", pages()).wiki(true).spaces(true))
        .link_trigger(LinkTrigger::new("KON-", |typed: &str| {
          (1..=3)
            .map(|n| {
              let key = format!("KON-{typed}{n}");
              LinkOffer::new(&key, &format!("https://tracker.example/{key}")).id(key.as_str())
            })
            .collect()
        }))
        .on_link_click(move |target| heard.lock().unwrap().push(target.to_owned()));
      Column::new().size(640.0, 480.0).child(adjust(editor).mount(ctx)).into()
    });
    stage.pass();
    Self {
      stage,
      value,
      editor,
      opened,
    }
  }

  /// Puts the focus in the box, the caret at the end of the text.
  fn focus(&mut self) {
    self.stage.tree.get_element_by_id_mut(CONTENT).unwrap().focus();
    self.stage.pass();
    let end = self.value.get().text.len();
    assert!(self.editor.select(end, end));
    self.stage.pass();
  }

  fn type_text(&mut self, text: &str) {
    for character in text.chars() {
      let key = character.to_string();
      self.stage.tree.key_down(key.clone(), "Key".into(), false, false, false);
      self.stage.tree.key_up(key, "Key".into(), false, false, false);
      self.stage.pass();
    }
  }

  fn markdown(&self) -> String {
    self.value.get().to_markdown()
  }

  /// The offers' rows that are filled with the highlight.
  fn highlighted(&self) -> Vec<Quad> {
    let highlight = lurq::app::theme::ThemePalette::light().get(PaletteColor::Highlight);
    self
      .stage
      .quads()
      .into_iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == highlight))
      .collect()
  }
}

#[test]
fn a_trigger_typed_opens_its_offers_under_it_and_enter_puts_in_a_wiki_link() {
  let mut editor = Editor::new(RichText::plain("See "));
  editor.focus();
  assert!(!editor.stage.paints("Roadmap 2027"), "nothing before a trigger");
  editor.type_text("[[");
  for page in ["Roadmap 2027", "Onboarding", "Release notes", "Where to start"] {
    assert!(editor.stage.paints(page), "{page} is offered");
  }
  editor.type_text("road");
  assert!(editor.stage.paints("Roadmap 2027"));
  assert!(!editor.stage.paints("Onboarding"), "narrowed by what was typed");
  // The list hangs under the trigger.
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let trigger = editor.editor.input().caret_point(4).unwrap();
  let (row_x, row_y, ..) = editor.stage.bounds("roadmap");
  assert!(
    row_x >= x + trigger.x && row_x < x + trigger.x + 40.0,
    "{row_x} under the trigger at {}",
    trigger.x
  );
  assert!(row_y > y + trigger.y);
  assert_eq!(
    editor.highlighted().len(),
    1,
    "the first offer is the one Enter chooses"
  );

  editor.stage.key("Enter");
  let document = editor.value.get();
  assert_eq!(
    document.text, "See Roadmap 2027",
    "the trigger and what was typed gave way to the link"
  );
  assert_eq!(document.spans.len(), 1);
  assert_eq!(document.spans[0].style.link.as_deref(), Some("/roadmap"));
  assert_eq!(
    document.spans[0].style.wiki,
    Some(WikiLink::Titled("Roadmap 2027".into()))
  );
  assert_eq!(editor.markdown(), "See [[/roadmap]]");
  assert!(!editor.stage.paints("Onboarding"), "the list closed");
  assert_eq!(
    editor
      .stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some(CONTENT)
  );
  // What is typed next is plain text after the link.
  editor.type_text(" ok");
  assert_eq!(editor.markdown(), "See [[/roadmap]] ok");
  // The choice is one step of the undo.
  assert!(editor.editor.undo());
  assert!(editor.editor.undo());
  editor.stage.pass();
  assert_eq!(editor.value.get().text, "See [[road");
}

#[test]
fn arrows_move_the_offer_and_tab_chooses_a_plain_link_of_a_key_prefix() {
  let mut editor = Editor::new(RichText::plain("Fixed in "));
  editor.focus();
  editor.type_text("KON-1");
  for key in ["KON-11", "KON-12", "KON-13"] {
    assert!(editor.stage.paints(key), "{key} is what the function found");
  }
  let first = editor.highlighted()[0].y;
  editor.stage.key("ArrowDown");
  let lit = editor.highlighted();
  assert_eq!(lit.len(), 1);
  assert_eq!(lit[0].y, editor.stage.bounds("KON-12").1);
  editor.stage.key("ArrowUp");
  editor.stage.key("ArrowUp");
  assert_eq!(
    editor.highlighted()[0].y,
    editor.stage.bounds("KON-13").1,
    "round the list"
  );
  editor.stage.key("ArrowDown");
  assert_eq!(editor.highlighted()[0].y, first);
  editor.stage.key("ArrowDown");
  editor.stage.key("Tab");
  assert_eq!(editor.markdown(), "Fixed in [KON-12](https://tracker.example/KON-12)");
  // A space ends a trigger that has none in what follows it.
  editor.type_text(" KON-2 ");
  assert!(!editor.stage.paints("KON-21"));
}

#[test]
fn escape_puts_the_list_away_until_the_caret_leaves_that_trigger() {
  let mut editor = Editor::new(RichText::plain(""));
  editor.focus();
  editor.type_text("[[on");
  assert!(editor.stage.paints("Onboarding"));
  editor.stage.key("Escape");
  assert!(!editor.stage.paints("Onboarding"), "put away");
  assert!(
    editor.stage.tree.focused_element().is_some(),
    "the text keeps the focus"
  );
  editor.type_text("b");
  assert!(!editor.stage.paints("Onboarding"), "still away for this trigger");
  editor.stage.key("Enter");
  assert_eq!(editor.value.get().text, "[[onb\n", "Enter is the text's again");
  // A trigger typed anew opens it again.
  editor.type_text("[[");
  assert!(editor.stage.paints("Onboarding"));
}

#[test]
fn a_click_chooses_an_offer_and_the_caret_leaving_closes_the_list() {
  let mut editor = Editor::new(RichText::plain("Read "));
  editor.focus();
  editor.type_text("[[");
  let (x, y, width, height) = editor.stage.bounds("onboarding");
  let button = lurq::app::events::MouseButton::Left;
  editor.stage.tree.mouse_down(x + width / 2.0, y + height / 2.0, button);
  editor.stage.tree.mouse_up(x + width / 2.0, y + height / 2.0, button);
  editor.stage.pass();
  assert_eq!(editor.markdown(), "Read [[/onboarding]]");
  editor.type_text(" [[");
  assert!(editor.stage.paints("Release notes"));
  // The caret goes back before the trigger: the list closes.
  assert!(editor.editor.select(2, 2));
  editor.stage.pass();
  assert!(!editor.stage.paints("Release notes"));
}

#[test]
fn a_read_only_editor_opens_no_list() {
  let mut editor = Editor::with(RichText::plain("See [[road"), |editor| editor.read_only(true));
  editor.focus();
  assert!(!editor.stage.paints("Roadmap 2027"));
}

#[test]
fn a_trigger_in_a_block_of_code_opens_nothing() {
  let mut editor = Editor::new(RichText::from_markdown("```\nlet a = [[x\n```"));
  editor.focus();
  editor.type_text("[[");
  assert!(!editor.stage.paints("Roadmap 2027"));
}

#[test]
fn a_wiki_link_read_with_titles_is_pressed_to_the_app_and_written_back_as_it_was() {
  let source = "Start at [[/onboarding]], then [[/roadmap|the plan]].";
  let mut editor = Editor::new(RichText::from_markdown_with_wiki_links(source, title));
  assert_eq!(editor.value.get().text, "Start at Onboarding, then the plan.");
  // A click on the link with the modifier tells the app its target.
  let input = editor.editor.input();
  let at = editor.value.get().text.find("Onboarding").unwrap();
  let point = input.caret_point(at + 3).unwrap();
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let (ctrl, meta) = (cfg!(not(target_vendor = "apple")), cfg!(target_vendor = "apple"));
  let button = lurq::app::events::MouseButton::Left;
  let (px, py) = (x + point.x + 18.0, y + point.y + 16.0 + 8.0);
  editor
    .stage
    .tree
    .mouse_down_with_meta(px, py, button, false, ctrl, false, meta);
  editor
    .stage
    .tree
    .mouse_up_with_meta(px, py, button, false, ctrl, false, meta);
  assert_eq!(*editor.opened.lock().unwrap(), ["/onboarding"]);
  // Typed into, the words are the person's: they are written with the link.
  editor.focus();
  let inside = editor.value.get().text.find("Onboarding").unwrap() + "Onboardin".len();
  assert!(editor.editor.select(inside, inside));
  editor.stage.pass();
  editor.type_text("X");
  assert_eq!(
    editor.markdown(),
    "Start at [[/onboarding|OnboardinXg]], then [[/roadmap|the plan]]."
  );
}

#[test]
fn markdown_with_wiki_links_pasted_into_an_editor_with_them_is_links() {
  let mut editor = Editor::new(RichText::plain("See "));
  editor.focus();
  assert!(
    editor
      .stage
      .tree
      .paste(ClipboardContent::text("[[/releases]] and [[/roadmap|plans]]"))
  );
  editor.stage.pass();
  assert_eq!(editor.value.get().text, "See Release notes and plans");
  assert_eq!(editor.markdown(), "See [[/releases]] and [[/roadmap|plans]]");
}
