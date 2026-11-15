//! The page of the drop zone: its playground, a zone the page hears, what it shows of a drag, what it takes, and
//! its words.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{DropZone, DropZoneState, Interaction, RejectReason, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// What the zone of the playground accepts, and how the code says it.
const ACCEPTS: [(&str, &[&str], &str); 4] = [
  ("anything", &[], ""),
  ("images", &["image/*"], "[\"image/*\"]"),
  ("PDF", &["pdf"], "[\"pdf\"]"),
  ("text and JSON", &["text/*", "json"], "[\"text/*\", \"json\"]"),
];

/// The greatest sizes the playground offers.
const SIZES: [(&str, Option<u64>, &str); 3] = [
  ("any", None, ""),
  ("1 MB", Some(1024 * 1024), "1024 * 1024"),
  ("5 MB", Some(5 * 1024 * 1024), "5 * 1024 * 1024"),
];

/// The drag the playground draws, with none over the zone.
const DRAGS: [(&str, Option<DropZoneState>, &str); 3] = [
  ("none", None, ""),
  ("taken", Some(DropZoneState::DragOver), "DropZoneState::DragOver"),
  ("refused", Some(DropZoneState::Rejected), "DropZoneState::Rejected"),
];

fn why(reason: RejectReason) -> &'static str {
  match reason {
    RejectReason::Kind => "not of a kind it takes",
    RejectReason::TooLarge => "too large",
    RejectReason::TooMany => "one too many",
    RejectReason::Folder => "a folder",
  }
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

pub struct DropZonePage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  accept: Signal<usize>,
  max_size: Signal<usize>,
  drag: Signal<usize>,
  multiple: Signal<bool>,
  icon: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  /// What the page's own zone was given, and what it refused.
  given: Signal<Vec<String>>,
}

impl DropZonePage {
  /// The playground: the zone as the panel's props make it, mounted, its handlers told in the Events tab.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (accept_name, accepted, accept_code) = ACCEPTS[self.accept.get().min(ACCEPTS.len() - 1)];
    let (size_name, max_size, size_code) = SIZES[self.max_size.get().min(SIZES.len() - 1)];
    let (drag_name, drag, drag_code) = DRAGS[self.drag.get().min(DRAGS.len() - 1)];
    let flags = [
      ("multiple", self.multiple.get()),
      ("icon", self.icon.get()),
      ("invalid", self.invalid.get()),
      ("disabled", self.disabled.get()),
    ];
    let make = {
      let (title, description) = (title.clone(), description.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut zone = DropZone::new()
          .title(&title)
          .description(&description)
          .accept(accepted.iter().copied())
          .multiple(set[0])
          .invalid(set[2])
          .disabled(set[3]);
        if !set[1] {
          zone = zone.icon(None);
        }
        if let Some(max) = max_size {
          zone = zone.max_size(max);
        }
        if let Some(state) = drag {
          zone = zone.state(state, true);
        }
        zone
      }
    };
    let (told, refused) = (self.playground.recorder(), self.playground.recorder());
    let zone = make()
      .force(self.playground.forced())
      .on_files(move |files| {
        // The whole path where the platform gives one (a drop or a choice on a desktop), the name where it
        // does not (a browser).
        let names: Vec<String> = files
          .iter()
          .map(|file| {
            file
              .path()
              .map_or_else(|| file.name().to_owned(), |path| path.display().to_string())
          })
          .collect();
        told(format!("on_files([{}])", names.join(", ")))
      })
      .on_reject(move |rejected| {
        let names: Vec<String> = rejected
          .iter()
          .map(|file| format!("{}: {:?}", file.file.name(), file.reason))
          .collect();
        refused(format!("on_reject([{}])", names.join(", ")))
      });

    let mut code = vec!["DropZone::new()".to_owned()];
    if title != "Drop files here" {
      code.push(format!("    .title({title:?})"));
    }
    if description != "or click to browse" {
      code.push(format!("    .description({description:?})"));
    }
    if !accept_code.is_empty() {
      code.push(format!("    .accept({accept_code})"));
    }
    if !size_code.is_empty() {
      code.push(format!("    .max_size({size_code})"));
    }
    if !flags[0].1 {
      code.push("    .multiple(false)".to_owned());
    }
    if !flags[1].1 {
      code.push("    .icon(None)".to_owned());
    }
    for (name, set) in &flags[2..] {
      if *set {
        code.push(format!("    .{name}(true)"));
      }
    }
    if !drag_code.is_empty() {
      code.push(format!("    .state({drag_code}, true)"));
    }
    code.push("    .on_files(|files| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root          {}", self.playground.measured()),
      format!(
        "├─ icon      {}",
        if flags[1].1 { "upload, 20 × 20 px" } else { "none" }
      ),
      "└─ text".to_owned(),
      format!("   ├─ title        {title:?}"),
      format!("   └─ description  {description:?}"),
    ];

    let accepts: Vec<(&str, usize)> = ACCEPTS
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let drags: Vec<(&str, usize)> = DRAGS.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let (to_accept, to_size, to_drag) = (self.accept.clone(), self.max_size.clone(), self.drag.clone());
    let reset = {
      let (title, description, accept, size, drag) = (
        self.title.clone(),
        self.description.clone(),
        self.accept.clone(),
        self.max_size.clone(),
        self.drag.clone(),
      );
      let (multiple, icon, invalid, disabled) = (
        self.multiple.clone(),
        self.icon.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
      );
      move || {
        title.set("Drop files here".to_owned());
        description.set("or click to browse".to_owned());
        accept.set(0);
        size.set(0);
        drag.set(0);
        multiple.set(true);
        icon.set(true);
        invalid.set(false);
        disabled.set(false);
      }
    };
    let changed = usize::from(title != "Drop files here")
      + usize::from(description != "or click to browse")
      + usize::from(!accept_code.is_empty())
      + usize::from(!size_code.is_empty())
      + usize::from(drag.is_some())
      + usize::from(!flags[0].1)
      + usize::from(!flags[1].1)
      + flags[2..].iter().filter(|(_, set)| *set).count();
    self.playground.card(
      ctx,
      Spec {
        title: "Drop Zone",
        make: Arc::new(move || make().into()),
        shown: Shown::built(move |ctx| zone.mount(ctx)),
        fill: false,
        forceable: true,
        facts: format!("{accept_name} · {size_name} · drag {drag_name}"),
        event: "on_files",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "accept",
            "impl IntoIterator<Item = &str>",
            self
              .playground
              .select(0, &accepts, &self.accept.get(), move |index| to_accept.set(index)),
          ),
          prop(
            "max_size",
            "u64",
            self
              .playground
              .select(1, &sizes, &self.max_size.get(), move |index| to_size.set(index)),
          ),
          prop(
            "state",
            "DropZoneState",
            self
              .playground
              .select(2, &drags, &self.drag.get(), move |index| to_drag.set(index)),
          ),
          divider(),
          flag("multiple", &self.multiple),
          flag("icon", &self.icon),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for DropZonePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      title: ctx.signal("Drop files here".to_owned()),
      description: ctx.signal("or click to browse".to_owned()),
      accept: ctx.signal(0),
      max_size: ctx.signal(0),
      drag: ctx.signal(0),
      multiple: ctx.signal(true),
      icon: ctx.signal(true),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      given: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let given = self.given.get();
    let (taken, refused) = (self.given.clone(), self.given.clone());
    let heard = DropZone::new()
      .accept(["image/*"])
      .max_size(5 * 1024 * 1024)
      .title("Drop images here")
      .description("PNG or JPG, up to 5 MB")
      .on_files(move |files| {
        taken.update(|given| given.extend(files.iter().map(|file| format!("{} — taken", file.name()))))
      })
      .on_reject(move |rejected| {
        refused.update(|given| {
          given.extend(
            rejected
              .iter()
              .map(|file| format!("{} — refused: {}", file.file.name(), why(file.reason))),
          )
        })
      })
      .mount(ctx);
    let mut said = Column::new().spacing(4.0);
    if given.is_empty() {
      said = said.child(quiet(
        "nothing yet: drop a file from the system on it, or press it",
        12.0,
      ));
    }
    for line in given.iter().rev().take(6) {
      said = said.child(quiet(line, 12.0));
    }
    let live = items().child(Column::new().spacing(12.0).child(heard).child(said));

    let states = items()
      .child(item(DropZone::new(), "at rest"))
      .child(item(DropZone::new().force(Some(Interaction::Hovered)), "hover"))
      .child(item(DropZone::new().force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(
        DropZone::new().state(DropZoneState::DragOver, true),
        "a drag it takes",
      ))
      .child(item(
        DropZone::new().state(DropZoneState::Rejected, true),
        "a drag it refuses",
      ))
      .child(item(DropZone::new().invalid(true), "invalid"))
      .child(item(DropZone::new().disabled(true), "disabled"));
    let words = items()
      .child(item(
        DropZone::new()
          .icon(icons::IMAGE)
          .title("Drop a photo")
          .description("PNG or JPG, up to 5 MB"),
        "its own icon and words",
      ))
      .child(item(
        DropZone::new()
          .title("Drop the photographs of the whole trip here, every one of them")
          .no_description(),
        "a title longer than the zone wraps",
      ))
      .child(item(
        DropZone::new()
          .state(DropZoneState::Rejected, true)
          .reject_text("Only images, up to 5 MB"),
        "what a refused drag says",
      ));

    let rows = [
      (
        "accept",
        "impl IntoIterator<Item = &str>",
        "anything",
        "Extensions (\"png\", \".png\") and kinds (\"image/*\", \"text/*\").",
      ),
      (
        "multiple",
        "bool",
        "true",
        "More than one file at once; past one, the rest are refused as too many.",
      ),
      ("max_size", "u64", "", "The greatest size of a file, in bytes."),
      ("max_files", "usize", "", "The most files of one drop or choice."),
      (
        "icon",
        "impl Into<IconChoice>",
        "upload",
        "The icon over its words, or None.",
      ),
      (
        "title / description",
        "&str",
        "\"Drop files here\" / \"or click to browse\"",
        "What it says.",
      ),
      ("no_description", "", "", "No words under the title."),
      (
        "drop_text / reject_text",
        "&str",
        "\"Drop to add\" / \"These files can't be added\"",
        "What it says while files are dragged over it.",
      ),
      (
        "label",
        "&str",
        "its title",
        "What it is called to tooling, and its dialog's title.",
      ),
      (
        "invalid / disabled",
        "bool",
        "false",
        "Its edge destructive; or it takes nothing, dimmed.",
      ),
      (
        "on_files",
        "impl Fn(&[DroppedFile])",
        "",
        "Told the files it took, from a drop or its dialog.",
      ),
      (
        "on_reject",
        "impl Fn(&[RejectedFile])",
        "",
        "Told the files it refused, and why.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Shows a drag over it and opens the dialog on a press, Enter or Space.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Drop Zone"],
        title: "Drop Zone",
        description: "Where files are dropped from the system, or chosen in its file dialog: it says what a drag \
                      over it will do, takes what it accepts and tells the app what it refused.",
        import: Some("use lurq::ui::DropZone;"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DropZone is a control on the right. The zone on the stage is mounted: drop a file from the \
           system on it, or press it to choose one, and the Events tab says what it took and what it refused.",
          playground,
        ),
        Section::new(
          "Drop Zone",
          "A zone for images up to 5 MB. What it took and what it refused, and why, are said under it.",
          self.frame.demo(
            0,
            live,
            "DropZone::new().accept([\"image/*\"]).max_size(5 * 1024 * 1024)",
            &[
              "DropZone::new()",
              "    .accept([\"image/*\"])",
              "    .max_size(5 * 1024 * 1024)",
              "    .title(\"Drop images here\")",
              "    .description(\"PNG or JPG, up to 5 MB\")",
              "    .on_files(|files| ..)",
              "    .on_reject(|refused| ..)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "A drag it takes is drawn in the info colors and says what the drop does; one it takes nothing of, in the \
           destructive colors. A theme draws these with DropZoneState, as the stage does here.",
          self.frame.demo(
            1,
            states,
            ".state(DropZoneState::DragOver, true)",
            &[
              "DropZone::new()",
              "DropZone::new().force(Some(Interaction::Hovered))",
              "DropZone::new().force(Some(Interaction::FocusVisible))",
              "DropZone::new().state(DropZoneState::DragOver, true)",
              "DropZone::new().state(DropZoneState::Rejected, true)",
              "DropZone::new().invalid(true)",
              "DropZone::new().disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Its words",
          "Its icon, its title and what it says under it are the app's. A title longer than the zone wraps, and the \
           zone grows.",
          self.frame.demo(
            2,
            words,
            ".title(\"Drop a photo\")",
            &[
              "DropZone::new().icon(icons::IMAGE).title(\"Drop a photo\").description(\"PNG or JPG, up to 5 MB\")",
              "DropZone::new().title(\"Drop the photographs of the whole trip here, every one of them\").no_description()",
              "DropZone::new().reject_text(\"Only images, up to 5 MB\")",
            ],
          ),
        ),
        Section::new("Props", "What a drop zone is given.", props_table(&rows, &[], self.frame.room())),
      ],
    )
  }
}
