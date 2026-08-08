# ChemSema Release Quality Matrix

This matrix records the current confidence level for major public surfaces. It
is a release-quality guide, not a marketing claim.

| Surface | Status | Verification |
| --- | --- | --- |
| CDXML import | Beta | Public fixtures, published paper figures, golden SVG snapshots, parser regressions |
| CDX import/export | Beta | Round-trip tests and binary storage regression coverage |
| CCJS 0.2 / CCJZ v1 | Beta | Schema, migration, stable diagnostics, five-format round trips, Rust/JS/Python cross-reading, viewport loading, COW/Zip64, journal, and performance gates; ecosystem/corpus/full-report boundaries remain in the stability contract |
| SVG export | Usable | Golden SVG snapshots and pixel comparison scripts |
| Office/OLE copy and embedding | Beta | Clipboard payload tests, EMF preview tests, Word paste/roundtrip validation scripts |
| Browser editor | Beta | Viewer interaction smoke tests and stability user-path scripts |
| Desktop app | Beta | Tauri build, file association config, hybrid latency regression, manual install validation |
| GUI test platform | Implementation in progress / production pointer, keyboard, homogeneous, mixed-object, depth-two grouping, and immutable artifact sentinels passed | Versioned schemas/runner, real Playwright path, coverage/impact/resource gates, deterministic checkpoint restore, unattended Hyper-V logon, verified dedicated-user baseline, content-addressed candidate deployment, guarded UIA/CDP targeting, persistent bounded input and session-0 CDP channels, one-call guest action transactions, SHA-verified real click/drag and allowlisted scan-code keyboard input, homogeneous and mixed-object selection/clipboard/history, depth-two molecule/arrow group/ungroup with nested clipboard duplication, hierarchy-aware incremental DOM patching, primitive-count and allowlisted distinct-identity DOM oracles, kernel/DOM dual-state receipts, guest-to-host PowerShell Direct artifact transfer with guest/host SHA-256 verification, complete final screenshot/DOM/CCJS/state/WebView-log payloads, Playwright screenshot/DOM/CCJS/state/console/trace bundles, and immutable report/artifact objects with validated manifests are operational; production performance traces/video/crash bundles, the full capability matrix, deeper and additional grouped/mixed-object cells, complex/large construction, model/fault/mutation testing, and demo qualification remain |
| CLI one-shot commands | Usable | Rust tests, `npm run verify`, stability report, generated-output verification |
| CLI JSONL session | Experimental/usable | Session unit tests and large-file performance report |
| Agent precise capture | Usable beta | PNG/SVG capture tests, public fixture crops, README example crops |
| Agent context/detail | Usable beta | Selector/context/detail tests and public fixture examples |
| Installer CLI PATH/App Paths | Beta | NSIS hooks and clean install/uninstall validation |

## Security Baseline

The current beta treats these areas as hardening priorities:

| Area | Baseline |
| --- | --- |
| File import | Public fixtures, parser regression tests, and planned malicious-input corpus expansion |
| CCJZ container | Entry-count, per-entry/total-size, path, duplicate/case-collision, hash, and declaration-binding limits today; a unified public rejection conformance corpus remains pending |
| XML/CDXML parsing | Parser tests today; depth and size limits are tracked as beta-hardening work |
| Raster/vector export | Output path verification today; render timeouts and large-output caps are tracked as beta-hardening work |
| CLI session | Deterministic JSONL protocol today; request timeout and resource-budget policies are tracked as beta-hardening work |
| File writes | Output existence and byte-count verification today; stricter write-scope policies remain future work |
| Office payloads | Clipboard/OLE schema tests today; malformed-payload validation remains future work |

## Release Gate

Before a public beta release:

1. Run `npm ci`.
2. Run `cargo build -p chemsema-office -p chemsema-cli --release`.
3. Run `cargo test`.
4. Run `npm run verify`.
5. Build the installer with `npm run desktop:build`.
6. Confirm GitHub CI passes for both `main` and the release tag.
7. Upload the installer asset and record its SHA256 digest.

These are the current beta gates, not complete GUI or demo qualification. Once implemented, stable releases and formal demos must also pass `gui-pr`, `gui-nightly`, final-installer `release-qualification`, and the [Demo Qualification Gate](./gui-test-platform-and-demo-reliability.md#14-demo-qualification-gate). Its manifest must prove current valid real-interaction evidence for every feature and object, all public properties, the `0/1/2/many` matrix, and complex/large construction. Unchanged closure evidence may be reused; affected, expired, and non-cacheable work must rerun. A first-run failure remains a flaky failure even when a retry passes.

## Current Communication Boundary

ChemSema has a verifiable prototype for CDXML fidelity, Office workflows, and
agent-oriented CLI operation. It is still a beta build and needs more real
files, real workflows, security hardening, and clean-install validation before
being described as a full ChemDraw replacement.
