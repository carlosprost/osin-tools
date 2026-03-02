<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { agentStore } from "../lib/agentStore.svelte.js";

  let activeTab = $state("persons"); // "persons" | "technical"
  let persons = $state([]);
  let technicalTargets = $state([]);
  let isLoading = $state(false);
  let error = $state(null);

  // Detail Modal State
  let showDetailModal = $state(false);
  let viewingPerson = $state(null);
  let viewingTechTarget = $state(null);
  let showTechDetailModal = $state(false);

  // Modal Person State
  let showPersonModal = $state(false);
  let editingPerson = $state(null); // Full person object
  let personTab = $state("basic"); // "basic", "addresses", "jobs", "socials"

  // Modal Technical Target State
  let showTechModal = $state(false);
  let techFormData = $state({
      id: null,
      name: "",
      type: "Domain", // Domain, IP, Email, Other
      category: "Technical",
      data: {}, // Para atributos clave-valor
      created_at: null
  });

  // Temporales para el input de hallazgos
  let newTechKey = $state("");
  let newTechVal = $state("");
  
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
          id: techFormData.id || crypto.randomUUID(),
          name: techFormData.name,
          target_type: techFormData.type,
          category: techFormData.category, // Agregado aquí dentro del objeto Target
          data: techFormData.data, 
          linked_targets: [],
          created_at: techFormData.created_at || new Date().toISOString()
      };
      
      const res = await invoke("create_target_cmd", {
          caseName: agentStore.activeCase.name,
          target: payload,
          category: techFormData.category
      });
      
      if(res.success) {
          showTechModal = false;
          techFormData = { id: null, name: "", type: "Domain", category: "Technical", data: {}, created_at: null };
          loadData();
      } else {
          alert("Error: " + res.error);
      }
  }

  async function handleDeleteTechTarget(id) {
      if(!confirm("¿Seguro que querés borrar este objetivo técnico? Se perderán todos sus hallazgos asociados.")) return;
      try {
          const res = await invoke("delete_target_cmd", {
              caseName: agentStore.activeCase.name,
              targetId: id
          });
          if(res.success) loadData();
          else alert("Error: " + res.error);
      } catch(e) { alert("Error: " + e); }
  }

  function openEditTechModal(t) {
      techFormData = {
          id: t.id,
          name: t.name,
          type: t.target_type,
          category: "Technical", // Por defecto
          data: { ...t.data },
          created_at: t.created_at
      };
      showTechModal = true;
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

  function openDetailModal(p) {
      viewingPerson = p;
      showDetailModal = true;
  }

  function openTechDetailModal(t) {
      viewingTechTarget = t;
      showTechDetailModal = true;
  }

  function closeModal() {
    showPersonModal = false;
    showTechModal = false;
    showDetailModal = false; 
    showTechDetailModal = false;
  }

  $effect(() => {
    if(agentStore.activeCase) loadData();
  });
</script>

<svelte:body class:printing={showDetailModal || showTechDetailModal} />

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
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="card person-card" role="button" tabindex="0" onclick={() => openDetailModal(p)}>
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
                                <button class="btn-icon" onclick={(e) => { e.stopPropagation(); openEditPersonModal(p); }}>✏️</button>
                                <button class="btn-icon delete" onclick={(e) => { e.stopPropagation(); handleDeletePerson(p.id); }}>🗑️</button>
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
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <tr class="clickable-row" role="button" tabindex="0" onclick={() => openTechDetailModal(t)}>
                            <td><span class="type-badge">{t.target_type}</span></td>
                            <td>
                                <div><strong>{t.name}</strong></div>
                                <div class="target-data-preview">
                                    {#if t.data?.detalles_tecnicos}
                                        {#each Object.keys(t.data.detalles_tecnicos) as herramienta}
                                            <small class="data-tag tool-badge">🔍 {herramienta.toUpperCase()}</small>
                                        {/each}
                                    {/if}
                                    {#each Object.entries(t.data || {}).filter(([k]) => k !== 'detalles_tecnicos') as [key, value]}
                                        <small class="data-tag"><b>{key}:</b> {typeof value === 'object' ? '...' : value}</small>
                                    {/each}
                                </div>
                            </td>
                            <td>{new Date(t.created_at).toLocaleDateString()}</td>
                            <td class="table-actions">
                                <button class="btn-icon" onclick={(e) => { e.stopPropagation(); openEditTechModal(t); }}>✏️</button>
                                <button class="btn-icon delete" onclick={(e) => { e.stopPropagation(); handleDeleteTechTarget(t.id); }}>🗑️</button>
                            </td>
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

  <!-- PERSON DETAIL MODAL (FICHA) -->
  {#if showDetailModal && viewingPerson}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal-backdrop detail-backdrop" role="button" tabindex="-1" onclick={() => showDetailModal = false}>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div class="modal detail-modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
            <div class="detail-header-actions no-print">
                <button class="btn-secondary" onclick={() => window.print()}>🖨️ Imprimir / Guardar PDF</button>
                <button class="btn-icon" onclick={() => showDetailModal = false}>❌</button>
            </div>
            
            <div class="ficha-content">
                <div class="ficha-header">
                    <div class="ficha-title">FICHA DE DATOS PERSONALES</div>
                    <div class="ficha-meta">
                        <span>ID: {viewingPerson.id.split('-')[0]}...</span>
                        <span>Caso: {agentStore.activeCase?.name || "N/A"}</span>
                    </div>
                </div>

                <div class="ficha-body">
                    <div class="section identity">
                        <h4>IDENTIDAD</h4>
                        <div class="data-grid">
                            <div class="data-item"><strong>Nombre:</strong> {viewingPerson.first_name || "-"}</div>
                            <div class="data-item"><strong>Apellido:</strong> {viewingPerson.last_name || "-"}</div>
                            <div class="data-item"><strong>DNI / ID:</strong> {viewingPerson.dni || "-"}</div>
                            <div class="data-item"><strong>Nacimiento:</strong> {viewingPerson.birth_date || "-"}</div>
                            <div class="data-item full">
                                <strong>Apodos / Alias:</strong> 
                                {#if viewingPerson.nicknames && viewingPerson.nicknames.length > 0}
                                    {viewingPerson.nicknames.map(n => n.value).join(", ")}
                                {:else}
                                    -
                                {/if}
                            </div>
                        </div>
                    </div>

                    <div class="section contact">
                        <h4>CONTACTO</h4>
                        <div class="data-grid">
                            <div class="data-item"><strong>Email:</strong> {viewingPerson.email || "-"}</div>
                            <div class="data-item"><strong>Teléfono:</strong> {viewingPerson.phone || "-"}</div>
                        </div>
                    </div>

                    <div class="section">
                        <h4>UBICACIONES ({viewingPerson.addresses.length})</h4>
                        {#if viewingPerson.addresses.length > 0}
                            <ul class="clean-list">
                                {#each viewingPerson.addresses as addr}
                                    <li>{addr.street} {addr.number}, {addr.locality}, {addr.state}, {addr.country} (CP: {addr.zip_code})</li>
                                {/each}
                            </ul>
                        {:else}
                            <p class="empty-text">No hay direcciones registradas.</p>
                        {/if}
                    </div>

                    <div class="section">
                        <h4>HISTORIAL LABORAL ({viewingPerson.jobs.length})</h4>
                        {#if viewingPerson.jobs.length > 0}
                            <ul class="clean-list">
                                {#each viewingPerson.jobs as job}
                                    <li><strong>{job.title}</strong> en {job.company} ({job.date_start || "?"} - {job.date_end || "Presente"})</li>
                                {/each}
                            </ul>
                        {:else}
                            <p class="empty-text">No hay empleos registrados.</p>
                        {/if}
                    </div>

                     <div class="section">
                        <h4>HUELLA DIGITAL ({viewingPerson.social_profiles.length})</h4>
                        {#if viewingPerson.social_profiles.length > 0}
                            <ul class="clean-list">
                                {#each viewingPerson.social_profiles as soc}
                                    <li><strong>{soc.platform}:</strong> {soc.username} ({soc.url})</li>
                                {/each}
                            </ul>
                        {:else}
                            <p class="empty-text">No hay perfiles sociales registrados.</p>
                        {/if}
                    </div>
                </div>

                <div class="ficha-footer">
                    Generado por OSINT Dashboard - {new Date().toLocaleString()}
                </div>
            </div>
        </div>
    </div>
  {/if}

  <!-- TECHNICAL TARGET DETAIL MODAL -->
  {#if showTechDetailModal && viewingTechTarget}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal-backdrop detail-backdrop" role="button" tabindex="-1" onclick={() => showTechDetailModal = false}>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div class="modal detail-modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
            <div class="detail-header-actions no-print">
                <button class="btn-secondary" onclick={() => window.print()}>🖨️ Imprimir / Guardar PDF</button>
                <button class="btn-icon" onclick={() => showTechDetailModal = false}>❌</button>
            </div>
            
            <div class="ficha-content">
                <div class="ficha-header">
                    <div class="ficha-title">INFORME TÉCNICO DE OBJETIVO</div>
                    <div class="ficha-meta">
                        <span>ID: {viewingTechTarget.id.split('-')[0]}...</span>
                        <span>Caso: {agentStore.activeCase?.name || "N/A"}</span>
                    </div>
                </div>

                <div class="ficha-body">
                    <div class="section tech-identity">
                        <h4>INFORMACIÓN GENERAL</h4>
                        <div class="data-grid">
                            <div class="data-item"><strong>Identificador:</strong> {viewingTechTarget.name}</div>
                            <div class="data-item"><strong>Tipo de Objetivo:</strong> {viewingTechTarget.target_type}</div>
                            <div class="data-item"><strong>Fecha de Registro:</strong> {new Date(viewingTechTarget.created_at).toLocaleString()}</div>
                        </div>
                    </div>

                    <div class="section tech-data">
                        <h4>DATOS Y HALLAZGOS TÉCNICOS</h4>
                        {#if viewingTechTarget.data?.detalles_tecnicos}
                            <!-- Estructura consolidada: una sección por herramienta (whois, ping, etc) -->
                            {#each Object.entries(viewingTechTarget.data.detalles_tecnicos) as [herramienta, campos]}
                                <div class="tool-section">
                                    <div class="tool-section-header">🔍 {herramienta.toUpperCase()}</div>
                                    <div class="hallazgos-grid">
                                        {#if typeof campos === 'object' && campos !== null}
                                            {#each Object.entries(campos) as [key, value]}
                                                {#if key === 'paquetes' && Array.isArray(value)}
                                                    <!-- Renderizado especial para paquetes de ping -->
                                                    <div class="hallazgo-item full-width no-border">
                                                        <div class="hallazgo-key terminal-font">paquetes:</div>
                                                        <div class="hallazgo-val">
                                                            <table class="ping-table">
                                                                <thead><tr><th>icmp_seq</th><th>ttl</th><th>time</th></tr></thead>
                                                                <tbody>
                                                                    {#each value as pkt}
                                                                        <tr>
                                                                            <td>{pkt.icmp_seq ?? '-'}</td>
                                                                            <td>{pkt.ttl ?? '-'}</td>
                                                                            <td>{pkt.time ?? '-'}</td>
                                                                        </tr>
                                                                    {/each}
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    </div>
                                                {:else if typeof value !== 'object'}
                                                    <div class="hallazgo-item no-border">
                                                        <div class="hallazgo-key terminal-font">{key}:</div>
                                                        <div class="hallazgo-val terminal-font">
                                                            {#if typeof value === 'string' && value.includes(',')}
                                                                {#each value.split(',').map(s => s.trim()) as line}
                                                                    <div class="val-line">{line}</div>
                                                                {/each}
                                                            {:else}
                                                                {value}
                                                            {/if}
                                                        </div>
                                                    </div>
                                                {/if}
                                            {/each}
                                        {:else}
                                            <div class="hallazgo-val terminal-font" style="padding: 10px;">{campos}</div>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        {:else if Object.keys(viewingTechTarget.data || {}).length > 0}
                            <!-- Fallback para datos no técnicos o manuales que no están en detalles_tecnicos -->
                            <div class="hallazgos-grid no-border">
                                {#each Object.entries(viewingTechTarget.data) as [key, value]}
                                    <div class="hallazgo-item no-border">
                                        <div class="hallazgo-key terminal-font">{key}:</div>
                                        <div class="hallazgo-val terminal-font">{value}</div>
                                    </div>
                                {/each}
                            </div>
                        {:else}
                            <p class="empty-text">No hay hallazgos técnicos registrados para este objetivo.</p>
                        {/if}
                    </div>

                    <div class="section tech-links">
                        <h4>VÍNCULOS RELACIONADOS ({viewingTechTarget.linked_targets.length})</h4>
                        {#if viewingTechTarget.linked_targets.length > 0}
                            <ul class="clean-list">
                                {#each viewingTechTarget.linked_targets as link}
                                    <li>Vinculado con <strong>{link.target_id}</strong> (Relación: {link.relation})</li>
                                {/each}
                            </ul>
                        {:else}
                            <p class="empty-text">No se han establecido vínculos para este objetivo todavía.</p>
                        {/if}
                    </div>
                </div>

                <div class="ficha-footer">
                    Generado por OSINT Dashboard - {new Date().toLocaleString()}
                </div>
            </div>
        </div>
    </div>
  {/if}

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
            <h3>{techFormData.id ? "Editar Objetivo" : "Nuevo Objetivo Técnico"}</h3>
             <form onsubmit={(e) => { e.preventDefault(); handleSaveTechTarget(); }}>
                <div class="form-group">
                    <label for="t_type">Tipo</label>
                    <select id="t_type" bind:value={techFormData.type}>
                        <option value="Domain">Dominio</option>
                        <option value="IP">IP</option>
                        <option value="Email">Email</option>
                        <option value="Username">Usuario / Alias</option>
                        <option value="Phone">Teléfono</option>
                        <option value="File">Archivo</option>
                        <option value="Hash">Hash</option>
                        <option value="Other">Otro</option>
                    </select>
                </div>
                 <div class="form-group">
                    <label for="t_identifier">Identificador (Nombre/IP/Email)</label>
                    <input id="t_identifier" type="text" required bind:value={techFormData.name} />
                </div>

                {#if techFormData.id}
                    <div class="form-group" style="margin-top: 15px;">
                        <span class="section-label">Datos Técnicos (Hallazgos)</span>
                        <div class="tech-data-editor">
                            {#each Object.entries(techFormData.data) as [key, value]}
                                <div class="data-edit-row">
                                    <input type="text" readonly value={key} style="width: 40%; background: var(--bg-tertiary);" />
                                    <input type="text" bind:value={techFormData.data[key]} style="width: 50%;" />
                                    <button type="button" class="btn-icon delete" onclick={() => {
                                        const newData = { ...techFormData.data };
                                        delete newData[key];
                                        techFormData.data = newData;
                                    }}>×</button>
                                </div>
                            {/each}
                            <div class="data-add-row" style="margin-top: 10px; display: flex; gap: 5px;">
                                <input type="text" placeholder="Clave (ej: ASN)" style="width: 40%;" bind:value={newTechKey} />
                                <input type="text" placeholder="Valor" style="width: 50%;" bind:value={newTechVal} />
                                <button type="button" class="btn-small" onclick={() => {
                                    if(newTechKey && newTechVal) {
                                        techFormData.data = { ...techFormData.data, [newTechKey]: newTechVal };
                                        newTechKey = "";
                                        newTechVal = "";
                                    }
                                }}>+</button>
                            </div>
                        </div>
                    </div>
                {/if}

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
  .card { background: var(--bg-secondary); border: 1px solid var(--border-color); padding: 15px; border-radius: 8px; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; }
  .card:hover { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(0,0,0,0.2); border-color: var(--accent-color); }

  .card-header { display: flex; justify-content: space-between; margin-bottom: 10px; }
  .badges { display: flex; gap: 10px; margin-top: 10px; }
  .badge { background: var(--bg-tertiary); padding: 2px 8px; border-radius: 12px; font-size: 0.8rem; }
  
  .table-container table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 10px; border-bottom: 1px solid var(--border-color); }
  .clickable-row { cursor: pointer; transition: background 0.2s; }
  .clickable-row:hover { background: var(--bg-primary); }
  .table-actions { text-align: right; width: 100px; }
  .target-data-preview { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 5px; }
  .data-tag { background: var(--bg-tertiary); padding: 1px 5px; border-radius: 3px; font-size: 0.75rem; border: 1px solid var(--border-color); }
  .type-badge { background: var(--bg-tertiary); padding: 2px 6px; border-radius: 4px; font-size: 0.85rem; }
  .tool-badge { background: rgba(var(--accent-rgb, 99,102,241), 0.15); color: var(--accent-color); border-color: var(--accent-color); font-weight: 600; }

  /* Secciones por herramienta en el modal de detalle técnico */
  .tool-section { margin-bottom: 24px; border: 1px solid #ddd; border-radius: 6px; overflow: hidden; background: white; }
  .tool-section-header { background: #f0f0f0; padding: 10px 15px; font-weight: 700; font-size: 0.8rem; letter-spacing: 0.08em; color: #111; border-bottom: 1px solid #ddd; display: flex; align-items: center; gap: 8px; }
  .tool-section-header::before { content: ""; display: block; width: 3px; height: 14px; background: var(--accent-color, #333); border-radius: 2px; }
  .hallazgo-item.full-width { grid-column: 1 / -1; }

  /* Tabla de estadísticas de ping */
  .ping-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; margin: 0; }
  .ping-table th { background: #f9f9f9; padding: 6px 12px; color: #666; font-weight: 600; text-align: left; border-bottom: 1px solid #eee; border-right: 1px solid #eee; }
  .ping-table th:last-child { border-right: none; }
  .ping-table td { padding: 6px 12px; border-bottom: 1px solid #f5f5f5; font-family: 'Consolas', monospace; border-right: 1px solid #eee; }
  .ping-table td:last-child { border-right: none; }
  .ping-table tr:last-child td { border-bottom: none; }

  .tech-data-editor .data-edit-row { display: flex; gap: 5px; margin-bottom: 5px; align-items: center; }
  .form-group label { display: block; font-size: 0.85rem; color: var(--text-muted); margin-bottom: 5px; cursor: pointer; }

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

  /* FICHA DE DATOS & PRINT STYLES */
  .detail-modal { width: 800px; max-width: 95vw; background: white; color: #333; height: 90vh; overflow-y: auto; display: flex; flex-direction: column; }
  .detail-header-actions { display: flex; justify-content: space-between; padding-bottom: 15px; border-bottom: 1px solid #eee; margin-bottom: 20px; }
  
  .ficha-content { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; padding: 20px; flex: 1; }
  .ficha-header { text-align: center; margin-bottom: 30px; border-bottom: 2px solid #333; padding-bottom: 10px; }
  .ficha-title { font-size: 1.8rem; font-weight: bold; margin-bottom: 5px; text-transform: uppercase; }
  .ficha-meta { font-size: 0.9rem; color: #666; display: flex; justify-content: center; gap: 20px; }
  
  .ficha-body .section { margin-bottom: 25px; }
  .ficha-body h4 { background: #eee; color: #222; font-weight: bold; padding: 8px 10px; margin: 0 0 10px 0; font-size: 1rem; text-transform: uppercase; border-left: 4px solid #333; }
  
  .data-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .data-item.full { grid-column: span 2; }
  
  .clean-list { list-style: none; padding: 0; margin: 0; }
  .clean-list li { padding: 5px 0; border-bottom: 1px dotted #ccc; }
  .clean-list li:last-child { border-bottom: none; }
  
  .ficha-footer { margin-top: 40px; border-top: 1px solid #eee; padding-top: 10px; text-align: center; font-size: 0.8rem; color: #999; }
  .empty-text { font-style: italic; color: #888; margin: 5px 0; }

  .hallazgos-grid { 
    display: flex; 
    flex-direction: column; 
    gap: 2px;
  }
  .hallazgos-grid.no-border { border: none; }
  .hallazgo-item { 
    display: flex; 
    align-items: flex-start;
    padding: 2px 0;
  }
  .hallazgo-item.no-border { border: none; }
  .hallazgo-key { 
    font-weight: 500; 
    width: 140px; 
    color: var(--accent-color, #1a73e8); 
    font-size: 0.85rem; 
    padding: 2px 0;
    flex-shrink: 0;
  }
  .hallazgo-val { 
    flex: 1; 
    word-break: break-all; 
    padding: 2px 0;
    font-size: 0.9rem;
    color: #333;
    line-height: 1.4;
  }
  .terminal-font {
    font-family: 'Cascadia Code', 'Consolas', 'Monaco', 'Courier New', monospace !important;
  }
  .val-line { display: block; margin-bottom: 2px; }
  .hallazgo-item:hover .hallazgo-key { background: transparent; color: var(--accent-color); }
  .hallazgo-item:hover .hallazgo-val { background: rgba(0,0,0,0.02); }

  /* --- SISTEMA DE IMPRESIÓN (Estrategia de Visibilidad Inversa) --- */
  @media print {
    @page { 
      margin: 0 !important; 
    }

    /* 1. Ocultar ABSOLUTAMENTE TODO en la página */
    :global(body *) {
      visibility: hidden !important;
    }

    /* 2. Habilitar la visibilidad de los contenedores estructurales */
    /* Pero sin que se vea su contenido (esto es clave) */
    :global(body),
    :global(.app-shell),
    :global(.main-content),
    :global(.content-scroll),
    :global(.targets-view) {
      visibility: visible !important;
      display: block !important;
      position: static !important;
      margin: 0 !important;
      padding: 0 !important;
      background: white !important;
      box-shadow: none !important;
      border: none !important;
      width: 100% !important;
      height: auto !important;
      overflow: visible !important;
    }

    /* 3. Mostrar el Modal y TODO su contenido */
    .detail-backdrop,
    .detail-backdrop *,
    .detail-modal,
    .detail-modal *,
    .ficha-content,
    .ficha-content * {
      visibility: visible !important;
    }

    /* 4. Asegurar el posicionamiento del modal sobre el resto */
    .detail-backdrop { 
      display: block !important;
      position: absolute !important;
      top: 0 !important;
      left: 0 !important;
      width: 100% !important;
      z-index: 9999999 !important;
      background: white !important;
    }

    .detail-modal {
      box-shadow: none !important;
      border: none !important;
      border-radius: 0 !important;
      background: white !important;
      width: 100% !important;
      max-width: none !important;
      margin: 0 !important;
    }

    .ficha-content { 
      padding: 1.5cm 1.5cm 1.5cm 3cm !important; /* Margen izquierdo de 3cm para encuadernación */
      min-height: 29cm;
      box-sizing: border-box;
    }

    /* 5. Forzar la ocultación de elementos que suelen "flotar" o molestar */
    :global(.sidebar),
    :global(.agent-container),
    :global(.top-bar),
    .no-print {
      display: none !important;
      visibility: hidden !important;
    }
    
    .ficha-body h4 { 
      background: #f0f0f0 !important; 
      color: #000 !important; 
      border-left: 4px solid #000 !important;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
    }

    ::-webkit-scrollbar { display: none; }
  }
</style>
