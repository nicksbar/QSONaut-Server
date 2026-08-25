<script lang="ts">
  import { api } from './api';
  import type { Club, ClubJoinRequest, ClubMember, ServerCapabilities, User } from './types';

  let { clubs, capabilities, currentUser, refresh }: { clubs: Club[]; capabilities: ServerCapabilities; currentUser: User | null; refresh: () => Promise<void> } = $props();
  let name = $state(''); let callsign = $state(''); let description = $state(''); let search = $state('');
  let editingId = $state<string | null>(null); let editName = $state(''); let editCallsign = $state(''); let editDescription = $state('');
  let selectedClubId = $state<string | null>(null); let roster = $state<ClubMember[]>([]); let requests = $state<ClubJoinRequest[]>([]); let rosterSearch = $state('');
  let working = $state(false); let error = $state(''); let notice = $state('');
  let visibleClubs = $derived(clubs.filter((club) => {
    const query = search.trim().toLowerCase();
    return !query || `${club.name} ${club.callsign || ''} ${club.description}`.toLowerCase().includes(query);
  }));
  let visibleRoster = $derived(roster.filter((member) => {
    const query = rosterSearch.trim().toLowerCase();
    return !query || `${member.callsign} ${member.display_name} ${member.membership_number || ''}`.toLowerCase().includes(query);
  }));

  async function createClub() {
    working = true; error = ''; notice = '';
    try {
      const club = await api<Club>('/api/v1/clubs', { method: 'POST', body: JSON.stringify({ name, callsign: callsign || null, description }) });
      name = callsign = description = ''; notice = `Created ${club.name}.`; await refresh();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function requestJoin(club: Club) {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${club.id}/join-requests`, { method: 'POST' });
      notice = `Membership request sent to ${club.name}.`; await refresh();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  function beginEdit(club: Club) {
    editingId = club.id; editName = club.name; editCallsign = club.callsign || ''; editDescription = club.description;
  }

  async function saveEdit(clubId: string) {
    working = true; error = ''; notice = '';
    try {
      const club = await api<Club>(`/api/v1/clubs/${clubId}`, { method: 'PATCH', body: JSON.stringify({ name: editName, callsign: editCallsign || null, description: editDescription }) });
      editingId = null; notice = `Updated ${club.name}.`; await refresh();
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function loadClub(club: Club) {
    selectedClubId = club.id; roster = []; requests = []; error = '';
    if (!club.can_manage) return;
    try {
      [roster, requests] = await Promise.all([
        api<ClubMember[]>(`/api/v1/clubs/${club.id}/members`),
        api<ClubJoinRequest[]>(`/api/v1/clubs/${club.id}/join-requests`),
      ]);
    } catch (cause) { error = (cause as Error).message; }
  }

  async function review(request: ClubJoinRequest, decision: 'approve' | 'reject') {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${request.club_id}/join-requests/${request.id}`, { method: 'PATCH', body: JSON.stringify({ decision, role: 'operator' }) });
      notice = decision === 'approve' ? `${request.callsign} joined the club.` : `Request from ${request.callsign} rejected.`;
      await refresh(); const club = clubs.find((entry) => entry.id === request.club_id); if (club) await loadClub(club);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function saveMember(member: ClubMember) {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${member.club_id}/members`, { method: 'PUT', body: JSON.stringify({ user_id: member.user_id, role: member.role, membership_status: member.membership_status, dues_status: member.dues_status, membership_number: member.membership_number || null, renewal_due_on: member.renewal_due_on || null }) });
      notice = `Updated ${member.callsign}.`; await refresh(); const club = clubs.find((entry) => entry.id === member.club_id); if (club) await loadClub(club);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function removeMember(member: ClubMember) {
    if (!confirm(`Remove ${member.callsign} from this club?`)) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${member.club_id}/members/${member.user_id}`, { method: 'DELETE' });
      notice = `Removed ${member.callsign}.`; await refresh(); const club = clubs.find((entry) => entry.id === member.club_id); if (club) await loadClub(club);
    } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }
</script>

<section>
  <div class="section-head"><div><p class="eyebrow">SHARED ACTIVITY SETUP</p><h2>Clubs</h2></div><b>{clubs.length} clubs</b></div>
  <p class="section-intro">Create and discover shared activity spaces, maintain club identity, and connect operators. Board governance, elections, voting cycles, and commercial controls belong to hosted organizational extensions.</p>
  {#if error}<p class="error banner">{error}</p>{/if}{#if notice}<p class="notice banner">{notice}</p>{/if}
  <div class="club-layout">
    <div>
      <div class="list-tools"><input aria-label="Search clubs" placeholder="Search clubs or callsigns" bind:value={search} /><small>{visibleClubs.length} of {clubs.length} clubs</small></div>
      {#if clubs.length === 0}<p class="empty">No clubs yet. Register the first shared activity space.</p>{:else if visibleClubs.length === 0}<p class="empty">No clubs match the current search.</p>{/if}
      {#each visibleClubs as club}
        <article class="record club-record">
          {#if editingId === club.id}
            <form class="edit-form" onsubmit={(event) => { event.preventDefault(); void saveEdit(club.id); }}>
              <label>Name<input maxlength="120" bind:value={editName} required /></label>
              <label>Club callsign<input maxlength="16" bind:value={editCallsign} /></label>
              <label>Description<textarea maxlength="1000" bind:value={editDescription}></textarea></label>
              <div class="actions"><button disabled={working}>SAVE CLUB</button><button type="button" onclick={() => editingId = null}>CANCEL</button></div>
            </form>
          {:else}
            <div class="club-open"><span><b>{club.name}</b><small>{club.callsign || 'COMMUNITY CLUB'}</small></span><em>{club.member_count} members</em></div>
            <p>{club.description || 'No description yet.'}</p>
            <div class="actions">
              {#if club.my_role}<span class="pill">{club.my_role}</span>{/if}
              {#if club.can_manage}<button onclick={() => beginEdit(club)}>EDIT CLUB</button><button onclick={() => void loadClub(club)}>MANAGE ROSTER</button>{/if}
              {#if club.join_request_status === 'pending'}<span class="pill">REQUEST PENDING</span>
              {:else if !club.my_role}<button disabled={working} onclick={() => requestJoin(club)}>REQUEST TO JOIN</button>{/if}
            </div>
          {/if}
        </article>
  {/each}
    </div>
    <form onsubmit={(event) => { event.preventDefault(); void createClub(); }}>
      <p class="eyebrow">NEW CLUB</p><h3>Register activity space</h3>
      <p class="form-help">The public community edition supports up to {capabilities.max_clubs ?? 'unlimited'} clubs. Hosted organizational features can expand management.</p>
      <label>Name<input maxlength="120" bind:value={name} required /></label>
      <label>Club callsign<input maxlength="16" bind:value={callsign} /></label>
      <label>Description<textarea maxlength="1000" bind:value={description}></textarea></label>
      <button disabled={working || (capabilities.max_clubs !== null && clubs.length >= capabilities.max_clubs)}>{working ? 'CREATING…' : 'CREATE CLUB'}</button>
    </form>
  </div>
  {#if selectedClubId && clubs.find((club) => club.id === selectedClubId)?.can_manage}
    <div class="club-console">
      <div class="section-head"><div><p class="eyebrow">ACCESS / MEMBERSHIP</p><h2>Join requests</h2></div><b>{requests.filter((request) => request.status === 'pending').length} pending</b></div>
      {#each requests.filter((request) => request.status === 'pending') as request}
        <article class="membership"><span><b>{request.callsign}</b><small>{request.display_name} · requested {new Date(request.requested_at).toLocaleDateString()}</small></span><button disabled={working} onclick={() => review(request, 'approve')}>APPROVE</button><button class="danger" disabled={working} onclick={() => review(request, 'reject')}>REJECT</button></article>
      {:else}<p class="empty">No membership requests waiting.</p>{/each}
      <div class="section-head"><div><p class="eyebrow">ROSTER / RECORDS</p><h2>Members</h2></div><b>{roster.length} records</b></div>
      <div class="list-tools"><input aria-label="Search club roster" placeholder="Search roster" bind:value={rosterSearch} /><small>{visibleRoster.length} of {roster.length} members</small></div>
      <div class="table-wrap"><table><thead><tr><th>Member</th><th>Role</th><th>Status</th><th>Dues</th><th>Member #</th><th>Renewal</th><th></th></tr></thead><tbody>
        {#if visibleRoster.length === 0}<tr><td colspan="7"><p class="empty">No roster members match the current search.</p></td></tr>{/if}
        {#each visibleRoster as member}
          <tr><td><b>{member.callsign}</b><br/><small>{member.display_name}</small></td><td><select bind:value={member.role}><option>owner</option><option>coordinator</option><option>operator</option><option>observer</option></select></td><td><select bind:value={member.membership_status}><option>active</option><option>lapsed</option><option>inactive</option></select></td><td><select bind:value={member.dues_status}><option value="not_tracked">not tracked</option><option>current</option><option>due</option><option>overdue</option><option>waived</option></select></td><td><input aria-label="Membership number" bind:value={member.membership_number} /></td><td><input aria-label="Renewal due date" type="date" bind:value={member.renewal_due_on} /></td><td><div class="actions"><button disabled={working} onclick={() => saveMember(member)}>SAVE</button>{#if member.user_id !== currentUser?.id}<button class="danger" disabled={working} onclick={() => removeMember(member)}>REMOVE</button>{/if}</div></td></tr>
        {/each}
      </tbody></table></div>
    </div>
  {/if}
  {#if currentUser?.global_role === 'administrator'}<p class="form-help boundary-note">Roster, membership, and access-request management is available in the public administrator tools. Board governance, elections, voting cycles, and commercial controls are provided by hosted organizational extensions.</p>{/if}
</section>

<style>
  section { display:grid; gap:18px; }
  .section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }
  .section-head h2, form h3 { margin:0; }
  .section-head b { color:var(--muted); font-size:.8rem; }
  .eyebrow { color:var(--cyan); font-size:.72rem; letter-spacing:.12em; margin:0 0 4px; }
  .section-intro, .form-help, .boundary-note { color:var(--muted); }
  .club-layout { display:grid; grid-template-columns:minmax(0,1.5fr) minmax(260px,1fr); gap:18px; }
  .list-tools, .actions { display:flex; gap:8px; align-items:center; }
  .list-tools { margin-bottom:10px; }
  .list-tools input { flex:1; }
  .list-tools small { color:var(--muted); white-space:nowrap; }
  .record, form { border:1px solid var(--line); background:var(--panel); border-radius:10px; padding:14px; }
  .club-record { margin-bottom:10px; }
  .edit-form { display:grid; gap:8px; border:0; padding:0; background:transparent; }
  .club-console { margin-top:20px; display:grid; gap:12px; }
  .club-console input, .club-console select { min-width:100px; }
  .club-console .actions { margin:0; }
  .club-console td small { color:var(--muted); }
  .club-open { display:flex; justify-content:space-between; gap:12px; align-items:center; }
  .club-open span { display:grid; gap:3px; }
  .club-open small, .club-open em { color:var(--muted); font-size:.78rem; font-style:normal; }
  form { display:grid; align-content:start; gap:10px; }
  label { display:grid; gap:5px; color:var(--text); }
  textarea { min-height:80px; resize:vertical; }
  .pill { border-radius:999px; background:var(--chip); color:var(--muted); padding:3px 8px; font-size:.72rem; }
  .error, .notice { padding:10px 12px; border-radius:8px; }
  .error { color:var(--red); background:color-mix(in srgb,var(--red) 12%,transparent); }
  .notice { color:var(--green); background:color-mix(in srgb,var(--green) 12%,transparent); }
  .empty { color:var(--muted); padding:18px 0; }
  @media (max-width: 760px) { .club-layout { grid-template-columns:1fr; } }
</style>
