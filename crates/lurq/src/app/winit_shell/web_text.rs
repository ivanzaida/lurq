//! Text that comes to a page other than as keys: an input method's composition (Chinese, Japanese, Korean), a dead
//! key and its letter, a phone's on-screen keyboard, dictation, the system's panel of emoji.
//!
//! winit hears a canvas's key presses and nothing else, and a canvas takes no text: the browser gives composed
//! text only to an element that edits text. So while a text input of the tree has the focus, the focus of the page
//! is on a textarea of the shell's own ([`WebText`]), kept transparent at the input's caret, where the input
//! method's window then opens. Its keys that are keys (a letter typed on a keyboard, an arrow, Backspace, a
//! shortcut) are handed on to the canvas, where winit hears them as before and the browser does nothing with
//! them; what the textarea is given as text, composed or inserted, is taken from it as the tree's
//! [`ImeEvent`]s, and the textarea is emptied again. A delete a phone's keyboard asks for as text (its own
//! Backspace) is the tree's Backspace.
//!
//! The listeners run between the loop's turns: they queue what came and wake the loop, and the next turn hands it
//! to the tree ([`WebText::take`]).

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, closure::Closure};
use winit::{platform::web::WindowExtWebSys, window::Window};

use crate::app::{
  Tree,
  events::{ImeCursorArea, ImeEvent},
  window::WindowWaker,
};

/// What came to the textarea, in order.
#[derive(Debug, PartialEq)]
pub(super) enum Typed {
  Ime(ImeEvent),
  /// A key the textarea was asked for as text: `"Backspace"`, `"Delete"`, `"Enter"`.
  Key(&'static str),
}

pub(super) struct WebText {
  textarea: web_sys::HtmlTextAreaElement,
  canvas: web_sys::HtmlCanvasElement,
  typed: Rc<RefCell<Vec<Typed>>>,
  _listeners: Vec<Closure<dyn FnMut(web_sys::Event)>>,
  /// Whether the textarea has the page's focus, for a text input of the tree.
  active: bool,
  area: Option<ImeCursorArea>,
}

impl WebText {
  /// The textarea, put into the page beside the window's canvas. `None` where the page has no place for it.
  pub(super) fn new(window: &Window, wake: WindowWaker) -> Option<Self> {
    let canvas = window.canvas()?;
    let document = web_sys::window()?.document()?;
    let textarea: web_sys::HtmlTextAreaElement = document.create_element("textarea").ok()?.dyn_into().ok()?;
    for (name, value) in [
      ("autocomplete", "off"),
      ("autocorrect", "off"),
      ("autocapitalize", "off"),
      ("spellcheck", "false"),
      ("aria-hidden", "true"),
      ("tabindex", "-1"),
    ] {
      let _ = textarea.set_attribute(name, value);
    }
    // There, and seen by nobody: no caret, no text, no box, nothing the pointer meets.
    let _ = textarea.set_attribute(
      "style",
      "position:fixed;left:0;top:0;width:1px;height:1em;padding:0;border:0;margin:0;outline:none;resize:none;\
       overflow:hidden;opacity:0;color:transparent;background:transparent;caret-color:transparent;\
       pointer-events:none;z-index:-1;font-size:16px;white-space:pre",
    );
    document.body()?.append_child(&textarea).ok()?;
    let typed = Rc::new(RefCell::new(Vec::new()));
    let mut listeners = Vec::new();
    let mut listen = |kind: &str, handler: Box<dyn FnMut(web_sys::Event)>| {
      let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(handler);
      let _ = textarea.add_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
      listeners.push(listener);
    };
    // A key that is a key goes to the canvas, as if it had the focus.
    for kind in ["keydown", "keyup"] {
      let canvas = canvas.clone();
      listen(
        kind,
        Box::new(move |event: web_sys::Event| {
          let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
            return;
          };
          if composes(key) {
            return;
          }
          event.prevent_default();
          if let Some(copy) = copy_of(kind, key) {
            let _ = canvas.dispatch_event(&copy);
          }
        }),
      );
    }
    {
      let (typed, wake) = (typed.clone(), wake.clone());
      listen(
        "compositionstart",
        Box::new(move |_| {
          typed.borrow_mut().push(Typed::Ime(ImeEvent::Enabled));
          wake();
        }),
      );
    }
    {
      let (typed, wake) = (typed.clone(), wake.clone());
      listen(
        "compositionupdate",
        Box::new(move |event: web_sys::Event| {
          let text = event
            .dyn_ref::<web_sys::CompositionEvent>()
            .and_then(web_sys::CompositionEvent::data)
            .unwrap_or_default();
          let end = text.len();
          typed.borrow_mut().push(Typed::Ime(ImeEvent::Preedit {
            text,
            cursor: Some((end, end)),
          }));
          wake();
        }),
      );
    }
    {
      let (typed, wake, area) = (typed.clone(), wake.clone(), textarea.clone());
      listen(
        "compositionend",
        Box::new(move |event: web_sys::Event| {
          let text = event
            .dyn_ref::<web_sys::CompositionEvent>()
            .and_then(web_sys::CompositionEvent::data)
            .unwrap_or_default();
          let mut typed = typed.borrow_mut();
          typed.push(Typed::Ime(ImeEvent::Preedit {
            text: String::new(),
            cursor: None,
          }));
          if !text.is_empty() {
            typed.push(Typed::Ime(ImeEvent::Commit(text)));
          }
          area.set_value("");
          wake();
        }),
      );
    }
    {
      let (typed, wake, area) = (typed.clone(), wake, textarea.clone());
      listen(
        "input",
        Box::new(move |event: web_sys::Event| {
          let Some(input) = event.dyn_ref::<web_sys::InputEvent>() else {
            return;
          };
          // A composition says what it makes when it ends.
          if input.is_composing() {
            return;
          }
          let came = match input.input_type().as_str() {
            "deleteContentBackward" | "deleteWordBackward" | "deleteSoftLineBackward" => Some(Typed::Key("Backspace")),
            "deleteContentForward" | "deleteWordForward" | "deleteSoftLineForward" => Some(Typed::Key("Delete")),
            "insertLineBreak" | "insertParagraph" => Some(Typed::Key("Enter")),
            _ => input
              .data()
              .filter(|text| !text.is_empty())
              .map(|text| Typed::Ime(ImeEvent::Commit(text))),
          };
          area.set_value("");
          if let Some(came) = came {
            typed.borrow_mut().push(came);
            wake();
          }
        }),
      );
    }
    Some(Self {
      textarea,
      canvas,
      typed,
      _listeners: listeners,
      active: false,
      area: None,
    })
  }

  /// What came since the last turn, in order.
  pub(super) fn take(&self) -> Vec<Typed> {
    std::mem::take(&mut *self.typed.borrow_mut())
  }

  /// Gives the page's focus to the textarea while a text input of the tree has it, at the input's caret, and
  /// back to the canvas when none has.
  pub(super) fn follow(&mut self, tree: &Tree) {
    let wanted = tree.ime_target().is_some();
    if wanted != self.active {
      self.active = wanted;
      self.area = None;
      if wanted {
        let _ = self.textarea.focus();
      } else if self.has_focus() {
        self.textarea.set_value("");
        let _ = self.canvas.focus();
      }
    }
    if !wanted {
      return;
    }
    // The canvas took the focus back (a press on it): the textarea takes it again, the input still has it.
    if !self.has_focus() && self.canvas_has_focus() {
      let _ = self.textarea.focus();
    }
    let area = tree.ime_cursor_area();
    if area == self.area {
      return;
    }
    self.area = area;
    if let Some(area) = area {
      let origin = self.canvas.get_bounding_client_rect();
      let style = self.textarea.style();
      let _ = style.set_property("left", &format!("{}px", origin.left() + f64::from(area.x)));
      let _ = style.set_property("top", &format!("{}px", origin.top() + f64::from(area.y)));
      let _ = style.set_property("height", &format!("{}px", area.height.max(1.0)));
    }
  }

  fn has_focus(&self) -> bool {
    let textarea: &web_sys::Element = self.textarea.as_ref();
    active_element().is_some_and(|element| element == *textarea)
  }

  fn canvas_has_focus(&self) -> bool {
    let canvas: &web_sys::Element = self.canvas.as_ref();
    active_element().is_some_and(|element| element == *canvas)
  }
}

impl Drop for WebText {
  fn drop(&mut self) {
    self.textarea.remove();
  }
}

fn active_element() -> Option<web_sys::Element> {
  web_sys::window()?.document()?.active_element()
}

/// Whether a key belongs to an input method or a dead key: the textarea's to compose with.
fn composes(key: &web_sys::KeyboardEvent) -> bool {
  key.is_composing() || matches!(key.key().as_str(), "Process" | "Dead" | "Unidentified") || key.key_code() == 229
}

/// The same key, to give the canvas.
fn copy_of(kind: &str, key: &web_sys::KeyboardEvent) -> Option<web_sys::Event> {
  let init = web_sys::KeyboardEventInit::new();
  init.set_key(&key.key());
  init.set_code(&key.code());
  init.set_location(key.location());
  init.set_repeat(key.repeat());
  init.set_ctrl_key(key.ctrl_key());
  init.set_shift_key(key.shift_key());
  init.set_alt_key(key.alt_key());
  init.set_meta_key(key.meta_key());
  init.set_bubbles(true);
  init.set_cancelable(true);
  web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init)
    .ok()
    .map(Into::into)
}

/// Hands what came to the tree: text as the input method's, and a key asked for as text as that key.
pub(super) fn give(tree: &mut Tree, typed: Vec<Typed>) {
  for came in typed {
    match came {
      Typed::Ime(event) => tree.ime(event),
      Typed::Key(key) => {
        tree.key_down_with_meta(key.to_owned(), key.to_owned(), false, false, false, false);
        tree.key_up_with_meta(key.to_owned(), key.to_owned(), false, false, false, false);
      }
    }
  }
}
