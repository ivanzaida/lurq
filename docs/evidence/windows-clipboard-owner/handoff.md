# Windows clipboard writer-owner repair

Source-only functional slice from published lurq 0.34.1 VCS `3bae59b101e269bb6f29e7954381d8f80c992c3c`. Branch `codex/windows-clipboard-owner`, isolated `H:/projects/pencil-web/.codex/worktrees/lurq-windows-clipboard-owner`. No applicable Lurq AGENTS/clean-code file exists in that base or the checked parent locations; Kontur's clean-code/core/testing workflow applies. No performance/cache/profiling/dependency-version work, SDK publication or merge is included.

The released Windows writer used arboard 3.6.1 → clipboard-win 5.4.1 → OpenClipboard(NULL), then Empty. Native07 Copy/readback succeeded but restoration later saw NULL HWND/PID and safely refused unknown-writer overwrite. Keeping arboard's Windows shim alive would not repair the missing HWND. This source replaces only Windows writes. Public copy/set/clear bool APIs and non-Windows/read behavior remain; existing macros, versions and lock bytes remain unchanged. The existing windows 0.62.2 dependency gains DataExchange/Memory API feature flags only.

`clipboard/windows.rs` holds one private message-only owner per calling thread, reused across writes, !Send/!Sync resources and no new thread/retry. The normal UI thread already has an event loop. Background callers get an owner limited to their caller thread's lifetime; no owner promise survives that thread's exit and no assertion of OS/job quiescence follows from PID/sequence or success. There is no delayed renderer or per-Copy destruction. Reentrant/TLS-teardown admission returns false rather than panicking or recreating foreign ownership.

`memory.rs` counts and prepares immediate, NUL-terminated movable UTF16 before opening/emptying. Allocation/lock/unlock failure cannot reach Empty. The raw GlobalUnlock and GlobalFree returns are preserved because windows 0.62.2's generated Result bindings do not model the documented successful-zero semantics. `native.rs` creates a message-only STATIC window using existing toolkit Windows/window idioms, refuses destroyed owner handles, checks Open/Empty/Set/Close without retries, and transfers HGLOBAL ownership only after Set succeeds. Close failure returns false even if Set already succeeded; post-Empty failure cannot promise atomic rollback. Only untransferred buffers are freed. Content-free cleanup warnings do not serialize text.

Relevant documented contracts: [OpenClipboard](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard), [EmptyClipboard](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-emptyclipboard), [SetClipboardData](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setclipboarddata), [GlobalUnlock](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-globalunlock), [GlobalFree](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-globalfree). NULL-owner protection in the V6 observer stays unchanged; this does not recover the lost original opaque RAM snapshot from native07.

Eight authored, UNEXECUTED Windows contracts:

- `clipboard::windows::tests::windows_clipboard_owner_survives_two_returns_and_transfers_are_not_freed`
- `clipboard::windows::tests::windows_clipboard_prepare_owner_or_open_failure_never_empties`
- `clipboard::windows::tests::windows_clipboard_empty_failure_closes_and_frees_without_transfer`
- `clipboard::windows::tests::windows_clipboard_transfer_failure_frees_and_closes_once`
- `clipboard::windows::tests::windows_clipboard_close_failure_is_not_success_or_double_free`
- `clipboard::windows::tests::windows_clipboard_unwind_closes_before_owner_and_frees_untransferred_data`
- `clipboard::windows::memory::tests::windows_clipboard_real_memory_encodes_surrogates_and_empty_nul`
- `clipboard::windows::native::tests::windows_clipboard_real_owner_is_process_owned_until_drop`

The first six exercise the actual coordinator with fake APIs. The last two use isolated synthetic HGLOBAL storage and a process-owned message-only window; they never inspect or mutate real clipboard contents. They do not substitute for actual Copy/owner/restore evidence.

Proposed finite validation after exact-head source QA and the root's sole compiler grant: `cargo test -p lurq --lib --locked --offline -j1 --no-default-features --features clipboard windows_clipboard -- --test-threads=1` (require all eight names), then `cargo check -p lurq --lib --locked --offline -j1 --no-default-features` (feature-off graph). Use the existing H CargoHome and warm SDK target, this checkout's H TEMP/TMP, exact head/tree/lock guards and fresh logs; no cold target or broad tests. No Cargo/metadata/build/native/Window/Clipboard calls have run during implementation.

Integration remains separate: adopt the reviewed local SDK source via guarded existing launcher override, prove actual package path/feature/artifact and unchanged allowed lock delta, build one default release with MCP on/DevTools off, then execute the corrected ordinary functional recipe on that exact source/exe under root grant. Required native gates include two ordinary Copy returns with owner HWND/PID attributable to the alive app thread, exact readback, V6 restoration before owned stop and unchanged foreign/NULL refusal, including early-entry cleanup. Do not attribute native07's final NULL write conclusively or claim restoration/product readiness until measured. No user Clipboard write is authorized by this source handoff.
