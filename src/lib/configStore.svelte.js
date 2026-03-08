import { invoke } from "@tauri-apps/api/core";

const SECURE_SERVICES = [
  "hunter_io",
  "shodan",
  "virustotal",
  "ipapi",
  "hibp_api_key",
  "linkedin_session",
  "instagram_session",
  "twitter_session",
  "fb_c_user",
  "fb_xs",
  "wsl_sudo_password",
  "telegram_token",
  "telegram_admin_id",
];

class ConfigStore {
  config = $state({
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
    fb_c_user: "",
    fb_xs: "",
    wsl_sudo_password: "",
    telegram_token: "",
    telegram_admin_id: "",
    telegram_active: false,
    ollama_url: "http://localhost:11434",
    ollama_model: "llama3.2:latest",
  });

  availableModels = $state([]);
  isLoadingModels = $state(false);

  constructor() {
    this.loadFromStorage();
  }

  async loadFromStorage() {
    // 1. Cargar datos generales de LocalStorage
    const saved = localStorage.getItem("osint_api_keys");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        this.config = { ...this.config, ...parsed };
      } catch (e) {
        console.error("Error cargando config desde localStorage:", e);
      }
    }

    // 2. Cargar SECRETOS desde Keyring (Nativo)
    for (const service of SECURE_SERVICES) {
      try {
        const res = await invoke("get_secure_secret", { service });
        if (res.success && res.data) {
          this.config[service] = res.data;
        }
      } catch (e) {
        console.warn(`No se pudo recuperar el secreto para ${service}:`, e);
      }
    }

    await this.refreshModels();
    await this.syncWithRust();
  }

  async refreshModels() {
    if (!this.config.ollama_url) return;

    this.isLoadingModels = true;
    try {
      const res = await invoke("get_ollama_models", {
        ollamaUrl: this.config.ollama_url,
      });
      if (res.success && res.data) {
        const data = JSON.parse(res.data);
        if (data.models) {
          this.availableModels = data.models.map((m) => m.name);
          if (
            this.availableModels.length > 0 &&
            !this.availableModels.includes(this.config.ollama_model)
          ) {
            console.log(
              "Aviso: El modelo configurado no parece estar en la lista de Ollama.",
            );
          }
        }
      } else {
        console.error("Error de Ollama:", res.error);
        this.availableModels = [];
      }
    } catch (e) {
      console.error("Error conectando con Ollama para listar modelos:", e);
      this.availableModels = [];
    } finally {
      this.isLoadingModels = false;
    }
  }

  async syncWithRust() {
    try {
      await invoke("update_osint_config", {
        newConfig: $state.snapshot(this.config),
      });
    } catch (e) {
      console.error("Error sincronizando config con Rust:", e);
    }
  }

  async saveConfig() {
    const configToSave = { ...$state.snapshot(this.config) };

    // Guardar secretos en Keyring y removerlos de localStorage
    for (const service of SECURE_SERVICES) {
      const value = configToSave[service];
      if (value) {
        await invoke("save_secure_secret", { service, value });
      } else {
        await invoke("delete_secure_secret", { service });
      }
      delete configToSave[service];
    }

    localStorage.setItem("osint_api_keys", JSON.stringify(configToSave));
    await this.syncWithRust();
  }

  async updateField(field, value) {
    this.config[field] = value;
    if (field === "ollama_url") {
      await this.refreshModels();
    }
    await this.syncWithRust();
  }
}

export const configStore = new ConfigStore();
