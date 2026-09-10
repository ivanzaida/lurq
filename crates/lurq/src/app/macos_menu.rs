//! AppKit ownership stays on the main thread. Native callbacks only enqueue
//! work, because AppKit can call them while winit has borrowed its handler.
use std::{cell::RefCell, collections::HashSet, ffi::CStr, mem};

use objc2::{
  ClassType, msg_send,
  rc::Retained,
  runtime::{AnyClass, AnyObject, ClassBuilder, Imp, Sel},
  sel,
};
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSString};

use super::{
  CloseRequestSource,
  menu::*,
  window::{Window, WindowCommand},
};

type SendEvent = unsafe extern "C-unwind" fn(&NSApplication, Sel, &NSEvent);
struct State {
  controller: MenuController,
  window: Window,
  target: Retained<AnyObject>,
  version: Option<u64>,
  original_send: SendEvent,
  consumed: HashSet<u16>,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

unsafe extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: &NSApplication) -> usize {
  // NSTerminateCancel returns to the regular run loop so the in-tree dialog can
  // paint. Acceptance exits winit normally; NSTerminateLater enters a modal loop.
  let window = STATE.with(|s| s.borrow().as_ref().map(|s| s.window.clone()));
  if let Some(window) = window {
    window.requeue_command(WindowCommand::RequestClose(CloseRequestSource::Os));
  }
  0
}
unsafe extern "C-unwind" fn activate(_: &AnyObject, _: Sel, item: &NSMenuItem) {
  let controller = STATE.with(|s| s.borrow().as_ref().map(|s| s.controller.clone()));
  if let (Some(controller), Some(value)) = (controller, item.representedObject()) {
    // We exclusively create these items and store an NSString as representedObject.
    let ptr: *const std::ffi::c_char = unsafe { msg_send![&*value, UTF8String] };
    if !ptr.is_null() {
      controller.activate(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().as_ref());
    }
  }
}
unsafe extern "C-unwind" fn send_event(app: &NSApplication, selector: Sel, event: &NSEvent) {
  let original = STATE.with(|s| s.borrow().as_ref().unwrap().original_send);
  let kind = event.r#type();
  if kind == NSEventType::KeyDown {
    // Ask the native menu exactly once, before winit/tree shortcuts. AppKit's
    // default sendEvent receives only unhandled events. Suppress the matching up
    // event too (winit otherwise forwards Command key-up explicitly).
    if let Some(menu) = app.mainMenu() {
      if menu.performKeyEquivalent(event) {
        STATE.with(|s| {
          s.borrow_mut().as_mut().unwrap().consumed.insert(event.keyCode());
        });
        return;
      }
    }
  } else if kind == NSEventType::KeyUp {
    if STATE.with(|s| s.borrow_mut().as_mut().unwrap().consumed.remove(&event.keyCode())) {
      return;
    }
  }
  unsafe {
    original(app, selector, event);
  }
}

pub(crate) struct NativeMenu {
  app: Retained<NSApplication>,
  delegate: Retained<AnyObject>,
  original_class: &'static AnyClass,
  send_method: &'static objc2::runtime::Method,
  original_send: Imp,
  old_menu: Option<Retained<NSMenu>>,
  old_services: Option<Retained<NSMenu>>,
}
impl NativeMenu {
  pub(crate) fn install(controller: MenuController, window: Window) -> Self {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let delegate: Retained<AnyObject> = unsafe { msg_send![&*app, delegate] };
    let original_class = delegate.class();
    // Subclass only this delegate instance, with no added ivars. Winit requires
    // its delegate's class/layout and callbacks: replacing it with a proxy breaks
    // ApplicationDelegate::get. This preserves both, including future callbacks.
    let class = AnyClass::get(c"LurqCloseAwareWinitDelegate").unwrap_or_else(|| {
      let mut builder = ClassBuilder::new(c"LurqCloseAwareWinitDelegate", original_class).unwrap();
      unsafe {
        builder.add_method(
          sel!(applicationShouldTerminate:),
          should_terminate as unsafe extern "C-unwind" fn(_, _, _) -> _,
        );
      }
      builder.register()
    });
    unsafe {
      AnyObject::set_class(&delegate, class);
    }
    let target_class = AnyClass::get(c"LurqMenuTarget").unwrap_or_else(|| {
      let mut builder = ClassBuilder::new(c"LurqMenuTarget", NSObject::class()).unwrap();
      unsafe {
        builder.add_method(sel!(lurqActivate:), activate as unsafe extern "C-unwind" fn(_, _, _));
      }
      builder.register()
    });
    let target: Retained<AnyObject> = unsafe { msg_send![target_class, new] };
    let send_method = app.class().instance_method(sel!(sendEvent:)).unwrap();
    let original_send = unsafe { send_method.set_implementation(mem::transmute::<SendEvent, Imp>(send_event)) };
    STATE.with(|s| {
      *s.borrow_mut() = Some(State {
        controller,
        window,
        target,
        version: None,
        original_send: unsafe { mem::transmute::<Imp, SendEvent>(original_send) },
        consumed: HashSet::new(),
      })
    });
    let old_menu = app.mainMenu();
    let old_services = app.servicesMenu();
    Self {
      app,
      delegate,
      original_class,
      send_method,
      original_send,
      old_menu,
      old_services,
    }
  }
}
impl Drop for NativeMenu {
  fn drop(&mut self) {
    self.app.setMainMenu(self.old_menu.as_deref());
    unsafe {
      self.app.setServicesMenu(self.old_services.as_deref());
      self.send_method.set_implementation(self.original_send);
      AnyObject::set_class(&self.delegate, self.original_class);
    }
    STATE.with(|s| *s.borrow_mut() = None);
  }
}

fn item(mtm: MainThreadMarker, label: &str, key: &str, action: Option<Sel>) -> Retained<NSMenuItem> {
  unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc(),
      &NSString::from_str(label),
      action,
      &NSString::from_str(key),
    )
  }
}
fn menu(mtm: MainThreadMarker, title: &str) -> Retained<NSMenu> {
  let menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(title));
  menu.setAutoenablesItems(false);
  menu
}
fn add_action(parent: &NSMenu, mtm: MainThreadMarker, target: &AnyObject, action: &MenuAction) {
  let key = action
    .accelerator
    .as_ref()
    .map(Accelerator::key_equivalent)
    .unwrap_or_default();
  let item = item(mtm, &action.label, &key, Some(sel!(lurqActivate:)));
  unsafe {
    item.setTarget(Some(target));
    item.setRepresentedObject(Some(&NSString::from_str(&action.id)));
  }
  item.setEnabled(action.enabled);
  let m = action.accelerator.as_ref().map(|a| a.modifiers).unwrap_or_default();
  let mut mask = NSEventModifierFlags::empty();
  if m.meta {
    mask |= NSEventModifierFlags::Command;
  }
  if m.ctrl {
    mask |= NSEventModifierFlags::Control;
  }
  if m.alt {
    mask |= NSEventModifierFlags::Option;
  }
  if m.shift {
    mask |= NSEventModifierFlags::Shift;
  }
  item.setKeyEquivalentModifierMask(mask);
  parent.addItem(&item);
}
fn add_submenu(parent: &NSMenu, mtm: MainThreadMarker, title: &str, child: &NSMenu) {
  let item = item(mtm, title, "", None);
  item.setSubmenu(Some(child));
  parent.addItem(&item);
}
fn build_menu(mtm: MainThreadMarker, target: &AnyObject, model: &Menu) -> Retained<NSMenu> {
  let result = menu(mtm, &model.title);
  for entry in &model.items {
    match entry {
      MenuItem::Item(action) => add_action(&result, mtm, target, action),
      MenuItem::Separator => result.addItem(&NSMenuItem::separatorItem(mtm)),
      MenuItem::Submenu(child) => add_submenu(&result, mtm, &child.title, &build_menu(mtm, target, child)),
    }
  }
  result
}
pub(crate) fn sync() {
  let update = STATE.with(|s| {
    let mut s = s.borrow_mut();
    let s = s.as_mut()?;
    let (version, model) = s.controller.snapshot(s.version)?;
    s.version = Some(version);
    model.map(|model| (model, s.target.clone()))
  });
  let Some((model, target)) = update else {
    return;
  };
  let mtm = MainThreadMarker::new().unwrap();
  let app = NSApplication::sharedApplication(mtm);
  let bar = menu(mtm, "");
  let application = menu(mtm, &model.application.name);
  if let Some(action) = &model.application.about {
    add_action(&application, mtm, &target, action);
  }
  if let Some(action) = &model.application.preferences {
    add_action(&application, mtm, &target, action);
  }
  application.addItem(&NSMenuItem::separatorItem(mtm));
  let services = menu(mtm, "Services");
  add_submenu(&application, mtm, "Services", &services);
  app.setServicesMenu(Some(&services));
  for (label, key, selector, flags) in [
    (
      format!("Hide {}", model.application.name),
      "h",
      sel!(hide:),
      NSEventModifierFlags::Command,
    ),
    (
      "Hide Others".into(),
      "h",
      sel!(hideOtherApplications:),
      NSEventModifierFlags::Command | NSEventModifierFlags::Option,
    ),
    (
      "Show All".into(),
      "",
      sel!(unhideAllApplications:),
      NSEventModifierFlags::empty(),
    ),
  ] {
    let item = item(mtm, &label, key, Some(selector));
    unsafe {
      item.setTarget(Some(&app));
    }
    item.setKeyEquivalentModifierMask(flags);
    application.addItem(&item);
  }
  application.addItem(&NSMenuItem::separatorItem(mtm));
  add_action(&application, mtm, &target, &model.application.quit);
  add_submenu(&bar, mtm, &model.application.name, &application);
  for model in &model.menus {
    add_submenu(&bar, mtm, &model.title, &build_menu(mtm, &target, model));
  }
  app.setMainMenu(Some(&bar));
}
