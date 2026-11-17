//! How a component that shows code draws its tokens: each name in the style its recipe gives a part of that name,
//! or the theme's code colors give it, dropping the tail of a name neither has.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use super::recipe::{Name, ResolvedRecipe};
use crate::{
  app::theme::{CodeScheme, CodeStyle, Theme},
  code::{CodeHighlighter, CodeToken, TokenName},
  components::{InputToken, InputTokenStyle, InputTokens},
  layout::{
    quad::RichTextSpan,
    text_style::{FontStyle, TextStyle},
  },
  node::{TextColor, color::Color},
};

/// The tokens of `code`: by `highlighter` where it is given, by the highlighter registered for `language`
/// otherwise, none where neither is. Read in a render, the language's renders that again when its highlighter
/// changes.
pub(crate) fn code_tokens(
  code: &str,
  language: Option<&str>,
  highlighter: Option<&CodeHighlighter>,
) -> Arc<[CodeToken]> {
  match (highlighter, language) {
    (Some(highlighter), _) if !code.is_empty() => highlighter.tokens(code),
    (None, Some(language)) => crate::code::tokens(code, language),
    _ => Arc::from([]),
  }
}

/// The style of each name of a token, as a component draws it.
pub(crate) struct TokenStyles {
  recipe: ResolvedRecipe,
  /// The component's own parts, which are no tokens' even where a token has their name.
  own: &'static [&'static str],
  scheme: Arc<CodeScheme>,
  base: TextStyle,
  styles: RefCell<HashMap<TokenName, Option<TextStyle>>>,
}

impl TokenStyles {
  /// The styles of tokens in `recipe` and `scheme` (the theme's code colors where it is none), over `base`, the
  /// style of the code where no token is.
  pub(crate) fn new(
    recipe: &ResolvedRecipe,
    own: &'static [&'static str],
    base: TextStyle,
    scheme: Option<Arc<CodeScheme>>,
  ) -> Self {
    Self {
      scheme: scheme.unwrap_or_else(|| recipe.theme().code_scheme()),
      recipe: recipe.clone(),
      own,
      base,
      styles: RefCell::default(),
    }
  }

  fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the recipe and the scheme say of `name`, each of color, weight and leaning from its most particular name
  /// that says it, the recipe's before the scheme's at each.
  pub(crate) fn code_style(&self, name: &str) -> CodeStyle {
    let mut style = CodeStyle::new();
    for name in crate::code::lookup_names(name) {
      let part = Name::new(name);
      if !self.own.contains(&name) && self.recipe.has(part) {
        let rest = self.recipe.part(part).rest();
        style = style.or(&CodeStyle {
          color: rest.foreground,
          weight: rest.text.weight,
          italic: rest.text.style.map(|style| style == FontStyle::Italic),
        });
      }
      if let Some(own) = self.scheme.get(part) {
        style = style.or(own);
      }
      if style.is_whole() {
        break;
      }
    }
    style
  }

  /// The style a token named `name` is drawn in; `None` where it is drawn as the code is.
  pub(crate) fn style(&self, name: TokenName) -> Option<TextStyle> {
    if let Some(style) = self.styles.borrow().get(&name) {
      return style.clone();
    }
    let own = self.code_style(name.as_str());
    let style = (!own.is_empty()).then(|| {
      let mut style = self.base.clone();
      if let Some(color) = &own.color {
        style.color = self.color(color);
      }
      if let Some(weight) = own.weight {
        style.weight = weight;
      }
      if let Some(italic) = own.italic {
        style.style = if italic { FontStyle::Italic } else { FontStyle::Normal };
      }
      style
    });
    self.styles.borrow_mut().insert(name, style.clone());
    style
  }

  fn color(&self, color: &TextColor) -> Color {
    match color {
      TextColor::Color(color) => *color,
      TextColor::Palette(role) => self.theme().palette_color(role.clone()),
    }
  }

  /// `text` (the bytes `range` of the code) in spans of the styles of `tokens`, which are in bytes of the code and in
  /// order.
  pub(crate) fn spans(&self, code: &str, range: std::ops::Range<usize>, tokens: &[CodeToken]) -> Vec<RichTextSpan> {
    let mut spans: Vec<RichTextSpan> = Vec::new();
    let mut push = |text: &str, style: Option<TextStyle>| {
      if text.is_empty() {
        return;
      }
      let style = style.unwrap_or_else(|| self.base.clone());
      match spans.last_mut() {
        Some(last) if last.style == style => last.text.push_str(text),
        _ => spans.push(RichTextSpan {
          text: text.to_owned(),
          style,
        }),
      }
    };
    let first = tokens.partition_point(|token| token.range.end <= range.start);
    let mut at = range.start;
    for token in &tokens[first..] {
      if token.range.start >= range.end {
        break;
      }
      let start = token.range.start.max(range.start);
      let end = token.range.end.min(range.end);
      push(&code[at..start], None);
      push(&code[start..end], self.style(token.kind));
      at = end;
    }
    push(&code[at..range.end], None);
    if spans.is_empty() {
      spans.push(RichTextSpan {
        text: String::new(),
        style: self.base.clone(),
      });
    }
    spans
  }

  /// The tokens of `code` as an input draws them: a run of each on one line, in its color, and its weight and its
  /// leaning where they are not the code's.
  pub(crate) fn input_tokens(&self, code: &str, tokens: &[CodeToken]) -> InputTokens {
    let clear = Color::new(0, 0, 0, 0);
    let mut runs = Vec::new();
    let mut styles: HashMap<TokenName, Option<InputTokenStyle>> = HashMap::new();
    for token in tokens {
      let style = styles
        .entry(token.kind)
        .or_insert_with(|| {
          self.style(token.kind).map(|style| {
            let color = style.color;
            let input = InputTokenStyle::new(color, clear);
            if style.weight != self.base.weight || style.style != self.base.style {
              input.text_style(style)
            } else {
              input
            }
          })
        })
        .clone();
      let Some(style) = style else {
        continue;
      };
      // A token of the input stands on one line: one of many lines is one run a line.
      let mut at = token.range.start;
      for piece in code[token.range.clone()].split_inclusive('\n') {
        let text = piece.trim_end_matches(['\n', '\r']);
        if !text.is_empty() {
          runs.push(InputToken::new(at..at + text.len(), style.clone()));
        }
        at += piece.len();
      }
    }
    InputTokens::new(code, runs)
      .unwrap_or_else(|_| InputTokens::new(code, Vec::new()).expect("no tokens are always valid"))
  }
}

impl crate::components::TextInput {
  /// Colors its value as code in `language`, as a code block colors it: by the highlighter registered for the
  /// language, each token in the color the theme's code colors give its name, again after every edit. The colors
  /// are the theme's as the input is built; the weight and the leaning a name may have are a code block's alone.
  ///
  /// ```
  /// use lurq::{components::TextInput, core::Signal};
  ///
  /// let query = TextInput::new(Signal::new("select * from notes".to_owned()))
  ///   .multiline()
  ///   .code_language("sql");
  /// ```
  pub fn code_language(self, language: &str) -> Self {
    let language: Arc<str> = Arc::from(language);
    // The language's highlighter as it is now: a render after it is replaced builds the input again.
    let _ = crate::code::highlighter(&language);
    self.code_tokens(move |value| code_tokens(value, Some(&language), None))
  }

  /// Colors its value as code by `highlighter`, as [`code_language`](Self::code_language) does by a language's.
  pub fn code_highlighter(self, highlighter: impl Into<CodeHighlighter>) -> Self {
    let highlighter = highlighter.into();
    self.code_tokens(move |value| highlighter.tokens(value))
  }

  fn code_tokens(self, tokens: impl Fn(&str) -> Arc<[CodeToken]> + Send + Sync + 'static) -> Self {
    let theme = super::theme::current_theme();
    let scheme = theme.code_scheme();
    let clear = Color::new(0, 0, 0, 0);
    self.tokens_with(move |value| {
      let mut styles: HashMap<TokenName, Option<InputTokenStyle>> = HashMap::new();
      let mut runs = Vec::new();
      for token in tokens(value).iter() {
        let style = styles
          .entry(token.kind)
          .or_insert_with(|| {
            scheme.resolve(token.kind.as_str()).color.map(|color| {
              let color = match color {
                TextColor::Color(color) => color,
                TextColor::Palette(role) => theme.palette_color(role),
              };
              InputTokenStyle::new(color, clear)
            })
          })
          .clone();
        let Some(style) = style else {
          continue;
        };
        let mut at = token.range.start;
        for piece in value[token.range.clone()].split_inclusive('\n') {
          let text = piece.trim_end_matches(['\n', '\r']);
          if !text.is_empty() {
            runs.push(InputToken::new(at..at + text.len(), style.clone()));
          }
          at += piece.len();
        }
      }
      InputTokens::new(value, runs)
        .unwrap_or_else(|_| InputTokens::new(value, Vec::new()).expect("no tokens are always valid"))
    })
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    app::theme::PaletteColor,
    code::{CodeToken, token},
    components::TextInput,
    core::Signal,
    node::{Element, TextColor, node_kind::NodeKind},
  };

  /// The colors of the tokens an input draws for the value it holds now, by the text they cover.
  fn drawn(input: &Element, value: &Signal<String>) -> Vec<(String, TextColor)> {
    let NodeKind::TextInput { state, .. } = input.node.node_kind() else {
      panic!("a text input");
    };
    let value = value.get_untracked();
    state
      .tokens()
      .map(|tokens| {
        tokens
          .tokens
          .iter()
          .map(|token| (value[token.range.clone()].to_owned(), token.style.foreground.clone()))
          .collect()
      })
      .unwrap_or_default()
  }

  #[test]
  fn a_code_field_colors_its_value_after_every_edit() {
    let info = TextColor::Color(super::super::theme::current_theme().palette_color(PaletteColor::Info));
    let value = Signal::new("let x = 1;".to_owned());
    let input: Element = TextInput::new(value.clone()).multiline().code_language("rust").into();
    let first = drawn(&input, &value);
    assert!(first.contains(&("let".to_owned(), info.clone())), "{first:?}");
    // An edit, or a value the app sets: the tokens are of the new value.
    value.set("fn main() {}".to_owned());
    let next = drawn(&input, &value);
    assert!(next.contains(&("fn".to_owned(), info.clone())), "{next:?}");
    assert!(next.iter().all(|(text, _)| text != "let"));
  }

  #[test]
  fn a_code_field_takes_a_highlighter_of_its_own() {
    let value = Signal::new("alpha beta".to_owned());
    let input: Element = TextInput::new(value.clone())
      .code_highlighter(|code: &str| vec![CodeToken::new(0..code.find(' ').unwrap_or(0), token::STRING)])
      .into();
    let drawn = drawn(&input, &value);
    let success = TextColor::Color(super::super::theme::current_theme().palette_color(PaletteColor::Success));
    assert_eq!(drawn, [("alpha".to_owned(), success)]);
  }
}
