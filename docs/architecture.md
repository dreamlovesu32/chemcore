# chemcore Architecture

## Purpose

`chemcore` is intended to be a long-lived chemistry document core shared by:

- browser hosts
- desktop hosts
- import/export pipelines
- future editing tools

The project should not optimize for a temporary frontend. It should optimize for
the final core architecture from the beginning, even if implementation is slow.

## Core Principles

### 1. Platform-independent core first

The primary asset is the document model, not the first renderer.

The core must define:

- document structure
- object identity
- coordinate systems
- style references
- grouping and z-order
- chemistry-bearing objects
- rendering contracts

Web and desktop are hosts, not separate products.

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

The architecture should not collapse those concerns into a single object model.

### 3. Stable file format, optimized runtime model

The file format is a persistence contract.

The runtime scene model is an execution model.

They should be close, but they do not need to be identical. The file format
should be explicit, versioned, and migration-friendly. The runtime model should
be suitable for:

- hit testing
- partial redraw
- selection
- command execution
- undo/redo

### 4. Renderer backends are replaceable

The first backend may be web-based, but the drawing API should not assume DOM,
React, or any browser-only primitive.

The long-term backend set may include:

- SVG
- Canvas / WebGL
- native desktop rendering
- export renderers for PDF / SVG

### 5. Import is a first-class subsystem

`chemcore` must be able to ingest legacy formats, especially CDXML.

That means imports should target the `chemcore` document model, not an
intermediate UI state.

## Layered Structure

The intended system is split into layers.

### Layer A: File Format

The persisted `chemcore` document.

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

The interface should not assume how the backend stores or draws primitives.

### Layer E: Host Adapters

Platform-specific implementations.

Examples:

- web viewer
- desktop shell
- CLI exporter

Hosts should not redefine the document model.

## Why CDXML Parsing Lives In The Core

CDXML is currently the main import path because it provides a practical bridge
from ChemDraw-based workflows into a `chemcore` document.

The active CDXML parser and writer live in the Rust engine:

- [crates/chemcore-engine/src/cdxml.rs](../crates/chemcore-engine/src/cdxml.rs)

Their role is:

- parse CDXML into native `ChemcoreDocument` objects and molecule fragments
- preserve enough import metadata to retain source drawing options
- export the current document back to ChemDraw-readable CDXML

## First Milestone

The first meaningful milestone is not full editing.

It is:

1. `chemcore` file format v0.1
2. `chemcore` runtime model v0.1
3. native CDXML import/export through the Rust engine
4. a renderer backend that proves the model is sufficient

That milestone answers the most important question:

"Can the document model faithfully represent the kind of chemistry pages we need
to support?"

## Non-Goals of v0.1

The following are explicitly out of scope for the first format version:

- full ChemDraw feature parity
- rich query chemistry
- high-end polymer semantics
- complete reaction semantics
- multipage layout
- collaborative editing
- binary cache formats

The first version should instead optimize for clarity, stability, and
inspectability.
