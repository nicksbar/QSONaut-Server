<script lang="ts">
  import { api, formatFrequency } from './api';
  import type { AccessRequest, Club, MemberDetail, User } from './types';

  let { members, clubs, accessRequests, refresh }: { members: User[]; clubs: Club[]; accessRequests: AccessRequest[]; refresh: () => Promise<void> } = $props();
  let pendingAccessRequests = $derived(accessRequests.filter((item) => item.status === 'pending'));
  let accessSearch = $state('');
  let visibleAccessRequests = $derived(pendingAccessRequests.filter((request) => {
    const query = accessSearch.trim().toLowerCase();
    return !query || `${request.callsign} ${request.hamdb_display_name} ${request.email} ${request.club_name}`.toLowerCase().includes(query);
  }));
  let memberSearch = $state('');
  let memberRoleFilter = $state('all');
  let memberPage = $state(1);
  const memberPageSize = 25;
  let filteredMembers = $derived(members.filter((member) => {
    const query = memberSearch.trim().toLowerCase();
    return (!query || `${member.callsign} ${member.display_name}`.toLowerCase().includes(query))
      && (memberRoleFilter === 'all' || member.global_role === memberRoleFilter);
  }));
  let memberPageCount = $derived(Math.max(1, Math.ceil(filteredMembers.length / memberPageSize)));
  let visibleMembers = $derived(filteredMembers.slice((memberPage - 1) * memberPageSize, memberPage * memberPageSize));
  let selected = $state<MemberDetail | null>(null);
  let displayName = $state('');
  let newPassword = $state('');
  let assignClub = $state('');
  let role = $state('operator');
  let globalRole = $state('member');
  let memberCall = $state('');
  let memberName = $state('');
  let memberPassword = $state('');
  let working = $state(false);
  let error = $state('');
  let notice = $state('');

  $effect(() => {
    if (!clubs.some((club) => club.id === assignClub)) assignClub = clubs[0]?.id || '';
  });

  async function run(work: () => Promise<void>) {
    working = true;
    error = notice = '';
    try { await work(); } catch (cause) { error = (cause as Error).message; } finally { working = false; }
  }

  async function openMember(id: string) {
    await run(async () => {
      selected = await api<MemberDetail>(`/api/v1/members/${id}`);
      displayName = selected.user.display_name;
      globalRole = selected.user.global_role;
    });
  }

  async function reloadSelected() {
    if (selected) selected = await api<MemberDetail>(`/api/v1/members/${selected.user.id}`);
  }

  async function createMember() {
    await run(async () => {
      const created = await api<User>('/api/v1/members', {
        method: 'POST',
        body: JSON.stringify({ callsign: memberCall, display_name: memberName, password: memberPassword }),
      });
      memberCall = memberName = memberPassword = '';
      await refresh();
      selected = await api<MemberDetail>(`/api/v1/members/${created.id}`);
      displayName = selected.user.display_name;
      notice = `${created.callsign} created. Assign a club role below.`;
    });
  }

  async function saveProfile() {
    if (!selected) return;
    await run(async () => {
      await api(`/api/v1/members/${selected!.user.id}`, {
        method: 'PATCH', body: JSON.stringify({ display_name: displayName, global_role: globalRole }),
      });
      await refresh(); await reloadSelected(); notice = 'Operator profile updated.';
    });
  }

  async function resetPassword() {
    if (!selected) return;
    await run(async () => {
      await api(`/api/v1/members/${selected!.user.id}/password`, {
        method: 'POST', body: JSON.stringify({ password: newPassword }),
      });
      newPassword = ''; notice = 'Password changed; existing sessions were revoked.';
    });
  }

  async function assign() {
    if (!selected || !assignClub) return;
    await run(async () => {
      await api(`/api/v1/clubs/${assignClub}/members`, {
        method: 'PUT', body: JSON.stringify({ user_id: selected!.user.id, role }),
      });
      await reloadSelected(); notice = 'Club assignment saved.';
    });
  }

  async function remove(clubId: string) {
    if (!selected) return;
    await run(async () => {
      await api(`/api/v1/clubs/${clubId}/members/${selected!.user.id}`, { method: 'DELETE' });
      await reloadSelected(); notice = 'Club assignment removed.';
    });
  }

  async function decideAccess(request: AccessRequest, decision: 'approved' | 'rejected') {
    await run(async () => {
      const result = await api<{ temporary_password?: string }>(`/api/v1/access/requests/${request.id}`, { method: 'PATCH', body: JSON.stringify({ decision }) });
      await refresh();
      notice = result.temporary_password
        ? `${request.callsign} account created. Temporary password (copy now; it is shown only once): ${result.temporary_password}`
        : `${request.callsign} access request ${decision}.`;
    });
  }

  function resetMemberPage() { memberPage = 1; }
</script>

<section class="member-layout">
  {#if error}<p class="error member-banner">{error}</p>{/if}
  {#if notice}<p class="notice member-banner">{notice}</p>{/if}
  <div>
    <section class="subsection access-requests">
      <p class="eyebrow amber">ACCESS / PENDING REVIEW</p>
      <div class="list-heading"><h2>Join requests</h2><small>{pendingAccessRequests.length} pending</small></div>
      <div class="list-tools"><input aria-label="Search access requests" placeholder="Search callsign, email, or club" bind:value={accessSearch} /></div>
      {#if pendingAccessRequests.length === 0}
        <p class="empty">No pending access requests.</p>
      {:else if visibleAccessRequests.length === 0}
        <p class="empty">No requests match the current search.</p>
      {:else}
        {#each visibleAccessRequests as request}
          <article class="membership"><span><b>{request.callsign}</b><small>{request.hamdb_display_name || 'HamDB name unavailable'} · {request.email}</small><small>{request.club_name || 'No club specified'} · {request.referral_source || 'No referral provided'}</small></span><span><button onclick={() => decideAccess(request, 'approved')} disabled={working}>APPROVE</button><button class="danger" onclick={() => decideAccess(request, 'rejected')} disabled={working}>REJECT</button></span></article>
        {/each}
      {/if}
    </section>
    <p class="eyebrow">PEOPLE / CLICK TO MANAGE</p>
    <div class="list-heading"><h2>Operators</h2><small>{filteredMembers.length} of {members.length}</small></div>
    <div class="list-tools"><input aria-label="Search operators" placeholder="Search callsign or name" bind:value={memberSearch} oninput={resetMemberPage} /><select aria-label="Filter operators by role" bind:value={memberRoleFilter} onchange={resetMemberPage}><option value="all">All roles</option><option value="administrator">Administrators</option><option value="member">Members</option></select></div>
    {#if visibleMembers.length === 0}
      <p class="empty">No operators match the current filters.</p>
    {:else}
      {#each visibleMembers as member}
        <button class:active={selected?.user.id === member.id} class="member-row" onclick={() => openMember(member.id)}>
          <span><b>{member.callsign}</b><small>{member.display_name}</small></span>
          <em>{member.global_role}</em>
        </button>
      {/each}
      <div class="list-pager"><button class="secondary" disabled={memberPage <= 1} onclick={() => memberPage -= 1}>PREVIOUS</button><small>PAGE {memberPage} / {memberPageCount}</small><button class="secondary" disabled={memberPage >= memberPageCount} onclick={() => memberPage += 1}>NEXT</button></div>
    {/if}
    <form class="compact" onsubmit={(event) => { event.preventDefault(); createMember(); }}>
      <h3>Add operator</h3>
      <label>Callsign<input maxlength="16" bind:value={memberCall} required /></label>
      <label>Display name<input maxlength="100" bind:value={memberName} required /></label>
      <label>Initial password<input type="password" minlength="12" maxlength="256" bind:value={memberPassword} required /></label>
      <button disabled={working}>CREATE AND OPEN</button>
    </form>
  </div>

  <div class="detail-panel">
    {#if selected}
      <div class="detail-head"><div><p class="eyebrow">OPERATOR RECORD</p><h2>{selected.user.callsign}</h2></div><span class="pill">{selected.user.global_role}</span></div>
      <form class="compact" onsubmit={(event) => { event.preventDefault(); saveProfile(); }}>
        <h3>Identity</h3>
        <label>Display name<input maxlength="100" bind:value={displayName} required /></label>
        <label>Global access<select bind:value={globalRole}><option value="member">member</option><option value="administrator">administrator</option></select></label>
        <button disabled={working}>SAVE PROFILE</button>
      </form>

      <section class="subsection">
        <h3>Club roles</h3>
        {#if selected.memberships.length === 0}<p class="empty">Not assigned to a club.</p>{/if}
        {#each selected.memberships as membership}
          <div class="membership"><span><b>{membership.club_name}</b><small>{membership.club_callsign || 'No club call'}</small></span><em>{membership.role}</em><button class="danger" onclick={() => remove(membership.club_id)}>REMOVE</button></div>
        {/each}
        <form class="inline-form" onsubmit={(event) => { event.preventDefault(); assign(); }}>
          <label>Club<select bind:value={assignClub}>{#each clubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>
          <label>Role<select bind:value={role}>{#each ['owner','coordinator','operator','observer'] as item}<option value={item}>{item}</option>{/each}</select></label>
          <button disabled={working || !assignClub}>ASSIGN / UPDATE</button>
        </form>
      </section>

      <section class="subsection">
        <h3>QSONaut stations</h3>
        {#if selected.stations.length === 0}<p class="empty">No opt-in station presence has been published by this operator yet.</p>{/if}
        {#each selected.stations as station}
          <article class="station-card"><i class:online={station.status === 'online'}></i><div><b>{station.station_label || 'QSONaut station'}</b><p>{[station.radio_manufacturer, station.radio_model].filter(Boolean).join(' ') || 'Radio not shared'} · {formatFrequency(station.frequency_hz)} · {station.mode || 'mode unknown'}</p><small>QSONaut {station.qsonaut_version} / {station.platform} · {new Date(station.last_seen).toLocaleString()}</small></div></article>
        {/each}
      </section>

      <form class="compact" onsubmit={(event) => { event.preventDefault(); resetPassword(); }}>
        <h3>Reset password</h3>
        <label>New password<input type="password" minlength="12" maxlength="256" bind:value={newPassword} required /></label>
        <button class="danger" disabled={working}>RESET AND REVOKE SESSIONS</button>
      </form>
    {:else}
      <div class="empty-state"><span>↗</span><h2>Select an operator</h2><p>Open their identity, club roles, QSONaut stations, and access controls.</p></div>
    {/if}
  </div>
</section>

<style>
  .list-heading { display:flex; align-items:baseline; justify-content:space-between; gap:12px; }
  .list-heading h2 { margin-bottom:0; }
  .list-heading small, .list-pager small { color:var(--muted); }
  .list-tools { display:grid; grid-template-columns:1fr 180px; gap:8px; margin:12px 0; }
  .list-tools input, .list-tools select { min-width:0; width:100%; box-sizing:border-box; }
  .list-pager { display:flex; align-items:center; justify-content:center; gap:14px; margin:14px 0 24px; }
  .list-pager .secondary { padding:7px 10px; }
  @media (max-width:600px) { .list-tools { grid-template-columns:1fr; } }
</style>
