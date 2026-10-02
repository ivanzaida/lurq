# Refined capture qualifications

This addendum qualifies the original immutable README and harness assertions; all 84 original manifest entries remain byte-identical. The source/binary, phase durations, document/ledger preservation and owned-process closure in that receipt are unchanged.

- The 72 completed records comprise **16 passes, 32 UI updates and 24 input dispatches**, not 72 rendered frames. No record drops or boundary exclusions occurred.
- Zoom returns to **14%**, but pan changes. This does not establish full viewport reversal. The final Canvas reports 1492 render instances and **4,272,928 pending bytes**, so fully settled rendering is not established. Pending bytes alone do not prove a stall.
- The `end2_immutable` native assertion rehashes the **same returned Python object**. It proves retained client-response stability, rather than a later server reread. Finalized-report immutability is established separately by independent source review and unit regressions. Session 1 retains 64 records after session 2 ends.
- The 973.389 ms `component_after_layout` duration covers the entire root and recursive hook sweep. Other hooks also run; it does not alone attribute all time to CanvasView or fonts. The separate 1.984 ms layout-compute and 108.641 ms Canvas asset-upload CPU timings support coarse attribution only. Nested scopes are not additive.

Independent source/evidence review confirmed this scoped diagnosis, with no source defect or additional run requested. It does not establish individual-node/font, GPU, FPS, precise collector overhead or optimized responsiveness. New Canvas text fields at checkpoint 5e50f862 remain unexecuted in this earlier 779190d8 capture.
