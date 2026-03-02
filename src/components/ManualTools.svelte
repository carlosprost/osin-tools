<script>
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    let command = "";
    let history = [];
    let isExecuting = false;
    let terminalOutput = "Consola de Kali Linux (WSL) - Lista para operar.\nIngresá un comando (ej: whois, cat, hostname -I).\n";
    let terminalContainer;

    async function handleExecute() {
        if (!command.trim() || isExecuting) return;

        const cmdToRun = command.trim();
        history = [cmdToRun, ...history.slice(0, 19)]; // Guardar en historial local
        command = "";
        isExecuting = true;
        terminalOutput += `\n> ${cmdToRun}\n`;

        try {
            const res = await invoke("run_manual_wsl", { command: cmdToRun });
            if (res.success) {
                terminalOutput += res.data + "\n";
            } else {
                terminalOutput += "❌ ERROR: " + (res.error || res.data) + "\n";
            }
        } catch (err) {
            terminalOutput += "❌ FALLO CRÍTICO: " + err + "\n";
        } finally {
            isExecuting = false;
            // Scroll al final
            setTimeout(() => {
                if (terminalContainer) {
                    terminalContainer.scrollTop = terminalContainer.scrollHeight;
                }
            }, 50);
        }
    }

    function handleKeyDown(e) {
        if (e.key === "Enter") {
            handleExecute();
        }
    }

    function clearConsole() {
        terminalOutput = "Consola limpia. Lista para operar.\n";
    }
</script>

<div class="manual-tools">
    <div class="header">
        <h1>🛠️ Herramientas Manuales</h1>
        <p>Terminal directa a WSL Kali para validación técnica de la investigación.</p>
    </div>

    <div class="terminal-container" bind:this={terminalContainer}>
        <pre>{terminalOutput}</pre>
        {#if isExecuting}
            <div class="cursor-line">
                <span class="prompt">_</span>
                <span class="loading">Pensando...</span>
            </div>
        {/if}
    </div>

    <div class="input-area">
        <span class="prompt">kali@sodiic:~$</span>
        <input 
            type="text" 
            bind:value={command} 
            placeholder="Escribí un comando y dale a Enter..." 
            on:keydown={handleKeyDown}
            disabled={isExecuting}
        />
        <button on:click={handleExecute} disabled={isExecuting || !command.trim()}>
            {isExecuting ? "Ejecutando..." : "Ejecutar"}
        </button>
        <button class="clear-btn" on:click={clearConsole}>Limpiar</button>
    </div>

    <div class="history-area">
        <h3>Historial Reciente</h3>
        <div class="history-tags">
            {#each history as hc}
                <button class="history-tag" on:click={() => command = hc}>{hc}</button>
            {:else}
                <p class="empty-hint">Sin historial de comandos.</p>
            {/each}
        </div>
    </div>
</div>

<style>
    .manual-tools {
        display: flex;
        flex-direction: column;
        height: 100%;
        padding: 24px;
        color: #e0e0e0;
        background: #0d1117;
        font-family: 'Inter', system-ui, -apple-system, sans-serif;
    }

    .header h1 {
        font-size: 1.8rem;
        margin-bottom: 8px;
        color: #58a6ff;
    }

    .header p {
        color: #8b949e;
        margin-bottom: 24px;
    }

    .terminal-container {
        flex: 1;
        background: #010409;
        border: 1px solid #30363d;
        border-radius: 8px;
        padding: 16px;
        font-family: 'Cascadia Code', 'Fira Code', monospace;
        font-size: 0.9rem;
        overflow-y: auto;
        margin-bottom: 16px;
        box-shadow: inset 0 0 10px rgba(0,0,0,0.5);
    }

    pre {
        margin: 0;
        white-space: pre-wrap;
        word-break: break-all;
        color: #39ff14; /* Matrix Green */
        line-height: 1.5;
    }

    .cursor-line {
        margin-top: 8px;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .prompt {
        color: #58a6ff;
        font-weight: bold;
    }

    .loading {
        color: #8b949e;
        font-style: italic;
        animation: pulse 1.5s infinite;
    }

    @keyframes pulse {
        0% { opacity: 0.4; }
        50% { opacity: 1; }
        100% { opacity: 0.4; }
    }

    .input-area {
        display: flex;
        align-items: center;
        gap: 12px;
        background: #161b22;
        padding: 12px;
        border-radius: 8px;
        border: 1px solid #30363d;
    }

    input {
        flex: 1;
        background: transparent;
        border: none;
        color: white;
        font-family: inherit;
        font-size: 1rem;
        outline: none;
    }

    button {
        background: #238636;
        color: white;
        border: none;
        padding: 8px 16px;
        border-radius: 6px;
        cursor: pointer;
        font-weight: 600;
        transition: background 0.2s;
    }

    button:hover:not(:disabled) {
        background: #2ea043;
    }

    button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .clear-btn {
        background: #30363d;
    }

    .clear-btn:hover:not(:disabled) {
        background: #3c444d;
    }

    .history-area {
        margin-top: 24px;
    }

    .history-area h3 {
        font-size: 1rem;
        margin-bottom: 12px;
        color: #8b949e;
    }

    .history-tags {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }

    .history-tag {
        background: #21262d;
        color: #c9d1d9;
        font-size: 0.8rem;
        padding: 4px 12px;
        border-radius: 4px;
        border: 1px solid #30363d;
        font-family: monospace;
    }

    .history-tag:hover {
        background: #30363d;
        border-color: #8b949e;
    }

    .empty-hint {
        color: #484f58;
        font-style: italic;
        font-size: 0.9rem;
    }
</style>
