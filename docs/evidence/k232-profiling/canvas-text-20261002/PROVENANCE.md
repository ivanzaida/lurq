# Scratch originals and archive copies

The `artifact-hashes.json` manifest hashes the **published archive copies**, not the original scratch bytes. Before Git staging, the existing `retain_native.copy` helper reads UTF-8 with optional BOM removal and universal newlines, then writes UTF-8/LF. In particular, the six primary JSON captures below changed from scratch CRLF to archive LF. Independent QA verified identical parsed JSON and identical bytes after CRLF normalization. The metric verdict is unchanged.

The `-text` attribute preserves those already-normalized archive bytes in Git. It does not establish byte-identical copying from scratch. Original scratch files remain unchanged at `H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance/.tmp/k232-native-profiling/results/canvas-text-native1`; archived captures and the existing manifest are also unchanged. The same text-copy helper was used for the refined packet. Neither raw nor published evidence is rewritten by this correction.

| Capture | Original scratch SHA256 (CRLF) | Published981d6d7 Git blob SHA256 (LF) |
| --- | --- | --- |
| `identity.json` | `b371f1a59c3b18dabc99b5b1abc69a40c08da56695d0b9fcde2ec022e5fb1aed` | `28920ce301358d68fff1783b92a14716a066b1a7bedcde4fb60a3ef3bcc78b67` |
| `end1.json` | `68e2b9149f4ddbf151a8038d484745b9e7e57925968b881e28e1556672084ccb` | `2d90500d40d1fb859724c785f43cdcd19d1070206d77a6303288fc458244844e` |
| `end2.json` | `c33fe9f9768a3ba0c45583ed4aec20cc9a3663a2c3727f1f09e402038ba14665` | `b2e4b913fc123e7c9daa0fb21d2685e6221eebfdfdaa92d2cfe82524ffc806cb` |
| `summary.json` | `5fed06197078bfb96f6cf030c1576027be495c86172b4b3d12dda893e164755d` | `9a803812da72573de07810923bb9575dc78fd46cca89222325ad900880dd6c63` |
| `top-cost-passes.json` | `c334d144dfa88b01e6ae45c7d35cbbd28cc8bc6c008797741f5373020fed9909` | `3e861f67d5419d5e3d3f18637d5d34ffcc6c6f82a0536fadc65c6dd0042e4678` |
| `cleanup.json` | `ce4991b317bce5b399098a688afd2b3b48a7cadc275305958e89367d67914a2f` | `5f6db737912a5756e5903994428c675f733bb09620d741c5328705180b235050` |

These two hash scopes come from the independent `canvas-text-native-review-5e50f862.md` review. The executed source remains5e50f862/treeed21bab0; `crates/lurq` is unchanged in the981d6d7 evidence carrier and in this documentation correction. No profiler execution or compiler rerun accompanies it.
