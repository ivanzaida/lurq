//! The menu of editing commands a finger opens by itself: three fingers tapped, text selected by touch, the caret
//! placed by a long press. lurq shows it over the app with no code of the app's, as a phone does: a
//! [`ContextMenu`]'s bar of the commands, in the theme's context menu recipe and the locale's words.

use super::{context_menu::ContextMenu, current_locale, menu::MenuItem, theme::current_theme};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    events::EditCommand,
    window::{EditMenuRequest, Window},
  },
  components::Popup,
  core::{ElementRef, Locale, Signal},
  node::{Element, Node, NodeUpdate},
};

/// How far from the point it was asked at the bar stands, in logical pixels, so that it does not cover the fingers
/// or what they selected.
const GAP: f32 = 12.0;

/// The locale's word for an editing command.
pub(crate) fn command_word(locale: &Locale, command: EditCommand) -> std::sync::Arc<str> {
  match command {
    EditCommand::Undo => locale.editing.undo.clone(),
    EditCommand::Redo => locale.editing.redo.clone(),
    EditCommand::Cut => locale.editing.cut.clone(),
    EditCommand::Copy => locale.editing.copy.clone(),
    EditCommand::Paste => locale.editing.paste.clone(),
    EditCommand::SelectAll => locale.common.select_all.clone(),
  }
}

/// The open menu of editing commands of `window`, hung from `anchor` at the point it was asked at; `None` while it is
/// closed. Reads the window's menu, so what builds it is built again when the menu opens or closes.
pub(crate) fn edit_menu_layer(window: &Window, anchor: ElementRef) -> Option<Element> {
  let request = window.edit_menu()?;
  Some(edit_menu(window, &request, anchor))
}

fn edit_menu(window: &Window, request: &EditMenuRequest, anchor: ElementRef) -> Element {
  let locale = current_locale();
  let items = request.commands.iter().map(|&command| {
    let window = window.clone();
    MenuItem::new(&command_word(&locale, command)).on_select(move || {
      window.set_edit_menu(None);
      window.request_edit(command);
    })
  });
  let mut panel = ContextMenu::new(Signal::new(true), Element::new())
    .bar(true)
    .inline(true)
    .items(items)
    .id("lurq-edit-menu")
    .build(&current_theme());
  // A finger choosing a command leaves the focus where it was: the command is run on it.
  keep_focus(&mut panel.node);
  let menu = window.edit_menu_signal();
  let safe = window.info().safe_area;
  // Measured first, it stands centred above the point, below it where there is no room above, and inside the
  // window and its safe area: the first frame shows it where it stays.
  Popup::new(anchor, panel)
    .open_when(true)
    .at(Some((request.x, request.y)))
    .at_placed(Placement::Top)
    .offset(0.0, GAP)
    .collision(CollisionStrategy::FlipThenClamp)
    .keep_clear(safe.top, safe.right, safe.bottom, safe.left)
    .takes_focus(false)
    .switch_on_anchor_press(false)
    .dismiss_on_escape(true)
    .dismiss_on_outside_click(true)
    .outside_press(crate::app::ctx::OutsidePress::PassThrough)
    .on_dismiss(move || menu.set(None))
    .into()
}

/// Makes every node of the menu one a press does not take the focus to, nor away from what has it.
fn keep_focus(node: &mut Node) {
  NodeUpdate::focusable(node, false);
  for child in &mut node.children {
    keep_focus(child);
  }
}
