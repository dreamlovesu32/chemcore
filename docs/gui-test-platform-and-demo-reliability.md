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

Mixed molecular/graphic coverage is now operational as `core.selection.clipboard-delete-mixed-bond-arrow.production`. From a blank document, real guarded mouse input creates one single bond and one solid arrow; real keyboard input then selects both object classes, copies and pastes through the Windows clipboard, deletes the four resulting objects, undoes the deletion, and redoes it. Candidate SHA-256 `dea620b455daeb253c4141e2e999eae376c5b53ecd0f7a7034795db401ea58f6` passed with evidence key `285e571b80b2442751b0cd74933e07b805bbe457405618c05ac689485ef02acf`. Independent receipts recorded bond primitives `0 -> 1 -> 2 -> 0 -> 2 -> 0`, distinct arrow identities `0 -> 1 -> 2 -> 0 -> 2 -> 0`, selection overlays of 21 and 39 primitives, and no final overlay or unexpected diagnostics. The platform now has a strict `dom-distinct-count` oracle that counts allowlisted `data-object-id`, `data-node-id`, or `data-bond-id` identities instead of mistaking one object's multiple SVG primitives for multiple objects. Its first run failed closed with evidence key `2aa13393f23d7fe85b0513aaf276b9b35593a45ae3159fb62ab6c5b2daccd893` because the scenario used the static markup label `Arrow`; the runtime correctly exposes the active default property name `Small arrow head`, so the locator was corrected without weakening uniqueness or visibility requirements. This closes one bond/arrow mixed-object cell only; grouped/nested, other object classes, additive/region selection, partial applicability, and cross-boundary clipboard cells remain explicit gaps.

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
