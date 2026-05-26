# Incremental Layout Plan

## Problem

`pass()` recomputes layout for the entire tree every frame, even when only a scroll offset or a single node's color changed.

## Goal

Only re-layout subtrees that actually need it. Skip layout entirely for frames where nothing structural changed.

## Design

### Dirty Sources

What can cause a node to need re-layout:
1. **Guard property change** — `Guard<T>::is_changed()` on color, border, radius, scrollbar_style. These are visual-only — don't affect layout sizes.
2. **Scroll offset change** — `ScrollState` changed. Affects child offset but NOT child sizes — only the scroll node's `LayoutResult.children[0].offset` changes.
3. **Size/constraint change** — viewport resize, frame constraint change. Requires full re-layout of affected subtree.
4. **Children added/removed** — structural change. Requires re-layout of parent.

### Key Insight

Most frame-to-frame changes are category 1 or 2 — they DON'T require layout recomputation at all:
- Color/border/radius changes → only affect quad generation, not layout
- Scroll offset → only affects the offset field in LayoutResult, not sizes

### Implementation

#### Phase 1: Cache LayoutResult on Node

Each node stores its last computed `LayoutResult`. On `pass()`:
- Walk the tree
- If node is structurally clean (no size/constraint changes), reuse cached result
- If only visual properties changed (Guard dirty), skip layout but regenerate quads
- If scroll offset changed, patch the offset in cached result

#### Phase 2: Dirty Flags on Node

Add a `layout_dirty: bool` flag to Node. Set it when:
- Frame constraints change
- Children are added/removed
- Text content changes (affects measurement)

Don't set it when:
- Color changes (visual only)
- Border changes (visual only — unless border placement is Outside which affects size)
- Scroll offset changes (offset only)

#### Phase 3: Incremental Compute

```
fn compute_incremental(node, constraints):
  if node.layout_dirty || constraints != node.last_constraints:
    result = full_layout(node, constraints)
    node.cache_result(result)
    node.clear_dirty()
  else:
    // patch scroll offsets if needed
    if node is ScrollModifier:
      patch_scroll_offset(node.cached_result, node.scroll_state)
    result = node.cached_result
  return result
```

#### What Changes

- `LayoutResult` cached per-node (or alongside node)
- `Constraints` cached per-node to detect constraint changes
- `pass()` checks dirty flags before computing
- Quad generation always runs (cheap) but could also be cached with visual dirty tracking

### Phases to Ship

1. Cache `LayoutResult` + `Constraints` on each node
2. Skip layout when nothing structural changed
3. Patch scroll offsets without re-layout
4. Only re-layout dirty subtrees
