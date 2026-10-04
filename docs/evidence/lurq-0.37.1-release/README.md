# lurq 0.37.1 release validation

Branch `codex/lurq-release-0.37.1`, based on `origin/master` `31b800e` (0.37.0). Integration evidence for the combined tree, not a repeat of PR #40's independent QA.

| Commit | What | Tree |
| --- | --- | --- |
| `49c0cba` | merge of `codex/k236-canvas-asset-cache` (PR #40, head `957d07d`) into 0.37.0 | `7d27258a094f4e456f2aa9362066f23f368cc4b7` |
| `f068339` | `chore(release): 0.37.1` | `7c4a9d7cd478f8deb0f33a1d428123d08b6fa9ff` |

Every log in `runs/` names `f068339` and its tree, with no dirty source file; the clippy base log names `31b800e`.

## Merge

- `git merge-tree --write-tree origin/master origin/codex/k236-canvas-asset-cache` reported no conflict and wrote tree `7d27258`. The merge commit has the same tree, so no conflict needed resolving and nothing was edited by hand.
- The only file both sides changed is `docs/src/content/docs/canvas.md`: 0.37.0 bumped the version line, and PR #40 rewrote the limits paragraph. Git merged the two hunks automatically.
- The merge base is `1e13216` (0.36.0). PR #40 adds 4 commits: `b6291f7`, `da78447`, `3693f75` and `957d07d`.

## Release commit

`f068339` changes the same 17 files as the 0.37.0 release commit `31b800e`:

- the version in `crates/lurq/Cargo.toml`;
- the `lurq` entry in `Cargo.lock`, and no other line;
- the version strings in `README.md`, `docs/README.md` and 12 guides under `docs/src/content/docs/`;
- the `CHANGELOG.md` entry.

## Host and settings

- Host: Windows 11 Pro 10.0.26200, NVIDIA GeForce RTX 5080, rustc and cargo 1.95.0.
- `CARGO_TARGET_DIR` and `TMP`/`TEMP` were under the worktree's `.tmp/`. Builds used `CARGO_BUILD_JOBS=4` and `--locked --offline`. The publish dry-run could not use `--offline`; see below.
- A process check (cargo, rustc, link, lld-link, kontur-desktop) ran as its own step before every Cargo invocation. Cargo started only when it showed none.
- Other agents built and ran Kontur between these runs. The probe started at 17:38:16, when the host was idle, and a check right after it finished (17:38:30) showed nothing running.

## Validation: combined tree compared with QA's PR #40 head

| Command (`cargo … --locked --offline`) | Combined `f068339` | QA `957d07d` | Log |
| --- | --- | --- | --- |
| `test -p lurq --lib --features canvas,perf_profile,wgpu,dx12,screenshot` | 237 passed, 0 failed, 22 ignored | 237 / 0 / 22 | `lib-all.log` |
| same plus `serde` | 240 passed, 0 failed, 22 ignored | 240 / 0 / 22 | `lib-serde.log` |
| `… wgpu_render::canvas -- --ignored --test-threads=1 --skip canvas_gpu_performance` | 5 passed: pixel parity, effects parity, camera cache and both residency tests | 5 passed | `gpu-ignored.log` |
| CI `test -p lurq --features canvas --test canvas_tests` | 49 passed | 49 | `ci-canvas-tests.log` |
| CI `test -p lurq --features canvas,perf_profile --lib` | 217 passed, 1 ignored | 217 / 1 | `ci-canvas-perf-lib.log` |
| CI `check -p lurq --features canvas,winit,wgpu,query,tokio` | ok, 4 warnings, all also in PR #40's own log | ok | `check-ci-canvas-wgpu.log` |
| CI `check -p lurq --features winit,wgpu,mcp,form,persistent_storage` | ok, 1 warning (`blend_screenshot_pixel`, untouched file) | ok | `check-ci-lifecycle.log` |
| `check -p lurq --lib --features mcp,canvas,dx12` | ok, 3 warnings, none in a touched file | ok | `check-dx12-mcp-canvas.log` |
| `check -p lurq --lib --features mcp,canvas,perf_profile,dx12` | ok, the same 3 warnings | ok | `check-dx12-mcp-canvas-perf.log` |
| `check -p lurq --lib --features canvas` | ok, 12 warnings, the same set as PR #40's `check-canvas-only.log` | ok | `check-canvas-only.log` |
| `check -p lurq --lib --features dx12` (no canvas) | ok, 1 warning (`blend_screenshot_pixel`) | ok | `check-dx12-no-canvas.log` |
| `test -p lurq --test layout_tests` (0.37.0's suites, including `shrink_give_way`) | 364 passed | not run by QA | `layout-tests.log` |
| `clippy -p lurq --lib --tests --features canvas,perf_profile,wgpu,dx12,screenshot`, head and 0.37.0 | 143 = 143 findings | 143 = 143 against 0.36.0 | `clippy-head.log`, `clippy-base-0370.log` |
| `rustfmt --edition 2024 --check` on the 7 new `.rs` files | clean | clean | (no log) |

The `profile_elapsed` unused-import warning in `wgpu_render/canvas.rs` appears without `perf_profile`, as on 0.36.0 and in QA's run.

Clippy: the file and message lists in `clippy-head-findings.txt` and `clippy-base-0370-findings.txt` are identical. The only difference is the existing `large_enum_variant` on `profiler/model.rs` `SampleData`, which grows from 1688 to 1736 bytes, the same change QA saw. There is no new finding.

### Release-gate commands from `publish-crates.yml` (beyond QA's scope)

QA did not run these. They are run here because Actions is disabled and this workflow is the publish gate.

| Command | Result | Log |
| --- | --- | --- |
| `test -p lurq_macros` | 2 passed | `ci-publish-macros.log` |
| `test -p lurq --features query --lib query::tests` | 14 passed | `ci-publish-query-lib.log` |
| `test -p lurq --features query --test query_tests` | 3 passed | `ci-publish-query-tests.log` |
| `test -p lurq --features query,tokio --lib query::tests` | 17 passed | `ci-publish-query-tokio-lib.log` |
| `test -p lurq --features query,tokio --test query_tests --test runtime_tests --test reactivity_tests` | 3, 179 and 160 passed | `ci-publish-query-tokio-suites.log` |
| `test -p lurq --test input_tests` (the whole suite, which includes CI's `checkbox`, `mask` and `text` filters) | 230 passed | `ci-publish-input-tests.log` |
| `test -p lurq --test runtime_tests text` | 8 passed | `ci-publish-runtime-text.log` |
| `test -p lurq --features resources --test resources_tests` | 16 passed | `ci-publish-resources-tests.log` |
| `test -p lurq --features svg --test svg_tests` | 3 passed | `ci-publish-svg-tests.log` |
| `test -p lurq --features svg --lib rasterize` | 4 passed | `ci-publish-svg-rasterize.log` |
| `lifecycle-menu.yml` portable job: `test -p lurq --lib --features winit,mcp` (Windows) | 187 passed | `ci-lifecycle-lib.log` |

Two CI commands are covered by other runs:
- `layout_tests text_centering` is part of the full `layout_tests` run above;
- `check -p lurq` with default features is the build that the publish dry-run verifies.

## Release probe (`app::canvas_frame_probe`, 24 frames, median of frames 5–24)

```text
cargo test -p lurq --release --lib --locked --offline --features canvas,perf_profile,wgpu,dx12,screenshot canvas_frame_probe -- --ignored --nocapture --test-threads=1
```

The binary was built first (`probe-release-build.log`), then run on an idle host (`probe-release.log`). The probe draws a synthetic bounded page, not the Kontur design. Times are CPU wall-clock ms.

| Backend / page | Counter | Combined `f068339` | QA `957d07d` | QA master `da78447` |
| --- | --- | ---: | ---: | ---: |
| DX12 labels | shape misses / evictions | 0 / 0 | 0 / 0 | 688 / 688 |
| DX12 labels | text ms | 0.26 | 0.25 | 16.48 |
| DX12 labels | textures created / evicted | 0 / 0 | 0 / 0 | 344 / 344 |
| DX12 labels | asset upload ms | 0.32 | 0.30 | 116.85 |
| DX12 labels | uploaded KiB | 0 | 0 | 2,711.62 |
| DX12 labels | Canvas total ms | 0.58 | 0.57 | 117.94 |
| DX12 images | textures created / evicted | 0 / 0 | 0 / 0 | 76 / 76 |
| DX12 images | asset upload ms / Canvas total ms | 1.02 / 1.79 | 0.86 / 1.39 | 26.99 / 27.70 |
| DX12 images | asset stretch KiB | 4,864 | 4,864 | n/a |
| WGPU labels | shape misses | 0 | 0 | 688 |
| WGPU labels | text / asset upload / Canvas total ms | 0.31 / 0.04 / 1.20 | 0.25 / 0.04 / 0.83 | 18.90 / 14.66 / 22.62 |
| WGPU images | uploaded KiB / Canvas total ms | 0 / 2.34 | 0 / 1.73 | 76 / 5.29 |

- On every page, `assets_uncached` and `shapes_uncached` are 0.
- First frames are cold:
  - DX12 labels: 688 shape misses and 344 textures, Canvas total 338.80 ms;
  - DX12 images: 1,100 textures, 558.38 ms.
- The steady counters match QA's head exactly. The steady times are within a few tenths of a millisecond of QA's.

## Packaging: `cargo publish --dry-run -p lurq --locked`

- `--offline` was refused, because publish queries the registry (`publish-dry-run-offline-refused.log`). The run without it passed (`publish-dry-run.log`). `--allow-dirty` was not needed, and the dry run stopped before uploading.
- Package: 648 files, 5.5 MiB (1.4 MiB compressed). The `.crate` is 1,429,846 bytes. `.cargo_vcs_info.json` records `f068339` with no dirty flag.
- The verify build passed with 3 dead-code warnings (`atlas_rects_to_apply`, `coalesce_dirty_rects`, `blend_screenshot_pixel`). None of these is in a file PR #40 changed.
- Registry warning: `chacha20 v0.10.1` in `Cargo.lock` is yanked. It is pulled in by `rand 0.10.2`, a lock entry that is older than this release (it dates from the MCP server commit `0a47598`). This release does not change it.
- File list (`package-file-list.txt`):
  - It holds every tracked file under `crates/lurq` except `target/name-atlas.pgm` and `target/name-atlas.png`, which `exclude = ["target/**"]` keeps out. Cargo adds `.cargo_vcs_info.json`, `Cargo.lock`, `Cargo.toml.orig` and the workspace `README.md`, which shows 0.37.1.
  - Compared with 0.37.0, the package gains only PR #40's 7 new `.rs` files.
  - No evidence, log, dump or `.tmp` file is included.

## Compatibility

- **Struct literals.** `CanvasProfile` gains 4 public fields and `CanvasTextProfile` gains 2. Both structs have only public fields, so struct literals and exhaustive patterns outside the crate stop compiling.
  - Cargo's SemVer rules treat this as a breaking change.
  - A `0.37.x` patch release reaches every `^0.37` dependent. QA's finding 5 recommended a minor bump; the version here follows the approved 0.37.1.
  - Kontur pins `=0.36.1` and constructs neither struct.
- **Eviction counts.** DX12 `asset_upload_details.cache_evictions` now also counts evictions made while drawing, so eviction counts are not like-for-like across versions.
- **Memory.** Each cache may settle at up to twice its budget while two frames need it: 128 MiB of charged textures per renderer and 16 MiB of shaped text per app by default.
- **JSON export.** The new export keys are additive.

## Open hazard: `origin/codex/k232-canvas-text-identities` (`d171b02`, another session, read only)

`git merge-tree` of this release head with `d171b02` conflicts in:
- `profiler/export.rs` and `profiler/model.rs`;
- `canvas/text.rs`, `text/profile_tests.rs` and `text/tests.rs`.

These are the same files QA found against `957d07d`.

That branch sets `asset_id: 0` in `shape_uncached` and assigns the real id through `self.identities.resolve(..)` in a hunk outside the conflict markers. If a resolution drops that call, every rendered label shares asset id 0. This release's GPU asset cache would then draw the first label's texture for all of them, and nothing fails to compile.

That merge needs a test that:
- distinct labels get distinct asset ids;
- a label shaped again keeps its id.

## Not run

- Kontur on the real design against this tree. The coordinator reports a separate A/B measurement of PR #40 on Kontur main; it is not part of this evidence.
- `canvas_gpu_performance` and `canvas_camera_prepare_benchmark`.
- `scripts/check-docs.py` (it needs a docs build).
- The macOS and Ubuntu jobs of `lifecycle-menu.yml`, including the macOS `native_macos` test and `check -p demo --bin lifecycle`, and the Ubuntu run of `publish-crates.yml`. Only the Windows half of the portable job ran.
- The D3D12 debug layer.
- A DX12 assertion test of the Canvas renderer. None exists; QA's scratch driver covered it for the PR.
