<script>
    import { onMount, afterUpdate } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog"; // Import dialog
    import * as faceService from "../lib/face_recognition.js";

    let messages = [
        {
            role: "system",
            content:
                "Agente inicializado. Listo para tareas OSINT, análisis de imágenes y biometría.",
        },
    ];
    let input = "";
    let isLoading = false;
    let chatContainer;
    let isTauri = false;
    let attachedImage = null; // Store path
    let targetDescriptor = null; // Biometric fingerprint

    onMount(async () => {
        isTauri = true;
        try {
            await invoke("download_face_models"); // Ensure models exist
            await faceService.loadModels();
            console.log("Sistema biométrico listo");
        } catch (e) {
            console.error("Error iniciando biometría:", e);
            messages = [
                ...messages,
                {
                    role: "error",
                    content:
                        "⚠️ Error cargando modelos biométricos. Ver consola.",
                },
            ];
        }
    });

    afterUpdate(() => {
        if (chatContainer) {
            chatContainer.scrollTop = chatContainer.scrollHeight;
        }
    });

    async function selectImage() {
        try {
            const file = await open({
                multiple: false,
                filters: [
                    { name: "Image", extensions: ["png", "jpg", "jpeg"] },
                ],
            });
            if (file) {
                attachedImage = file;
                const result = await invoke("read_file_base64", { path: file });
                if (result.success) {
                    const img = document.createElement("img");
                    img.src = result.data;
                    img.onload = async () => {
                        const descriptor =
                            await faceService.getFaceDescriptor(img);
                        if (descriptor) {
                            targetDescriptor = descriptor;
                            messages = [
                                ...messages,
                                {
                                    role: "system",
                                    content:
                                        "✅ Rostro detectado y perfilado. Se comparará con las búsquedas.",
                                },
                            ];
                        } else {
                            messages = [
                                ...messages,
                                {
                                    role: "system",
                                    content:
                                        "⚠️ No se detectó rostro en la imagen adjunta.",
                                },
                            ];
                        }
                    };
                }
            }
        } catch (e) {
            console.error("Dialog error or Bio error:", e);
        }
    }

    async function sendMessage() {
        if (!input.trim() && !attachedImage) return;

        const userMsg = input;
        const imgPath = attachedImage;

        messages = [
            ...messages,
            { role: "user", content: userMsg, image: imgPath },
        ];
        input = "";
        attachedImage = null; // Clear attachment after sending
        isLoading = true;

        try {
            const result = await invoke("ask_agent", {
                query: userMsg,
                imagePath: imgPath,
            });

            if (result.success) {
                messages = [
                    ...messages,
                    { role: "assistant", content: result.data },
                ];

                // Automatic Face Verification on content
                if (targetDescriptor) {
                    // Extract Markdown Image links matches
                    const regex = /!\[.*?\]\((.*?)\)/g;
                    let match;
                    while ((match = regex.exec(result.data)) !== null) {
                        const url = match[1];
                        verifyFaceInUrl(url);
                    }
                }
            } else {
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
                        {#if msg.image}
                            <div class="msg-image">
                                <small>📎 Imagen adjunta</small>
                                <!-- We can try to show preview if we have it, but path is local -->
                                <!-- For now just text indication is enough as preview is in input area before sending -->
                            </div>
                        {/if}
                        <!-- Render Markdown (basic) -->
                        <div class="markdown-body">
                            {@html msg.content
                                .replace(/\n/g, "<br>")
                                .replace(/\*\*(.*?)\*\*/g, "<b>$1</b>")}
                        </div>
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

    <div class="input-area-wrapper">
        {#if attachedImage}
            <div class="image-preview">
                <span>📎 {attachedImage.split(/[\\/]/).pop()}</span>
                <button
                    class="remove-btn"
                    on:click={() => (attachedImage = null)}>x</button
                >
            </div>
        {/if}
        <div class="input-area">
            <button
                class="attach-btn"
                on:click={selectImage}
                title="Adjuntar Imagen">📷</button
            >
            <input
                type="text"
                bind:value={input}
                placeholder="Pregunta o adjunta una imagen..."
                on:keydown={(e) => e.key === "Enter" && sendMessage()}
            />
            <button on:click={sendMessage} disabled={isLoading}>Enviar</button>
        </div>
    </div>
</div>

<style>
    /* ... (previous styles) ... */

    .input-area-wrapper {
        background: var(--bg-secondary);
        border-top: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
    }

    .image-preview {
        padding: 5px 15px;
        background: rgba(var(--accent-rgb), 0.1);
        font-size: 0.8rem;
        display: flex;
        align-items: center;
        gap: 10px;
        border-bottom: 1px solid var(--border-color);
    }

    .remove-btn {
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0 5px;
        font-weight: bold;
    }

    .input-area {
        padding: 16px;
        display: flex;
        gap: 10px;
        /* background removed here as wrapper handles it */
    }

    .attach-btn {
        padding: 0 16px;
        background: var(--bg-tertiary);
        color: var(--text-primary);
        border: 1px solid var(--border-color);
    }

    /* Keep existing styles */
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

    /* .content pre removed as we use markdown-body now */

    input {
        flex: 1;
        padding: 12px;
        border-radius: 6px;
        border: 1px solid var(--border-color);
        background: var(--bg-primary);
        color: var(--text-primary);
        font-family: var(--font-mono);
    }

    /* Specific button styling mostly handled by global but ensuring here */
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
