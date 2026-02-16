<script>
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    // Default keys state
    let apiKeys = $state({
        hunter_io: "",
        shodan: "",
        virustotal: "",
        ipapi: "",
        hibp_api_key: "",
        proxy_url: "",
        tor_active: false,
        mac_masking_active: false,
        original_mac: "",
        linkedin_session: "",
        instagram_session: "",
        twitter_session: "",
        facebook_session: "",
    });

    let showSavedMessage = $state(false);

    onMount(() => {
        // Load keys from localStorage
        const savedKeys = localStorage.getItem("osint_api_keys");
        if (savedKeys) {
            const parsed = JSON.parse(savedKeys);
            // IMPORTANTE: Los servicios siempre inician apagados por seguridad
             apiKeys = { 
                ...apiKeys, 
                ...parsed,
                tor_active: false,
                mac_masking_active: false,
                proxy_url: ""
            };
            // Sincronizar con Rust al cargar
            syncWithRust();
        }
    });

    async function syncWithRust() {
        try {
            await invoke("update_osint_config", {
                config: $state.snapshot(apiKeys)
            });
        } catch (e) {
            console.error("Error sincronizando config con Rust:", e);
        }
    }

    async function saveKeys() {
        localStorage.setItem("osint_api_keys", JSON.stringify($state.snapshot(apiKeys)));
        await syncWithRust();
        showSavedMessage = true;
        setTimeout(() => {
            showSavedMessage = false;
        }, 3000);
    }

    async function handleTorChange(event) {
        const active = event.target.checked;
        try {
            const res = await invoke("set_tor_active", { active });
            if (res.success) {
                // Sincronizar proxy url localmente
                if (active) {
                    apiKeys.proxy_url = "socks5h://127.0.0.1:9050";
                } else {
                    apiKeys.proxy_url = "";
                }
                localStorage.setItem("osint_api_keys", JSON.stringify($state.snapshot(apiKeys)));
            } else {
                // Revertir si falló
                apiKeys.tor_active = !active;
            }
        } catch (e) {
            console.error("Error al activar Tor:", e);
            apiKeys.tor_active = !active;
        }
    }

    async function handleMacChange(event) {
        const active = event.target.checked;
        try {
            const res = await invoke("set_mac_masking", { active });
            if (res.success) {
                localStorage.setItem("osint_api_keys", JSON.stringify($state.snapshot(apiKeys)));
            } else {
                apiKeys.mac_masking_active = !active;
                alert("Error: " + res.error);
            }
        } catch (e) {
            console.error("Error al alternar MAC:", e);
            apiKeys.mac_masking_active = !active;
            alert("Error: " + e + "\n\n(Asegurate de estar ejecutando la app como administrador)");
        }
    }

    function clearKeys() {
        if (
            confirm(
                "¿Estás seguro de que quieres borrar todas las claves y configuraciones?",
            )
        ) {
            apiKeys.hunter_io = "";
            apiKeys.shodan = "";
            apiKeys.virustotal = "";
            apiKeys.ipapi = "";
            apiKeys.hibp_api_key = "";
            apiKeys.proxy_url = "";
            apiKeys.tor_active = false;
            apiKeys.mac_masking_active = false;
            apiKeys.original_mac = "";
            
            localStorage.removeItem("osint_api_keys");
            syncWithRust();
        }
    }
</script>

<div class="settings">
    <div class="settings__header">
        <h2 class="settings__title">Configuración</h2>
        <p class="settings__subtitle">
            Administra las claves de API externas para desbloquear todo el potencial.
        </p>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Investigación Avanzada (OSINT Pro)</h3>
        <p class="settings__card-description text-muted">
            Configura el anonimato y fuentes de datos críticas.
        </p>

        <div class="settings__form-group settings__form-group--toggle">
            <div class="settings__flex-between">
                <div>
                    <strong class="settings__label-strong">Red Tor (Dark Web)</strong>
                    <p class="settings__label-hint text-muted small">Inicia el servicio Tor integrado para acceso anónimo y búsqueda en .onion</p>
                </div>
                <label class="settings__switch switch">
                    <input type="checkbox" bind:checked={apiKeys.tor_active} onchange={handleTorChange}>
                    <span class="slider round"></span>
                </label>
            </div>
            {#if apiKeys.tor_active}
                <div class="settings__status-badge settings__status-badge--tor">
                    🌐 Conectado a la Red Tor (Proxy SOCKS5h activo)
                </div>
            {/if}
        </div>

        <div class="settings__form-group settings__form-group--toggle">
            <div class="settings__flex-between">
                <div>
                    <strong class="settings__label-strong">Enmascaramiento MAC (Spoofing)</strong>
                    <p class="settings__label-hint text-muted small">Cambia la dirección física de tu placa de red activa para mayor anonimato (Requiere Admin)</p>
                </div>
                <label class="settings__switch switch">
                    <input type="checkbox" bind:checked={apiKeys.mac_masking_active} onchange={handleMacChange}>
                    <span class="slider round"></span>
                </label>
            </div>
            {#if apiKeys.mac_masking_active}
                <div class="settings__status-badge settings__status-badge--mac">
                    🎭 Identidad Física Protegida (MAC Spoofing Activo)
                </div>
                {#if apiKeys.original_mac}
                    <div class="settings__mac-info">
                        <div class="settings__mac-item">
                            <span class="settings__mac-label">MAC Física Real:</span>
                            <span class="settings__mac-value">{apiKeys.original_mac}</span>
                        </div>
                        <div class="settings__mac-item">
                            <span class="settings__mac-label">Estado:</span>
                            <span class="settings__mac-value settings__mac-value--masked">Enmascarada (Oculta)</span>
                        </div>
                    </div>
                {/if}
            {/if}
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="hibp">HaveIBeenPwned API (Leaks)</label>
            <input
                class="settings__input"
                type="password"
                id="hibp"
                bind:value={apiKeys.hibp_api_key}
                placeholder="Key..."
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://haveibeenpwned.com/API/Key" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Claves API OSINT</h3>
        <p class="settings__card-description text-muted">
            Las claves se almacenan localmente en tu navegador (LocalStorage).
        </p>

        <div class="settings__form-group">
            <label class="settings__label" for="hunter_io">Hunter.io (Búsqueda de Email)</label>
            <input
                class="settings__input"
                type="password"
                id="hunter_io"
                bind:value={apiKeys.hunter_io}
                placeholder="pk_..."
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://hunter.io/api" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="shodan">Shodan (Búsqueda de Dispositivos)</label>
            <input
                class="settings__input"
                type="password"
                id="shodan"
                bind:value={apiKeys.shodan}
                placeholder="Key..."
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://account.shodan.io/" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="virustotal">VirusTotal (Malware/Dominios)</label>
            <input
                class="settings__input"
                type="password"
                id="virustotal"
                bind:value={apiKeys.virustotal}
                placeholder="Key..."
            />
            <small class="settings__small"
                ><a class="settings__link"
                    href="https://www.virustotal.com/gui/user/apikey"
                    target="_blank">Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="ipapi">ipapi (Datos IP Avanzados)</label>
            <input
                class="settings__input"
                type="password"
                id="ipapi"
                bind:value={apiKeys.ipapi}
                placeholder="Key..."
            />
            <small class="settings__small">(Opcional para datos básicos)</small>
        </div>

        <div class="settings__actions">
            <button class="settings__btn settings__btn--save" onclick={saveKeys}>
                💾 Guardar Configuración
            </button>
            <button class="settings__btn settings__btn--clear" onclick={clearKeys}>
                Borrar Claves
            </button>
        </div>

        {#if showSavedMessage}
            <div class="settings__toast">¡Configuración guardada correctamente!</div>
        {/if}
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Identidades y Sesiones Autenticadas</h3>
        <p class="settings__card-description text-muted">
            Pega tus cookies de sesión para saltar muros de login y obtener información profunda.
        </p>

        <div class="settings__form-group">
            <label class="settings__label" for="li_at">LinkedIn Session (li_at)</label>
            <input class="settings__input" type="password" id="li_at" bind:value={apiKeys.linkedin_session} placeholder="AQED..." />
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="insta_sid">Instagram Session (sessionid)</label>
            <input class="settings__input" type="password" id="insta_sid" bind:value={apiKeys.instagram_session} placeholder="66..." />
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="x_token">Twitter/X Auth Token (auth_token)</label>
            <input class="settings__input" type="password" id="x_token" bind:value={apiKeys.twitter_session} placeholder="abc..." />
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="fb_session">Facebook Cookie String</label>
            <input class="settings__input" type="password" id="fb_session" bind:value={apiKeys.facebook_session} placeholder="c_user=...; xs=..." />
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Aplicación</h3>
        <div class="settings__info-row">
            <span>Versión</span>
            <span class="settings__mono">v0.5.0 (Beta)</span>
        </div>
        <div class="settings__info-row">
            <span>Almacenamiento Usado</span>
            <span class="settings__mono">LocalOnly</span>
        </div>
    </div>
</div>

<style>
    .settings {
        max-width: 800px;
        margin: 0 auto;
        animation: settings-fade-in 0.3s ease;
    }

    .settings__header {
        margin-bottom: 2rem;
    }
    .settings__subtitle {
        color: var(--text-muted);
    }

    .settings__card {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 2rem;
        margin-bottom: 2rem;
    }

    .settings__card-title {
        margin-bottom: 0.5rem;
        color: var(--text-primary);
    }
    .settings__card-description {
        margin-bottom: 1.5rem;
        font-size: 0.9rem;
    }

    .settings__form-group {
        margin-bottom: 1.5rem;
    }

    .settings__label {
        display: block;
        margin-bottom: 8px;
        font-weight: 500;
    }

    .settings__input {
        width: 100%;
        padding: 10px;
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        border-radius: 4px;
        font-family: var(--font-mono);
    }
    .settings__input:focus {
        border-color: var(--accent-color);
        outline: none;
    }

    .settings__small {
        display: block;
        margin-top: 4px;
        font-size: 0.8rem;
    }
    .settings__link {
        color: var(--accent-color);
        text-decoration: none;
    }
    .settings__link:hover {
        text-decoration: underline;
    }

    .settings__actions {
        display: flex;
        gap: 10px;
        margin-top: 2rem;
    }

    .settings__switch {
        position: relative;
        display: inline-block;
        width: 50px;
        height: 24px;
    }

    .settings__switch input {
        opacity: 0;
        width: 0;
        height: 0;
    }

    .slider {
        position: absolute;
        cursor: pointer;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: #334155;
        transition: .4s;
        border: 1px solid #475569;
    }

    .slider:before {
        position: absolute;
        content: "";
        height: 16px;
        width: 16px;
        left: 3px;
        bottom: 3px;
        background-color: white;
        transition: .4s;
    }

    input:checked + .slider {
        background-color: var(--accent-color);
        border-color: var(--accent-color);
    }

    input:checked + .slider:before {
        transform: translateX(26px);
    }

    .slider.round {
        border-radius: 24px;
    }

    .slider.round:before {
        border-radius: 50%;
    }

    .settings__btn {
        padding: 10px 20px;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 600;
        transition: background 0.2s;
    }

    .settings__btn--save {
        background: var(--accent-color);
        color: white;
        border: none;
    }
    .settings__btn--save:hover {
        background: var(--accent-hover);
    }

    .settings__btn--clear {
        background: transparent;
        border: 1px solid var(--danger-color);
        color: var(--danger-color);
    }
    .settings__btn--clear:hover {
        background: rgba(239, 68, 68, 0.1);
    }

    .settings__info-row {
        display: flex;
        justify-content: space-between;
        padding: 10px 0;
        border-bottom: 1px solid var(--border-color);
    }
    .settings__info-row:last-child {
        border-bottom: none;
    }

    .settings__mono {
        font-family: var(--font-mono);
    }

    .settings__toast {
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: var(--accent-color);
        color: white;
        padding: 12px 24px;
        border-radius: 4px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        animation: settings-slide-up 0.3s ease;
        z-index: 1001;
    }

    @keyframes settings-fade-in {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
    @keyframes settings-slide-up {
        from {
            transform: translateY(100%);
            opacity: 0;
        }
        to {
            transform: translateY(0);
            opacity: 1;
        }
    }

    .settings__mac-info {
        margin-top: 15px;
        background: rgba(15, 23, 42, 0.6);
        padding: 12px;
        border-radius: 8px;
        border: 1px dashed #334155;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .settings__mac-item {
        display: flex;
        justify-content: space-between;
        font-size: 0.85em;
    }

    .settings__mac-label {
        color: #94a3b8;
    }

    .settings__mac-value {
        font-family: var(--font-mono);
        color: white;
    }

    .settings__mac-value--masked {
        color: #10b981;
        font-weight: bold;
    }

    .settings__status-badge {
        padding: 5px 10px;
        border-radius: 4px;
        font-size: 0.85em;
        margin-top: 10px;
        display: inline-block;
    }

    .settings__status-badge--tor {
        background: rgba(var(--accent-rgb), 0.1);
        border: 1px solid var(--accent-color);
        color: var(--accent-color);
    }

    .settings__status-badge--mac {
        background: rgba(16, 185, 129, 0.1);
        border: 1px solid rgba(16, 185, 129, 0.3);
        color: #10b981;
    }

    .settings__flex-between {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .settings__label-strong {
        display: block;
        margin-bottom: 2px;
    }
</style>
