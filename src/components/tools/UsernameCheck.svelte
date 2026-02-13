<script>
    // src/components/tools/UsernameCheck.svelte
    export let onBack;

    let username = "";
    let isScanning = false;
    let results = [];

    const platforms = [
        { name: "Twitter", icon: "🐦", url: "twitter.com/" },
        { name: "Instagram", icon: "📸", url: "instagram.com/" },
        { name: "GitHub", icon: "🐙", url: "github.com/" },
        { name: "Reddit", icon: "🔴", url: "reddit.com/user/" },
        { name: "Twitch", icon: "🎮", url: "twitch.tv/" },
        { name: "Facebook", icon: "📘", url: "facebook.com/" },
        { name: "TikTok", icon: "🎵", url: "tiktok.com/@" },
        { name: "Pinterest", icon: "📌", url: "pinterest.com/" },
    ];

    async function checkUsername() {
        if (!username.trim()) return;

        isScanning = true;
        results = [];

        // Simulate API calls/checks
        for (const platform of platforms) {
            // Add "loading" state
            const tempId = Math.random();
            results = [
                ...results,
                { ...platform, status: "loading", id: tempId },
            ];

            // Artificial delay to simulate network request
            await new Promise((r) => setTimeout(r, 300 + Math.random() * 500));

            // Mock logic: Random availability (mostly taken for common names)
            const isTaken = Math.random() > 0.5;

            results = results.map((r) =>
                r.id === tempId
                    ? { ...r, status: isTaken ? "taken" : "available" }
                    : r,
            );
        }

        isScanning = false;
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>Verificar Usuario</h3>
    </div>

    <div class="search-box">
        <div class="input-group">
            <span class="prefix">@</span>
            <input
                type="text"
                bind:value={username}
                placeholder="nombre de usuario"
                on:keydown={(e) => e.key === "Enter" && checkUsername()}
            />
        </div>
        <button
            class="btn-primary"
            on:click={checkUsername}
            disabled={isScanning || !username}
        >
            {isScanning ? "Verificando..." : "Verificar Disponibilidad"}
        </button>
    </div>

    {#if results.length > 0}
        <div class="results-list">
            {#each results as result}
                <div
                    class="result-item"
                    class:taken={result.status === "taken"}
                    class:available={result.status === "available"}
                >
                    <div class="platform-info">
                        <span class="icon">{result.icon}</span>
                        <span class="name">{result.name}</span>
                    </div>

                    <div class="status">
                        {#if result.status === "loading"}
                            <span class="loader">•••</span>
                        {:else if result.status === "taken"}
                            <span class="badge taken">ENCONTRADO</span>
                            <a
                                href="https://{result.url}{username}"
                                target="_blank"
                                rel="noopener noreferrer"
                                class="link">Ver Perfil ↗</a
                            >
                        {:else}
                            <span class="badge available">NO ENCONTRADO</span>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>
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
            transform: translateY(0);
        }
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
        font-size: 1rem;
        padding: 0;
    }
    .btn-back:hover {
        color: var(--accent-color);
        text-decoration: underline;
    }

    .search-box {
        display: flex;
        gap: 10px;
        margin-bottom: 2rem;
    }

    .input-group {
        flex: 1;
        display: flex;
        align-items: center;
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        padding-left: 12px;
    }

    .input-group:focus-within {
        border-color: var(--accent-color);
    }

    .prefix {
        color: var(--text-muted);
        font-weight: 600;
    }

    input {
        flex: 1;
        padding: 12px;
        background: none;
        border: none;
        color: var(--text-primary);
        font-family: var(--font-mono);
        outline: none;
    }

    .btn-primary {
        padding: 0 24px;
        background-color: var(--accent-color);
        color: #fff;
        border: none;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        transition: background-color 0.2s;
    }
    .btn-primary:hover:not(:disabled) {
        background-color: var(--accent-hover);
    }
    .btn-primary:disabled {
        background-color: var(--bg-tertiary);
        color: var(--text-muted);
        cursor: not-allowed;
    }

    .results-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .result-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 16px;
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        transition: border-color 0.2s;
    }

    .result-item.taken {
        border-left: 4px solid var(--accent-color);
    }
    .result-item.available {
        border-left: 4px solid var(--danger-color);
    }

    .platform-info {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .status {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .badge {
        font-size: 0.75rem;
        padding: 2px 8px;
        border-radius: 4px;
        font-weight: 700;
        text-transform: uppercase;
    }

    .badge.taken {
        background-color: rgba(16, 185, 129, 0.2);
        color: var(--accent-color);
    }
    .badge.available {
        background-color: rgba(239, 68, 68, 0.2);
        color: var(--danger-color);
    }

    .link {
        font-size: 0.85rem;
        color: var(--text-secondary);
        text-decoration: none;
    }
    .link:hover {
        color: var(--text-primary);
        text-decoration: underline;
    }

    .loader {
        animation: pulse 1s infinite;
        color: var(--text-muted);
    }
    @keyframes pulse {
        0% {
            opacity: 0.5;
        }
        50% {
            opacity: 1;
        }
        100% {
            opacity: 0.5;
        }
    }
</style>
