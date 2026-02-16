<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { agentStore } from "../lib/agentStore.svelte.js";

  let activeTab = $state("persons"); // "persons" | "technical"
  let persons = $state([]);
  let technicalTargets = $state([]);
  let isLoading = $state(false);
  let error = $state(null);

  // Modal Person State
  let showPersonModal = $state(false);
  let editingPerson = $state(null); // Full person object
  let personTab = $state("basic"); // "basic", "addresses", "jobs", "socials"

  // Modal Technical Target State
  let showTechModal = $state(false);
  let techFormData = $state({
      name: "",
      type: "Domain", // Domain, IP, Email, Other
      category: "Technical"
  });
  
  // Person Basic Form
  let basicFormData = $state({
    first_name: "",
    last_name: "",
    dni: "",
    email: "",
    phone: "",
    birth_date: ""
  });

  // Sub-forms states
  let newAddress = $state({ street: "", number: "", locality: "", state: "", country: "", zip_code: "" });
  let newJob = $state({ title: "", company: "", date_start: "", date_end: "" });
  let newSocial = $state({ platform: "", username: "", url: "" });
  let newNickname = $state(""); // Para input de apodos

  async function loadData() {
    if (!agentStore.activeCase) return;
    isLoading = true;
    error = null;
    try {
        // Cargar Personas
        const pRes = await invoke("get_persons_cmd", { caseName: agentStore.activeCase.name });
        if (pRes.success) persons = JSON.parse(pRes.data);

        // Cargar Technical Targets
        const tRes = await invoke("get_targets_json_cmd", { caseName: agentStore.activeCase.name });
        if (tRes.success) {
            technicalTargets = JSON.parse(tRes.data).filter(t => t.target_type !== "Person");
        }
    } catch (e) {
        error = e.toString();
    } finally {
        isLoading = false;
    }
  }

  // --- PERSON ACTIONS ---

  async function handleDeletePerson(id) {
    if(!confirm("¿Eliminar persona y todos sus datos?")) return;
    await invoke("delete_person_cmd", { caseName: agentStore.activeCase.name, personId: id });
    loadData();
  }

  async function handleSavePersonBasic() {
    try {
        // Validación Flexible: Debe tener al menos Nombre O un Apodo (si está editando y ya tiene apodos)
        // Pero al crear, si no puso nombre, necesitamos al menos un apodo.
        // Como los apodos se agregan en el mismo modal pero requieren ID de persona, 
        // para "Crear" con solo apodo, primero creamos la persona "vacía" y luego le metemos el apodo?
        // O permitimos crear con nombre vacío y luego el usuario agrega apodos?
        // Mejor: Permitimos first_name vacío si el usuario entiende que debe agregar apodos luego.
        // O exigimos: O first_name tiene algo, O (si es edición) tiene apodos.
        
        // Simplificación: Dejamos pasar first_name vacío. El backend lo permite.
        
        const payload = {
            id: editingPerson ? editingPerson.id : crypto.randomUUID(),
            first_name: basicFormData.first_name || null, // Ahora opcional en frontend también
            last_name: basicFormData.last_name || null,
            dni: basicFormData.dni || null,
            email: basicFormData.email || null,
            phone: basicFormData.phone || null,
            birth_date: basicFormData.birth_date || null,
            created_at: editingPerson ? editingPerson.created_at : new Date().toISOString(),
            nicknames: editingPerson ? editingPerson.nicknames : [],
            addresses: editingPerson ? editingPerson.addresses : [],
            jobs: editingPerson ? editingPerson.jobs : [],
            social_profiles: editingPerson ? editingPerson.social_profiles : []
        };

        let res;
        const cn = agentStore.activeCase.name; // Extraer nombre
        
        if (editingPerson) {
            res = await invoke("update_person_cmd", {
                caseName: cn,
                person: payload
            });
        } else {
            res = await invoke("create_person_cmd", {
                caseName: cn,
                person: payload
            });
        }

        if (res.success) {
            if (!editingPerson) {
                closeModal();
                loadData();
            } else {
                editingPerson = JSON.parse(res.data);
                loadData();
                alert("Datos básicos actualizados.");
            }
        } else {
            alert("Error: " + res.error);
        }
    } catch(e) { alert("Error: " + e); }
  }

  // --- SUB ENTITY ACTIONS ---

  async function addNickname() {
    if(!editingPerson || !newNickname.trim()) return;
    const res = await invoke("add_nickname_cmd", {
        caseName: agentStore.activeCase.name,
        personId: editingPerson.id,
        nickname: { id: null, value: newNickname }
    });
    if(res.success) {
        editingPerson.nicknames = [...editingPerson.nicknames, JSON.parse(res.data)];
        newNickname = "";
    }
  }

  async function removeNickname(id) {
      if(!editingPerson) return;
      await invoke("remove_nickname_cmd", { caseName: agentStore.activeCase.name, nicknameId: id });
      editingPerson.nicknames = editingPerson.nicknames.filter(n => n.id !== id);
  }

  async function addAddress() {
      if(!editingPerson) return;
      const res = await invoke("add_address_cmd", {
          caseName: agentStore.activeCase.name,
          personId: editingPerson.id,
          address: { ...newAddress, id: null } 
      });
      if(res.success) {
          editingPerson.addresses = [...editingPerson.addresses, JSON.parse(res.data)];
          newAddress = { street: "", number: "", locality: "", state: "", country: "", zip_code: "" };
      }
  }

  async function removeAddress(id) {
      if(!editingPerson) return;
      await invoke("remove_address_cmd", { caseName: agentStore.activeCase.name, addressId: id });
      editingPerson.addresses = editingPerson.addresses.filter(a => a.id !== id);
  }

  async function addJob() {
      if(!editingPerson) return;
      const res = await invoke("add_job_cmd", {
          caseName: agentStore.activeCase.name,
          personId: editingPerson.id,
          job: { ...newJob, id: null }
      });
      if(res.success) {
          editingPerson.jobs = [...editingPerson.jobs, JSON.parse(res.data)];
          newJob = { title: "", company: "", date_start: "", date_end: "" };
      }
  }

  async function removeJob(id) {
       if(!editingPerson) return;
      await invoke("remove_job_cmd", { caseName: agentStore.activeCase.name, jobId: id });
      editingPerson.jobs = editingPerson.jobs.filter(j => j.id !== id);
  }

  async function addSocial() {
      if(!editingPerson) return;
      const res = await invoke("add_social_cmd", {
          caseName: agentStore.activeCase.name,
          personId: editingPerson.id,
          social: { ...newSocial, id: null }
      });
      if(res.success) {
          editingPerson.social_profiles = [...editingPerson.social_profiles, JSON.parse(res.data)];
          newSocial = { platform: "", username: "", url: "" };
      }
  }

  async function removeSocial(id) {
      if(!editingPerson) return;
      await invoke("remove_social_cmd", { caseName: agentStore.activeCase.name, socialId: id });
      editingPerson.social_profiles = editingPerson.social_profiles.filter(s => s.id !== id);
  }

  // --- TECHNICAL TARGET ACTIONS ---

  async function handleSaveTechTarget() {
      const payload = {
          id: crypto.randomUUID(),
          name: techFormData.name,
          target_type: techFormData.type,
          data: {}, 
          linked_targets: [],
          created_at: new Date().toISOString()
      };
      
      const res = await invoke("create_target_cmd", {
          caseName: agentStore.activeCase.name,
          target: payload,
          category: techFormData.category
      });
      
      if(res.success) {
          showTechModal = false;
          techFormData.name = "";
          loadData();
      } else {
          alert("Error: " + res.error);
      }
  }

  // --- UI HELPERS ---

  function openNewPersonModal() {
    editingPerson = null;
    basicFormData = { first_name: "", last_name: "", dni: "", email: "", phone: "", birth_date: "" };
    personTab = "basic";
    showPersonModal = true;
  }

  function openEditPersonModal(p) {
    editingPerson = p;
    basicFormData = {
        first_name: p.first_name,
        last_name: p.last_name,
        dni: p.dni,
        email: p.email,
        phone: p.phone,
        birth_date: p.birth_date
    };
    personTab = "basic";
    showPersonModal = true;
  }

  function closeModal() {
    showPersonModal = false;
    showTechModal = false;
  }

  $effect(() => {
    if(agentStore.activeCase) loadData();
  });
</script>

<div class="targets-view">
  <div class="view-header">
    <h2>Gestión de Objetivos</h2>
    <div class="tabs">
        <button class:active={activeTab === "persons"} onclick={() => activeTab = "persons"}>Personas</button>
        <button class:active={activeTab === "technical"} onclick={() => activeTab = "technical"}>Datos Técnicos</button>
    </div>
  </div>

  <div class="content">
    {#if activeTab === "persons"}
        <div class="toolbar">
            <button class="btn-primary" onclick={openNewPersonModal}>+ Nueva Persona</button>
        </div>
        {#if persons.length === 0}
            <div class="empty-state">No hay personas registradas.</div>
        {:else}
            <div class="grid">
                {#each persons as p}
                    <div class="card person-card">
                        <div class="card-header">
                            <h3>
                                {#if p.first_name}
                                    {p.first_name} {p.last_name || ""}
                                {:else if p.nicknames && p.nicknames.length > 0}
                                    "{p.nicknames[0].value}"
                                {:else}
                                    Sin Identificar
                                {/if}
                            </h3>
                            <div class="actions">
                                <button class="btn-icon" onclick={() => openEditPersonModal(p)}>✏️</button>
                                <button class="btn-icon delete" onclick={() => handleDeletePerson(p.id)}>🗑️</button>
                            </div>
                        </div>
                        <div class="card-body">
                            <small>DNI: {p.dni || "N/A"}</small>
                            <div class="badges">
                                <span class="badge" title="Direcciones">📍 {p.addresses.length}</span>
                                <span class="badge" title="Trabajos">💼 {p.jobs.length}</span>
                                <span class="badge" title="Redes">🔗 {p.social_profiles.length}</span>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}

    {:else}
        <!-- TECHNICAL TAB -->
        <div class="toolbar">
            <button class="btn-primary" onclick={() => showTechModal = true}>+ Nuevo Objetivo Técnico</button>
        </div>
        <div class="table-container">
            <table>
                <thead>
                    <tr>
                        <th>Tipo</th>
                        <th>Nombre / Identificador</th>
                        <th>Creado</th>
                    </tr>
                </thead>
                <tbody>
                    {#each technicalTargets as t}
                        <tr>
                            <td><span class="type-badge">{t.target_type}</span></td>
                            <td>{t.name}</td>
                            <td>{new Date(t.created_at).toLocaleDateString()}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
            {#if technicalTargets.length === 0}
                <div class="empty-state">No hay objetivos técnicos registrados.</div>
            {/if}
        </div>
    {/if}
  </div>

  <!-- PERSON MODAL -->
  {#if showPersonModal}
    <div class="modal-backdrop">
        <div class="modal large">
            <h3>{editingPerson ? "Editar Persona" : "Nueva Persona"}</h3>
            
            <div class="modal-tabs">
                <button class:active={personTab === "basic"} onclick={() => personTab = "basic"}>Básico</button>
                {#if editingPerson}
                    <button class:active={personTab === "addresses"} onclick={() => personTab = "addresses"}>Direcciones</button>
                    <button class:active={personTab === "jobs"} onclick={() => personTab = "jobs"}>Trabajos</button>
                    <button class:active={personTab === "socials"} onclick={() => personTab = "socials"}>Redes</button>
                {/if}
            </div>

            <div class="modal-body">
                {#if personTab === "basic"}
                    <form onsubmit={(e) => { e.preventDefault(); handleSavePersonBasic(); }}>
                        <div class="form-row">
                            <input type="text" placeholder="Nombre (Opcional)" bind:value={basicFormData.first_name} />
                            <input type="text" placeholder="Apellido" bind:value={basicFormData.last_name} />
                        </div>
                        
                        <!-- APODOS SECTION (Solo visible en edición para simplificar flujo) -->
                        {#if editingPerson}
                            <div class="form-group" style="margin-bottom: 10px;">
                                <label for="nickname_input" style="font-size: 0.85rem; color: var(--text-muted);">Apodos / Alias</label>
                                <div class="tags-input">
                                    {#each editingPerson.nicknames as nick}
                                        <span class="tag">
                                            {nick.value}
                                            <button type="button" onclick={() => removeNickname(nick.id)}>×</button>
                                        </span>
                                    {/each}
                                    <div class="input-wrapper">
                                        <input 
                                            id="nickname_input"
                                            type="text" 
                                            placeholder="+ Agregar Apodo" 
                                            bind:value={newNickname} 
                                            onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addNickname())}
                                        />
                                        <button type="button" class="btn-small p-1" onclick={addNickname}>OK</button>
                                    </div>
                                </div>
                            </div>
                        {:else}
                            <div class="info-note" style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 10px;">
                                * Podrás agregar Apodos después de crear el perfil básico.
                            </div>
                        {/if}

                        <div class="form-row">
                            <input type="text" placeholder="DNI" bind:value={basicFormData.dni} />
                            <input type="date" placeholder="Fecha Nacimiento" bind:value={basicFormData.birth_date} />
                        </div>
                        <div class="form-row">
                            <input type="email" placeholder="Email" bind:value={basicFormData.email} />
                            <input type="tel" placeholder="Teléfono" bind:value={basicFormData.phone} />
                        </div>
                        <div class="modal-actions">
                            {#if !editingPerson}<button type="button" class="btn-secondary" onclick={closeModal}>Cancelar</button>{/if}
                            <button type="submit" class="btn-primary">{editingPerson ? "Actualizar Básico" : "Crear Persona"}</button>
                        </div>
                    </form>
                {:else if personTab === "addresses"}
                    <div class="sub-list">
                        {#each editingPerson.addresses as addr}
                            <div class="item">
                                <span>{addr.street} {addr.number}, {addr.locality}</span>
                                <button onclick={() => removeAddress(addr.id)}>❌</button>
                            </div>
                        {/each}
                    </div>
                    <div class="add-form">
                        <input type="text" placeholder="Calle" bind:value={newAddress.street} />
                        <input type="text" placeholder="Altura" bind:value={newAddress.number} style="width: 80px;" />
                        <input type="text" placeholder="Localidad" bind:value={newAddress.locality} />
                        <button class="btn-small" onclick={addAddress}>+</button>
                    </div>
                {:else if personTab === "jobs"}
                    <div class="sub-list">
                        {#each editingPerson.jobs as job}
                            <div class="item">
                                <span>{job.title} en {job.company}</span>
                                <button onclick={() => removeJob(job.id)}>❌</button>
                            </div>
                        {/each}
                    </div>
                     <div class="add-form">
                        <input type="text" placeholder="Puesto" bind:value={newJob.title} />
                        <input type="text" placeholder="Empresa" bind:value={newJob.company} />
                        <button class="btn-small" onclick={addJob}>+</button>
                    </div>
                {:else if personTab === "socials"}
                     <div class="sub-list">
                        {#each editingPerson.social_profiles as soc}
                            <div class="item">
                                <span>{soc.platform}: {soc.username}</span>
                                <button onclick={() => removeSocial(soc.id)}>❌</button>
                            </div>
                        {/each}
                    </div>
                     <div class="add-form">
                        <input type="text" placeholder="Plataforma" bind:value={newSocial.platform} />
                        <input type="text" placeholder="Usuario" bind:value={newSocial.username} />
                        <button class="btn-small" onclick={addSocial}>+</button>
                    </div>
                {/if}
            </div>

            <div class="modal-footer">
                <button class="btn-secondary" onclick={closeModal}>Cerrar</button>
            </div>
        </div>
    </div>
  {/if}

  <!-- TECH TARGET MODAL -->
  {#if showTechModal}
     <div class="modal-backdrop">
        <div class="modal">
            <h3>Nuevo Objetivo Técnico</h3>
             <form onsubmit={(e) => { e.preventDefault(); handleSaveTechTarget(); }}>
                <div class="form-group">
                    <label for="t_type">Tipo</label>
                    <select id="t_type" bind:value={techFormData.type}>
                        <option value="Domain">Dominio</option>
                        <option value="IP">IP</option>
                        <option value="Email">Email</option>
                        <option value="Other">Otro</option>
                    </select>
                </div>
                 <div class="form-group">
                    <label for="t_identifier">Identificador (Nombre/IP/Email)</label>
                    <input id="t_identifier" type="text" required bind:value={techFormData.name} />
                </div>
                <div class="modal-actions">
                     <button type="button" class="btn-secondary" onclick={closeModal}>Cancelar</button>
                    <button type="submit" class="btn-primary">Guardar</button>
                </div>
             </form>
        </div>
     </div>
  {/if}

</div>

<style>
  .targets-view { padding: 20px; color: var(--text-primary); }
  .view-header { display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-color); padding-bottom: 15px; margin-bottom: 20px; }
  .tabs button { background: none; border: none; padding: 10px 20px; cursor: pointer; color: var(--text-muted); font-size: 1rem; border-bottom: 2px solid transparent; }
  .tabs button.active { color: var(--accent-color); border-bottom-color: var(--accent-color); }
  .toolbar { margin-bottom: 15px; display: flex; justify-content: flex-end; }
  
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 15px; }
  .card { background: var(--bg-secondary); border: 1px solid var(--border-color); padding: 15px; border-radius: 8px; }
  .card-header { display: flex; justify-content: space-between; margin-bottom: 10px; }
  .badges { display: flex; gap: 10px; margin-top: 10px; }
  .badge { background: var(--bg-tertiary); padding: 2px 8px; border-radius: 12px; font-size: 0.8rem; }
  
  .table-container table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 10px; border-bottom: 1px solid var(--border-color); }
  .type-badge { background: var(--bg-tertiary); padding: 2px 6px; border-radius: 4px; font-size: 0.85rem; }

  /* Modals */
  .modal-backdrop { position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.6); display: flex; justify-content: center; align-items: center; z-index: 999; }
  .modal { background: var(--bg-secondary); padding: 20px; border-radius: 8px; width: 400px; box-shadow: 0 5px 15px rgba(0,0,0,0.3); }
  .modal.large { width: 600px; }
  
  .modal-tabs { display: flex; border-bottom: 1px solid var(--border-color); margin-bottom: 15px; }
  .modal-tabs button { flex: 1; background: none; border: none; padding: 10px; cursor: pointer; color: var(--text-muted); }
  .modal-tabs button.active { color: var(--accent-color); border-bottom: 2px solid var(--accent-color); }

  .form-row { display: flex; gap: 10px; margin-bottom: 10px; }
  input, select { width: 100%; padding: 8px; background: var(--bg-primary); border: 1px solid var(--border-color); color: var(--text-primary); border-radius: 4px; }
  
  .sub-list .item { display: flex; justify-content: space-between; padding: 5px; border-bottom: 1px solid var(--border-color); align-items: center; }
  .add-form { display: flex; gap: 5px; margin-top: 10px; }
  
  .btn-primary { background: var(--accent-color); color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; }
  .btn-secondary { background: transparent; border: 1px solid var(--border-color); color: var(--text-muted); padding: 8px 16px; border-radius: 4px; cursor: pointer; }
  .btn-small { padding: 4px 8px; background: var(--accent-color); color: white; border: none; border-radius: 4px; cursor: pointer; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 15px; }
  .modal-footer { margin-top: 15px; display: flex; justify-content: flex-end; border-top: 1px solid var(--border-color); padding-top: 10px; }

  /* Tags for Nicknames */
  .tags-input { display: flex; flex-wrap: wrap; gap: 5px; background: var(--bg-primary); padding: 5px; border: 1px solid var(--border-color); border-radius: 4px; }
  .tag { background: var(--bg-tertiary); padding: 2px 8px; border-radius: 12px; font-size: 0.85rem; display: flex; align-items: center; gap: 5px; }
  .tag button { background: none; border: none; cursor: pointer; color: var(--text-muted); font-size: 1rem; line-height: 1; padding: 0; }
  .tag button:hover { color: var(--accent-color); }
  .tags-input .input-wrapper { flex: 1; min-width: 120px; display: flex; gap: 5px; }
  .tags-input input { border: none; background: transparent; padding: 5px; outline: none; flex: 1; }
  .p-1 { padding: 2px 6px; font-size: 0.7rem; }
</style>
