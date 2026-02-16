<script>
    import { agentStore } from "../lib/agentStore.svelte.js";

    let { onReport = () => {}, onLink = () => {} } = $props();
</script>

<div class="investigation-toolbar">
    <div class="case-header">
        <div class="case-badge" title="Investigación Activa">
            <span class="dot"></span>
            {agentStore.activeCase?.name || "Sin Caso"}
        </div>
    </div>

    <div class="tools-group">
        <button class="tool-btn" onclick={() => onReport()} title="Generar Informe del Caso">
            <span class="icon">📑</span> 
            <span class="text">Informe</span>
        </button>
        <button class="tool-btn" onclick={() => onLink()} title="Vincular Objetivos">
            <span class="icon">🔗</span>
            <span class="text">Vincular</span>
        </button>
        <button class="tool-btn" title="Exportar Evidencia">
            <span class="icon">📤</span>
            <span class="text">Exportar</span>
        </button>
    </div>
</div>

<style>
    .investigation-toolbar {
        display: flex;
        flex-direction: column;
        gap: 12px;
        padding: 12px;
        background: var(--bg-tertiary);
        border-bottom: 1px solid var(--border-color);
    }

    .case-header {
        display: flex;
        justify-content: center;
    }

    .tools-group {
        display: flex;
        gap: 8px;
    }

    .tool-btn {
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 6px 12px;
        border-radius: 6px;
        font-size: 0.75rem;
        font-weight: 600;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 6px;
        transition: all 0.2s;
    }

    .tool-btn:hover {
        border-color: var(--accent-color);
        background: var(--bg-secondary);
    }

    .icon { font-size: 1rem; }

    .case-badge {
        font-size: 0.7rem;
        background: rgba(16, 185, 129, 0.1);
        color: #10b981;
        padding: 4px 10px;
        border-radius: 20px;
        display: flex;
        align-items: center;
        gap: 6px;
        font-weight: 700;
        text-transform: uppercase;
        border: 1px solid rgba(16, 185, 129, 0.2);
    }

    .dot {
        width: 6px;
        height: 6px;
        background: #10b981;
        border-radius: 50%;
        box-shadow: 0 0 8px #10b981;
    }

    @media (max-width: 400px) {
        .text { display: none; }
    }
</style>
