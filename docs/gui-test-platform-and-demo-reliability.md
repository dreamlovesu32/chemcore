# ChemSema GUI Test Platform and Demo Reliability Architecture

Status: long-term architecture and implementation contract. This document defines the final boundaries for GUI automation, real desktop validation, model-based testing, fault injection, demo qualification, and test artifacts. Ad hoc scripts, manual smoke tests, and step-by-step AI computer use are not substitutes for this evidence.

This contract complements the [release quality matrix](./release-quality.md), the [Windows desktop and Office architecture](./windows-desktop-office-architecture.md), the [core contract audit](./core-contract-audit-2026-07-23.zh-CN.md), and the [CCJS 0.2 stability architecture](./ccjs-v0.2-stability-architecture.zh-CN.md).

## 1. Decision

ChemSema will build an independent GUI test platform inside the main repository. Platform code, product scenarios, the Test ABI, fixtures, coverage registries, and release gates must change atomically with the product. Large traces, videos, soak logs, VM images, installers, and corpora belong in artifact storage; the repository retains immutable manifests, hashes, minimal failures, and qualification results.

One versioned scenario model drives four execution surfaces:

1. **WebdriverIO Tauri** for the real Tauri application, WebView, IPC, windows, and frontend/backend logs;
2. **Playwright** for high-throughput browser tests, real WebView2 CDP cross-checks, visual comparisons, accessibility snapshots, and traces;
3. **Windows UI Automation and input injection** for native dialogs, focus, clipboard, file associations, external drag/drop, touch, pen, and IME behavior;
4. **Production black-box execution** against the final installer and installed application without test backdoors.

AI may propose scenarios, identify coverage gaps, classify failure bundles, and turn exploration into candidate regressions. A deterministic runner performs, reproduces, shrinks, and judges every action sequence without an LLM in the execution loop.

## 2. Reliability problem

A user gesture crosses several authorities:

```text
Windows input/window
  -> WebView DOM/SVG
  -> viewer interaction state
  -> WASM engine
  -> Tauri/native service
  -> file/clipboard/Office/recovery storage
  -> viewer revision and render synchronization
```

Passing unit tests cannot prove that focus, timing, revisions, caches, rendering, persistence, and recovery agree across this chain. GUI reliability must therefore be a versioned product capability, not a collection of browser scripts.

The platform must prove that:

- input reached the intended visible target;
- viewer, engine, native service, and persisted state agree on revision;
- visible output agrees with semantic state;
- undo/redo, save/reopen, and crash recovery preserve their contracts;
- delay, cancellation, resource failure, and window changes cannot leave half-committed state;
- the immutable production candidate repeats the actual demo journey on clean machines;
- the test system detects seeded faults and mutations instead of merely passing correct code.

## 3. Repository and artifact boundary

The main repository owns:

- scenario, result, coverage, and artifact-manifest schemas;
- the runner, scheduler, drivers, oracles, generators, shrinker, fault injection, and mutation harness;
- the test-only ABI and build feature;
- product and demo scenarios;
- small canonical fixtures, minimal historical regressions, and required visual baselines;
- capability/environment matrices and CI workflows;
- qualification manifests bound to candidate hashes.

External artifact storage owns large success traces, videos, corpora, long soak logs, VM/runtime images, installers, crash dumps, and profiles. Every retained artifact is addressed by URI, size, SHA-256, source commit, candidate hash, environment, driver version, seed, and retention policy. A qualification report must never point only to `latest`.

A generic runner core may be extracted only after it serves at least two independently released products through a stable public API. ChemSema scenarios, fixtures, Test ABI, coverage, and qualification remain here.

Automation must never take over the developer's active Windows desktop. Engine, format, headless-browser, UIA-pattern, and static-oracle work runs in background workers. Real click/drag/draw, focus, touch, pen, IME, native-dialog, and system-shortcut cases run only on an unlocked Hyper-V guest desktop or dedicated test machine. The host coordinator starts and monitors workers and transfers manifests; it never injects host-session input or reads host clipboard/user files. Injection fails closed unless VM/session identity, target process, foreground window, and test root are all verified. Hardware behavior that Hyper-V cannot represent uses a dedicated physical worker.

## 4. Target repository layout

```text
packages/gui-test/
  src/{cli,protocol,runner,scheduler,actions,oracles,coverage,generators,shrinker}
  src/drivers/{wdio-tauri,playwright-browser,playwright-webview2,windows-uia,production-black-box}
  src/{fault-injection,mutation,reporters}

crates/chemsema-test-support/
  test-build-only Rust observability, fault injection, and Test ABI

tests/gui/
  schemas/
  scenarios/{core,tools,dialogs,documents,clipboard-office,windows,accessibility,performance,recovery,demo}
  fixtures/
  baselines/
  coverage/
  qualification/

.github/workflows/{gui-pr,gui-nightly,demo-qualification,release-qualification}.yml
```

The platform exposes a standalone CLI for listing, running, exploring, reproducing, shrinking, and qualifying scenarios.

## 5. Scenario protocol

All fixed, generated, and minimized cases use `chemsema.gui.scenario.v1`, validated by JSON Schema. Scenarios cannot hide their behavior inside arbitrary JavaScript closures.

A scenario declares:

- stable id, title, schema, risk, owner, and originating defect;
- required capabilities and allowed drivers;
- fixture, window, DPI, locale, theme, and runtime profile;
- stable action ids, completion conditions, and time budgets;
- intermediate and final oracles;
- coverage tags and reproducible seeds;
- an empty-by-default allowlist of expected diagnostics.

Targets resolve in this order:

1. role and accessible name;
2. stable test id or AutomationId;
3. document entity/node/bond identity;
4. authoritative world geometry;
5. raw screen coordinates only for OS-boundary cases with recorded window and DPI metadata.

CSS ancestry, `nth-child`, incidental labels, and ephemeral runtime ids are not durable contracts.

The action vocabulary includes controls, pointer, keyboard and IME, touch and pen, documents, clipboard and Office, windows, runtime failures, and observation checkpoints. Every action declares its coordinate space, device, modifiers, completion condition, and budget. Fixed sleeps are not normal completion conditions.

### Exhaustive real-user interaction contract

The coverage unit is not "the page opened"; it is a user completing a feature through public interaction. Every user-visible tool, button, menu or context command, shortcut, dialog, property editor, file command, and system integration must have at least one scenario that uses the same real click, key, drag, draw, text-entry, or selection path as a user and then applies independent oracles. Engine calls, the Test ABI, JavaScript functions, and direct document injection may arrange setup or diagnostics, but never count as real interaction coverage for that feature.

Every creatable object type must be created or drawn through the GUI, not merely loaded from a fixture. Its lifecycle covers selection and deselection, hover/focus, movement and, where applicable, resize, rotation, handle editing, text or chemical editing; every public writable property; copy/cut/paste/duplicate/delete; undo/redo; save/close/reopen; and applicable import/export, clipboard, and Office round trips. Property cases cover defaults, representative values, boundaries, mixed values, invalid/cancel paths, and persistence. A property-field change is not sufficient unless visual, semantic-document, and persisted results agree.

Cardinality is mandatory: `0`, `1`, `2`, and `many`, including homogeneous and heterogeneous selections. Multi-object cases cover additive/removal selection, marquee/lasso, select-all, overlap/intersection, distant and partly offscreen objects, locked/hidden/inapplicable objects, inside/outside and nested groups, and large documents. Every batch-capable feature verifies common-property updates, mixed-state presentation, partial applicability, hierarchy and relative-position preservation, atomicity, one-transaction undo/redo, and cleanup of selection, previews, and handles.

Object types are also tested in combination: connection, snapping, alignment, distribution, hierarchy, group/ungroup, z-order, cross-group movement, copied references, dependent deletion, cross-document paste, and every publicly allowed or forbidden interaction among molecules, bonds, atom labels, text, symbols, and other graphical objects. Forbidden combinations must produce explicit feedback, no document mutation, and no dirty history entry.

The minimum auditable coverage cell is:

```text
feature × object-type × cardinality × selection/state × input-mode
        × property/value-class × persistence-boundary × platform-profile
```

Critical and high-risk editing surfaces cover their declared full matrix. Constrained pairwise or model generation may reduce redundant combinations elsewhere, but may not omit a public feature, object type, cardinality class, writable property, or persistence boundary. The registry distinguishes `real-user-path`, `setup-only`, and `oracle-only`; only the first satisfies functional interaction coverage. A feature cannot merge without registered object, cardinality, state, property, and real-input scenarios.

## 6. Driver contract

Every driver implements:

```text
prepare(profile)
launch(candidate)
capabilities()
resolve(target)
perform(action)
observe(query)
checkpoint(label)
collect_artifacts(policy)
shutdown()
```

Each action returns a standard receipt with resolved target, input type, timing, before/after revision and window, completion evidence, diagnostics, and artifact references.

### WebdriverIO Tauri

The primary real-desktop driver launches test or production candidates, drives WebView elements, verifies multiple windows, captures frontend/backend logs, and uses only explicitly granted test IPC. Webdriver and execute/mock plugins are registered solely under a test build feature.

### Playwright browser and WebView2

The browser driver handles high-concurrency interaction, visual, ARIA, and long model sequences. The WebView2 driver connects to the real desktop WebView through an isolated CDP port and user-data directory, cross-validates WebdriverIO, and emits Playwright traces. Workers never share a profile.

### Windows UIA and input

UIA patterns are preferred for accessible controls, windows, and native dialogs. OS input injection is reserved for hover, free-canvas gestures, shortcuts, touch, pen, and IME. Injection runs only on an exclusive unlocked desktop after revalidating the foreground window, target rectangle, DPI, and process identity.

### Production black box

The black-box driver accepts only a final installer, install directory, or release archive. It cannot call the Test ABI, use debug globals, register test plugins, or set internal documents. Its evidence comes from public UI/UIA and public DOM state, process status, browser-level performance traces, logs and dumps, files, system clipboard/Office payloads, and final screenshots. CDP may transport public observations and browser traces, but it cannot inject the user action or read application-private state.

### Execution pools

`background-worker` shards run concurrently within resource budgets. Each `interactive-isolated-worker` desktop permits only one real input stream; real-GUI parallelism therefore uses multiple isolated VMs or dedicated sessions, never competing drivers on one desktop. Capability routing cannot substitute a weaker driver for required real-input evidence.

## 7. Test ABI and observability

The current debug global is not a stable protocol. It will be replaced by test-build-only `chemsema.test.abi.v1`.

Permitted capabilities include fixture setup, isolated reset, canonical document fingerprints, revisions, selection and undo/redo state, coordinate conversion, authoritative bounds, quiescence across UI/render/native/journal/autosave work, counters and pending tasks, deterministic clock/random/fault profiles, structured events, and controlled failures.

The ABI must not perform the user action under test, ship in production, execute arbitrary code, change chemistry/hit-testing/rendering rules, or create a separate business implementation.

Every high-level interaction emits a structured journal containing action id, command, revision, runtime authority, commit/cancel/failure, render patch, native acknowledgement, diagnostics, and timing. Failure analysis cannot depend on concatenated console strings.

## 8. Independent oracles

Important cases combine multiple oracles.

### Interaction

- visible, stable, unobscured targets;
- correct focus, hover, cursor, selection, and handles;
- one gesture start and exactly one commit or cancel;
- complete cleanup of previews, masks, overlays, and pending actions.

### Document and chemistry

- canonical CCJS fingerprint and chemical invariants;
- one revision per content command;
- reversible undo/redo fingerprints;
- agreement among local WASM, native service, and saved snapshot.

### Rendering and visual output

- semantic DOM/SVG objects, render primitives, local and whole-window screenshots;
- per-object and per-file gates rather than aggregate pass counts;
- baselines bound to OS, driver, runtime, fonts, DPI, theme, and GPU profile;
- reviewed baseline changes with cause, scope, and diff evidence.

### Accessibility

- ARIA/UIA tree, names, roles, states, and patterns;
- complete keyboard navigation and dialog focus behavior;
- a navigable semantic representation for canvas objects.

### Persistence and external integration

- verified output bytes and hashes;
- reopen in a new process;
- independent CLI/engine validation and format round trips;
- clipboard, Office/OLE, EMF preview, file association, and recovery evidence.

### Runtime quality

- zero unexpected exceptions, rejections, panics, traps, or driver errors;
- unexpected warnings/errors and recovery fallbacks fail according to policy;
- explicit startup/action/render/save/recovery budgets;
- bounded memory, handles, process count, and temporary files after loops.

## 9. Model-based, generated, and AI-assisted testing

The authoritative state model spans document lifecycle, active tool, selection/focus, pointer gesture, dialog/menu, tab/window, clipboard, persistence/recovery, and backend health. Coverage is measured over states and transitions, not test count alone.

Seeded generators produce normal journeys, boundary targeting, rapid switching, cancellation/re-entry, focus loss, asynchronous races, large documents, delayed/failed resources, cross-format/tab/window operations, and Office workflows.

Every failure stores the seed, model state, action receipts, candidate, and evidence. A shrinker removes steps, reduces fixtures, simplifies coordinates, and minimizes objects while preserving the failure signature, yielding a permanent regression.

AI may generate candidate cases and classify evidence, but cannot be the sole oracle, approve baselines, replace machine reports with prose, or claim coverage without a reproducible scenario.

## 10. Fault injection and mutation qualification

Test builds must deterministically simulate IPC delay/reordering/failure, missing/denied/full/partial file writes, clipboard contention and malformed formats, WebView reload, WASM startup failure, resource/font/image failures, autosave/journal/exit races, and Office service or preview failures.

Each fault has a stable id, trigger, count, delay, and expected user outcome. Real machine damage is never used to simulate a fault.

Qualification also seeds mutations such as removed listeners, shifted hit testing, dropped patches or acknowledgements, falsely successful corrupt saves, stale UI snapshots, leaked previews/selections, skipped undo revisions, and swallowed exceptions. Core gates must kill every required mutation. A surviving mutation is a coverage or oracle defect even when correct code passes.

## 11. Coverage registry

`chemsema.gui.coverage.v1` tracks:

- a `real-user-path` scenario for every user-visible control, command, and property rather than only Test ABI or fixture setup;
- create/select/hover/focus/move/resize/rotate/style/copy/cut/paste/delete/undo/redo per tool and object type;
- import/render/hit/edit/export/save/reopen per object type;
- `0/1/2/many`, homogeneous/heterogeneous, grouped/nested, overlapping, locked, hidden, partially applicable, and large-document multi-object states;
- defaults, representative/boundary/mixed/invalid values, batch editing, undo/redo, and round trips for every public writable property;
- connection, snapping, alignment, distribution, hierarchy, grouping, z-order, dependent deletion, and cross-document object relationships;
- confirm/cancel/invalid-input/keyboard/focus behavior per dialog;
- shortcuts and modifiers;
- open/edit/save/reopen/export per format;
- Web, Tauri test build, production build, Office, and OS boundaries;
- success/cancel/error/timeout/recovery/crash branches;
- DPI/window/locale/theme/runtime/GPU profiles;
- permanent scenarios for every historical defect and demo action.

New tools, object types, commands, properties, dialogs, formats, and system capabilities must register their required cells. Unregistered items, items without real click/draw/edit scenarios, and items covered only by internal injection block CI.

### Complex and large documents

Opening a fixture does not prove construction. Starting from a blank document, GUI scenarios build `small`, mixed `complex`, `large` (hundreds of objects or roughly 1,000 atoms), and `xlarge` (initially 5,000 atoms or equivalent interaction/render complexity) tiers. Complex cases combine heterogeneous objects, molecules, graphics/text/symbols, applicable relationships, mixed selections and properties, nested groups, hierarchy, and copy/paste. Large tiers cover both progressive UI construction and continued editing after open, and assert incremental patches rather than full refreshes, latency, feedback cleanup, memory/handles, autosave/journal, undo/redo, save/reopen, recovery, and canonical fingerprints. Real UI copy, batch commands, and templates may accelerate construction; direct injection of the final document cannot count. Long mixed sequences and 24-hour soak remain mandatory.

### Code-to-test impact graph and evidence reuse

`chemsema.gui.impact.v1` links source files, crates/packages, generated WASM, viewer surfaces, native/Office commands, schemas, capabilities, drivers, oracles, and scenarios. Selection follows the transitive closure of the actual diff rather than directory guesses or manual tags.

Passing evidence is content-addressed by scenario/data, product component closure and generated artifacts, fixtures/baselines, driver/oracle/runner, build flags, environment profile, and capability-contract version. It is reusable only while that closure is unchanged, the environment remains compatible, the evidence is unexpired, and no related escaped defect invalidates it. Reports distinguish `executed`, `reused`, and `invalidated` with reasons. Lockfile, compiler, WASM, WebView2, font, schema, driver, oracle, data, and environment changes invalidate their dependent cells. Shared interaction/render/document changes or uncertain boundaries expand the closure.

Completeness means every required coverage cell has current valid evidence, not that every run starts from zero. Unchanged deterministic evidence is reused. Changed/expired cells, new random/model seeds, soak, leak, fault, environment-drift, and historical-defect cases run. The selector itself is mutation-tested; every missed regression permanently widens the relevant dependency edge.

## 12. Determinism, isolation, and reporting

Workers use separate temporary roots, document directories, WebView profiles, ports, journal/autosave/log/artifact directories, random seeds, and clock/locale settings. System clipboard cases acquire an exclusive lock. Tests never touch user configuration or project files.

Retries collect evidence but never rewrite the first failure as success. Fail-then-pass is a flaky failure and remains blocking.

All ChemSema workers share a hard aggregate budget of **10 CPU execution units** (Windows logical processors/vCPUs) and **30 GiB of host committed-memory increase**. Guest vCPUs, host worker slots, and coordination all debit one CPU budget; guest allocation, `vmwp`/Hyper-V overhead, host runners, caches, and reports all debit one measured memory budget. Hyper-V limits, Windows Job Objects/affinity, process sampling, and admission control enforce it. The default profile uses two interactive workers of at most 4 vCPU/10 GiB each, reserving 2 CPU units/10 GiB for host work and virtualization overhead. `xlarge`, Office, and high-memory cases use one heavy worker of at most 8 vCPU/20 GiB with the same reserve. Disk, GPU, thermal, and power pressure pauses/checkpoints work rather than creating flake.

VM profiles use versioned clean checkpoints, dedicated test accounts, isolated storage/network/clipboard, environment attestation, artifact export, and rollback. PowerShell Direct credentials are collected only through a secure system prompt, encrypted outside the repository with the host user's DPAPI, and ACL-limited to that user, SYSTEM, and Administrators; plaintext credentials never enter commands, logs, scenarios, or CI artifacts. For online profiles, the coordinator derives the live Hyper-V switch prefix/gateway, configures the guest, and independently verifies DNS, HTTPS, and file round trips instead of trusting an old DHCP lease. VMs never mount user project directories or personal profiles. Coordinator recovery is idempotent after reboot or power loss.

Every `chemsema.gui.run.v1` report records the commit and dirty state, candidate/installer hashes, environment and driver matrix, scenarios/seeds/workers, action receipts, oracle results, coverage deltas, impact inputs and evidence reuse/invalidation reasons, host/guest/session isolation, resource curves, faults/mutations, failure signatures, and artifact manifest.

A failure bundle includes the original and minimized scenario, fixture and final snapshots, written files, traces or action journal, structured logs and crash references, screenshots and visual/accessibility diffs, and one copy-ready reproduction command.

## 13. CI gates

### `verify`

Retains Rust, format, WASM, container, and static contracts. It is not described as complete GUI validation.

### `gui-pr`

Uses `chemsema.gui.impact.v1` to run the transitive affected closure, reuses other valid content-addressed evidence, and mutation-tests the selector. Uncertain boundaries expand rather than narrow selection. It also runs semantic/visual/accessibility/log/performance oracles and mutation smoke for the changed area.

### `gui-nightly`

Requires current evidence for the full feature/object/tool/property and `0/1/2/many` matrix, but executes only new, affected, expired, rotating-environment, and non-cacheable cells. Complex/large construction, new model seeds, long sequences, soak, leak, random fault, native Windows/Office, and environment-drift work continues even when source is unchanged.

### `release-qualification`

Accepts only an immutable final installer candidate whose manifest maps every required feature, object, property, multi-object, and complex/large cell to evidence valid for its current component closure. Identical closures may reuse evidence after unrelated changes. The final installer still runs non-reusable clean-install/cold-start, production integration sentinels, affected features, save/reopen, upgrade/uninstall/reinstall, and release soak. A change invalidates its dependent evidence, not unrelated proofs.

## 14. Demo Qualification Gate

Every formal demonstration has a versioned `chemsema.gui.demo.v1` journey matching the exact planned files and actions. The presenter cannot substitute unqualified files or paths at the last minute.

A demo candidate must:

1. install and cold-start on a clean Windows VM;
2. run offline without a dev server, stale cache, or online assets;
3. use the production build with no Test ABI;
4. complete every demo journey at least 1,000 consecutive times with zero failure, flake, or unexpected diagnostic;
5. pass on at least three independent machine/VM profiles;
6. cover 100%, 125%, 150%, and 200% DPI plus the actual presentation resolution;
7. complete at least 24 hours of mixed-journey soak without crash, hang, unhandled error, or unrecoverable state;
8. stay within approved memory, handle, process, temporary-file, autosave, and journal budgets;
9. reopen and independently validate every saved file in a new process;
10. qualify target Office versions when Office/OLE is part of the demonstration;
11. archive the candidate, installer, scenario, environment, report, and hashes;
12. be the immutable artifact used on stage.

A first-run failure invalidates the qualification even if a retry passes. A fix produces a new candidate hash and a new qualification.

## 15. Runtime reliability requirements

Testing must be paired with a single structured UI action/error bus, explicit commit/cancel/failure receipts, local/native revision barriers, startup health checks and offline resource guarantees, verified autosave/journal recovery, explicit timeout states, exportable diagnostics, and safe degradation instead of false success.

Any recovery fallback, background error, unexpected warning, or over-budget action is a signal even if the final UI appears correct.

## 16. Migration

Existing GUI, Playwright, interaction, stability, toolbar, text, runtime, large-document, and Office scripts are valuable source evidence but will not grow as separate monoliths.

Migration proceeds by inventorying each case and assertion, extracting shared infrastructure, converting behavior into the versioned scenario protocol, moving observation to Test ABI oracles while retaining real UI actions, cross-running browser and Tauri drivers, adding UIA/black-box variants, mutation-qualifying replacements, and retiring old entries only after coverage equivalence is auditable.

No current gate is weakened merely because the new platform is under construction.

## 17. Implementation phases

1. **Incident ledger and coverage baseline**: map every historical demo bug and existing test to a future scenario id.
2. **Protocols and runner**: schemas, impact graph/evidence keys, 10-CPU-unit/30-GiB scheduler, isolation, receipts, oracles, CLI, reporting, shrinking, and runner mutation tests.
3. **Test ABI and desktop drivers**: structured events, quiescence, faults, Hyper-V background/interactive pools, host-input fail-closed, WebdriverIO Tauri, Playwright WebView2, and test/production permission checks.
4. **Regression migration**: move all current suites, establish visual/accessibility/persistence/performance oracles, and enable `gui-pr`.
5. **Model, fault, and platform matrix**: generators, shrinker, complex/large/xlarge blank-document construction, 24-hour soak, Office/OS boundaries, nightly matrix, and zero-tolerance flake policy.
6. **Demo and release qualification**: recorder, clean VM, final installer, 1,000 repeats, 24-hour soak, immutable manifests, and archived evidence.

## 18. Completion definition

The platform is not complete because it can click controls or once reports green. Completion requires versioned scenarios, all four formal driver surfaces, automated test/production and host-desktop isolation, permanent coverage for every historical demo defect, real public-input execution of every user-visible feature, GUI creation/drawing and full writable-property/lifecycle coverage for every object type, explicit `0/1/2/many` homogeneous and heterogeneous multi-object and cross-object scenarios, blank-document complex/large/5,000-atom-equivalent construction, machine-readable coverage, an audited impact graph with valid evidence reuse, aggregate 10-CPU-unit/30-GiB enforcement, reproducible and shrinkable seeded failures, a fully killed core mutation set, operational PR/nightly/release/demo gates, no retry-to-green, production-installer evidence, hashed artifact manifests, and synchronized documentation/protocol/help.

## 19. Current workstation validation (2026-08-08)

The Hyper-V module is present and `vmms`/`vmcompute` are running on a host with 24 logical processors and about 63.4 GiB RAM. `jiajun\dream` is enabled in `Hyper-V Administrators`. The existing Windows 11 test VM (document alias `windows-gui-worker-current`) is Generation 2 with 8 vCPU and 4–20 GiB dynamic memory; its configuration, checkpoint, and VHDX/AVHDX chain were readable. On 2026-08-08 it was started, reported healthy heartbeat/time/KVP/shutdown integration, and was normally stopped with its automatic checkpoint merged. Office activation is user-confirmed but was not independently checked in the Office UI.

The dedicated `chemsema-test` guest account and `vmicvmsession` are enabled. Its credential was collected through a secure prompt, stored outside the repository with DPAPI, and ACL-limited to `jiajun\dream`, SYSTEM, and Administrators. PowerShell Direct connected successfully. Because Default Switch DHCP yielded only `169.254.*`, the coordinator derived the host's `172.31.0.1/20` network and assigned guest `172.31.15.250/20`. DNS and HTTPS 443 succeeded; a real request to `https://www.microsoft.com/` returned HTTP 200 and 201,253 bytes. Host-to-guest SHA-256 and guest-to-host content round trips matched. VM lifecycle, PowerShell Direct, guest networking, and bidirectional file transport are operational.

### Initial executable platform slice

The repository now contains the first executable vertical slice under `packages/gui-test` and `tests/gui`. It includes JSON Schemas for scenarios, run reports, coverage, impact graphs, artifact manifests, and worker profiles; strict schema validation; canonical content-addressed evidence keys; transitive impact selection; fail-closed aggregate 10-CPU-unit/30-GiB admission control; action budgets and before/after receipts; a fake driver; a Playwright browser driver; and a Hyper-V coordinator. The versioned `core.bond.draw-single` scenario uses public accessible targeting and real pointer drag input. The same scenario passes runner self-tests through the fake driver and executes successfully through headless Edge, producing a validated `chemsema.gui.run.v1` report. Existing regression scripts remain active while their cases are inventoried and migrated.

The coordinator has now crossed the isolated-desktop boundary on `windows-gui-worker-current`. The dedicated account logs on unattended using an LSA-stored autologon secret; no plaintext Winlogon password is present. The Rust agent is transferred with host/guest SHA-256 equality, runs without creating or stealing a console window, and distinguishes service session 0 from the unlocked interactive `Default` desktop. Desktop candidates are built from current source, copied into SHA-256-addressed guest directories, rehashed before launch, and started at ordinary user integrity. Every activation/click/drag receipt binds the dedicated account, nonzero session, exact candidate PID and executable, foreground window, in-window coordinates, and bounded run directory.

The first production-desktop sentinel was executed through the formal scenario runner on 2026-08-08 without touching the host foreground. The `production-black-box` driver boots the isolated VM, installs content-addressed agent and candidate binaries, applies and verifies a dedicated-user desktop baseline, launches the production desktop, resolves the scoped `Single bond` control and `#viewer-container` through guest-loopback CDP, and sends a real Windows click plus an eight-step real drag through the guarded guest agent. The validated `chemsema.gui.run.v1` report records both completed actions, candidate SHA-256 `72f99bcd35b8dc24a837001e0fa6d707bc26e50f18a6428bec9fb42c6a27103f`, rendered bonds changing from zero to one, a dirty window title, passing DOM and diagnostic oracles, and evidence key `cbbcbca14237b0281e683b35f0907d473c33b4c1a45cd255bc14006370214176`. Every CLI run stores the validated report as an immutable SHA-256 object and writes a schema-validated manifest under its evidence key and run id. Windows UI Automation remains responsible for native/window surfaces; CDP supplies semantic bounds and independent observations while OS input remains external and guarded. The recurring `CloudExperienceHost` account prompt is reduced by the versioned dedicated-user baseline and, when still present, dismissed only after the agent verifies the exact system path, window class, title, and application model ID. At this first-sentinel stage, full screenshot/trace/log bundles and the remaining capability matrix were still incomplete.

Since that first report, deterministic reset has become operational: every production run restores the profile checkpoint by immutable ID, rejects automatic checkpoints, and verifies the worker remains off before boot. Input now uses one persistent interactive guest agent with a bounded file channel and fixed click/drag/key protocol rather than a scheduled task per action; keyboard input is allowlisted, rejects secure-attention and Windows-key combinations, uses physical scan codes, and revalidates the exact foreground candidate before and after injection.

The formal `core.history.undo-redo-bond.production` scenario now passes against an immutable production candidate. It uses a real mouse click and drag to create a bond, then real `Control+Z` and `Control+Y` chords, with both the kernel document and independent DOM observation proving `0 -> 1 -> 0 -> 1`. Candidate SHA-256 `739faffa72717bff3eeca5b2817ff1c5f8459a49ab7bc06ab2e0a9ed3bc10773` produced evidence key `5d1dbe4bce601b3e950232b5191756859445bf44581e3526dc3dc91277e757fa`; all four actions remained within their original 12-second budgets and both final oracles passed. The scenario exposed and fixed three real production defects rather than weakening the gate: undo/redo bypassed the versioned command-result path, render-empty object transitions retained stale primitives, and a stale/missing primitive index could leave ghost DOM. Production receipts now retain the loaded app asset URL plus engine type, history state, kernel bond count, command result, and incremental-sync mode so future failures distinguish input, command, model, and rendering layers.

Persistent CDP observation is now operational through versioned bounded request/response channels. One hidden observer is installed once per run, is required by schema and runtime checks to execute as SYSTEM in session 0, and accepts only the fixed `locate`, `state`, `count`, `count-state`, `distinct-count`, and `distinct-count-state` modes; distinct counts additionally require one of three allowlisted identity attributes. It cannot take the interactive foreground or execute arbitrary expressions. The same production history scenario passed with evidence key `703df348f29d23c4845063aa9a34c72fcd3e1ecf5e41fa23d7e90c4cfb5e7ed3`. Its four action durations fell from approximately 7.2 seconds each to 5.1-5.3 seconds each while preserving the original budgets and semantic oracles.

Versioned guest action transactions are also operational. For production actions, the runner resolves the public target and then uses one PowerShell Direct invocation whose bounded guest script obtains an independent CDP `before`, submits exactly one guarded input request, evaluates the declared fixed completion condition, and returns the final CDP `after`. The input and CDP agents remain separate processes and protocols, so transaction batching does not let the input implementation manufacture its own oracle. Request and receipt schemas are strict; the completion timeout must leave four seconds inside the end-to-end action budget for target resolution and transport. The production history scenario passed with evidence key `5f30cc7fb7d6dbf83600f7ba26135768a183314278be08bce2852b9e1a5ee159`; its four actions completed in 3.1-3.4 seconds each, another approximately 36% reduction from persistent CDP alone, with unchanged `0 -> 1 -> 0 -> 1` kernel/DOM evidence. At that stage, direct authenticated guest-to-host artifact transfer and complete production bundles remained incomplete.

The first production multi-object workflow is now operational as `core.selection.clipboard-delete-multi-bond.production`. Starting from a blank document, guarded OS input draws two bonds, activates box selection, selects all, copies through the real Windows clipboard, pastes to four bonds, selects all again, deletes atomically, and verifies undo/redo. Candidate SHA-256 `dea620b455daeb253c4141e2e999eae376c5b53ecd0f7a7034795db401ea58f6` passed with evidence key `ff6fc4512e70cee602ce87118087408de8634076731bc6c9b82c9ca98519695c`; independent DOM evidence recorded `0 -> 1 -> 2 -> 4 -> 0 -> 4 -> 0`, the second full selection expanded the overlay from 21 to 39 primitives, and the final state had neither stale selection overlay nor unexpected diagnostics. This scenario found and fixed a real revision-stable interaction-cache defect: select-all updated the engine selection and selection bounds after paste, but rendered a cached empty overlay. The action protocol now also requires a four-second target-resolution/transport reserve inside every end-to-end action budget, so a failed completion returns its precise diagnostic before the outer budget expires.

Mixed molecular/graphic coverage is now operational as `core.selection.clipboard-delete-mixed-bond-arrow.production`. From a blank document, real guarded mouse input creates one single bond and one solid arrow; real keyboard input then selects both object classes, copies and pastes through the Windows clipboard, deletes the four resulting objects, undoes the deletion, and redoes it. Candidate SHA-256 `dea620b455daeb253c4141e2e999eae376c5b53ecd0f7a7034795db401ea58f6` passed with evidence key `285e571b80b2442751b0cd74933e07b805bbe457405618c05ac689485ef02acf`. Independent receipts recorded bond primitives `0 -> 1 -> 2 -> 0 -> 2 -> 0`, distinct arrow identities `0 -> 1 -> 2 -> 0 -> 2 -> 0`, selection overlays of 21 and 39 primitives, and no final overlay or unexpected diagnostics. The platform now has a strict `dom-distinct-count` oracle that counts allowlisted `data-object-id`, `data-node-id`, or `data-bond-id` identities instead of mistaking one object's multiple SVG primitives for multiple objects. Its first run failed closed with evidence key `2aa13393f23d7fe85b0513aaf276b9b35593a45ae3159fb62ab6c5b2daccd893` because the scenario used the static markup label `Arrow`; the runtime correctly exposes the active default property name `Small arrow head`, so the locator was corrected without weakening uniqueness or visibility requirements. This closes one bond/arrow select-all and clipboard cell only; grouped/nested and region/additive selection are tracked by their own scenarios, while other object classes, partial applicability, and cross-boundary clipboard cells remain explicit gaps.

Depth-two mixed grouping is now operational as `core.group.nested-mixed-clipboard.production`. Guarded real mouse input creates a molecule and two arrows from a blank document; real `Control+G` first groups the molecule with an arrow and then groups that mixed group with the second arrow. The selected nested root is copied and pasted through the Windows clipboard, both outer roots are selected and batch-ungrouped with `Control+Shift+G`, and the one-transaction change is undone and redone. Candidate SHA-256 `50b3b36ffbdc95eebf1588ec80a7fe258ab7681ec094925ce6db49b400b3a308` passed with evidence key `0bca63951877cdaf30d3452ef11bde6d43a29f5aa355f8cd950da837ee5b638e`. Independent structural evidence recorded group identities `0 -> 1 -> 2 -> 4 -> 2 -> 4 -> 2`, nested group identities `0 -> 1 -> 2 -> 0 -> 2 -> 0` around copy/ungroup/history, bonds `0 -> 1 -> 2`, arrows `0 -> 1 -> 2 -> 4`, a 39-primitive child selection immediately after batch ungroup, and zero transient overlay after history restoration. Two general product defects were fixed: group selection omitted selected molecule objects and failed when a selected ancestor also had selected descendants, while ungroup classified molecule children as generic graphics; and incremental rendering appended aggregate group primitives without reparenting old DOM, duplicating bonds and erasing observable hierarchy. The engine now canonicalizes to outermost selected objects and restores complete molecule selection, while the viewer rebuilds only the affected hierarchy subtrees with recursive object wrappers and refreshed primitive indexes instead of refreshing the whole document. The first production attempt preserved failure evidence `11ca9037e59119b4ff727fd887701e951abca6f1e7b338894f7a92f022e05773` for the duplicate-DOM defect. A second attempt preserved `c1cd08ca88422434273126c74959514071e1ac6ed3dacd1f0aae8156b0ab2161`; all structure oracles passed, but it exposed an incorrectly timed scenario oracle, which was moved to verify child selection immediately after ungroup and the established history contract of clearing transient selection after redo. This closes only depth-two molecule/arrow grouping via shortcuts and same-document clipboard; context-menu grouping, deeper levels, other object classes, transforms, cross-group movement, locked/hidden members, save/reopen, and format boundaries remain explicit work.

Production artifact export is now operational and fail-closed. The runner requires driver payload descriptors in `chemsema.gui.run.v1`, binds their SHA-256 values into the evidence key, and stores the report plus every payload as immutable content-addressed objects referenced by a schema-validated manifest. The production driver captures a final PNG, complete public DOM, public runtime/window/render state, browser-level performance trace, and WebView log inside the guest. The guest hashes each bounded payload before transfer; the coordinator validates paths and identities, copies files through an authenticated PowerShell Direct session, and independently rechecks host size and SHA-256 before the evidence writer accepts them. Truncation, missing bytes, hash drift, artifact-name collisions, trace data loss or overflow, collection failure, environment/diagnostic collection failure, and shutdown failure all fail the run; artifacts already collected before a later failure are retained with `failure`, never `sample`, policy. A first diagnostic run, evidence key `0d6d2cb791f607294a9d66102e243e5ab3e61b72c39b3b3b467fab06ac261165`, exposed a 2 MiB DOM truncation and is not qualifying evidence. The uncompromised console-channel rerun correctly failed with evidence key `c5cc3386f0fa512d8ae77dec8e3f0edf5dcab4e144e297e83b88286a2456bb55` after the complete 6.9 MiB DOM exceeded its 120-second transport budget. The direct-transfer replacement passed the same real production scenario against candidate `50b3b36ffbdc95eebf1588ec80a7fe258ab7681ec094925ce6db49b400b3a308` with evidence key `66e2698709c04187c8f238efb1d07fccf435133f2f21ecf47c61a5a7410807df`: all six manifest objects (report plus five payloads) rehashed exactly, the 6,912,415-byte DOM ended in the complete closing document tags, no payload reported truncation, and the VM returned to `Off`. The impact graph selected all six platform scenarios and they were requalified after the final fail-closed retention update: the Playwright scenario passed with six payloads including a trace at `7cb8c2ac4c676d35ff13d1de9fac983c8062d0cb08fc24071a3fbe9bb386ea86`; the five production scenarios passed with complete five-payload bundles at `a36d4e3a1740091726b35671cab3e7f738f2ab2b99f9fd779bfecbe8de55f0f6`, `fda0802af92104c966e33b147404639a9ee3fbe7bc5c8ec7970e8b593e28616e`, `f22ee3b2dd1937028bd4acd6478bbe82215c650992bc7a96e5e23b73338818bf`, `6fe73889df63f26ffdf7fd23748ad95c0a33e1d1b52bb2f1c5c680e2adc6ccce`, and `fe24b02fb018f88276b064606b112b293c9602b49204dbfe595bb551643b4d8d`. Every manifest object rehashed exactly, no payload was truncated, no scenario reported diagnostics, and every production run returned the VM to `Off`.

Production CDP is now enforced as an observer rather than an input path or privileged product API. Regression tests reject `window.__chemsemaDebug` and in-memory CCJS export in the production script. The performance trace starts before the first user action, uses CDP `Tracing.start` with `ReturnAsStream`, waits for `Tracing.tracingComplete`, rejects reported data loss, reads through bounded `IO.read`, and ends during final collection. Ordinary receipts remain limited to 20 seconds; only the fixed `artifact-export` mode receives bounded 90-second guest and 110-second host deadlines. The retained failure keys `6db5152b88ff38708d62a73dc569b72894d5ebf220bd53dca93e4ef1fe607a49` and `08c40cf795f3b9e01986e914f2c6a0c0fb6e0d2a495a543a5a1aad22d18454aa` exposed final-EOF empty-array handling, while `b01117987dcfa6496e2e09cfd74e13c9f0476c40872a4e90e3c57458fa6bb960` completed all 19 complex actions but exceeded the old finalization deadline; later passes did not overwrite those failures. The final six-scenario closure passed at `21dd6e3d70e92825a25c02774744a98a11d0bb01e065f77c4d09999df5b28b72`, `01b1344d09fb2129e04b92e072a77a2c8e3c7097e6947d6ddfa9a6507d4ef71f`, `54a1b9801adcfb7c546818ec066c33f3305f967226eaddbcbfb078c1a5a432e1`, `262350489416c5048f0e6de1d7119797595c50181317781b67b92a61729446d4`, `998f2ed8e4130b03bad700ce7f819ff36e75ffe6727db8bea7de138931c939d1`, and `c41db8db42e66fcc29f67b13d5a5c196af061fc4cca9745fd93c4142ff5a01de`. All payloads rehashed exactly and none was truncated; the five production traces parsed with 30,328, 46,504, 120,892, 150,253, and 226,318 events, and every production run returned the VM to `Off`. Real GUI save/external-parse/reopen chemical-document evidence, video, crash dumps, and action-level screenshots remain explicit work.

The current entry points are:

```powershell
npm run gui-platform -- list
npm run gui-platform -- validate tests/gui/scenarios/core/draw-single-bond.json
npm run gui-platform -- audit
npm run gui-platform -- impact viewer/app.js
npm run gui-platform -- worker host-attest
npm run gui-platform -- worker start
npm run gui-platform -- worker guest-attest
npm run gui-platform -- worker prepare-guest
npm run gui-platform -- worker install-agent
npm run gui-platform -- worker configure-desktop-baseline
npm run gui-platform -- worker agent-attest-service
npm run gui-platform -- worker stop
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond.json --driver fake
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond.json --driver playwright-browser
npm run gui-platform -- run tests/gui/scenarios/core/draw-single-bond-production.json --driver production-black-box
npm run gui-platform:test
```

### Native save, independent file oracle, discard, reopen, and continued editing

The production scenario `core.document.save-open-roundtrip.production` now passes the complete real-user path without product-state injection: draw one bond; open the Windows Save As dialog; focus, select, and type the bounded output path through real mouse/keyboard input; save; independently transfer and validate the CCJS file; draw a second unsaved bond; close the dirty tab; click `Don't Save`; reopen the saved file through the Windows Open dialog; prove that only the saved bond returns; and draw a second bond again. Native filename fields are addressed by exact UI Automation ids and control/class constraints. While a modal native dialog is visible, the driver uses only UI Automation and guarded OS input; after the exact top-level dialog disappears, it reads the dedicated interactive session to refresh foreground geometry without forcing activation, then resumes WebView observation.

The qualifying run used production candidate SHA-256 `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07` and evidence key `e71f891c964b3cecc4ac0d1f456e4b870c72ed27ebfa001068aed7aaf4d019d6`. The saved 2,787-byte CCJS had SHA-256 `fa5d1660cc988f21c357884f1da0e8e7eee2b9b18930e46b6a1c1268b988372b`; release CLI `inspect` plus chemical validation independently proved CCJS 0.2, two nodes, one bond, one molecule, one object, zero validation issues. The final public DOM proved two bonds after reopening and continued editing. Evidence contains the saved CCJS, its independent inspect report, final screenshot/state/complete DOM, WebView log, and a valid nonempty 3,138,150-byte gzip performance trace. Host/guest file sizes and SHA-256 values matched, and the VM returned to `Off` with zero assigned memory.

This path exposed and fixed a product defect: choosing `Don't Save` closed a dirty saved tab but left its exact recovery-journal entry, so reopening the unchanged disk file resurrected discarded edits. The lifecycle now compacts that document's recovery record before closing on discard, with a focused journal regression test. Failed exploratory runs remain retained as failure evidence; their gates were not weakened or rerun-to-green.

The impact-selected qualification closure for commit `7a529cd` passed all seven registered scenarios. The Playwright browser scenario produced evidence key `9dc9e46476f82cb3f0d626af73f987eec42a9a4a2d862f9626cfba2c34f5589f`. The production single-bond, undo/redo, multi-bond clipboard/delete, mixed bond/arrow, nested mixed-group, and save/open scenarios produced evidence keys `91f33166dd237b2cd9d9532a76f72f29f80076bb9013b8a2f2cf7ebcd93e3cc7`, `b41dbaa41388d5d935f0bd1216178ed952238f518d6fe5b834b8b9bcce067302`, `0104c9132065c15056602beddea1a2a7beb880a6158c22e1a758f2b293cfd830`, `a3b36dcf1e77516c0c602ce5be6be8aa59a996cf14679a9a86e0513fb60ce6a4`, `c7073a695b43a246e30af1873a225b8dfa880822f1396287e4192533fc580fe6`, and `e71f891c964b3cecc4ac0d1f456e4b870c72ed27ebfa001068aed7aaf4d019d6`. All 67 actions completed, every manifest object rehashed exactly, no artifact was truncated, no run reported diagnostics, and each production run returned the VM to `Off` with zero assigned memory. The impact graph now explicitly maps the GUI input-agent crate and `Cargo.lock` to the GUI platform and maps the recovery-journal regression to document I/O, eliminating uncertainty-driven full expansion for those known paths.

Region and additive mixed-object cardinalities are now operational as `core.selection.region-additive-mixed-cardinalities.production`. Guarded real mouse input creates one molecule and two arrows, then exercises an empty box selection, a one-arrow box selection, a heterogeneous molecule-plus-arrow box selection, and a Shift-held box drag that adds the third object. Deletion effects and undo/redo provide semantic oracles rather than relying on overlay appearance alone: the one-object path leaves one arrow and one bond, the heterogeneous two-object path leaves one arrow and no bond, and the additive many-object path leaves neither arrows nor bonds before undo restores both arrows and redo removes them again. Candidate SHA-256 `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07` passed all 21 actions and four final oracles with evidence key `ef69a3036502697a5960df64886aee9f7ac6c73e6e01f7d65e4683aaeb658b36`; all six manifest objects rehashed exactly, diagnostics were empty, and the VM returned to `Off` with zero assigned memory. The first production attempt failed closed at the initial unmodified click and retained evidence key `d5450e515986e4d69b4349f35b2258087d620f61178f8dc610493d6f708e9758`: PowerShell had normalized an absent `modifiers` field into a one-element null array. Empty modifiers are now filtered before allowlist validation, while nonempty modifiers remain unique, bounded to three, and restricted to Shift, Control, and Alt across schema, driver, coordinator, and native input-agent layers.

The impact-selected qualification closure for commit `1f65db5` passed all eight registered scenarios without uncertainty expansion. The Playwright browser scenario produced evidence key `4fb19ef38e44d8a0441bb70bcc09f12bfa95fdf10f2124553c39acbb5870ceca`. The production single-bond, undo/redo, multi-bond clipboard/delete, mixed bond/arrow, nested mixed-group, save/open, and region/additive-cardinality scenarios produced evidence keys `388e4efea4cfde977dbc46a4262852b6a128f0370379b53295e93479854b89c2`, `7ffae8c76778a0fa9bc42b8bf83341f706ebf2da0d9e2f67792356cbdbe70092`, `beb3627670e4c9c9c4101a12dd342b1da58effbe11602ca66e40a84325059368`, `683e2398d4d7abd74e9a173ba617fd723b04fb8e63c7439271a911ef1e4cf719`, `d72eb7c63641ce93a985306e18722f5049f7dd9bd1788f4afe3a28516cd670c6`, `43c53632e254d14b94dd003d4c7c01755eb1189c43fda5d93ef6a4b85c0afbb1`, and `ef69a3036502697a5960df64886aee9f7ac6c73e6e01f7d65e4683aaeb658b36`. All 88 actions and 26 final oracles passed; all 51 manifest objects rehashed to their declared sizes and SHA-256 values; no artifact was truncated and no run reported diagnostics. The production VM finished `Off` with eight configured processors and zero assigned memory.

Cross-document clipboard coverage is now operational as `core.clipboard.cross-document-mixed.production`. Real guarded input creates and selects a molecule plus arrow in the source document, copies through the Windows clipboard, creates a second document through the public `New file` tab button, proves the active destination is blank, and pastes into that independent document. Receipts record document tabs `1 -> 2`, destination bonds `0 -> 1`, and destination arrow identities `0 -> 1`; five final oracles prove two tabs, exactly one active tab, and the exact mixed-object destination counts. Candidate `4a7dcc47e2f4469f5aed4f7963c6a7506fa413f7c20879984a9179632ebb6b07` passed all 11 actions with evidence key `f20a768332299ecc0d642ac3e4605607f9271749fdb119d14f3be32fd5b7d835`. All six manifest objects rehashed exactly, diagnostics were empty, and the VM returned to `Off` with zero assigned memory. Browser/desktop transfer, Office, paste-special, and independently opened document boundaries remain explicit work.

Production action transport now reuses one bounded host broker and one authenticated PowerShell Direct `PSSession` per scenario. The nine-scenario qualification for `01c3525` first failed closed on mixed-object redo with evidence key `fe6b0ad2f3f4bee8d4643d5e59706fa7e286a29a9775adc0e397e48f67ed76cf`: the product correctly changed bonds `2 -> 0`, but the per-action temporary host process/session returned at 12.003 seconds and violated the unchanged 12-second end-to-end budget. Raising the budget was rejected. The broker accepts only bounded JSONL requests for the allowlisted `action-transaction` operation, converts a strict parameter allowlist to named splatting, and closes before VM shutdown; guest input and CDP observation remain separate. An initial broker integration failure was retained at `7ae540b9b1a3dd0bf8c4955dba3f17ae03504611779b1442bf0a91f2e705828e` and fixed by replacing positional array splatting with validated named parameters. The exact mixed scenario then passed at `636d57f48d5c343f468cda12d5a719374194edbaf58e0871de08021ece342fa4`: after the one-time 4.292-second session-establishment action, the remaining actions completed in 2.077–2.301 seconds and redo completed in 2.301 seconds.

The broker qualification closure passed all nine registered scenarios: browser `576267d3c1695ad40e619b59e82a41d0bd9b59f21f4f0a1a2d1324c656a868ba`; production single-bond `8f3050f525f7d5fca641b49a02ebacc99ffa9d7fd4940d23f68bd06b7e1808d9`, history `9e2d1f03b021f6b4c9eca52ae0328228c09075568938cc622a32a426c02a971a`, multi-bond `50c9042b06a256d8add5c140fb088ae5f50fd2c659a406ea6feb9d21711b37fd`, mixed `636d57f48d5c343f468cda12d5a719374194edbaf58e0871de08021ece342fa4`, nested group `fd1b4d24dc89b84c07bf32afbcdedbeb014bea5202dc7339d654ea973447cacd`, save/open `7111c295b24643fba15762b04b9af4cdbda244ea5da70408087f3200642d9212`, region/additive `86360d45c8cd3199286d8a8b513746b05636d67c28068877f00ce04880f04628`, and cross-document clipboard `2eb71278cea5ec472ba0c2ce6947470487b0a308e9b57c59ab962dfe80c25552`. All 99 actions and 31 final oracles passed; all 57 manifest objects rehashed exactly; diagnostics and truncation were zero. The VM finished `Off` with zero assigned memory. The impact graph now distinguishes production transport, production driver, browser driver, and platform-test sources so transport-only changes invalidate the eight production scenarios without needlessly rerunning the browser scenario.

### Locked-object contract and partial-applicability deletion

CCJS 0.2 already defines `locked` as whether an object is editable, but the desktop previously had no public Lock/Unlock operation and the generic delete path did not enforce that contract. Locking was therefore only partially observed by a few interactions such as image dragging and spectra handling. The engine now exposes one public context-menu `Lock`/`Unlock` command, records the change as one undoable command, maps selected molecule nodes and bonds to their owning molecule, and treats an object as effectively locked when either it or any ancestor is locked. Generic deletion now canonicalizes the selection, removes only effectively editable members, leaves locked members untouched, makes an all-locked deletion a no-op, and records a mixed deletion as one undo/redo transaction.

The production scenario `core.selection.locked-partial-delete.production` exercises the public path with real guarded input: draw two arrows, region-select the first, right-click that exact rendered entity, choose `Lock`, reopen the menu and observe `Unlock`, select the locked and unlocked arrows together, delete, undo, redo, and finally prove that the surviving arrow still exposes `Unlock`. Candidate SHA-256 `7dfee3e1fe541336f9809d46a299febc6cfa1d965314beea02b2e69269d66124` passed all 15 actions and 3 final oracles with evidence key `32e99f68f555666037266c87e983b9b204a52ea46db2f008973781c7e70dc24a`; its six manifest objects rehashed exactly and diagnostics were empty. This work also added a semantic `entity-id` production target over stable rendered object wrappers, strict public context-menu completion checks, and stricter scenario Schema branches that reject irrelevant target/value fields instead of allowing them to fail later in the action protocol.

Two failed runs remain preserved as diagnostic evidence. `8b43321a71b1cc6ccb77199340041a16ae41c08922ad6d7494776aecd85a3658` proved that the first semantic locator incorrectly required `data-object-type`, while incremental object wrappers correctly exposed `data-object-id` plus `data-renderer`; the generalized locator now uses that stable contract. `0378f5c5eaa815b798f76fa5414851742974a7226e9471f0460d513104ded95a` proved that an `actionable` completion carrying a target had slipped through the scenario Schema but was correctly rejected by the stricter transaction Schema; the scenario now uses a DOM completion and both Schemas reject such irrelevant combinations.

The affected qualification closure then passed ten reports: browser single-bond `8eb5a5061cdfd43c8c2ff7bc76024fc56466f88e66912c55c739f32173216b77`; production single-bond `b2180ec1aec77a0937456da1fcf212cdc99e89ec64aca26f5d8c2282f2ce4725`, history `a9c90df5943ed038b095f408f704e02f7548e0c4a50f60ace0769b0637be281d`, multi-bond clipboard/delete `febb2f752c619f0043a54ee242c39c10ad1bcb47fda351f348fbe83358bef6ec`, mixed bond/arrow `03a5a3eef6a8123884e769e63ed68a40ea7a004e83fa9d8087a6966dcb3ba724`, nested grouping `9ae1dfae810db87d204f13694877d9c575820b5d831f205bc16076e9b9f9ccc6`, save/reopen `e170d036ccad86b12716f1d65c74ef937109c6e909353fcf7ae679ad99d8111c`, region/additive selection `6ccd952adb99d57956f8b0bd3828c02c5018422a2e341d3a87648353fb55794f`, cross-document clipboard `d0885adeee208a740d9db9c0d9c4bfa255f1761dba16a6a7d345512bfb32bd00`, and the new locked partial-delete scenario above. Every action completed, every oracle passed, diagnostics were empty, and all 63 manifest objects were independently re-read and matched their declared sizes and SHA-256 values. The VM finished `Off` with zero assigned memory.

This closes only the tested locked/unlocked two-arrow partial-delete cell. It does not claim complete lock coverage: grouped and nested selections, hidden or overlapping objects, other object classes and properties, locked ancestors through the GUI, save/reopen and format boundaries, and large-document behavior remain explicit registry work. The coverage registry consequently records this capability as partially migrated rather than complete.

### Locked partial transforms and world-geometry evidence

Selection transforms now project the visible selection onto effectively editable members before building move, rotate, resize, arrange, or command targets. A locked molecule contributes no movable nodes, and an object locked directly or by an ancestor contributes no object transform. The document preview renderer applies the same effective-lock rule, so its low-latency DOM transform cannot visually move a member that the engine correctly excluded. Pointer down, move, and up are serialized for an active press; a regression test proves that an immediate move/up waits for asynchronous gesture setup instead of racing past it.

The production protocol adds `entity-rect-deltas`, a general completion oracle that observes one to sixteen stable rendered entity ids before and after the same guarded OS-input transaction. It derives document-world rectangles by transforming all four local `getBBox()` corners through `documentContent.getCTM().inverse() * entity.getCTM()`. This removes viewBox/root camera changes while retaining nested and object transforms. Expectations explicitly declare `stationary` or `moved` plus a bounded world-unit tolerance; receipts preserve screen rectangles, world rectangles, maximum deltas, and the per-entity verdict.

The 18-action scenario `core.selection.locked-transform.production` draws two arrows, locks the first through the public context menu, selects both, drags, undoes and redoes, clears selection, publicly unlocks the first arrow, then repeats drag/undo/redo with both editable. Candidate SHA-256 `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` passed with evidence key `0d4567affec5d8661ce1940d93a8b53c2f19665c7f58d6ed426ce914bd37dbb1`. During the locked drag and both history transitions, `obj_line_1` moved exactly 0 world units while `obj_line_2` moved about 38.25; after Unlock, both objects moved about 33.00 during drag, undo, and redo. All six manifest objects independently matched their declared size and SHA-256, diagnostics were empty, and the VM returned to `Off` with zero assigned memory.

Four failed runs remain diagnostic evidence rather than being hidden by the final pass. `8f9e1c432aacc35e6ae76a6068e2334d6c0d1e3cdff9e955c68baa4c944886ac` showed that screen rectangles confound object movement with viewBox changes. `4b9a39118f40ac244f636b48a5a19b098ea554a8fe0c0f869467adbe10c1294b` and `9e114a0dc3e4361d69b09fa2e6ec1a6d967599bdeca40d23f10ec9512f895cba` showed that plain `getBBox()` omits element transforms and that fast input could overtake asynchronous pointer-down setup. `f31bef1e689f724bdc473c29a63995e985eb4baddd5ca901709837ee08b3e2ff` finally proved from the DOM that both objects received the same committed preview transform, exposing the frontend effective-lock mismatch after the backend filter was already correct.

This closes only pointer movement, Unlock, and history for a two-arrow mixed-applicability selection. Engine paths for rotation, resize, and arrangement now enforce the same projection, but they do not yet have equivalent production GUI scenarios. Molecules, text, shapes, groups and locked ancestors; property editors and other commands; save/reopen and format boundaries; complex/large/xlarge documents; long sequences and endurance remain explicit incomplete cells. The registry therefore adds `capability.selection.transform-partial` as partially migrated, not complete.

The final impact-selected closure for candidate `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` passed all eleven registered scenarios with no uncertainty expansion: browser single-bond `a037d110e20ae9f63392292f56efd4109827ead8478ad8f128b54ddecde2ec6c`; production single-bond `cfaac5ef5a8db226154ea2fcd9f87e3531b269022f669a561afabee3308d2ec1`, history `44549b59e7745bd5aba2ed002d1895f27b03589462545c200662c868cde0140e`, multi-object clipboard/delete `6f667b79643666b414e059549b614f01222cb854665d43d25afc6452c780bc99`, mixed bond/arrow `8673c04af5ec75c01c9ef2b26cdf2f4751ba512340e41f3cda22189392164f7a`, region/additive `140916d4a2aec4521bbebb6fe3118fedfdce0f1c3978dcd33d31e7f65dd8cd4b`, locked partial-delete `e169a4df9a441803700b74358d3e9b6a122e95a1acb67b83b05e208bb300b7fa`, locked partial-transform `d9e6adfcd62e8546d57cd9e39cb512546f97da84ece62473f68937582638f8d3`, cross-document clipboard `b85d94199ac85d4bcab68f5b95f46d1d29bcfcb2a350d67ab6d2ff0d67e99deb`, nested grouping `b4a7423528da7e97e95d3dd0493de55b10decaddc303cea9043962d0cf3c04a7`, and save/open `9a22648023ba54cfb382c34c520840241f8d4608fa7037276684b9290ed8d36f`. All 132 actions and 37 final oracles passed with zero diagnostics. All 69 manifest objects were independently read as UTF-8 where applicable and matched their declared sizes and SHA-256 values; the production VM finished `Off` with zero assigned memory.

The cross-type production scenario `core.selection.locked-molecule-arrow-transform.production` extends the contract to a GUI-created molecule and arrow. It locks the molecule through the public context menu, proves the molecule's aggregated world geometry remains at exactly 0 movement while the arrow moves about 43.50 world units across drag/undo/redo, applies public Unlock, and then proves both semantic entities move about 43.50 across the second drag and history roundtrip. Candidate `22294e1dfccd1460b8c97a408c7b2f13ebabb713e703afb3427a922c04f61e5d` passed all 19 actions and four final oracles with evidence key `3bd51ce9eafcb7318d5ed51a1089913dfb19d419a0c5daffe83c54f6153a7aa3`; all six manifest objects rehashed exactly, diagnostics were empty, and the VM returned to `Off` with zero assigned memory.

Semantic `entity-id` observation now supports both a visible scene-object render root and a molecule represented by multiple primitives carrying the same object id. Locating input prefers a visible render root and otherwise chooses one stable visible primitive. Geometry observation prefers a visible render root and otherwise unions every visible primitive's screen and relative-CTM world bounds. Horizontal or vertical SVG lines are actionable when either axis has nonzero extent; empty incremental-render sentinel groups never shadow real primitives. Retained failures document each invalid assumption: `8688518ca0d26b01ccfa3f7f78fb203ff5d6c0993e0d3e79e3818439eae86279` required a `data-renderer` wrapper; `1f8cf81a246759d2966a16612d856a626b0783ca5a276dbbaf467dd213f0804c` treated a horizontal line's zero-height rectangle as invisible; `2045c9cac396906170ee97d0c31a9a500225bd47ac8c454c07194efc3ee2539d` exposed the scenario's incorrect arrow id after molecule ids had been allocated; and `29674d04c1c5b171e78e6b89491ab2f18c81f1cc19eb8cfd9e1f4a3fcf432fa4` showed an empty render sentinel hiding the real molecule primitives during postcondition observation.

The resulting twelve-scenario impact closure passed with browser evidence `7c91004b393e8f2dcbf24860c33df09e7f4299a25077e7e3cf93c462b6195504`; production single-bond `735526d363cd02ac5edba8f917536459d82c50a667d6897a823cf96f1088350f`, history `e74b877c11f94d36dc39d2e68e8ed7b77a13fa5b534b02f4630c50b35063db74`, multi-object `447138916016ccad629b012190b0e33b299a16e92ac5560da63dc1419758397a`, mixed bond/arrow `2243dfa3bd1e78bc5be1640ea27f00b6f38bef735849d31c8a0c66359a4bcf70`, region/additive `700efe10f693d8d39cf3f076b62b6882a63890f4b75b63f0d387dfc8bfb89fc7`, locked molecule/arrow transform `12696f5863b77aff23b6218b8d7fca81130cba648b2ff92eee2d7dec68473f20`, locked partial-delete `64a704e5ea8cbe4efd0cddf583f4e2272c0e7e28acf606d7069097a8eca47ce7`, locked arrow transform `4d5a08ca15d41666160f727ba9e2507867eadd8b3b6bb9c1cafa28a6dc6b149f`, cross-document clipboard `bd4d652f589d173bcdd1cf9c6e58e8f46a23a1737a3a1f2cd03d17082ded802b`, nested grouping `8f6e6d7d1e1e6ad6e81f0e2814bcf45c9ca1e7fa49e0e92356e7f100b399bb7c`, and save/open `32b67e2ec93b63538830ec71b2b6b10170731414afda857176383301df76619f`. All 151 actions and 41 final oracles passed with zero diagnostics. All 75 manifest objects independently matched their declared size and SHA-256; the VM finished `Off` with zero assigned memory.

### Atomic interaction and descendant transforms for locked ancestor groups

The production scenario `core.group.locked-ancestor-transform.production` creates two arrows through real mouse input, groups them, locks the parent through the public `Lock` menu, creates one editable root object, and selects both roots. Across the locked drag, undo, and redo, descendants `obj_line_1` and `obj_line_2` each move exactly 0 world units while root object `obj_line_4` moves about 33.000. After clearing selection, the scenario reacquires the locked parent through either visible descendant and invokes public `Unlock`; all three objects then move about 33.000 across the second drag and its undo/redo roundtrip. Candidate SHA-256 `01bba532076bffbef1770be96c5f5a17abb080b5e6654c3b7dadd7d7ecf4b6ec` passed all 22 actions and four final oracles with evidence key `7fb2ace1b8e9df6cf76742f1c0de6dbcaaaec66d7b6264c457cde5d03acb9ddb`; five artifacts were retained, diagnostics were zero, and the VM finished `Off` with zero assigned memory.

The scenario found and fixed two product defects plus one test-platform defect. A selected group's bounding-box center can be empty, while context hit testing previously demoted a visible descendant to an ordinary child. The engine now promotes descendant context hits to the selected ancestor group and treats any locked ancestor group as an atomic object: ordinary click or right-click on a descendant selects the locked parent, so the user can reliably reach `Unlock`; nested ancestry is searched outward. Production semantic targeting now chooses a genuinely rendered descendant as a group's input point while world-geometry observation still aggregates the complete semantic entity. Failure evidence `a80bcb249bce75a47fb6eeb0c8f262af5f0cc8454ee134a18c5d84041e5f2bb0` and `1a78ad44008926f9bd60a4357077ac61ad34914c622a781b02713759e8cf86b8` record the empty bounding-box-center menu failure. `70be641a752d6d38df0e0ea94907bb870758eceb59fcd9f50ab83de874a3f473` proves that locked descendant transforms already held while the parent could not yet be reacquired for Unlock. The registry now contains 26 entries and 13 scenarios with zero unexplained gaps or warnings. Nested locked ancestors, partial descendant selection, deletion and property commands, save/reopen, and format boundaries remain explicit work.

The final 13-scenario impact closure passed for candidate `01bba532076bffbef1770be96c5f5a17abb080b5e6654c3b7dadd7d7ecf4b6ec`: browser single-bond `6af74e1e86c62d1d265807f7eeef8272030b046f6e2331e5b078bf9dc773a329`; production single-bond `a31f33f17939db7faf9f76c36e96a25cdb63d53bcf3f694e7b2836eb3cd8b1c1`, history `6f7f5ed13262d4820b377aa0a24191dd1a17ca9ed024159c62ce0dd776ef8911`, multi-object `e4eac55c2e18683c1fc8f3ca75381508a67eedda9195980d9b3eda9e9afcd189`, mixed bond/arrow `0b0837dff78e9ecdbe64eeef65526e5ba7f8c4b6e7a713fabffe0c428debb91c`, region/additive `fd2f6583ec3c387c4aa578ae1f834ac92107cab8b36b4b0ec693a6c227d3260f`, locked partial-delete `a92eae8291430061e739e7045dfd23a5a2b5441393e6838fbcda38044704b392`, locked arrow transform `836cf34e89f7e351f2c0d69a841db0570931bf0e962c51d84120ec2fc4c34500`, locked molecule/arrow transform `dc28bf3f51884bf3a3f5ef23d042f41ec9a47de72cda33547f40656b53798fa0`, cross-document clipboard `d5abbc23b4cc0cbeee1f901af62ba8a24d130f788133794db341195a21fa3ef7`, nested grouping `02a79f2a623543d0b6e52caeb9c42695e8b749b4002bedc21488724627f35a47`, save/open `78cd47f8ed7ca87ae3c7994971bfbab11be6322bc2061630a7f071d785f2295d`, and locked ancestor grouping `7fb2ace1b8e9df6cf76742f1c0de6dbcaaaec66d7b6264c457cde5d03acb9ddb`. All 173 actions and 45 final oracles passed with zero failed actions, failed oracles, or diagnostics. All 81 manifest objects were independently reread and matched their declared size and SHA-256. Every production report named the single candidate above. The VM finished `Off` with zero assigned memory and no remaining `vmwp` process.

### Multi-arrow public property editing and history

The production scenario `core.arrow.multi-property-history.production` creates two arrows with real mouse gestures, selects both, opens the public nested context menus, changes both line styles from Plain to Bold, and changes both end heads from Full to Half Arrow at End Left. Every state check reopens the menu and observes engine-regenerated `aria-checked` items, so toolbar state cannot masquerade as a document mutation. The scenario then undoes the endpoint transaction, independently undoes the bold transaction, redoes both in order, and proves the uniform two-object state after every transition. Final candidate SHA-256 `14797d15edb9058edbb873b31dce86aae765da6d9b35b3b6f4d224e7b6cbc0ef` passed all 26 actions and three final oracles with evidence key `584c8477696aecfcc948f2d72d2de5269f6da9c037c870a2bb210ce4b1707750`; five artifacts were retained, diagnostics were zero, and the VM finished `Off` with zero assigned memory and no `vmwp` process.

The first retained failure, candidate `cee51fe5277eb0142884b9338db7ad577ae331c6aea94ced71720d37cd53f99a` with evidence key `258d50478daf7c59ae1a940558fe9a4ae42f71e517a995e6c9de861e14d8cf65`, stopped at the first property observation and exposed two product defects. `Select All` included the invisible zero-node, zero-bond default editor molecule in an otherwise graphic-only document, incorrectly turning two arrows into a heterogeneous selection. Even a genuinely homogeneous multi-selection was routed to a generic menu that omitted shared line and arrow properties. The engine now excludes that default empty editor placeholder from Select All without excluding authored empty molecule objects that carry logical semantics, and preserves Line Style plus Arrowheads for homogeneous line selections, with Line Style retained for homogeneous curves. A kernel regression proves both arrow payloads, uniform checked-menu projection, and the two independent undo/redo transactions. Context-menu items also have stable accessible names and submenu semantics instead of names polluted by checkmarks and disclosure glyphs.

The registry now contains 27 entries and 14 scenarios with zero unexplained gaps or warnings. This closes only solid-arrow line style and end-head editing for a uniform pair in runtime history. Arrow variants, sizes, curvature, tail styles, no-go marks, color, mixed-value selections, locked and grouped targets, persistence/import/export boundaries, and large-document behavior remain explicit work; `capability.arrow.properties` therefore remains partially migrated.

The subsequent exact impact closure retained one infrastructure-boundary failure instead of retrying it away. In `core.selection.locked-molecule-arrow-transform.production`, evidence `902b90dc493f66132ab3fdf967ca348273649780bc0b2cce3ff9d340620f505a` completed the functional geometry transition for the final redo but exceeded the old 12,000 ms outer action budget at 12,011 ms while collecting guarded input, target, completion, and evidence state. The six `entity-rect-deltas` actions now have a 30,000 ms total transaction budget, while their exact `stationary`/`moved` completion predicates still have the original 8,000 ms functional timeout. This separates transport overhead from product responsiveness without weakening any geometry oracle. The qualified rerun passed all 19 actions and four final oracles with evidence `0790503e0d7fe80f7161fc4c27a82fa9cd56df44925a0d52c08e4868c0b433bb`.

The final 14-scenario closure passed for production candidate `14797d15edb9058edbb873b31dce86aae765da6d9b35b3b6f4d224e7b6cbc0ef`: browser single-bond `ba2b839556e213c1e45932b58d523499ad7e0deff4db45725252422ecdfb8ec4`; production single-bond `fea57c19e35171685f1e322518b3f0b6df1e763ae8dcba6842f30c3f7b367f5b`, history `d630f18cbf5908dd822bcd6c82d86764a960300bb0c48bda63e8d1c9ba09b0b9`, multi-bond clipboard/delete `3c84fcf13b35147361ddb0617096176798afc78bb2f425a8f279fdb1b8f5e7c9`, mixed bond/arrow clipboard/delete `82a8cb941265feb258c588dc8c2eeac78d70bf2e27abc88b93c100854e9f5bc3`, region/additive selection `4de4068153284818234a9774a663ff238f6a5d43ef33a5670d76048aecedb198`, multi-arrow properties `584c8477696aecfcc948f2d72d2de5269f6da9c037c870a2bb210ce4b1707750`, locked partial deletion `f93110161d822178d7cc32f79b10f1db7027abd278a3f5e835cec07159de291a`, locked arrow transform `884c675301e94be15886b0a807cba385627e5269df664ec6f06ba0724c016d23`, locked molecule/arrow transform `0790503e0d7fe80f7161fc4c27a82fa9cd56df44925a0d52c08e4868c0b433bb`, cross-document clipboard `1edf81ab0877082eb5c04d485db9b398b6cc67444ecc89f5c684031af2483178`, nested mixed group clipboard `6a29f8c74ee363f4e6db5cbbea088027da718a398fecaf90e68af85ded330f60`, locked ancestor group transform `3585f6d357f1769b3f34654c863f82cd6038527a370196de04919bc7cdfb14ac`, and save/open roundtrip `fe7ff9a16aa9329af3957814d619ae9c0b097a4638e1cafdccf4506044933689`. An independent UTF-8 audit found 14 unique scenario/driver pairs, 199 completed actions, 48 passing final oracles, zero failed statuses/actions/oracles/diagnostics, and one shared candidate across all 13 production reports. All 87 manifest objects were reread and matched their declared byte size and SHA-256. The isolated Windows GUI VM finished `Off` with no remaining `vmwp` process.

### Locked mixed-property edits and source-bound candidates

Arrow property mutation now applies the same effective-lock contract as deletion and transforms. A mixed selection filters locked objects both when the command is recorded and again when it executes. Line Style modifies only editable lines. Arrowhead context actions use the new partial `apply-arrow-endpoints` transaction, so changing one head or tail preserves every object's unrelated variant, head size, curvature, no-go mark, bold state, and opposite endpoint instead of copying the first selected arrow's complete toolbar state. The kernel regression covers a locked/plain arrow plus an editable arrow across Bold, head/tail patches, and two independent undo/redo transactions.

The production scenario `core.arrow.locked-mixed-properties.production` draws two arrows, locks the first through the public menu, applies Bold and Half Arrow at Start Left to the mixed selection, proves the engine reports intentionally unchecked mixed values, independently undoes and redoes both transactions, and finally opens each arrow's menu separately. The locked arrow remains Plain with a Full end and no start head; the editable arrow is Bold with the same Full end plus the new half-left start. The first retained run, evidence `79e93a91316a78663057ed9eb38ab62c74d4104080668ececf2054bb7d06710d`, correctly failed after 12 completed actions because the VM had installed stale candidate `14797d15edb9058edbb873b31dce86aae765da6d9b35b3b6f4d224e7b6cbc0ef`. Candidate `e3da58661616e2708d95ddacb7e500f98455f98e0b1ef198312be00742c41e2a` then passed, but was superseded when the platform itself was hardened. The final source-bound candidate `008a2e13dc651603b14ff098c7aad412ff4c73d05fb12cce17375327d9e2a7cf` passed all 30 actions and three final oracles with evidence `25397a0116a195d28099bf49b9e495cc7000053e67f18ddcd35b3b533b7e9d3d`; diagnostics were empty, all six manifest objects rehashed exactly, and the VM returned `Off` with no `vmwp` process.

Production GUI runs no longer silently reuse an arbitrary release executable. Both desktop build entrypoints now emit `chemsema.desktop-candidate-build.v1`, binding the executable SHA-256 to a deterministic content hash over the current product source closure. Before guest preparation, the Hyper-V coordinator independently verifies that the manifest exists, the candidate bytes still match, and all source inputs still produce the recorded closure hash. Missing manifests, replaced binaries, and source drift fail before the VM starts. The live negative check rejected the pre-contract binary with “Desktop candidate build manifest is missing”; the replacement manifest bound 430 source files at `80d9983e7581cbf0441dd7d5b94dd6829b311cd23d1cdc8ec159d312f52cc6bf` to the final candidate above. Unit tests independently reject both candidate-byte and source-closure drift.

The final source-bound 15-scenario closure passed on that candidate: browser single-bond `7bc7badc2ec28f244d0a4b85c6c6e6146db4f075532d82af9ef0a884c8c35c78`; production single-bond `d1f63a5fdddab753021ff4e612a2ddff5ad54cb3438830d7a887b28c9f498f45`, history `7e6a909b4520c3ee399276ab387e4ca1a442f7cdfff7f41801517e21c042b6ea`, multi-bond clipboard/delete `ad0f01d4be2a17081ee48d4bec3bc1d6e54a36c5723073d0c77329d9d92056f4`, mixed bond/arrow clipboard/delete `76a7db99ad037e1b1bcd25158b76b041ca0d49179ea89828401c344cd98e1780`, region/additive selection `e14598555661582b0d05a197befb4d3c7190f2f683fb30d16808b784641f889d`, uniform multi-arrow properties `3fb577ae2dc88cec9a6f31fb71931cf84dd0fc14c847e55a91edcad97e7821dd`, locked mixed-arrow properties `25397a0116a195d28099bf49b9e495cc7000053e67f18ddcd35b3b533b7e9d3d`, locked partial deletion `26620a9ce21d1e0f82eb084a1e77f773c60db3b73f3a214b7094796ffa3428ac`, locked arrow transform `2b2ef902d84f06a92fab6e9684e0499b94502719e109203c5c8a38a5ef8dbe90`, locked molecule/arrow transform `448c0127cba66712fcd679025d21d57dfa6ffab0ce98baaba7c6b271d018febd`, cross-document clipboard `cd12ec5635e231b49bde878ddf35e261999280755fc523c0b7ab4c5476f39c67`, nested mixed-group clipboard `25365f374e58fbc30d8cdb2e6514afdba2bfbb2b0c32353f295a382729c137c0`, locked ancestor-group transform `6a57719ca371088ab4dfc93428ea3035ba8dee6321898bf4054f16bc0f34cf78`, and save/open roundtrip `76b5c7dd93dbcda36a1fd518d12ae2e6bab046afa5245da8dc2502689ec2acb5`. An independent UTF-8 audit verified 15 unique scenario/driver pairs, 229 completed real-input actions, 51 passing final oracles, zero failed statuses/actions/oracles/diagnostics, and the same candidate SHA-256 across all 14 production reports. All 93 manifest objects were reread and matched their declared byte size and SHA-256. The isolated Windows GUI VM finished `Off` with no remaining `vmwp` process. The registry now has 27 entries and 15 scenarios with zero unexplained gaps or warnings; this is the exact affected closure for this unit, not completion of the still-explicit object/property, import/export, large-document, environment-matrix, soak, or 1,000-repeat goals.

### Complete public arrow-property patches and saved-document qualification

Existing arrows now expose public context-menu editing for Arrow Type, Arrow Head Size, Arrow Curve, No-Go Mark, Arrowheads, Line Style, and Color. These controls no longer copy a temporary toolbar snapshot over every selected arrow. The engine records a field-level `apply-arrow-style-patch` command whose optional variant, size, curve, head, tail, bold, and no-go members preserve all unmentioned payload fields and each object's style reference. Effective locks are projected both when a command is recorded and when it executes. The legacy complete-style command remains available for explicit presets, while ordinary property actions use partial patches. A related product defect was fixed in Line Style: toggling Bold used to change `arrowHead.bold` without recomputing the size-dependent `length`, `centerLength`, and `width`, so a Large arrow was later projected as Small. Bold/plain transitions now infer the current size before changing weight and regenerate the corresponding dimensions.

The 33-action production scenario `core.arrow.property-matrix-persistence.production` draws two arrows, selects both, and uses only public menus to apply Mirrored Curved, red, Large, 120 degrees, Double Slash, Half Arrow at Start Left, and Bold. It then reopens the menu and requires all seven independent checked states, saves through the native Windows dialog, transfers the CCJS through the bounded SHA-verified channel, and independently verifies both objects as `curved-mirror`, curve `120`, length `45`, full head, half-left tail, bold, hash no-go, and `#ff0000`. Candidate SHA-256 `fa647f870acba6ac919799033c0ca4b333f3208abcc8c294baff49c82e736844`, bound to source closure `355705fe5c303c22ac6b184dadcc68fb4b7555ce71964d964d30efdf667678ca`, passed all actions and three final oracles with evidence key `a899bc39e0a14a04b95e1679ece905a88d676be06cf050fed8b13ac293543589`.

The retained failures materially strengthened the platform rather than being retried away. Evidence `764525c0fcff481ab71430fd7b64a5cb1b4bb9142822471f5f1e4c0b5aa673ee` showed that the center of a curved SVG group's bounding box can be empty; `entity-id` input now chooses the longest visible `document-graphic` geometry, takes its real path midpoint with `getPointAtLength`, transforms it through `getScreenCTM`, and uses the old semantic rectangle only as a bounded fallback. Evidence `1ea3d4def822fd5b8c93974aab6c9c7defb52e718dc83bcdd47e54c9478e123d` exposed inconsistent selector bounds between scenario and action schemas; both sides and the host/guest bridges now enforce one 2,048-character limit. Evidence `f612d0f568566c5122ec9fd84c4af092fe9ef4ef01da70c41e836ea1f9352106` found the Large-to-Small Bold defect above. Evidence `bc426d72876a34da500f59e4f28a2bd881b8f0a7175f4af6393172806b5f7bf7` showed that a 45-second outer save budget could expire while native-dialog dismissal, foreground re-attestation, transfer, and inspection were still bounded independently; document-save actions now require at least 90 seconds without weakening their exact completion checks. Evidence `895395a5e72dcdf144fc012b5536c1776a22af1041ec0abbbdb6ec6526fd7105` proved that unconditional chemical validation incorrectly rejects a graphic-only document's empty editor molecule. Every saved CCJS still receives structural validation; chemical validation is additionally required whenever a nonempty molecular graph exists. Failed inspection now retains both the SHA-verified CCJS and a bounded diagnostic artifact instead of discarding the decisive bytes.

The exact impact graph selected all 16 registered scenarios because shared engine, viewer, production transport, driver, and schema surfaces changed. All 16 passed: 262 of 262 real-input actions and 54 final oracles completed with zero diagnostics. All 15 production reports bind the single candidate above; the browser report is independently content-addressed. An independent audit reread all 101 objects in the 16 evidence manifests, totaling 138,173,922 bytes, and every declared size and SHA-256 matched. The registry now has 27 entries, 16 scenarios, zero unexplained gaps, and zero warnings. The isolated VM finished `Off`, with eight configured processors, zero assigned memory, and no `vmwp` process. This closes the tested multi-arrow property/persistence matrix, not the still-explicit remaining object/property values, import/export families, complex/large/xlarge documents, environment matrix, endurance, or 1,000-repeat demonstration goals.

## 20. Upstream foundations

- Tauri WebDriver and WebdriverIO: <https://v2.tauri.app/develop/tests/webdriver/>
- Tauri WebDriver CI: <https://v2.tauri.app/develop/tests/webdriver/ci/>
- Playwright WebView2: <https://playwright.dev/docs/webview2>
- Playwright actions and auto-waiting: <https://playwright.dev/docs/input>, <https://playwright.dev/docs/actionability>
- Playwright traces: <https://playwright.dev/docs/trace-viewer>
- Playwright visual comparisons: <https://playwright.dev/docs/test-snapshots>
- Playwright ARIA snapshots: <https://playwright.dev/docs/aria-snapshots>
- Microsoft UI Automation testing: <https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-usefortesting>
- Windows UI automation and input: <https://learn.microsoft.com/en-nz/windows/apps/dev-tools/winapp-cli/ui-automation>
- Hyper-V PowerShell Direct: <https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/powershell-direct>

These tools supply drivers and evidence. They do not replace ChemSema's scenario model, chemistry oracles, demo qualification, or release responsibility.
