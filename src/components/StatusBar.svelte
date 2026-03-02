<script>
  import { agentStore } from "../lib/agentStore.svelte.js";
  import { configStore } from "../lib/configStore.svelte.js";
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";

  // La barra ahora es persistente, pero el contenido cambia
  let isLoading = $derived(agentStore.isLoading);
  let statusText = $derived(agentStore.statusMessage);

  async function handleOpenFolder() {
    if (agentStore.activeCase) {
      try {
        await invoke("open_case_folder", { case_name: agentStore.activeCase.name });
      } catch (e) {
        console.error("Error al abrir carpeta:", e);
      }
    }
  }

  function handleKeydown(e) {
    if (e.key === "Enter" || e.key === " ") {
      if (e.key === " ") e.preventDefault();
      handleOpenFolder();
    }
  }
</script>

<div class="status-bar" class:active={isLoading} transition:fade={{ duration: 200 }}>
  <div class="left-section">
    <div class="indicator-group" class:loading={isLoading}>
      {#if isLoading}
        <div class="spinner"></div>
      {:else}
        <div class="dot"></div>
      {/if}
      <span class="status-message">{statusText}</span>
    </div>
  </div>

  <div class="center-section">
    <!-- Reservado para progreso de tareas específicas si se requiere -->
    {#if agentStore.activeTask}
        <span class="task-info">Ejecutando: {agentStore.activeTask.name}</span>
    {/if}
  </div>

  <div class="right-section">
    <div class="system-stats">
        <span class="stat-item">
            <span class="label">Agente:</span>
            <span class="value cyan">SODIIC_BOT v1.2</span>
        </span>
        <span class="divider"></span>
        <span class="stat-item">
            <span class="label">Motor:</span>
            <select 
              class="status-select" 
              bind:value={configStore.config.ollama_model}
              onchange={() => configStore.syncWithRust()}
              title="Cambiar modelo de Ollama"
            >
              {#if configStore.availableModels.length === 0}
                <option value={configStore.config.ollama_model}>{configStore.config.ollama_model}</option>
              {/if}
              {#each configStore.availableModels as model}
                <option value={model}>{model}</option>
              {/each}
            </select>
        </span>
        <span class="divider"></span>
        <button class="stat-item clickable-case" 
                class:active={!!agentStore.activeCase} 
                onclick={handleOpenFolder} 
                onkeydown={handleKeydown} 
                aria-label="Abrir carpeta del caso"
                disabled={!agentStore.activeCase}>
            {#if agentStore.activeCase}
                <span class="case-badge">📁 {agentStore.activeCase.name}</span>
            {:else}
                <span class="no-case">No hay caso activo</span>
            {/if}
        </button>
    </div>
  </div>
</div>

<style>
  .status-bar {
    position: relative;
    width: 100%;
    height: 28px;
    background: #0f172a; /* Azul muy oscuro tipo IDE */
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    z-index: 10000;
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    font-size: 11px;
    color: var(--text-muted);
    user-select: none;
    transition: background 0.3s ease;
  }

  .status-bar.active {
    background: #1e293b; /* Un tono más claro cuando hay acción */
  }

  .left-section, .right-section {
    display: flex;
    align-items: center;
    gap: 15px;
    height: 100%;
  }

  .indicator-group {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .indicator-group.loading .status-message {
    color: var(--accent-color);
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #10b981; /* Verde esmeralda */
    border-radius: 50%;
    box-shadow: 0 0 8px rgba(16, 185, 129, 0.4);
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid rgba(255, 255, 255, 0.1);
    border-top: 1.5px solid var(--accent-color);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .status-message {
    letter-spacing: 0.2px;
  }

  .task-info {
    color: var(--accent-color);
    opacity: 0.8;
  }

  .system-stats {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .stat-item {
    display: flex;
    gap: 4px;
  }

  .stat-item .label {
    opacity: 0.5;
  }

  .stat-item .value {
    font-weight: 600;
  }

  .stat-item .value.cyan {
    color: #22d3ee;
  }

  .divider {
    width: 1px;
    height: 12px;
    background: rgba(255, 255, 255, 0.1);
  }

  .case-badge {
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 8px;
    border-radius: 4px;
    color: var(--text-primary);
  }

  .clickable-case {
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font-family: inherit;
    font-size: inherit;
    color: inherit;
    display: flex;
    align-items: center;
  }

  .clickable-case:not(:disabled):hover .case-badge {
    background: rgba(255, 255, 255, 0.15);
    color: var(--accent-color);
  }

  .clickable-case:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .status-select {
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: inherit;
    font-weight: 600;
    padding: 0;
    margin: 0;
    cursor: pointer;
    outline: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    appearance: none;
  }

  .status-select:hover {
    color: var(--accent-color);
  }

  .status-select option {
    background: #0f172a;
    color: var(--text-primary);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Evitar que el texto se rompa en pantallas chicas */
  @media (max-width: 600px) {
    .right-section { display: none; }
  }
</style>
