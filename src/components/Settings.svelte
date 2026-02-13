<script>
    // src/components/Settings.svelte
    import { onMount } from "svelte";

    // Default keys state
    let apiKeys = {
        hunter_io: "",
        shodan: "",
        virustotal: "",
        ipapi: "",
    };

    let showSavedMessage = false;

    onMount(() => {
        // Load keys from localStorage
        const savedKeys = localStorage.getItem("osint_api_keys");
        if (savedKeys) {
            apiKeys = JSON.parse(savedKeys);
        }
    });

    function saveKeys() {
        localStorage.setItem("osint_api_keys", JSON.stringify(apiKeys));
        showSavedMessage = true;
        setTimeout(() => {
            showSavedMessage = false;
        }, 3000);
    }

    function clearKeys() {
        if (
            confirm(
                "¿Estás seguro de que deseas borrar todas las claves API guardadas?",
            )
        ) {
            apiKeys = {
                hunter_io: "",
                shodan: "",
                virustotal: "",
                ipapi: "",
            };
            localStorage.removeItem("osint_api_keys");
        }
    }
</script>

<div class="settings-view">
    <div class="header">
        <h2>Configuración</h2>
        <p class="subtitle">
            Administra las claves de API externas para desbloquear todo el
            potencial.
        </p>
    </div>

    <div class="settings-card">
        <h3>Claves API</h3>
        <p class="text-muted">
            Las claves se almacenan localmente en tu navegador (LocalStorage).
        </p>

        <div class="form-group">
            <label for="hunter_io">Hunter.io (Búsqueda de Email)</label>
            <input
                type="password"
                id="hunter_io"
                bind:value={apiKeys.hunter_io}
                placeholder="pk_..."
            />
            <small
                ><a href="https://hunter.io/api" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="form-group">
            <label for="shodan">Shodan (Búsqueda de Dispositivos)</label>
            <input
                type="password"
                id="shodan"
                bind:value={apiKeys.shodan}
                placeholder="Key..."
            />
            <small
                ><a href="https://account.shodan.io/" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="form-group">
            <label for="virustotal">VirusTotal (Malware/Dominios)</label>
            <input
                type="password"
                id="virustotal"
                bind:value={apiKeys.virustotal}
                placeholder="Key..."
            />
            <small
                ><a
                    href="https://www.virustotal.com/gui/user/apikey"
                    target="_blank">Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="form-group">
            <label for="ipapi">ipapi (Datos IP Avanzados)</label>
            <input
                type="password"
                id="ipapi"
                bind:value={apiKeys.ipapi}
                placeholder="Key..."
            />
            <small>(Opcional para datos básicos)</small>
        </div>

        <div class="actions">
            <button class="btn-save" on:click={saveKeys}>
                💾 Guardar Configuración
            </button>
            <button class="btn-clear" on:click={clearKeys}>
                Borrar Claves
            </button>
        </div>

        {#if showSavedMessage}
            <div class="toast">¡Configuración guardada correctamente!</div>
        {/if}
    </div>

    <div class="settings-card">
        <h3>Aplicación</h3>
        <div class="info-row">
            <span>Versión</span>
            <span class="mono">v0.5.0 (Beta)</span>
        </div>
        <div class="info-row">
            <span>Almacenamiento Usado</span>
            <span class="mono">LocalOnly</span>
        </div>
    </div>
</div>

<style>
    .settings-view {
        max-width: 800px;
        margin: 0 auto;
        animation: fadeIn 0.3s ease;
    }

    .header {
        margin-bottom: 2rem;
    }
    .subtitle {
        color: var(--text-muted);
    }

    .settings-card {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 2rem;
        margin-bottom: 2rem;
    }

    h3 {
        margin-bottom: 0.5rem;
        color: var(--text-primary);
    }
    .text-muted {
        margin-bottom: 1.5rem;
        font-size: 0.9rem;
    }

    .form-group {
        margin-bottom: 1.5rem;
    }

    label {
        display: block;
        margin-bottom: 8px;
        font-weight: 500;
    }

    input {
        width: 100%;
        padding: 10px;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        border-radius: 4px;
        font-family: var(--font-mono);
    }
    input:focus {
        border-color: var(--accent-color);
        outline: none;
    }

    small {
        display: block;
        margin-top: 4px;
        font-size: 0.8rem;
    }
    small a {
        color: var(--accent-color);
        text-decoration: none;
    }
    small a:hover {
        text-decoration: underline;
    }

    .actions {
        display: flex;
        gap: 10px;
        margin-top: 2rem;
    }

    .btn-save {
        background: var(--accent-color);
        color: white;
        border: none;
        padding: 10px 20px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 600;
    }
    .btn-save:hover {
        background: var(--accent-hover);
    }

    .btn-clear {
        background: transparent;
        border: 1px solid var(--danger-color);
        color: var(--danger-color);
        padding: 10px 20px;
        border-radius: 4px;
        cursor: pointer;
    }
    .btn-clear:hover {
        background: rgba(239, 68, 68, 0.1);
    }

    .info-row {
        display: flex;
        justify-content: space-between;
        padding: 10px 0;
        border-bottom: 1px solid var(--border-color);
    }
    .info-row:last-child {
        border-bottom: none;
    }

    .toast {
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: var(--accent-color);
        color: white;
        padding: 12px 24px;
        border-radius: 4px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        animation: slideUp 0.3s ease;
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
    @keyframes slideUp {
        from {
            transform: translateY(100%);
            opacity: 0;
        }
        to {
            transform: 0;
            opacity: 1;
        }
    }
</style>
