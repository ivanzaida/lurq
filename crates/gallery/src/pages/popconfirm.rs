//! The page of the popconfirm: its playground, the question the board asks,
//! and a destructive one.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Row,
  core::{ElementRef, Signal},
  node::Element,
  ui::{Button, ButtonVariant, IconChoice, Popconfirm, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const TITLE: &str = "Delete this file?";
const DESCRIPTION: &str = "It will be moved to the trash.";

/// The icons the playground offers: the warning sign it has when it is given none, another, and none.
const ICONS: [(&str, Option<&str>); 3] = [
  ("warning", Some(icons::WARNING)),
  ("trash", Some(icons::TRASH)),
  ("None", None),
];

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
  icon: Signal<Option<&'static str>>,
  dismiss_on_escape: Signal<bool>,
  dismiss_on_outside_press: Signal<bool>,
  board: (ElementRef, Signal<bool>),
  leave: (ElementRef, Signal<bool>),
}

impl PopconfirmPage {
  /// The playground: the button on the stage asks the question the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (confirm, cancel, destructive) = (self.confirm.get(), self.cancel.get(), self.destructive.get());
    let (icon, on_escape, on_outside) = (
      self.icon.get(),
      self.dismiss_on_escape.get(),
      self.dismiss_on_outside_press.get(),
    );
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
        let mut popconfirm = Popconfirm::new(open, anchor)
          .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
          .icon(match icon {
            Some(name) => IconChoice::from(name),
            None => IconChoice(None),
          })
          .destructive(destructive)
          .dismiss_on_escape(on_escape)
          .dismiss_on_outside_press(on_outside)
          .confirm(if confirm.is_empty() { "Yes" } else { &confirm }, said("confirm"))
          .cancel(if cancel.is_empty() { "No" } else { &cancel }, said("cancel"))
          .on_dismiss(said("on_dismiss"));
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
      "// open: Signal<bool>, anchor: ElementRef".to_owned(),
      "Popconfirm::new(open.clone(), anchor.clone())".to_owned(),
      "    .trigger(Button::new(\"Delete file\").variant(ButtonVariant::Outline))".to_owned(),
    ];
    match icon {
      Some(icons::WARNING) => {}
      Some(_) => code.push("    .icon(icons::TRASH)".to_owned()),
      None => code.push("    .icon(None)".to_owned()),
    }
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if destructive {
      code.push("    .destructive(true)".to_owned());
    }
    if !on_escape {
      code.push("    .dismiss_on_escape(false)".to_owned());
    }
    if !on_outside {
      code.push("    .dismiss_on_outside_press(false)".to_owned());
    }
    code.push(format!("    .confirm({confirm:?}, || ..)"));
    code.push(format!("    .cancel({cancel:?}, || ..)"));
    code.push("    .on_dismiss(|| ..)".to_owned());
    let tree = vec![
      "panel       250 px · radius 8 · shadow md · alertdialog".to_owned(),
      format!(
        "├─ header   {}title 13/500 · description 12",
        icon.map(|name| format!("{name} 16 · ")).unwrap_or_default()
      ),
      format!(
        "└─ footer   {cancel} · {confirm}{}, the focus on {}",
        if destructive { " (destructive)" } else { "" },
        if destructive { "cancel" } else { "confirm" }
      ),
    ];
    let texts = [
      (self.title.clone(), TITLE),
      (self.description.clone(), DESCRIPTION),
      (self.confirm.clone(), "Yes"),
      (self.cancel.clone(), "No"),
    ];
    let (to_destructive, to_icon) = (self.destructive.clone(), self.icon.clone());
    let to_flags = [self.dismiss_on_escape.clone(), self.dismiss_on_outside_press.clone()];
    let set_icon = self.icon.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Popconfirm",
        make: Arc::new(move || make(ElementRef::new(), Signal::new(false), None).into()),
        shown: Shown::of(shown),
        fill: false,
        forceable: false,
        facts: (if self.open.get() { "open" } else { "closed" }).to_owned(),
        event: "confirm / cancel / on_dismiss",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop("confirm", "&str", field(&self.confirm)),
          prop("cancel", "&str", field(&self.cancel)),
          prop(
            "icon",
            "impl Into<IconChoice>",
            self.playground.select(0, &ICONS, &icon, move |icon| set_icon.set(icon)),
          ),
          divider(),
          flag("destructive", &self.destructive),
          flag("dismiss_on_escape", &self.dismiss_on_escape),
          flag("dismiss_on_outside_press", &self.dismiss_on_outside_press),
        ],
        changed: [
          title != TITLE,
          description != DESCRIPTION,
          confirm != "Yes",
          cancel != "No",
          icon != Some(icons::WARNING),
          destructive,
          !on_escape,
          !on_outside,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for (signal, text) in &texts {
            signal.set((*text).to_owned());
          }
          to_destructive.set(false);
          to_icon.set(Some(icons::WARNING));
          for flag in &to_flags {
            flag.set(true);
          }
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
      playground: Playground::new(ctx, 1),
      anchor: ElementRef::new(),
      open: ctx.signal(false),
      title: ctx.signal(TITLE.to_owned()),
      description: ctx.signal(DESCRIPTION.to_owned()),
      confirm: ctx.signal("Yes".to_owned()),
      cancel: ctx.signal("No".to_owned()),
      destructive: ctx.signal(false),
      icon: ctx.signal(Some(icons::WARNING)),
      dismiss_on_escape: ctx.signal(true),
      dismiss_on_outside_press: ctx.signal(true),
      board: (ElementRef::new(), ctx.signal(false)),
      leave: (ElementRef::new(), ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (anchor, open) = &self.board;
    let board = Row::new().child(
      Popconfirm::new(open.clone(), anchor.clone())
        .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
        .title(TITLE)
        .description(DESCRIPTION)
        .confirm("Yes", || {})
        .cancel("No", || {}),
    );
    let (anchor, open) = &self.leave;
    let leave = Row::new().child(
      Popconfirm::new(open.clone(), anchor.clone())
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
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: either button, Escape and a press outside set it to false.",
      ),
      (
        "anchor",
        "impl Into<ElementRef>",
        "",
        "What it hangs from: the trigger, or an element of the app's.",
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
      (
        "icon",
        "impl Into<IconChoice>",
        "warning",
        "An icon of the set before the title, or None for no icon.",
      ),
      (
        "confirm",
        "&str, Fn()",
        "\"Yes\"",
        "The filled button, which has the focus as it opens: it closes the popconfirm, then runs.",
      ),
      (
        "cancel",
        "&str, Fn()",
        "\"No\"",
        "The outlined button, which has the focus as it opens where it is destructive: it closes the popconfirm, \
         then runs. Escape and a press outside run it too.",
      ),
      (
        "destructive",
        "bool",
        "false",
        "The confirm button and the icon in the destructive color, and the focus on the cancel button.",
      ),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Which side of what it hangs from it is on.",
      ),
      (
        "offset",
        "f32, f32",
        "0, 4",
        "How far it is moved, across and down, from where its placement puts it.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where there is no room on its side.",
      ),
      ("width", "impl Into<Length>", "250", "How wide the panel is."),
      ("label", "&str", "the title", "What tooling calls it."),
      (
        "on_dismiss",
        "Fn()",
        "",
        "Runs after a person closed it by Escape or a press outside, after the cancel button's handler; not on \
         an answer. Each one given runs, in order.",
      ),
      ("dismiss_on_escape", "bool", "true", "Whether Escape closes it."),
      (
        "dismiss_on_outside_press",
        "bool",
        "true",
        "Whether a press outside it and what it hangs from closes it.",
      ),
      (
        "inline",
        "bool",
        "false",
        "Draws the panel where it is declared, open or not, without its trigger: for a gallery or a test.",
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
          "As it opens the focus goes to the confirm button. Escape and a press outside close it, as the cancel \
           button does.",
          self.frame.demo(
            0,
            board,
            "Popconfirm::new(open, anchor).trigger(..)",
            &[
              "// open: Signal<bool>, anchor: ElementRef",
              "Popconfirm::new(open.clone(), anchor.clone())",
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
          "What cannot be undone is confirmed in the destructive color, and as it opens the focus goes to the \
           cancel button, so that Enter does not remove what was not read.",
          self.frame.demo(
            1,
            leave,
            ".destructive(true)",
            &[
              "// open: Signal<bool>, anchor: ElementRef",
              "Popconfirm::new(open.clone(), anchor.clone())",
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
          props_table(&rows, &["open", "anchor"], self.frame.room()),
        ),
      ],
    )
  }
}
