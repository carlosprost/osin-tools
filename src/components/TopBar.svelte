<script>
  import CaseSelector from "./CaseSelector.svelte";
  let { currentView = "" } = $props();
</script>

<header class="top-bar">
  <h2 class="view-title">
    {currentView.charAt(0).toUpperCase() + currentView.slice(1)}
  </h2>
  <div class="actions">
    <CaseSelector />
    <input
      type="text"
      placeholder="Buscar objetivos..."
      class="search-input"
    />
  </div>
</header>

<style>
  .top-bar {
    height: var(--header-height);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--spacing-lg);
    background-color: var(--bg-primary);
    flex-shrink: 0;
    margin-bottom: var(--spacing-lg);
  }

  .view-title {
    font-size: 1.25rem;
    font-weight: 600;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .search-input {
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 4px;
    width: 250px;
    font-size: 0.9rem;
    transition: border-color 0.2s;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-color);
  }
</style>
