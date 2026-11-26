---
title: DevTools
description: Enabling DevTools, inspecting components, selecting nodes, profiling renders, and signal debugging.
---

# DevTools

DevTools is feature-gated. Without the `devtools` Cargo feature, the app does not store extra prop trees, signal values, memo values, effect metadata, or profiler snapshots for inspection.

DevTools opens in a secondary window, which a page of a browser does not have yet: it is a desktop's for now. See [Web](../web/#not-done-yet).

## Enable It

Build with `lurq/devtools` and mount DevTools from the main tree. For the WGPU desktop example, also enable `winit` and `wgpu`. `devtools` enables `perf_profile`, so a DevTools build has frame timings, memory counters and [profiling sessions](../profiling/), and pays the profiler's idle cost on every pass (see [Profiler](#profiler)).

```rust
use lurq::app::{App, Tree};

let mut app = App::new();
let mut tree = Tree::new();

tree.set_render_engine_factory(|| Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new()));
tree.mount_root::<Root>(&mut app, RootProps);
tree.mount_devtools(&mut app);
```

The winit shell sees DevTools as a secondary window. It does not need inspector-specific code.

## What DevTools Shows

DevTools is drawn in a theme of its own: the design system's, light or dark with the system. What the inspected app does to its theme (another palette, recipes of its own, another renderer for a component) does not reach the tool that inspects it. Every tab is built with the design system's components from its board (`pencil/lurq-devtools.pen`), with the theme's own icons: DevTools loads no font of its own.

The DevTools window has these tabs:

| Tab | Shows |
| --- | --- |
| Components | Retained component/element tree, selected node details, props, signals, contexts, effects, shape/style rows, and optional overlay. Author-supplied `id`/`class` attributes appear first in each tree row and in the Attributes section, like browser devtools. |
| Profiler | A recording of every pass of the inspected windows between Start and Stop: a frame timeline with counter tracks (FPS, nodes, draw calls, memory, allocations) and input and UI-update markers, an opt-in filmstrip of frame screenshots, FPS, and each frame's phases, counters and full frame profile. |
| Storage | With the `persistent_storage` feature: what the app keeps in its [persistent storage](../persistent-storage/), a row for each key with the type of its value, the value and its bytes. |
| Signals | Every signal, memo and effect with its value, type, subscriber count and owner component, and for the one selected what owns it, what it notifies and its changes. |
| Context | Every context a component provides, with what it holds, how many components read it and its provider, and for the one selected its provider, its consumers and its changes. |

## Components

The tab is the tree of the inspected window and, beside it, the inspector of the node selected in the tree. The divider between them is dragged.

- **The tree** is a table: a row for each node that is shown, with what it is called and what is written of it (`key`, `id`, `class`, the text of a `Text`), whether it is a component or an element, and how many children it has. A press of a row selects it; a press of its chevron, a double press or Enter closes and opens it. The rows are one stop of Tab: Up and Down move through them, Home and End to the first and the last, PageUp and PageDown by what is shown, Space selects. Thousands of nodes cost about a screen of them: the rows in view are built.
- **Pick** makes the next press in the inspected window select the node under it: what the node is in is opened, and its row is brought into view.
- **The inspector's header** names the node, its kind and its `NodeId`. **Overlay** outlines the selected node in the inspected window; **Screenshot** saves a picture of it.
- **Under the header**: *Rendered by*, the nearest component the node is in, whose render built it; for a component, *Render count*, how many times it has rendered since it was mounted, and *Last render*, how long its last render took with the renders of what it mounted in it; and *Children*.
- **The lists** of the node, a tab each with how many it holds: *Props* (the struct and its fields, with the fields they hold), *Context* (what it provides and uses), *Reactive* (its signals, memos and effects, each with its id, value and type) and *Attributes*.
- **Layout** draws the node as a box in a box, its padding around its content, with where it is in the window and in what holds it, its size, its content's, whether it overflows, and whether it is hovered, active or focused. **Style** has a row for each thing said of the node's style, a color with its swatch, and the style of each state under a heading of its own (*On hover*, *On press*, *On focus*, *On keyboard focus*).

## Component Search

In the tree, the arrows up and down go from row to row, Left closes the selected row and Right opens it, as a press of its chevron does, and so do Enter and a double press.

Ctrl+F (Cmd+F on macOS) goes to the search box from anywhere in the tab. The count under the tree says how many nodes the search found; the rows shown only because what it found is in them are drawn muted, and are not counted.

The Components tab search box accepts plain text and a small query language.

Plain terms search component/node names, keys, and text values:

```text
button
settings modal
"Save changes"
```

Multiple terms are combined with AND. A row matches only when every positive term matches and no negated term matches. Matching descendants keep their ancestors visible, and collapsed branches are expanded while the search is active.

Field prefixes narrow a term to one part of the node snapshot:

| Query | Matches |
| --- | --- |
| `tag:Button` | Full or short component/node tag. |
| `name:Button` | Alias for `tag:`. |
| `component:Button` | Alias for `tag:`. |
| `key:submit` | Component key. |
| `text:"Save changes"` | Text value. |
| `value:"Save changes"` | Alias for `text:`. |
| `kind:component` | Node kind, `component` or `element`. |
| `state:focused` | Runtime state, `focused`, `hovered`, or `active`. |

Prefix a term with `-` to exclude nodes:

```text
tag:Button -key:secondary
kind:element state:focused
text:"Save changes" -key:secondary
```

Quotes keep spaces inside one term. Backslash escapes work inside quoted terms:

```text
text:"Delete \"draft\""
```

## Inspectable Props

When `devtools` is enabled, component props must implement `DevtoolsInspectable`.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct InfoCardProps {
  title: &'static str,
  body: &'static str,
  accent: &'static str,
  metadata: Metadata,
}

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Metadata {
  count: i32,
  enabled: bool,
}
```

Nested structs that also derive `DevtoolsInspectable` are shown recursively. Scalar values are shown with type and value. Mark sensitive or noisy fields with `#[devtools_ignore]`.

A field whose type does not write itself for DevTools (a handle of a library, a closure) does not stop the derive: it is shown with its type and nothing of its value. The numbers, `bool`, `char`, `String`, `&str`, `Duration`, `PathBuf`, and `Option`, `Result`, `Box`, `Arc` and `Rc` of what is inspectable write themselves; a `Vec`, an array, a `VecDeque`, a map or a set says how many it holds. A field of a type parameter is read only where the parameter is bound: `struct Page<T: DevtoolsInspectable>`.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Credentials {
  user: String,
  #[devtools_ignore]
  token: String,
}
```

Enums show their current variant.

Masked text inputs expose their displayed mask and `masked=true` in node text and shape snapshots, and [sensitive text](../styling-events/#sensitive-text) (`Text::sensitive()`) shows as `•••` with `sensitive=true`; a node screenshot covers it with a grey bar. This does not redact arbitrary application props, signals, custom annotations, or logs: hold secret values in `lurq::core::Sensitive<T>`, which DevTools shows as `•••` in signal values, history and props, or use `#[devtools_ignore]` for sensitive fields you expose to inspection.

## Inspectable Signals And Memos

With `devtools`, signal and memo values must be inspectable too:

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct CounterState {
  count: i32,
}

let state = ctx.signal(CounterState { count: 0 });
let doubled = ctx.memo({
  let state = state.clone();
  move || state.get().count * 2
});
```

The engine keeps, for each signal and memo: its id, its type, its value as it is written, the component that owns it, how many subscribers it has, and its last 64 changes, each with the moment it happened (by the clock and the zone of the system; in UTC where the engine cannot read the zone), the value before and the value after.

## Inspectable Contexts

What a component provides needs no trait: `ctx.provide`, `ctx.provide_if_changed` and `ctx.create_context` take any type, with `devtools` and without. DevTools shows what a context holds where its type derives `DevtoolsInspectable`:

```rust
#[derive(Clone, Hash, lurq::DevtoolsInspectable)]
struct Locale {
  locale: &'static str,
  greeting: &'static str,
}

let locale = ctx.create_context(Locale { locale: "en-US", greeting: "Hello!" });
```

The derive registers an inspector for the type, and DevTools finds it by the type of the value it is given. A type that registered none is listed with its value unknown, `Navigator { unknown }`, and none of its changes are shown: there is nothing to tell two values of it apart by. A type whose `DevtoolsInspectable` is written by hand registers itself with `lurq::register_inspectable!(Depth);`, and so does a type with type parameters for each form of it that is provided, `lurq::register_inspectable!(Page<Settings>);`: a derive cannot register a type for every parameter it could have.

The engine's own contexts show their value where it is worth reading (`OutletDepth(0)`), and are unknown where it is not (`Navigator`). The values an app provides as they are, a number, a `String`, a `bool`, are read too.

The engine keeps, for each context a component provides: its type, what it holds as the type writes it, and its last 64 changes. A reactive context changes when it is set to another value; a plain one when the component provides another value than it did.

## Signals

The tab lists every signal, memo and effect of the inspected window, the signals first, then the memos, then the effects, each in the order they were made. **Filter** shows the rows that have every word typed, in their number, value, type or owner; Ctrl+F (Cmd+F on macOS) goes to it. The count under the list says how many of each are shown.

Beside the list is the row selected in it: what it is, the component that owns it, its value, and how many subscribers it notifies, over its **change history**, the newest change first.

The history is the engine's own record of the changes, not a reading taken by DevTools, so a value that changed twice between two frames shows both changes. **Recording** says which of them the tab shows:

- It is on when DevTools opens: every change the engine still has is shown.
- Turned off, it leaves out what changes from then on. Turned on again, it shows what changes after that; what changed in between stays out.
- **Clear** forgets what was shown and goes on as it was: recording, it shows what changes from now on.

## Context

The tab lists the contexts the components of the inspected window provide, in the order of its tree: a reactive one with a bolt, a plain one with the mark of sharing. Each row has what the context holds on one line, how many components read it, and the component that provides it. The same type provided by two components is two rows: a component reads the nearest one above it. **Filter** shows the rows that have every word typed, in their type, value or provider; Ctrl+F (Cmd+F on macOS) goes to it.

Beside the list is the row selected in it: its type with its whole path, the component that provides it, what it holds, and how many consumers it notifies, over its **change history**. A change has the value before and the value after, each written out a line for each field. **Recording** and **Clear** do what they do in the Signals tab; each tab has its own.

## Storage

With the `persistent_storage` feature the window has a Storage tab: the values of the app's [persistent storage](../persistent-storage/) as they are now, a row for each key in the order of the keys, with the type of the value, the value as it is written and how many bytes it takes, and under them how many keys there are and how many bytes in all. A stored struct has a row for each of its fields. A type of the app's is called by its last name, and the pointer resting on it shows the whole path.

## Overlay And Pick Mode

The Components tab can draw an overlay over the selected inspected node. The overlay is controlled from DevTools, while the main tree owns the actual overlay drawing.

Pick mode reverses selection:

1. Click the pick button in DevTools.
2. Click an item in the inspected app.
3. The matching component tree row is selected in DevTools.
4. The tree panel expands ancestors and scrolls the selected row into view.

The main tree exposes this through internal tree methods such as debug overlay selection and node picking; the shell only routes secondary-window pick requests.

## Profiler

`devtools` turns on frame profiling (`perf_profile`) at compile time:

```toml
lurq = { version = "0.76.0", features = ["winit", "wgpu", "devtools"] }
```

The Profiler tab records on the inspected tree's [profiling session service](../profiling/), the one `Tree::profiling_handle()` returns:

The tab's controls are in a toolbar of its own under the tab bar: what starts and stops a recording, what a recording takes, and at its end the files and **Clear**, which go onto a row of their own where the window is too narrow for all of it. The last error is written in it, cut at its end where it is long.

1. **Start recording** starts a [recording session](../profiling/#two-session-modes) on it. Nothing is recorded before you press it, and the tab copies nothing while DevTools renders.
2. While it runs, the toolbar shows beside **Stop recording** how long it has run, the memory its samples hold and how many it took, reread four times a second. A recording keeps every sample; at its 1 GiB ceiling it stops taking more and says so.
3. **Stop recording** ends the session. The recording holds every pass of the inspected windows (the main window and its other secondary windows) that began after Start and finished before Stop, each as its own frame: none is skipped, merged with an identical one, or capped to a fixed count. Passes already running at Start are left out and counted.

DevTools' own window is not recorded. Turn on **Include DevTools** before Start to record it too. Turn on **Components** before Start for a [detailed recording](../profiling/#detailed-recordings): each component render and `after_layout` hook becomes a span named by the component's type, each pass counts the nodes it laid out again per component, and each UI update keeps what made its components render. **Clear** forgets the recording, and ends one still running.

Turn on **Screenshots** before Start to record [screenshots of presented frames](../profiling/#frame-screenshots) as well: at most four a second, each fitted into 320 x 200 pixels, with [sensitive text](../styling-events/#sensitive-text) covered by a grey bar as in a node screenshot. It is off by default, and then no frame is read back. Each screenshot copies its frame on the GPU; the readback and scaling run off the frame's thread, but the frame that made the copy is slower, so it is marked in the timeline. Screenshots count toward the 1 GiB ceiling (about 19 minutes of a 16:9 window rendering at the defaults, less the samples' share), and the status line counts them.

With a recording shown, **Save** writes it to a lurq recording file and **Export for Firefox Profiler** to a profile that [profiler.firefox.com](https://profiler.firefox.com) opens with its call tree, flame graph and frame/input markers. **Load** opens a file Save wrote in place of the recording, with the same timeline, flame chart and tables; a file it cannot read leaves the recording shown and says why in the toolbar, and while a recording runs Load is disabled. Each asks for its file in the system's file dialog, one dialog at a time; iOS has none, so there the tab has no file buttons and an app can call the [recording file functions](../profiling/#recording-files) itself.

After Stop the tab shows:

- **Recording**: the whole recording in equal slices, each drawn as its longest pass. Press and drag over it, or shift-click to extend from the last press, to select a time range; **Whole recording** selects all of it. A recording of more than 600 passes opens on its last 600.
- **Frames**: the selected range on a time axis. Each pass is a bar at its start time, as tall as its duration, against a line at the 16.7 ms frame budget (one 60 Hz refresh): in the theme's warning color over the budget and its destructive color over twice it, the selected pass in the foreground color, faded when the pass presented no frame. A bar is as wide as the passes of the range leave it, from nine pixels down to two. The header counts the passes over budget and the frames that missed a vsync. Each row says at its start what it is and what it comes to: the selected pass's value, or the range's highest. With screenshots, a **filmstrip** above the bars splits the range into 16 equal parts on the same time axis, each showing the first screenshot whose frame's pass starts in it; hovering a thumbnail previews it in the sidebar, a click selects its pass. The rows below the bars mark, at the same times:
  - **missed vsync**, in the destructive color: the frames that came a refresh or more late inside a run of frames (an animation, a drag, a scroll). A frame after the app idled is never marked: the renderer draws on demand, so a gap is not a dropped frame. See [present timing](../profiling/#input-latency-presented-frames-and-pass-reasons) for how a run is told apart and what the backends report (CPU present times only).
  - **captured**, with screenshots: the frames that copied a screenshot. Their render timings include the copy; do not read them as the app's.
  - **gpu**: a bar at each pass that has [GPU time](../profiling/#coverage-and-units) (its UI and Canvas passes, measured with wgpu timestamp queries), scaled to one frame budget or its longest bar. When none was measured, it says why (the adapter has no timestamp queries, the native DX12 renderer, or no result arrived).
  - **Counter tracks**: one bar per pass at its start, scaled to the range's highest value, for **fps** (from the interval since its window's previous presented frame, up to 250 ms), **nodes**, **draw calls** (of a presented frame's render list), **memory** (the runtime estimate taken at the end of the pass) and **allocs** (allocations per pass). A click on a counter bar selects its pass, as on the frame bars. The allocation track says **unavailable** unless the app installs [`CountingAllocator`](../profiling/#counting-allocations).
  - **input**: where the shell received each input event, in the warning color where its input-to-present latency (from receiving the event to the present of the first frame after it) is over 50 ms and in the destructive color over 100 ms. The row gives the count and the longest latency.
  - **ui update**: where root rebuilds and subtree refreshes began; those that led to the selected pass stand out.

  A range with more than 600 passes asks for a narrower one instead of drawing them.
- **FPS**: from the intervals between frames presented in the range, for the window that presented most. Intervals over 250 ms are the app idling and are left out, so FPS is blank (`–`) when the app was idle, and does not drop because the app paused between bursts.
- The **sidebar** for the selected bar (the slowest pass of the range until you click one): which pass it is, whether it was over the budget, and when it began; with screenshots, a preview of the thumbnail under the pointer or else of what the pass's window showed (its frame's screenshot, or the window's last one before it); then its duration against the budget, its GPU time (with why a presented frame has none), whether its frame was presented and whether it missed a vsync (and how many, where more than one), its window, whether it copied a screenshot, and why the pass ran (its reasons, such as `redraw_requested` or `timeline_active`). GPU time arrives one or more frames after its pass, so the last frames before Stop may have none. Under that, four lists:
  - **Phases**: the pass's recorded spans, nested by depth, each with a bar of its share of the pass and its time, in the color of its phase. In a detailed recording they include the components by name.
  - **Causes**, in a detailed recording: what made each component render in the UI updates that led to the pass (those of its window since its previous pass, during input handling or inside the pass): the component, what changed (`signal`, `props`, `memo`, ...) and the type of its value, and for a memo what made it recompute, on a `via` row under it. They are taken from the notifications as they happened.
  - **Input**: the inputs this frame presented, each with its interaction id, queue delay, dispatch time and input-to-present latency.
  - **Counters**: everything its sample holds, group by group: the pass timings and flags, its GPU time split into UI and Canvas, its present timing (interval, refresh interval, whether it continued a run), its counters (nodes, memory, cache sizes and allocations), Canvas text work, the nodes it laid out again per component and reason (in a detailed recording), and its frame profile's timings, counts, render (draw calls included), Canvas, Canvas asset upload, text and memory figures.

In a window too low for the frames and the analysis, the frames scroll and the analysis keeps a height it can be read at; in a narrow one, what is said in a title gives way to its numbers and its buttons.

Under the timeline, three views analyse the passes that start in the selected range (**Range**), or the selected pass alone (**Selected frame**):

- **Flame chart**: every span to scale on a time axis. The passes' toolkit phases are on the UI track, each pass a root with its phases nested below it; [application scopes](../profiling/#synchronous-application-scopes) are on a track per lane (UI lane, worker lane), nested by their parent. The mouse wheel zooms around the pointer, a horizontal wheel or a drag pans, **Reset zoom** shows the whole range again. Hovering a span shows its name, total and self time and the pass it belongs to; clicking selects it (and its pass in the timeline), the line under the chart says what it took, what it is in and what it is (the pass, a phase, a component, a scope), and **Zoom to selection** fits the view to it. A span has the color of its phase along its top (the tree's phases, layout, what is made of the layout to draw, the drawing, and the app's own scopes, in the theme's five colors of a chart), and a component that of the phase it is in. The name of the selected span is picked out in the two tables and among the phases of the selected pass, and a row selected in a table is picked out the same way. Spans narrower than 3 px are merged into muted blocks until you zoom in, so a range of thousands of passes draws a few hundred elements. The wheel is the chart's, so the chart is in nothing that scrolls: where it has more rows than room, a drag moves it up and down as it moves it along its time. A recording keeps its last 128 application scopes; the bar says how many earlier ones it no longer holds.
- **Call tree**: the spans of one track (Frames, or an application lane) merged by their path from the root, top-down, with total time, self time (total less its children), count, and both times as a percent of the track's total. For Frames that total is the pass time of the passes analysed. Rows open to the passes' phases; a row's chevron, Enter or a double press opens or closes it, and a press on a column title sorts by it, the longest and the most first, and again turns it. It is the design system's table, built for the rows in view, so it lists every row.
- **Bottom-up**: a row per span name with its self time and its total time, and under it the names that called it, outward. A phase inside another of its name counts its total once. The rows' self times add up to the same total as the call tree.

In a detailed recording, components show in all three views by their type name: under `ui_rebuild` each component that rendered (with the components it mounted below it), and under `component_after_layout` each component's hook (with its descendants' hooks below it), so the call tree merges the renders of one component type on one path.

Phases recorded on another thread can overlap: each span is trimmed to its parent and to the end of the sibling before it, so no time counts twice. The aggregation is `lurq::app::profiler::SpanForest` (`forest_of`, `call_tree`, `bottom_up`), plain data that runs on any `ProfileReport`.

Measured on a synthetic recording of 2000 passes with 62,000 spans and the analysis over all of it (`a_recording_of_2000_frames_stays_interactive`, Windows, headless pass of the whole tab, no GPU): in a release build, opening it takes about 40 ms, a hover about 1.5 ms (at most 3 ms), a zoom step 1 ms (at most 2 ms), a pan step 0.7 ms, and the first call tree or bottom-up table 6–9 ms, then about 3 ms; a debug build opens it in about 60 ms and takes about 9, 7 and 5 ms per hover, zoom and pan step.

The recording holds only phase names, times and counts, and in a detailed recording the static type names of components and of the signals, memos, stores and contexts that made them render: type names, never values. No text, prop or signal values, keys, element or signal ids, input values, pointer positions or paths. With **Screenshots** on it also holds the pixels of the frames it took, as the window showed them except for sensitive text, which is covered. The Signals tab shows signal values and their changes, which the engine keeps apart from any recording.

`Tree::last_profile()` returns the latest low-level frame profile for custom tooling. In a build with neither `devtools` nor `perf_profile` it holds default values.

Profiling costs a DevTools build something on every pass, even with no session open: the pass timers run and each coarse phase is published so a session started during a slow pass can see it. Measured with `cargo bench -p lurq --bench profiler_idle` (Windows, release, headless pass): a pass over 40 unchanged rows takes about 0.31–0.34 µs without `perf_profile` and 0.52–0.61 µs with it and no session, so about 0.2–0.35 µs more per pass. While a session is recording, each captured pass also gathers its spans and counters: about 3.6 µs more on that scene, and about 19 µs more on a pass that rebuilds a component and lays out the rows again (31 µs without `perf_profile`, 34 µs with it and no session, 53 µs with a session). See [profiling](../profiling/#overhead) for details.

The same service also gives independent bounded captures through `Tree::profiling_handle()` and MCP's `lurq_profile_start`/`lurq_profile_read`/`lurq_profile_end`, which run alongside a DevTools recording. GPU time is measured on the wgpu renderer only, while some session is active; the native DX12 renderer reports it unavailable.

## Perf Overlay

The perf overlay is separate from DevTools but feeds data that DevTools can show.

```rust
tree.draw_perf_overlay();
```

The overlay samples FPS once per second. Enable `perf_profile` to include frame stage timings such as layout, resolve, glyph, acquire, upload, encode, submit, and present.

## Common Issues

If a prop type fails to compile only with `devtools`, derive or implement `DevtoolsInspectable`.

If a context shows as `Type { unknown }`, derive `DevtoolsInspectable` for its type, or register the type with `lurq::register_inspectable!` where its `DevtoolsInspectable` is written by hand or it has type parameters.

If signal values show as unknown, check that the signal type implements `DevtoolsInspectable` and that the value is stored through `ctx.signal`, `ctx.store`, or `ctx.memo`.

If DevTools does not repaint after picking while the app window is focused, check the secondary window redraw path in the shell. Secondary windows must request/present frames even when they are not the active OS window.
