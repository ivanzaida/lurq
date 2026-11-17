//! The page of the path field: its playground, the board's field whose button the page answers, a field that
//! opens the system's dialog, its states and sizes, and a path field in a form's field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  files::FileDialog,
  layout::Alignment,
  node::Element,
  ui::{Field, Interaction, PathField, PathFieldSize, PathKind},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// What the playground's field picks, as the panel names it, and how the code names it.
const KINDS: [(&str, Option<PathKind>, &str); 4] = [
  ("none", None, ""),
  ("Folder", Some(PathKind::Folder), "PathKind::Folder"),
  ("File", Some(PathKind::File), "PathKind::File"),
  ("SaveFile", Some(PathKind::SaveFile), "PathKind::SaveFile"),
];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(PathFieldSize, &str, &str); 3] = [
  (PathFieldSize::Sm, "sm", "PathFieldSize::Sm"),
  (PathFieldSize::Md, "md", "PathFieldSize::Md"),
  (PathFieldSize::Lg, "lg", "PathFieldSize::Lg"),
];

/// The path the board's field holds.
const BOARD: &str = "H:\\projects\\lurq\\pencil";

/// Something on a stage with what it is under it, as wide as the stage has room for.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(9.0)
    .child(Row::new().width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, one under the other.
fn items() -> Column {
  Column::new().width(FILL).spacing(24.0).align_items(Alignment::Start)
}

/// A path field that holds `path`: for a stage that shows a state.
fn showing(path: &str) -> PathField {
  PathField::new(Signal::new(path.to_owned()))
}

pub struct PathFieldPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  browse_label: Signal<String>,
  kind: Signal<usize>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  /// The board's field: its path, and how often its button was pressed.
  folder: Signal<String>,
  asked: Signal<u32>,
  /// The fields that open the system's dialog.
  project: Signal<String>,
  report: Signal<String>,
  /// The field in a form's field.
  output: Signal<String>,
}

impl PathFieldPage {
  /// The playground: the path field as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placeholder, browse_label) = (self.placeholder.get(), self.browse_label.get());
    let (kind_name, kind, kind_code) = KINDS[self.kind.get().min(KINDS.len() - 1)];
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, browse_label, value) = (placeholder.clone(), browse_label.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let field = PathField::new(value.clone())
          .placeholder(&placeholder)
          .browse_label(&browse_label)
          .size(size)
          .invalid(set[0])
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3]);
        match kind {
          Some(kind) => field.picks(kind),
          None => field,
        }
      }
    };
    let (browsed, picked) = (self.playground.recorder(), self.playground.recorder());
    let shown = make()
      .force(self.playground.forced())
      .on_browse(move || browsed("on_browse()".to_owned()))
      .on_pick(move |path| picked(format!("on_pick({path:?})")));
    // A window of its own opens the dialog too.
    let mounted = make.clone();
    self.playground.window_shows(move |ctx| mounted().mount(ctx));

    let mut code = vec![
      "// path: Signal<String>".to_owned(),
      "PathField::new(path.clone())".to_owned(),
    ];
    if !placeholder.is_empty() {
      code.push(format!("    .placeholder({placeholder:?})"));
    }
    if browse_label != "Browse…" {
      code.push(format!("    .browse_label({browse_label:?})"));
    }
    if kind.is_some() {
      code.push(format!("    .picks({kind_code})"));
    }
    if size != PathFieldSize::Sm {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .on_browse(|| ..)".to_owned());
    if kind.is_some() {
      code.push("    .on_pick(|path| ..)".to_owned());
      code.push("    .mount(ctx)".to_owned());
    }

    let height = match size {
      PathFieldSize::Sm => 32,
      PathFieldSize::Md => 36,
      PathFieldSize::Lg => 40,
    };
    let tree = vec![
      format!("root        {}   8 apart", self.playground.measured()),
      format!("├─ field    the input: {height} high · the room the button leaves"),
      format!(
        "│  ├─ icon     16, muted   {}",
        match kind {
          Some(PathKind::File | PathKind::SaveFile) => "file",
          _ => "folder",
        }
      ),
      "│  └─ control  the path, mono".to_owned(),
      format!("└─ button   outline · {height} high   {browse_label:?}"),
    ];

    let kinds: Vec<(&str, usize)> = KINDS.iter().enumerate().map(|(index, kind)| (kind.0, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_kind, to_size) = (self.kind.clone(), self.size.clone());
    let reset = {
      let (value, placeholder, browse_label, kind, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.browse_label.clone(),
        self.kind.clone(),
        self.size.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      ];
      move || {
        value.set(BOARD.to_owned());
        placeholder.set("Choose a folder".to_owned());
        browse_label.set("Browse…".to_owned());
        kind.set(0);
        size.set(0);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(kind.is_some())
      + usize::from(size != PathFieldSize::Sm)
      + usize::from(self.value.get() != BOARD)
      + usize::from(placeholder != "Choose a folder")
      + usize::from(browse_label != "Browse…");
    self.playground.card(
      ctx,
      Spec {
        title: "Path Field",
        make: Arc::new(move || make().into()),
        shown: Shown::built(move |ctx| shown.mount(ctx)),
        fill: on("full_width"),
        forceable: true,
        facts: format!("{size_name} · picks {kind_name}"),
        event: "on_browse",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("browse_label", "&str", field(&self.browse_label)),
          prop(
            "picks",
            "PathKind",
            self
              .playground
              .select(0, &kinds, &self.kind.get(), move |index| to_kind.set(index)),
          ),
          prop(
            "size",
            "PathFieldSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for PathFieldPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      value: ctx.signal(BOARD.to_owned()),
      placeholder: ctx.signal("Choose a folder".to_owned()),
      browse_label: ctx.signal("Browse…".to_owned()),
      kind: ctx.signal(1),
      size: ctx.signal(0),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      folder: ctx.signal(BOARD.to_owned()),
      asked: ctx.signal(0),
      project: ctx.signal(String::new()),
      report: ctx.signal(String::new()),
      output: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's field: the page answers its button, and counts the presses.
    let asked = self.asked.clone();
    let times = self.asked.get();
    let board = items().child(item(
      PathField::new(self.folder.clone()).on_browse(move || asked.update(|asked| *asked += 1)),
      &format!(
        "the signal has: \"{}\" · Browse… pressed {times} {}",
        self.folder.get(),
        if times == 1 { "time" } else { "times" }
      ),
    ));

    // Fields that open the system's dialog: mounted, and told what their path is of.
    let dialogs = items()
      .child(item(
        PathField::new(self.project.clone())
          .placeholder("Where the project is kept")
          .picks(PathKind::Folder)
          .dialog(FileDialog::new().title("Project folder"))
          .mount(ctx),
        "a folder: the dialog opens where the path is; a folder dropped on the field is taken too",
      ))
      .child(item(
        PathField::new(self.report.clone())
          .placeholder("Where the report is written")
          .picks(PathKind::SaveFile)
          .browse_label("Save as…")
          .dialog(
            FileDialog::new()
              .title("Save the report")
              .filter("Markdown", &["md"])
              .file_name("report.md"),
          )
          .mount(ctx),
        "a file to save: the icon of a file, a button that says what it does",
      ));

    let states = items()
      .child(item(showing("").placeholder("Choose a folder"), "empty"))
      .child(item(showing(BOARD), "with a path"))
      .child(item(showing(BOARD).force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("H:\\projects\\nowhere").invalid(true), "invalid"))
      .child(item(
        showing(BOARD).read_only(true),
        "read only: the button takes nothing",
      ))
      .child(item(showing(BOARD).disabled(true), "disabled"));

    let mut sizes = items();
    for (size, name, _) in SIZES {
      sizes = sizes.child(item(showing(BOARD).size(size), name));
    }

    let mut output = Field::new(
      "Output folder",
      PathField::new(self.output.clone())
        .picks(PathKind::Folder)
        .full_width(true),
    )
    .description("Builds are written here.")
    .required(true)
    .full_width(true);
    if self.output.get().trim().is_empty() {
      output = output.error("Choose where builds are written");
    }
    let in_field = items().child(output.mount(ctx));

    let rows = [
      (
        "value",
        "Signal<String>",
        "",
        "PathField::new: the path it shows, and what is typed or chosen is set on.",
      ),
      (
        "on_browse",
        "impl Fn()",
        "",
        "Called on a press of the button, before the system's dialog opens where the field opens one.",
      ),
      (
        "picks",
        "PathKind",
        "",
        "What the path is of: Folder, File or SaveFile. Mounted, the button then opens the system's dialog for \
         it and sets what is chosen; a drop on the field is taken where it is of that kind.",
      ),
      (
        "dialog",
        "FileDialog",
        "",
        "The dialog as the button opens it: its title, its filters, the name a file to save starts with. It \
         opens where the path is unless it has a folder of its own.",
      ),
      (
        "on_pick",
        "impl Fn(&str)",
        "",
        "Called with the path chosen in the dialog or dropped on the field, once the signal is set.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Puts it where it goes, keeping the dialog it opens. Not mounted, the button tells on_browse and opens \
         nothing.",
      ),
      (
        "browse_label",
        "&str",
        "\"Browse…\"",
        "What the button says, and is called.",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "folder",
        "The glyph at the start of the box: a file for a field that picks one. None for no icon.",
      ),
      ("placeholder", "&str", "", "What the field says while it is empty."),
      (
        "size",
        "PathFieldSize",
        "Sm",
        "Sm 32, Md 36, Lg 40: the input's and the button's. The board's is the small one.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "What the app says of the path: the edge of the box is in the judgment's color.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "The path can be read and copied and not changed: the button takes nothing either.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Neither the field nor its button takes input.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 440.",
      ),
      (
        "on_input / on_submit",
        "impl Fn(..)",
        "",
        "As an input has them: an edit before it is made, and Enter in the field with the path typed.",
      ),
      (
        "label",
        "&str",
        "",
        "What the field is called to what reads the tree. A Field gives it its label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Path Field"],
        title: "Path Field",
        description: "The path of a folder or a file in an input, with a button beside it that browses for one: \
                      it tells the app, or opens the system's dialog and sets what is chosen.",
        import: Some("use lurq::ui::{PathField, PathKind};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of PathField is a control on the right. With picks set, the field on the stage is mounted: \
           its button opens the system's dialog, and what is chosen is the signal's.",
          playground,
        ),
        Section::new(
          "Path field",
          "The board's: a path in the mono type after a folder, and Browse… 8 after it. The button tells the app, \
           which opens what it likes and sets the signal; the path is typed in as in any input.",
          self.frame.demo(
            0,
            board,
            "PathField::new(path).on_browse(..)",
            &[
              "// path: Signal<String>",
              "PathField::new(path.clone()).on_browse(move || choose_a_folder(&path))",
            ],
          ),
        ),
        Section::new(
          "The system's dialog",
          "A field that says what its path is of, mounted, opens the system's dialog itself: for a folder, a file \
           to open, or a file to save. The dialog starts where the path is, what is chosen is set on the signal, \
           and on_pick hears of it. A folder or a file dragged from the system onto the field is its path too.",
          self.frame.demo(
            1,
            dialogs,
            ".picks(PathKind::Folder).mount(ctx)",
            &[
              "PathField::new(project.clone())",
              "    .placeholder(\"Where the project is kept\")",
              "    .picks(PathKind::Folder)",
              "    .dialog(FileDialog::new().title(\"Project folder\"))",
              "    .mount(ctx)",
              "",
              "PathField::new(report.clone())",
              "    .picks(PathKind::SaveFile)",
              "    .browse_label(\"Save as…\")",
              "    .dialog(FileDialog::new().title(\"Save the report\").filter(\"Markdown\", &[\"md\"]).file_name(\"report.md\"))",
              "    .on_pick(|path| write_to(path))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The states are the input's: the edge of the box says them. A field that is read only keeps its path \
           selectable, and its button takes nothing; a disabled one is dimmed with its button.",
          self.frame.demo(
            2,
            states,
            ".invalid(true)",
            &[
              "PathField::new(path.clone()).placeholder(\"Choose a folder\")",
              "PathField::new(path.clone()).invalid(true)",
              "PathField::new(path.clone()).read_only(true)",
              "PathField::new(path.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The input's sizes, which the button follows. The board draws the small one, which a path field has \
           unless told.",
          self.frame.demo(
            3,
            sizes,
            ".size(PathFieldSize::Md)",
            &[
              "PathField::new(path.clone())                       // Sm: 32",
              "PathField::new(path.clone()).size(PathFieldSize::Md)   // 36",
              "PathField::new(path.clone()).size(PathFieldSize::Lg)   // 40",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A path field is a control a Field holds: called by the field's label, judged as the field is. Empty, \
           this one is wrong; choose a folder and it is not.",
          self.frame.demo(
            4,
            in_field,
            "Field::new(\"Output folder\", PathField::new(path))",
            &[
              "let mut field = Field::new(",
              "    \"Output folder\",",
              "    PathField::new(output.clone()).picks(PathKind::Folder).full_width(true),",
              ")",
              ".description(\"Builds are written here.\")",
              ".required(true);",
              "if output.get().is_empty() {",
              "    field = field.error(\"Choose where builds are written\");",
              "}",
              "field.mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a path field is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
