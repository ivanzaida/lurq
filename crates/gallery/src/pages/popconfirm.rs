//! The page of the popconfirm: its playground, the question the board asks,
//! and a destructive one.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Row,
  core::{ElementRef, Signal},
  node::Element,
  ui::{Button, ButtonVariant, Popconfirm, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Spec, divider, field, flag, prop},
};

const TITLE: &str = "Delete this file?";
const DESCRIPTION: &str = "It will be moved to the trash.";

pub struct PopconfirmPage {
  frame: Frame,
  playground: Playground,
  anchor: ElementRef,
  open: Signal<bool>,
  title: Signal<String>,
  description: Signal<String>,
  confirm: Signal<String>,
  cancel: Signal<String>,
  destructive: Signal<bool>,
  board: (ElementRef, Signal<bool>),
  leave: (ElementRef, Signal<bool>),
}

impl PopconfirmPage {
  /// The playground: the button on the stage asks the question the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (confirm, cancel, destructive) = (self.confirm.get(), self.cancel.get(), self.destructive.get());
    let record = self.playground.recorder();
    let make = {
      let (title, description, confirm, cancel) = (title.clone(), description.clone(), confirm.clone(), cancel.clone());
      move |anchor: ElementRef, open: Signal<bool>, record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let said = |what: &'static str| {
          let record = record.clone();
          move || {
            if let Some(record) = &record {
              record(format!("{what}()"));
            }
          }
        };
        let mut popconfirm = Popconfirm::new(anchor, open)
          .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
          .destructive(destructive)
          .confirm(if confirm.is_empty() { "Yes" } else { &confirm }, said("confirm"))
          .cancel(if cancel.is_empty() { "No" } else { &cancel }, said("cancel"));
        if !title.is_empty() {
          popconfirm = popconfirm.title(&title);
        }
        if !description.is_empty() {
          popconfirm = popconfirm.description(&description);
        }
        popconfirm
      }
    };
    let recorded: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
    let shown = make(self.anchor.clone(), self.open.clone(), Some(recorded));

    let mut code = vec![
      "// anchor: ElementRef, open: Signal<bool>".to_owned(),
      "Popconfirm::new(anchor.clone(), open.clone())".to_owned(),
      "    .trigger(Button::new(\"Delete file\").variant(ButtonVariant::Outline))".to_owned(),
    ];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if destructive {
      code.push("    .destructive(true)".to_owned());
    }
    code.push(format!("    .confirm({confirm:?}, || ..)"));
    code.push(format!("    .cancel({cancel:?}, || ..)"));
    let tree = vec![
      "popover     250 px · radius 8 · shadow md · alertdialog".to_owned(),
      "├─ message  warning 16 · title 13/500 · description 12".to_owned(),
      format!(
        "└─ actions  {cancel} · {confirm}{}",
        if destructive { " (destructive)" } else { "" }
      ),
    ];
    let texts = [
      (self.title.clone(), TITLE),
      (self.description.clone(), DESCRIPTION),
      (self.confirm.clone(), "Yes"),
      (self.cancel.clone(), "No"),
    ];
    let to_destructive = self.destructive.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Popconfirm",
        make: Arc::new(move || make(ElementRef::new(), Signal::new(false), None).into()),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: (if self.open.get() { "open" } else { "closed" }).to_owned(),
        event: "confirm / cancel",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop("confirm", "&str", field(&self.confirm)),
          prop("cancel", "&str", field(&self.cancel)),
          divider(),
          flag("destructive", &self.destructive),
        ],
        changed: [
          title != TITLE,
          description != DESCRIPTION,
          confirm != "Yes",
          cancel != "No",
          destructive,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for (signal, text) in &texts {
            signal.set((*text).to_owned());
          }
          to_destructive.set(false);
        }),
      },
    )
  }
}

impl Component for PopconfirmPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      anchor: ElementRef::new(),
      open: ctx.signal(false),
      title: ctx.signal(TITLE.to_owned()),
      description: ctx.signal(DESCRIPTION.to_owned()),
      confirm: ctx.signal("Yes".to_owned()),
      cancel: ctx.signal("No".to_owned()),
      destructive: ctx.signal(false),
      board: (ElementRef::new(), ctx.signal(false)),
      leave: (ElementRef::new(), ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (anchor, open) = &self.board;
    let board = Row::new().child(
      Popconfirm::new(anchor.clone(), open.clone())
        .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
        .title(TITLE)
        .description(DESCRIPTION)
        .confirm("Yes", || {})
        .cancel("No", || {}),
    );
    let (anchor, open) = &self.leave;
    let leave = Row::new().child(
      Popconfirm::new(anchor.clone(), open.clone())
        .trigger(
          Button::new("Leave channel")
            .variant(ButtonVariant::Ghost)
            .icon(icons::SIGN_OUT),
        )
        .title("Leave #design?")
        .description("You will need an invitation to join again.")
        .destructive(true)
        .confirm("Leave", || {})
        .cancel("Stay", || {}),
    );
    let rows = [
      (
        "anchor",
        "impl Into<ElementRef>",
        "",
        "What it hangs from: the trigger, or an element of the app's.",
      ),
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: either button, Escape and a press outside set it to false.",
      ),
      (
        "trigger",
        "impl Into<Element>",
        "",
        "A button that asks: it turns the signal.",
      ),
      (
        "title",
        "&str",
        "",
        "The question, 13/500. It is what the popconfirm is called.",
      ),
      ("description", "&str", "", "More of it, 12 in the muted color."),
      ("icon", "&str", "warning", "An icon of the set before the title."),
      (
        "confirm",
        "&str, Fn()",
        "\"Yes\"",
        "The filled button: it closes the popconfirm, then runs.",
      ),
      (
        "cancel",
        "&str, Fn()",
        "\"No\"",
        "The outlined button, which has the focus as it opens: it closes the popconfirm, then runs. Escape runs it too.",
      ),
      (
        "destructive",
        "bool",
        "false",
        "The confirm button and the icon in the destructive color.",
      ),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Which side of what it hangs from it is on.",
      ),
      (
        "offset",
        "impl Into<SpacingValue>",
        "4",
        "How far it is from what it hangs from.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Popconfirm"],
        title: "Popconfirm",
        description: "A question in a small panel hung from what was pressed, answered by one of two buttons: \
                      delete a file, leave a channel.",
        import: Some("use lurq::ui::Popconfirm;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Popconfirm is a control on the right. The button on the stage asks the question they make; \
           the Events tab says which button answered it.",
          playground,
        ),
        Section::new(
          "Popconfirm",
          "As it opens the focus goes to the cancel button, so that Enter does not confirm what was not read. \
           Escape and a press outside close it.",
          self.frame.demo(
            0,
            board,
            "Popconfirm::new(anchor, open).trigger(..)",
            &[
              "// anchor: ElementRef, open: Signal<bool>",
              "Popconfirm::new(anchor.clone(), open.clone())",
              "    .trigger(Button::new(\"Delete file\").variant(ButtonVariant::Outline))",
              "    .title(\"Delete this file?\")",
              "    .description(\"It will be moved to the trash.\")",
              "    .confirm(\"Yes\", || {})",
              "    .cancel(\"No\", || {})",
            ],
          ),
        ),
        Section::new(
          "Destructive",
          "What cannot be undone is confirmed in the destructive color.",
          self.frame.demo(
            1,
            leave,
            ".destructive(true)",
            &[
              "// anchor: ElementRef, open: Signal<bool>",
              "Popconfirm::new(anchor.clone(), open.clone())",
              "    .trigger(Button::new(\"Leave channel\").variant(ButtonVariant::Ghost).icon(icons::SIGN_OUT))",
              "    .title(\"Leave #design?\")",
              "    .description(\"You will need an invitation to join again.\")",
              "    .destructive(true)",
              "    .confirm(\"Leave\", || {})",
              "    .cancel(\"Stay\", || {})",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a popconfirm is given.",
          props_table(&rows, &["anchor", "open"], self.frame.room()),
        ),
      ],
    )
  }
}
