//! The page of sharing: `app::Share`, an address given to the system's share sheet from a window. Its playground
//! is a Share button with the address, the title and the text it shares; its sections are the button of an app
//! that copies the address where there is no sheet, what each platform shows, and the calls.

use std::sync::Arc;

use lurq::{
  app::{Share, ShareError, component::Component, ctx::Ctx, share},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, icons},
};

use super::kit::samples;
use crate::{
  doc::{Frame, Header, Section},
  kit::{mono, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const ADDRESS: &str = "https://lurq.dev/docs";
const TITLE: &str = "The lurq docs";

/// What a button tells the playground's events.
type Tell = Arc<dyn Fn(String) + Send + Sync>;

/// A Share button: what it shares, and where it says what the window answered.
#[derive(Clone)]
struct ShareButtonProps {
  id: &'static str,
  url: String,
  title: String,
  text: String,
  /// Whether the sheet comes from the button.
  at: bool,
  /// Whether the address is copied where there is no sheet.
  copies: bool,
  answer: Option<Signal<String>>,
  tell: Option<Tell>,
}

impl PartialEq for ShareButtonProps {
  fn eq(&self, other: &Self) -> bool {
    (self.id, &self.url, &self.title, &self.text, self.at, self.copies)
      == (other.id, &other.url, &other.title, &other.text, other.at, other.copies)
  }
}

impl lurq::app::component::DevtoolsInspectable for ShareButtonProps {}

/// A button that shares what its props say through the window it is in.
struct ShareButton {
  button: ElementRef,
}

impl Component for ShareButton {
  type Props = ShareButtonProps;

  fn create(_: &mut Ctx) -> Self {
    Self {
      button: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<ShareButtonProps>().clone();
    // A handler has no `Ctx`: it holds the window.
    let (window, button) = (ctx.window(), self.button.clone());
    Button::new("Share")
      .icon(icons::SHARE)
      .variant(ButtonVariant::Outline)
      .id(props.id)
      .ref_element(self.button.clone())
      .on_click(move |_| {
        let mut shared = Share::url(&props.url);
        if !props.title.is_empty() {
          shared = shared.title(&props.title);
        }
        if !props.text.is_empty() {
          shared = shared.text(&props.text);
        }
        if props.at {
          shared = shared.at(&button);
        }
        let answered = window.share(shared);
        let said = match &answered {
          Ok(()) => "Ok(()): the sheet was asked for".to_owned(),
          Err(ShareError::Unsupported) if props.copies => {
            window.copy_to_clipboard(props.url.clone());
            "Err(Unsupported): no share sheet here, the address was copied".to_owned()
          }
          Err(error) => format!("Err({error:?}): {error}"),
        };
        if let Some(tell) = &props.tell {
          tell(format!("share({:?}) -> {}", props.url, said));
        }
        if let Some(answer) = &props.answer {
          answer.set(said);
        }
      })
  }
}

/// A line of what something says: its name, and the value in a mono face with an id.
fn says(name: &str, value: &str, id: &str) -> Row {
  Row::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(quiet(name, 12.0))
    .child(mono(value, 12.0).id(id))
}

pub struct SharePage {
  frame: Frame,
  playground: Playground,
  url: Signal<String>,
  title: Signal<String>,
  text: Signal<String>,
  at: Signal<bool>,
  /// What the window answered the playground's button, and the section's.
  answer: Signal<String>,
  copied: Signal<String>,
}

impl SharePage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (url, title, text, at) = (self.url.get(), self.title.get(), self.text.get(), self.at.get());
    let answer = self.answer.get();
    let recorder = self.playground.recorder();
    let button = {
      let (url, title, text) = (url.clone(), title.clone(), text.clone());
      let (answer, recorder) = (self.answer.clone(), recorder.clone());
      move |live: bool| ShareButtonProps {
        id: if live { "share-button" } else { "share-button-apart" },
        url: url.clone(),
        title: title.clone(),
        text: text.clone(),
        at,
        copies: false,
        answer: live.then(|| answer.clone()),
        tell: live.then(|| Arc::new(recorder.clone()) as Tell),
      }
    };
    let shown = {
      let button = button.clone();
      move |ctx: &mut Ctx| -> Element {
        Column::new()
          .spacing(12.0)
          .align_items(Alignment::Center)
          .child(ctx.mount::<ShareButton>(button(true)))
          .child(says("answered", &answer, "share-answer"))
          .into()
      }
    };
    // The window of its own shares through that window.
    let plain = move || -> Element { Element::component::<ShareButton>(button(false)) };

    let controls = vec![
      prop("Share::url", "impl Into<String>", field(&self.url)),
      divider(),
      prop("title", "impl Into<String>", field(&self.title)),
      divider(),
      prop("text", "impl Into<String>", field(&self.text)),
      divider(),
      flag("at(&button)", &self.at),
    ];
    let reset = {
      let (url, title, text, at, answer) = (
        self.url.clone(),
        self.title.clone(),
        self.text.clone(),
        self.at.clone(),
        self.answer.clone(),
      );
      Arc::new(move || {
        url.set(ADDRESS.to_owned());
        title.set(TITLE.to_owned());
        text.set(String::new());
        at.set(true);
        answer.set("nothing yet".to_owned());
      })
    };
    let mut code = vec![
      "let (window, button) = (ctx.window(), self.button.clone());".to_owned(),
      "Button::new(\"Share\").ref_element(self.button.clone()).on_click(move |_| {".to_owned(),
      format!("    let share = Share::url({url:?})"),
    ];
    if !title.is_empty() {
      code.push(format!("        .title({title:?})"));
    }
    if !text.is_empty() {
      code.push(format!("        .text({text:?})"));
    }
    if at {
      code.push("        .at(&button)".to_owned());
    }
    if let Some(last) = code.last_mut() {
      last.push(';');
    }
    code.push("    let _ = window.share(share);".to_owned());
    code.push("})".to_owned());
    let tree = vec![
      format!("button   {}", self.playground.measured()),
      "└─ on_click   window.share(Share)   the window's shell shows the system's sheet".to_owned(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Share",
        make: Arc::new(plain),
        shown: Shown::built(shown),
        fill: false,
        forceable: false,
        facts: if share::supported() {
          "the system has a share sheet".to_owned()
        } else {
          "no share sheet here".to_owned()
        },
        event: "share",
        code,
        tree,
        controls,
        changed: [url != ADDRESS, title != TITLE, !text.is_empty(), !at]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for SharePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      url: ctx.signal(ADDRESS.to_owned()),
      title: ctx.signal(TITLE.to_owned()),
      text: ctx.signal(String::new()),
      at: ctx.signal(true),
      answer: ctx.signal("nothing yet".to_owned()),
      copied: ctx.signal("nothing yet".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let platforms = [
      (
        "iOS",
        "UIActivityViewController",
        "",
        "The share sheet, over what the window shows. On an iPad it is a popover that points at `at`, or stands at \
         the middle of the window. The title is the subject of what takes one, a mail.",
      ),
      (
        "macOS",
        "NSSharingServicePicker",
        "",
        "The menu of services, under `at` or at the middle of the window. It has no place for a title.",
      ),
      (
        "Windows",
        "DataTransferManager",
        "",
        "The system's share window, shown for the app's window and placed by the system. It asks for a title, and is \
         given the address where there is none; the text is its description.",
      ),
      (
        "A browser",
        "navigator.share",
        "",
        "Where the browser has it and the page is a secure one, asked within the press that shares.",
      ),
      (
        "Linux",
        "",
        "",
        "There is none: `ShareError::Unsupported`, and nothing is done.",
      ),
    ];
    let rows = [
      (
        "Share::url",
        "impl Into<String>",
        "",
        "The address shared. Only an address with a scheme (`https:`, `mailto:`, an app's own): never a path or a \
         `file:` address, which is `ShareError::NotAnAddress`.",
      ),
      (
        "title",
        "impl Into<String>",
        "",
        "What the address is called, where the system has a place for it.",
      ),
      (
        "text",
        "impl Into<String>",
        "",
        "A text that goes with the address: what a message starts with.",
      ),
      (
        "at / at_rect",
        "&ElementRef / ElementRect",
        "the window's middle",
        "The element the sheet comes from, as it is when the handler runs: an iPad's popover points at it, and a \
         Mac's menu opens under it.",
      ),
      (
        "WindowHandle::share",
        "Share -> Result<(), ShareError>",
        "",
        "Shows the sheet over the window: what a handler calls on the window it holds (`ctx.window()`). `Ok` once \
         the sheet is asked for; what the person does in it is not told.",
      ),
      (
        "Ctx::share",
        "Share -> Result<(), ShareError>",
        "",
        "The same, from a component, for the window it is in.",
      ),
      (
        "share::supported",
        "fn() -> bool",
        "",
        "Whether there is a sheet to show, for an app that shows another button where there is none.",
      ),
      (
        "share::stand_in",
        "Fn(&Share)",
        "",
        "Puts a test's own function in place of the system for the thread the handlers run on: it is given what \
         would have been shared, and no sheet is shown.",
      ),
    ];
    let playground = self.play(ctx);
    let copied = self.copied.get();
    let or_copy = samples().child(
      Column::new()
        .spacing(12.0)
        .child(ctx.mount_keyed::<ShareButton>(
          "or-copy",
          ShareButtonProps {
            id: "share-or-copy",
            url: ADDRESS.to_owned(),
            title: TITLE.to_owned(),
            text: String::new(),
            at: true,
            copies: true,
            answer: Some(self.copied.clone()),
            tell: None,
          },
        ))
        .child(says("answered", &copied, "share-or-copy-answer"))
        .child(says(
          "share::supported()",
          &share::supported().to_string(),
          "share-supported",
        )),
    );
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Share"],
        title: "Share",
        description: "An address given to the system's share sheet: the apps and people the system offers it to, \
                      as an iPhone's Share button does. A window shows the sheet; what the person picks in it is \
                      the system's.",
        import: Some("use lurq::app::{Share, ShareError};"),
        status: Some("New"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Press Share: the window is asked for the system's sheet with the address, the title and the text of \
           the props, and what it answered is under the button. The answer is that the sheet was asked for, or why \
           it was not; what is picked in the sheet is not told to the app.",
          playground,
        ),
        Section::new(
          "Where there is no sheet",
          "Linux has no share sheet, and not every browser has one. The window says so at once, and does nothing \
           else: an app copies the address then, as this button does, or shows another button to begin with.",
          self.frame.demo(
            0,
            or_copy,
            "Err(ShareError::Unsupported)",
            &[
              "let window = ctx.window();",
              "Button::new(\"Share\").on_click(move |_| {",
              "    if window.share(Share::url(address).title(title)) == Err(ShareError::Unsupported) {",
              "        window.copy_to_clipboard(address);",
              "    }",
              "})",
            ],
          ),
        ),
        Section::new(
          "What each platform shows",
          "The sheet is the system's own, and each system has its own.",
          props_table(&platforms, &[], self.frame.room()),
        ),
        Section::new(
          "Props",
          "What is shared, and the calls that share it.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
