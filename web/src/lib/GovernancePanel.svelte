<script lang="ts">
  import { api } from './api';
  import ClubPaymentMethodsPanel from './ClubPaymentMethodsPanel.svelte';
  import type { Club, ClubMember, Governance, GovernanceCalendarEntry, GovernanceElection, GovernancePosition } from './types';

  let { clubs, initialClubId = null }: { clubs: Club[]; initialClubId?: string | null } = $props();
  let selectedClubId = $state('');
  let governance = $state<Governance | null>(null);
  let members = $state<ClubMember[]>([]);
  let calendar = $state<GovernanceCalendarEntry[]>([]);
  let loading = $state(false);
  let working = $state(false);
  let error = $state('');
  let notice = $state('');
  let positionName = $state('');
  let positionType = $state<'officer' | 'board'>('officer');
  let seats = $state(1);
  let termYears = $state(1);
  let electionParity = $state<'any' | 'even' | 'odd'>('any');
  let positionDescription = $state('');
  let assignmentPositionId = $state('');
  let assignmentUserId = $state('');
  let assignmentSeat = $state(1);
  let assignmentStarts = $state(new Date().toISOString().slice(0, 10));
  let assignmentEnds = $state(`${new Date().getFullYear()}-12-31`);
  let assignmentMethod = $state<'elected' | 'appointed' | 'acting'>('appointed');
  let electionTitle = $state('');
  let electionYear = $state(new Date().getFullYear());
  let electionStatus = $state<'planned' | 'nominations' | 'voting' | 'closed' | 'certified' | 'cancelled'>('planned');
  let electionOpens = $state('');
  let electionCloses = $state('');
  let electionNotes = $state('');
  let electionPositionIds = $state<string[]>([]);

  let manageableClubs = $derived(clubs.filter((club) => club.can_manage));
  let selectedClub = $derived(manageableClubs.find((club) => club.id === selectedClubId));
  let positions = $derived(governance?.positions || []);
  let assignments = $derived(governance?.assignments || []);
  let elections = $derived(governance?.elections || []);

  async function load() {
    if (!selectedClubId) return;
    loading = true; error = '';
    try {
      [governance, members, calendar] = await Promise.all([
        api<Governance>(`/api/v1/clubs/${selectedClubId}/governance`),
        api<ClubMember[]>(`/api/v1/clubs/${selectedClubId}/members`),
        api<GovernanceCalendarEntry[]>(`/api/v1/clubs/${selectedClubId}/governance/calendar`),
      ]);
      assignmentPositionId = assignmentPositionId || positions[0]?.id || '';
      assignmentUserId = assignmentUserId || members.find((member) => member.membership_status === 'active')?.user_id || '';
    } catch (cause) { error = (cause as Error).message; }
    finally { loading = false; }
  }

  async function exportCalendar() {
    working = true; error = '';
    try {
      const response = await fetch(`/api/v1/clubs/${selectedClubId}/governance/calendar.ics`, { credentials: 'same-origin' });
      if (!response.ok) throw new Error('Calendar export failed.');
      const url = URL.createObjectURL(await response.blob());
      const link = document.createElement('a');
      link.href = url; link.download = `${selectedClub?.name || 'club'}-governance.ics`; link.click();
      URL.revokeObjectURL(url);
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  function selectClub() { governance = null; members = []; void load(); }

  async function createPosition() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api<GovernancePosition>(`/api/v1/clubs/${selectedClubId}/governance/positions`, { method: 'POST', body: JSON.stringify({ name: positionName, position_type: positionType, seats, term_years: termYears, election_parity: electionParity, description: positionDescription }) });
      positionName = ''; positionDescription = ''; notice = 'Governance position created.'; await load();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function assignPosition() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/governance/assignments`, { method: 'POST', body: JSON.stringify({ position_id: assignmentPositionId, user_id: assignmentUserId, seat_number: assignmentSeat, starts_on: assignmentStarts, ends_on: assignmentEnds, selection_method: assignmentMethod }) });
      notice = 'Position assignment recorded.'; await load();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function createElection() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/governance/elections`, { method: 'POST', body: JSON.stringify({ title: electionTitle, election_year: electionYear, status: electionStatus, opens_at: electionOpens ? new Date(electionOpens).toISOString() : null, closes_at: electionCloses ? new Date(electionCloses).toISOString() : null, notes: electionNotes, position_ids: electionPositionIds }) });
      electionTitle = ''; electionNotes = ''; electionPositionIds = []; notice = 'Election cycle created.'; await load();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function setElectionStatus(election: GovernanceElection, status: string) {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/governance/elections/${election.id}`, { method: 'PATCH', body: JSON.stringify({ status }) });
      notice = `Election moved to ${status}.`; await load();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  $effect(() => {
    const nextClubId = initialClubId || clubs.find((club) => club.can_manage)?.id || '';
    if (!selectedClubId || (initialClubId && initialClubId !== selectedClubId)) selectedClubId = nextClubId;
    if (selectedClubId) void load();
  });
</script>

<section class="governance">
  <div class="section-head"><div><p class="eyebrow">HOSTED ORGANIZATION / GOVERNANCE</p><h1>Board and election management</h1><p class="section-intro">Define club roles, record assignments, and manage election-cycle status. This workspace does not conduct voting or send invitations.</p></div><button class="secondary" onclick={() => void load()} disabled={loading}>REFRESH</button></div>
  {#if manageableClubs.length === 0}<p class="empty">You do not manage an organization with hosted governance enabled.</p>{:else}
    <label class="club-picker">Organization<select bind:value={selectedClubId} onchange={selectClub}>{#each manageableClubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>
    {#if error}<p class="error banner">{error}</p>{/if}{#if notice}<p class="notice banner">{notice}</p>{/if}
    {#if selectedClub && governance}
      <div class="governance-grid">
        <form onsubmit={(event) => { event.preventDefault(); void createPosition(); }}><p class="eyebrow">POSITIONS</p><h2>Define a seat</h2><label>Name<input maxlength="120" bind:value={positionName} required /></label><label>Type<select bind:value={positionType}><option value="officer">Officer</option><option value="board">Board</option></select></label><label>Seats<input type="number" min="1" max="100" bind:value={seats} required /></label><label>Term years<input type="number" min="1" max="10" bind:value={termYears} required /></label><label>Election parity<select bind:value={electionParity}><option value="any">Any year</option><option value="even">Even years</option><option value="odd">Odd years</option></select></label><label>Description<textarea maxlength="1000" bind:value={positionDescription}></textarea></label><button disabled={working}>CREATE POSITION</button></form>
        <form onsubmit={(event) => { event.preventDefault(); void assignPosition(); }}><p class="eyebrow">ASSIGNMENTS</p><h2>Record an assignment</h2><label>Position<select bind:value={assignmentPositionId} required>{#each positions as position}<option value={position.id}>{position.name}</option>{/each}</select></label><label>Active member<select bind:value={assignmentUserId} required>{#each members.filter((member) => member.membership_status === 'active') as member}<option value={member.user_id}>{member.callsign} · {member.display_name}</option>{/each}</select></label><label>Seat number<input type="number" min="1" bind:value={assignmentSeat} required /></label><label>Starts<input type="date" bind:value={assignmentStarts} required /></label><label>Ends<input type="date" bind:value={assignmentEnds} required /></label><label>Selection method<select bind:value={assignmentMethod}><option value="appointed">Appointed</option><option value="elected">Elected</option><option value="acting">Acting</option></select></label><button disabled={working || !positions.length || !members.length}>RECORD ASSIGNMENT</button></form>
        <form onsubmit={(event) => { event.preventDefault(); void createElection(); }}><p class="eyebrow">ELECTION CYCLES</p><h2>Plan a cycle</h2><label>Title<input maxlength="160" bind:value={electionTitle} required /></label><label>Year<input type="number" min="2000" max="2200" bind:value={electionYear} required /></label><label>Initial status<select bind:value={electionStatus}><option value="planned">Planned</option><option value="nominations">Nominations</option><option value="voting">Voting</option><option value="closed">Closed</option><option value="certified">Certified</option><option value="cancelled">Cancelled</option></select></label><label>Opens <span class="form-help">optional</span><input type="datetime-local" bind:value={electionOpens} /></label><label>Closes <span class="form-help">optional</span><input type="datetime-local" bind:value={electionCloses} /></label><label>Positions <select multiple bind:value={electionPositionIds}>{#each positions as position}<option value={position.id}>{position.name}</option>{/each}</select></label><label>Notes<textarea maxlength="2000" bind:value={electionNotes}></textarea></label><button disabled={working}>CREATE ELECTION CYCLE</button></form>
      </div>
      <div class="records"><section><div class="section-head"><h2>Current positions</h2><b>{positions.length}</b></div>{#if positions.length}<ul>{#each positions as position}<li><span><b>{position.name}</b><small>{position.position_type} · {position.seats} seat{position.seats === 1 ? '' : 's'} · {position.term_years}-year term</small></span><em>{position.election_parity}</em></li>{/each}</ul>{:else}<p class="empty">No positions defined.</p>{/if}</section><section><div class="section-head"><h2>Assignments</h2><b>{assignments.length}</b></div>{#if assignments.length}<ul>{#each assignments as assignment}<li><span><b>{assignment.callsign} · {positions.find((position) => position.id === assignment.position_id)?.name || 'Position'}</b><small>{assignment.selection_method} · {assignment.starts_on} to {assignment.ends_on}</small></span><em>seat {assignment.seat_number}</em></li>{/each}</ul>{:else}<p class="empty">No assignments recorded.</p>{/if}</section><section><div class="section-head"><h2>Election cycles</h2><b>{elections.length}</b></div>{#if elections.length}<ul>{#each elections as election}<li><span><b>{election.title}</b><small>{election.election_year} · {election.notes || 'No notes'}</small></span><select value={election.status} onchange={(event) => void setElectionStatus(election, (event.currentTarget as HTMLSelectElement).value)}>{#each ['planned','nominations','voting','closed','certified','cancelled'] as status}<option value={status}>{status}</option>{/each}</select></li>{/each}</ul>{:else}<p class="empty">No election cycles planned.</p>{/if}</section></div>
      <section class="calendar"><div class="section-head"><div><p class="eyebrow">GOVERNANCE CALENDAR</p><h2>Schedule and deadlines</h2></div><button class="secondary" onclick={() => void exportCalendar()} disabled={working}>EXPORT .ICS</button></div>{#each calendar as entry}<article><time datetime={entry.occurs_at}>{entry.all_day ? entry.occurs_at.slice(0, 10) : new Date(entry.occurs_at).toLocaleString()}</time><span><b>{entry.title}</b><small>{entry.detail}</small></span><em>{entry.kind.replaceAll('_', ' ')}</em></article>{:else}<p class="empty">No election, term, or renewal dates are recorded.</p>{/each}</section>
      <p class="boundary-note"><b>Boundary:</b> governance records are hosted extension data. Voting, eligibility snapshots, quorum, immutable results, audit exports, and invitations remain separate follow-up capabilities.</p>
      <ClubPaymentMethodsPanel clubId={selectedClubId} />
    {:else if loading}<p class="empty">Loading governance records…</p>{/if}
  {/if}
</section>

<style>
  .governance { display:grid; gap:18px; }.section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }.section-head h1,.section-head h2 { margin:0; }.section-head b { color:var(--muted); }.eyebrow { color:var(--cyan); font-size:.72rem; letter-spacing:.12em; margin:0 0 4px; }.section-intro,.form-help,.boundary-note,.empty { color:var(--muted); }.club-picker { max-width:420px; display:grid; gap:6px; }.governance-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:14px; }.governance form { display:grid; gap:9px; border:1px solid var(--line); background:var(--panel); border-radius:10px; padding:16px; }.governance h2 { margin:0 0 4px; font-size:1.05rem; }.governance label { display:grid; gap:5px; }.governance textarea { min-height:72px; resize:vertical; }.governance select[multiple] { min-height:76px; }.records { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:14px; }.records section { border:1px solid var(--line); background:var(--panel); padding:16px; border-radius:10px; }.records ul { list-style:none; margin:10px 0 0; padding:0; display:grid; gap:8px; }.records li { display:flex; justify-content:space-between; gap:10px; border-top:1px solid var(--line); padding-top:8px; }.records li span { display:grid; gap:3px; }.records small,.records em { color:var(--muted); font-size:.76rem; font-style:normal; }.error,.notice { padding:10px 12px; border-radius:8px; }.error { color:var(--red); background:color-mix(in srgb,var(--red) 12%,transparent); }.notice { color:var(--green); background:color-mix(in srgb,var(--green) 12%,transparent); }.boundary-note { border-left:2px solid var(--amber); padding:10px 14px; }.secondary { color:var(--cyan); }@media (max-width:900px) { .governance-grid,.records { grid-template-columns:1fr; } }
  .calendar { display:grid; gap:10px; border:1px solid var(--line); background:var(--panel); padding:16px; border-radius:10px; }
  .calendar article { display:grid; grid-template-columns:110px 1fr auto; gap:12px; align-items:center; border-top:1px solid var(--line); padding-top:9px; }
  .calendar article span { display:grid; gap:3px; }.calendar small,.calendar em { color:var(--muted); font-size:.76rem; font-style:normal; }.calendar em { text-transform:uppercase; }
</style>
