<script>
    import { open } from "@tauri-apps/plugin-shell";

    export let onBack;

    export let imageUrl = "";

    // Engine Definitions
    const engines = [
        {
            name: "Google Lens",
            url: (img) =>
                `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(img)}`,
            icon: "🇬",
        },
        {
            name: "Yandex",
            url: (img) =>
                `https://yandex.com/images/search?rpt=imageview&url=${encodeURIComponent(img)}`,
            icon: "🇾",
        },
        {
            name: "TinEye",
            url: (img) =>
                `https://tineye.com/search?url=${encodeURIComponent(img)}`,
            icon: "👁️",
        },
        {
            name: "Bing",
            url: (img) =>
                `https://www.bing.com/images/search?view=detailv2&iss=sbi&form=SBIHMP&q=imgurl:${encodeURIComponent(img)}`,
            icon: "🇧",
        },
        {
            name: "SauceNAO",
            url: (img) =>
                `https://saucenao.com/search.php?db=999&url=${encodeURIComponent(img)}`,
            icon: "🎌",
        },
    ];

    // Open URL Handler
    async function openUrl(url) {
        try {
            await open(url);
        } catch (e) {
            console.warn("Shell open failed, reusing window.open", e);
            window.open(url, "_blank");
        }
    }

    async function search(engine) {
        if (!imageUrl) return;
        await openUrl(engine.url(imageUrl));
    }

    function searchAll() {
        if (!imageUrl) return;
        engines.forEach((engine) => search(engine));
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>🖼️ Búsqueda Inversa de Imágenes</h3>
        <p class="description">
            Busca dónde aparece una imagen en múltiples motores simultáneamente.
        </p>
    </div>

    <div class="control-panel">
        <label for="img-url">URL de la Imagen</label>
        <div class="input-group">
            <input
                id="img-url"
                type="text"
                bind:value={imageUrl}
                placeholder="https://ejemplo.com/foto.jpg"
                on:keydown={(e) => e.key === "Enter" && searchAll()}
            />
        </div>

        <div class="actions">
            <button
                class="btn-primary full-width"
                on:click={searchAll}
                disabled={!imageUrl}
            >
                🔍 Buscar en Todos
            </button>
        </div>
    </div>

    {#if imageUrl}
        <div class="engines-grid">
            {#each engines as engine}
                <button class="engine-card" on:click={() => search(engine)}>
                    <span class="icon">{engine.icon}</span>
                    <span class="name">{engine.name}</span>
                    <span class="arrow">↗</span>
                </button>
            {/each}
        </div>

        <div class="preview">
            <h4>Vista Previa</h4>
            <img
                src={imageUrl}
                alt="Preview"
                on:error={(e) => (e.currentTarget.style.display = "none")}
            />
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
    .control-panel {
        background: var(--bg-secondary);
        padding: 20px;
        border-radius: 12px;
        border: 1px solid var(--border-color);
        margin-bottom: 20px;
    }
    label {
        display: block;
        margin-bottom: 8px;
        font-weight: 500;
        color: var(--text-primary);
    }
    .input-group input {
        width: 100%;
        padding: 12px;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        color: var(--text-primary);
        font-family: var(--font-mono);
        margin-bottom: 16px;
    }
    .btn-primary.full-width {
        width: 100%;
        display: flex;
        justify-content: center;
        align-items: center;
        gap: 8px;
    }
    .engines-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 12px;
        margin-bottom: 24px;
    }
    .engine-card {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 12px;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        color: var(--text-primary);
        cursor: pointer;
        transition: all 0.2s;
    }
    .engine-card:hover {
        border-color: var(--accent-color);
        transform: translateY(-2px);
    }
    .engine-card .icon {
        font-size: 1.2rem;
    }
    .engine-card .name {
        flex: 1;
        font-weight: 500;
        text-align: left;
    }
    .engine-card .arrow {
        color: var(--text-secondary);
        font-size: 0.8rem;
    }
    .preview {
        text-align: center;
    }
    .preview img {
        max-width: 100%;
        max-height: 300px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        margin-top: 10px;
    }
</style>
