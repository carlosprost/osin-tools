<script>
    import { agentStore } from "../lib/agentStore.svelte.js";
    
    let showCreate = $state(false);
    let name = $state("");
    let desc = $state("");

    async function handleCreate() {
        if (!name.trim()) return;
        const success = await agentStore.createCase(name, desc);
        if (success) {
            name = "";
            desc = "";
            showCreate = false;
        }
    }
</script>

<div class="welcome">
    <div class="welcome__card">
        <div class="welcome__logo-section">
            <div class="welcome__bot-avatar">
                <img src="/src/assets/bot_sodiic.png" alt="SODIIC_BOT" />
            </div>
            <h1 class="welcome__brand">SODIIC</h1>
        </div>
        
        <p class="welcome__intro-text">
            Bienvenido al **Sistema de Organización de Investigaciones e Inteligencia Criminal**. Para comenzar a operar, necesitás inicializar una investigación o abrir una existente.
        </p>

        {#if !showCreate}
            <div class="welcome__actions">
                <button class="welcome__btn welcome__btn--primary" onclick={() => showCreate = true}>
                    <span class="welcome__icon">➕</span> Nueva Investigación
                </button>
                
                {#if agentStore.availableCases.length > 0}
                    <div class="welcome__divider"><span class="welcome__divider-text">o abrí una existente</span></div>
                    <div class="welcome__cases-list">
                        {#each agentStore.availableCases as caseName}
                            <button class="welcome__case-item" onclick={() => agentStore.selectCase(caseName)}>
                                <span class="welcome__case-icon">📂</span> {caseName}
                            </button>
                        {/each}
                    </div>
                {/if}
            </div>
        {:else}
            <div class="welcome__create-form">
                <h2 class="welcome__form-title">Inicializar Investigación</h2>
                <div class="welcome__form-group">
                    <label class="welcome__label" for="caseName_welcome">Nombre del Objetivo / Caso</label>
                    <input class="welcome__input" id="caseName_welcome" type="text" bind:value={name} placeholder="Ej: Red Falcón 2024" />
                </div>
                <div class="welcome__form-group">
                    <label class="welcome__label" for="caseDesc_welcome">Motivo o Descripción</label>
                    <textarea class="welcome__textarea" id="caseDesc_welcome" bind:value={desc} placeholder="Detalles de la operación..."></textarea>
                </div>
                <div class="welcome__form-actions">
                    <button class="welcome__btn welcome__btn--secondary" onclick={() => showCreate = false}>Volver</button>
                    <button class="welcome__btn welcome__btn--primary" onclick={handleCreate}>Empezar Investigación</button>
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    .welcome {
        position: fixed;
        inset: 0;
        background: var(--bg-primary);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
        padding: 20px;
    }

    .welcome__card {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        padding: 40px;
        border-radius: 20px;
        max-width: 500px;
        width: 100%;
        text-align: center;
        box-shadow: 0 30px 60px rgba(0,0,0,0.4);
        animation: welcome-pop 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    }

    @keyframes welcome-pop {
        from { opacity: 0; transform: scale(0.9) translateY(20px); }
        to { opacity: 1; transform: scale(1) translateY(0); }
    }

    .welcome__logo-section {
        margin-bottom: 24px;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
    }

    .welcome__bot-avatar {
        width: 120px;
        height: 120px;
        border-radius: 50%;
        overflow: hidden;
        border: 4px solid var(--accent-color);
        background: #000;
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 0 30px rgba(0, 210, 255, 0.3);
    }

    .welcome__bot-avatar img {
        width: 200%;
        height: 200%;
        object-fit: cover;
        transform: translate(0, 10%);
    }
    
    .welcome__brand {
        font-size: 1.5rem;
        margin: 0;
        color: var(--text-primary);
        letter-spacing: 1px;
    }

    .welcome__brand-accent { color: var(--accent-color); font-weight: 800; }

    .welcome__intro-text {
        color: var(--text-secondary);
        line-height: 1.6;
        margin-bottom: 32px;
        font-size: 0.95rem;
    }

    .welcome__actions {
        display: flex;
        flex-direction: column;
        gap: 20px;
    }

    .welcome__btn {
        padding: 14px 24px;
        border-radius: 12px;
        font-size: 1rem;
        font-weight: 700;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 10px;
        transition: transform 0.2s, background 0.2s;
        border: none;
    }

    .welcome__btn--primary {
        background: var(--accent-color);
        color: white;
    }

    .welcome__btn--primary:hover {
        transform: translateY(-2px);
        background: var(--accent-hover);
    }

    .welcome__btn--secondary {
        background: none;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
    }

    .welcome__btn--secondary:hover {
        background: var(--bg-tertiary);
    }

    .welcome__divider {
        position: relative;
        text-align: center;
        margin: 10px 0;
    }

    .welcome__divider::before {
        content: "";
        position: absolute;
        top: 50%;
        left: 0;
        right: 0;
        height: 1px;
        background: var(--border-color);
    }

    .welcome__divider-text {
        position: relative;
        background: var(--bg-secondary);
        padding: 0 12px;
        font-size: 0.75rem;
        color: var(--text-muted);
        text-transform: uppercase;
    }

    .welcome__cases-list {
        display: grid;
        grid-template-columns: 1fr;
        gap: 10px;
        max-height: 200px;
        overflow-y: auto;
        padding-right: 4px;
    }

    .welcome__case-item {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 12px;
        border-radius: 8px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 12px;
        font-weight: 600;
        transition: all 0.2s;
        text-align: left;
    }

    .welcome__case-item:hover {
        border-color: var(--accent-color);
        background: var(--bg-secondary);
    }

    .welcome__create-form { text-align: left; }
    .welcome__form-title { font-size: 1.25rem; margin-bottom: 20px; color: var(--text-primary); }

    .welcome__form-group {
        margin-bottom: 20px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .welcome__label { font-size: 0.85rem; color: var(--text-secondary); }

    .welcome__input, .welcome__textarea {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 12px;
        border-radius: 8px;
        outline: none;
        font-size: 0.9rem;
        transition: border-color 0.2s;
    }

    .welcome__input:focus, .welcome__textarea:focus {
        border-color: var(--accent-color);
    }

    .welcome__textarea { height: 100px; resize: none; }

    .welcome__form-actions {
        display: flex;
        gap: 12px;
        margin-top: 10px;
    }
</style>
