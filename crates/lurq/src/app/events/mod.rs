pub mod keyboard_event;
pub mod mouse_event;
pub mod scroll_event;

pub use keyboard_event::*;
pub use mouse_event::*;
pub use scroll_event::*;

pub enum Event {
  Mouse(MouseEvent),
  Keyboard(KeyboardEvent),
  Scroll(ScrollEvent),
}
