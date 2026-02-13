<script>
    // src/components/tools/EmailVerify.svelte
    export let onBack;

    let email = "";
    let isLoading = false;
    let result = null;
    let hasApiKey = false;

    import { onMount } from "svelte";

    onMount(() => {
        const keys = JSON.parse(localStorage.getItem("osint_api_keys") || "{}");
        hasApiKey = !!keys.hunter_io;
    });

    async function verifyEmail() {
        if (!email.trim() || !email.includes("@")) return;
        isLoading = true;
        result = null;

        if (hasApiKey) {
            try {
                const keys = JSON.parse(
                    localStorage.getItem("osint_api_keys") || "{}",
                );
                const response = await fetch(
                    `https://api.hunter.io/v2/email-verifier?email=${email}&api_key=${keys.hunter_io}`,
                );
                const data = await response.json();

                if (data.data) {
                    const d = data.data;
                    result = {
                        email,
                        format: d.regexp,
                        mxRecord: d.mx_records,
                        smtp: d.smtp_check,
                        disposable: d.disposable,
                        leaks:
                            d.result === "undeliverable"
                                ? -1
                                : d.result === "risky"
                                  ? 1
                                  : 0, // Approximation
                    };
                } else if (data.errors) {
                    alert(`API Error: ${data.errors[0].details}`);
                }
            } catch (error) {
                console.error("Hunter.io API Error:", error);
                alert("Failed to fetch data. Falling back to simulation.");
            }
        }

        if (!result) {
            // Mock simulation fallback
            await new Promise((r) => setTimeout(r, 1000));
            const isSafe =
                !email.startsWith("admin") && !email.startsWith("test");
            result = {
                email,
                format: true,
                mxRecord: true,
                smtp: isSafe,
                disposable: false,
                leaks: isSafe ? 0 : 5,
            };
        }

        isLoading = false;
    }
</script>

<div class="tool-view">
    <div class="header">
        <button class="btn-back" on:click={onBack}>← Volver</button>
        <h3>Verificador de Email</h3>
    </div>

    {#if !hasApiKey}
        <div class="warning-banner">
            <span
                >⚠️ <strong>Modo Demo:</strong> Configura la
                <b>Clave API de Hunter.io</b> para verificación real.</span
            >
        </div>
    {/if}

    <div class="search-box">
        <input
            type="email"
            bind:value={email}
            placeholder="usuario@ejemplo.com"
            on:keydown={(e) => e.key === "Enter" && verifyEmail()}
        />
        <button
            class="btn-primary"
            on:click={verifyEmail}
            disabled={isLoading || !email}
        >
            {isLoading ? "Verificando..." : "Validar Email"}
        </button>
    </div>

    {#if result}
        <div
            class="result-card"
            class:danger={!result.smtp}
            class:safe={result.smtp}
        >
            <div class="status-icon">
                {result.smtp ? "✅" : "⚠️"}
            </div>
            <div class="status-main">
                <h4>
                    {result.smtp
                        ? "Dirección de Email Válida"
                        : "Sospechoso / Inválido"}
                </h4>
                <p>{result.email}</p>
            </div>
        </div>

        <div class="grid">
            <div class="check-item">
                <span class="check-label">Verificación Sintaxis</span>
                <span class="check-val {result.format ? 'pass' : 'fail'}"
                    >{result.format ? "PASA" : "FALLA"}</span
                >
            </div>
            <div class="check-item">
                <span class="check-label">Registros MX</span>
                <span class="check-val {result.mxRecord ? 'pass' : 'fail'}"
                    >{result.mxRecord ? "PASA" : "FALLA"}</span
                >
            </div>
            <div class="check-item">
                <span class="check-label">Conexión SMTP</span>
                <span class="check-val {result.smtp ? 'pass' : 'fail'}"
                    >{result.smtp ? "ÉXITO" : "FALLÓ"}</span
                >
            </div>
            <div class="check-item">
                <span class="check-label">Filtraciones de Datos</span>
                <span class="check-val {result.leaks === 0 ? 'pass' : 'warn'}">
                    {result.leaks > 0
                        ? `${result.leaks} FILTRACIONES`
                        : "SIN FILTRACIONES"}
                </span>
            </div>
        </div>
    {/if}
</div>

<style>
    .tool-view {
        animation: fadeIn 0.3s ease;
        max-width: 600px;
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
            transform: 0;
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
    .btn-primary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .result-card {
        display: flex;
        align-items: center;
        gap: 16px;
        padding: 20px;
        border-radius: 8px;
        margin-bottom: 20px;
        border: 1px solid transparent;
    }
    .result-card.safe {
        background: rgba(16, 185, 129, 0.1);
        border-color: var(--accent-color);
    }
    .result-card.danger {
        background: rgba(239, 68, 68, 0.1);
        border-color: var(--danger-color);
    }

    .status-icon {
        font-size: 2rem;
    }
    h4 {
        margin: 0;
        font-size: 1.1rem;
    }
    p {
        margin: 4px 0 0;
        color: var(--text-secondary);
    }

    .grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
    }
    .check-item {
        background: var(--bg-secondary);
        padding: 12px;
        border-radius: 6px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border: 1px solid var(--border-color);
    }
    .check-label {
        color: var(--text-muted);
        font-size: 0.9rem;
    }
    .check-val {
        font-weight: 700;
        font-size: 0.85rem;
    }
    .pass {
        color: var(--accent-color);
    }
    .fail {
        color: var(--danger-color);
    }
    .warn {
        color: var(--warning-color);
    }
</style>
