<script lang="ts">
  import { api } from './api';
  import type { Club, ContestTemplate, Event } from './types';

  let { events, clubs, templates, refresh }: { events: Event[]; clubs: Club[]; templates: ContestTemplate[]; refresh: () => Promise<void> } = $props();
  let clubId = $state('');
  let name = $state('');
  let templateId = $state('');
  let specialCallsign = $state('');
  let start = $state('');
  let end = $state('');
  let status = $state('draft');
  let config = $state<Record<string, string>>({});
  let working = $state(false);
  let error = $state('');
  let configuredTemplateId = $state('');
  let selectedTemplate = $derived(templates.find((template) => template.id === templateId));

  $effect(() => { if (!clubs.some((club) => club.id === clubId)) clubId = clubs[0]?.id || ''; });
  $effect(() => {
    if (templateId !== configuredTemplateId && selectedTemplate) {
      configuredTemplateId = templateId;
      const next: Record<string, string> = {};
      for (const key of Object.keys(selectedTemplate.required_fields)) next[key] = '';
      config = next;
    } else if (templateId !== configuredTemplateId) {
      configuredTemplateId = templateId; config = {};
    }
  });

  function templateName(id: string | null): string {
    return templates.find((template) => template.id === id)?.name || 'General club event';
  }

  async function run(work: () => Promise<void>) {
    working = true; error = '';
    try { await work(); } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function createEvent() {
    await run(async () => {
      await api<Event>('/api/v1/events', {
        method: 'POST',
        body: JSON.stringify({
          club_id: clubId,
          name,
          contest_name: selectedTemplate?.name || '',
          special_callsign: specialCallsign || null,
          starts_at: new Date(start).toISOString(),
          ends_at: new Date(end).toISOString(),
          status,
          contest_template_id: templateId || null,
          contest_config: config,
        }),
      });
      name = specialCallsign = start = end = '';
      status = 'draft'; templateId = ''; config = {};
      await refresh();
    });
  }

  async function setStatus(id: string, nextStatus: string) {
    await run(async () => {
      await api(`/api/v1/events/${id}/status`, { method: 'PATCH', body: JSON.stringify({ status: nextStatus }) });
      await refresh();
    });
  }
</script>

<section class="grid events-grid">
  <div>
    <p class="eyebrow">OPERATIONS / CONTEST INSTANCES</p>
    <h2>Events</h2>
    {#if events.length === 0}<p class="empty">No events scheduled.</p>{/if}
    {#each events as event}
      <article class="record event-record">
        <b>{event.name}</b><span class="pill">{event.status}</span>
        <p>{templateName(event.contest_template_id)} · {event.special_callsign || 'club callsign'} · {new Date(event.starts_at).toLocaleString()}</p>
        {#if Object.keys(event.contest_config || {}).length}<small class="config-line">{Object.entries(event.contest_config).map(([key, value]) => `${key}: ${value}`).join(' · ')}</small>{/if}
        <div class="actions">
          {#if event.status === 'draft'}<button onclick={() => setStatus(event.id, 'scheduled')}>SCHEDULE</button>{/if}
          {#if !['active','completed','cancelled'].includes(event.status)}<button onclick={() => setStatus(event.id, 'active')}>ACTIVATE</button>{/if}
          {#if event.status === 'active'}<button onclick={() => setStatus(event.id, 'completed')}>COMPLETE</button>{/if}
          {#if !['completed','cancelled'].includes(event.status)}<button class="danger" onclick={() => setStatus(event.id, 'cancelled')}>CANCEL</button>{/if}
        </div>
      </article>
    {/each}
  </div>

  <div>
    <form onsubmit={(event) => { event.preventDefault(); createEvent(); }}>
      <h3>Create club operation</h3>
      <p class="form-help">Choose a contest template for rule-aware setup, or leave it general for a club event.</p>
      <label>Club<select bind:value={clubId} required>{#each clubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>
      <label>Operation name<input maxlength="150" bind:value={name} required /></label>
      <label>Contest template<select bind:value={templateId}><option value="">General / non-contest event</option>{#each templates as template}<option value={template.id}>{template.icon} {template.organization} · {template.name}</option>{/each}</select></label>
      {#if selectedTemplate}
        <aside class="template-card">
          <b>{selectedTemplate.icon} {selectedTemplate.name}</b><p>{selectedTemplate.description}</p>
          <small>{String(selectedTemplate.scoring_rules.formula || 'Template scoring')}</small>
          <small>{String(selectedTemplate.schedule.summary || 'Schedule selected manually')} · catalog v{selectedTemplate.definition_version}</small>
          {#if selectedTemplate.rules_url}<a href={selectedTemplate.rules_url} target="_blank" rel="noreferrer">CURRENT RULES ↗</a>{/if}
        </aside>
        {#each Object.entries(selectedTemplate.required_fields) as [key, field]}
          <label>{key.replaceAll('_', ' ')}{#if field.required}<sup>required</sup>{/if}
            {#if field.options}
              <select bind:value={config[key]} required={field.required}><option value="">Select…</option>{#each field.options as option}<option value={option}>{option}</option>{/each}</select>
            {:else}<input bind:value={config[key]} required={field.required} />{/if}
            {#if field.description}<small>{field.description}</small>{/if}
          </label>
        {/each}
        <div class="rule-strip"><span>Bands: {selectedTemplate.validation_rules.bands?.join(', ') || 'open'}</span><span>Modes: {selectedTemplate.validation_rules.modes?.join(', ') || 'open'}</span></div>
      {/if}
      <label>Special callsign<input maxlength="16" bind:value={specialCallsign} /></label>
      <label>Starts<input type="datetime-local" bind:value={start} required /></label>
      <label>Ends<input type="datetime-local" bind:value={end} required /></label>
      <label>Initial state<select bind:value={status}>{#each ['draft','scheduled','active'] as item}<option value={item}>{item}</option>{/each}</select></label>
      {#if error}<p class="error">{error}</p>{/if}
      <button disabled={working || !clubId}>{working ? 'CREATING…' : 'CREATE OPERATION'}</button>
    </form>
  </div>
</section>
