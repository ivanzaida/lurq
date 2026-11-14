//! The page of the token input: its playground, a list of tags typed as tokens, tokens of each variant, its states,
//! its sizes, a template on rows whose variables are tokens, and a field that holds one.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Interaction, TagVariant, Token, TokenInput, TokenInputSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// How wide the board draws a token input.
const WIDTH: f32 = 260.0;

/// The sizes, as the board names an input's, how the code names them, and the height of a token in each.
const SIZES: [(TokenInputSize, &str, &str, f32); 3] = [
  (TokenInputSize::Sm, "sm", "TokenInputSize::Sm", 20.0),
  (TokenInputSize::Md, "md", "TokenInputSize::Md", 24.0),
  (TokenInputSize::Lg, "lg", "TokenInputSize::Lg", 28.0),
];

/// The variants a token can be, and how the code names them.
const VARIANTS: [(&str, TagVariant, &str); 5] = [
  ("neutral", TagVariant::Neutral, "TagVariant::Neutral"),
  ("info", TagVariant::Info, "TagVariant::Info"),
  ("success", TagVariant::Success, "TagVariant::Success"),
  ("warning", TagVariant::Warning, "TagVariant::Warning"),
  ("destructive", TagVariant::Destructive, "TagVariant::Destructive"),
];

/// What separates the items of the list: the selects of the playground.
const SEPARATORS: [(&str, char); 2] = [("space", ' '), ("comma", ',')];

/// A token input as wide as the board draws one, with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(WIDTH)
    .spacing(9.0)
    .child(shown)
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

/// The tags of a list typed with spaces: each word with a space after it is a token.
fn tags(value: Signal<String>) -> TokenInput {
  TokenInput::new(value)
    .tokens(|text| Token::separated(text, ' '))
    .placeholder("Add tag")
}

/// A template's variables, `{{name}}`: tokens without a cross.
fn variables(text: &str) -> Vec<Token> {
  let mut tokens = Vec::new();
  let mut from = 0;
  while let Some(open) = text[from..].find("{{").map(|at| from + at) {
    let Some(close) = text[open..].find("}}").map(|at| open + at + 2) else {
      break;
    };
    if !text[open..close].contains(char::is_whitespace) {
      tokens.push(Token::new(open..close).variant(TagVariant::Info).removable(false));
    }
    from = close;
  }
  tokens
}

pub struct TokenInputPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  size: Signal<usize>,
  variant: Signal<usize>,
  separator: Signal<usize>,
  removable: Signal<bool>,
  rows: Signal<bool>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  tags: Signal<String>,
  template: Signal<String>,
  recipients: Signal<String>,
}

impl TokenInputPage {
  /// The playground: the token input as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let name = self.label.get();
    let (size, size_name, size_code, token_height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (variant_name, variant, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (separator_name, separator) = SEPARATORS[self.separator.get().min(SEPARATORS.len() - 1)];
    let (removable, rows) = (self.removable.get(), self.rows.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let value = self.value.clone();
    let plain = {
      let (placeholder, value, name) = (placeholder.clone(), value.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut input = TokenInput::new(value.clone())
          .tokens(move |text| {
            Token::separated(text, separator)
              .into_iter()
              .map(|token| token.variant(variant).removable(removable))
              .collect()
          })
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .read_only(set[3])
          .disabled(set[4])
          .full_width(set[5]);
        if rows {
          input = input.rows(2, 6);
        }
        if !name.is_empty() {
          input = input.label(&name);
        }
        input
      }
    };
    let record = self.playground.recorder();
    let input = plain()
      .force(self.playground.forced())
      .on_input(move |event| record(format!("on_input({:?})", event.new_value)));

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "TokenInput::new(value.clone())".to_owned(),
    ];
    let token = match (variant != TagVariant::Neutral, removable) {
      (false, true) => "token".to_owned(),
      (true, true) => format!("token.variant({variant_code})"),
      (false, false) => "token.removable(false)".to_owned(),
      (true, false) => format!("token.variant({variant_code}).removable(false)"),
    };
    if token == "token" {
      code.push(format!("    .tokens(|text| Token::separated(text, {separator:?}))"));
    } else {
      code.push(format!(
        "    .tokens(|text| Token::separated(text, {separator:?}).into_iter().map(|token| {token}).collect())"
      ));
    }
    code.push(format!("    .placeholder({placeholder:?})"));
    if !name.is_empty() {
      code.push(format!("    .label({name:?})"));
    }
    if size != TokenInputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    if rows {
      code.push("    .rows(2, 6)".to_owned());
    }
    for (flag, set) in flags {
      if set {
        code.push(format!("    .{flag}(true)"));
      }
    }

    let held = value.get();
    let found = Token::separated(&held, separator).len();
    let tree = vec![
      format!("root      {}", self.playground.measured()),
      format!("├─ text    {} characters", held.chars().count()),
      format!("└─ tokens  {found}, {variant_name}, {token_height} high"),
    ];

    let (to_value, to_placeholder, to_label) = (self.value.clone(), self.placeholder.clone(), self.label.clone());
    let (to_size, to_variant, to_separator) = (self.size.clone(), self.variant.clone(), self.separator.clone());
    let (to_removable, to_rows) = (self.removable.clone(), self.rows.clone());
    let flag_signals = [
      self.invalid.clone(),
      self.warning.clone(),
      self.valid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    let (choose_size, choose_variant, choose_separator) =
      (self.size.clone(), self.variant.clone(), self.separator.clone());
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, variant)| (variant.0, index))
      .collect();
    let separators: Vec<(&str, usize)> = SEPARATORS
      .iter()
      .enumerate()
      .map(|(index, separator)| (separator.0, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Token Input",
        make: Arc::new(move || Column::new().width(360.0).child(plain()).into()),
        shown: Column::new().width(FILL).child(input).into(),
        fill: true,
        forceable: true,
        facts: format!("{size_name} · {found} tokens · {separator_name}"),
        event: "on_input",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("label", "&str", field(&self.label)),
          prop(
            "size",
            "TokenInputSize",
            segmented_fill(&sizes, &self.size.get(), move |index| choose_size.set(index)),
          ),
          prop(
            "token: variant",
            "TagVariant",
            self.playground.select(0, &variants, &self.variant.get(), move |index| {
              choose_variant.set(index)
            }),
          ),
          prop(
            "separator",
            "char",
            segmented_fill(&separators, &self.separator.get(), move |index| {
              choose_separator.set(index)
            }),
          ),
          flag("token: removable", &self.removable),
          flag("rows(2, 6)", &self.rows),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [
          held != "rust wgpu ",
          placeholder != "Add tag",
          !name.is_empty(),
          size != TokenInputSize::Md,
          variant != TagVariant::Neutral,
          separator != ' ',
          !removable,
          rows,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set("rust wgpu ".to_owned());
          to_placeholder.set("Add tag".to_owned());
          to_label.set(String::new());
          to_size.set(1);
          to_variant.set(0);
          to_separator.set(0);
          to_removable.set(true);
          to_rows.set(false);
          for flag in &flag_signals {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TokenInputPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      value: ctx.signal("rust wgpu ".to_owned()),
      placeholder: ctx.signal("Add tag".to_owned()),
      label: ctx.signal(String::new()),
      size: ctx.signal(1),
      variant: ctx.signal(0),
      separator: ctx.signal(0),
      removable: ctx.signal(true),
      rows: ctx.signal(false),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      tags: ctx.signal("rust wgpu ".to_owned()),
      template: ctx.signal("Hello {{name}}, your order {{order}} shipped on {{date}}.".to_owned()),
      recipients: ctx.signal("ivan@lurq.dev, ivy@lurq.dev, ".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let held = self.tags.get();
    let live = items().child(item(tags(self.tags.clone()), &format!("the signal holds {:?}", held)));
    let mut variants = items();
    for (name, variant, _) in VARIANTS {
      variants = variants.child(item(
        TokenInput::new(Signal::new(format!("{name} "))).tokens(move |text| {
          Token::separated(text, ' ')
            .into_iter()
            .map(|token| token.variant(variant))
            .collect()
        }),
        name,
      ));
    }
    variants = variants.child(item(
      TokenInput::new(Signal::new("pinned ".to_owned())).tokens(|text| {
        Token::separated(text, ' ')
          .into_iter()
          .map(|token| token.removable(false))
          .collect()
      }),
      "removable(false)",
    ));
    let filled = || Signal::new("rust wgpu ".to_owned());
    let states = items()
      .child(item(tags(Signal::new(String::new())), "default"))
      .child(item(tags(filled()), "filled"))
      .child(item(tags(filled()).force(Some(Interaction::Hovered)), "hover"))
      .child(item(tags(filled()).force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(tags(filled()).invalid(true), "invalid"))
      .child(item(tags(filled()).warning(true), "warning"))
      .child(item(tags(filled()).valid(true), "valid"))
      .child(item(tags(filled()).read_only(true), "read only: no cross"))
      .child(item(tags(filled()).disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _, token_height) in SIZES {
      sizes = sizes.child(item(
        tags(filled()).size(size),
        &format!("{name} · tokens {token_height}"),
      ));
    }
    let template = items().child(
      Column::new().width(420.0).child(
        TokenInput::new(self.template.clone())
          .tokens(variables)
          .rows(2, 6)
          .full_width(true),
      ),
    );
    let recipients = self.recipients.get();
    let count = Token::separated(&recipients, ',').len();
    let in_field = {
      let field = Field::new(
        "Recipients",
        TokenInput::new(self.recipients.clone())
          .tokens(|text| Token::separated(text, ','))
          .placeholder("name@example.com,")
          .full_width(true),
      )
      .required(true)
      .full_width(true);
      if count == 0 {
        field.error("Add at least one recipient: type an address and a comma.")
      } else {
        field.description(&format!("{count} recipient{}.", if count == 1 { "" } else { "s" }))
      }
    };
    let in_field = items().child(Column::new().width(360.0).child(in_field));
    let props = [
      (
        "value",
        "Signal<String>",
        "",
        "The text, tokens and all. What is typed is written into it, and what the app writes there shows.",
      ),
      (
        "tokens",
        "impl Fn(&str) -> Vec<Token>",
        "",
        "How the app finds its tokens in the text, called in every render. Token::separated finds a list's items.",
      ),
      (
        "Token::variant",
        "TagVariant",
        "Neutral",
        "What a token says by its color, as a tag does: the secondary fill, or the muted fill of a color.",
      ),
      (
        "Token::removable",
        "bool",
        "true",
        "The cross at the end of a token, which a click takes it away by, as an edit that undo brings back.",
      ),
      (
        "Token::atomic",
        "bool",
        "true",
        "The caret goes past it whole, a click selects it, Backspace selects it and then deletes it.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What it shows while it holds nothing, in the muted foreground.",
      ),
      (
        "rows",
        "(usize, usize)",
        "one line",
        "Text on several rows, between the two counts, where tokens and words wrap together.",
      ),
      (
        "size",
        "TokenInputSize",
        "Md",
        "An input's 32, 36 and 40, with tokens of 20, 24 and 28.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What it holds is wrong: the destructive color.",
      ),
      ("warning", "bool", "false", "Allowed with a caveat: the warning color."),
      ("valid", "bool", "false", "Checked and right: the success color."),
      (
        "read_only",
        "bool",
        "false",
        "Focused, selected and copied, not changed: its tokens have no cross.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it takes no input and Tab passes it by.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 260.",
      ),
      (
        "on_input",
        "impl Fn(&TextInputEvent)",
        "",
        "Called before every edit, a token taken away by its cross too; it may refuse it.",
      ),
      (
        "controller",
        "TextInputController",
        "",
        "Kept by the app: reads the selection, and puts a token chosen from a picker at it.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. A field gives it its own label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Token Input"],
        title: "Token Input",
        description: "A box of text in which what the app recognizes stands as a token: tags, recipients, the \
                      variables of a template. The text is the value; the tokens are found in it in every render.",
        import: Some("use lurq::ui::{Token, TokenInput};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TokenInput is a control on the right. The input on the stage is live: type a word and a \
           space to make a token, click a cross to take one away, Backspace after one to select it.",
          playground,
        ),
        Section::new(
          "Token input",
          "The page holds the text in a signal, and every word with a space after it is a token. The word being \
           typed is text until its space.",
          self.frame.demo(
            0,
            live,
            "TokenInput::new(tags.clone()).tokens(..)",
            &[
              "// tags: Signal<String>",
              "TokenInput::new(tags.clone())",
              "    .tokens(|text| Token::separated(text, ' '))",
              "    .placeholder(\"Add tag\")",
            ],
          ),
        ),
        Section::new(
          "Tokens",
          "A token is in the colors of a tag of its variant. One that is not removable has no cross: Backspace still \
           takes it away.",
          self.frame.demo(
            1,
            variants,
            "Token::new(range).variant(TagVariant::Info)",
            &[
              "TokenInput::new(value.clone()).tokens(|text| {",
              "    Token::separated(text, ' ')",
              "        .into_iter()",
              "        .map(|token| token.variant(TagVariant::Success))",
              "        .collect()",
              "})",
              "",
              "Token::new(0..6).removable(false)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is an input's: its edge says its state. A read-only one has no crosses; a disabled one is dimmed \
           and takes nothing.",
          self.frame.demo(
            2,
            states,
            ".invalid(true)",
            &[
              "tags(value.clone())",
              "tags(filled.clone()).force(Some(Interaction::Hovered))",
              "tags(filled.clone()).force(Some(Interaction::FocusVisible))",
              "tags(filled.clone()).invalid(true)",
              "tags(filled.clone()).warning(true)",
              "tags(filled.clone()).valid(true)",
              "tags(filled.clone()).read_only(true)",
              "tags(filled.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "An input's heights, 32, 36 and 40, with tokens of 20, 24 and 28 and their text of 12, 13 and 14.",
          self.frame.demo(
            3,
            sizes,
            ".size(TokenInputSize::Sm)",
            &[
              "tags(value.clone()).size(TokenInputSize::Sm)",
              "tags(value.clone())",
              "tags(value.clone()).size(TokenInputSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "On rows",
          "With rows the text wraps, tokens with it, and each row has room for a token. Here the variables of a \
           template are tokens with no cross: type {{ and a name and }} to make another.",
          self.frame.demo(
            4,
            template,
            ".rows(2, 6)",
            &[
              "// template: Signal<String>",
              "TokenInput::new(template.clone())",
              "    .tokens(variables) // {{name}}: Token::new(range).variant(TagVariant::Info).removable(false)",
              "    .rows(2, 6)",
              "    .full_width(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field calls the input by its label and gives it its judgment: here the error makes it invalid until it \
           holds a recipient.",
          self.frame.demo(
            5,
            in_field,
            "Field::new(\"Recipients\", input).error(..)",
            &[
              "// recipients: Signal<String>",
              "let count = Token::separated(&recipients.get(), ',').len();",
              "let field = Field::new(",
              "    \"Recipients\",",
              "    TokenInput::new(recipients.clone())",
              "        .tokens(|text| Token::separated(text, ','))",
              "        .placeholder(\"name@example.com,\")",
              "        .full_width(true),",
              ")",
              ".required(true)",
              ".full_width(true);",
              "if count == 0 {",
              "    field.error(\"Add at least one recipient: type an address and a comma.\")",
              "} else {",
              "    field.description(&format!(\"{count} recipients.\"))",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a token input is given.",
          props_table(&props, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
