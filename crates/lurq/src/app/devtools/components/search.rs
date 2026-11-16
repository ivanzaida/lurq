//! What the search box of the Components tab understands: plain words, and `field:value` terms that look in one
//! part of a node. Every term must hold; a term that starts with `-` must not.

use super::super::{
  kit::short_tag,
  snapshot::{DevToolsNode, DevToolsNodeKind},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchQuery {
  terms: Vec<Term>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Term {
  field: Field,
  /// In lower case: a search does not tell the cases apart.
  value: String,
  negated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
  Any,
  Tag,
  Key,
  Text,
  Kind,
  State,
}

impl SearchQuery {
  /// The query a person typed, or `None` where it asks for nothing.
  pub(crate) fn parse(query: &str) -> Option<Self> {
    let terms = tokens(query)
      .iter()
      .filter_map(|token| Term::parse(token))
      .collect::<Vec<_>>();
    (!terms.is_empty()).then_some(Self { terms })
  }

  /// Whether the node itself is one the query asks for. Its ancestors are shown with it; that is the tree's to do.
  pub(crate) fn matches(&self, node: &DevToolsNode) -> bool {
    self.terms.iter().all(|term| term.holds(node) != term.negated)
  }
}

impl Term {
  fn parse(token: &str) -> Option<Self> {
    let token = token.trim();
    let (negated, token) = token.strip_prefix('-').map_or((false, token), |rest| (true, rest));
    let (field, value) = token
      .split_once(':')
      .and_then(|(field, value)| Some((Field::named(field)?, value)))
      .unwrap_or((Field::Any, token));
    let value = value.trim();
    (!value.is_empty()).then(|| Self {
      field,
      value: value.to_lowercase(),
      negated,
    })
  }

  fn holds(&self, node: &DevToolsNode) -> bool {
    let has = |text: &str| text.to_lowercase().contains(&self.value);
    let tag = || has(&node.tag) || has(short_tag(&node.tag));
    let key = || node.key.as_deref().is_some_and(has);
    let text = || node.text.as_deref().is_some_and(has);
    match self.field {
      Field::Any => tag() || key() || text(),
      Field::Tag => tag(),
      Field::Key => key(),
      Field::Text => text(),
      Field::Kind => has(match node.kind {
        DevToolsNodeKind::Component => "component",
        DevToolsNodeKind::Element => "element",
      }),
      Field::State => {
        (node.hovered && has("hovered")) || (node.active && has("active")) || (node.focused && has("focused"))
      }
    }
  }
}

impl Field {
  fn named(name: &str) -> Option<Self> {
    Some(match name.to_ascii_lowercase().as_str() {
      "tag" | "name" | "component" => Self::Tag,
      "key" => Self::Key,
      "text" | "value" => Self::Text,
      "kind" => Self::Kind,
      "state" => Self::State,
      _ => return None,
    })
  }
}

/// The words of a query: split at spaces, but for what is between quotes, where a backslash keeps the next
/// character as it is.
fn tokens(query: &str) -> Vec<String> {
  let mut tokens = Vec::new();
  let mut current = String::new();
  let mut chars = query.chars();
  let mut quoted = false;
  while let Some(ch) = chars.next() {
    match ch {
      '"' => quoted = !quoted,
      '\\' if quoted => current.extend(chars.next()),
      ch if ch.is_whitespace() && !quoted => {
        if !current.is_empty() {
          tokens.push(std::mem::take(&mut current));
        }
      }
      _ => current.push(ch),
    }
  }
  if !current.is_empty() {
    tokens.push(current);
  }
  tokens
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::IdGenerator;

  fn query(text: &str) -> SearchQuery {
    SearchQuery::parse(text).expect("the query asks for something")
  }

  fn button() -> DevToolsNode {
    let mut node = DevToolsNode::for_test(
      IdGenerator::new().next(),
      "demo::Button",
      DevToolsNodeKind::Element,
      vec![],
    );
    node.key = Some("primary-action".to_owned());
    node.text = Some("Save changes".to_owned());
    node.focused = true;
    node
  }

  #[test]
  fn nothing_typed_asks_for_nothing() {
    assert_eq!(SearchQuery::parse(""), None);
    assert_eq!(SearchQuery::parse("   "), None);
    assert_eq!(SearchQuery::parse("key:"), None);
  }

  #[test]
  fn a_plain_word_looks_in_the_tag_the_key_and_the_text() {
    let node = button();
    assert!(query("button").matches(&node));
    assert!(query("demo::button").matches(&node));
    assert!(query("primary").matches(&node));
    assert!(query("\"save changes\"").matches(&node));
    assert!(!query("missing").matches(&node));
  }

  #[test]
  fn a_field_looks_in_its_part_alone() {
    let node = button();
    assert!(query("tag:Button").matches(&node));
    assert!(query("name:Button").matches(&node));
    assert!(query("key:primary-action").matches(&node));
    assert!(!query("key:Button").matches(&node));
    assert!(query("text:\"Save changes\"").matches(&node));
    assert!(query("kind:element").matches(&node));
    assert!(!query("kind:component").matches(&node));
    assert!(query("state:focused").matches(&node));
    assert!(!query("state:hovered").matches(&node));
  }

  #[test]
  fn every_term_holds_and_a_negated_one_does_not() {
    let node = button();
    assert!(query("kind:element state:focused").matches(&node));
    assert!(!query("kind:element state:hovered").matches(&node));
    assert!(query("text:\"Save changes\" -key:secondary").matches(&node));
    assert!(!query("text:\"Save changes\" -key:primary").matches(&node));
  }

  #[test]
  fn an_unknown_field_is_a_plain_word() {
    let mut node = button();
    node.text = Some("ratio 16:9".to_owned());
    assert!(query("16:9").matches(&node));
  }
}
