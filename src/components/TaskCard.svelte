<script>
    let { taskName = "Tarea", status = "Iniciando...", isComplete = false } = $props();
</script>

<div class="task-card" class:complete={isComplete}>
    <div class="task-header">
        <div class="status-icon">
            {#if isComplete}
                <span class="check">✅</span>
            {:else}
                <div class="spinner"></div>
            {/if}
        </div>
        <div class="task-info">
            <h4 class="task-name">{taskName}</h4>
            <p class="task-status">{status}</p>
        </div>
    </div>
</div>

<style>
    .task-card {
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 12px;
        margin: 8px 0;
        transition: all 0.3s ease;
        border-left: 4px solid var(--accent-color);
    }

    .task-card.complete {
        border-left-color: #10b981;
        opacity: 0.9;
    }

    .task-header {
        display: flex;
        gap: 12px;
        align-items: center;
    }

    .status-icon {
        width: 24px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid rgba(16, 185, 129, 0.2);
        border-top-color: var(--accent-color);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    .task-info {
        flex: 1;
    }

    .task-name {
        font-size: 0.9rem;
        font-weight: 600;
        margin: 0;
        color: var(--text-primary);
    }

    .task-status {
        font-size: 0.75rem;
        color: var(--text-secondary);
        margin: 2px 0 0 0;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
