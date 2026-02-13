<script>
    // src/components/tools/DomainEmailSearch.svelte
    export let onBack;

    let domain = "";
    let isLoading = false;
    let results = [];
    let searchedDomain = "";
    let hasApiKey = false;

    import { onMount } from "svelte";

    onMount(() => {
        const keys = JSON.parse(localStorage.getItem("osint_api_keys") || "{}");
        hasApiKey = !!keys.hunter_io;
    });

    const commonPatterns = [
        "contact",
        "info",
        "support",
        "sales",
        "admin",
        "hr",
        "jobs",
        "media",
        "press",
    ];
    const firstNames = ["john", "jane", "michael", "sarah", "david", "emily"];

    async function searchEmails() {
        if (!domain.trim()) return;

        isLoading = true;
        results = [];
        searchedDomain = domain;

        if (hasApiKey) {
            try {
                const keys = JSON.parse(
                    localStorage.getItem("osint_api_keys") || "{}",
                );
                const response = await fetch(
                    `https://api.hunter.io/v2/domain-search?domain=${domain}&api_key=${keys.hunter_io}`,
                );
                const data = await response.json();

                if (data.data && data.data.emails) {
                    results = data.data.emails.map((e) => ({
                        email: e.value,
                        type: e.type, // 'personal' or 'generic'
                        confidence: e.confidence,
                    }));
                } else if (data.errors) {
                    alert(`API Error: ${data.errors[0].details}`);
                }
            } catch (error) {
                console.error("Hunter.io API Error:", error);
                alert(
                    "Failed to fetch data from Hunter.io. Check console/network.",
                );
            }
        } else {
            // Demo/Mock logic
            await new Promise((r) => setTimeout(r, 1500));

            // Generic emails
            commonPatterns.forEach((pattern) => {
                if (Math.random() > 0.6) {
                    results.push({
                        email: `${pattern}@${domain}`,
                        type: "Generic",
                        confidence: Math.floor(80 + Math.random() * 20),
                    });
                }
            });

            // Personal emails (simulated)
            firstNames.forEach((name) => {
                if (Math.random() > 0.7) {
                    results.push({
                        email: `${name}@${domain}`,
                        type: "Personal",
                        confidence: Math.floor(40 + Math.random() * 50),
                    });
                }
            });

            // Sort by confidence
            results = results.sort((a, b) => b.confidence - a.confidence);
        }

        isLoading = false;
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>Búsqueda de Emails por Dominio</h3>
    </div>

    {#if !hasApiKey}
        <div class="warning-banner">
            <span
                >⚠️ <strong>Modo Demo:</strong> Configura la
                <b>Clave API de Hunter.io</b> para datos reales.</span
            >
            <button class="btn-sm" on:click={() => window.location.reload()}
                >Configurar Claves</button
            >
        </div>
    {/if}

    <div class="search-box">
        <input
            type="text"
            bind:value={domain}
            placeholder="empresa.com"
            on:keydown={(e) => e.key === "Enter" && searchEmails()}
        />
        <button
            class="btn-primary"
            on:click={searchEmails}
            disabled={isLoading || !domain}
        >
            {isLoading ? "Buscando..." : "Buscar Emails"}
        </button>
    </div>

    {#if searchedDomain && !isLoading}
        <div class="results-header">
            <h4>
                Encontrados {results.length} emails para
                <span class="text-accent">{searchedDomain}</span>
            </h4>
        </div>

        {#if results.length > 0}
            <div class="results-grid">
                {#each results as result}
                    <div class="result-card">
                        <div class="email-row">
                            <span class="email-icon">📧</span>
                            <span class="email-text">{result.email}</span>
                            <button
                                class="copy-btn"
                                on:click={() =>
                                    navigator.clipboard.writeText(result.email)}
                                title="Copiar">📋</button
                            >
                        </div>
                        <div class="meta-row">
                            <span class="tag {result.type.toLowerCase()}"
                                >{result.type}</span
                            >
                            <span
                                class="confidence"
                                title="Puntuación de Confianza"
                            >
                                🎯 {result.confidence}%
                            </span>
                        </div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class="no-results">
                <p>
                    No se encontraron direcciones de email públicas para este
                    dominio.
                </p>
            </div>
        {/if}
    {/if}
</div>

<style>
    .tool-view {
        animation: fadeIn 0.3s ease;
        max-width: 800px;
        margin: 0 auto;
    }
    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: 0;
        }
        to {
            opacity: 1;
            transform: 0;
        }
    }

    .warning-banner {
        background: rgba(251, 191, 36, 0.1);
        border: 1px solid rgba(251, 191, 36, 0.3);
        color: #fbbf24;
        padding: 10px 16px;
        border-radius: 6px;
        margin-bottom: 2rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.9rem;
    }

    .btn-sm {
        background: rgba(251, 191, 36, 0.2);
        border: none;
        color: #fbbf24;
        padding: 4px 12px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 600;
    }
    .btn-sm:hover {
        background: rgba(251, 191, 36, 0.3);
    }

    .header {
        display: flex;
        align-items: center;
        gap: 20px;
        margin-bottom: 2rem;
    }
    .btn-back {
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
    }
    .btn-back:hover {
        color: var(--accent-color);
    }

    .search-box {
        display: flex;
        gap: 10px;
        margin-bottom: 2rem;
    }
    input {
        flex: 1;
        padding: 12px;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        border-radius: 6px;
        font-family: var(--font-mono);
    }
    input:focus {
        border-color: var(--accent-color);
        outline: none;
    }

    .btn-primary {
        padding: 0 24px;
        background: var(--accent-color);
        color: #fff;
        border: none;
        border-radius: 6px;
        cursor: pointer;
        font-weight: 600;
    }
    .btn-primary:hover:not(:disabled) {
        background: var(--accent-hover);
    }
    .btn-primary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .results-header {
        margin-bottom: 1rem;
        border-bottom: 1px solid var(--border-color);
        padding-bottom: 0.5rem;
    }
    .text-accent {
        color: var(--accent-color);
    }

    .results-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1rem;
    }

    .result-card {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 1rem;
        transition: transform 0.2s;
    }
    .result-card:hover {
        transform: translateY(-2px);
        border-color: var(--accent-color);
    }

    .email-row {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 0.8rem;
    }
    .email-text {
        font-family: var(--font-mono);
        font-weight: 500;
        font-size: 0.95rem;
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .copy-btn {
        background: none;
        border: none;
        cursor: pointer;
        opacity: 0.5;
        transition: opacity 0.2s;
    }
    .copy-btn:hover {
        opacity: 1;
    }

    .meta-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.8rem;
    }
    .tag {
        padding: 2px 8px;
        border-radius: 12px;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 0.7rem;
    }
    .tag.generic {
        background: rgba(96, 165, 250, 0.2);
        color: #60a5fa;
    }
    .tag.personal {
        background: rgba(251, 191, 36, 0.2);
        color: #fbbf24;
    }

    .confidence {
        color: var(--text-muted);
    }

    .no-results {
        text-align: center;
        color: var(--text-secondary);
        padding: 2rem;
        background: var(--bg-secondary);
        border-radius: 8px;
        border: 1px dashed var(--border-color);
    }
</style>
