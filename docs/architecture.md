# chemsema Architecture

## Purpose

`chemsema` is intended to be a long-lived chemistry document core shared by:

- browser hosts
- desktop hosts
- import/export pipelines
- editor, CLI, and agent tools

The project optimizes for the final core architecture from the beginning.

## Core Principles

### 1. Platform-independent core first

The document model is the primary asset.

The core must define:

- document structure
- object identity
- coordinate systems
- style references
- grouping and z-order
- chemistry-bearing objects
- rendering contracts

Web and desktop are hosts for the same core.

### 2. Separate chemistry semantics from document semantics

Chemical structure data and document object data solve different problems.

Chemical semantics include:

- atoms
- bonds
- stereochemistry
- molecular abbreviations
- `molblock2d`

Document semantics include:

- object positioning
- grouping
- style references
- text boxes
- arrows
- visibility
- z-order
- transforms

The architecture keeps those concerns in separate models.

### 3. Stable file format, optimized runtime model

The file format is a persistence contract.

The runtime scene model is an execution model.

They should be close, with execution-oriented differences where useful. The file format
should be explicit, versioned, and migration-friendly. The runtime model should
be suitable for:

- hit testing
- partial redraw
- selection
- command execution
- undo/redo

### 4. Renderer backends are replaceable

The first backend may be web-based, and the drawing API should remain independent
of DOM, React, or any browser-only primitive.

The long-term backend set may include:

- SVG
- Canvas / WebGL
- native desktop rendering
- export renderers for PDF / SVG

### 5. Import is a first-class subsystem

`chemsema` must be able to ingest legacy formats, especially CDXML.

Imports should target the `chemsema` document model directly.

## Layered Structure

The intended system is split into layers.

### Layer A: File Format

The persisted `chemsema` document.

Responsibilities:

- versioning
- object serialization
- style table serialization
- object relationships
- metadata

Non-goals:

- runtime caching
- UI-only transient state

### Layer B: Runtime Document Model

The in-memory document graph.

Responsibilities:

- object lookup by id
- parent-child relationships
- object typing
- transforms
- style resolution

This layer should be deterministic and suitable for backend-agnostic rendering.

### Layer C: Scene and Geometry Services

Shared logic that both web and desktop hosts need.

Responsibilities:

- world coordinates
- local coordinates
- bounding boxes
- z-order walking
- hit testing
- transform composition
- visibility checks

### Layer D: Renderer Interface

A backend-agnostic draw contract.

The interface should support at least:

- begin/end frame
- push/pop transform
- draw text
- draw line/path
- draw molecule
- apply style

The interface remains independent of backend primitive storage and drawing.

### Layer E: Host Adapters

Platform-specific implementations.

Examples:

- web viewer
- desktop shell
- CLI exporter

Hosts reuse the core document model.

### Layer F: Quality And Qualification Plane

GUI testing is an independent quality plane across every host, not an ad hoc viewer script. Versioned scenarios execute through browser, real Tauri/WebView2, Windows UIA/real-input, and final-installer black-box drivers, with interaction, chemistry, rendering, accessibility, persistence, and runtime-quality oracles. Real user input must cover every public feature, actual creation/drawing and every public property of every object type, and `0/1/2/many` homogeneous and heterogeneous multi-object operations; internal APIs and prebuilt documents cannot substitute for this evidence. The Test ABI exists only in test builds; production candidates receive separate black-box qualification. See the [GUI Test Platform and Demo Reliability Architecture](./gui-test-platform-and-demo-reliability.md).

## Why CDXML Parsing Lives In The Core

CDXML is currently the main import path because it provides a practical bridge
from ChemDraw-based workflows into a `chemsema` document.

The active CDXML parser and writer live in the Rust engine:

- [crates/chemsema-engine/src/cdxml.rs](../crates/chemsema-engine/src/cdxml.rs)

Their role is:

- parse CDXML into native `ChemSemaDocument` objects and molecule fragments
- preserve enough import metadata to retain source drawing options
- export the current document back to ChemDraw-readable CDXML

## Current Document Milestone

The current persistence contract is CCJS v0.2: scene entities are flat, `hierarchy` is the single-ownership tree index, `relations` carry typed cross-object semantics, and the runtime spatial grid is derived from the snapshot. `.ccjs` is complete UTF-8 JSON; `.ccjz` is the deterministic, hashed, chunk-readable Container v1. Local interaction uses Document Patch and crash recovery uses a separate Journal.

See [format-v0.2.md](./format-v0.2.md) for the field contract and the [Chinese architecture rationale](./ccjs-architecture-and-format-rationale.zh-CN.md) plus [stability contract](./ccjs-v0.2-stability-architecture.zh-CN.md) for design decisions, implementation status, and remaining stable-release gates. v0.1 remains migration input only and no longer describes the current architecture.

Long-term work still includes broader ChemDraw coverage, rich query chemistry, advanced polymer semantics, multi-page layout, and collaboration. Editor visible-region CCJZ loading, undo-preserving hydration, unchanged-entry reuse, and browser Zip64 are implemented. Large arrays remain in HDF5/Zarr-class specialist formats and are connected to document semantics through CCJZ attachments.
