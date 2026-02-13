<script>
    import { onMount, afterUpdate } from "svelte";
    import { invoke } from "@tauri-apps/api/core";

    let messages = [
        {
            role: "system",
            content: "Agente inicializado. Listo para tareas OSINT.",
        },
    ];
    let input = "";
    let isLoading = false;
    let chatContainer;
    let isTauri = false;

    onMount(async () => {
        // En Tauri v2, si usamos el import, asumimos que estamos en el entorno correcto o manejamos el error en invoke.
        isTauri = true;
        // Si quisieramos verificar estrictamente, podriamos probar un invoke simple al inicio.
    });

    afterUpdate(() => {
        if (chatContainer) {
            chatContainer.scrollTop = chatContainer.scrollHeight;
        }
    });

    async function sendMessage() {
        if (!input.trim() || isLoading) return;

        const userMsg = { role: "user", content: input };
        messages = [...messages, userMsg];
        const currentInput = input;
        input = "";
        isLoading = true;

        try {
            if (isTauri) {
                // const { invoke } = window.__TAURI__.core; // Ya importado arriba
                const response = await invoke("ask_agent", {
                    query: currentInput,
                });

                if (response.success) {
                    messages = [
                        ...messages,
                        { role: "assistant", content: response.data },
                    ];
                } else {
                    messages = [
                        ...messages,
                        { role: "error", content: `Error: ${response.error}` },
                    ];
                }
            } else {
                // Web Mock
                await new Promise((r) => setTimeout(r, 1000));
                messages = [
                    ...messages,
                    {
                        role: "assistant",
                        content: `[MODO WEB] Escanearía "${currentInput}" si estuviera en Tauri.`,
                    },
                ];
            }
        } catch (e) {
            messages = [
                ...messages,
                { role: "error", content: `Error del Sistema: ${e}` },
            ];
        }

        isLoading = false;
    }
</script>

<div class="agent-view">
    <div class="chat-container" bind:this={chatContainer}>
        {#each messages as msg}
            <div class="message {msg.role}">
                <div class="bubble">
                    {#if msg.role === "assistant"}
                        <span class="icon">🤖</span>
                    {:else if msg.role === "user"}
                        <span class="icon">👤</span>
                    {:else if msg.role === "system"}
                        <span class="icon">🔧</span>
                    {:else}
                        <span class="icon">❌</span>
                    {/if}
                    <div class="content">
                        <pre>{msg.content}</pre>
                    </div>
                </div>
            </div>
        {/each}
        {#if isLoading}
            <div class="message assistant">
                <div class="bubble thinking">
                    <span class="icon">🤖</span>
                    <span class="dots">Pensando...</span>
                </div>
            </div>
        {/if}
    </div>

    <div class="input-area">
        <input
            type="text"
            bind:value={input}
            placeholder="Pregunta al Agente (ej: 'Escanear google.com')"
            on:keydown={(e) => e.key === "Enter" && sendMessage()}
        />
        <button on:click={sendMessage} disabled={isLoading}>Enviar</button>
    </div>
</div>

<style>
    .agent-view {
        display: flex;
        flex-direction: column;
        height: 100%;
        max-height: 80vh; /* Adjust based on layout */
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        overflow: hidden;
    }

    .chat-container {
        flex: 1;
        overflow-y: auto;
        padding: 20px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .message {
        display: flex;
    }

    .message.user {
        justify-content: flex-end;
    }

    .bubble {
        max-width: 80%;
        padding: 12px 16px;
        border-radius: 12px;
        display: flex;
        gap: 10px;
        align-items: flex-start;
    }

    .message.user .bubble {
        background: var(--accent-color);
        color: white;
        border-radius: 12px 12px 0 12px;
    }

    .message.assistant .bubble {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        border-radius: 12px 12px 12px 0;
    }

    .message.system .bubble {
        background: rgba(251, 191, 36, 0.1);
        color: #fbbf24;
        border: 1px solid rgba(251, 191, 36, 0.3);
        width: 100%;
        justify-content: center;
    }

    .message.error .bubble {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
        border: 1px solid rgba(239, 68, 68, 0.3);
    }

    .content pre {
        white-space: pre-wrap;
        font-family: var(--font-mono);
        margin: 0;
        font-size: 0.9rem;
    }

    .input-area {
        padding: 16px;
        background: var(--bg-secondary);
        border-top: 1px solid var(--border-color);
        display: flex;
        gap: 10px;
    }

    input {
        flex: 1;
        padding: 12px;
        border-radius: 6px;
        border: 1px solid var(--border-color);
        background: var(--bg-primary);
        color: var(--text-primary);
        font-family: var(--font-mono);
    }

    button {
        padding: 0 24px;
        background: var(--accent-color);
        color: white;
        border: none;
        border-radius: 6px;
        cursor: pointer;
        font-weight: 600;
    }

    button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
