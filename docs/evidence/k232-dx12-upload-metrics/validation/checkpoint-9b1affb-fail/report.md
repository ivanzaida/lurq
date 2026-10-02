# Independent DX12 upload metrics checkpoint

Result: FAIL on exact `9b1affb544572b75be9d6e19cb897146a05db5fa`, tree `53e8a0bac526e8b61fd77b512fd37ecb5028a17f`.

The enabled profiling contracts passed 4/4, and the disabled profiling contracts passed 2/2. The actual Windows DX12 check with `mcp,canvas,perf_profile,dx12` failed with E0425: `canvas.rs:460` calls `viewport`, but the extraction omitted the parent import of `resources::viewport`. The DX12 check without profiling was not executed because the runner stops on its first failure.

The earlier source audit and 21-function body-equality proof remain valid within their stated scope; neither proved Rust name resolution. This compile failure blocks the candidate. The implementation owner must repair the import, then both actual DX12 configurations need a fresh explicit grant and check. The six contracts may carry only after verifying their relevant source remains unchanged.

Raw logs, exact commands/environment, counts, source/lock identities and the two executed test binary hashes are retained here. No product source was edited, no native window was launched, and no release build or GPU execution was attempted. Session 71814 terminated; the owned compiler lane was released. External processes were preserved.