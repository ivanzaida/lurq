//! The page of the token input: its playground over a list, a list of tags, tokens of each variant, its states, its
//! sizes, a template on rows whose variables are tokens found in its text, and a field that holds a list.

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

/// What makes an item of what is typed, besides Enter: the choices of the playground.
const SEPARATORS: [(&str, char); 2] = [("space", ' '), ("comma", ',')];

/// What the playground's list holds to begin with.
fn first_tags() -> Vec<String> {
  vec!["rust".to_owned(), "wgpu".to_owned()]
}

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
  items: Signal<Vec<String>>,
  draft: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  size: Signal<usize>,
  variant: Signal<usize>,
  separator: Signal<usize>,
  removable: Signal<bool>,
  unique: Signal<bool>,
  commit_on_blur: Signal<bool>,
  rows: Signal<bool>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  tags: Signal<Vec<String>>,
  template: Signal<String>,
  recipients: Signal<Vec<String>>,
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
    let (unique, commit_on_blur) = (self.unique.get(), self.commit_on_blur.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let (items, draft) = (self.items.clone(), self.draft.clone());
    let plain = {
      let (placeholder, items, draft, name) = (placeholder.clone(), items.clone(), draft.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut input = TokenInput::list(items.clone())
          .draft(draft.clone())
          .separators([separator])
          .unique(unique)
          .commit_on_blur(commit_on_blur)
          .token(move |_, token| token.variant(variant).removable(removable))
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
    let (changed, typed) = (self.playground.recorder(), self.playground.recorder());
    let input = plain()
      .force(self.playground.forced())
      .on_change(move |items| changed(format!("on_change({items:?})")))
      .on_draft(move |draft| typed(format!("on_draft({draft:?})")));

    let mut code = vec![
      "// tags: Signal<Vec<String>>".to_owned(),
      "TokenInput::list(tags.clone())".to_owned(),
    ];
    if separator != ',' {
      code.push(format!("    .separators([{separator:?}])"));
    }
    if unique {
      code.push("    .unique(true)".to_owned());
    }
    if commit_on_blur {
      code.push("    .commit_on_blur(true)".to_owned());
    }
    match (variant != TagVariant::Neutral, removable) {
      (false, true) => {}
      (true, true) => code.push(format!("    .token(|_, token| token.variant({variant_code}))")),
      (false, false) => code.push("    .token(|_, token| token.removable(false))".to_owned()),
      (true, false) => code.push(format!(
        "    .token(|_, token| token.variant({variant_code}).removable(false))"
      )),
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

    let (held, typed_now) = (items.get(), draft.get());
    let found = held.len();
    let tree = vec![
      format!("root      {}", self.playground.measured()),
      format!("├─ tokens  {found}, {variant_name}, {token_height} high"),
      format!("└─ draft   {typed_now:?}"),
    ];

    let (to_items, to_draft) = (self.items.clone(), self.draft.clone());
    let (to_placeholder, to_label) = (self.placeholder.clone(), self.label.clone());
    let (to_unique, to_commit) = (self.unique.clone(), self.commit_on_blur.clone());
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
        facts: format!("{size_name} · {found} in the list · draft {typed_now:?} · Enter or {separator_name}"),
        event: "on_change",
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
          flag("unique", &self.unique),
          flag("commit_on_blur", &self.commit_on_blur),
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
          held != first_tags() || !typed_now.is_empty(),
          placeholder != "Add tag",
          !name.is_empty(),
          size != TokenInputSize::Md,
          variant != TagVariant::Neutral,
          separator != ' ',
          !removable,
          unique,
          commit_on_blur,
          rows,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_items.set(first_tags());
          to_draft.set(String::new());
          to_unique.set(false);
          to_commit.set(false);
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
      items: ctx.signal(first_tags()),
      draft: ctx.signal(String::new()),
      placeholder: ctx.signal("Add tag".to_owned()),
      label: ctx.signal(String::new()),
      size: ctx.signal(1),
      variant: ctx.signal(0),
      separator: ctx.signal(0),
      removable: ctx.signal(true),
      unique: ctx.signal(false),
      commit_on_blur: ctx.signal(false),
      rows: ctx.signal(false),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      tags: ctx.signal(first_tags()),
      template: ctx.signal("Hello {{name}}, your order {{order}} shipped on {{date}}.".to_owned()),
      recipients: ctx.signal(vec!["ivan@lurq.dev".to_owned(), "ivy@lurq.dev".to_owned()]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let held = self.tags.get();
    let live = items().child(item(
      TokenInput::list(self.tags.clone())
        .separators([',', ' '])
        .unique(true)
        .placeholder("Add tag"),
      &format!("the list holds {held:?}"),
    ));
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
    let count = self.recipients.get().len();
    let in_field = {
      let field = Field::new(
        "Recipients",
        TokenInput::list(self.recipients.clone())
          .separators([',', ';', ' '])
          .unique(true)
          .commit_on_blur(true)
          .token(|address, token| {
            if address.contains('@') {
              token
            } else {
              token.variant(TagVariant::Destructive)
            }
          })
          .placeholder("name@example.com")
          .full_width(true),
      )
      .required(true)
      .full_width(true);
      if count == 0 {
        field.error("Add at least one recipient: type an address and Enter.")
      } else {
        field.description(&format!("{count} recipient{}.", if count == 1 { "" } else { "s" }))
      }
    };
    let in_field = items().child(Column::new().width(360.0).child(in_field));
    let props = [
      (
        "TokenInput::list",
        "Signal<Vec<String>>",
        "",
        "A list: each item is a token. What is typed is the draft, no part of the list until it is made an item.",
      ),
      (
        "separators",
        "impl IntoIterator<Item = char>",
        "[',']",
        "The characters that make an item of the draft, as Enter does. A pasted list is split at them.",
      ),
      (
        "draft",
        "Signal<String>",
        "",
        "What is typed and is no item yet: read it for suggestions, set it to change what the input shows.",
      ),
      (
        "unique",
        "bool",
        "false",
        "An item the list already has is not added again.",
      ),
      (
        "commit_on_blur",
        "bool",
        "false",
        "The draft becomes an item when the focus leaves, in place of staying typed.",
      ),
      (
        "token",
        "impl Fn(&str, Token) -> Token",
        "",
        "Dresses the token of each item of a list: a variant by what the item is, or no cross.",
      ),
      (
        "on_change",
        "impl Fn(&[String])",
        "",
        "Called with the list when a person changed it: an item made, pasted or taken away.",
      ),
      (
        "on_draft",
        "impl Fn(&str)",
        "",
        "Called with the draft when a person changed it.",
      ),
      (
        "TokenInput::new",
        "Signal<String>",
        "",
        "Text with tokens in it, where the text around them is the value too: a template with variables.",
      ),
      (
        "tokens",
        "impl Fn(&str) -> Vec<Token>",
        "",
        "How the app finds its tokens in a text, called in every render.",
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
        description: "A box that holds a list as tokens, with what is being typed after them: tags, recipients. \
                      Or a text in which the app finds its tokens: the variables of a template.",
        import: Some("use lurq::ui::{Token, TokenInput};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TokenInput is a control on the right. The input on the stage is live over a list: type a \
           word and Enter or the separator to add it, click a cross to take one away, Backspace after one to select \
           it. What is typed and is no token yet is the draft, which the list does not hold.",
          playground,
        ),
        Section::new(
          "Token input",
          "The page holds the list in a signal of strings. Enter, a comma or a space makes a token of the word being \
           typed; until then the list does not have it. A tag it already has is not added again.",
          self.frame.demo(
            0,
            live,
            "TokenInput::list(tags.clone())",
            &[
              "// tags: Signal<Vec<String>>",
              "TokenInput::list(tags.clone())",
              "    .separators([',', ' '])",
              "    .unique(true)",
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
          "With rows the text wraps, tokens with it, and each row has room for a token. Here the input holds a \
           text, not a list: the variables of a template are tokens found in it, with no cross. Type {{ and a name \
           and }} to make another.",
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
          "A field calls the input by its label and gives it its judgment: here the error makes it invalid until the \
           list holds a recipient. An address with no @ is a destructive token, and the draft becomes an item when \
           the focus leaves.",
          self.frame.demo(
            5,
            in_field,
            "Field::new(\"Recipients\", input).error(..)",
            &[
              "// recipients: Signal<Vec<String>>",
              "let count = recipients.get().len();",
              "let field = Field::new(",
              "    \"Recipients\",",
              "    TokenInput::list(recipients.clone())",
              "        .separators([',', ';', ' '])",
              "        .unique(true)",
              "        .commit_on_blur(true)",
              "        .token(|address, token| {",
              "            if address.contains('@') { token } else { token.variant(TagVariant::Destructive) }",
              "        })",
              "        .placeholder(\"name@example.com\")",
              "        .full_width(true),",
              ")",
              ".required(true)",
              ".full_width(true);",
              "if count == 0 {",
              "    field.error(\"Add at least one recipient: type an address and Enter.\")",
              "} else {",
              "    field.description(&format!(\"{count} recipients.\"))",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a token input is given.",
          props_table(&props, &["TokenInput::list"], self.frame.room()),
        ),
      ],
    )
  }
}
