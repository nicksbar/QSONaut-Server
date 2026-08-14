<script lang="ts">
  import { api } from './api';
  import type { Club, ClubGovernance, ClubJoinRequest, ClubMember, User } from './types';

  let { clubs, currentUser, refresh }: { clubs: Club[]; currentUser: User | null; refresh: () => Promise<void> } = $props();
  let name = $state(''); let callsign = $state(''); let description = $state('');
  let selectedClubId = $state<string | null>(null);
  let roster = $state<ClubMember[]>([]); let requests = $state<ClubJoinRequest[]>([]);
  let governance = $state<ClubGovernance>({ positions: [], assignments: [], elections: [] });
  let positionName = $state(''); let positionType = $state('officer'); let positionSeats = $state(1); let positionYears = $state(2); let positionParity = $state('any');
  let assignmentPosition = $state(''); let assignmentUser = $state(''); let assignmentSeat = $state(1); let assignmentStart = $state(''); let assignmentEnd = $state(''); let assignmentMethod = $state('elected');
  let electionTitle = $state(''); let electionYear = $state(new Date().getFullYear()); let electionStatus = $state('planned'); let electionOpens = $state(''); let electionCloses = $state(''); let electionPositions = $state<string[]>([]);
  let working = $state(false); let error = $state(''); let notice = $state('');

  async function createClub() {
    working = true; error = ''; notice = '';
    try {
      const club = await api<Club>('/api/v1/clubs', { method: 'POST', body: JSON.stringify({ name, callsign: callsign || null, description }) });
      name = callsign = description = ''; notice = `Created ${club.name}. You are its owner.`;
      await refresh(); selectedClubId = club.id; await loadClub(club.id);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function requestJoin(club: Club) {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${club.id}/join-requests`, { method: 'POST' });
      notice = `Membership request sent to ${club.name}.`; await refresh();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function loadClub(clubId: string) {
    const club = clubs.find((entry) => entry.id === clubId);
    selectedClubId = clubId; roster = []; requests = []; governance = { positions: [], assignments: [], elections: [] }; error = '';
    try {
      governance = await api<ClubGovernance>(`/api/v1/clubs/${clubId}/governance`);
      if (club?.can_manage) [roster, requests] = await Promise.all([api<ClubMember[]>(`/api/v1/clubs/${clubId}/members`), api<ClubJoinRequest[]>(`/api/v1/clubs/${clubId}/join-requests`)]);
    } catch (cause) { error = (cause as Error).message; }
  }

  async function review(request: ClubJoinRequest, decision: 'approve' | 'reject') {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${request.club_id}/join-requests/${request.id}`, { method: 'PATCH', body: JSON.stringify({ decision, role: 'operator' }) });
      notice = decision === 'approve' ? `${request.callsign} joined as an operator.` : `Request from ${request.callsign} rejected.`;
      await refresh(); await loadClub(request.club_id);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function saveMember(member: ClubMember) {
    working = true; error = ''; notice = '';
    try {
      const updated = await api<ClubMember>(`/api/v1/clubs/${member.club_id}/members`, { method: 'PUT', body: JSON.stringify({ user_id: member.user_id, role: member.role, membership_status: member.membership_status, dues_status: member.dues_status, membership_number: member.membership_number || null, renewal_due_on: member.renewal_due_on || null }) });
      notice = `Updated ${updated.callsign}.`; await refresh(); await loadClub(member.club_id);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function removeMember(member: ClubMember) {
    if (!confirm(`Remove ${member.callsign} from this club?`)) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${member.club_id}/members/${member.user_id}`, { method: 'DELETE' });
      notice = `Removed ${member.callsign}.`; await refresh(); await loadClub(member.club_id);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function createPosition() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/positions`, { method: 'POST', body: JSON.stringify({ name: positionName, position_type: positionType, seats: positionSeats, term_years: positionYears, election_parity: positionParity, description: '' }) });
      notice = `Created ${positionName}.`; positionName = ''; await loadClub(selectedClubId);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function assignPosition() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/position-assignments`, { method: 'POST', body: JSON.stringify({ position_id: assignmentPosition, user_id: assignmentUser, seat_number: assignmentSeat, starts_on: assignmentStart, ends_on: assignmentEnd, selection_method: assignmentMethod }) });
      notice = 'Recorded position term.'; await loadClub(selectedClubId);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  function toggleElectionPosition(id: string, selected: boolean) {
    electionPositions = selected
      ? (electionPositions.includes(id) ? electionPositions : [...electionPositions, id])
      : electionPositions.filter((candidate) => candidate !== id);
  }

  async function createElection() {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/elections`, { method: 'POST', body: JSON.stringify({ title: electionTitle, election_year: electionYear, status: electionStatus, opens_at: electionOpens ? new Date(electionOpens).toISOString() : null, closes_at: electionCloses ? new Date(electionCloses).toISOString() : null, notes: '', position_ids: electionPositions }) });
      notice = `Scheduled ${electionTitle}.`; electionTitle = ''; electionPositions = []; await loadClub(selectedClubId);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function updateElection(election: ClubGovernance['elections'][number]) {
    if (!selectedClubId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/elections/${election.id}`, { method: 'PATCH', body: JSON.stringify({ status: election.status }) });
      notice = `Moved ${election.title} to ${election.status}.`; await loadClub(selectedClubId);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }
</script>

<section>
  <div class="section-head"><div><p class="eyebrow">CLUB OPERATIONS</p><h2>Organizations</h2></div><b>{clubs.length} clubs</b></div>
  <p class="section-intro">Create a club, request membership, and keep roster work visible before renewals become a scramble.</p>
  {#if error}<p class="error banner">{error}</p>{/if}{#if notice}<p class="notice banner">{notice}</p>{/if}
  <div class="club-layout">
    <div>
      {#if clubs.length === 0}<p class="empty">No clubs yet. Register the first one and become its owner.</p>{/if}
      {#each clubs as club}
        <article class="record club-record" class:active={selectedClubId === club.id}>
          <button class="club-open" onclick={() => loadClub(club.id)}><span><b>{club.name}</b><small>{club.callsign || 'COMMUNITY CLUB'}</small></span><em>{club.member_count} members{club.renewal_attention_count ? ` · ${club.renewal_attention_count} need attention` : ''}</em></button>
          <p>{club.description || 'No description yet.'}</p>
          <div class="actions">
            {#if club.my_role}<span class="pill">{club.my_role}</span>{/if}
            {#if club.can_manage}<button onclick={() => loadClub(club.id)}>MANAGE ROSTER</button>
            {:else if club.join_request_status === 'pending'}<span class="pill">REQUEST PENDING</span>
            {:else if !club.my_role}<button disabled={working} onclick={() => requestJoin(club)}>REQUEST TO JOIN</button>{/if}
          </div>
        </article>
      {/each}
    </div>
    <form onsubmit={(event) => { event.preventDefault(); createClub(); }}>
      <p class="eyebrow">NEW CLUB</p><h3>Register organization</h3><p class="form-help">The creator becomes the first owner and can approve membership requests.</p>
      <label>Name<input maxlength="120" bind:value={name} required /></label><label>Club callsign<input maxlength="16" bind:value={callsign} /></label><label>Description<textarea maxlength="1000" bind:value={description}></textarea></label>
      <button disabled={working}>{working ? 'CREATING…' : 'CREATE CLUB'}</button>
    </form>
  </div>

  {#if selectedClubId && clubs.find((club) => club.id === selectedClubId)?.can_manage}
    <div class="club-console">
      <div class="section-head"><div><p class="eyebrow">APPROVAL QUEUE</p><h2>Join requests</h2></div><b>{requests.filter((request) => request.status === 'pending').length} pending</b></div>
      {#each requests.filter((request) => request.status === 'pending') as request}
        <article class="membership"><span><b>{request.callsign}</b><small>{request.display_name} · requested {new Date(request.requested_at).toLocaleDateString()}</small></span><button disabled={working} onclick={() => review(request, 'approve')}>APPROVE</button><button class="danger" disabled={working} onclick={() => review(request, 'reject')}>REJECT</button></article>
      {:else}<p class="empty">No membership requests waiting.</p>{/each}
      <div class="section-head"><div><p class="eyebrow">ROSTER / RENEWALS</p><h2>Members</h2></div><b>{roster.length} records</b></div>
      <div class="table-wrap"><table><thead><tr><th>Member</th><th>Role</th><th>Status</th><th>Dues</th><th>Member #</th><th>Renewal</th><th></th></tr></thead><tbody>
        {#each roster as member}
          <tr><td><b>{member.callsign}</b><br/><small>{member.display_name}</small></td><td><select bind:value={member.role}><option>owner</option><option>coordinator</option><option>operator</option><option>observer</option></select></td><td><select bind:value={member.membership_status}><option>active</option><option>lapsed</option><option>inactive</option></select></td><td><select bind:value={member.dues_status}><option value="not_tracked">not tracked</option><option>current</option><option>due</option><option>overdue</option><option>waived</option></select></td><td><input aria-label="Membership number" bind:value={member.membership_number} /></td><td><input aria-label="Renewal due date" type="date" bind:value={member.renewal_due_on} /></td><td><div class="actions"><button disabled={working} onclick={() => saveMember(member)}>SAVE</button>{#if member.user_id !== currentUser?.id}<button class="danger" disabled={working} onclick={() => removeMember(member)}>REMOVE</button>{/if}</div></td></tr>
        {/each}
      </tbody></table></div>
    </div>
  {/if}

  {#if selectedClubId}
    <div class="club-console">
      <div class="section-head"><div><p class="eyebrow">GOVERNANCE</p><h2>Officers & board</h2></div><b>{governance.positions.length} positions</b></div>
      <div class="governance-grid">
        {#each governance.positions as position}
          <article class="template-card"><div class="detail-head"><b>{position.name}</b><span class="pill">{position.position_type}</span></div><p>{position.seats} seat{position.seats === 1 ? '' : 's'} · {position.term_years}-year term · {position.election_parity === 'any' ? 'every year cycle' : `${position.election_parity}-year elections`}</p>
            {#each governance.assignments.filter((assignment) => assignment.position_id === position.id) as assignment}<div class="rule-strip"><b>Seat {assignment.seat_number}: {assignment.callsign}</b><span>{assignment.selection_method} · {assignment.starts_on} → {assignment.ends_on}</span></div>{:else}<small class="empty">No recorded terms.</small>{/each}
          </article>
        {:else}<p class="empty">Define officer and board positions to build the governance calendar.</p>{/each}
      </div>

      <div class="section-head"><div><p class="eyebrow">ELECTION CALENDAR</p><h2>Voting cycles</h2></div><b>{governance.elections.length} cycles</b></div>
      {#each governance.elections as election}
        <article class="record"><b>{election.title}</b><span class="pill">{election.status}</span><p>{election.election_year} · {election.position_ids.map((id) => governance.positions.find((position) => position.id === id)?.name).filter(Boolean).join(', ') || 'positions to be assigned'}{election.opens_at ? ` · opens ${new Date(election.opens_at).toLocaleDateString()}` : ''}</p>{#if currentUser?.global_role === 'administrator' || clubs.find((club) => club.id === selectedClubId)?.my_role === 'owner'}<div class="actions"><select aria-label="Election status" bind:value={election.status}><option>planned</option><option>nominations</option><option>voting</option><option>closed</option><option>certified</option><option>cancelled</option></select><button disabled={working} onclick={() => updateElection(election)}>UPDATE WORKFLOW</button></div>{/if}</article>
      {:else}<p class="empty">No election cycles scheduled.</p>{/each}

      {#if currentUser?.global_role === 'administrator' || clubs.find((club) => club.id === selectedClubId)?.my_role === 'owner'}
        <div class="governance-forms">
          <form class="compact" onsubmit={(event) => { event.preventDefault(); createPosition(); }}><h3>Define position</h3><label>Title<input bind:value={positionName} required /></label><label>Type<select bind:value={positionType}><option>officer</option><option>board</option></select></label><label>Seats<input type="number" min="1" max="100" bind:value={positionSeats} required /></label><label>Term years<input type="number" min="1" max="10" bind:value={positionYears} required /></label><label>Election cycle<select bind:value={positionParity}><option value="any">annual / any year</option><option value="even">even years</option><option value="odd">odd years</option></select></label><button disabled={working}>ADD POSITION</button></form>
          <form class="compact" onsubmit={(event) => { event.preventDefault(); assignPosition(); }}><h3>Record term</h3><label>Position<select bind:value={assignmentPosition} required><option value="">Select</option>{#each governance.positions as position}<option value={position.id}>{position.name}</option>{/each}</select></label><label>Member<select bind:value={assignmentUser} required><option value="">Select</option>{#each roster.filter((member) => member.membership_status === 'active') as member}<option value={member.user_id}>{member.callsign} · {member.display_name}</option>{/each}</select></label><label>Seat<input type="number" min="1" bind:value={assignmentSeat} /></label><label>Method<select bind:value={assignmentMethod}><option>elected</option><option>appointed</option><option>acting</option></select></label><label>Starts<input type="date" bind:value={assignmentStart} required /></label><label>Ends<input type="date" bind:value={assignmentEnd} required /></label><button disabled={working}>RECORD TERM</button></form>
          <form class="compact" onsubmit={(event) => { event.preventDefault(); createElection(); }}><h3>Schedule election</h3><label>Title<input bind:value={electionTitle} required /></label><label>Election year<input type="number" min="2000" max="2200" bind:value={electionYear} /></label><label>Status<select bind:value={electionStatus}><option>planned</option><option>nominations</option><option>voting</option><option>closed</option><option>certified</option><option>cancelled</option></select></label><label>Opens<input type="datetime-local" bind:value={electionOpens} /></label><label>Closes<input type="datetime-local" bind:value={electionCloses} /></label><fieldset><legend>Positions on ballot</legend>{#each governance.positions as position}<label class="check"><input type="checkbox" checked={electionPositions.includes(position.id)} onchange={(event) => toggleElectionPosition(position.id, event.currentTarget.checked)} />{position.name}</label>{/each}</fieldset><button disabled={working}>SCHEDULE CYCLE</button></form>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .club-layout,.governance-forms{display:grid;grid-template-columns:1.3fr .8fr;gap:38px}.club-record{padding:18px}.club-record.active{box-shadow:inset 2px 0 var(--cyan);background:#0a2027}.club-open{grid-column:1/-1;display:flex;justify-content:space-between;text-align:left;width:100%;border:0;background:transparent;color:#dce9eb;cursor:pointer;padding:0}.club-open span{display:grid}.club-open small,.club-open em{color:var(--muted);font-style:normal}.club-console{margin-top:45px}.club-console input,.club-console select{min-width:110px}.club-console .actions{margin:0}.club-console td small{color:var(--muted)}.governance-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px;margin-bottom:35px}.governance-forms{grid-template-columns:repeat(3,1fr);margin-top:35px}.governance-forms fieldset{border:1px solid var(--line);display:grid;gap:6px}.check{display:flex;align-items:center}.check input{width:auto}@media(max-width:900px){.club-layout,.governance-forms{grid-template-columns:1fr}}
</style>
