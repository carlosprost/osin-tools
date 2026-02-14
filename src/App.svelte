<script>
  // App.svelte - Main Layout
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { osintTools } from "./lib/osint_tools.js";
  import ToolCard from "./components/ToolCard.svelte";

  import IpLookup from "./components/tools/IpLookup.svelte";
  import UsernameCheck from "./components/tools/UsernameCheck.svelte";
  import DomainIntel from "./components/tools/DomainIntel.svelte";
  import EmailVerify from "./components/tools/EmailVerify.svelte";
  import DomainEmailSearch from "./components/tools/DomainEmailSearch.svelte";
  import ExifViewer from "./components/tools/ExifViewer.svelte";
  import ReverseImageSearch from "./components/tools/ReverseImageSearch.svelte";
  import Settings from "./components/Settings.svelte";
  import AgentChat from "./components/AgentChat.svelte";

  let currentView = "dashboard";
  let activeToolId = null;
  let toolParams = {}; // Parameters passed to tools

  onMount(async () => {
    // Listen for events from Agent to open tools
    const unlisten = await listen("open-tool", (event) => {
      console.log("Evento open-tool recibido:", event.payload);
      const { tool, image } = event.payload;

      if (tool === "reverse_image") {
        toolParams = { image };
        currentView = "tools";
        activeToolId = "reverse-image";
      }
    });

    return () => {
      unlisten();
    };
  });
</script>

<div class="app-shell">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="brand">
      <span class="logo-icon">Target</span>
      <span class="logo-text">OSINT<span class="text-accent">DASH</span></span>
    </div>

    <nav class="nav-menu">
      <button
        class="nav-item"
        class:active={currentView === "dashboard"}
        on:click={() => (currentView = "dashboard")}
      >
        <span class="icon">📊</span>
        Panel
      </button>
      <button
        class="nav-item"
        class:active={currentView === "tools"}
        on:click={() => (currentView = "tools")}
      >
        <span class="icon">🛠️</span>
        Herramientas
      </button>
      <button
        class="nav-item"
        class:active={currentView === "network"}
        on:click={() => (currentView = "network")}
      >
        <span class="icon">🌐</span>
        Red
      </button>
      <button
        class="nav-item"
        class:active={currentView === "settings"}
        on:click={() => (currentView = "settings")}
      >
        <span class="icon">⚙️</span>
        Configuración
      </button>

      <div class="divider"></div>

      <button
        class="nav-item special-agent"
        class:active={currentView === "agent"}
        on:click={() => (currentView = "agent")}
      >
        <span class="icon">🤖</span>
        Agente IA
      </button>
    </nav>

    <div class="sidebar-footer">
      <div class="status-indicator online"></div>
      <span class="mono">Sistema Online</span>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="main-content">
    <header class="top-bar">
      <h2 class="view-title">
        {currentView.charAt(0).toUpperCase() + currentView.slice(1)}
      </h2>
      <div class="actions">
        <!-- Placeholder for global search or actions -->
        <input
          type="text"
          placeholder="Buscar objetivos..."
          class="search-input"
        />
      </div>
    </header>

    <div class="content-scroll">
      {#if currentView === "dashboard"}
        <div class="dashboard-grid">
          <div class="card">
            <h3>Escaneos Recientes</h3>
            <p class="text-muted">Sin actividad reciente.</p>
          </div>
          <div class="card">
            <h3>Estado del Sistema</h3>
            <div class="stat-row">
              <span>Carga CPU</span>
              <span class="mono text-accent">12%</span>
            </div>
            <div class="stat-row">
              <span>Memoria</span>
              <span class="mono text-accent">340MB</span>
            </div>
          </div>
        </div>
      {:else if currentView === "tools"}
        {#if !activeToolId}
          <div class="tools-grid">
            {#each osintTools as tool (tool.id)}
              <div
                class="tool-wrapper"
                on:click={() => {
                  if (tool.status !== "development") activeToolId = tool.id;
                }}
                on:keydown={(e) =>
                  e.key === "Enter" &&
                  tool.status !== "development" &&
                  (activeToolId = tool.id)}
                role="button"
                tabindex="0"
              >
                <ToolCard {tool} />
              </div>
            {/each}
          </div>
        {:else if activeToolId === "ip-lookup"}
          <IpLookup onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "username-check"}
          <UsernameCheck onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "domain-intel"}
          <DomainIntel onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "email-verify"}
          <EmailVerify onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "exif-viewer"}
          <ExifViewer onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "reverse-image"}
          <ReverseImageSearch
            imageUrl={toolParams?.image || ""}
            onBack={() => {
              activeToolId = null;
              toolParams = {};
            }}
          />
        {:else if activeToolId === "domain-email-search"}
          <DomainEmailSearch onBack={() => (activeToolId = null)} />
        {:else}
          <div class="placeholder-view">
            <button on:click={() => (activeToolId = null)}>← Volver</button>
            <p>
              La herramienta <strong>{activeToolId}</strong> está en construcción.
            </p>
          </div>
        {/if}
      {:else if currentView === "settings"}
        <Settings />
      {:else if currentView === "agent"}
        <AgentChat />
      {:else}
        <div class="placeholder-view">
          <p>
            El módulo <strong>{currentView}</strong> está actualmente en desarrollo.
          </p>
        </div>
      {/if}
    </div>
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    background-color: var(--bg-primary);
  }

  /* Sidebar */
  .sidebar {
    width: var(--sidebar-width);
    background-color: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
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

  .text-accent {
    color: var(--accent-color);
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

  /* Main Content */
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-primary);
    overflow: hidden;
  }

  .top-bar {
    height: var(--header-height);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--spacing-lg);
    background-color: var(--bg-primary); /* Glass effect could go here */
  }

  .search-input {
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 4px;
    width: 250px;
    font-size: 0.9rem;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-color);
  }

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-lg);
  }

  /* Dashboard Grid */
  .dashboard-grid,
  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--spacing-lg);
  }

  .card {
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: var(--spacing-md);
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    margin-top: 8px;
    font-size: 0.9rem;
  }

  .text-muted {
    color: var(--text-muted);
  }
</style>
