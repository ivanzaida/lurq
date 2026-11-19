// simtouch: real touches on the iOS simulator shown in Xcode's DeviceHub, made of the Mac's mouse events.
//
// The simulator turns a click on its screen into a finger, as a person's finger is: UIKit's own views take it,
// the system's pickers, menus and sheets among them. Coordinates are in pixels of `xcrun simctl io <id>
// screenshot` (1206x2622 for an iPhone 17 upright; the two are swapped on its side): what is seen in a
// screenshot is where to touch.
//
//   simtouch tap X Y
//   simtouch long X Y [MS]                 a finger held, 800 ms unless told
//   simtouch drag X1 Y1 X2 Y2 [MS]         a finger moved, 300 ms unless told
//   simtouch pinch X1 Y1 X2 Y2 [MS]        two fingers, mirrored about the screen's middle (Option held)
//   simtouch frame                         where the screen is on the Mac's display, in points
//
// SIM_PX=WxH says the screenshot's size when it is not an iPhone 17's.

import AppKit
import ApplicationServices

func fail(_ message: String) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(1)
}

let portrait: (Double, Double) = {
  if let size = ProcessInfo.processInfo.environment["SIM_PX"] {
    let parts = size.split(separator: "x").compactMap { Double($0) }
    if parts.count == 2 { return (min(parts[0], parts[1]), max(parts[0], parts[1])) }
  }
  return (1206, 2622)
}()

guard let hub = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.dt.Devices").first
  ?? NSWorkspace.shared.runningApplications.first(where: { $0.localizedName == "DeviceHub" })
else { fail("DeviceHub is not running: open /Applications/Xcode.app/Contents/Applications/DeviceHub.app") }

func attribute<T>(_ element: AXUIElement, _ name: String) -> T? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
  return value as? T
}

func frame(of element: AXUIElement) -> CGRect? {
  guard let position: AXValue = attribute(element, kAXPositionAttribute),
        let size: AXValue = attribute(element, kAXSizeAttribute) else { return nil }
  var point = CGPoint.zero, extent = CGSize.zero
  AXValueGetValue(position, .cgPoint, &point)
  AXValueGetValue(size, .cgSize, &extent)
  return CGRect(origin: point, size: extent)
}

/// The device's screen: the largest group of the window whose shape is the screen's, upright or on its side.
func screen() -> CGRect {
  let app = AXUIElementCreateApplication(hub.processIdentifier)
  guard let windows: [AXUIElement] = attribute(app, kAXWindowsAttribute), !windows.isEmpty else {
    fail("DeviceHub has no window (is the process allowed Accessibility in System Settings?)")
  }
  let ratio = portrait.0 / portrait.1
  var best: CGRect?
  func walk(_ element: AXUIElement, _ depth: Int) {
    if depth > 14 { return }
    if let rect = frame(of: element), rect.width > 50, rect.height > 50 {
      let shape = min(rect.width, rect.height) / max(rect.width, rect.height)
      if abs(shape - ratio) < 0.01, rect.width * rect.height > (best.map { $0.width * $0.height } ?? 0) {
        best = rect
      }
    }
    for child in (attribute(element, kAXChildrenAttribute) as [AXUIElement]?) ?? [] { walk(child, depth + 1) }
  }
  for window in windows { walk(window, 0) }
  guard let found = best else { fail("no device screen in DeviceHub's window: is a device shown?") }
  return found
}

let rect = screen()
let upright = rect.height >= rect.width
let pixels = upright ? portrait : (portrait.1, portrait.0)

func point(_ x: Double, _ y: Double) -> CGPoint {
  CGPoint(x: rect.minX + x / pixels.0 * rect.width, y: rect.minY + y / pixels.1 * rect.height)
}

func post(_ type: CGEventType, _ at: CGPoint, _ flags: CGEventFlags = []) {
  let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: at, mouseButton: .left)!
  event.flags = flags
  event.post(tap: .cghidEventTap)
}

func pause(_ ms: Double) { usleep(useconds_t(ms * 1000)) }

/// A finger from `from` to `to` over `ms`, held or not before it lifts.
func stroke(_ from: CGPoint, _ to: CGPoint, _ ms: Double, _ flags: CGEventFlags = []) {
  post(.mouseMoved, from, flags)
  pause(30)
  post(.leftMouseDown, from, flags)
  let steps = max(2, Int(ms / 8))
  for step in 1...steps {
    let t = Double(step) / Double(steps)
    post(.leftMouseDragged, CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t), flags)
    pause(ms / Double(steps))
  }
  post(.leftMouseUp, to, flags)
}

let args = Array(CommandLine.arguments.dropFirst())
func number(_ index: Int, _ fallback: Double? = nil) -> Double {
  if index < args.count, let value = Double(args[index]) { return value }
  if let fallback { return fallback }
  fail("missing argument \(index)")
}

guard let command = args.first else { fail("simtouch tap|long|drag|pinch|frame ...") }
if command == "frame" {
  print("\(rect.minX) \(rect.minY) \(rect.width) \(rect.height) \(upright ? "upright" : "on its side")")
  exit(0)
}
hub.activate()
pause(250)
switch command {
case "tap":
  let at = point(number(1), number(2))
  post(.mouseMoved, at); pause(30)
  post(.leftMouseDown, at); pause(60)
  post(.leftMouseUp, at)
case "long":
  let at = point(number(1), number(2))
  post(.mouseMoved, at); pause(30)
  post(.leftMouseDown, at); pause(number(3, 800))
  post(.leftMouseUp, at)
case "drag":
  stroke(point(number(1), number(2)), point(number(3), number(4)), number(5, 300))
case "pinch":
  stroke(point(number(1), number(2)), point(number(3), number(4)), number(5, 400), .maskAlternate)
default:
  fail("unknown command \(command)")
}
pause(50)
