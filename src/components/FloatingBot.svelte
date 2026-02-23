<script>
    import { onMount } from 'svelte';
    import { fade, fly } from 'svelte/transition';
    import { agentStore } from '../lib/agentStore.svelte.js';

    let visible = $state(false);
    let showBubble = $state(false);
    let currentMessage = $state("");
    
    // Draggable state
    let pos = $state({ x: 0, y: 0 });
    let isDragging = $state(false);
    let dragStart = { x: 0, y: 0 };

    const messages = [
        "¡Hola! Soy Sodiic, tu asistente OSINT. ¿En qué te ayudo hoy?",
        "Recordá usar Tor si necesitás anonimato total en la red.",
        "Estoy revisando los objetivos... todo parece en orden.",
        "¿Viste que podés comparar rostros con la herramienta biométrica?",
        "Un buen investigador siempre chequea dos veces sus fuentes.",
        "Si encontrás algo raro, avisame y lo analizamos juntos.",
        "Che, no te olvides de cifrar los reportes antes de exportarlos.",
        "Estoy acá flotando por si me necesitás. ¡Dale gas!",
        "La paciencia es la mejor herramienta de un analista OSINT.",
        "Si necesitás analizar una imagen, arrastrala al panel del agente."
    ];

    function getRandomMessage() {
        return messages[Math.floor(Math.random() * messages.length)];
    }

    function showRandomAdvice() {
        if (isDragging) return; // No mostrar burbuja si estamos arrastrando
        currentMessage = getRandomMessage();
        showBubble = true;
        setTimeout(() => {
            showBubble = false;
        }, 6000);
    }

    function handleMouseDown(e) {
        isDragging = true;
        dragStart = {
            x: e.clientX - pos.x,
            y: e.clientY - pos.y
        };
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    }

    function handleMouseMove(e) {
        if (!isDragging) return;
        pos = {
            x: e.clientX - dragStart.x,
            y: e.clientY - dragStart.y
        };
    }

    function handleMouseUp() {
        isDragging = false;
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
    }

    onMount(() => {
        // Posición inicial (abajo a la derecha)
        pos = { x: 0, y: 0 };

        // Aparece después de un ratito
        setTimeout(() => {
            visible = true;
            setTimeout(showRandomAdvice, 2000);
        }, 1500);

        // Tirar frases cada 3 minutos
        const interval = setInterval(showRandomAdvice, 180000);
        return () => clearInterval(interval);
    });
</script>

{#if visible && !agentStore.isPanelOpen}
    <div 
        class="sodiic-bot" 
        transition:fly={{ y: 50, duration: 800 }}
        style="transform: translate({pos.x}px, {pos.y}px)"
    >
        {#if showBubble}
            <div class="sodiic-bubble" transition:fade>
                <p>{currentMessage}</p>
                <div class="sodiic-bubble-arrow"></div>
            </div>
        {/if}
        
        <button 
            class="sodiic-avatar-container" 
            onclick={showRandomAdvice}
            onmousedown={handleMouseDown}
            aria-label="Pedir consejo a Sodiic"
            type="button"
            class:is-dragging={isDragging}
        >
            <div class="sodiic-avatar">
                <img src="/src/assets/bot_sodiic.png" alt="Sodiic Bot" />
            </div>
            <div class="sodiic-glow"></div>
        </button>
    </div>
{/if}

<style>
    .sodiic-bot {
        position: fixed;
        bottom: 30px;
        right: 30px;
        z-index: 9999;
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        pointer-events: none;
    }

    .sodiic-avatar-container {
        pointer-events: auto;
        cursor: pointer;
        position: relative;
        width: 100px;
        height: 100px;
        background: none;
        border: none;
        padding: 0;
        animation: float 4s ease-in-out infinite;
        transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    }

    .sodiic-avatar-container:hover {
        transform: scale(1.1) rotate(5deg);
    }

    .sodiic-avatar {
        width: 100%;
        height: 100%;
        background: #000;
        border-radius: 50%;
        overflow: hidden;
        border: 3px solid var(--accent-color, #00d2ff);
        box-shadow: 0 0 20px rgba(0, 210, 255, 0.4);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .sodiic-avatar img {
        width: 200%; /* Ajuste de zoom equilibrado */
        height: 200%;
        object-fit: cover;
        transform: translate(0%, 10%); /* Centrado X y ajuste Y */
    }

    .sodiic-glow {
        position: absolute;
        bottom: -15px;
        left: 50%;
        transform: translateX(-50%);
        width: 60px;
        height: 10px;
        background: radial-gradient(ellipse at center, rgba(0, 210, 255, 0.4) 0%, rgba(0, 0, 0, 0) 70%);
        border-radius: 50%;
        filter: blur(5px);
        animation: shadow-size 4s ease-in-out infinite;
    }

    .sodiic-bubble {
        background: var(--bg-tertiary, #1a1a1a);
        color: var(--text-primary, #fff);
        padding: 12px 16px;
        border-radius: 12px;
        border: 1px solid var(--border-color, #333);
        box-shadow: 0 10px 25px rgba(0,0,0,0.5);
        margin-bottom: 15px;
        max-width: 200px;
        font-size: 0.85rem;
        line-height: 1.4;
        position: relative;
        text-align: left;
    }

    .sodiic-bubble-arrow {
        position: absolute;
        bottom: -8px;
        right: 40px;
        width: 0;
        height: 0;
        border-left: 8px solid transparent;
        border-right: 8px solid transparent;
        border-top: 8px solid var(--bg-tertiary, #1a1a1a);
    }

    @keyframes float {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-15px); }
    }

    @keyframes shadow-size {
        0%, 100% { transform: translateX(-50%) scale(1); opacity: 0.4; }
        50% { transform: translateX(-50%) scale(0.7); opacity: 0.2; }
    }
</style>
