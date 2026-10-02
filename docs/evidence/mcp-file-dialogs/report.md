# Request-scoped MCP file selection

KONTUR-229: https://plane.lurq.dev/pw-studio/browse/KONTUR-229/

Base: upstream master `2a7f079b6db9cb8d7afad2fcc36bcb00e6191515`. Tested product source: `bf0c571e89567e3b73d13cc4752657514112e51b`, tree `42b7f8e4a8366e7af0b9683b1d85a864ecebae79`. Final packaging changes only documentation/evidence. No dependency, version or native dialog behavior changes without app opt-in.

The public broker captures one actual window's weak identity and a UUID per normal picker operation. Config registration plus a separate `file_dialogs` scope enables list/respond tools. Responses use canonical window, operation and request identity. Paths stay data: app domain validation, file access, overwrite/atomic-save policy and document/session guards remain application responsibilities. Cancellation or permission loss never falls through to native UI. The documented handler explicitly branches to unchanged rfd only when the broker was never registered.

Retained run1 failed with E0451 at the moved WindowHandle construction; no tests ran. A parent-only visibility repair preserves SDK encapsulation. Source review also found a transient close-command marker; run2 includes the persistent accepted-close repair and retained-handle after-drain regression. Accepted close and vetoed request stay distinct. WindowHandle and existing window tests were extracted by responsibility so every changed Rust file is at most 586 lines.

Run2 on exact tested source passed MCP 16/16 (eight broker regressions and eight existing MCP tool tests), window 10/10, and default no-MCP library check. Commands/results are retained in `run1` and `run2`; original failures and warnings remain. The tests cover bounds, native fallback boundary, Unicode absolute paths, multiple window/request identities, invalid/stale/duplicate responses, future-drop cleanup, close/veto/replacement, revocation/shutdown and reentrant wake outside internal locks. Twenty-six distinct tests; no zero-test pass.

Environment: Windows, owning H worktree `.tmp/target`, `.tmp/cargo-home`, `.tmp/rust-temp`; the registry is reused via an H junction to the existing registry cache. Cargo locked/offline, -j1, jobs1, incremental0, dev/test debug0. Compiler lane explicitly released after exec10846 terminal0. No GPU/native application build, CUA, user file access, consumer source change, install, publishing/release or merge was performed.

Independent authenticated protocol QA remains pending. OS chooser visuals/automation and actual consumer import/export integration are separate acceptance; this SDK cannot intercept unmodified direct rfd calls. K215 physical checks are not claimed by this change. Upstream PR remains draft until independent review/integration.
