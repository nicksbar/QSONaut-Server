<script lang="ts">
  import { api } from './api';
  import type { Club, ClubMember, ContestTemplate, Event, EventParticipant, ManagedCallsign } from './types';

  let { events, clubs, templates, administrator, refresh }: { events: Event[]; clubs: Club[]; templates: ContestTemplate[]; administrator: boolean; refresh: () => Promise<void> } = $props();
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
  let editingId = $state<string | null>(null);
  let eventSearch = $state('');
  let eventStatusFilter = $state('all');
  let eventClubFilter = $state('all');
  let eventPage = $state(1);
  let managingEventId = $state<string | null>(null);
  let participants = $state<EventParticipant[]>([]);
  let identities = $state<ManagedCallsign[]>([]);
  let clubMembers = $state<ClubMember[]>([]);
  let participantId = $state<string | null>(null);
  let participantUserId = $state('');
  let participantIdentityId = $state('');
  let participantRole = $state('operator');
  let participantStatus = $state('active');
  let participantStation = $state('');
  let participantBand = $state('');
  let participantMode = $state('');
  const eventPageSize = 20;
  let manageableClubs = $derived(clubs.filter((club) => club.can_manage));
  let selectedTemplate = $derived(templates.find((template) => template.id === templateId));
  let managingEvent = $derived(events.find((event) => event.id === managingEventId));
  let assignableIdentities = $derived(identities.filter((identity) => identity.club_id === managingEvent?.club_id && (identity.event_id === null || identity.event_id === managingEventId)));
  let specialIdentity = $derived(assignableIdentities.find((identity) => identity.identity_type === 'special' && identity.event_id === managingEventId));
  let filteredEvents = $derived(events.filter((event) => {
    const query = eventSearch.trim().toLowerCase();
    return (!query || `${event.name} ${event.contest_name} ${event.special_callsign || ''}`.toLowerCase().includes(query))
      && (eventStatusFilter === 'all' || event.status === eventStatusFilter)
      && (eventClubFilter === 'all' || event.club_id === eventClubFilter);
  }));
  let eventPageCount = $derived(Math.max(1, Math.ceil(filteredEvents.length / eventPageSize)));
  let visibleEvents = $derived(filteredEvents.slice((eventPage - 1) * eventPageSize, eventPage * eventPageSize));

  $effect(() => { if (!manageableClubs.some((club) => club.id === clubId)) clubId = manageableClubs[0]?.id || ''; });
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

  function canManageEvent(event: Event): boolean {
    return manageableClubs.some((club) => club.id === event.club_id);
  }

  function localDateTime(value: string): string {
    const date = new Date(value);
    const pad = (part: number) => String(part).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function editEvent(event: Event) {
    editingId = event.id;
    clubId = event.club_id;
    name = event.name;
    templateId = event.contest_template_id || '';
    specialCallsign = event.special_callsign || '';
    start = localDateTime(event.starts_at);
    end = localDateTime(event.ends_at);
    status = event.status;
    config = Object.fromEntries(Object.entries(event.contest_config || {}).map(([key, value]) => [key, String(value)]));
    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
  }

  function clearForm() {
    editingId = null; name = specialCallsign = start = end = '';
    status = 'draft'; templateId = ''; config = {};
  }

  function resetEventPage() { eventPage = 1; }

  async function run(work: () => Promise<void>) {
    working = true; error = '';
    try { await work(); } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function createEvent() {
    await run(async () => {
      await api<Event>(editingId ? `/api/v1/events/${editingId}` : '/api/v1/events', {
        method: editingId ? 'PATCH' : 'POST',
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
      clearForm();
      await refresh();
    });
  }

  async function setStatus(id: string, nextStatus: string) {
    await run(async () => {
      await api(`/api/v1/events/${id}/status`, { method: 'PATCH', body: JSON.stringify({ status: nextStatus }) });
      await refresh();
    });
  }

  function clearParticipantForm() {
    participantId = null; participantUserId = clubMembers.find((member) => member.membership_status === 'active')?.user_id || '';
    participantIdentityId = assignableIdentities.find((identity) => identity.status === 'active' && identity.verification_status === 'verified')?.id || '';
    participantRole = 'operator'; participantStatus = 'active'; participantStation = participantBand = participantMode = '';
  }

  async function manageEvent(event: Event) {
    await run(async () => {
      managingEventId = event.id;
      [participants, identities, clubMembers] = await Promise.all([
        api<EventParticipant[]>(`/api/v1/events/${event.id}/participants`),
        api<ManagedCallsign[]>('/api/v1/identities'),
        api<ClubMember[]>(`/api/v1/clubs/${event.club_id}/members`),
      ]);
      clearParticipantForm();
    });
  }

  function editParticipant(participant: EventParticipant) {
    participantId = participant.id; participantUserId = participant.user_id; participantIdentityId = participant.callsign_id;
    participantRole = participant.role; participantStatus = participant.status; participantStation = participant.station_label;
    participantBand = participant.band || ''; participantMode = participant.mode || '';
  }

  async function saveParticipant() {
    const member = clubMembers.find((item) => item.user_id === participantUserId);
    if (!managingEvent || !member) return;
    await run(async () => {
      const base = `/api/v1/events/${managingEvent!.id}/participants`;
      await api<EventParticipant>(participantId ? `${base}/${participantId}` : base, {
        method: participantId ? 'PATCH' : 'POST',
        body: JSON.stringify({
          user_id: participantUserId, callsign_id: participantIdentityId, operator_callsign: member.callsign,
          role: participantRole, status: participantStatus, starts_at: null, ends_at: null,
          station_label: participantStation, band: participantBand || null, mode: participantMode || null,
        }),
      });
      participants = await api<EventParticipant[]>(base);
      await refresh(); clearParticipantForm();
    });
  }

  async function decideSpecialIdentity(decision: string) {
    if (!specialIdentity) return;
    await run(async () => {
      await api(`/api/v1/identities/${specialIdentity!.id}/status`, { method: 'PATCH', body: JSON.stringify({ decision }) });
      identities = await api<ManagedCallsign[]>('/api/v1/identities');
    });
  }
</script>

<section class="grid events-grid">
  <div>
    <p class="eyebrow">OPERATIONS / CONTEST INSTANCES</p>
    <div class="list-heading"><h2>Events</h2><small>{filteredEvents.length} of {events.length}</small></div>
    <div class="list-tools"><input aria-label="Search events" placeholder="Search operations or callsign" bind:value={eventSearch} oninput={resetEventPage} /><select aria-label="Filter events by status" bind:value={eventStatusFilter} onchange={resetEventPage}><option value="all">All statuses</option>{#each ['draft','scheduled','active','completed','cancelled'] as statusOption}<option value={statusOption}>{statusOption}</option>{/each}</select><select aria-label="Filter events by club" bind:value={eventClubFilter} onchange={resetEventPage}><option value="all">All clubs</option>{#each clubs as club}<option value={club.id}>{club.name}</option>{/each}</select></div>
    {#if events.length === 0}
      <p class="empty">No events scheduled.</p>
    {:else if visibleEvents.length === 0}
      <p class="empty">No events match the current filters.</p>
    {/if}
    {#each visibleEvents as event}
      <article class="record event-record">
        <b>{event.name}</b><span class="pill">{event.status}</span>
        <p>{clubs.find((club) => club.id === event.club_id)?.name || 'club'} · {templateName(event.contest_template_id)} · {event.special_callsign || 'club callsign'}</p><small>{new Date(event.starts_at).toLocaleString()} → {new Date(event.ends_at).toLocaleString()} · {event.participant_count} participant{event.participant_count === 1 ? '' : 's'}</small>
        {#if Object.keys(event.contest_config || {}).length}<small class="config-line">{Object.entries(event.contest_config).map(([key, value]) => `${key}: ${value}`).join(' · ')}</small>{/if}
        {#if canManageEvent(event)}<div class="actions">
          <button onclick={() => editEvent(event)}>EDIT</button><button onclick={() => manageEvent(event)}>OPERATORS</button>
          {#if event.status === 'draft'}<button onclick={() => setStatus(event.id, 'scheduled')}>SCHEDULE</button>{/if}
          {#if !['active','completed','cancelled'].includes(event.status)}<button onclick={() => setStatus(event.id, 'active')}>ACTIVATE</button>{/if}
          {#if event.status === 'active'}<button onclick={() => setStatus(event.id, 'completed')}>COMPLETE</button>{/if}
          {#if !['completed','cancelled'].includes(event.status)}<button class="danger" onclick={() => setStatus(event.id, 'cancelled')}>CANCEL</button>{/if}
        </div>{/if}
      </article>
    {/each}
    {#if visibleEvents.length > 0}<div class="list-pager"><button class="secondary" disabled={eventPage <= 1} onclick={() => eventPage -= 1}>PREVIOUS</button><small>PAGE {eventPage} / {eventPageCount}</small><button class="secondary" disabled={eventPage >= eventPageCount} onclick={() => eventPage += 1}>NEXT</button></div>{/if}
    {#if managingEvent}
      <section class="assignment-panel">
        <div class="list-heading"><h3>{managingEvent.name} operators</h3><button class="secondary" onclick={() => { managingEventId = null; participants = []; }}>CLOSE</button></div>
        {#if specialIdentity}<p><b>{specialIdentity.callsign}</b> special identity · {specialIdentity.status} / {specialIdentity.verification_status}
          {#if administrator && specialIdentity.verification_status === 'pending'}<button onclick={() => decideSpecialIdentity('approve')}>APPROVE</button><button class="danger" onclick={() => decideSpecialIdentity('reject')}>REJECT</button>{/if}
          {#if specialIdentity.status === 'active'}<button class="danger" onclick={() => decideSpecialIdentity('suspend')}>SUSPEND</button>{/if}
        </p>{/if}
        {#if participants.length === 0}<p class="empty">No station assignments.</p>{/if}
        {#each participants as participant}<article class="record"><b>{participant.operator_callsign} → {participant.operating_callsign}</b><span class="pill">{participant.status}</span><p>{participant.role} · {participant.station_label || 'unlabelled station'} · {participant.band || 'all bands'} · {participant.mode || 'all modes'}</p><button onclick={() => editParticipant(participant)}>EDIT ASSIGNMENT</button></article>{/each}
        <form onsubmit={(event) => { event.preventDefault(); saveParticipant(); }}>
          <h4>{participantId ? 'Edit assignment' : 'Assign operator'}</h4>
          <label>Operator<select bind:value={participantUserId} required>{#each clubMembers.filter((member) => member.membership_status === 'active') as member}<option value={member.user_id}>{member.callsign} · {member.role}</option>{/each}</select></label>
          <label>Operating identity<select bind:value={participantIdentityId} required>{#each assignableIdentities.filter((identity) => identity.status === 'active' && identity.verification_status === 'verified') as identity}<option value={identity.id}>{identity.callsign} · {identity.identity_type}</option>{/each}</select></label>
          <label>Event role<select bind:value={participantRole}>{#each ['operator','coordinator','logger','observer'] as item}<option value={item}>{item}</option>{/each}</select></label>
          <label>Status<select bind:value={participantStatus}>{#each ['pending','invited','active','suspended','withdrawn','completed'] as item}<option value={item}>{item}</option>{/each}</select></label>
          <label>Station label<input maxlength="80" bind:value={participantStation} /></label>
          <label>Band restriction<input maxlength="16" placeholder="Optional, e.g. 20m" bind:value={participantBand} /></label>
          <label>Mode restriction<input maxlength="24" placeholder="Optional, e.g. DIGITAL" bind:value={participantMode} /></label>
          {#if participantId}<button type="button" class="secondary" onclick={clearParticipantForm}>CANCEL EDIT</button>{/if}<button disabled={working || !participantUserId || !participantIdentityId}>{participantId ? 'SAVE ASSIGNMENT' : 'ASSIGN OPERATOR'}</button>
        </form>
      </section>
    {/if}
  </div>

  <div>
    <form onsubmit={(event) => { event.preventDefault(); createEvent(); }}>
      <h3>{editingId ? 'Edit club operation' : 'Create club operation'}</h3>
      <p class="form-help">Choose a contest template for rule-aware setup, or leave it general for a club event.</p>
      <label>Club<select bind:value={clubId} required>{#each manageableClubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>
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
      <label>Special callsign<input maxlength="16" bind:value={specialCallsign} /><small>Club managers submit this identity for approval; administrators verify it immediately.</small></label>
      <label>Starts<input type="datetime-local" bind:value={start} required /></label>
      <label>Ends<input type="datetime-local" bind:value={end} required /></label>
      <label>Initial state<select bind:value={status}>{#each ['draft','scheduled','active'] as item}<option value={item}>{item}</option>{/each}</select></label>
      {#if error}<p class="error">{error}</p>{/if}
      {#if editingId}<button type="button" class="secondary" onclick={clearForm} disabled={working}>CANCEL EDIT</button>{/if}<button disabled={working || !clubId}>{working ? 'SAVING…' : editingId ? 'SAVE OPERATION' : 'CREATE OPERATION'}</button>
    </form>
  </div>
</section>

<style>
  .list-heading { display:flex; align-items:baseline; justify-content:space-between; gap:12px; }
  .list-heading h2 { margin-bottom:0; }
  .list-heading small, .list-pager small { color:var(--muted); }
  .list-tools { display:grid; grid-template-columns:1.5fr 1fr 1fr; gap:8px; margin:12px 0; }
  .list-tools input, .list-tools select { min-width:0; width:100%; box-sizing:border-box; }
  .list-pager { display:flex; align-items:center; justify-content:center; gap:14px; margin:14px 0 24px; }
  .list-pager .secondary { padding:7px 10px; }
  @media (max-width:700px) { .list-tools { grid-template-columns:1fr; } }
</style>
