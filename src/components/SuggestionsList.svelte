<script>
    /**
     * @typedef {Object} Suggestion
     * @property {string} id
     * @property {string} label
     * @property {string} [description]
     * @property {string} [type]
     */

    let { items = [], selectedIndex = 0, onSelect, type = 'command' } = $props();
</script>

<div class="suggestions-list">
    <div class="suggestions-list__header">
        {type === 'command' ? 'Comandos Disponibles' : 'Objetivos del Caso'}
    </div>
    <div class="suggestions-list__content">
        {#each items as item, i}
            <button 
                class="suggestions-list__item" 
                class:suggestions-list__item--active={i === selectedIndex}
                onclick={() => onSelect(item)}
            >
                <div class="suggestions-list__item-main">
                    <span class="suggestions-list__icon">
                        {#if type === 'command'}
                            /
                        {:else if item.type === 'Person'}
                            👤
                        {:else}
                            🎯
                        {/if}
                    </span>
                    <span class="suggestions-list__label">{item.label}</span>
                </div>
                {#if item.description}
                    <span class="suggestions-list__desc">{item.description}</span>
                {/if}
            </button>
        {/each}
        {#if items.length === 0}
            <div class="suggestions-list__empty">No se encontraron resultados</div>
        {/if}
    </div>
</div>

<style>
    .suggestions-list {
        position: absolute;
        bottom: calc(100% + 12px);
        left: 0;
        right: 0;
        background: rgba(26, 27, 38, 0.95);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 12px;
        box-shadow: 0 15px 35px rgba(0,0,0,0.6), 0 0 0 1px rgba(255,255,255,0.05);
        z-index: 9999;
        overflow: hidden;
        backdrop-filter: blur(20px);
        max-height: 400px;
        display: flex;
        flex-direction: column;
        animation: suggestions-slide-up 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }

    @keyframes suggestions-slide-up {
        from { transform: translateY(10px); opacity: 0; }
        to { transform: translateY(0); opacity: 1; }
    }

    .suggestions-list__header {
        padding: 8px 12px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--text-muted);
        background: rgba(255,255,255,0.03);
        border-bottom: 1px solid var(--border-color);
    }

    .suggestions-list__content {
        overflow-y: auto;
    }

    .suggestions-list__item {
        width: 100%;
        padding: 10px 12px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        background: none;
        border: none;
        color: var(--text-primary);
        cursor: pointer;
        transition: background 0.15s, transform 0.1s;
        text-align: left;
    }

    .suggestions-list__item:hover {
        background: rgba(255,255,255,0.05);
    }

    .suggestions-list__item--active {
        background: var(--accent-color) !important;
        color: white;
    }

    .suggestions-list__item-main {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .suggestions-list__icon {
        font-family: var(--font-mono);
        opacity: 0.7;
        width: 16px;
        text-align: center;
    }

    .suggestions-list__label {
        font-size: 0.9rem;
        font-weight: 500;
    }

    .suggestions-list__desc {
        font-size: 0.75rem;
        opacity: 0.6;
        font-style: italic;
    }

    .suggestions-list__item--active .suggestions-list__desc {
        opacity: 0.9;
    }

    .suggestions-list__empty {
        padding: 20px;
        text-align: center;
        font-size: 0.85rem;
        color: var(--text-muted);
    }
</style>
