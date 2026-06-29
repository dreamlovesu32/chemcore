# ChemCore Release Quality Matrix

This matrix records the current confidence level for major public surfaces. It
is a release-quality guide, not a marketing claim.

| Surface | Status | Verification |
| --- | --- | --- |
| CDXML import | Beta | Public fixtures, published paper figures, golden SVG snapshots, parser regressions |
| CDX import/export | Beta | Round-trip tests and binary storage regression coverage |
| SVG export | Usable | Golden SVG snapshots and pixel comparison scripts |
| Office/OLE copy and embedding | Beta | Clipboard payload tests, EMF preview tests, Word paste/roundtrip validation scripts |
| Browser editor | Beta | Viewer interaction smoke tests and stability user-path scripts |
| Desktop app | Beta | Tauri build, file association config, hybrid latency regression, manual install validation |
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
| XML/CDXML parsing | Parser tests today; depth and size limits are tracked as beta-hardening work |
| Raster/vector export | Output path verification today; render timeouts and large-output caps are tracked as beta-hardening work |
| CLI session | Deterministic JSONL protocol today; request timeout and resource-budget policies are tracked as beta-hardening work |
| File writes | Output existence and byte-count verification today; stricter write-scope policies remain future work |
| Office payloads | Clipboard/OLE schema tests today; malformed-payload validation remains future work |

## Release Gate

Before a public beta release:

1. Run `npm ci`.
2. Run `cargo build -p chemcore-office -p chemcore-cli --release`.
3. Run `cargo test`.
4. Run `npm run verify`.
5. Build the installer with `npm run desktop:build`.
6. Confirm GitHub CI passes for both `main` and the release tag.
7. Upload the installer asset and record its SHA256 digest.

## Current Communication Boundary

ChemCore has a verifiable prototype for CDXML fidelity, Office workflows, and
agent-oriented CLI operation. It is still a beta build and needs more real
files, real workflows, security hardening, and clean-install validation before
being described as a full ChemDraw replacement.
