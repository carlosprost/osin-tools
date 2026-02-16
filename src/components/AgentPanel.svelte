<script>
    import { agentStore } from "../lib/agentStore.svelte.js";
    import TaskCard from "./TaskCard.svelte";
    import InvestigationToolbar from "./InvestigationToolbar.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog";
    import * as faceService from "../lib/face_recognition.js";

    let input = $state("");
    let chatContainer = $state(null);
    let attachedImage = $state(null);
    let targetDescriptor = $state(null); // Biometric fingerprint

    $effect(() => {
        if (agentStore.messages && chatContainer) {
            chatContainer.scrollTop = chatContainer.scrollHeight;
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
        const q = input;
        const img = attachedImage;
        input = "";
        attachedImage = null;
        await agentStore.sendMessage(q, img);

        // Scan last message for images to verify
        const lastMsg = agentStore.messages[agentStore.messages.length - 1];
        if (lastMsg && lastMsg.role === "assistant" && targetDescriptor) {
            const regex = /!\[.*?\]\((.*?)\)/g;
            let match;
            while ((match = regex.exec(lastMsg.content)) !== null) {
                verifyFaceInUrl(match[1]);
            }
        }
    }
</script>

{#if agentStore.isPanelOpen}
    <aside class="agent-panel">
        <div class="agent-panel__header">
            <div class="agent-panel__header-main">
                <span class="agent-panel__bot-icon">🤖</span>
                <h3 class="agent-panel__title">Agente OSINT</h3>
            </div>
            <button class="agent-panel__close-btn" onclick={() => agentStore.togglePanel()}>✕</button>
        </div>

        <InvestigationToolbar 
            onReport={() => console.log("Generando reporte...")}
            onLink={() => console.log("Vinculando objetivos...")}
        />

        <div class="agent-panel__chat-viewport" bind:this={chatContainer}>
            {#each agentStore.messages as msg}
                <div class="agent-panel__message agent-panel__message--{msg.role}">
                    <div class="agent-panel__bubble">
                        {#if msg.role === "assistant"}
                            <div class="agent-panel__markdown-body">
                                {@html msg.content
                                    .replace(/\{"name":\s*".*?",\s*"parameters":\s*\{.*?\}\}/g, "") 
                                    .replace(/\n\s*\n/g, "\n") 
                                    .replace(/\n/g, "<br>")
                                    .replace(/\*\*(.*?)\*\*/g, "<b>$1</b>")}
                            </div>
                        {:else if msg.role === "user"}
                            <div class="agent-panel__user-content">
                                {#if msg.image}
                                    <div class="agent-panel__attached-hint">📎 {msg.image.split(/[\\/]/).pop()}</div>
                                {/if}
                                <p class="agent-panel__text">{msg.content}</p>
                            </div>
                        {:else if msg.role === "system"}
                            <div class="agent-panel__system-note">
                                <span>🔧</span> {msg.content}
                            </div>
                        {:else}
                            <div class="agent-panel__error-note">
                                <span>❌</span> {msg.content}
                            </div>
                        {/if}
                    </div>
                </div>
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
            {#if attachedImage}
                <div class="agent-panel__preview-bar">
                    <span>📎 {attachedImage.split(/[\\/]/).pop()}</span>
                    <button class="agent-panel__preview-close" onclick={() => attachedImage = null}>✕</button>
                </div>
            {/if}
            <div class="agent-panel__input-row">
                <button class="agent-panel__icon-btn" onclick={selectImage} title="Adjuntar imagen">📷</button>
                <input 
                    class="agent-panel__input"
                    type="text" 
                    placeholder="Escribe un mensaje..." 
                    bind:value={input}
                    onkeydown={(e) => e.key === "Enter" && handleSend()}
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
                <button class="agent-panel__abort-btn" onclick={() => invoke("abort_agent")}>DETENER AGENTE ⏹</button>
            {/if}
        </div>
    </aside>
{/if}

<style>
    .agent-panel-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0,0,0,0.4);
        backdrop-filter: blur(2px);
        z-index: 1000;
    }

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

    .agent-panel__bot-icon { font-size: 1.5rem; }
    .agent-panel__title { font-size: 1rem; margin: 0; }

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
    }

    .agent-panel__message--assistant .agent-panel__bubble {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 4px 12px 12px 12px;
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
