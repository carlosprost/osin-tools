<script>
  // Dashboard component
</script>

<div class="dashboard-grid">
  <div class="card">
    <h3>Escaneos Recientes</h3>
    <p class="text-muted">Sin actividad reciente.</p>
  </div>
  <div class="card">
    <h3>Estado del Sistema</h3>
    <div class="stat-row">
      <span>Carga CPU</span>
      <span class="mono text-accent">12%</span>
    </div>
    <div class="stat-row">
      <span>Memoria</span>
      <span class="mono text-accent">340MB</span>
    </div>
  </div>
</div>

<style>
  .dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--spacing-lg);
  }

  .card {
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: var(--spacing-md);
    transition: transform 0.2s, box-shadow 0.2s;
  }

  .card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    margin-top: 8px;
    font-size: 0.9rem;
  }

  .text-accent {
    color: var(--accent-color);
  }

  .text-muted {
    color: var(--text-muted);
  }

  .mono {
    font-family: var(--font-mono);
  }

  h3 {
    margin-bottom: var(--spacing-sm);
    font-size: 1.1rem;
    color: var(--text-primary);
  }
</style>
