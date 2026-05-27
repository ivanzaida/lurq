# lurq Demo App — Wireframe Design (Pencil)

## Overview

A tabbed showcase app that demonstrates every engine capability.
Sidebar navigation on the left, content area on the right.

Window size: **1200 x 800**

---

## Master Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  lurq engine demo                                          [theme]  │
│  GPU-accelerated UI toolkit                          [light][dark]  │
├──────────┬───────────────────────────────────────────────────────────┤
│          │                                                          │
│  NAV     │  CONTENT AREA                                            │
│          │                                                          │
│ ┌──────┐ │  (changes based on selected tab)                         │
│ │Layout│ │                                                          │
│ ├──────┤ │                                                          │
│ │Sizing│ │                                                          │
│ ├──────┤ │                                                          │
│ │Posit.│ │                                                          │
│ ├──────┤ │                                                          │
│ │Scroll│ │                                                          │
│ ├──────┤ │                                                          │
│ │Visual│ │                                                          │
│ ├──────┤ │                                                          │
│ │Text  │ │                                                          │
│ ├──────┤ │                                                          │
│ │Events│ │                                                          │
│ ├──────┤ │                                                          │
│ │React.│ │                                                          │
│ ├──────┤ │                                                          │
│ │Comps.│ │                                                          │
│ ├──────┤ │                                                          │
│ │Contxt│ │                                                          │
│ └──────┘ │                                                          │
│          │                                                          │
└──────────┴───────────────────────────────────────────────────────────┘
```

- Sidebar: 200px wide, dark surface `#1e293b`, vertical scroll if needed
- Nav items: white text, selected item gets `#3b82f6` left border + highlight
- Header: 56px tall, title left-aligned, theme toggle right-aligned
- Content: fills remaining space, `#0f172a` background, 24px padding

---

## Tab 1: Layout

**Purpose:** Row, Column, Stack containers + Justify + Alignment + Flex + Wrap

### Section 1.1 — Row vs Column

```
┌─ Row ──────────────────────────────┐  ┌─ Column ──────────┐
│ ┌──────┐ ┌──────┐ ┌──────┐        │  │ ┌──────────────┐   │
│ │  A   │ │  B   │ │  C   │        │  │ │      A       │   │
│ └──────┘ └──────┘ └──────┘        │  │ ├──────────────┤   │
└────────────────────────────────────┘  │ │      B       │   │
                                        │ ├──────────────┤   │
                                        │ │      C       │   │
                                        │ └──────────────┘   │
                                        └────────────────────┘
```

- 3 colored boxes in each (red, green, blue) with labels
- `.spacing(12.0)` visible between items

### Section 1.2 — Justify Modes

Show 6 mini-rows, each demonstrating a Justify variant:

```
Start:        [A][B][C]
End:                      [A][B][C]
Center:            [A][B][C]
SpaceBetween: [A]      [B]      [C]
SpaceAround:   [A]    [B]    [C]
SpaceEvenly:    [A]    [B]    [C]
```

- Label on the left, row visualization on the right
- Each row has a subtle border to show its bounds
- 3 small colored boxes (60x40) as children

### Section 1.3 — Cross-Axis Alignment

Show a tall row (120px) with 3 children of different heights:

```
align: Start         align: Center        align: End          align: Stretch
┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│┌──┐┌────┐┌─┐   │  │                │  │                │  │┌──┐┌────┐┌────┐│
││A ││ B  ││C│   │  │  ┌──┐┌────┐   │  │                │  ││  ││    ││    ││
│└──┘│    │└─┘   │  │  │A ││ B  │┌─┐│  │   ┌──┐┌────┐  │  ││  ││    ││    ││
│    │    │      │  │  └──┘│    ││C││  │   │A ││ B  │┌─┐│  ││  ││    ││    ││
│    └────┘      │  │      └────┘└─┘│  │   └──┘│    ││C││  ││  ││    ││    ││
│                │  │                │  │       └────┘└─┘│  │└──┘└────┘└────┘│
└────────────────┘  └────────────────┘  └────────────────┘  └────────────────┘
```

### Section 1.4 — Flex Distribution

```
┌─ flex(1) ─────────┬─ flex(2) ──────────────────────┬─ flex(1) ─────────┐
│                    │                                │                    │
│    1/4 width       │          2/4 width             │    1/4 width       │
│                    │                                │                    │
└────────────────────┴────────────────────────────────┴────────────────────┘
```

- Show ratio labels
- Different colors per section

### Section 1.5 — Stack (Overlay)

```
┌──────────────────────────────┐
│  ┌────────────────────────┐  │
│  │  Blue (200x200)        │  │
│  │  ┌─────────────────┐   │  │
│  │  │ Green (150x150)  │  │  │
│  │  │  ┌────────────┐  │  │  │
│  │  │  │ Red (100x100│  │  │  │
│  │  │  └────────────┘  │  │  │
│  │  └─────────────────┘   │  │
│  └────────────────────────┘  │
└──────────────────────────────┘
```

- 3 overlapping squares, each smaller, centered via stack_align(Center)

### Section 1.6 — Flex Wrap

```
┌──────────────────────────────────────┐
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐  │
│ │ 1  │ │ 2  │ │ 3  │ │ 4  │ │ 5  │  │
│ └────┘ └────┘ └────┘ └────┘ └────┘  │
│ ┌────┐ ┌────┐ ┌────┐                │
│ │ 6  │ │ 7  │ │ 8  │                │
│ └────┘ └────┘ └────┘                │
└──────────────────────────────────────┘
```

- Row with `.wrap()`, 8 items that overflow to next line

---

## Tab 2: Sizing & Spacing

### Section 2.1 — Dimension Types

```
Fixed (Px):    ┌─── 200px ───┐
               │              │
               └──────────────┘

Percentage:    ┌────────── 50% of parent ──────────┐
               │                                    │
               └────────────────────────────────────┘

Auto:          ┌─── fits content ───┐
               │ Hello World        │
               └────────────────────┘
```

### Section 2.2 — Padding Showcase

```
┌─ pad(20) ──────────────────────────┐
│                                     │
│    ┌────────────────────────┐      │
│    │  Uniform 20px padding  │      │
│    └────────────────────────┘      │
│                                     │
└─────────────────────────────────────┘

┌─ pad_xy(40, 10) ──────────────────────────────┐
│          ┌────────────────────────┐            │
│          │  40px horiz, 10px vert │            │
│          └────────────────────────┘            │
└────────────────────────────────────────────────┘
```

- Show dotted lines or color difference to indicate padding area

### Section 2.3 — Frame Constraints (min/max)

```
min_width: 200, max_width: 400

Short text:   ┌────── 200px (min) ──────┐
              │ Hi                       │
              └──────────────────────────┘

Long text:    ┌──────────────── 400px (max) ────────────────┐
              │ This is a really long text that hits the max │
              └──────────────────────────────────────────────┘
```

### Section 2.4 — Spacer

```
┌─────────┐                              ┌─────────┐
│  Left   │          (spacer)            │  Right  │
└─────────┘                              └─────────┘
```

- Row with item, spacer with flex(1), item — pushes items to edges

---

## Tab 3: Positioning

### Section 3.1 — Relative Offset

```
  Normal:          With offset(20, 10):
  ┌───┐            ┌ ─ ┐
  │ A │               ┌───┐
  └───┘            └ ─ ┘│ A │
  ┌───┐            ┌───┐└───┘
  │ B │            │ B │
  └───┘            └───┘
```

- Ghost outline shows original position, solid box shows offset position
- Sibling B unaffected (stays in place)

### Section 3.2 — Absolute Positioning (in Stack)

```
┌────────────────────────────────────┐
│ Stack (400x300)                    │
│                                    │
│   ┌────────────┐                   │
│   │ abs(20,20) │                   │
│   │ 120x80     │                   │
│   └────────────┘                   │
│                     ┌──────────┐   │
│                     │abs(200,  │   │
│                     │    150)  │   │
│                     └──────────┘   │
│                                    │
└────────────────────────────────────┘
```

- Two absolutely positioned colored boxes with coordinate labels
- Interactive: drag to reposition (using mouse_move + signal for x/y)

### Section 3.3 — Stack Alignment (9-point)

```
┌─────────────────────────────────────────────────┐
│ TopStart       TopCenter       TopEnd           │
│ ┌─────┐        ┌─────┐           ┌─────┐       │
│ │     │        │     │           │     │       │
│ └─────┘        └─────┘           └─────┘       │
│                                                  │
│ CenterStart     Center        CenterEnd         │
│ ┌─────┐        ┌─────┐           ┌─────┐       │
│ │     │        │     │           │     │       │
│ └─────┘        └─────┘           └─────┘       │
│                                                  │
│ BottomStart   BottomCenter    BottomEnd         │
│ ┌─────┐        ┌─────┐           ┌─────┐       │
│ │     │        │     │           │     │       │
│ └─────┘        └─────┘           └─────┘       │
└─────────────────────────────────────────────────┘
```

- 9 stacks, each showing a child aligned to one of the 9 positions

---

## Tab 4: Scroll Containers

### Section 4.1 — Vertical Scroll

```
┌─ Vertical Scroll ─────────────┐
│ ┌─────────────────────────┐ ▲ │
│ │ Item 1                  │ █ │
│ ├─────────────────────────┤ █ │
│ │ Item 2                  │ ░ │
│ ├─────────────────────────┤ ░ │
│ │ Item 3                  │ ░ │
│ ├─────────────────────────┤ ░ │
│ │ Item 4                  │ ░ │
│ └─────────────────────────┘ ▼ │
│   (20 items total)             │
└────────────────────────────────┘
```

### Section 4.2 — Horizontal Scroll

```
┌─ Horizontal Scroll ────────────────────────┐
│ ┌────────┬────────┬────────┬────────┬───── │
│ │ Card 1 │ Card 2 │ Card 3 │ Card 4 │ Car │
│ │        │        │        │        │     │
│ └────────┴────────┴────────┴────────┴───── │
│  ◄ ████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ ►   │
│   (10 cards total)                          │
└─────────────────────────────────────────────┘
```

### Section 4.3 — Both-Axis Scroll

```
┌─ 2D Scroll ────────────────────┐
│ ┌──────────────────────────┐ ▲ │
│ │                          │ █ │
│ │   (large content area    │ ░ │
│ │    e.g. 2000x2000 grid)  │ ░ │
│ │                          │ ░ │
│ └──────────────────────────┘ ▼ │
│  ◄ ████░░░░░░░░░░░░░░░░░ ►    │
└────────────────────────────────┘
```

- Grid of colored cells (20x20 grid of 100x100 cells)

### Section 4.4 — Scrollbar Styles

```
  Default (8px)     Thin (4px)      Wide (12px)     Hidden
  ┌────────┐ █     ┌──────────┐▏   ┌───────┐ ███   ┌──────────────┐
  │        │ █     │          │▏   │       │ ███   │              │
  │        │ ░     │          │░   │       │ ░░░   │  (no bar)    │
  │        │ ░     │          │░   │       │ ░░░   │              │
  └────────┘       └──────────┘    └───────┘       └──────────────┘
```

---

## Tab 5: Visual Styling

### Section 5.1 — Color Palette

```
┌────────────────────────────────────────────────────┐
│                                                     │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐       │
│  │#ef │ │#f97│ │#eab│ │#22c│ │#3b8│ │#8b5│       │
│  │4444│ │316│ │308│ │55e│ │2f6│ │cf6│       │
│  │red │ │org│ │yel│ │grn│ │blu│ │pur│       │
│  └────┘ └────┘ └────┘ └────┘ └────┘ └────┘       │
│                                                     │
│  Alpha gradient:                                    │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐               │
│  │100%│ │ 80%│ │ 60%│ │ 40%│ │ 20%│               │
│  └────┘ └────┘ └────┘ └────┘ └────┘               │
│                                                     │
└────────────────────────────────────────────────────┘
```

### Section 5.2 — Border Positions

```
  Inside              Center              Outside
  ┌──────────┐       ╔══════════╗       ┌────────────┐
  │ ┌──────┐ │       ║          ║       │            │
  │ │      │ │       ║          ║       │            │
  │ │      │ │       ║          ║       │            │
  │ └──────┘ │       ╚══════════╝       └────────────┘
  └──────────┘
  (shrinks content)  (half in/out)      (expands bounds)
```

- Each with a 4px border, same base size, visible difference

### Section 5.3 — Border Radius

```
  rounded(0)    rounded(8)    rounded(16)   rounded(50%)
  ┌──────────┐  ╭──────────╮  ╭──────────╮  ╭──────────╮
  │          │  │          │  │          │  │          │
  │          │  │          │  │          │  │          │
  └──────────┘  ╰──────────╯  ╰──────────╯  ╰──────────╯
   sharp         subtle        rounded       pill/circle
```

### Section 5.4 — Clipping (Overflow)

```
  Overflow::Visible           Overflow::Hidden (.clip())
  ┌──────────┐                ┌──────────┐
  │  ┌───────┼──┐             │  ┌───────│
  │  │ child │  │             │  │ child │
  │  └───────┼──┘             │  └───────│
  └──────────┘                └──────────┘
   child overflows             child clipped at boundary
```

---

## Tab 6: Typography

### Section 6.1 — Font Sizes

```
  48px  The quick brown fox
  32px  The quick brown fox
  24px  The quick brown fox
  20px  The quick brown fox
  16px  The quick brown fox (default)
  14px  The quick brown fox
  12px  The quick brown fox
```

### Section 6.2 — Font Weights

```
  Thin (100)     The quick brown fox
  Light (300)    The quick brown fox
  Normal (400)   The quick brown fox
  Medium (500)   The quick brown fox
  Bold (700)     The quick brown fox
  Black (900)    The quick brown fox
```

### Section 6.3 — Font Styles

```
  Normal:  The quick brown fox jumps over the lazy dog
  Italic:  The quick brown fox jumps over the lazy dog
```

### Section 6.4 — Text Colors

```
  ┌───────────────────────────────────────────────┐
  │  White text on dark    (#ffffff on #1e293b)   │
  │  Primary blue text     (#3b82f6)              │
  │  Success green text    (#22c55e)              │
  │  Warning amber text    (#f59e0b)              │
  │  Error red text        (#ef4444)              │
  │  Muted gray text       (#64748b)              │
  └───────────────────────────────────────────────┘
```

---

## Tab 7: Events & Interaction

### Section 7.1 — Click Events

```
  ┌─────────────────────┐    Event Log:
  │                     │    ┌──────────────────────────┐
  │   Click Me          │    │ > click at (142, 87)     │
  │                     │    │ > dblclick at (142, 87)  │
  └─────────────────────┘    │ > mouse_down Left        │
                             │ > mouse_up Left          │
  ┌─────────────────────┐    │                          │
  │  Double-Click Me    │    │                          │
  └─────────────────────┘    └──────────────────────────┘
```

- Button on left, scrollable event log on right
- Log shows event type, coordinates, button

### Section 7.2 — Hover & Interaction States

```
  Normal          Hovered         Active (pressed)
  ┌──────────┐   ┌──────────┐    ┌──────────┐
  │  Button  │   │  Button  │    │  Button  │
  │  #3b82f6 │   │  #60a5fa │    │  #2563eb │
  └──────────┘   └──────────┘    └──────────┘

  State display:
  ┌────────────────────────────────┐
  │ hovered: false                 │
  │ active:  false                 │
  │ focused: false                 │
  └────────────────────────────────┘
```

- Uses `.interactive(state)` to track and display states
- Color changes on hover/active via signal

### Section 7.3 — Mouse Tracking

```
  ┌──────────────────────────────────────┐
  │                                      │
  │         ● (mouse cursor)             │
  │                                      │
  │  x: 234  y: 156                      │
  │  entered: true                       │
  │                                      │
  └──────────────────────────────────────┘
```

- Tracks mouse position via on_mouse_move
- Shows enter/leave state
- Optionally: small dot follows cursor position

### Section 7.4 — Keyboard Input

```
  ┌──────────────────────────────────┐
  │  [Focus this area]               │
  │                                  │
  │  Last key: "ArrowRight"          │
  │  Code:     "ArrowRight"          │
  │  Shift: false  Ctrl: false       │
  │  Alt: false                      │
  │                                  │
  │  Key history:                    │
  │  a → b → Enter → ArrowUp        │
  └──────────────────────────────────┘
```

### Section 7.5 — Scroll Events

```
  ┌─ Scroll Area ──────────────────────┐
  │                                     │
  │   (scroll inside to see events)     │
  │                                     │
  │                                     │
  └─────────────────────────────────────┘

  Scroll phase: Idle
  Delta: x=0.0  y=0.0
  Position: x=0.0  y=0.0
```

---

## Tab 8: Reactivity

### Section 8.1 — Signals

```
  ┌─ Counter (Signal<i32>) ──────────────────┐
  │                                           │
  │   ┌─────┐   ╔═══════╗   ┌─────┐         │
  │   │  -  │   ║  42   ║   │  +  │         │
  │   └─────┘   ╚═══════╝   └─────┘         │
  │                                           │
  │   .set(0)  [Reset]                        │
  │   .update(|n| *n += 10)  [+10]           │
  └───────────────────────────────────────────┘
```

### Section 8.2 — Store + Lens

```
  ┌─ User Store ──────────────────────────────┐
  │                                            │
  │  Store { name: "Ada", age: 36 }           │
  │                                            │
  │  name lens: ┌──────────────┐ [Set "Grace"]│
  │             │ Ada          │              │
  │             └──────────────┘              │
  │  age lens:  ┌──────────────┐ [+1] [-1]   │
  │             │ 36           │              │
  │             └──────────────┘              │
  │                                            │
  │  Full state: { name: "Ada", age: 36 }     │
  └────────────────────────────────────────────┘
```

- Demonstrates field-level reactivity: changing name doesn't re-render age display

### Section 8.3 — Memo (Derived Values)

```
  ┌─ Memo Demo ────────────────────────────────┐
  │                                             │
  │  count: [−] 7 [+]                          │
  │                                             │
  │  doubled (memo):  14                        │
  │  is_even (memo):  false                     │
  │  label (memo):    "7 items"                │
  │                                             │
  │  Render count: 3  (memos skip when equal)  │
  └─────────────────────────────────────────────┘
```

### Section 8.4 — Effects & Watchers

```
  ┌─ Effect Log ─────────────────────────────────┐
  │                                               │
  │  count: [−] 5 [+]     name: [input field]   │
  │                                               │
  │  Effect log (auto-tracked):                  │
  │  ┌─────────────────────────────────────────┐ │
  │  │ effect ran: count=5, name="Ada"         │ │
  │  │ effect ran: count=4, name="Ada"         │ │
  │  │ effect ran: count=4, name="Grace"       │ │
  │  └─────────────────────────────────────────┘ │
  │                                               │
  │  Watcher log (explicit subscription):        │
  │  ┌─────────────────────────────────────────┐ │
  │  │ count changed to 5                      │ │
  │  │ count changed to 4                      │ │
  │  └─────────────────────────────────────────┘ │
  └───────────────────────────────────────────────┘
```

### Section 8.5 — Batch Updates

```
  ┌─ Batch Demo ───────────────────────────────┐
  │                                             │
  │  a: 1    b: 2    c: 3                      │
  │                                             │
  │  [Update All (no batch)]  renders: 3       │
  │  [Update All (batched)]   renders: 1       │
  │                                             │
  │  Render counter: 7                          │
  └─────────────────────────────────────────────┘
```

---

## Tab 9: Components

### Section 9.1 — Component with Props

```
  ┌─ Card Component (reusable) ──────────────────────────────┐
  │                                                           │
  │  ┌─ Card(title="Info") ───┐  ┌─ Card(title="Warn") ───┐ │
  │  │  Info                  │  │  Warning                │ │
  │  │  ──────────────────    │  │  ──────────────────     │ │
  │  │  Some info content     │  │  Be careful!           │ │
  │  └────────────────────────┘  └─────────────────────────┘ │
  │                                                           │
  │  ┌─ Card(title="Error", color="#ef4444") ───────────────┐│
  │  │  Error                                                ││
  │  │  ───────────────────────────────────                  ││
  │  │  Something went wrong                                 ││
  │  └───────────────────────────────────────────────────────┘│
  └───────────────────────────────────────────────────────────┘
```

### Section 9.2 — Slot Children

```
  ┌─ Panel with Slots ──────────────────────────┐
  │                                              │
  │  Panel {                                     │
  │  ┌─ title bar ─────────────────────────────┐ │
  │  │  My Panel                               │ │
  │  ├─────────────────────────────────────────┤ │
  │  │                                         │ │
  │  │  (slot children rendered here)          │ │
  │  │  → "Hello from parent!"                │ │
  │  │  → [Nested Button]                     │ │
  │  │                                         │ │
  │  └─────────────────────────────────────────┘ │
  │  }                                           │
  │  children.len() = 2                          │
  └──────────────────────────────────────────────┘
```

### Section 9.3 — Keyed List (for_each)

```
  ┌─ Dynamic List ───────────────────────────────┐
  │                                               │
  │  [Add Item]  [Shuffle]  [Remove Last]        │
  │                                               │
  │  ┌─ key="a" ──────────────────────────────┐  │
  │  │  Item A  (mount count: 1)        [×]   │  │
  │  ├─ key="b" ──────────────────────────────┤  │
  │  │  Item B  (mount count: 1)        [×]   │  │
  │  ├─ key="c" ──────────────────────────────┤  │
  │  │  Item C  (mount count: 1)        [×]   │  │
  │  └────────────────────────────────────────┘  │
  │                                               │
  │  Shuffling preserves component state!        │
  └───────────────────────────────────────────────┘
```

- Shows mount count to prove components survive reordering

### Section 9.4 — Error Boundary

```
  ┌─ Error Boundary Demo ────────────────────────┐
  │                                               │
  │  [Toggle Error]                               │
  │                                               │
  │  Normal state:          Error state:          │
  │  ┌──────────────────┐  ┌──────────────────┐  │
  │  │ ✓ Component OK   │  │ ⚠ Something went │  │
  │  │   count: 5       │  │   wrong (fallback)│  │
  │  └──────────────────┘  └──────────────────┘  │
  └───────────────────────────────────────────────┘
```

### Section 9.5 — Lifecycle

```
  ┌─ Lifecycle Demo ──────────────────────────────┐
  │                                                │
  │  [Mount Component]  [Unmount Component]       │
  │                                                │
  │  ┌─────────────────────────────────────────┐  │
  │  │  LifecycleDemo (mounted)                │  │
  │  └─────────────────────────────────────────┘  │
  │                                                │
  │  Lifecycle log:                                │
  │  ┌─────────────────────────────────────────┐  │
  │  │ > create() called                       │  │
  │  │ > render() called (count: 1)            │  │
  │  │ > on_mounted() called                   │  │
  │  │ > render() called (count: 2)            │  │
  │  └─────────────────────────────────────────┘  │
  └────────────────────────────────────────────────┘
```

---

## Tab 10: Context & Themes

### Section 10.1 — Theme Switching

```
  ┌─ Theme Demo ─────────────────────────────────┐
  │                                               │
  │  Current theme: Light  [Toggle Theme]        │
  │                                               │
  │  ┌─ Preview Card ──────────────────────────┐ │
  │  │                                          │ │
  │  │  Heading Text (theme.fonts.heading)      │ │
  │  │  Body text (theme.fonts.body)            │ │
  │  │  Mono text (theme.fonts.mono)            │ │
  │  │                                          │ │
  │  │  ┌──────────┐ ┌──────────┐              │ │
  │  │  │ Primary  │ │Secondary │              │ │
  │  │  │ Button   │ │ Button   │              │ │
  │  │  └──────────┘ └──────────┘              │ │
  │  │                                          │ │
  │  │  bg: theme.colors.background             │ │
  │  │  surface: theme.colors.surface           │ │
  │  │  text: theme.colors.text                 │ │
  │  └──────────────────────────────────────────┘ │
  └───────────────────────────────────────────────┘
```

- Whole preview card re-renders with theme colors on toggle

### Section 10.2 — Context Provide/Consume

```
  ┌─ Context Demo ───────────────────────────────┐
  │                                               │
  │  Provider: locale = [en-US ▾]                │
  │                                               │
  │  ┌─ Parent Component ──────────────────────┐ │
  │  │                                          │ │
  │  │  ┌─ Child A ──────────────────────────┐ │ │
  │  │  │  Consumed locale: "en-US"          │ │ │
  │  │  │  Greeting: "Hello!"               │ │ │
  │  │  └────────────────────────────────────┘ │ │
  │  │                                          │ │
  │  │  ┌─ Child B ──────────────────────────┐ │ │
  │  │  │  Consumed locale: "en-US"          │ │ │
  │  │  │  Date format: "MM/DD/YYYY"        │ │ │
  │  │  └────────────────────────────────────┘ │ │
  │  └──────────────────────────────────────────┘ │
  │                                               │
  │  Change to "ja-JP" → both children update    │
  └───────────────────────────────────────────────┘
```

### Section 10.3 — Reactive Context

```
  ┌─ Reactive Context ───────────────────────────┐
  │                                               │
  │  create_context:  ┌──────────┐               │
  │  user_role =      │ "admin"  │  [Set "user"] │
  │                   └──────────┘               │
  │                                               │
  │  consume_context in deep child:              │
  │  ┌──────────────────────────────────────────┐│
  │  │  role = "admin"                          ││
  │  │  (updates reactively when changed above) ││
  │  └──────────────────────────────────────────┘│
  └───────────────────────────────────────────────┘
```

---

## Tab 11: Node Refs & Debug

### Section 11.1 — Node Refs

```
  ┌─ Node Ref Demo ──────────────────────────────┐
  │                                               │
  │  ┌─ Target Element ─────────────────────────┐│
  │  │                                           ││
  │  │        (colored box)                      ││
  │  │                                           ││
  │  └───────────────────────────────────────────┘│
  │                                               │
  │  Measured rect:                               │
  │  x: 234.0   y: 142.0                        │
  │  width: 300.0   height: 80.0                 │
  │  relative_x: 24.0  relative_y: 86.0         │
  │  center: (384.0, 182.0)                      │
  │  attached: true                               │
  └───────────────────────────────────────────────┘
```

---

## Color Scheme

### Dark Theme (default)
| Role        | Hex       |
|-------------|-----------|
| Background  | `#0f172a` |
| Surface     | `#1e293b` |
| Border      | `#334155` |
| Text        | `#f8fafc` |
| Text Muted  | `#64748b` |
| Primary     | `#3b82f6` |
| Secondary   | `#8b5cf6` |
| Success     | `#22c55e` |
| Warning     | `#f59e0b` |
| Error       | `#ef4444` |
| Accent      | `#06b6d4` |

### Light Theme
| Role        | Hex       |
|-------------|-----------|
| Background  | `#f8fafc` |
| Surface     | `#ffffff` |
| Border      | `#e2e8f0` |
| Text        | `#0f172a` |
| Text Muted  | `#94a3b8` |
| Primary     | `#2563eb` |
| Secondary   | `#7c3aed` |
| Success     | `#16a34a` |
| Warning     | `#d97706` |
| Error       | `#dc2626` |
| Accent      | `#0891b2` |

---

## Pencil Implementation Notes

1. **Page structure**: Create one Pencil page per tab (11 pages total)
2. **Master template**: Header + sidebar should be a shared page background
3. **Reusable stencils**: Create stencils for repeated elements:
   - Nav item (normal / selected state)
   - Section header with label
   - Colored box (parameterized fill)
   - Button (normal / hover / active states)
   - Code label (monospace text in dark pill)
4. **Annotations**: Use Pencil's annotation layer to label features being demonstrated
5. **Interactivity arrows**: Use red arrows to indicate "click here" / "drag here" interactions
6. **State transitions**: Show before/after states side-by-side where relevant
