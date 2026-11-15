//! A document is drawn in the theme it is built in: mounted inside `ctx.in_theme(&other, ..)`, every role it draws
//! with is that theme's, with the app's theme the other appearance; also when that theme changes its appearance
//! later, and when the document was first mounted outside it and is mounted inside it after (a playground's stage
//! switched to the other appearance).

use std::cell::RefCell;

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, Theme},
    window::Appearance,
  },
  components::{Markdown, MarkdownProps},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
};

use super::links::{pass, quads};

const SOURCE: &str = "# Heading\n\nWords with a [link](https://lurq.dev) and `code`.\n\n> Quoted words.\n\n| Name | Kind |\n| --- \
                      | --- |\n| radius | f32 |\n\n> [!NOTE]\n> Noted words.\n\n```rust\nfn main() {}\n```\n\nA \
                      reference[^1].\n\n[^1]: The footnote.\n";

#[derive(Clone)]
struct Scene {
  inner: Theme,
  scoped: Signal<bool>,
}

thread_local! {
  static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

struct Host;

impl Component for Host {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = SCENE.with(|slot| slot.borrow().clone().expect("a scene"));
    let props = MarkdownProps::new(SOURCE).width(600.0).selectable(true);
    if scene.scoped.get() {
      ctx.in_theme(&scene.inner, |ctx| Markdown::mount(ctx, props))
    } else {
      Markdown::mount(ctx, props)
    }
  }
}

/// The app in `app_look`, the document in a theme of `inner_look` where `scoped`.
fn mount(app_look: Appearance, inner_look: Appearance, scoped: bool) -> (App, Tree, Scene) {
  let inner = Theme::new();
  inner.set_preferred_appearance(Some(inner_look));
  let scene = Scene {
    inner,
    scoped: Signal::new(scoped),
  };
  SCENE.with(|slot| *slot.borrow_mut() = Some(scene.clone()));
  let mut app = App::new();
  app.theme().set_preferred_appearance(Some(app_look));
  let mut tree = Tree::new();
  tree.resize(700, 900);
  tree.mount_root::<Host>(&mut app, ());
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  (app, tree, scene)
}

/// The color the words `text` are drawn in.
fn color_of(tree: &Tree, text: &str) -> Color {
  quads(tree)
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::RichText { spans, .. } => spans
        .iter()
        .find(|span| span.text.trim() == text)
        .map(|span| span.style.color),
      QuadContent::Text { text: words, style, .. } if words.trim() == text => Some(style.color),
      _ => None,
    })
    .unwrap_or_else(|| panic!("{text:?} is drawn"))
}

/// Each kind of block's words are in `theme`'s roles, and none in the foreground of the other appearance.
fn assert_in(tree: &Tree, theme: &Theme, other: &Theme, what: &str) {
  let role = |role: PaletteColor| theme.palette().get(role);
  for (text, expected) in [
    ("Heading", PaletteColor::Foreground),
    ("Words with a", PaletteColor::Foreground),
    ("link", PaletteColor::Info),
    ("Quoted words.", PaletteColor::MutedForeground),
    ("radius", PaletteColor::Foreground),
    ("Noted words.", PaletteColor::Foreground),
    ("fn", PaletteColor::Info),
    ("The footnote.", PaletteColor::MutedForeground),
  ] {
    assert_eq!(
      color_of(tree, text),
      role(expected.clone()),
      "{what}: {text:?} in the theme's {expected:?}"
    );
  }
  let wrong = other.palette().get(PaletteColor::Foreground);
  assert_ne!(wrong, role(PaletteColor::Foreground));
  let drawn_in_wrong: Vec<_> = quads(tree)
    .into_iter()
    .flat_map(|quad| match quad.content {
      QuadContent::RichText { spans, .. } => spans
        .into_iter()
        .filter(|span| span.style.color == wrong && !span.text.trim().is_empty())
        .map(|span| span.text)
        .collect(),
      QuadContent::Text { text, style, .. } if style.color == wrong && !text.trim().is_empty() => vec![text],
      _ => Vec::new(),
    })
    .collect();
  assert!(
    drawn_in_wrong.is_empty(),
    "{what}: words in the other appearance's foreground: {drawn_in_wrong:?}"
  );
}

/// A theme in `look`, for its colors.
fn theme_in(look: Appearance) -> Theme {
  let theme = Theme::new();
  theme.set_preferred_appearance(Some(look));
  theme
}

#[test]
fn a_document_in_a_light_theme_under_a_dark_app_is_drawn_in_the_light_one() {
  let (_app, tree, scene) = mount(Appearance::Dark, Appearance::Light, true);
  assert_in(&tree, &scene.inner, &theme_in(Appearance::Dark), "light in dark");
}

#[test]
fn a_document_in_a_dark_theme_under_a_light_app_is_drawn_in_the_dark_one() {
  let (_app, tree, scene) = mount(Appearance::Light, Appearance::Dark, true);
  assert_in(&tree, &scene.inner, &theme_in(Appearance::Light), "dark in light");
}

#[test]
fn a_document_follows_its_theme_when_it_turns_to_the_other_appearance() {
  let (mut app, mut tree, scene) = mount(Appearance::Dark, Appearance::Light, true);
  scene.inner.set_preferred_appearance(Some(Appearance::Dark));
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert_in(&tree, &scene.inner, &theme_in(Appearance::Light), "turned dark");
  scene.inner.set_preferred_appearance(Some(Appearance::Light));
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert_in(&tree, &scene.inner, &theme_in(Appearance::Dark), "turned light again");
}

#[test]
fn a_document_mounted_outside_a_theme_and_then_inside_it_is_drawn_in_it() {
  // A stage switched to the other appearance: the document's component is the same, in another theme.
  let (mut app, mut tree, scene) = mount(Appearance::Dark, Appearance::Light, false);
  assert_in(
    &tree,
    &app.theme().clone(),
    &theme_in(Appearance::Light),
    "in the app's",
  );
  scene.scoped.set(true);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert_in(&tree, &scene.inner, &theme_in(Appearance::Dark), "then in its own");
  scene.scoped.set(false);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert_in(&tree, &app.theme().clone(), &theme_in(Appearance::Light), "and back");
}
