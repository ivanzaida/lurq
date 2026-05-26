# Layout Implementation Plan

## Done

- [x] `Constraints` — min/max width/height, tight/loose/unbounded constructors, constrain()
- [x] `Size` — width/height pair
- [x] `Offset` — x/y pair
- [x] `Alignment` — Start/Center/End/Stretch + cross_offset(), to_stack_alignment()
- [x] `StackAlignment` — 9-point 2D alignment + resolve_offset()
- [x] `FlexDirection` — Row/Column
- [x] `LayoutKind` — enum of all node kinds (Leaf, Text, Row, Column, Stack, modifiers)
- [x] `FrameConstraints` — optional width/height/min/max for frame modifier
- [x] `LayoutResult` / `ChildLayout` — layout output tree with sizes and offsets
- [x] `Node` — compositional model with container constructors and chainable modifiers
- [x] `Node::layout()` — constraint-based layout engine (flex, stack, padding, frame, offset, passthrough)
- [x] `Padding` — 4-side padding with constructors (all, symmetric, horizontal, vertical) + builder setters
- [x] `Dimension` — Auto/Px/Pct + to_px()
- [x] `Color` — RGBA with hex/hsl conversion

## TODO

### Text measurement
- [ ] Implement actual text measurement in `Node::layout_text()`
- [ ] Choose or integrate a font/text shaping library
- [ ] Text wrapping within width constraints
- [ ] Line height / font size as text node properties

### Scroll container
- [ ] Add `LayoutKind::Scroll` variant (horizontal, vertical, both)
- [ ] Scroll layout passes unbounded constraints on scroll axis
- [ ] Scroll viewport clips to parent constraints
- [ ] Scroll offset tracking

### Percentage resolution
- [ ] `Dimension::Pct` needs parent size context to resolve
- [ ] Pass resolved parent size through layout, or resolve in a pre-pass
- [ ] Update `Padding::to_px()` / frame constraints to handle percentages

### Border modifier
- [ ] Add `LayoutKind::BorderModifier { width: f32, color: Color }`
- [ ] Border affects layout (adds to size like padding)
- [ ] `Node::border()` modifier method

### Stretch alignment
- [ ] In flex layout, `Alignment::Stretch` should re-layout child with tight cross-axis constraint
- [ ] Currently treated as Start

### Overflow
- [ ] Add overflow handling to containers (visible/hidden/scroll)
- [ ] Clip children that exceed container bounds during render

### Intrinsic sizes
- [ ] Allow nodes to report preferred/minimum intrinsic sizes
- [ ] Used by flex layout when no explicit size or flex factor is set

### Flex enhancements
- [ ] `justify` (main-axis distribution): Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly
- [ ] Flex shrink factor
- [ ] Flex basis
- [ ] Flex wrap

### Rendering
- [ ] Traverse `LayoutResult` tree to produce draw commands
- [ ] Background color fill from `Node::color`
- [ ] Border rendering
- [ ] Text rendering
- [ ] Coordinate system: absolute positions from accumulated offsets

### Reactivity integration
- [ ] Connect `Signal` changes to re-layout
- [ ] Dirty tracking — only re-layout subtrees that changed
- [ ] Component trait integration with layout nodes
