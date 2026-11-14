//! The page of the key: its playground, a key, a shortcut, a Mac's glyphs, a sequence, the sizes, the plain text a
//! menu writes, keys in a line of text, and the inline code the board draws beside them.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{InlineCode, Kbd, KbdSize, KbdVariant, KeyPlatform},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, field, prop},
};

/// What the playground builds from the keys typed: a key, a shortcut, or a sequence of steps parted by spaces.
const FORMS: [(&str, &str); 3] = [
  ("new", "Kbd::new"),
  ("shortcut", "Kbd::shortcut"),
  ("sequence", "Kbd::sequence"),
];

/// The sizes, as the code names them, and how high each is.
const SIZES: [(KbdSize, &str, &str, u32); 3] = [
  (KbdSize::Sm, "sm", "KbdSize::Sm", 16),
  (KbdSize::Md, "md", "KbdSize::Md", 20),
  (KbdSize::Lg, "lg", "KbdSize::Lg", 24),
];

const VARIANTS: [(KbdVariant, &str, &str); 2] = [
  (KbdVariant::Default, "default", "KbdVariant::Default"),
  (KbdVariant::Plain, "plain", "KbdVariant::Plain"),
];

const PLATFORMS: [(KeyPlatform, &str, &str); 2] = [
  (KeyPlatform::Standard, "standard", "KeyPlatform::Standard"),
  (KeyPlatform::Mac, "mac", "KeyPlatform::Mac"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// A line of text of 15, as the board's document writes it.
fn words(content: &str) -> lurq::components::Text {
  text(content, 15.0, FontWeight::Normal)
}

pub struct KbdPage {
  frame: Frame,
  playground: Playground,
  keys: Signal<String>,
  form: Signal<usize>,
  variant: Signal<usize>,
  size: Signal<usize>,
  platform: Signal<usize>,
}

impl KbdPage {
  /// The playground: the keys as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let keys = self.keys.get();
    let form = self.form.get().min(FORMS.len() - 1);
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (platform, platform_name, platform_code) = PLATFORMS[self.platform.get().min(PLATFORMS.len() - 1)];
    let steps: Vec<String> = keys.split_whitespace().map(str::to_owned).collect();

    let plain = {
      let (keys, steps) = (keys.clone(), steps.clone());
      move || -> Element {
        match form {
          0 => Kbd::new(&keys),
          1 => Kbd::shortcut(&keys),
          _ => Kbd::sequence(&steps),
        }
        .variant(variant)
        .size(size)
        .platform(platform)
        .into()
      }
    };

    let mut code = vec![match form {
      0 => format!("Kbd::new({keys:?})"),
      1 => format!("Kbd::shortcut({keys:?})"),
      _ => format!("Kbd::sequence({steps:?})"),
    }];
    if variant != KbdVariant::Default {
      code.push(format!("    .variant({variant_code})"));
    }
    if size != KbdSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    if platform != KeyPlatform::Standard {
      code.push(format!("    .platform({platform_code})"));
    }

    let mut tree = vec![format!(
      "root      {}   {variant_name} · {size_name} · {height} px   text",
      self.playground.measured()
    )];
    if variant == KbdVariant::Plain {
      tree.push("└─ label   a step as one text".to_owned());
    } else {
      tree.push("├─ key     a box for each key".to_owned());
      tree.push("│  └─ label".to_owned());
      if form == 2 && steps.len() > 1 {
        tree.push("└─ separator   \"then\" between steps".to_owned());
      }
    }

    let forms: Vec<(&str, usize)> = FORMS.iter().enumerate().map(|(index, f)| (f.0, index)).collect();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.1, index)).collect();
    let platforms: Vec<(&str, usize)> = PLATFORMS.iter().enumerate().map(|(index, p)| (p.1, index)).collect();
    let (to_form, to_variant, to_size, to_platform) = (
      self.form.clone(),
      self.variant.clone(),
      self.size.clone(),
      self.platform.clone(),
    );
    let reset = (
      self.keys.clone(),
      self.form.clone(),
      self.variant.clone(),
      self.size.clone(),
      self.platform.clone(),
    );
    let make = plain.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Kbd",
        make: Arc::new(move || make()),
        shown: plain(),
        fill: false,
        forceable: false,
        facts: format!("{variant_name} · {size_name} · {platform_name}"),
        event: "",
        code,
        tree,
        controls: vec![
          prop("keys", "&str", field(&self.keys)),
          prop(
            "form",
            "new / shortcut / sequence",
            segmented_fill(&forms, &form, move |index| to_form.set(index)),
          ),
          divider(),
          prop(
            "variant",
            "KbdVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| to_variant.set(index)),
          ),
          prop(
            "size",
            "KbdSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          prop(
            "platform",
            "KeyPlatform",
            segmented_fill(&platforms, &self.platform.get(), move |index| to_platform.set(index)),
          ),
        ],
        changed: [
          keys != "Ctrl+K",
          form != 1,
          variant != KbdVariant::Default,
          size != KbdSize::Md,
          platform != KeyPlatform::Standard,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set("Ctrl+K".to_owned());
          reset.1.set(1);
          reset.2.set(0);
          reset.3.set(1);
          reset.4.set(0);
        }),
      },
    )
  }
}

impl Component for KbdPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      keys: ctx.signal("Ctrl+K".to_owned()),
      form: ctx.signal(1),
      variant: ctx.signal(0),
      size: ctx.signal(1),
      platform: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let single = items()
      .child(item(Kbd::new("Esc"), "Esc"))
      .child(item(Kbd::new("Tab"), "Tab"))
      .child(item(Kbd::new("Enter"), "Enter"))
      .child(item(Kbd::new("Page Up"), "Page Up"))
      .child(item(Kbd::new("ArrowUp"), "ArrowUp"))
      .child(item(Kbd::new("/"), "/"));
    let shortcuts = items()
      .child(item(Kbd::shortcut("Ctrl+K"), "Ctrl+K"))
      .child(item(Kbd::shortcut("Ctrl+Shift+P"), "Ctrl+Shift+P"))
      .child(item(Kbd::shortcut("Alt+ArrowLeft"), "Alt+ArrowLeft"))
      .child(item(Kbd::shortcut("Ctrl++"), "Ctrl++"));
    let mac = items()
      .child(item(Kbd::shortcut("Mod+K").platform(KeyPlatform::Mac), "Mod+K"))
      .child(item(
        Kbd::shortcut("Mod+Shift+P").platform(KeyPlatform::Mac),
        "Mod+Shift+P",
      ))
      .child(item(
        Kbd::shortcut("Ctrl+Alt+Delete").platform(KeyPlatform::Mac),
        "Ctrl+Alt+Delete",
      ))
      .child(item(
        Kbd::shortcut("Mod+Backspace").platform(KeyPlatform::Mac),
        "Mod+Backspace",
      ))
      .child(item(
        Kbd::shortcut("Mod+K").platform(KeyPlatform::current()),
        "the platform it runs on",
      ));
    let sequences = items()
      .child(item(Kbd::sequence(["G", "D"]), "G, D"))
      .child(item(Kbd::sequence(["Ctrl+K", "Ctrl+S"]), "Ctrl+K, Ctrl+S"));
    let sizes = SIZES.iter().fold(items(), |row, (size, name, _, height)| {
      row.child(item(Kbd::shortcut("Mod+N").size(*size), &format!("{name} · {height}")))
    });
    let plain = items()
      .child(item(Kbd::shortcut("Ctrl+D").variant(KbdVariant::Plain), "words"))
      .child(item(
        Kbd::shortcut("Mod+Shift+S")
          .variant(KbdVariant::Plain)
          .platform(KeyPlatform::Mac),
        "a Mac",
      ))
      .child(item(Kbd::new("F12").variant(KbdVariant::Plain), "one key"));
    let line = Row::new()
      .spacing(6.0)
      .align_items(Alignment::Center)
      .child(words("Press"))
      .child(Kbd::shortcut("Ctrl+K"))
      .child(words("to search the docs."));
    let code = Row::new()
      .spacing(4.0)
      .align_items(Alignment::Center)
      .child(words("Read the theme with"))
      .child(InlineCode::new("ctx.theme()"))
      .child(words("in a render."));
    let rows = [
      ("Kbd::new", "&str", "", "One key, written as it is given."),
      (
        "Kbd::shortcut",
        "&str",
        "",
        "The keys of a shortcut pressed together, parted by +: a box each.",
      ),
      (
        "Kbd::sequence",
        "impl IntoIterator<Item: AsRef<str>>",
        "",
        "Steps pressed one after another, each a key or a shortcut, with \"then\" between them.",
      ),
      (
        "Kbd::accelerator",
        "&Accelerator",
        "",
        "The keys of an accelerator of the app's menu: its modifiers, then its key.",
      ),
      (
        "platform",
        "KeyPlatform",
        "Standard",
        "Words, or a Mac's glyphs with the modifiers first in a Mac's order. KeyPlatform::current() is the one it runs on.",
      ),
      (
        "variant",
        "KbdVariant",
        "Default",
        "A box for each key, or each step as one text with no box, as a menu writes it.",
      ),
      ("size", "KbdSize", "Md", "16, 20 or 24 high, at 10, 11 or 12."),
      (
        "separator",
        "&str",
        "\"then\"",
        "What stands between the steps of a sequence.",
      ),
      ("InlineCode::new", "&str", "", "A short text of code in a run of text."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Kbd"],
        title: "Kbd",
        description: "A key of the keyboard, the keys of a shortcut, or keys pressed one after another, read and not \
                      operated: Ctrl K, ⌘ K, G then D.",
        import: Some("use lurq::ui::Kbd;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Kbd is a control on the right. Keys take no press and no focus: they are read. A sequence is \
           typed with its steps parted by spaces.",
          playground,
        ),
        Section::new(
          "A key",
          "A box 20 high on the muted fill, with an edge, its text 11 in the family of code. Keys are named as lurq \
           names them in a keyboard event: Escape, ArrowUp, Backspace; or as a person writes them.",
          self.frame.demo(
            0,
            single,
            "Kbd::new(\"Esc\")",
            &[
              "Kbd::new(\"Esc\")",
              "Kbd::new(\"Tab\")",
              "Kbd::new(\"Enter\")",
              "Kbd::new(\"Page Up\")",
              "Kbd::new(\"ArrowUp\")",
              "Kbd::new(\"/\")",
            ],
          ),
        ),
        Section::new(
          "A shortcut",
          "The keys pressed together, a box each and 4 apart, with nothing between them. A + after another is the \
           key of that name.",
          self.frame.demo(
            1,
            shortcuts,
            "Kbd::shortcut(\"Ctrl+K\")",
            &[
              "Kbd::shortcut(\"Ctrl+K\")",
              "Kbd::shortcut(\"Ctrl+Shift+P\")",
              "Kbd::shortcut(\"Alt+ArrowLeft\")",
              "Kbd::shortcut(\"Ctrl++\")",
            ],
          ),
        ),
        Section::new(
          "On a Mac",
          "A Mac's glyphs, the modifiers first and in the order its menus write them: ⌃ ⌥ ⇧ ⌘. Mod is ⌘ on a Mac and \
           Ctrl elsewhere.",
          self.frame.demo(
            2,
            mac,
            ".platform(KeyPlatform::Mac)",
            &[
              "Kbd::shortcut(\"Mod+K\").platform(KeyPlatform::Mac)",
              "Kbd::shortcut(\"Mod+Shift+P\").platform(KeyPlatform::Mac)",
              "Kbd::shortcut(\"Ctrl+Alt+Delete\").platform(KeyPlatform::Mac)",
              "Kbd::shortcut(\"Mod+Backspace\").platform(KeyPlatform::Mac)",
              "Kbd::shortcut(\"Mod+K\").platform(KeyPlatform::current())",
            ],
          ),
        ),
        Section::new(
          "A sequence",
          "Steps pressed one after another, each a key or a shortcut, with \"then\" between them.",
          self.frame.demo(
            3,
            sequences,
            "Kbd::sequence([\"G\", \"D\"])",
            &[
              "Kbd::sequence([\"G\", \"D\"])",
              "Kbd::sequence([\"Ctrl+K\", \"Ctrl+S\"])",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "16 high at 10, as in a tooltip; 20 at 11; 24 at 12 and 6 apart, as in a list of an app's shortcuts.",
          self.frame.demo(
            4,
            sizes,
            ".size(KbdSize::Lg)",
            &[
              "Kbd::shortcut(\"Mod+N\").size(KbdSize::Sm)",
              "Kbd::shortcut(\"Mod+N\")",
              "Kbd::shortcut(\"Mod+N\").size(KbdSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Plain",
          "Each step as one text with no box, in the muted color: what a menu writes at the end of an item.",
          self.frame.demo(
            5,
            plain,
            ".variant(KbdVariant::Plain)",
            &[
              "Kbd::shortcut(\"Ctrl+D\").variant(KbdVariant::Plain)",
              "Kbd::shortcut(\"Mod+Shift+S\")",
              "    .variant(KbdVariant::Plain)",
              "    .platform(KeyPlatform::Mac)",
              "Kbd::new(\"F12\").variant(KbdVariant::Plain)",
            ],
          ),
        ),
        Section::new(
          "In a line of text",
          "Keys stand in a row with the words around them, centred on the line.",
          self.frame.demo(
            6,
            line,
            "Kbd::shortcut(\"Ctrl+K\")",
            &[
              "Row::new()",
              "    .spacing(6.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Text::new(\"Press\"))",
              "    .child(Kbd::shortcut(\"Ctrl+K\"))",
              "    .child(Text::new(\"to search the docs.\"))",
            ],
          ),
        ),
        Section::new(
          "Inline code",
          "A short text of code in a run of text: 13 in the family of code, on the muted fill.",
          self.frame.demo(
            7,
            code,
            "InlineCode::new(\"ctx.theme()\")",
            &[
              "Row::new()",
              "    .spacing(4.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Text::new(\"Read the theme with\"))",
              "    .child(InlineCode::new(\"ctx.theme()\"))",
              "    .child(Text::new(\"in a render.\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What keys are given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
