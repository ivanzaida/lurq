//! The procedure over a window that is never shown, in the place winit's is: what it answers the system's hit
//! test with, and what it gives the procedure under it for the non-client mouse messages the system then sends.
//! The window stands far off every screen, so the pointer is never over it.

use std::{
  cell::RefCell,
  sync::{
    Arc, Once,
    atomic::{AtomicU32, Ordering},
  },
};

use windows::{
  Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
      CreateWindowExW, DefWindowProcW, DestroyWindow, HTCAPTION, HTCLIENT, HTCLOSE, HTMAXBUTTON, HTMINBUTTON,
      RegisterClassW, SendMessageW, WINDOW_EX_STYLE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCHITTEST,
      WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_NCRBUTTONDOWN,
      WM_NCXBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN, WNDCLASSW, WS_POPUP,
    },
  },
  core::w,
};

use super::{WM_MOUSELEAVE, client_button_message, hit_code, lay_over, point, point_lparam};
use crate::{
  app::{App, Tree, events::MouseButton},
  components::{Rect, Row},
  core::ElementRef,
  node::WindowControl,
};

const BAR_HEIGHT: f32 = 36.0;
const CONTROL_WIDTH: f32 = 46.0;
/// Where the window stands: off every screen, within what a mouse message's point holds.
const ORIGIN: i32 = -30000;

thread_local! {
  /// The mouse messages the procedure under the one tested heard, as winit's would.
  static HEARD: RefCell<Vec<(u32, usize, isize)>> = const { RefCell::new(Vec::new()) };
}

/// The procedure under the one tested: it keeps the mouse messages it hears and leaves the rest to the system.
unsafe extern "system" fn under(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  // The mouse messages, client and non-client: 0x00A0..=0x00AD and 0x0200..=0x020D, and the leave.
  if matches!(message, 0x00A0..=0x00AD | 0x0200..=0x020D | WM_MOUSELEAVE | WM_NCMOUSELEAVE) {
    HEARD.with_borrow_mut(|heard| heard.push((message, wparam.0, lparam.0)));
    return LRESULT(0);
  }
  // SAFETY: the system's procedure, for the window the message is for.
  unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// A window with no frame, never shown, and a tree whose controls the procedure laid over it answers from.
struct Stand {
  hwnd: HWND,
  tree: Tree,
  app: App,
  maximize: ElementRef,
  clicks: Arc<AtomicU32>,
}

impl Drop for Stand {
  fn drop(&mut self) {
    // SAFETY: the test's own window.
    let _ = unsafe { DestroyWindow(self.hwnd) };
  }
}

impl Stand {
  fn new() -> Self {
    static CLASS: Once = Once::new();
    let class = w!("lurq-window-control-hits-test");
    CLASS.call_once(|| {
      let registered = unsafe {
        RegisterClassW(&WNDCLASSW {
          lpfnWndProc: Some(under),
          lpszClassName: class,
          ..Default::default()
        })
      };
      assert_ne!(registered, 0, "the test's window class");
    });
    // SAFETY: a window of the class registered above, on this thread; it is never shown.
    let hwnd = unsafe {
      CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        w!(""),
        WS_POPUP,
        ORIGIN,
        ORIGIN,
        400,
        300,
        None,
        None,
        None,
        None,
      )
    }
    .expect("the test's window");
    let mut tree = Tree::new();
    tree.resize(400, 300);
    tree.publish_window_controls();
    lay_over(hwnd.0 as isize, tree.window().clone());
    HEARD.with_borrow_mut(Vec::clear);
    Self {
      hwnd,
      tree,
      app: App::new(),
      maximize: ElementRef::new(),
      clicks: Arc::new(AtomicU32::new(0)),
    }
  }

  /// Lays out a title bar: room of `room` wide, then the three controls if `marked`, or plain elements in their
  /// place.
  fn bar(&mut self, room: f32, marked: bool) {
    let control = |control: WindowControl| {
      let element = Rect::new(CONTROL_WIDTH, BAR_HEIGHT);
      if marked {
        element.window_control(control)
      } else {
        element
      }
    };
    let clicks = self.clicks.clone();
    self.tree.set_root(
      Row::new()
        .height(BAR_HEIGHT)
        .child(Rect::new(room, BAR_HEIGHT))
        .child(control(WindowControl::Minimize))
        .child(
          control(WindowControl::Maximize)
            .ref_element(self.maximize.clone())
            .on_click(move |_| {
              clicks.fetch_add(1, Ordering::Relaxed);
            }),
        )
        .child(control(WindowControl::Close)),
    );
    self.tree.pass_headless(&mut self.app);
  }

  /// The point `(x, y)` of the client area as a non-client mouse message carries it: on the screen.
  fn screen(&self, x: i32, y: i32) -> LPARAM {
    let mut at = POINT { x, y };
    // SAFETY: the test's own window.
    assert!(unsafe { ClientToScreen(self.hwnd, &mut at) }.as_bool());
    point_lparam(at)
  }

  fn send(&self, message: u32, wparam: usize, lparam: LPARAM) -> isize {
    // SAFETY: the test's own window, on its own thread: the procedure is called at once.
    unsafe { SendMessageW(self.hwnd, message, Some(WPARAM(wparam)), Some(lparam)) }.0
  }

  /// What the window answers the system's hit test with at `(x, y)` of its client area.
  fn hit(&self, x: i32, y: i32) -> u32 {
    self.send(WM_NCHITTEST, 0, self.screen(x, y)) as u32
  }

  /// The messages heard under the procedure since the last call, as (message, point in the client area), given to
  /// the tree as the shell gives it what winit hears: a move, a press and a release at the point of the last move,
  /// and the pointer leaving.
  fn deliver(&mut self) -> Vec<(u32, i32, i32)> {
    let heard = HEARD.with_borrow_mut(std::mem::take);
    let mut seen = Vec::new();
    for (message, _, lparam) in heard {
      let at = point(LPARAM(lparam));
      let (x, y) = (at.x as f32, at.y as f32);
      match message {
        WM_MOUSEMOVE => self.tree.mouse_move(x, y),
        WM_LBUTTONDOWN => self.tree.mouse_down(x, y, MouseButton::Left),
        WM_LBUTTONUP => self.tree.mouse_up(x, y, MouseButton::Left),
        WM_MOUSELEAVE => self.tree.mouse_leave_window(),
        _ => {}
      }
      seen.push((message, at.x, at.y));
    }
    seen
  }
}

#[test]
fn the_three_controls_have_the_systems_three_answers() {
  assert_eq!(hit_code(WindowControl::Minimize), HTMINBUTTON);
  assert_eq!(hit_code(WindowControl::Maximize), HTMAXBUTTON);
  assert_eq!(hit_code(WindowControl::Close), HTCLOSE);
}

#[test]
fn a_point_keeps_its_sign_through_a_message() {
  // Screens left of or above the first have points below zero.
  for at in [(0, 0), (1440, 900), (-100, -20), (-30000, 250)] {
    let back = point(point_lparam(POINT { x: at.0, y: at.1 }));
    assert_eq!((back.x, back.y), at);
  }
}

#[test]
fn a_second_press_of_a_double_click_is_a_press() {
  assert_eq!(client_button_message(WM_NCLBUTTONDOWN), Some(WM_LBUTTONDOWN));
  assert_eq!(client_button_message(WM_NCLBUTTONDBLCLK), Some(WM_LBUTTONDOWN));
  assert_eq!(client_button_message(WM_NCLBUTTONUP), Some(WM_LBUTTONUP));
  assert_eq!(client_button_message(WM_NCRBUTTONDOWN), Some(WM_RBUTTONDOWN));
  assert_eq!(client_button_message(WM_NCXBUTTONDOWN), Some(WM_XBUTTONDOWN));
  assert_eq!(client_button_message(WM_NCMOUSEMOVE), None);
}

#[test]
fn the_hit_test_answers_the_controls_where_they_are_laid_out() {
  let mut stand = Stand::new();
  // Nothing laid out yet, then a bar with no element marked: the client area everywhere.
  assert_eq!(stand.hit(250, 18), HTCLIENT);
  stand.bar(200.0, false);
  for x in [100, 220, 250, 300] {
    assert_eq!(stand.hit(x, 18), HTCLIENT, "{x}");
  }
  // The three marked.
  stand.bar(200.0, true);
  assert_eq!(stand.hit(100, 18), HTCLIENT);
  assert_eq!(stand.hit(220, 18), HTMINBUTTON);
  assert_eq!(stand.hit(250, 18), HTMAXBUTTON);
  assert_eq!(stand.hit(300, 18), HTCLOSE);
  assert_eq!(stand.hit(250, 40), HTCLIENT, "under the bar");
  // The layout changes: the answer follows it.
  stand.bar(250.0, true);
  assert_eq!(stand.hit(220, 18), HTCLIENT);
  assert_eq!(stand.hit(260, 18), HTMINBUTTON);
  assert_eq!(stand.hit(300, 18), HTMAXBUTTON);
  // At another scale the controls are where they are drawn: the maximize one at 369..438 physical.
  stand.tree.set_scale_factor(1.5);
  stand.tree.resize(400, 300);
  stand.bar(200.0, true);
  assert_eq!(stand.hit(368, 20), HTMINBUTTON);
  assert_eq!(stand.hit(370, 20), HTMAXBUTTON);
  assert_eq!(stand.hit(370, 55), HTCLIENT);
}

#[test]
fn the_non_client_pointer_over_a_control_is_the_elements() {
  let mut stand = Stand::new();
  stand.bar(200.0, true);
  let on = stand.screen(250, 18);
  let code = HTMAXBUTTON as usize;

  // The pointer comes over the control: the element is under it.
  assert_eq!(stand.send(WM_NCMOUSEMOVE, code, on), 0);
  assert_eq!(stand.deliver(), [(WM_MOUSEMOVE, 250, 18)]);
  assert!(stand.maximize.hovered());

  // A press and a release on it: held down, then clicked.
  stand.send(WM_NCLBUTTONDOWN, code, on);
  assert_eq!(stand.deliver(), [(WM_MOUSEMOVE, 250, 18), (WM_LBUTTONDOWN, 250, 18)]);
  assert!(stand.maximize.active());
  stand.send(WM_NCLBUTTONUP, code, on);
  assert_eq!(stand.deliver(), [(WM_MOUSEMOVE, 250, 18), (WM_LBUTTONUP, 250, 18)]);
  assert!(!stand.maximize.active());
  assert_eq!(stand.clicks.load(Ordering::Relaxed), 1);

  // A press dragged off it and let go: winit holds the pointer after a press, so the rest are client messages.
  stand.send(WM_NCLBUTTONDOWN, code, on);
  stand.send(WM_MOUSEMOVE, 1, point_lparam(POINT { x: 100, y: 18 }));
  stand.send(WM_LBUTTONUP, 0, point_lparam(POINT { x: 100, y: 18 }));
  stand.deliver();
  assert_eq!(stand.clicks.load(Ordering::Relaxed), 1, "let go off the control");

  // The pointer leaves the window from the control: winit is told it left, and the element is under it no more.
  stand.send(WM_NCMOUSEMOVE, code, on);
  stand.deliver();
  assert!(stand.maximize.hovered());
  stand.send(WM_NCMOUSELEAVE, 0, LPARAM(0));
  assert_eq!(stand.deliver(), [(WM_MOUSELEAVE, 0, 0), (WM_NCMOUSELEAVE, 0, 0)]);
  assert!(!stand.maximize.hovered());
}

#[test]
fn other_non_client_messages_are_left_as_they_are() {
  let mut stand = Stand::new();
  stand.bar(200.0, true);
  // The bar's own room, which the window is dragged by: the press is the system's.
  let room = stand.screen(100, 18);
  stand.send(WM_NCMOUSEMOVE, HTCAPTION as usize, room);
  assert_eq!(stand.deliver().len(), 1);
  // The answer of a button of a title bar the system draws, where the app has no control.
  stand.send(WM_NCLBUTTONDOWN, HTMAXBUTTON as usize, room);
  let heard = stand.deliver();
  assert_eq!(heard.len(), 1);
  assert_eq!(heard[0].0, WM_NCLBUTTONDOWN);
  // No element marked: the same messages over where the controls were are left alone too.
  stand.bar(200.0, false);
  stand.send(WM_NCMOUSEMOVE, HTMAXBUTTON as usize, stand.screen(250, 18));
  let heard = stand.deliver();
  assert_eq!(heard.len(), 1);
  assert_eq!(heard[0].0, WM_NCMOUSEMOVE);
  assert!(!stand.maximize.hovered());
}
