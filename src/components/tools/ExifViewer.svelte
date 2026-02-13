<script>
    import { onMount } from "svelte";
    export let onBack;

    // Check if we are in Tauri
    const isTauri =
        typeof window !== "undefined" &&
        "current" in (window.__TAURI__?.os || {}); // robust check or just generic
    // @ts-ignore
    const invoke =
        window.__TAURI__?.core?.invoke ||
        (async () => ({ success: false, error: "Not in Desktop Mode" }));
    // @ts-ignore
    const open = window.__TAURI__?.dialog?.open || (async () => []);

    let selectedPath = "";
    let metadata = "";
    let isLoading = false;
    let error = "";

    async function selectFile() {
        if (!window.__TAURI__) {
            error = "This feature requires the Desktop Application.";
            return;
        }

        try {
            const file = await open({
                multiple: false,
                filters: [
                    {
                        name: "Images",
                        extensions: ["jpg", "jpeg", "png", "tiff"],
                    },
                ],
            });

            if (file) {
                selectedPath = file;
                extractMetadata();
            }
        } catch (e) {
            error = `Selection failed: ${e}`;
        }
    }

    async function extractMetadata() {
        if (!selectedPath) return;

        isLoading = true;
        error = "";
        metadata = "";

        try {
            const result = await invoke("extract_metadata", {
                path: selectedPath,
            });
            if (result.success) {
                metadata = result.data;
            } else {
                error = result.error || "Unknown error";
            }
        } catch (e) {
            error = `Extraction failed: ${e}`;
        } finally {
            isLoading = false;
        }
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>🔍 Metadata/EXIF Viewer</h3>
        <p class="description">
            Extraer metadatos ocultos (GPS, Dispositivo) de imágenes locales.
        </p>
    </div>

    <div class="control-panel">
        {#if !isTauri}
            <div class="warning-banner">
                ⚠️ Esta herramienta funciona mejor en la <strong
                    >App de Escritorio</strong
                >.
            </div>
        {/if}

        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div
            class="upload-zone"
            on:click={selectFile}
            role="button"
            tabindex="0"
        >
            {#if selectedPath}
                <div class="file-info">
                    <span class="icon">📄</span>
                    <span class="path">{selectedPath}</span>
                </div>
                <button class="btn-sm" on:click|stopPropagation={selectFile}
                    >Change File</button
                >
            {:else}
                <div class="placeholder">
                    <span class="icon-lg">📁</span>
                    <p>Click to open an image file</p>
                    <small>Supports JPG, PNG, TIFF</small>
                </div>
            {/if}
        </div>

        {#if selectedPath && !metadata && !isLoading}
            <button class="btn-primary full-width" on:click={extractMetadata}
                >Extract Metadata</button
            >
        {/if}
    </div>

    {#if isLoading}
        <div class="loader-container">
            <span class="loader">Loading...</span>
        </div>
    {/if}

    {#if error}
        <div class="error-msg">
            ❌ {error}
        </div>
    {/if}

    {#if metadata}
        <div class="results-area">
            <h4>Extraction Results</h4>
            <pre>{metadata}</pre>
        </div>
    {/if}
</div>

<style>
    .header {
        margin-bottom: 24px;
        text-align: center;
    }

    .description {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .upload-zone {
        border: 2px dashed var(--border-color);
        border-radius: 12px;
        padding: 32px;
        text-align: center;
        background: var(--bg-tertiary);
        cursor: pointer;
        transition: all 0.2s;
        margin-bottom: 20px;
    }

    .upload-zone:hover {
        border-color: var(--accent-color);
        background: rgba(0, 240, 255, 0.05);
    }

    .placeholder {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 10px;
        color: var(--text-secondary);
    }

    .icon-lg {
        font-size: 3rem;
        opacity: 0.7;
    }

    .file-info {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 10px;
        font-family: var(--font-mono);
        margin-bottom: 12px;
    }

    .results-area {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 16px;
        margin-top: 20px;
    }

    pre {
        white-space: pre-wrap;
        font-family: var(--font-mono);
        font-size: 0.85rem;
        color: var(--text-primary);
        max-height: 300px;
        overflow-y: auto;
    }

    .btn-primary.full-width {
        width: 100%;
        margin-top: 10px;
    }
</style>
