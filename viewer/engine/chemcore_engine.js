/* @ts-self-types="./chemcore_engine.d.ts" */

export class WasmEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmengine_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    activeArrowEditDegrees() {
        const ret = wasm.wasmengine_activeArrowEditDegrees(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {string} variant
     * @param {string} head_size
     * @param {string} curve
     * @param {string} head_style
     * @param {string} tail_style
     * @param {string} no_go
     * @param {boolean} bold
     * @returns {boolean}
     */
    applyArrowEndpointOptionsToSelection(variant, head_size, curve, head_style, tail_style, no_go, bold) {
        const ptr0 = passStringToWasm0(variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(head_size, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(curve, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(head_style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(tail_style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(no_go, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyArrowEndpointOptionsToSelection(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, bold);
        return ret !== 0;
    }
    /**
     * @param {string} variant
     * @param {string} head_size
     * @param {boolean} head
     * @param {boolean} tail
     * @param {boolean} bold
     * @returns {boolean}
     */
    applyArrowOptionsToSelection(variant, head_size, head, tail, bold) {
        const ptr0 = passStringToWasm0(variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(head_size, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyArrowOptionsToSelection(this.__wbg_ptr, ptr0, len0, ptr1, len1, head, tail, bold);
        return ret !== 0;
    }
    /**
     * @param {string} style
     * @returns {boolean}
     */
    applyBondStyleToSelection(style) {
        const ptr0 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyBondStyleToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} kind
     * @returns {boolean}
     */
    applyBracketKindToSelection(kind) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyBracketKindToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} color
     * @returns {boolean}
     */
    applyColorToSelection(color) {
        const ptr0 = passStringToWasm0(color, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyColorToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} selection_json
     * @returns {boolean}
     */
    applyElementPaletteJson(selection_json) {
        const ptr0 = passStringToWasm0(selection_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyElementPaletteJson(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * @param {string} style
     * @returns {boolean}
     */
    applyHoveredBondStyle(style) {
        const ptr0 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyHoveredBondStyle(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} style
     * @returns {boolean}
     */
    applyLineStyleToSelection(style) {
        const ptr0 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyLineStyleToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} settings_json
     * @returns {boolean}
     */
    applyObjectSettingsDialogJson(settings_json) {
        const ptr0 = passStringToWasm0(settings_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyObjectSettingsDialogJson(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * @param {string} phase
     * @returns {boolean}
     */
    applyOrbitalPhaseToSelection(phase) {
        const ptr0 = passStringToWasm0(phase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyOrbitalPhaseToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} style
     * @returns {boolean}
     */
    applyOrbitalStyleToSelection(style) {
        const ptr0 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyOrbitalStyleToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} template
     * @returns {boolean}
     */
    applyOrbitalTemplateToSelection(template) {
        const ptr0 = passStringToWasm0(template, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyOrbitalTemplateToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} command
     * @returns {boolean}
     */
    applySelectionArrangeCommand(command) {
        const ptr0 = passStringToWasm0(command, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applySelectionArrangeCommand(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} payload_json
     * @returns {boolean}
     */
    applySelectionNumericDialogJson(payload_json) {
        const ptr0 = passStringToWasm0(payload_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applySelectionNumericDialogJson(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * @param {string} command
     * @returns {boolean}
     */
    applySelectionOrderCommand(command) {
        const ptr0 = passStringToWasm0(command, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applySelectionOrderCommand(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} style
     * @returns {boolean}
     */
    applyShapeStyleToSelection(style) {
        const ptr0 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyShapeStyleToSelection(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @param {string} session_json
     * @returns {boolean}
     */
    applyTextEdit(session_json) {
        const ptr0 = passStringToWasm0(session_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyTextEdit(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * @param {string} command
     * @param {string} value
     * @returns {boolean}
     */
    applyTextStyleToSelection(command, value) {
        const ptr0 = passStringToWasm0(command, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(value, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_applyTextStyleToSelection(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret !== 0;
    }
    /**
     * @param {string} kind
     * @returns {string}
     */
    arrowToolIconSvg(kind) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_arrowToolIconSvg(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    beginHoverArrowEdit(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_beginHoverArrowEdit(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    beginHoverShapeEdit(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_beginHoverShapeEdit(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} additive
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    beginSelectionMove(x, y, additive, alt_key) {
        const ret = wasm.wasmengine_beginSelectionMove(this.__wbg_ptr, x, y, additive, alt_key);
        return ret !== 0;
    }
    /**
     * @param {string} handle
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    beginSelectionResize(handle, x, y) {
        const ptr0 = passStringToWasm0(handle, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_beginSelectionResize(this.__wbg_ptr, ptr0, len0, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    beginSelectionRotate(x, y) {
        const ret = wasm.wasmengine_beginSelectionRotate(this.__wbg_ptr, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    beginTextEdit(x, y) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_beginTextEdit(this.__wbg_ptr, x, y);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string | undefined}
     */
    beginTlcSpotDragJson(x, y) {
        const ret = wasm.wasmengine_beginTlcSpotDragJson(this.__wbg_ptr, x, y);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @param {string} variant
     * @param {number} stroke_width
     * @param {number} bold_width
     * @returns {string}
     */
    bondToolIconSvg(variant, stroke_width, bold_width) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_bondToolIconSvg(this.__wbg_ptr, ptr0, len0, stroke_width, bold_width);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    canRedo() {
        const ret = wasm.wasmengine_canRedo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    canUndo() {
        const ret = wasm.wasmengine_canUndo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    centerSelectionOnPage() {
        const ret = wasm.wasmengine_centerSelectionOnPage(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {number} stroke_width
     * @returns {string}
     */
    chainToolIconSvg(stroke_width) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_chainToolIconSvg(this.__wbg_ptr, stroke_width);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    clearInteraction() {
        wasm.wasmengine_clearInteraction(this.__wbg_ptr);
    }
    /**
     * @returns {boolean}
     */
    clearSelection() {
        const ret = wasm.wasmengine_clearSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string | undefined}
     */
    clipboardDocumentJson() {
        const ret = wasm.wasmengine_clipboardDocumentJson(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @returns {string | undefined}
     */
    clipboardSelectionJson() {
        const ret = wasm.wasmengine_clipboardSelectionJson(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @param {string} current_color
     * @param {string} custom_colors_json
     * @returns {string}
     */
    colorDialogPaletteJson(current_color, custom_colors_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(current_color, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(custom_colors_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_colorDialogPaletteJson(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    contextHitTestJson(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_contextHitTestJson(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} hit_json
     * @param {boolean} has_paste
     * @returns {string}
     */
    contextMenuJson(hit_json, has_paste) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(hit_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_contextMenuJson(this.__wbg_ptr, ptr0, len0, has_paste);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    copySelection() {
        const ret = wasm.wasmengine_copySelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    cutSelection() {
        const ret = wasm.wasmengine_cutSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    deleteSelection() {
        const ret = wasm.wasmengine_deleteSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {Uint8Array}
     */
    documentCdx() {
        const ret = wasm.wasmengine_documentCdx(this.__wbg_ptr);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {string}
     */
    documentCdxml() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_documentCdxml(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    documentColorsJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_documentColorsJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    documentJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_documentJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    documentSdf() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_documentSdf(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    documentStylePreset() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_documentStylePreset(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    documentSvg() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_documentSvg(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    elementPaletteJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_elementPaletteJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} command_json
     * @returns {string}
     */
    executeCommandJson(command_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(command_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_executeCommandJson(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    expandLabelsInSelection() {
        const ret = wasm.wasmengine_expandLabelsInSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    finishHoverArrowEdit(x, y, alt_key) {
        const ret = wasm.wasmengine_finishHoverArrowEdit(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    finishHoverShapeEdit(x, y, alt_key) {
        const ret = wasm.wasmengine_finishHoverShapeEdit(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    finishSelectionMove(x, y, alt_key) {
        const ret = wasm.wasmengine_finishSelectionMove(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    finishSelectionResize(x, y) {
        const ret = wasm.wasmengine_finishSelectionResize(this.__wbg_ptr, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    finishSelectionRotate(x, y, alt_key) {
        const ret = wasm.wasmengine_finishSelectionRotate(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string | undefined}
     */
    finishTlcSpotDragJson(x, y) {
        const ret = wasm.wasmengine_finishTlcSpotDragJson(this.__wbg_ptr, x, y);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @returns {boolean}
     */
    groupSelection() {
        const ret = wasm.wasmengine_groupSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    hasClipboard() {
        const ret = wasm.wasmengine_hasClipboard(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string}
     */
    historyJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_historyJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    hoverArrowAction(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_hoverArrowAction(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string}
     */
    hoverShapeAction(x, y) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_hoverShapeAction(this.__wbg_ptr, x, y);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    interactionRenderListJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_interactionRenderListJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    joinSelection() {
        const ret = wasm.wasmengine_joinSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string}
     */
    lastCommandResultJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_lastCommandResultJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {Uint8Array} cdx
     */
    loadDocumentCdx(cdx) {
        const ptr0 = passArray8ToWasm0(cdx, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_loadDocumentCdx(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} cdxml
     */
    loadDocumentCdxml(cdxml) {
        const ptr0 = passStringToWasm0(cdxml, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_loadDocumentCdxml(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} json
     */
    loadDocumentJson(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_loadDocumentJson(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} sdf
     */
    loadDocumentSdf(sdf) {
        const ptr0 = passStringToWasm0(sdf, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_loadDocumentSdf(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    constructor() {
        const ret = wasm.wasmengine_new();
        this.__wbg_ptr = ret;
        WasmEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {string}
     */
    objectSettingsDialogJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_objectSettingsDialogJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} template
     * @param {string} style
     * @param {string} phase
     * @returns {string}
     */
    orbitalToolIconSvg(template, style, phase) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(template, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(phase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_orbitalToolIconSvg(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            deferred4_0 = ret[0];
            deferred4_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    pasteClipboard() {
        const ret = wasm.wasmengine_pasteClipboard(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {string} json
     * @returns {boolean}
     */
    pasteClipboardJson(json) {
        const ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_pasteClipboardJson(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     */
    pointerDown(x, y, alt_key) {
        wasm.wasmengine_pointerDown(this.__wbg_ptr, x, y, alt_key);
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     */
    pointerMove(x, y, alt_key) {
        wasm.wasmengine_pointerMove(this.__wbg_ptr, x, y, alt_key);
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     */
    pointerUp(x, y, alt_key) {
        wasm.wasmengine_pointerUp(this.__wbg_ptr, x, y, alt_key);
    }
    /**
     * @param {string} request_json
     * @returns {string}
     */
    previewTextEditLayout(request_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_previewTextEditLayout(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @param {string} session_json
     * @returns {string}
     */
    previewTextRuns(session_json) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(session_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_previewTextRuns(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    redo() {
        const ret = wasm.wasmengine_redo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {string} scope
     * @returns {string}
     */
    renderBoundsJson(scope) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(scope, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_renderBoundsJson(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    renderListJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_renderListJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} label
     * @returns {boolean}
     */
    replaceHoveredEndpointLabel(label) {
        const ptr0 = passStringToWasm0(label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_replaceHoveredEndpointLabel(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @returns {bigint}
     */
    revision() {
        const ret = wasm.wasmengine_revision(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * @param {number} degrees
     * @returns {boolean}
     */
    rotateSelectionDegrees(degrees) {
        const ret = wasm.wasmengine_rotateSelectionDegrees(this.__wbg_ptr, degrees);
        return ret !== 0;
    }
    /**
     * @param {number} percent
     * @returns {boolean}
     */
    scaleSelection(percent) {
        const ret = wasm.wasmengine_scaleSelection(this.__wbg_ptr, percent);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    selectAll() {
        const ret = wasm.wasmengine_selectAll(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} additive
     */
    selectAtPoint(x, y, additive) {
        wasm.wasmengine_selectAtPoint(this.__wbg_ptr, x, y, additive);
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} additive
     * @returns {boolean}
     */
    selectComponentAtPoint(x, y, additive) {
        const ret = wasm.wasmengine_selectComponentAtPoint(this.__wbg_ptr, x, y, additive);
        return ret !== 0;
    }
    /**
     * @param {string} points_json
     * @param {boolean} additive
     */
    selectInPolygon(points_json, additive) {
        const ptr0 = passStringToWasm0(points_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmengine_selectInPolygon(this.__wbg_ptr, ptr0, len0, additive);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} x1
     * @param {number} y1
     * @param {number} x2
     * @param {number} y2
     * @param {boolean} additive
     */
    selectInRect(x1, y1, x2, y2, additive) {
        wasm.wasmengine_selectInRect(this.__wbg_ptr, x1, y1, x2, y2, additive);
    }
    /**
     * @returns {string}
     */
    selectionBoundsJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_selectionBoundsJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    selectionChemistrySummaryJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_selectionChemistrySummaryJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    selectionContainsPoint(x, y) {
        const ret = wasm.wasmengine_selectionContainsPoint(this.__wbg_ptr, x, y);
        return ret !== 0;
    }
    /**
     * @param {string} kind
     * @returns {string}
     */
    selectionNumericDialogJson(kind) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_selectionNumericDialogJson(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} variant
     * @param {string} head_size
     * @param {string} curve
     * @param {string} head_style
     * @param {string} tail_style
     * @param {string} no_go
     * @param {boolean} bold
     */
    setArrowEndpointOptions(variant, head_size, curve, head_style, tail_style, no_go, bold) {
        const ptr0 = passStringToWasm0(variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(head_size, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(curve, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(head_style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(tail_style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ptr5 = passStringToWasm0(no_go, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len5 = WASM_VECTOR_LEN;
        wasm.wasmengine_setArrowEndpointOptions(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5, bold);
    }
    /**
     * @param {string} variant
     * @param {string} head_size
     * @param {boolean} head
     * @param {boolean} tail
     * @param {boolean} bold
     */
    setArrowOptions(variant, head_size, head, tail, bold) {
        const ptr0 = passStringToWasm0(variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(head_size, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.wasmengine_setArrowOptions(this.__wbg_ptr, ptr0, len0, ptr1, len1, head, tail, bold);
    }
    /**
     * @param {string} kind
     */
    setBracketOptions(kind) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_setBracketOptions(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {boolean} enabled
     * @returns {boolean}
     */
    setChemicalCheckForSelection(enabled) {
        const ret = wasm.wasmengine_setChemicalCheckForSelection(this.__wbg_ptr, enabled);
        return ret !== 0;
    }
    /**
     * @param {string} preset
     */
    setDocumentStylePreset(preset) {
        const ptr0 = passStringToWasm0(preset, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_setDocumentStylePreset(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} symbol
     * @param {number} atomic_number
     */
    setElementOptions(symbol, atomic_number) {
        const ptr0 = passStringToWasm0(symbol, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_setElementOptions(this.__wbg_ptr, ptr0, len0, atomic_number);
    }
    /**
     * @param {string} template
     * @param {string} style
     * @param {string} phase
     * @param {string} color
     */
    setOrbitalOptions(template, style, phase, color) {
        const ptr0 = passStringToWasm0(template, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(phase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(color, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        wasm.wasmengine_setOrbitalOptions(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
    }
    /**
     * @param {string} kind
     * @param {string} style
     * @param {string} color
     */
    setShapeOptions(kind, style, color) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(color, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        wasm.wasmengine_setShapeOptions(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
    }
    /**
     * @param {string} kind
     */
    setSymbolOptions(kind) {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_setSymbolOptions(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} template
     */
    setTemplate(template) {
        const ptr0 = passStringToWasm0(template, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_setTemplate(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} active_tool
     * @param {string} bond_variant
     */
    setTool(active_tool, bond_variant) {
        const ptr0 = passStringToWasm0(active_tool, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(bond_variant, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.wasmengine_setTool(this.__wbg_ptr, ptr0, len0, ptr1, len1);
    }
    /**
     * @param {string} kind
     * @param {string} style
     * @returns {string}
     */
    shapeToolIconSvg(kind, style) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(style, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_shapeToolIconSvg(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    stateJson() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.wasmengine_stateJson(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} kind
     * @returns {string}
     */
    symbolToolIconSvg(kind) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_symbolToolIconSvg(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} kind
     * @returns {string}
     */
    textFormatIconSvg(kind) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_textFormatIconSvg(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    textSymbolPaletteJson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_textSymbolPaletteJson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string | undefined}
     */
    tlcLaneGuideHitTestJson(x, y) {
        const ret = wasm.wasmengine_tlcLaneGuideHitTestJson(this.__wbg_ptr, x, y);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string | undefined}
     */
    tlcSpotHitTestJson(x, y) {
        const ret = wasm.wasmengine_tlcSpotHitTestJson(this.__wbg_ptr, x, y);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @param {string} custom_colors_json
     * @returns {string}
     */
    toolbarColorPaletteJson(custom_colors_json) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(custom_colors_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_toolbarColorPaletteJson(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    undo() {
        const ret = wasm.wasmengine_undo(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    ungroupSelection() {
        const ret = wasm.wasmengine_ungroupSelection(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    updateHoverArrowEdit(x, y, alt_key) {
        const ret = wasm.wasmengine_updateHoverArrowEdit(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    updateHoverShapeEdit(x, y, alt_key) {
        const ret = wasm.wasmengine_updateHoverShapeEdit(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    updateSelectionMove(x, y, alt_key) {
        const ret = wasm.wasmengine_updateSelectionMove(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {boolean}
     */
    updateSelectionResize(x, y) {
        const ret = wasm.wasmengine_updateSelectionResize(this.__wbg_ptr, x, y);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {boolean} alt_key
     * @returns {boolean}
     */
    updateSelectionRotate(x, y, alt_key) {
        const ret = wasm.wasmengine_updateSelectionRotate(this.__wbg_ptr, x, y, alt_key);
        return ret !== 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @returns {string | undefined}
     */
    updateTlcSpotDragJson(x, y) {
        const ret = wasm.wasmengine_updateTlcSpotDragJson(this.__wbg_ptr, x, y);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
}
if (Symbol.dispose) WasmEngine.prototype[Symbol.dispose] = WasmEngine.prototype.free;
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_9c75d47bf9e7731e: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./chemcore_engine_bg.js": import0,
    };
}

const WasmEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmengine_free(ptr, 1));

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('chemcore_engine_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
