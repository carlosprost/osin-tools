<script>
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { configStore } from "../lib/configStore.svelte.js";

    let showSavedMessage = $state(false);

    onMount(async () => {
        // La carga inicial y persistencia la gestiona el configStore.
        // Solo refrescamos modelos por si acaso al montar la vista de ajustes.
        await configStore.refreshModels();
    });

    async function saveKeys() {
        await configStore.saveConfig();
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
                configStore.config.proxy_url = active ? "socks5h://127.0.0.1:9050" : "";
                saveKeys();
            } else {
                configStore.config.tor_active = !active;
            }
        } catch (e) {
            console.error("Error al activar Tor:", e);
            configStore.config.tor_active = !active;
        }
    }

    async function handleMacChange(event) {
        const active = event.target.checked;
        try {
            const res = await invoke("set_mac_masking", { active });
            if (res.success) {
                saveKeys();
            } else {
                configStore.config.mac_masking_active = !active;
                alert("Error: " + res.error);
            }
        } catch (e) {
            console.error("Error al alternar MAC:", e);
            configStore.config.mac_masking_active = !active;
            alert("Error: " + e + "\n\n(Asegurate de estar ejecutando la app como administrador)");
        }
    }

    async function handleTelegramChange(event) {
        const active = event.target.checked;
        try {
            await invoke(active ? "start_telegram_cmd" : "stop_telegram_cmd");
            saveKeys();
        } catch (e) {
            console.error("Error al alterar Telegram:", e);
            configStore.config.telegram_active = !active;
            alert("Error Telegram: " + e);
        }
    }

    async function clearKeys() {
        if (confirm("¿Estás seguro de que quieres borrar todas las claves y configuraciones?")) {
            const services = [
                'hunter_io', 'shodan', 'virustotal', 'ipapi', 'hibp_api_key',
                'linkedin_session', 'instagram_session', 'twitter_session', 
                'fb_c_user', 'fb_xs',
                'wsl_sudo_password', 'telegram_token', 'telegram_admin_id'
            ];

            for (const service of services) {
                await invoke("delete_secure_secret", { service });
                configStore.config[service] = "";
            }

            configStore.config.tor_active = false;
            configStore.config.mac_masking_active = false;
            configStore.config.proxy_url = "";
            
            localStorage.removeItem("osint_api_keys");
            await configStore.syncWithRust();
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
                    <input type="checkbox" bind:checked={configStore.config.tor_active} onchange={handleTorChange}>
                    <span class="slider round"></span>
                </label>
            </div>
            {#if configStore.config.tor_active}
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
                    <input type="checkbox" bind:checked={configStore.config.mac_masking_active} onchange={handleMacChange}>
                    <span class="slider round"></span>
                </label>
            </div>
            {#if configStore.config.mac_masking_active}
                <div class="settings__status-badge settings__status-badge--mac">
                    🎭 Identidad Física Protegida (MAC Spoofing Activo)
                </div>
                {#if configStore.config.original_mac}
                    <div class="settings__mac-info">
                        <div class="settings__mac-item">
                            <span class="settings__mac-label">MAC Física Real:</span>
                            <span class="settings__mac-value">{configStore.config.original_mac}</span>
                        </div>
                    </div>
                {/if}
            {/if}
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="hibp">HaveIBeenPwned API (Leaks)</label>
                {#if configStore.config.hibp_api_key}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                class="settings__input"
                type="password"
                id="hibp"
                bind:value={configStore.config.hibp_api_key}
                placeholder={configStore.config.hibp_api_key ? "••••••••••••••••" : "Tu API Key..."}
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://haveibeenpwned.com/API/Key" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Configuración Avanzada (Linux / WSL)</h3>
        <p class="settings__card-description text-muted">
            Permite al bot ejecutar herramientas nativas con privilegios de superusuario de forma automatizada.
        </p>
        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="wsl_sudo">WSL Sudo Password</label>
                {#if configStore.config.wsl_sudo_password}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                id="wsl_sudo"
                class="settings__input"
                type="password"
                placeholder={configStore.config.wsl_sudo_password ? "••••••••••••••••" : "Contraseña de tu usuario en Kali/WSL"}
                bind:value={configStore.config.wsl_sudo_password}
            />
            <small class="settings__small text-accent">
                Se guarda de forma segura en el Almacén de Credenciales de Windows (Keyring).
            </small>
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Claves API OSINT</h3>
        <p class="settings__card-description text-muted">
            Las claves se almacenan localmente en tu navegador (LocalStorage).
        </p>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="hunter_io">Hunter.io (Emails)</label>
                {#if configStore.config.hunter_io}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                class="settings__input"
                type="password"
                id="hunter_io"
                bind:value={configStore.config.hunter_io}
                placeholder={configStore.config.hunter_io ? "••••••••••••••••" : "pk_..."}
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://hunter.io/api" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="shodan">Shodan (Infraestructura)</label>
                {#if configStore.config.shodan}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                class="settings__input"
                type="password"
                id="shodan"
                bind:value={configStore.config.shodan}
                placeholder={configStore.config.shodan ? "••••••••••••••••" : "Key..."}
            />
            <small class="settings__small"
                ><a class="settings__link" href="https://account.shodan.io/" target="_blank"
                    >Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="virustotal">VirusTotal (Malware)</label>
                {#if configStore.config.virustotal}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                class="settings__input"
                type="password"
                id="virustotal"
                bind:value={configStore.config.virustotal}
                placeholder={configStore.config.virustotal ? "••••••••••••••••" : "Key..."}
            />
            <small class="settings__small"
                ><a class="settings__link"
                    href="https://www.virustotal.com/gui/user/apikey"
                    target="_blank">Obtener Clave ↗</a
                ></small
            >
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="ipapi">ipapi (IP Intel)</label>
                {#if configStore.config.ipapi}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input
                class="settings__input"
                type="password"
                id="ipapi"
                bind:value={configStore.config.ipapi}
                placeholder={configStore.config.ipapi ? "••••••••••••••••" : "Key..."}
            />
            <small class="settings__small">(Opcional para datos básicos)</small>
        </div>

    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Identidades y Sesiones Autenticadas</h3>
        <p class="settings__card-description text-muted">
            Pega tus cookies de sesión para saltar muros de login y obtener información profunda.
        </p>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="li_at">LinkedIn Session (li_at)</label>
                {#if configStore.config.linkedin_session}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input class="settings__input" type="password" id="li_at" bind:value={configStore.config.linkedin_session} placeholder={configStore.config.linkedin_session ? "••••••••••••••••" : "AQED..."} />
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="insta_sid">Instagram Session (sessionid)</label>
                {#if configStore.config.instagram_session}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input class="settings__input" type="password" id="insta_sid" bind:value={configStore.config.instagram_session} placeholder={configStore.config.instagram_session ? "••••••••••••••••" : "66..."} />
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="x_token">Twitter/X Auth Token (auth_token)</label>
                {#if configStore.config.twitter_session}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input class="settings__input" type="password" id="x_token" bind:value={configStore.config.twitter_session} placeholder={configStore.config.twitter_session ? "••••••••••••••••" : "abc..."} />
        </div>

        <div class="settings__form-group">
            <strong class="settings__label-strong">Facebook Cookie (Sesión)</strong>
            <div class="settings__fb-grid">
                <div class="settings__fb-field">
                    <div class="settings__label-row">
                        <label class="settings__fb-label" for="fb_c_user">c_user</label>
                        {#if configStore.config.fb_c_user}
                            <span class="settings__badge settings__badge--configured-small">✓</span>
                        {/if}
                    </div>
                    <input id="fb_c_user" class="settings__input settings__input--small" type="password" bind:value={configStore.config.fb_c_user} placeholder={configStore.config.fb_c_user ? "••••" : "ID Usuario"} />
                </div>
                <div class="settings__fb-field">
                    <div class="settings__label-row">
                        <label class="settings__fb-label" for="fb_xs">xs (session secret)</label>
                        {#if configStore.config.fb_xs}
                            <span class="settings__badge settings__badge--configured-small">✓</span>
                        {/if}
                    </div>
                    <input id="fb_xs" class="settings__input settings__input--small" type="password" bind:value={configStore.config.fb_xs} placeholder={configStore.config.fb_xs ? "••••" : "Token secreto"} />
                </div>
            </div>
            <small class="settings__small text-muted">Ingresá los valores individuales de las cookies para saltear el login.</small>
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Integración con Telegram (Bot)</h3>
        <p class="settings__card-description text-muted">
            Inicia un bot en background para comunicarte con el Agente desde el celular.
        </p>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="tg_token">Protocolo Bot Token</label>
                {#if configStore.config.telegram_token}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input class="settings__input" type="password" id="tg_token" bind:value={configStore.config.telegram_token} placeholder={configStore.config.telegram_token ? "••••••••••••••••" : "123456789:ABCDefgh..."} />
        </div>

        <div class="settings__form-group">
            <div class="settings__label-row">
                <label class="settings__label" for="tg_admin">ID Administrador (Restricción de Seguridad)</label>
                {#if configStore.config.telegram_admin_id}
                    <span class="settings__badge settings__badge--configured">✓ Configurado</span>
                {/if}
            </div>
            <input class="settings__input" type="text" id="tg_admin" bind:value={configStore.config.telegram_admin_id} placeholder="EJ: 987654321" />
            <small class="settings__small text-accent">Debés interactuar primero con el bot en Telegram para saber tu ID o pedirselo a @userinfobot.</small>
        </div>

        <div class="settings__form-group settings__form-group--toggle">
            <div class="settings__flex-between">
                <div>
                    <strong class="settings__label-strong">Activar Bot MODO ESPERA</strong>
                    <p class="settings__label-hint text-muted small">Arranca o detiene el daemon de escucha asíncrono.</p>
                </div>
                <label class="settings__switch switch">
                    <input type="checkbox" bind:checked={configStore.config.telegram_active} onchange={handleTelegramChange}>
                    <span class="slider round"></span>
                </label>
            </div>
            {#if configStore.config.telegram_active}
                <div class="settings__status-badge" style="background: rgba(56, 189, 248, 0.1); border: 1px solid rgba(56, 189, 248, 0.3); color: #38bdf8; margin-top: 10px; display: inline-block; padding: 5px 10px; border-radius: 4px; font-size: 0.85em;">
                    🤖 Escuchando comandos en Background (Polling Activo)
                </div>
            {/if}
        </div>
    </div>

    <div class="settings__card">
        <h3 class="settings__card-title">Configuración de Inteligencia Artificial (Ollama)</h3>
        <p class="settings__card-description text-muted">
            Configura la conexión al motor de inferencia local.
        </p>

        <div class="settings__form-group">
            <label class="settings__label" for="ollama_url">URL de Ollama Base</label>
            <input 
                class="settings__input" 
                type="text" 
                id="ollama_url" 
                bind:value={configStore.config.ollama_url} 
                onblur={() => configStore.refreshModels()}
                placeholder="http://localhost:11434" 
            />
            <small class="settings__small">Ruta donde SODIIC irá a buscar el modelo de texto y los embeddings vectoriales.</small>
        </div>

        <div class="settings__form-group">
            <label class="settings__label" for="ollama_model">Modelo Principal de Ollama</label>
            <div class="settings__flex-row">
                <select 
                    class="settings__input" 
                    id="ollama_model" 
                    bind:value={configStore.config.ollama_model}
                    onchange={() => configStore.syncWithRust()}
                >
                    {#if configStore.availableModels.length === 0}
                        <option value={configStore.config.ollama_model}>{configStore.config.ollama_model} (Manual)</option>
                    {/if}
                    {#each configStore.availableModels as model}
                        <option value={model}>{model}</option>
                    {/each}
                </select>
                <button 
                    class="settings__btn-icon" 
                    onclick={() => configStore.refreshModels()} 
                    title="Refrescar modelos"
                    disabled={configStore.isLoadingModels}
                >
                    {configStore.isLoadingModels ? '⌛' : '🔄'}
                </button>
            </div>
            <small class="settings__small">
                Seleccioná el modelo local instalado. 
                {#if configStore.isLoadingModels}
                    <span class="text-accent">Buscando modelos...</span>
                {/if}
            </small>
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

    <div class="settings__footer-actions">
        <button class="settings__btn settings__btn--save settings__btn--large" onclick={saveKeys}>
            💾 Guardar Toda la Configuración
        </button>
        <button class="settings__btn settings__btn--clear" onclick={clearKeys}>
            Restablecer App
        </button>
    </div>

    {#if showSavedMessage}
        <div class="settings__toast">✅ Todo guardado exitosamente en el sistema.</div>
    {/if}
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

    .settings__label-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 8px;
    }

    .settings__label-row .settings__label {
        margin-bottom: 0;
    }

    .settings__badge {
        font-size: 0.75rem;
        padding: 2px 8px;
        border-radius: 4px;
        font-weight: 600;
    }

    .settings__badge--configured {
        background: rgba(16, 185, 129, 0.1);
        color: #10b981;
        border: 1px solid rgba(16, 185, 129, 0.3);
    }
    
    .settings__badge--configured-small {
        background: #10b981;
        color: white;
        width: 16px;
        height: 16px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 50%;
        padding: 0;
        font-size: 10px;
    }

    .settings__fb-grid {
        display: grid;
        grid-template-columns: 1fr 2fr;
        gap: 15px;
        margin-bottom: 10px;
        background: rgba(0, 0, 0, 0.2);
        padding: 15px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
    }

    .settings__fb-label {
        color: var(--text-muted);
        font-family: var(--font-mono);
        text-transform: uppercase;
        letter-spacing: 0.5px;
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

    .settings__footer-actions {
        display: flex;
        justify-content: center;
        gap: 15px;
        margin-top: 3rem;
        padding: 2rem;
        background: var(--bg-secondary);
        border-top: 1px solid var(--border-color);
        border-radius: 8px;
        position: sticky;
        bottom: 10px;
        z-index: 100;
        box-shadow: 0 -10px 20px rgba(0, 0, 0, 0.2);
    }

    .settings__btn--large {
        padding: 15px 30px;
        font-size: 1.1rem;
        flex: 1;
        max-width: 400px;
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

    .settings__flex-row {
        display: flex;
        gap: 8px;
        align-items: center;
    }

    .settings__btn-icon {
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 8px 12px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .settings__btn-icon:hover:not(:disabled) {
        border-color: var(--accent-color);
        background: rgba(var(--accent-rgb), 0.1);
    }

    .settings__btn-icon:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
