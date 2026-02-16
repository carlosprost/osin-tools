<script>
  // App.svelte - Main Layout (Modularized)
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core"; // Importar invoke
  import { osintTools } from "./lib/osint_tools.js";
  import ToolCard from "./components/ToolCard.svelte";

  // Components
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Dashboard from "./components/Dashboard.svelte";
  
  // Tools
  import IpLookup from "./components/tools/IpLookup.svelte";
  import UsernameCheck from "./components/tools/UsernameCheck.svelte";
  import DomainIntel from "./components/tools/DomainIntel.svelte";
  import EmailVerify from "./components/tools/EmailVerify.svelte";
  import DomainEmailSearch from "./components/tools/DomainEmailSearch.svelte";
  import ExifViewer from "./components/tools/ExifViewer.svelte";
  import ReverseImageSearch from "./components/tools/ReverseImageSearch.svelte";
  import FaceComparison from "./components/tools/FaceComparison.svelte";
  import Settings from "./components/Settings.svelte";
  import TargetsView from "./components/TargetsView.svelte";
  import AgentPanel from "./components/AgentPanel.svelte";
  import WelcomeView from "./components/WelcomeView.svelte";
  import { agentStore } from "./lib/agentStore.svelte.js";

  let currentView = $state("dashboard");
  let activeToolId = $state(null);
  let toolParams = $state({ image: null });

  // Resizing state
  let isResizing = $state(false);
  let panelWidth = $state(400);

  function startResizing() {
    isResizing = true;
  }

  function stopResizing() {
    isResizing = false;
  }

  function onMouseMove(e) {
    if (!isResizing) return;
    const newWidth = window.innerWidth - e.clientX;
    if (newWidth > 300 && newWidth < 800) {
      panelWidth = newWidth;
    }
  }

  onMount(() => {
    // --- GLOBAL CONFIG SYNC ---
    const savedKeys = localStorage.getItem("osint_api_keys");
    if (savedKeys) {
        try {
            const parsed = JSON.parse(savedKeys);
            const config = {
                ...parsed,
                tor_active: false,
                mac_masking_active: false,
                proxy_url: ""
            };
            console.log("Iniciando sincronización global de configuración...");
            invoke("update_osint_config", { config })
                .then(() => console.log("Configuración sincronizada con Backend correctamente."))
                .catch(e => console.error("Error sincronizando config global:", e));
        } catch (e) {
            console.error("Error leyendo configuración guardada:", e);
        }
    }

    let unlistenFn;
    const setupListener = async () => {
      unlistenFn = await listen("open-tool", (event) => {
        console.log("Evento open-tool recibido:", event.payload);
        const { tool, image } = event.payload;

        if (tool === "reverse_image") {
          toolParams = { image };
          currentView = "tools";
          activeToolId = "reverse-image";
        }
      });
    };

    setupListener();

    return () => {
      if (unlistenFn) unlistenFn();
    };
  });
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={stopResizing} />

<div class="app-shell">
  <Sidebar bind:currentView />

  {#if !agentStore.activeCase}
    <WelcomeView />
  {/if}

  <main class="main-content">
    <TopBar {currentView} />

    <div class="content-scroll">
      {#if currentView === "dashboard"}
        <Dashboard />
      {:else if currentView === "tools"}
        {#if !activeToolId}
          <div class="tools-grid">
            {#each osintTools as tool (tool.id)}
              <div
                class="tool-wrapper"
                onclick={() => {
                  if (tool.status !== "development") activeToolId = tool.id;
                }}
                onkeydown={(e) =>
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
              toolParams = { image: null };
            }}
          />
        {:else if activeToolId === "domain-email-search"}
          <DomainEmailSearch onBack={() => (activeToolId = null)} />
        {:else if activeToolId === "biometric-comparison"}
          <FaceComparison onBack={() => (activeToolId = null)} />
        {:else}
          <div class="placeholder-view">
            <button class="btn-back" onclick={() => (activeToolId = null)}>← Volver</button>
            <p>
              La herramienta <strong>{activeToolId}</strong> está en construcción.
            </p>
          </div>
        {/if}
      {:else if currentView === "settings"}
        <Settings />
      {:else if currentView === "targets"}
        <TargetsView />
      {:else}
        <div class="placeholder-view">
          <p>
            El módulo <strong>{currentView}</strong> está actualmente en desarrollo.
          </p>
        </div>
      {/if}
    </div>
  </main>
  
  {#if agentStore.isPanelOpen}
    <div 
        class="resizer" 
        onmousedown={startResizing}
        role="presentation"
    ></div>
    <div class="agent-container" style="width: {panelWidth}px">
        <AgentPanel />
    </div>
  {/if}
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    background-color: var(--bg-primary);
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-primary);
    overflow: hidden;
  }

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-lg);
  }

  /* Resizer */
  .resizer {
    width: 4px;
    cursor: col-resize;
    background: var(--border-color);
    transition: background 0.2s;
    z-index: 100;
  }

  .resizer:hover, .resizer:active {
    background: var(--accent-color);
  }

  .agent-container {
    height: 100%;
    flex-shrink: 0;
    overflow: hidden;
  }

  /* Tools Grid */
  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--spacing-lg);
  }

  .tool-wrapper {
    cursor: pointer;
    outline: none;
  }

  .placeholder-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: var(--spacing-md);
  }

  .btn-back {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-back:hover {
    background-color: var(--border-color);
  }
</style>
