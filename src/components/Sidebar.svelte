<script>
  import { agentStore } from "../lib/agentStore.svelte.js";
  import { invoke } from "@tauri-apps/api/core";

  let { currentView = $bindable("dashboard") } = $props();

  async function deleteCurrentCase() {
    if (!agentStore.activeCase) return;
    
    const caseName = agentStore.activeCase.name;
    const confirmName = prompt(`Para eliminar el caso "${caseName}", escribe su nombre para confirmar:`);
    
    if (confirmName !== caseName) {
            if(confirmName !== null) alert("Nombre incorrecto. No se eliminó nada.");
            return;
    }

    try {
        const res = await invoke("delete_case_cmd", { caseName });
        if (res.success) {
            alert("Caso eliminado.");
            agentStore.activeCase = null;
            // Forzar recarga o ir a dashboard
            currentView = "dashboard";
            window.location.reload(); // Simple reload to clear state
        } else {
            alert("Error: " + res.error);
        }
    } catch (e) {
        alert("Error de sistema: " + e);
    }
  }
</script>

<aside class="sidebar">
  <div class="brand">
    <span class="logo-icon">🛡️</span>
    <span class="logo-text">SODIIC</span>
  </div>

  <nav class="nav-menu">
    <button
      class="nav-item"
      class:active={currentView === "dashboard"}
      onclick={() => (currentView = "dashboard")}
    >
      <span class="icon">📊</span>
      Panel
    </button>
    <button
      class="nav-item"
      class:active={currentView === "tools"}
      onclick={() => (currentView = "tools")}
    >
      <span class="icon">🛠️</span>
      Herramientas
    </button>
    <button
      class="nav-item"
      class:active={currentView === "network"}
      onclick={() => (currentView = "network")}
    >
      <span class="icon">🌐</span>
      Red
    </button>
    <button
      class="nav-item"
      class:active={currentView === "settings"}
      onclick={() => (currentView = "settings")}
    >
      <span class="icon">⚙️</span>
      Configuración
    </button>
    <button
      class="nav-item"
      class:active={currentView === "targets"}
      onclick={() => (currentView = "targets")}
    >
      <span class="icon">🎯</span>
      Objetivos
    </button>
    <button
      class="nav-item"
      class:active={currentView === "manual-tools"}
      onclick={() => (currentView = "manual-tools")}
    >
      <span class="icon">💻</span>
      Consola Manual
    </button>

    <div class="divider"></div>

    <button
      class="nav-item special-agent"
      onclick={() => agentStore.togglePanel()}
    >
      <div class="sidebar__bot-avatar">
        <img src="/src/assets/bot_sodiic.png" alt="SODIIC_BOT" />
      </div>
      SODIIC_BOT
    </button>
  </nav>


  <div class="sidebar-footer">
    <div class="status-indicator online"></div>
    <span class="mono">Sistema Online</span>
    {#if agentStore.activeCase}
        <button class="btn-danger-small" onclick={deleteCurrentCase} title="Eliminar Caso Actual">🗑️</button>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    background-color: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    height: 100%;
  }

  .brand {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 var(--spacing-md);
    border-bottom: 1px solid var(--border-color);
    font-weight: 700;
    font-size: 1.2rem;
    letter-spacing: 1px;
  }

  .logo-icon {
    margin-right: 8px;
  }

  .nav-menu {
    flex: 1;
    padding: var(--spacing-md) 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    padding: 10px var(--spacing-md);
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s;
    font-family: var(--font-sans);
    font-size: 0.95rem;
    width: 100%;
  }

  .nav-item:hover {
    background-color: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .nav-item.active {
    background-color: rgba(16, 185, 129, 0.1);
    color: var(--accent-color);
    border-right: 3px solid var(--accent-color);
  }

  .nav-item .icon {
    margin-right: 10px;
    font-size: 1.1em;
  }

  .sidebar-footer {
    padding: var(--spacing-md);
    border-top: 1px solid var(--border-color);
    font-size: 0.8rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--text-muted);
  }
  .status-indicator.online {
    background-color: var(--accent-color);
    box-shadow: 0 0 5px var(--accent-color);
  }

  .divider {
    height: 1px;
    background-color: var(--border-color);
    margin: var(--spacing-sm) var(--spacing-md);
    opacity: 0.5;
  }

  .special-agent {
    margin-top: auto;
    border-top: 1px solid var(--border-color);
    padding-top: var(--spacing-md);
    background: linear-gradient(to right, transparent, rgba(16, 185, 129, 0.05));
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .sidebar__bot-avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    overflow: hidden;
    border: 1px solid var(--accent-color);
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .sidebar__bot-avatar img {
    width: 200%;
    height: 200%;
    object-fit: cover;
    transform: translate(0, 10%);
  }
</style>
