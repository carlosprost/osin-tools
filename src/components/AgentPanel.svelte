<script>
    import { agentStore } from "../lib/agentStore.svelte.js";
    import TaskCard from "./TaskCard.svelte";
    import InvestigationToolbar from "./InvestigationToolbar.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog";
    import * as faceService from "../lib/face_recognition.js";
    import snarkdown from "snarkdown";
    import SuggestionsList from "./SuggestionsList.svelte";

    let input = $state("");
    let chatContainer = $state(null);
    let attachedImage = $state(null);
    let targetDescriptor = $state(null); // Biometric fingerprint

    // --- Sugerencias (Slash & Mentions) ---
    let showSuggestions = $state(false);
    let suggestionType = $state('command'); // 'command' | 'mention' | 'model'
    let filteredSuggestions = $state([]);
    let suggestionIndex = $state(0);
    let availableTargets = $state([]);
    let availableModels = $state([]);

    const commands = [
        { id: 'manual', label: 'manual', description: 'Ver manual operativo' },
        { id: 'modelo', label: 'modelo', description: 'Cambiar modelo de Ollama' },
        { id: 'config', label: 'config', description: 'Ver configuración actual' },
        { id: 'clear', label: 'clear', description: 'Limpiar chat' },
        { id: 'info', label: 'info', description: 'Estado del sistema' }
    ];

    $effect(() => {
        if ((agentStore.messages.length || agentStore.isLoading) && chatContainer) {
            // Usamos requestAnimationFrame para asegurar que el DOM se haya renderizado
            requestAnimationFrame(() => {
                chatContainer.scrollTop = chatContainer.scrollHeight;
            });
        }
    });

    // Cargar objetivos y modelos preventivamente para que el @ y / sean instantáneos
    $effect(() => {
        if (agentStore.activeCase) {
            loadTargets();
            loadModels();
        }
    });

    async function selectImage() {
        try {
            const file = await open({
                multiple: false,
                filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg"] }],
            });
            if (file) {
                attachedImage = file;
                // Pre-profile face if attached
                const result = await invoke("read_file_base64", { path: file });
                if (result.success) {
                    const img = new Image();
                    img.src = result.data;
                    img.onload = async () => {
                        const descriptor = await faceService.getFaceDescriptor(img, true); // Use fast mode for agent
                        if (descriptor) {
                            targetDescriptor = descriptor;
                            agentStore.messages.push({
                                role: "system",
                                content: "✅ Rostro perfilado para seguimiento automático."
                            });
                        }
                    };
                }
            }
        } catch (e) {
            console.error("Error seleccionando imagen:", e);
        }
    }

    async function verifyFaceInUrl(url) {
        if (!targetDescriptor) return;
        try {
            const img = new Image();
            img.crossOrigin = "anonymous";
            img.src = url;
            img.onload = async () => {
                const descriptor = await faceService.getFaceDescriptor(img, true);
                if (descriptor) {
                    const distance = faceService.compareFaces(targetDescriptor, descriptor);
                    if (distance < 0.6) {
                        agentStore.messages.push({
                            role: "system",
                            content: `🎯 ¡COINCIDENCIA BIOMÉTRICA! Rostro detectado en: ${url}`
                        });
                        agentStore.saveHistory();
                    }
                }
            };
        } catch (e) {
            console.error("Error en verificación automática:", e);
        }
    }

    async function handleSend() {
        if (!input.trim() && !attachedImage) return;

        let displayInput = input;
        let internalQuery = input;

        // Si es un comando tipo slash, lo procesamos pero mantenemos el texto literal para el usuario
        if (input.startsWith('/')) {
            const parts = input.trim().split(' ');
            const cmd = parts[0].slice(1).toLowerCase();
            const arg = parts.slice(1).join(' ');

            if (cmd === 'clear') {
                agentStore.messages = [];
                input = "";
                return;
            }
            
            // Traducimos el comando a una orden clara para el Agente si es necesario,
            // pero mantenemos el /comando en el chat del usuario.
            if (cmd === 'manual') internalQuery = "Mostrame tu manual de operación detallado.";
            else if (cmd === 'config') internalQuery = "Mostrame tu configuración actual y estado de APIs.";
            else if (cmd === 'info') internalQuery = "Estado del sistema e identidad técnica.";
            else if (cmd === 'modelo') {
                if (arg) internalQuery = `Cambiá el modelo de Ollama a '${arg}'`;
                else internalQuery = "Listame los modelos de Ollama disponibles.";
            }
        }

        const q = internalQuery;
        const disp = displayInput;
        const img = attachedImage;
        
        input = "";
        attachedImage = null;

        // Agregamos el mensaje del usuario manualmente para que sea el literal
        agentStore.messages.push({ role: "user", content: disp, image: img });
        agentStore.saveHistory();
        
        // Llamamos al agente con la query interna pero sin que el store agregue otro mensaje de usuario
        await agentStore.processQuery(q, img);

        // Scan last message for images...
        const lastMsg = agentStore.messages[agentStore.messages.length - 1];
        if (lastMsg && lastMsg.role === "assistant" && targetDescriptor) {
            const regex = /!\[.*?\]\((.*?)\)/g;
            let match;
            while ((match = regex.exec(lastMsg.content)) !== null) {
                verifyFaceInUrl(match[1]);
            }
        }
    }

    async function loadTargets() {
        if (!agentStore.activeCase) return;
        try {
            const pRes = await invoke("get_persons_cmd", { caseName: agentStore.activeCase.name });
            const tRes = await invoke("get_targets_json_cmd", { caseName: agentStore.activeCase.name });
            
            let targets = [];
            if (pRes.success) {
                const persons = JSON.parse(pRes.data);
                targets.push(...persons.map(p => ({
                    id: p.id,
                    label: (p.first_name || p.last_name) ? `${p.first_name || ""} ${p.last_name || ""}`.trim() : (p.nicknames[0]?.value || "Sin Identificar"),
                    type: 'Person',
                    description: p.dni ? `DNI: ${p.dni}` : 'Persona'
                })));
            }
            if (tRes.success) {
                const tech = JSON.parse(tRes.data);
                targets.push(...tech.map(t => ({
                    id: t.id,
                    label: t.name,
                    type: t.target_type,
                    description: t.target_type
                })));
            }
            availableTargets = targets;
        } catch (e) {
            console.error("Error cargando objetivos:", e);
        }
    }

    async function loadModels() {
        try {
            const res = await invoke("get_ollama_models");
            if (res.success) {
                availableModels = res.data.map(m => ({
                    id: m.name,
                    label: m.name,
                    type: 'Model',
                    description: `${(m.size / 1e9).toFixed(1)} GB`
                }));
            }
        } catch (e) {
            console.error("Error cargando modelos:", e);
        }
    }

    async function handleInput(e) {
        const val = e.target.value;
        const cursor = e.target.selectionStart;
        const textBefore = val.slice(0, cursor);
        
        // Manejo especial para sub-comandos como /modelo 
        if (textBefore.startsWith('/modelo ') || textBefore === '/modelo ') {
            const query = textBefore.slice(8).toLowerCase();
            if (availableModels.length === 0) await loadModels();
            filteredSuggestions = availableModels.filter(m => m.label.toLowerCase().includes(query));
            suggestionType = 'model';
            showSuggestions = true;
            suggestionIndex = 0;
            return;
        }

        const lastSlash = textBefore.lastIndexOf('/');
        const lastAt = textBefore.lastIndexOf('@');
        
        if (lastSlash > lastAt && lastSlash !== -1 && (lastSlash === 0 || val[lastSlash-1] === ' ')) {
            const query = textBefore.slice(lastSlash + 1).toLowerCase();
            if (query.includes(' ')) { showSuggestions = false; return; }
            filteredSuggestions = commands.filter(c => c.label.toLowerCase().includes(query));
            suggestionType = 'command';
            showSuggestions = true;
            suggestionIndex = 0;
        } else if (lastAt > lastSlash && lastAt !== -1 && (lastAt === 0 || val[lastAt-1] === ' ')) {
            const query = textBefore.slice(lastAt + 1).toLowerCase();
            if (query.includes(' ')) { showSuggestions = false; return; }
            if (availableTargets.length === 0) loadTargets();
            filteredSuggestions = availableTargets.filter(t => t.label.toLowerCase().includes(query));
            suggestionType = 'mention';
            showSuggestions = true;
            suggestionIndex = 0;
        } else {
            showSuggestions = false;
        }
    }

    async function handleSelectSuggestion(item) {
        const val = input;
        const cursor = chatInputRef.selectionStart;
        const textBefore = val.slice(0, cursor);
        const textAfter = val.slice(cursor);
        
        // Si el usuario selecciona 'modelo', transitamos a la selección de modelos
        if (item.id === 'modelo' && suggestionType === 'command') {
            input = "/modelo ";
            showSuggestions = true;
            if (availableModels.length === 0) await loadModels();
            filteredSuggestions = availableModels;
            suggestionType = 'model';
            setTimeout(() => chatInputRef.focus(), 10);
            return;
        }

        const symbol = suggestionType === 'mention' ? '@' : (suggestionType === 'model' ? '' : '/');
        const lastSymbol = (suggestionType === 'mention' || suggestionType === 'command') 
            ? textBefore.lastIndexOf(symbol)
            : textBefore.lastIndexOf(' '); 
        
        if (suggestionType === 'model') {
            input = "/modelo " + item.label + " ";
        } else {
            const newTextBefore = textBefore.slice(0, lastSymbol) + symbol + item.label + " ";
            input = newTextBefore + textAfter;
        }
        
        showSuggestions = false;
        setTimeout(() => chatInputRef.focus(), 20);
    }

    function handleKeyDown(e) {
        if (showSuggestions) {
            if (e.key === "ArrowDown") {
                e.preventDefault();
                suggestionIndex = (suggestionIndex + 1) % filteredSuggestions.length;
            } else if (e.key === "ArrowUp") {
                e.preventDefault();
                suggestionIndex = (suggestionIndex - 1 + filteredSuggestions.length) % filteredSuggestions.length;
            } else if (e.key === "Enter" || e.key === "Tab") {
                if (filteredSuggestions[suggestionIndex]) {
                    e.preventDefault();
                    handleSelectSuggestion(filteredSuggestions[suggestionIndex]);
                }
            } else if (e.key === "Escape") {
                showSuggestions = false;
            }
        } else if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    }

    let chatInputRef = $state(null);
</script>

{#if agentStore.isPanelOpen}
    <aside class="agent-panel">
        <div class="agent-panel__header">
            <div class="agent-panel__header-main">
                <div class="agent-panel__bot-avatar">
                    <img src="/src/assets/bot_sodiic.png" alt="SODIIC_BOT" />
                </div>
                <h3 class="agent-panel__title">SODIIC_BOT</h3>
            </div>
            <button class="agent-panel__close-btn" onclick={() => agentStore.togglePanel()}>✕</button>
        </div>

        <InvestigationToolbar 
            onReport={() => console.log("Generando reporte...")}
            onLink={() => console.log("Vinculando objetivos...")}
        />

        <div class="agent-panel__chat-viewport" bind:this={chatContainer}>
            {#each agentStore.messages as msg}
                {#if msg.role !== "system"}
                    {@const cleanContent = msg.role === "assistant" 
                        ? msg.content
                            .replace(/\{"name":\s*".*?",\s*"parameters":\s*\{[\s\\S]*?\}\}/g, "") 
                            .replace(/\[TOOL_CALLS\][\s\\S]*?$/g, "")
                            .replace(/Llamando a [a-zA-Z_]+\(.*?\)/g, "")
                            .trim()
                        : msg.content.trim()}

                    {#if cleanContent || (msg.role === "user" && msg.image)}
                        <div class="agent-panel__message agent-panel__message--{msg.role}">
                            <div class="agent-panel__bubble">
                                {#if msg.role === "assistant"}
                                    <div class="agent-panel__markdown-body">
                                        {@html snarkdown(cleanContent)}
                                    </div>
                                {:else}
                                    <div class="agent-panel__user-content">
                                        {#if msg.image}
                                            <div class="agent-panel__attached-hint">📎 {msg.image.split(/[\\/]/).pop()}</div>
                                        {/if}
                                        <p class="agent-panel__text">{msg.content}</p>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {/if}
                {/if}
            {/each}

            {#if agentStore.isLoading}
                <div class="agent-panel__task-feedback">
                    <TaskCard 
                        taskName="Investigación en curso" 
                        status={agentStore.statusMessage} 
                        isComplete={false} 
                    />
                </div>
            {/if}
        </div>

        <div class="agent-panel__input-container">
            {#if showSuggestions && filteredSuggestions.length > 0}
                <SuggestionsList 
                    items={filteredSuggestions} 
                    selectedIndex={suggestionIndex} 
                    onSelect={handleSelectSuggestion}
                    type={suggestionType}
                />
            {/if}

            {#if attachedImage}
                <div class="agent-panel__preview-bar">
                    <span>📎 {attachedImage.split(/[\\/]/).pop()}</span>
                    <button class="agent-panel__preview-close" onclick={() => attachedImage = null}>✕</button>
                </div>
            {/if}
            <div class="agent-panel__input-row">
                <button class="agent-panel__icon-btn" onclick={selectImage} title="Adjuntar imagen">📷</button>
                <input 
                    bind:this={chatInputRef}
                    class="agent-panel__input"
                    type="text" 
                    placeholder="Escribe un mensaje..." 
                    bind:value={input}
                    oninput={handleInput}
                    onkeydown={handleKeyDown}
                />
                <button class="agent-panel__send-btn" onclick={handleSend} disabled={agentStore.isLoading}>
                    {#if agentStore.isLoading}
                        <div class="agent-panel__mini-spinner"></div>
                    {:else}
                        ➤
                    {/if}
                </button>
            </div>
            {#if agentStore.isLoading}
                <button class="agent-panel__abort-btn" onclick={() => agentStore.abort()}>DETENER AGENTE ⏹</button>
            {/if}
        </div>
    </aside>
{/if}

<style>
    .agent-panel {
        position: relative;
        width: 100%;
        height: 100%;
        background: var(--bg-secondary);
        border-left: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        z-index: 10;
        box-shadow: -10px 0 30px rgba(0,0,0,0.2);
        flex-shrink: 0;
    }

    .agent-panel__header {
        height: var(--header-height);
        padding: 0 20px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        border-bottom: 1px solid var(--border-color);
        background: var(--bg-primary);
    }

    .agent-panel__header-main {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .agent-panel__bot-avatar {
        width: 32px;
        height: 32px;
        border-radius: 50%;
        overflow: hidden;
        border: 2px solid var(--accent-color);
        background: #000;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .agent-panel__bot-avatar img {
        width: 200%;
        height: 200%;
        object-fit: cover;
        transform: translate(0, 10%);
    }

    .agent-panel__title { font-size: 1rem; margin: 0; text-transform: uppercase; letter-spacing: 1px; }

    .agent-panel__close-btn {
        background: none;
        border: none;
        color: var(--text-muted);
        font-size: 1.1rem;
        cursor: pointer;
        transition: color 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .agent-panel__close-btn:hover { color: var(--text-primary); }

    .agent-panel__chat-viewport {
        flex: 1;
        overflow-y: auto;
        padding: 20px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .agent-panel__message { display: flex; }
    .agent-panel__message--user { justify-content: flex-end; }

    .agent-panel__bubble {
        max-width: 85%;
        padding: 12px 16px;
        border-radius: 12px;
        font-size: 0.9rem;
        overflow-wrap: break-word;
        word-break: break-word;
        word-wrap: break-word;
        overflow: hidden;
    }

    .agent-panel__message--assistant .agent-panel__bubble {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 4px 12px 12px 12px;
    }

    .agent-panel__markdown-body :global(table) {
        border-collapse: collapse;
        width: 100%;
        margin: 8px 0;
        font-size: 0.85rem;
    }
    .agent-panel__markdown-body :global(th), .agent-panel__markdown-body :global(td) {
        border: 1px solid var(--border-color);
        padding: 6px;
        text-align: left;
    }
    .agent-panel__markdown-body :global(th) {
        background: rgba(255,255,255,0.05);
    }
    .agent-panel__markdown-body :global(ul), .agent-panel__markdown-body :global(ol) {
        padding-left: 20px;
        margin: 8px 0;
    }

    .agent-panel__message--user .agent-panel__bubble {
        background: var(--accent-color);
        color: white;
        border-radius: 12px 12px 4px 12px;
    }

    .agent-panel__system-note, .agent-panel__error-note {
        font-size: 0.8rem;
        font-family: var(--font-mono);
        text-align: center;
    }

    .agent-panel__system-note { color: var(--warning-color); }
    .agent-panel__error-note { color: var(--danger-color); }

    .agent-panel__attached-hint {
        font-size: 0.7rem;
        opacity: 0.8;
        margin-bottom: 4px;
        background: rgba(255,255,255,0.1);
        padding: 2px 6px;
        border-radius: 4px;
    }

    .agent-panel__task-feedback {
        margin-top: 10px;
    }

    .agent-panel__input-container {
        padding: 16px;
        background: var(--bg-primary);
        border-top: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        gap: 10px;
        position: relative; /* Crítico para el SuggestionsList */
        overflow: visible; /* IMPORTANTE: Para que el menú no se corte */
    }

    .agent-panel__preview-bar {
        display: flex;
        justify-content: space-between;
        background: var(--bg-tertiary);
        padding: 6px 12px;
        font-size: 0.8rem;
        border-radius: 6px;
    }

    .agent-panel__preview-close {
        background: none;
        border: none;
        color: var(--danger-color);
        cursor: pointer;
    }

    .agent-panel__input-row {
        display: flex;
        gap: 8px;
    }

    .agent-panel__input {
        flex: 1;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 10px 14px;
        border-radius: 8px;
        outline: none;
    }

    .agent-panel__icon-btn {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 8px;
        border-radius: 8px;
        cursor: pointer;
    }

    .agent-panel__send-btn {
        background: var(--accent-color);
        border: none;
        color: white;
        padding: 8px 16px;
        border-radius: 8px;
        cursor: pointer;
    }

    .agent-panel__abort-btn {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid var(--danger-color);
        color: var(--danger-color);
        padding: 6px;
        font-size: 0.75rem;
        font-weight: 700;
        cursor: pointer;
        border-radius: 6px;
    }

    .agent-panel__mini-spinner {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255,255,255,0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: agent-panel-spin 1s linear infinite;
    }

    @keyframes agent-panel-spin { to { transform: rotate(360deg); } }
</style>
