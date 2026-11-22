//! The pages of the engine's primitives: what `lurq::components`, `lurq::layout` and `lurq::core` give an app
//! below the design system, each on a page built as a component's is, with a playground first. They are the
//! pages of the demo the gallery replaced, with the primitives the demo never showed.

pub mod animation;
pub mod backgrounds;
pub mod borders;
pub mod canvas;
pub mod components;
pub mod context;
pub mod controls;
pub mod drag_and_drop;
pub mod events;
pub mod gestures;
pub mod glass;
pub mod glyph_atlas;
pub mod gpu_viewport;
pub mod grid;
pub mod kit;
pub mod layout;
pub mod overlays;
pub mod pager;
pub mod persistent_storage;
pub mod phone_stack;
pub mod position;
pub mod presence;
pub mod reactivity;
pub mod rich_text;
pub mod router;
pub mod router_front;
pub mod scroll;
pub mod sizing;
pub mod sticky;
pub mod style;
pub mod svg;
pub mod text;
pub mod text_input;
pub mod theme;
pub mod transform;
pub mod virtualized_list;
pub mod visibility;
pub mod visual;
pub mod zoom_scroll;

/// What the engine's pages need of the tree before the gallery is mounted on it: the keyframes they run, which a
/// tree is told once. Called wherever a tree of the gallery is made: the window, and the tests that lay it out.
///
/// The pages' keyframes are numbered from `0x0D00` to `0x0DFF`: lurq's own components register theirs from `0xFF00`
/// up (`AnimationEngine::new`), and an app's own start at 1, as the docs' do.
pub fn prepare(tree: &mut lurq::app::Tree) {
  animation::register_keyframes(tree);
  presence::register_keyframes(tree);
}
