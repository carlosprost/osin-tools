<script>
    // src/components/tools/IpLookup.svelte
    import { onMount } from "svelte";

    export let onBack; // Function to go back to tools list

    let ipInput = "";
    let isLoading = false;
    let error = null;
    let result = null;
    let hasApiKey = false;

    onMount(() => {
        const keys = JSON.parse(localStorage.getItem("osint_api_keys") || "{}");
        hasApiKey = !!keys.ipapi;
    });

    async function lookupIp() {
        if (!ipInput.trim()) {
            // If empty, fetch own IP
            ipInput = "";
        }

        isLoading = true;
        error = null;
        result = null;

        try {
            // Start with base logic
            let url;
            const keys = JSON.parse(
                localStorage.getItem("osint_api_keys") || "{}",
            );

            if (keys.ipapi) {
                // Use ipapi.com if key exists (Premium)
                // Note: ipapi.com structure is slightly different, but for this demo we assume similar enough or user uses ipapi.co paid key
                // Let's assume the user puts an ipapi.co key for simplicity as the code is built for it
                // https://ipapi.co/api/#introduction
                url = ipInput
                    ? `https://ipapi.co/${ipInput}/json/?key=${keys.ipapi}`
                    : `https://ipapi.co/json/?key=${keys.ipapi}`;
            } else {
                // Free tier (Rate limited)
                url = ipInput
                    ? `https://ipapi.co/${ipInput}/json/`
                    : "https://ipapi.co/json/";
            }

            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`API Error: ${response.status}`);
            }

            const data = await response.json();

            if (data.error) {
                throw new Error(data.reason || "Invalid IP address");
            }

            result = data;
            // If we searched for self, update input to show the IP found
            if (!ipInput) ipInput = data.ip;
        } catch (err) {
            error = err.message;
        } finally {
            isLoading = false;
        }
    }

    // Auto-search logic could go here on mount if desired
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Back</button>
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>Búsqueda de IP</h3>
        <h3>Búsqueda de IP</h3>
    </div>

    {#if !hasApiKey}
        <div class="warning-banner">
            <span
                >ℹ️ Usando <strong>Nivel Gratuito</strong> (Límite Tasas). Añade
                <b>clave ipapi</b> en configuración para mejores límites.</span
            >
        </div>
    {/if}

    <div class="search-box">
        <input
            type="text"
            bind:value={ipInput}
            placeholder="Ingresa Dirección IP (vacío para tu IP)"
            on:keydown={(e) => e.key === "Enter" && lookupIp()}
        />
        <button class="btn-primary" on:click={lookupIp} disabled={isLoading}>
            {isLoading ? "Escaneando..." : "Escanear Objetivo"}
        </button>
    </div>

    {#if error}
        <div class="error-msg">
            ⚠️ {error}
        </div>
    {/if}

    {#if result}
        <div class="result-grid">
            <div class="result-card wide">
                <span class="label">Ubicación</span>
                <span class="value big"
                    >{result.city}, {result.region}, {result.country_name}</span
                >
                <span class="coords">{result.latitude}, {result.longitude}</span
                >
            </div>

            <div class="result-card">
                <span class="label">Dirección IP</span>
                <span class="value mono">{result.ip}</span>
            </div>

            <div class="result-card">
                <span class="label">Red (ASN)</span>
                <span class="value">{result.org} ({result.asn})</span>
            </div>

            <div class="result-card">
                <span class="label">Proveedor (ISP)</span>
                <span class="value">{result.isp}</span>
            </div>

            <div class="result-card">
                <span class="label">Zona Horaria</span>
                <span class="value"
                    >{result.timezone} ({result.utc_offset})</span
                >
            </div>
        </div>

        <div class="raw-data">
            <details>
                <summary>Ver JSON Crudo</summary>
                <pre>{JSON.stringify(result, null, 2)}</pre>
            </details>
        </div>
    {/if}
</div>

<style>
    .tool-view {
        animation: fadeIn 0.3s ease;
        max-width: 800px;
        margin: 0 auto;
    }

    .warning-banner {
        background: rgba(59, 130, 246, 0.1);
        border: 1px solid rgba(59, 130, 246, 0.3);
        color: #60a5fa;
        padding: 8px 12px;
        border-radius: 6px;
        margin-bottom: 1.5rem;
        font-size: 0.85rem;
        text-align: center;
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

    input {
        flex: 1;
        padding: 12px;
        background-color: var(--bg-secondary);
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
        background-color: var(--accent-color);
        color: #fff;
        border: none;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        transition: background-color 0.2s;
    }
    .btn-primary:hover {
        background-color: var(--accent-hover);
    }
    .btn-primary:disabled {
        background-color: var(--text-muted);
        cursor: not-allowed;
    }

    .error-msg {
        padding: 1rem;
        background-color: rgba(239, 68, 68, 0.1);
        border: 1px solid var(--danger-color);
        color: var(--danger-color);
        border-radius: 6px;
        margin-bottom: 1rem;
    }

    .result-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 1rem;
    }

    .result-card {
        background-color: var(--bg-secondary);
        padding: 1rem;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .result-card.wide {
        grid-column: 1 / -1;
        background-color: var(--bg-tertiary);
    }

    .label {
        font-size: 0.8rem;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .value {
        font-weight: 600;
        font-size: 1.1rem;
    }
    .value.big {
        font-size: 1.5rem;
        color: var(--accent-color);
    }
    .value.mono {
        font-family: var(--font-mono);
    }
    .coords {
        font-size: 0.9rem;
        color: var(--text-secondary);
        font-family: var(--font-mono);
    }

    .raw-data {
        margin-top: 2rem;
    }
    details {
        cursor: pointer;
        color: var(--text-secondary);
    }
    pre {
        background-color: #000;
        padding: 1rem;
        border-radius: 6px;
        overflow-x: auto;
        color: var(--accent-color);
        font-family: var(--font-mono);
        font-size: 0.85rem;
    }
</style>
