<script>
    import { onMount } from "svelte";
    import * as faceapi from "face-api.js";
    import * as faceService from "../../lib/face_recognition.js";
    import { invoke } from "@tauri-apps/api/core";

    let { onBack = () => {} } = $props();

    let images = $state([]); // { url, descriptor, name }
    let results = $state([]); // Matrix
    let isLoading = $state(false);
    let useTiny = $state(true); // Default to Fast Mode
    let status = $state("");

    onMount(() => {
        // Pre-cargar modelos en segundo plano para evitar lag al iniciar
        invoke("download_face_models").then(() => {
            faceService.loadModels(useTiny).catch(console.error);
        });
    });

    async function handleFileSelect(event) {
        const files = Array.from(event.target.files);
        for (const file of files) {
            const url = URL.createObjectURL(file);
            images = [...images, { url, descriptor: null, name: file.name }];
        }
    }

    async function runAnalysis() {
        if (images.length < 2) {
            status = "Selecciona al menos 2 imágenes.";
            return;
        }
        isLoading = true;
        const msgStart = `[BIOMETRÍA] Iniciando análisis All-vs-All de ${images.length} imágenes...`;
        status = "Cargando modelos...";
        await invoke("log_info", { message: msgStart });
        try {
            await invoke("download_face_models");
            await faceService.loadModels(useTiny);

            for (let i = 0; i < images.length; i++) {
                if (!images[i].descriptor) {
                    const startTime = performance.now();
                    status = `Analizando imagen ${i + 1}/${images.length} (${useTiny ? "Modo Rápido" : "Modo Preciso"})...`;
                    await invoke("log_info", { message: `[BIOMETRÍA] Paso 1: Obteniendo imagen de URL Blob...` });
                    
                    const img = await faceapi.fetchImage(images[i].url);
                    await invoke("log_info", { message: `[BIOMETRÍA] Paso 2: Imagen obtenida. Calculando descriptor...` });
                    
                    const desc = await faceService.getFaceDescriptor(img, useTiny);
                    await invoke("log_info", { message: `[BIOMETRÍA] Paso 3: Análisis de red neuronal completado.` });
                    
                    const duration = (performance.now() - startTime).toFixed(0);
                    
                    if (!desc) {
                        images[i].error = "No se detectó rostro";
                        await invoke("log_info", { message: `[BIOMETRÍA] Imagen ${i+1} analizada en ${duration}ms (FALLÓ: No se detectó rostro)` });
                    } else {
                        images[i].descriptor = desc;
                        await invoke("log_info", { message: `[BIOMETRÍA] Imagen ${i+1} analizada en ${duration}ms` });
                    }
                }
            }

            status = "Calculando matriz de similitud...";
            const matrix = [];
            for (let i = 0; i < images.length; i++) {
                const row = [];
                for (let j = 0; j < images.length; j++) {
                    if (i === j) {
                        row.push(100);
                    } else if (!images[i].descriptor || !images[j].descriptor) {
                        row.push(0);
                    } else {
                        const dist = faceService.compareFaces(
                            images[i].descriptor,
                            images[j].descriptor
                        );
                        // Mapeo mejorado: 0.0 -> 100%, 0.6 (threshold) -> ~70%
                        // Usamos una curva que penaliza más después del threshold
                        let score;
                        if (dist < 0.6) {
                            score = 100 - (dist * 50); // 0.0 -> 100, 0.6 -> 70
                        } else {
                            score = Math.max(0, 70 - ((dist - 0.6) * 120)); // Baja rápido después de 0.6
                        }
                        row.push(score.toFixed(1));
                    }
                }
                matrix.push(row);
            }
            results = matrix;
            status = "Análisis finalizado.";
            await invoke("log_info", { message: "[BIOMETRÍA] Análisis de matriz completado." });
        } catch (e) {
            console.error(e);
            status = "Error crítico: " + e;
        }
        isLoading = false;
    }

    function clear() {
        images = [];
        results = [];
        status = "";
    }
</script>

<div class="bio-comparison">
    <div class="tool-header">
        <button class="btn-back" onclick={() => onBack()}>← Volver</button>
        <h3>Comparación Biométrica Avanzada</h3>
        <p class="subtitle">Compara múltiples rostros entre sí (Análisis All-vs-All)</p>
    </div>

    <div class="controls">
        <label class="upload-btn">
            <span>➕ Agregar Imágenes</span>
            <input type="file" multiple accept="image/*" onchange={handleFileSelect} hidden />
        </label>
        <label class="toggle-control">
            <input type="checkbox" bind:checked={useTiny} disabled={isLoading} />
            <span class="toggle-label">Modo Rápido (Lighweight)</span>
        </label>
        <button class="primary-btn" onclick={runAnalysis} disabled={isLoading || images.length < 2}>
            {isLoading ? "Procesando..." : "🚀 Iniciar Comparación"}
        </button>
        <button class="secondary-btn" onclick={clear} disabled={isLoading}>
            Limpiar
        </button>
    </div>

    {#if status}
        <div class="search-status {status.includes('Error') ? 'error' : 'info'}">
            {status}
        </div>
    {/if}

    <div class="image-grid">
        {#each images as img, i}
            <div class="image-card">
                <img src={img.url} alt={img.name} />
                <div class="badge">S{i + 1}</div>
                {#if img.error}
                    <div class="error-overlay">⚠️ {img.error}</div>
                {/if}
            </div>
        {/each}
    </div>

    {#if results.length > 0}
        <div class="results-table-container">
            <h4>Matriz de Coincidencias (%)</h4>
            <div class="table-wrapper">
                <table>
                    <thead>
                        <tr>
                            <th>/</th>
                            {#each images as _, i}
                                <th>S{i + 1}</th>
                            {/each}
                        </tr>
                    </thead>
                    <tbody>
                        {#each results as row, i}
                            <tr>
                                <td class="row-header">S{i + 1}</td>
                                {#each row as score, j}
                                    <td class="score {score > 70 ? 'match' : score > 50 ? 'likely' : 'diff'}">
                                        {score}%
                                    </td>
                                {/each}
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        </div>
    {/if}
</div>

<style>
    .bio-comparison {
        display: flex;
        flex-direction: column;
        gap: 20px;
        padding: 10px;
    }

    .subtitle {
        color: #94a3b8;
        font-size: 0.9em;
    }

    .controls {
        display: flex;
        gap: 15px;
        align-items: center;
        background: var(--bg-secondary);
        padding: 15px;
        border-radius: 12px;
        border: 1px solid var(--border-color);
    }

    .toggle-control {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
        padding: 8px 12px;
        background: var(--bg-tertiary);
        border-radius: 6px;
        font-size: 0.85rem;
    }

    .toggle-label {
        color: var(--text-secondary);
    }

    .toggle-control input:checked + .toggle-label {
        color: var(--accent-color);
    }

    .btn-back {
        background: transparent;
        border: 1px solid #334155;
        color: white;
        padding: 5px 12px;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.8em;
        margin-right: 10px;
        transition: 0.2s;
    }

    .btn-back:hover {
        background: #334155;
    }

    .upload-btn {
        background: #334155;
        padding: 10px 20px;
        border-radius: 8px;
        cursor: pointer;
        transition: 0.2s;
        border: 1px solid #475569;
    }

    .upload-btn:hover {
        background: #475569;
    }

    .primary-btn {
        background: #10b981;
        padding: 10px 20px;
        border: none;
        border-radius: 8px;
        color: white;
        font-weight: bold;
        cursor: pointer;
    }

    .secondary-btn {
        background: transparent;
        border: 1px solid #475569;
        padding: 10px 20px;
        border-radius: 8px;
        color: white;
        cursor: pointer;
    }

    .image-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
        gap: 15px;
    }

    .image-card {
        position: relative;
        border-radius: 12px;
        overflow: hidden;
        border: 2px solid #334155;
        aspect-ratio: 1;
    }

    .image-card img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .badge {
        position: absolute;
        top: 5px;
        left: 5px;
        background: rgba(16, 185, 129, 0.9);
        color: white;
        padding: 2px 8px;
        border-radius: 4px;
        font-size: 0.7em;
        font-weight: bold;
    }

    .error-overlay {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        background: rgba(239, 68, 68, 0.9);
        color: white;
        font-size: 0.7em;
        padding: 5px;
        text-align: center;
    }

    .results-table-container {
        margin-top: 20px;
        background: #1e293b;
        padding: 20px;
        border-radius: 12px;
        border: 1px solid #334155;
    }

    .table-wrapper {
        overflow-x: auto;
    }

    table {
        width: 100%;
        border-collapse: collapse;
        margin-top: 15px;
    }

    th, td {
        padding: 12px;
        text-align: center;
        border-bottom: 1px solid #334155;
    }

    .row-header {
        font-weight: bold;
        color: #10b981;
    }

    .score {
        font-weight: bold;
    }

    .match { color: #10b981; }
    .likely { color: #fbbf24; }
    .diff { color: #94a3b8; }

    .search-status {
        padding: 15px;
        border-radius: 8px;
        font-size: 0.9em;
    }

    .info { background: #1e293b; border-left: 4px solid #3b82f6; }
    .error { background: #450a0a; border-left: 4px solid #ef4444; }
</style>
