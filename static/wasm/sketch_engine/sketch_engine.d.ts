/* tslint:disable */
/* eslint-disable */

/**
 * JSON-encoded `AddEdgeRequest` → JSON-encoded `SketchCommandResult`.
 */
export function wasm_add_edge(json: string): string;

/**
 * JSON-encoded `AddPointRequest` → JSON-encoded `SketchCommandResult`.
 */
export function wasm_add_point(json: string): string;

/**
 * Returns engine version + build info — used for handshake checks.
 */
export function wasm_engine_info(): string;

/**
 * `{ "sketch": <SketchGraph> }` → JSON-encoded `ValidationResult`.
 */
export function wasm_validate_sketch(json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly wasm_add_edge: (a: number, b: number) => [number, number];
    readonly wasm_add_point: (a: number, b: number) => [number, number];
    readonly wasm_engine_info: () => [number, number];
    readonly wasm_validate_sketch: (a: number, b: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
