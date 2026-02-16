<script>
    // src/components/tools/DomainIntel.svelte
    export let onBack;

    let domain = "";
    let isLoading = false;
    let hasApiKey = false;

    import { onMount } from "svelte";

    onMount(() => {
        const keys = JSON.parse(localStorage.getItem("osint_api_keys") || "{}");
        hasApiKey = !!keys.virustotal;
    });
    let data = null;

    async function scanDomain() {
        if (!domain.trim()) return;

        isLoading = true;

        if (hasApiKey) {
            try {
                const keys = JSON.parse(
                    localStorage.getItem("osint_api_keys") || "{}",
                );
                // Note: VirusTotal standard API might be blocked by CORS in browser.
                // This is a best-effort integration for frontend-only tools.
                const response = await fetch(
                    `https://www.virustotal.com/api/v3/domains/${domain}`,
                    {
                        headers: { "x-apikey": keys.virustotal },
                    },
                );

                if (!response.ok) throw new Error(response.statusText);

                const json = await response.json();
                const attr = json.data.attributes;

                // Map VirusTotal data to our UI structure
                data = {
                    domain: domain,
                    registrar: attr.registrar || "Unknown",
                    creationDate: new Date(
                        attr.creation_date * 1000,
                    ).toLocaleDateString(),
                    expiryDate: "N/A (See WHOIS)",
                    dns: (attr.last_dns_records || []).slice(0, 5).map((r) => ({
                        type: r.type,
                        value: r.value,
                        ttl: r.ttl,
                    })),
                    subdomains: [], // VT doesn't list subdomains in this endpoint directly
                };
            } catch (error) {
                console.error("VirusTotal API Error (likely CORS):", error);
                alert(
                    "VirusTotal Request Failed (CORS or Invalid Key). Switching to Simulation.",
                );
                await simulateScan(); // Fallback
            }
        } else {
            await simulateScan();
        }

        isLoading = false;
    }

    async function simulateScan() {
        // Mock Scan (Original Logic)
        await new Promise((r) => setTimeout(r, 2000));

        data = {
            domain: domain,
            registrar: "GoDaddy.com, LLC",
            creationDate: "2020-05-12",
            expiryDate: "2025-05-12",
            dns: [
                { type: "A", value: "104.21.55.2", ttl: "300" },
                { type: "A", value: "172.67.144.9", ttl: "300" },
                { type: "MX", value: "mail.protonmail.ch", ttl: "3600" },
                {
                    type: "TXT",
                    value: "v=spf1 include:_spf.google.com ~all",
                    ttl: "3600",
                },
            ],
            subdomains: ["api", "mail", "dev", "portal", "vpn"],
        };
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>Inteligencia de Dominio</h3>
    </div>

    {#if !hasApiKey}
        <div class="warning-banner">
            <span
                >⚠️ <strong>Modo Demo:</strong> Configura la
                <b>Clave API de VirusTotal</b> para inteligencia real.</span
            >
        </div>
    {/if}

    <div class="search-box">
        <input
            type="text"
            bind:value={domain}
            placeholder="ejemplo.com"
            on:keydown={(e) => e.key === "Enter" && scanDomain()}
        />
        <button
            class="btn-primary"
            on:click={scanDomain}
            disabled={isLoading || !domain}
        >
            {isLoading ? "Recuperando registros..." : "Analizar Dominio"}
        </button>
    </div>

    {#if data}
        <div class="results-container">
            <div class="section">
                <h4>Información Whois</h4>
                <div class="info-grid">
                    <div class="info-item">
                        <span class="label">Registrador</span>
                        <span class="value">{data.registrar}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">Creado</span>
                        <span class="value">{data.creationDate}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">Expira</span>
                        <span class="value">{data.expiryDate}</span>
                    </div>
                </div>
            </div>

            <div class="section">
                <h4>Registros DNS</h4>
                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th class="type">Tipo</th>
                                <th class="value">Valor</th>
                                <th class="ttl">TTL</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each data.dns as record}
                                <tr>
                                    <td
                                        ><span class="badge {record.type}"
                                            >{record.type}</span
                                        ></td
                                    >
                                    <td class="mono">{record.value}</td>
                                    <td>{record.ttl}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>

            <div class="section">
                <h4>Subdominios Encontrados</h4>
                <div class="tags">
                    {#each data.subdomains as sub}
                        <span class="tag">{sub}.{data.domain}</span>
                    {/each}
                </div>
            </div>
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
        background: rgba(251, 191, 36, 0.1);
        border: 1px solid rgba(251, 191, 36, 0.3);
        color: #fbbf24;
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

    .section {
        margin-bottom: 24px;
        background: var(--bg-secondary);
        padding: 16px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
    }
    h4 {
        margin-top: 0;
        margin-bottom: 16px;
        color: var(--text-secondary);
        font-size: 0.9rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        border-bottom: 1px solid var(--border-color);
        padding-bottom: 8px;
    }

    .info-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
        gap: 16px;
    }
    .info-item {
        display: flex;
        flex-direction: column;
    }
    .label {
        font-size: 0.8rem;
        color: var(--text-muted);
        margin-bottom: 4px;
    }
    .value {
        font-weight: 500;
    }

    .table-container {
        overflow-x: auto;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.9rem;
    }
    th {
        text-align: left;
        color: var(--text-muted);
        padding: 8px;
        border-bottom: 1px solid var(--border-color);
        font-weight: normal;
    }
    td {
        padding: 8px;
        border-bottom: 1px solid var(--border-color);
    }
    tr:last-child td {
        border-bottom: none;
    }

    .badge {
        font-size: 0.75rem;
        padding: 2px 6px;
        border-radius: 4px;
        font-family: var(--font-mono);
        font-weight: 700;
        color: #1a1a1a;
        display: inline-block;
        width: 40px;
        text-align: center;
    }
    .badge.A {
        background: #60a5fa;
    }
    .badge.MX {
        background: #f472b6;
    }
    .badge.TXT {
        background: #fbbf24;
    }
    .mono {
        font-family: var(--font-mono);
        color: var(--text-primary);
    }

    .tags {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }
    .tag {
        background: var(--bg-tertiary);
        padding: 4px 10px;
        border-radius: 12px;
        font-size: 0.85rem;
        border: 1px solid var(--border-color);
        font-family: var(--font-mono);
    }
</style>
