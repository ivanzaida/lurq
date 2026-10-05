# lurq 0.41.0: canvas text and asset cache residency

Builder evidence for branch `codex/k236-canvas-cache-0.39`. It is not independent QA. The branch:
- ports PR #40 (QA PASS at `957d07d`, based on 0.36.0);
- adds a cache-frame rule for 0.38.0's replacement presentations;
- cuts 0.41.0 from master `2ba3171` (0.40.1).

## Version and base

The release was planned as 0.39.0 from `8b0c2ac` (0.38.0). While this branch was being validated, master released 0.39.0, 0.40.0 and 0.40.1, none of them with this fix. The first dry run reported `crate lurq@0.39.0 already exists`.

The branch was therefore rebuilt on `2ba3171` and the release became 0.41.0: the next minor from master, which the coordinator confirmed. Before the final runs, `origin/master` was still `2ba3171`, and crates.io's newest version was 0.40.1.

| Commit | What |
| --- | --- |
| `2d60466` | cherry-pick of `b6291f7`: PR #40's three regression tests, on master's code |
| `02a037c` | cherry-pick of `da78447`: the release probe, on master's code |
| `b97f728` | cherry-pick of `3693f75`: the caches, conflicts resolved into 0.38.0's renderer modules |
| `19c0702` | cherry-pick of `957d07d`: PR #40's own evidence, unchanged |
| `492974f` | `fix(canvas): keep a replacement drawn in batches in one cache frame` |
| `21507db` | `chore(release): 0.41.0` |
| `584d084` | first version of this evidence; **the validated head** (tree code identical to `21507db`) |

- `runs/` holds the runs on the 0.40.1 base: at `584d084`, plus `2d60466`, `02a037c` and `2ba3171` for the master side. Every log names its head, tree and the number of dirty tracked files. All are clean except the mutation run, which has 1 dirty file by design.
- `runs-0.38.0-base/` holds the first, complete round on `8b0c2ac` (head `4075300`, then 0.39.0). On both bases, the branch adds and removes the same lines in `crates/lurq/src`.

## Port

- Cherry-picks (`-x`) rather than a merge. The tests and the probe land first on master's own code, and the conflict resolution sits in one commit that can be compared with `3693f75`.
- `3693f75` conflicted where 0.38.0 (#41) split the renderers.
  - The DX12 encode changes went into `dx12_render/canvas/encoding.rs`.
  - The WGPU draw changes went into `wgpu_render/canvas/drawing.rs`. Its `draw` was byte-identical to 0.36.0's apart from `pub(super)`, which was checked before replacing it.
  - The WGPU constructor and `with_asset_budget` went into `initialization.rs`.
  - On 0.40.1, the probe's module declaration in `app/mod.rs` needed a hand resolution, next to the capture modules 0.40.1 added.
- Both renderers keep #41's presentation fields and replace only the asset map, byte count and tick with the one `AssetCache`. `git grep` finds no per-backend eviction loop or 64 MiB constant left.

## The cache frame and the retained front

PR #40 ends a cache frame at every GPU encode. 0.38.0 lets a replacement presentation be drawn in bounded batches over several encodes, and Kontur does that for a large page. With a frame per encode, one page spans several frames. A page charged more than a budget then evicts its own first batches while its last batch is drawn, and every replacement uploads and shapes them again.

**Rule** (`canvas::FrameBoundary`, used by both renderers): an encode ends the cache frame unless one of the renderer's canvases still holds a replacement after it, and none of that canvas's replacements ended during it.
- An ordinary canvas, or a replacement begun and committed in one encode, ends a frame at every encode, as in PR #40.
- A replacement drawn over several encodes is one frame. It ends at the encode that commits, aborts or supersedes it, or that resizes or detaches its canvas.
- The text frames of the canvases a renderer encoded end with its cache frame.
- A texture the asset cache does not keep is still retired at every encode. While a frame continues, the cache holds at most its ceiling plus one encode's overflow.

**Can a retained front reference a texture the cache evicted or retired?** No.
- The front and the captured artwork are render targets owned by the presentation leases, not cached textures. They hold pixels drawn before the replacement began.
- Cached textures are sampled only by draws into the back, and are released after the GPU work that sampled them. WGPU keeps a dropped texture alive while submitted work uses it. DX12 retires it into the frame slot, which is cleared only after that slot's fence.
- The retained-front tests evict and release every texture the front was drawn from. They then read the pending front back and reproject its artwork, on WGPU and on DX12.

**Does a retained presentation pass age entries?** No. A pass that only reprojects retained artwork uses neither cache, and ending a frame with nothing used since the last end does nothing.

**Tests:**
- `frame_boundary::tests` (6) and two new `asset_cache::tests`.
- WGPU `replacement_residency_tests` (3, GPU, ignored):
  - 24 images over a 16-texture budget in three batches: uploads `[24 KiB, 0, 0]`, then five artwork-only passes, then 0 again;
  - 120 labels at 64 px, over the 8 MiB shape budget, in three batches;
  - a retained front outliving its evicted textures.
- DX12 `dx12_canvas_residency_tests` (4, GPU, ignored), through the real engine and a hidden window:
  - 1,100 images over the default budget: uploads `[1100 KiB, 0, 0, 0]`;
  - 12 images over an 8-texture ceiling: uploads `[12, 4, 4, 4]` textures, with every cell's colour read back;
  - the replacement-in-batches cases;
  - the retained front.
- **Fails without the rule** (`runs/mutation-every-encode-ends-frame.log`, at `584d084` with `end_encode` forced to end every frame): 7 tests fail.
  - 4 `FrameBoundary` tests.
  - The WGPU image page re-uploads `[24576, 24576, 24576]`.
  - The WGPU labels page cannot stretch past the budget (8,367,467 bytes).
  - DX12 fails the same image page.
  The retained-front tests and PR #40's tests still pass.

**Known limits**, unchanged from PR #40:
- The text engine is app-wide, and each renderer ends text frames for the canvases it encoded. Several windows, or passes that draw only another canvas, age the shared text cache.
- A fresh engine's first frame is least recently used within its budget. That frame lasts until the first replacement ends, so the labels tests assert the steady state from the third replacement.
- An application that leaves a replacement open keeps both caches from ageing until it ends. They stay within their ceilings.

## Host and settings

- Windows 11 Pro 10.0.26200, NVIDIA GeForce RTX 5080, rustc and cargo 1.95.0.
- Builds used `--locked --offline` (except the dry run), `CARGO_BUILD_JOBS=4`, and `CARGO_TARGET_DIR` and `TMP`/`TEMP` under this worktree's `.tmp/`.
- A process check (cargo, rustc, link, lld-link, kontur-desktop) ran as its own step before every Cargo invocation. While another agent's build or Kontur run was active, it waited and re-checked every 3 minutes. Free space on `H:` was checked against the 35 GB floor before each build, and stayed between 215 and 274 GB. See `runs/process-checks.txt`.

## Results at `584d084`

| Command (`cargo … --locked --offline`) | Result | Log |
| --- | --- | --- |
| `test -p lurq --lib --features canvas,perf_profile,wgpu,dx12,screenshot` | 266 passed, 0 failed, 32 ignored | `lib-all.log` |
| same plus `serde` | 269 passed, 0 failed, 32 ignored | `lib-serde.log` |
| `… wgpu_render::canvas -- --ignored --test-threads=1 --skip canvas_gpu_performance` | 11 passed | `gpu-ignored.log` |
| `… dx12_canvas_residency -- --ignored --test-threads=1` | 4 passed | `dx12-residency.log` |
| PR #41's 10 tests: `… presentation -- --include-ignored --test-threads=1` | 10 passed | `pr41-presentation.log` |
| CI `test --features canvas --test canvas_tests` | 49 passed | `ci-canvas-tests.log` |
| CI `test --features canvas,perf_profile --lib` | 242 passed, 1 ignored | `ci-canvas-perf-lib.log` |
| CI `check --features canvas,winit,wgpu,query,tokio` | ok | `check-ci-canvas-wgpu.log` |
| CI `check --features winit,wgpu,mcp,form,persistent_storage` | ok | `check-ci-lifecycle.log` |
| `check --lib --features mcp,canvas,dx12` / `mcp,canvas,perf_profile,dx12` / `canvas` / `dx12` | ok ×4 | `check-dx12-*.log`, `check-canvas-only.log` |
| `check -p lurq` (default features) | ok | `check-default.log` |
| `test --test layout_tests` | 364 passed | `layout-tests.log` |
| `clippy --lib --tests --features canvas,perf_profile,wgpu,dx12,screenshot`, head and `2ba3171` | 151 = 151 findings, same files and messages. Only `profiler/model.rs` `SampleData`'s existing `large_enum_variant` grows, from 1688 to 1736 bytes. | `clippy-*.log`, `clippy-*-findings.txt` |
| `rustfmt --edition 2024 --check` on the 5 new `.rs` files | clean | (no log) |

No compiler warning in these logs is on a line the branch adds or changes. This was checked against `git diff -U0 2ba3171 HEAD`.

### `publish-crates.yml` gate (Actions is disabled)

| Command | Result | Log |
| --- | --- | --- |
| `test -p lurq_macros` | 2 passed | `ci-publish-macros.log` |
| `test --features query --lib query::tests` | 14 passed | `ci-publish-query-lib.log` |
| `test --features query --test query_tests` | 3 passed | `ci-publish-query-tests.log` |
| `test --features query,tokio --lib query::tests` | 17 passed | `ci-publish-query-tokio-lib.log` |
| `test --features query,tokio --test query_tests --test runtime_tests --test reactivity_tests` | 3, 179 and 160 passed | `ci-publish-query-tokio-suites.log` |
| `test --test input_tests` (the whole suite, covering CI's `checkbox`, `mask` and `text` filters) | 253 passed | `ci-publish-input-tests.log` |
| `test --test runtime_tests text` | 8 passed | `ci-publish-runtime-text.log` |
| `test --features resources --test resources_tests` | 16 passed | `ci-publish-resources-tests.log` |
| `test --features svg --test svg_tests` | 3 passed | `ci-publish-svg-tests.log` |
| `test --features svg --lib rasterize` | 4 passed | `ci-publish-svg-rasterize.log` |
| `lifecycle-menu.yml` portable job, Windows half: `test --lib --features winit,mcp` | 215 passed | `ci-lifecycle-lib.log` |

`layout_tests text_centering` is part of the full `layout_tests` run, and `check -p lurq` with default features is above.

### Master fails, branch passes

| Test | Master code with the tests (`2d60466`) | Branch `584d084` |
| --- | --- | --- |
| `canvas::text::tests::a_page_with_more_keys_than_the_old_entry_limit_is_shaped_once` | FAIL: `frame 2 shaped 700 of 700 texts again` | ok (`lib-all.log`) |
| `residency_tests::gpu_canvas_keeps_every_asset_of_a_frame_larger_than_the_budget` | FAIL: re-uploads `[77824, 77824, 77824]` | ok (`gpu-ignored.log`) |
| `residency_tests::gpu_canvas_uploads_a_page_of_labels_once` | FAIL: re-uploads `[687528, 687528, 687528]` | ok (`gpu-ignored.log`) |

The master-side logs are `base-2d60466-text-collapse.log` and `base-2d60466-gpu-residency.log`. The same three failed on `8b0c2ac`'s code in the first round.

### Release probe (`app::canvas_frame_probe`, 24 frames, median of frames 5–24)

- Both binaries were built with `cargo test -p lurq --release --lib --features canvas,perf_profile,wgpu,dx12,screenshot --no-run`: master code at `02a037c` (`2ba3171` plus the probe) and the branch at `584d084`.
- They ran back to back on an idle host, 14:15:53–14:16:10, with a clear process check before and after each.
- Logs: `probe-release-master-02a037c.log`, `probe-release-head-584d084.log` and `probe-process-checks.txt`.
- The probe draws a synthetic bounded page as an ordinary canvas: not the Kontur design, and not a replacement in batches. Times are CPU wall-clock ms.

| Backend / page | Counter | Master `02a037c` | Branch `584d084` |
| --- | --- | ---: | ---: |
| DX12 labels | shape misses / evictions | 688 / 688 | 0 / 0 |
| DX12 labels | text ms | 15.02 | 0.27 |
| DX12 labels | textures created / evicted | 344 / 344 | 0 / 0 |
| DX12 labels | asset upload ms | 91.88 | 0.30 |
| DX12 labels | uploaded KiB | 2,711.62 | 0 |
| DX12 labels | Canvas total ms | 92.74 | 0.59 |
| DX12 images | textures created / evicted | 76 / 76 | 0 / 0 |
| DX12 images | asset upload ms / Canvas total ms | 21.42 / 22.06 | 0.96 / 1.70 |
| DX12 images | asset stretch KiB | n/a | 4,864 |
| WGPU labels | shape misses | 688 | 0 |
| WGPU labels | text / asset upload / Canvas total ms | 14.53 / 11.10 / 17.02 | 0.36 / 0.05 / 1.15 |
| WGPU images | uploaded KiB / asset upload ms / Canvas total ms | 76 / 2.62 / 5.25 | 0 / 0.16 / 2.48 |

- `assets_uncached` and `shapes_uncached` are 0 on every page of the branch.
- First frames are cold on both builds. DX12 labels Canvas total: 281.07 ms on master, 287.72 ms on the branch.

### Packaging: `cargo publish --dry-run -p lurq --locked` at `584d084`

- The dry run passed and stopped before uploading (`warning: aborting upload due to dry run`). `--allow-dirty` was not needed.
- The package held 693 files, 5.7 MiB (1.4 MiB compressed). The `.crate` is 1,483,246 bytes. `.cargo_vcs_info.json` records `584d084` with no dirty flag. Logs: `publish-dry-run.log`, `package-file-list.log`.
- There was no "already exists" warning for 0.41.0.
- The verify build passed with 3 dead-code warnings (`atlas_rects_to_apply`, `coalesce_dirty_rects`, `blend_screenshot_pixel`), none in a changed file.
- Registry warning: `chacha20 v0.10.1` in `Cargo.lock` is yanked. This is unchanged and older than this branch.
- File-list check, comparing the list against `git ls-files crates/lurq`:
  - The package is every tracked file except `target/name-atlas.pgm` and `target/name-atlas.png`, which `exclude = ["target/**"]` keeps out. Cargo adds `.cargo_vcs_info.json`, `Cargo.lock`, `Cargo.toml.orig` and the workspace `README.md`.
  - Against `2ba3171`'s tracked files, it gains exactly the branch's 12 new `.rs` files.
  - There is no evidence, log, dump or `.tmp` file.

## Compatibility (0.41.0)

- **Breaking.** `CanvasProfile` gains 4 public fields and `CanvasTextProfile` gains 2. Both have only public fields, so struct literals and exhaustive patterns outside the crate stop compiling. Under Cargo's SemVer rules that is a minor bump for 0.x.
- **New API, additive.** `CanvasAssetBudget` and `with_canvas_asset_budget` on both engines. The JSON export keys are additive.
- **Eviction counts.** DX12 `cache_evictions` now also counts evictions made while drawing.
- **Memory.** Each cache may settle at up to twice its budget while two frames need it, and a replacement drawn in batches holds its frame until it ends.
- **CHANGELOG.** It adds the 0.38.0 entry master lacked, a summary of #41.

## Open hazard: `origin/codex/k232-canvas-text-identities` (`d171b02`, unmerged, read only)

`git merge-tree` of this branch with `d171b02` conflicts in:
- `profiler/export.rs` and `profiler/model.rs`;
- `canvas/text.rs`, `text/profile_tests.rs` and `text/tests.rs`.

In the merged `canvas/text.rs`, `result.asset_id = self.identities.resolve(..)` lands outside the conflict markers, after `shape_uncached`, which sets `asset_id: 0`. If a resolution drops that call, every rendered label shares asset id 0. The shared GPU asset cache would then draw the first label's texture for all of them, with no compile error.

That merge needs a test that:
- distinct labels get distinct asset ids;
- a label shaped again keeps its id.

## Not run

- Kontur on the real design against this branch.
- `canvas_gpu_performance` and `canvas_camera_prepare_benchmark`.
- `scripts/check-docs.py` (it needs a docs build).
- The macOS and Ubuntu jobs of `lifecycle-menu.yml` and `publish-crates.yml`.
- The D3D12 debug layer.
