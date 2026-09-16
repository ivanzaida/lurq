//! Hardware regression: cargo test -p lurq --features wgpu --test wgpu_lifecycle_tests -- --ignored
#![cfg(all(windows, feature = "wgpu"))]

use std::{
  num::NonZeroIsize,
  sync::{Arc, Barrier, Once},
};

use lurq::{
  app::{App, Tree, wgpu_render::WgpuRenderEngine},
  components::Stack,
};
use raw_window_handle::{
  DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle,
};
use windows::{
  Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
      CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_POPUP,
    },
  },
  core::w,
};

struct HiddenWindow(HWND);
impl HiddenWindow {
  fn new() -> Self {
    static REGISTER: Once = Once::new();
    let name = w!("LurqConcurrentWgpuTest");
    unsafe {
      REGISTER.call_once(|| {
        assert_ne!(
          RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(procedure),
            lpszClassName: name,
            ..Default::default()
          }),
          0
        );
      });
      Self(
        CreateWindowExW(
          WINDOW_EX_STYLE::default(),
          name,
          name,
          WS_POPUP,
          0,
          0,
          256,
          256,
          None,
          None,
          None,
          None,
        )
        .unwrap(),
      )
    }
  }
}
extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
  unsafe { DefWindowProcW(hwnd, msg, w, l) }
}
impl HasWindowHandle for HiddenWindow {
  fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
    let raw = Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).unwrap());
    Ok(unsafe { WindowHandle::borrow_raw(raw.into()) })
  }
}
impl HasDisplayHandle for HiddenWindow {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
    Ok(DisplayHandle::windows())
  }
}
impl Drop for HiddenWindow {
  fn drop(&mut self) {
    unsafe {
      let _ = DestroyWindow(self.0);
    }
  }
}

#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates two hidden surfaces"]
fn concurrent_engines_render_and_teardown() {
  // Run in separate processes too: the original loader fault depended on DLL
  // initialization/teardown and was not reliably exposed by one long process.
  for _ in 0..3 {
    let start = Arc::new(Barrier::new(2));
    let (first_tx, first_rx) = std::sync::mpsc::channel();
    let (second_tx, second_rx) = std::sync::mpsc::channel();
    let threads: Vec<_> = [(first_tx, second_rx), (second_tx, first_rx)]
      .into_iter()
      .map(|(ready, peer)| {
        let start = start.clone();
        std::thread::spawn(move || {
          let window = HiddenWindow::new();
          let mut app = App::new();
          let mut tree = Tree::new();
          tree.resize(256, 256);
          tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
          tree.set_root(Stack::new().width(256.).height(256.));
          start.wait();
          assert!(tree.pass(&mut app, &window).rendered);
          ready.send(()).unwrap();
          // Both engines have presented and are alive before either is dropped.
          peer.recv_timeout(std::time::Duration::from_secs(30)).unwrap();
          tree.request_redraw();
          assert!(tree.pass(&mut app, &window).rendered);
          drop(tree);
          drop(window);
        })
      })
      .collect();
    for thread in threads {
      thread.join().unwrap();
    }
  }
}
