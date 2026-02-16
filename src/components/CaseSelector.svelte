<script>
    import { agentStore } from "../lib/agentStore.svelte.js";

    let showModal = $state(false);
    let newCaseName = $state("");
    let newCaseDesc = $state("");

    async function handleCreate() {
        if (!newCaseName.trim()) return;
        const success = await agentStore.createCase(newCaseName, newCaseDesc);
        if (success) {
            showModal = false;
            newCaseName = "";
            newCaseDesc = "";
        }
    }
</script>

<div class="case-selector">
    <div class="active-info">
        <span class="label">Investigación:</span>
        <select 
            class="case-select" 
            value={agentStore.activeCase?.name || ""} 
            onchange={(e) => agentStore.selectCase(e.currentTarget.value)}
        >
            {#if !agentStore.activeCase}
                <option value="" disabled>Seleccionar investigación...</option>
            {/if}
            {#each agentStore.availableCases as caseName}
                <option value={caseName}>{caseName}</option>
            {/each}
        </select>
    </div>

    <button class="btn-new" onclick={() => showModal = true}>
        <span class="icon">➕</span> Nuevo Caso
    </button>
</div>

{#if showModal}
    <div class="modal-overlay">
        <div class="modal">
            <h3>Nueva Investigación</h3>
            <div class="form-group">
                <label for="name">Nombre del Caso</label>
                <input id="name" type="text" bind:value={newCaseName} placeholder="Ej: Operación Falcón" />
            </div>
            <div class="form-group">
                <label for="desc">Descripción (Opcional)</label>
                <textarea id="desc" bind:value={newCaseDesc} placeholder="Detalles de la investigación..."></textarea>
            </div>
            <div class="modal-actions">
                <button class="btn-cancel" onclick={() => showModal = false}>Cancelar</button>
                <button class="btn-confirm" onclick={handleCreate}>Crear Caso</button>
            </div>
        </div>
    </div>
{/if}

<style>
    .case-selector {
        display: flex;
        align-items: center;
        gap: 16px;
        background: var(--bg-secondary);
        padding: 4px 12px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
    }

    .active-info {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .label {
        font-size: 0.75rem;
        color: var(--text-muted);
        text-transform: uppercase;
        font-weight: 600;
    }

    .case-select {
        background: none;
        border: none;
        color: var(--accent-color);
        font-weight: 600;
        cursor: pointer;
        font-size: 0.9rem;
        outline: none;
    }

    .btn-new {
        background: var(--accent-color);
        color: white;
        border: none;
        padding: 4px 10px;
        border-radius: 4px;
        font-size: 0.8rem;
        font-weight: 600;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 4px;
        transition: opacity 0.2s;
    }

    .btn-new:hover { opacity: 0.9; }

    /* Modal */
    .modal-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0,0,0,0.8);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .modal {
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        padding: 24px;
        border-radius: 12px;
        width: 400px;
        box-shadow: 0 20px 50px rgba(0,0,0,0.5);
    }

    .form-group {
        margin-bottom: 16px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    label { font-size: 0.85rem; color: var(--text-secondary); }

    input, textarea {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 10px;
        border-radius: 6px;
        outline: none;
    }

    input:focus, textarea:focus { border-color: var(--accent-color); }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 12px;
        margin-top: 24px;
    }

    .btn-cancel {
        background: none;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        padding: 8px 16px;
        border-radius: 6px;
        cursor: pointer;
    }

    .btn-confirm {
        background: var(--accent-color);
        color: white;
        border: none;
        padding: 8px 20px;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
    }
</style>
