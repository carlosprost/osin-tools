<script>
  export let tool;
</script>

<div class="tool-card">
  <div class="tool-header">
    <span class="tool-icon">{tool.icon}</span>
    <h4>{tool.name}</h4>
    {#if tool.status === "beta"}
      <span class="badge beta">BETA</span>
    {:else if tool.status === "development"}
      <span class="badge dev">DESARROLLO</span>
    {/if}
  </div>
  <p>{tool.description}</p>
  <button class="btn-sm" disabled={tool.status === "development"}>
    {tool.status === "development" ? "Próximamente" : "Iniciar"}
  </button>
</div>

<style>
  .tool-card {
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: var(--spacing-lg);
    transition:
      transform 0.2s,
      border-color 0.2s,
      box-shadow 0.2s;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .tool-card:hover {
    transform: translateY(-4px);
    border-color: var(--accent-color);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .tool-icon {
    font-size: 1.5rem;
  }

  h4 {
    margin: 0;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
  }

  p {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin: 0;
    line-height: 1.5;
    flex: 1;
  }

  .btn-sm {
    padding: 8px 16px;
    font-size: 0.9rem;
    font-weight: 500;
    background-color: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
    align-self: flex-start;
  }

  .btn-sm:hover:not(:disabled) {
    border-color: var(--accent-color);
    color: var(--accent-color);
    background-color: rgba(16, 185, 129, 0.05);
  }

  .btn-sm:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .badge {
    font-size: 0.7rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 700;
  }

  .badge.beta {
    background-color: var(--warning-color);
    color: #1a1a1a;
  }

  .badge.dev {
    background-color: var(--bg-tertiary);
    color: var(--text-muted);
    border: 1px solid var(--border-color);
  }
</style>
