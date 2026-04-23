/* tslint:disable */
/* eslint-disable */

export class WasmEngine {
    free(): void;
    [Symbol.dispose](): void;
    clearInteraction(): void;
    documentJson(): string;
    constructor();
    pointerDown(x: number, y: number): void;
    pointerMove(x: number, y: number): void;
    pointerUp(x: number, y: number): void;
    renderListJson(): string;
    setTool(active_tool: string, bond_variant: string): void;
    stateJson(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmengine_free: (a: number, b: number) => void;
    readonly wasmengine_clearInteraction: (a: number) => void;
    readonly wasmengine_documentJson: (a: number) => [number, number, number, number];
    readonly wasmengine_new: () => number;
    readonly wasmengine_pointerDown: (a: number, b: number, c: number) => void;
    readonly wasmengine_pointerMove: (a: number, b: number, c: number) => void;
    readonly wasmengine_pointerUp: (a: number, b: number, c: number) => void;
    readonly wasmengine_renderListJson: (a: number) => [number, number, number, number];
    readonly wasmengine_setTool: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly wasmengine_stateJson: (a: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
