# Independent closed ABBA review

Scoped correctness PASS for sampled-asset source `cf0c5d81ddab2c960f1aff34a4b5117a73ade9e5` and optimized candidate binary `b95da900c49f9d9e6ba521ac457180454dc5226889cf1fcbb1ffd8e00e88764f`. Baseline is Lurq `644ff6613411e28dc3035c0c910df64fcf915b51` / binary `e1de508381460bad06aaa10b38bc0c7ef68bcd10ce7b84541effcbe5125658ae`; both use Kontur `61d73d08b7c047d6c62ad55d6ed127a495881772`.

The reviewer performed offline JSON/hash reduction and full decoded RGBA comparisons, not another Cargo/native run. `review_packet.py` and `derived-review.json` retain exact reviewed input paths/hashes and independently recomputed results. Build identities, both immutable binary hashes, wrapper/local-lock hashes, ABBA source switching/restoration receipts and the terminal-zero controller agree. Builder descriptor tests are 2/2 PASS and non-profiler DX12 check is PASS; these executed receipts were inspected, not rerun by the reviewer. The separate source review found no concrete ownership/API blocker: only sampled cache-miss textures change flags/clear metadata; target/depth allocation and upload/retirement/fences remain unchanged.

| Fixture / run | Inclusive CPU upload ms | Texture-creation CPU ms | Workload ms |
| --- | ---: | ---: | ---: |
| Genuine18 / 01 Editor / A1 baseline | 2348.3936 | 2286.4500 | 3936.5553 |
| Genuine18 / 01 Editor / B1 candidate | 382.1408 | 361.9171 | 3097.7314 |
| Genuine18 / 01 Editor / B2 candidate | 429.4785 | 405.6638 | 1756.1715 |
| Genuine18 / 01 Editor / A2 baseline | 638.3638 | 606.6030 | 2009.2659 |

Raw completed pass records independently reproduce every upload child total and event count. All four have 1238 misses/creations/arena uploads/evictions, 248 hits, 1256 descriptor pairs, zero dedicated uploads, 14,143,488 padded bytes and 11,762,496 actual asset bytes. The four child scopes partition their intended CPU operations inside inclusive asset upload; eviction remains separate inside Canvas total. Parent-minus-four-children residuals are 3.4521, 1.6810, 1.9872 and 2.3226 ms. Cache gauges are per-pass observations, not summed events or GPU-memory measurements. A1 has nine passes, the others ten; all profiles finalize without dropped, truncated, excluded or in-flight samples.

All four retained profiles still contain exactly the captured revision-1 document/ledger bytes. Original and final hashes agree across runs. Page identity, empty selection, 14% zoom, pan 128.6/48, full 1492 visible/render instances, DX12 backend, 2160×1440 physical window at 1.5 scale and 1404×1257 Canvas bounds are preserved, without text/canvas refusal. The last-reported pending-byte snapshot is not interpreted as a live drain state.

All eight requested PNG pairs were independently decoded and compared over every RGBA channel with no excluded ROI. Corresponding baseline/candidate before and after captures are exactly equal. Each run's before/after differs by the same 4611 pixels, maximum channel delta 12, confined to half-open box [589,1147,692,1196]. Actual B1 before/after images were visually inspected: page artwork and neutral tools remain present; the difference is the bottom zoom-control hover highlight. This proves cross-variant pixel preservation for the captured states, not an exact zoom roundtrip or universal rendering parity.

The four launch/cleanup receipts pair PIDs 64952, 23176, 59860 and 62716 with exited=true and port4903 closed. The harness terminated its own processes; this is cleanup evidence rather than graceful application shutdown evidence.

Performance conclusion: both candidate observations have lower measured upload and texture-creation wall times than either baseline observation on this finite fixture. This is consistent with the proposed capability change being useful, but A1 is a large outlier and external compilers were observed in every run. Two observations per variant under uncontrolled load do not establish a stable percentage improvement, statistical significance, GPU-time/FPS improvement, or general interaction latency. Do not promote the descriptive mean ratios (roughly 73% upload / 18% workload) into a promised speedup. No additional execution or source changes are requested by this review.
