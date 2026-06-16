---
title: lurq
description: Documentation for the lurq Rust UI toolkit.
---

# lurq

`lurq` is a Rust UI toolkit with typed component builders, retained runtime state, reactive signals, GPU-backed rendering, and an in-app DevTools window.

The docs are organized around the questions that come up while building:

- [Getting Started](./getting-started/) shows the feature flags, demo command, and the smallest app shape.
- [Migration To v0.13](./migration-v0-13/) covers replacing legacy modal APIs, popup workarounds, and overlay host assertions.
- [Mental Model](./mental-model/) explains how `App`, `Tree`, components, elements, layout, input, and render passes fit together.
- [Components](./components/) covers props, mounting, keyed children, slots, and lifecycle.
- [Reactivity](./reactivity/) covers signals, stores, memos, effects, refs, contexts, and debug inspectability.
- [Layout](./layout/) covers constraints, row/column/stack, flex, scroll, absolute positioning, and clipping.
- [Theme](./theme/) covers strict palette, typography, radius, spacing, and form theme roles.
- [Animation And Transforms](./animation-transforms/) covers transitions, keyframes, easing, transform composition, transform animation, and transformed text.
- [Styling And Events](./styling-events/) covers visual modifiers, hover/active/focus styles, cursor state, handlers, text selection, inputs, clipboard behavior, and drag and drop.
- [Forms](./forms/) covers form handles, field binding, submission, `FormValues`, and the `Button` component.
- [Routing](./routing/) covers declarative routes, nested layouts, links, params, guards, and history navigation.
- [Futures And Timers](./futures-timers/) covers async data fetching, dependency-driven re-execution, future actions, tokio integration, timeouts, and intervals.
- [Persistent Storage](./persistent-storage/) covers typed values that survive app restarts.
- [Internationalization](./i18n/) covers translation resources, locale switching, interpolation, namespaces, and reactive re-rendering.
- [Modals](./modals/) covers render-flow modal overlays, targets, and controlling visibility.
- [App Runtime](./app-runtime/) covers `App`, `Tree`, render engine factories, windows, profiling, and frame flow.
- [DevTools](./devtools/) covers enabling the devtools feature, mounting the secondary window, inspecting components, and profiling renders.

For API lookup, keep [Ctx](./ctx/), [Typed Component API](./dsl/), and [Runtime And Retained Tree](./retained_nodes/) open.
