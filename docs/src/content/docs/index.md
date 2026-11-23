---
title: lurq
description: Documentation for the lurq Rust UI toolkit.
---

# lurq

`lurq` is a Rust UI toolkit with typed component builders, retained runtime state, reactive signals, GPU-backed rendering, and an in-app DevTools window.

The guides describe **0.71.0**. Migration guides and dated performance reports retain their historical scope.

The docs are organized around the questions that come up while building:

- [Getting Started](./getting-started/) shows the feature flags, the gallery's command, and the smallest app shape.
- [Design System](./design-system/) covers the components of `lurq::ui` and what a theme styles them with: recipes, renderers, a bare theme, and `Button`.
- [Migration To v0.67](./migration-v0-67/) covers moving an app from 0.66: what 0.63 deprecated is removed (the palette roles of 0.61, `FormTheme`, `dark()` and `for_appearance(..)` of the phone's pieces, `ActivityIndicator`, `WinitWindow::on_frame`), and the types that gained a field for glass.
- [Migration To v0.63](./migration-v0-63/) covers moving an app from 0.62: the design system's theme becoming the default and how to keep the 0.62 look, the renamed palette roles, Markdown on the theme's roles, and what changed in layout, scrolling, the title bar and presence.
- [Migration To v0.13](./migration-v0-13/) covers replacing legacy modal APIs, popup workarounds, and overlay host assertions.
- [Mental Model](./mental-model/) explains how `App`, `Tree`, components, elements, layout, input, and render passes fit together.
- [Components](./components/) covers props, mounting, keyed children, slots, and lifecycle.
- [Reactivity](./reactivity/) covers signals, stores, memos, effects, refs, contexts, and debug inspectability.
- [Layout](./layout/) covers constraints, row/column/stack, flex, scroll, absolute positioning, and clipping.
- [Theme](./theme/) covers palette, typography, radius, spacing, border size, shadow, and form theme roles, including application-defined extras.
- [Animation And Transforms](./animation-transforms/) covers transitions, keyframes, easing, transform composition, transform animation, and transformed text.
- [Styling And Events](./styling-events/) covers visual modifiers, hover/active/focus styles, cursor state, handlers, text selection, inputs, clipboard behavior and paste events, and drag and drop, in the app and of files from the OS.
- [Focus And Keyboard Navigation](./focus-navigation/) covers focusable elements, `tab_index`, Tab scopes, modal focus traps, focused styles, and testing focus headlessly.
- [Forms](./forms/) covers form handles, field binding, submission, `FormValues`, and the `Button` component.
- [Routing](./routing/) covers declarative routes, nested layouts, links, params, guards, and history navigation.
- [Futures And Timers](./futures-timers/) covers async data fetching, dependency-driven re-execution, streams, future actions, tokio integration, timeouts, and intervals.
- [Queries](./queries/) covers shared async reads, request deduplication, caching, and typed invalidation across components and trees.
- [Persistent Storage](./persistent-storage/) covers typed values that survive app restarts.
- [Internationalization](./i18n/) covers translation resources, locale switching, interpolation, namespaces, and reactive re-rendering.
- [Modals](./modals/) covers render-flow modal overlays, targets, controlling visibility, and how popups handle a press outside them.
- [App Runtime](./app-runtime/) covers `App`, `Tree`, render engine factories, windows, profiling, frame flow, and browser-style element lookup (`get_element_by_id`, typed interaction handles).
- [Window lifecycle and native menus](./window-lifecycle-menus/) covers close requests, confirmation dialogs, and macOS menus.
- [Web](./web/) covers running an app as a page of a browser through WebGPU or WebGL2: what `main` waits for, the fonts, the canvas, and what a page does differently.
- [DevTools](./devtools/) covers enabling the devtools feature, mounting the secondary window, inspecting components, and profiling renders.
- [MCP Server](./mcp/) covers agent inspection, input, permissions, and custom tools.
- [Resources And Media](./resources-media/) covers fonts, raw and decoded images, SVGs, resources, and Markdown.
- [Canvas 2D](./canvas/) covers persistent drawing, gradients, shadows, blur, blend modes, layers, and GPU limits.
- [Testing](./testing/) covers headless tests, feature-specific checks, and benchmarks.
- [Text Pipeline Optimization](./text-pipeline-optimization/) summarizes the current implementation and links to dated benchmark reports.

For API lookup, keep [Ctx](./ctx/), [Typed Component API](./dsl/), and [Runtime And Retained Tree](./retained_nodes/) open.
