import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

class AgentStore {
  messages = $state([]);
  isLoading = $state(false);
  statusMessage = $state("Listo");
  isPanelOpen = $state(false);
  activeTask = $state(null); // { name, status, steps: [] }
  activeCase = $state(null); // { name, description, targets: [] }
  availableCases = $state([]);

  constructor() {
    // Inicialización asíncrona diferida
    setTimeout(() => this.loadHistory(), 100);
    this.setupEvents();
  }

  async setupEvents() {
    await listen("agent-status", (event) => {
      this.statusMessage = event.payload;
      if (this.activeTask) {
        this.activeTask.status = event.payload;
      }
    });
  }

  async loadHistory() {
    try {
      this.availableCases = await invoke("list_cases");
      // Cargar el último caso si existe o uno por defecto
      if (this.availableCases.length > 0) {
        await this.selectCase(this.availableCases[0]);
      } else {
        this.messages = [
          {
            role: "system",
            content:
              "¡Buenas! Acá el Agente OSINT reportándose. No veo ninguna investigación activa. Creá una nueva para empezar a trackear objetivos.",
          },
        ];
      }
    } catch (e) {
      console.error("Error cargando casos:", e);
    }
  }

  async createCase(name, description = "") {
    try {
      const result = await invoke("create_case", { name, description });
      if (result.success) {
        await this.loadHistory();
        await this.selectCase(name);
        return true;
      }
      return false;
    } catch (e) {
      console.error("Error creando caso:", e);
      return false;
    }
  }

  async selectCase(name) {
    this.isLoading = true;
    this.statusMessage = "Inicializando Agente y Contexto...";
    try {
      const result = await invoke("load_case", { name });
      if (result.success) {
        this.activeCase = JSON.parse(result.data);

        // Cargar historial real desde el backend
        const history = await invoke("get_case_history", { caseName: name });
        try {
          const parsedHistory = JSON.parse(history);
          if (parsedHistory.length > 0) {
            // Filtrar mensajes que son puramente de control técnico [TOOL_CALLS]
            this.messages = parsedHistory.filter((msg) => {
              if (
                msg.role === "assistant" &&
                msg.content.startsWith("[TOOL_CALLS]")
              )
                return false;
              return true;
            });
          } else {
            // Si está vacío, mensaje de bienvenida por defecto para el caso
            this.messages = [
              {
                role: "system",
                content: `Investigación activa: **${name}**. ¿En qué andamos hoy?`,
              },
            ];
          }
        } catch (e) {
          console.error("Error parseando historial:", e);
          this.messages = [
            { role: "system", content: `Error cargando historia de ${name}.` },
          ];
        }
      }
    } catch (e) {
      console.error("Error cargando caso:", e);
    } finally {
      this.isLoading = false;
      this.statusMessage = "Listo";
    }
  }

  saveHistory() {
    if (this.activeCase) {
      invoke("save_case_history", {
        caseName: this.activeCase.name,
        historyJson: JSON.stringify(this.messages),
      });
    }
  }

  async sendMessage(query, imagePath = null) {
    if (!query.trim() && !imagePath) return;

    const userMsg = { role: "user", content: query, image: imagePath };
    this.messages.push(userMsg);
    this.saveHistory();

    this.isLoading = true;
    this.statusMessage = "Pensando...";

    // Reset active task for new request
    this.activeTask = null;

    try {
      const result = await invoke("ask_agent", {
        query: query,
        imagePath: imagePath,
        caseName: this.activeCase?.name || null,
      });

      console.log("DEBUG [agentStore]: Resultado de ask_agent:", result);

      if (result.success) {
        this.messages.push({ role: "assistant", content: result.data });
        this.saveHistory(); // Guardar tras recibir respuesta
      } else {
        this.messages.push({
          role: "error",
          content: result.data || "Error desconocido del agente.",
        });
      }
    } catch (e) {
      this.messages.push({ role: "error", content: `Error del Sistema: ${e}` });
    } finally {
      this.isLoading = false;
      this.statusMessage = "Listo";
      this.activeTask = null;
      this.saveHistory();
    }
  }

  /**
   * Procesa una query contra el agente sin agregar un mensaje de usuario a la UI.
   * Útil para comandos slash que ya fueron agregados manualmente o procesos internos.
   */
  async processQuery(query, imagePath = null) {
    this.isLoading = true;
    this.statusMessage = "Pensando...";
    this.activeTask = null;

    try {
      const result = await invoke("ask_agent", {
        query: query,
        imagePath: imagePath,
        caseName: this.activeCase?.name || null,
      });

      if (result.success) {
        this.messages.push({ role: "assistant", content: result.data });
        this.saveHistory();
      } else {
        this.messages.push({
          role: "error",
          content: result.data || "Error desconocido del agente.",
        });
      }
    } catch (e) {
      this.messages.push({ role: "error", content: `Error del Sistema: ${e}` });
    } finally {
      this.isLoading = false;
      this.statusMessage = "Listo";
      this.activeTask = null;
      this.saveHistory();
    }
  }

  async abort() {
    try {
      await invoke("abort_agent");
      this.isLoading = false;
      this.statusMessage = "Interrumpido";
      this.messages.push({
        role: "system",
        content: "🛑 Investigación detenida por el usuario.",
      });
      this.saveHistory();
    } catch (e) {
      console.error("Error al abortar agente:", e);
    }
  }

  clearHistory() {
    this.messages = [
      {
        role: "system",
        content:
          "Historial borrado. Borrón y cuenta nueva, che. ¿Por dónde arrancamos ahora?",
      },
    ];
    this.saveHistory();
  }

  togglePanel() {
    this.isPanelOpen = !this.isPanelOpen;
  }

  // Helper to identify if a message is a "tool usage" to show it as a task card
  isToolUsage(content) {
    return content.includes("Usando herramienta:");
  }
}

export const agentStore = new AgentStore();
