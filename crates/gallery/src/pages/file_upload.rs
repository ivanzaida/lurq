//! The page of the file upload: its playground, whose upload a timer plays, the board's list, each state of a
//! file, the upload with a drop zone, and a long name.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
  },
  components::{Column, Row},
  core::Signal,
  files::DroppedFile,
  layout::Alignment,
  node::Element,
  ui::{FileUpload, UploadFile, UploadState},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// What the upload of the playground accepts, and how the code says it.
const ACCEPTS: [(&str, &[&str], &str); 3] = [
  ("anything", &[], ""),
  ("images", &["image/*"], "[\"image/*\"]"),
  ("text and JSON", &["text/*", "json"], "[\"text/*\", \"json\"]"),
];

/// Where the playground's dialog chooses files from, and how the code says it: on a phone the menu before the dialog
/// has these, and one alone opens at once.
pub(super) fn sources() -> Vec<(&'static str, Vec<lurq::files::FileSource>, &'static str)> {
  use lurq::files::FileSource::{Files, PhotoLibrary};
  #[allow(unused_mut)]
  let mut sources = vec![
    ("what it accepts", vec![], ""),
    ("photo library", vec![PhotoLibrary], "[FileSource::PhotoLibrary]"),
    ("files", vec![Files], "[FileSource::Files]"),
    (
      "photo library and files",
      vec![PhotoLibrary, Files],
      "[FileSource::PhotoLibrary, FileSource::Files]",
    ),
  ];
  #[cfg(target_os = "ios")]
  {
    use lurq::files::FileSource::Camera;
    sources.push(("camera", vec![Camera], "[FileSource::Camera]"));
    sources.push((
      "camera and photo library",
      vec![Camera, PhotoLibrary],
      "[FileSource::Camera, FileSource::PhotoLibrary]",
    ));
  }
  sources
}

/// The most files the playground's list holds.
const MOST: [(&str, Option<usize>); 3] = [("any", None), ("3", Some(3)), ("5", Some(5))];

/// How far a file goes at each tick of the playground's upload.
const STEP: f32 = 0.08;

fn file(name: &str, size: u64) -> UploadFile {
  UploadFile::new(DroppedFile::new(name, Some(size)))
}

/// The board's two files: one sent, one 62% of the way.
fn boards() -> Vec<UploadFile> {
  vec![
    file("theme.json", 4096).done(),
    file("screenshot.png", 812_000).uploading(0.62),
  ]
}

/// Moves the playground's upload on by a tick: a file waiting starts, one being sent goes on, and the fourth file of
/// the list, and every fourth after it, fails half way, for the page to show a failure. Whether anything is left to
/// send.
fn tick(files: &Signal<Vec<UploadFile>>) -> bool {
  let mut moving = false;
  files.update(|files| {
    for (index, file) in files.iter_mut().enumerate() {
      *file = match file.state {
        UploadState::Waiting => file.clone().uploading(0.0),
        UploadState::Uploading if index % 4 == 3 && file.progress >= 0.5 => {
          file.clone().failed("The server refused it. Try again.")
        }
        UploadState::Uploading if file.progress + STEP >= 1.0 => file.clone().done(),
        UploadState::Uploading => file.clone().uploading(file.progress + STEP),
        _ => continue,
      };
      moving |= matches!(file.state, UploadState::Waiting | UploadState::Uploading);
    }
  });
  moving
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

pub struct FileUploadPage {
  frame: Frame,
  playground: Playground,
  /// The playground's list, which the timer moves on.
  files: Signal<Vec<UploadFile>>,
  /// The timer that plays the upload: started by what is given or retried, stopped once nothing moves. The board's
  /// file at 62% stands still until it starts.
  upload: Interval,
  accept: Signal<usize>,
  most: Signal<usize>,
  /// Which of [`sources`] the dialog chooses from.
  source: Signal<usize>,
  zone: Signal<bool>,
  multiple: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
}

impl FileUploadPage {
  /// The playground: the upload as the panel's props make it, mounted, bound to the page's list.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (accept_name, accepted, accept_code) = ACCEPTS[self.accept.get().min(ACCEPTS.len() - 1)];
    let (most_name, most) = MOST[self.most.get().min(MOST.len() - 1)];
    let all_sources = sources();
    let (_, chosen_sources, sources_code) = all_sources[self.source.get().min(all_sources.len() - 1)].clone();
    let flags = [
      ("zone", self.zone.get()),
      ("multiple", self.multiple.get()),
      ("invalid", self.invalid.get()),
      ("disabled", self.disabled.get()),
    ];
    let files = self.files.get();
    let make = {
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move |files: FileUpload| {
        let mut upload = files
          .accept(accepted.iter().copied())
          .zone(set[0])
          .multiple(set[1])
          .invalid(set[2])
          .disabled(set[3]);
        if let Some(most) = most {
          upload = upload.max_files(most);
        }
        if !chosen_sources.is_empty() {
          upload = upload.sources(chosen_sources.iter().copied());
        }
        upload
      }
    };
    let (given, removed, retried, refused) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let (start, restart, again) = (self.upload.clone(), self.upload.clone(), self.files.clone());
    let upload = self
      .playground
      .styled(make(FileUpload::new(self.files.clone())))
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
        given(format!("on_files([{}])", names.join(", ")));
        start.start();
      })
      .on_remove(move |file| removed(format!("on_remove({})", file.file.name())))
      .on_retry(move |file| {
        retried(format!("on_retry({})", file.file.name()));
        let id = file.id.clone();
        again.update(|files| {
          if let Some(file) = files.iter_mut().find(|file| file.id == id) {
            *file = file.clone().waiting();
          }
        });
        restart.start();
      })
      .on_reject(move |rejected| {
        let names: Vec<String> = rejected
          .iter()
          .map(|file| format!("{}: {:?}", file.file.name(), file.reason))
          .collect();
        refused(format!("on_reject([{}])", names.join(", ")))
      });

    let mut code = vec![
      "// files: Signal<Vec<UploadFile>>".to_owned(),
      "FileUpload::new(files.clone())".to_owned(),
    ];
    if !accept_code.is_empty() {
      code.push(format!("    .accept({accept_code})"));
    }
    if let Some(most) = most {
      code.push(format!("    .max_files({most})"));
    }
    if !sources_code.is_empty() {
      code.push(format!("    .sources({sources_code})"));
    }
    if flags[0].1 {
      code.push("    .zone(true)".to_owned());
    }
    if !flags[1].1 {
      code.push("    .multiple(false)".to_owned());
    }
    for (name, set) in &flags[2..] {
      if *set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .on_files(|given| send(given))".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      format!(
        "├─ trigger  {}",
        if flags[0].1 { "drop zone" } else { "\"Choose files\"" }
      ),
    ];
    if files.is_empty() {
      tree.push("└─ list     empty".to_owned());
    } else {
      tree.push(format!("└─ list     {} files", files.len()));
      for (index, file) in files.iter().enumerate() {
        let state = match file.state {
          UploadState::Waiting => "waiting".to_owned(),
          UploadState::Uploading => format!("{}%", (file.progress * 100.0).round()),
          UploadState::Done => "done".to_owned(),
          UploadState::Failed => "failed".to_owned(),
        };
        let branch = if index + 1 == files.len() { "└─" } else { "├─" };
        tree.push(format!("   {branch} {}  {state}", file.file.name()));
      }
    }

    let accepts: Vec<(&str, usize)> = ACCEPTS
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let mosts: Vec<(&str, usize)> = MOST.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let source_names: Vec<(&str, usize)> = all_sources
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let (to_accept, to_most, to_source) = (self.accept.clone(), self.most.clone(), self.source.clone());
    let reset = {
      let (files, accept, most, upload, source) = (
        self.files.clone(),
        self.accept.clone(),
        self.most.clone(),
        self.upload.clone(),
        self.source.clone(),
      );
      let (zone, multiple, invalid, disabled) = (
        self.zone.clone(),
        self.multiple.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
      );
      move || {
        upload.stop();
        files.set(boards());
        accept.set(0);
        most.set(0);
        source.set(0);
        zone.set(false);
        multiple.set(true);
        invalid.set(false);
        disabled.set(false);
      }
    };
    let changed = usize::from(files != boards())
      + usize::from(!accept_code.is_empty())
      + usize::from(most.is_some())
      + usize::from(!sources_code.is_empty())
      + usize::from(flags[0].1)
      + usize::from(!flags[1].1)
      + flags[2..].iter().filter(|(_, set)| *set).count();
    let still = {
      let files = self.files.clone();
      move || make(FileUpload::at(files.get_untracked())).into()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "File Upload",
        make: Arc::new(still),
        shown: Shown::built(move |ctx| upload.mount(ctx)),
        fill: false,
        forceable: true,
        facts: format!("{accept_name} · at most {most_name}"),
        event: "on_files",
        code,
        tree,
        controls: vec![
          prop(
            "accept",
            "impl IntoIterator<Item = &str>",
            self
              .playground
              .select(0, &accepts, &self.accept.get(), move |index| to_accept.set(index)),
          ),
          prop(
            "max_files",
            "usize",
            self
              .playground
              .select(1, &mosts, &self.most.get(), move |index| to_most.set(index)),
          ),
          prop(
            "sources",
            "impl IntoIterator<Item = FileSource>",
            self
              .playground
              .select(2, &source_names, &self.source.get(), move |index| to_source.set(index)),
          ),
          divider(),
          flag("zone", &self.zone),
          flag("multiple", &self.multiple),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for FileUploadPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let files = ctx.signal(boards());
    // The timer stops itself once nothing is left to send: nothing the playground starts is left running.
    let slot: Arc<Mutex<Option<Interval>>> = Arc::default();
    let upload = ctx.create_interval(Duration::from_millis(150), {
      let (files, slot) = (files.clone(), slot.clone());
      move || {
        if !tick(&files)
          && let Some(upload) = slot.lock().unwrap().as_ref()
        {
          upload.stop();
        }
      }
    });
    *slot.lock().unwrap() = Some(upload.clone());
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      files,
      upload,
      accept: ctx.signal(0),
      most: ctx.signal(0),
      source: ctx.signal(0),
      zone: ctx.signal(false),
      multiple: ctx.signal(true),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items().child(item(FileUpload::at(boards()), "a file sent and one 62% of the way"));
    let states = items().child(item(
      FileUpload::at(vec![
        file("notes.txt", 2_150),
        file("cover.png", 1_340_000).uploading(0.3),
        file("theme.json", 4096).done(),
        file("archive.zip", 48_000_000).failed("The file is larger than 25 MB."),
        file("locked.pdf", 92_000).uploading(0.8).disabled(true),
      ]),
      "waiting · uploading · done · failed · a file that is not removed",
    ));
    let zone = items().child(item(
      FileUpload::at(vec![file("screenshot.png", 812_000).uploading(0.62)]).zone(true),
      "zone(true): a drop zone in place of the button",
    ));
    let long = items().child(item(
      FileUpload::at(vec![file(
        "the-photographs-of-the-whole-trip-to-the-mountains-in-the-summer.png",
        4_200_000,
      )]),
      "a long name is cut; its size keeps its room",
    ));

    let rows = [
      (
        "files",
        "Signal<Vec<UploadFile>>",
        "",
        "The list: what is given is added as waiting, a removed file taken out.",
      ),
      (
        "controlled",
        "Vec<UploadFile>, on_change",
        "",
        "The list the app holds, told the whole new list.",
      ),
      ("at", "Vec<UploadFile>", "", "A list shown and not operated."),
      (
        "accept / multiple / max_size / max_files",
        "",
        "",
        "What it takes, as a drop zone's; max_files counts the list.",
      ),
      ("zone", "bool", "false", "A drop zone in place of the button."),
      ("title / description", "&str", "", "The zone's words, where it has one."),
      ("choose_label", "&str", "\"Choose files\"", "What the button says."),
      (
        "remove_label / retry_label",
        "&str",
        "\"Remove\" / \"Retry\"",
        "What a file's controls are called, before its name.",
      ),
      ("size_text", "impl Fn(u64) -> String", "\"4 KB\"", "How a size is said."),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling, and its dialog's title.",
      ),
      (
        "invalid / disabled",
        "bool",
        "false",
        "Its button's edge destructive; or nothing is given or removed.",
      ),
      (
        "on_change",
        "impl Fn(&[UploadFile])",
        "",
        "Told the whole new list a person made.",
      ),
      (
        "on_files",
        "impl Fn(&[DroppedFile])",
        "",
        "Told the files given: those for the app to send.",
      ),
      (
        "on_reject",
        "impl Fn(&[RejectedFile])",
        "",
        "Told the files it refused, and why.",
      ),
      (
        "on_remove / on_retry",
        "impl Fn(&UploadFile)",
        "",
        "Told a file removed, or a failed one to send again.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Opens the dialog, shows a drag on its zone, and moves the focus on as files go.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "File Upload"],
        title: "File Upload",
        description: "The files a person gives, by the system's file dialog or a drop, in a list with how far each \
                      is sent. The app sends them and says how far: lurq sends nothing.",
        import: Some("use lurq::ui::{FileUpload, UploadFile};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of FileUpload is a control on the right. The upload on the stage is mounted: choose files or \
           drop them on it, and a timer of the page plays their upload, the fourth file of the list failing half way. The timer \
           stops once nothing is left to send.",
          playground,
        ),
        Section::new(
          "File Upload",
          "The board's upload: the outline button, and under it a file sent, with the success check, and one being \
           sent, with its bar and how far it is.",
          self.frame.demo(
            0,
            board,
            "FileUpload::new(files.clone())",
            &[
              "// files: Signal<Vec<UploadFile>>",
              "files.set(vec![",
              "    UploadFile::new(DroppedFile::new(\"theme.json\", Some(4096))).done(),",
              "    UploadFile::new(DroppedFile::new(\"screenshot.png\", Some(812_000))).uploading(0.62),",
              "]);",
              "FileUpload::new(files.clone()).on_files(|given| ..).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A file's states",
          "Waiting, it says its size and can be removed; being sent, how far, with its bar; sent, the check; failed, \
           its error under it and a retry before the remove control. The app sets each.",
          self.frame.demo(
            1,
            states,
            "UploadFile::new(file).failed(\"..\")",
            &[
              "UploadFile::new(file)",
              "UploadFile::new(file).uploading(0.3)",
              "UploadFile::new(file).done()",
              "UploadFile::new(file).failed(\"The file is larger than 25 MB.\")",
              "UploadFile::new(file).uploading(0.8).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "With a zone",
          "A drop zone in place of the button, for a form where files are the main thing given.",
          self.frame.demo(2, zone, ".zone(true)", &["FileUpload::new(files.clone()).zone(true).mount(ctx)"]),
        ),
        Section::new(
          "A long name",
          "A name longer than its line ends in an ellipsis; its size stays whole after it.",
          self.frame.demo(3, long, "UploadFile::new(file)", &["FileUpload::new(files.clone())"]),
        ),
        Section::new("Props", "What a file upload is given.", props_table(&rows, &["files"], self.frame.room())),
      ],
    )
  }
}
