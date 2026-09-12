<script lang="ts">
  import { onMount } from 'svelte';
  import ActivityPanel from '$lib/ActivityPanel.svelte';
  import ActivityMapPanel from '$lib/ActivityMapPanel.svelte';
  import ActivitySummaryPanel from '$lib/ActivitySummaryPanel.svelte';
  import ActivityVisibilityPanel from '$lib/ActivityVisibilityPanel.svelte';
  import ClubsPanel from '$lib/ClubsPanel.svelte';
  import ChannelMessagesPanel from '$lib/ChannelMessagesPanel.svelte';
  import EventsPanel from '$lib/EventsPanel.svelte';
  import GovernancePanel from '$lib/GovernancePanel.svelte';
  import IdentityPanel from '$lib/IdentityPanel.svelte';
  import MembersPanel from '$lib/MembersPanel.svelte';
  import MyStationsPanel from '$lib/MyStationsPanel.svelte';
  import PaymentsPanel from '$lib/PaymentsPanel.svelte';
  import StationLinkPanel from '$lib/StationLinkPanel.svelte';
  import SiteFooter from '$lib/SiteFooter.svelte';
  import ServerManagementPanel from '$lib/ServerManagementPanel.svelte';
  import { ApiError, api } from '$lib/api';
  import type { AccessCallsignLookup, AccessChallenge, AccessRequest, ChannelMessage, Club, ContestTemplate, DiagnosticReport, Event, ManagedCallsign, QsoLog, ServerCapabilities, Station, User, UserProfile } from '$lib/types';

  type Phase = 'loading' | 'setup' | 'login' | 'ready' | 'error';
  type Tab = 'overview' | 'clubs' | 'members' | 'events' | 'governance' | 'payments' | 'identities' | 'activity' | 'station' | 'profile' | 'admin' | 'admin-operations' | 'admin-identities' | 'server-management' | 'server-data' | 'server-messages';

  let phase = $state<Phase>('loading');
  let tab = $state<Tab>('overview');
  let user = $state<User | null>(null);
  let error = $state('');
  let busy = $state(false);
  let callsign = $state('');
  let displayName = $state('');
  let password = $state('');
  let profileName = $state('');
  let profile = $state<UserProfile | null>(null);
  let newPassword = $state('');
  let profileBusy = $state(false);
  let profileNotice = $state('');
  let clubs = $state<Club[]>([]);
  let capabilities = $state<ServerCapabilities>({ edition: 'community', max_clubs: 5, features: [] });
  let members = $state<User[]>([]);
  let accessRequests = $state<AccessRequest[]>([]);
  let events = $state<Event[]>([]);
  let templates = $state<ContestTemplate[]>([]);
  let identities = $state<ManagedCallsign[]>([]);
  let clubScopeId = $state<string | null>(null);
  let stations = $state<Station[]>([]);
  let logs = $state<QsoLog[]>([]);
  let messages = $state<ChannelMessage[]>([]);
  let diagnostics = $state<DiagnosticReport[]>([]);
  let diagnosticsAdminNotice = $state('');
  let diagnosticsAdminBusy = $state(false);
  let activityRefreshInFlight = false;
  let operatorMenuOpen = $state(false);
  let joinOpen = $state(false);
  let joinBusy = $state(false);
  let joinNotice = $state('');
  let joinCalls = $state('');
  let joinEmail = $state('');
  let joinClub = $state('');
  let joinReferral = $state('');
  let joinAnswer = $state('');
  let joinChallenge = $state<AccessChallenge | null>(null);
  let joinLookup = $state<AccessCallsignLookup | null>(null);
  const qsonautDesktopUrl = 'https://github.com/nicksbar/QSONaut';
  const qsonautServerUrl = 'https://github.com/nicksbar/QSONaut-Server';
  let canManageEvents = $derived(user?.global_role === 'administrator' || clubs.some((club) => club.can_manage));
  let activeMemberships = $derived(clubs.filter((club) => club.my_membership_status === 'active'));
  let workspaceEvents = $derived(events.filter((event) => activeMemberships.some((club) => club.id === event.club_id)));
  let workspaceIdentities = $derived(identities.filter((identity) => identity.owner_user_id === user?.id || activeMemberships.some((club) => club.id === identity.club_id)));
  let scopedEvents = $derived(clubScopeId ? workspaceEvents.filter((event) => event.club_id === clubScopeId) : workspaceEvents);
  let scopedClub = $derived(clubs.find((club) => club.id === clubScopeId));
  let myLogs = $derived(logs.filter((log) => log.user_id === user?.id));

  async function refreshActivity() {
    if (activityRefreshInFlight || !user) return;
    activityRefreshInFlight = true;
    try {
      logs = await api<QsoLog[]>('/api/v1/logs');
      if (user.global_role === 'administrator') {
        [stations, messages, diagnostics] = await Promise.all([
          api<Station[]>('/api/v1/stations'),
          api<ChannelMessage[]>('/api/v1/channel-messages'),
          api<DiagnosticReport[]>('/api/v1/diagnostics'),
        ]);
      }
      else {
        [stations, diagnostics] = await Promise.all([
          api<Station[]>('/api/v1/stations'),
          api<DiagnosticReport[]>('/api/v1/diagnostics'),
        ]);
        messages = [];
      }
    } finally {
      activityRefreshInFlight = false;
    }
  }

  async function exportDiagnostics() {
    diagnosticsAdminBusy = true; diagnosticsAdminNotice = '';
    try {
      const reports = await api<DiagnosticReport[]>('/api/v1/diagnostics/export');
      const url = URL.createObjectURL(new Blob([JSON.stringify(reports, null, 2)], { type: 'application/json' }));
      const link = document.createElement('a'); link.href = url; link.download = `qsonaut-diagnostics-${new Date().toISOString().slice(0, 10)}.json`; link.click(); URL.revokeObjectURL(url);
      diagnosticsAdminNotice = `Exported ${reports.length} diagnostic reports.`;
    } catch (cause) { diagnosticsAdminNotice = (cause as Error).message; }
    finally { diagnosticsAdminBusy = false; }
  }

  async function purgeDiagnostics() {
    if (!confirm('Purge expired diagnostic reports and retained share artifacts?')) return;
    diagnosticsAdminBusy = true; diagnosticsAdminNotice = '';
    try { await api('/api/v1/diagnostics/retention/purge', { method: 'POST' }); diagnosticsAdminNotice = 'Expired retained artifacts purged.'; await refreshActivity(); }
    catch (cause) { diagnosticsAdminNotice = (cause as Error).message; }
    finally { diagnosticsAdminBusy = false; }
  }

  async function purgeAllDiagnostics() {
    if (!confirm('Permanently delete every submitted hardware validation report? This cannot be undone.')) return;
    diagnosticsAdminBusy = true; diagnosticsAdminNotice = '';
    try { await api('/api/v1/diagnostics', { method: 'DELETE' }); diagnosticsAdminNotice = 'Permanently deleted all submitted reports.'; await refreshActivity(); }
    catch (cause) { diagnosticsAdminNotice = (cause as Error).message; }
    finally { diagnosticsAdminBusy = false; }
  }

  async function load() {
    [clubs, events, templates, identities, capabilities] = await Promise.all([
      api<Club[]>('/api/v1/clubs'),
      api<Event[]>('/api/v1/events'),
      api<ContestTemplate[]>('/api/v1/contest-templates'),
      api<ManagedCallsign[]>('/api/v1/identities'),
      api<ServerCapabilities>('/api/v1/capabilities'),
    ]);
    if (user?.global_role === 'administrator') {
      [members, accessRequests] = await Promise.all([
        api<User[]>('/api/v1/members'),
        api<AccessRequest[]>('/api/v1/access/requests'),
      ]);
      await refreshActivity();
    } else {
      members = []; accessRequests = []; stations = []; messages = [];
      await refreshActivity();
    }
  }

  async function initialize() {
    try {
      const setup = await api<{ setup_required: boolean }>('/api/v1/auth/setup');
      if (setup.setup_required) { phase = 'setup'; return; }
      const current = await api<User>('/api/v1/auth/me');
      user = current; profileName = current.display_name; phase = 'ready'; await Promise.all([load(), loadProfile()]);
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 401) {
        phase = 'login';
      } else {
        error = (cause as Error).message || 'The server could not be reached.';
        phase = 'error';
      }
    }
  }

  async function authenticate() {
    busy = true; error = '';
    try {
      const setup = phase === 'setup';
      const current = await api<User>(setup ? '/api/v1/auth/setup' : '/api/v1/auth/login', {
        method: 'POST',
        body: JSON.stringify(setup
          ? { callsign, display_name: displayName, password }
          : { callsign, password }),
      });
      user = current; profileName = current.display_name; password = ''; phase = 'ready'; await Promise.all([load(), loadProfile()]);
    } catch (cause) { error = (cause as Error).message; } finally { busy = false; }
  }

  async function beginJoin() {
    joinOpen = true; joinNotice = ''; joinAnswer = ''; joinLookup = null;
    try { joinChallenge = await api<AccessChallenge>('/api/v1/access/challenge'); }
    catch (cause) { joinNotice = (cause as Error).message; }
  }

  async function lookupJoinCall() {
    joinBusy = true; joinNotice = ''; joinLookup = null;
    try {
      joinLookup = await api<AccessCallsignLookup>(`/api/v1/access/lookup/${encodeURIComponent(joinCalls.trim())}`);
      joinCalls = joinLookup.callsign;
    } catch (cause) { joinNotice = (cause as Error).message; } finally { joinBusy = false; }
  }

  async function submitJoin() {
    if (!joinChallenge) return;
    joinBusy = true; joinNotice = '';
    try {
      await api('/api/v1/access/request', { method: 'POST', body: JSON.stringify({
        challenge_id: joinChallenge.id, challenge_answer: joinAnswer, callsign: joinCalls,
        email: joinEmail, club_name: joinClub, referral_source: joinReferral,
      }) });
      joinNotice = 'Request received. An administrator will review your access request.';
      joinChallenge = null; joinAnswer = ''; joinLookup = null;
    } catch (cause) {
      joinNotice = (cause as Error).message;
      if (joinNotice.includes('new challenge')) {
        try { joinChallenge = await api<AccessChallenge>('/api/v1/access/challenge'); }
        catch (challengeError) { joinChallenge = null; joinNotice = (challengeError as Error).message; }
      }
    } finally { joinBusy = false; }
  }

  async function logout() {
    await api('/api/v1/auth/logout', { method: 'POST' });
    phase = 'login'; user = null; profile = null;
  }

  async function loadProfile() {
    profile = await api<UserProfile>('/api/v1/auth/profile');
    profileName = profile.user.display_name;
  }

  function openTab(next: Tab) {
    tab = next;
    if (next !== 'events') clubScopeId = null;
    operatorMenuOpen = false;
    if (next === 'profile' && user) void loadProfile().catch((cause) => { profileNotice = (cause as Error).message; });
  }

  function openClubOperations(club: Club) {
    clubScopeId = club.id;
    tab = 'events';
    operatorMenuOpen = false;
  }

  function profileInput() {
    if (!profile) return null;
    return {
      display_name: profileName,
      grid: profile.grid, qth: profile.qth,
      first_name: profile.first_name, middle_name: profile.middle_name,
      surname: profile.surname, suffix: profile.suffix,
      license_class: profile.license_class, license_status: profile.license_status,
      license_expires_on: profile.license_expires_on,
      address_line_1: profile.address_line_1, address_line_2: profile.address_line_2,
      state: profile.state, postal_code: profile.postal_code, country: profile.country,
      latitude: profile.latitude, longitude: profile.longitude,
    };
  }

  async function saveProfile() {
    profileBusy = true; profileNotice = '';
    try {
      const input = profileInput();
      if (!input) return;
      profile = await api<UserProfile>('/api/v1/auth/profile', { method: 'PATCH', body: JSON.stringify(input) });
      user = profile.user;
      profileNotice = 'Profile saved.';
    } catch (cause) { profileNotice = (cause as Error).message; } finally { profileBusy = false; }
  }

  function openGovernance(club: Club) {
    clubScopeId = club.id;
    tab = 'governance';
    operatorMenuOpen = false;
  }

  async function refreshHamdb() {
    profileBusy = true; profileNotice = '';
    try {
      profile = await api<UserProfile>('/api/v1/auth/profile/hamdb', { method: 'POST' });
      user = profile.user; profileName = profile.user.display_name;
      profileNotice = 'License profile refreshed from HamDB. Review it, then save any local edits.';
    } catch (cause) { profileNotice = (cause as Error).message; } finally { profileBusy = false; }
  }

  async function savePassword() {
    profileBusy = true; profileNotice = '';
    try {
      await api('/api/v1/auth/me/password', { method: 'POST', body: JSON.stringify({ password: newPassword }) });
      newPassword = ''; profileNotice = 'Password changed. Sign in again with the new password.'; user = null; phase = 'login';
    } catch (cause) { profileNotice = (cause as Error).message; } finally { profileBusy = false; }
  }

  onMount(() => {
    void initialize();
    const activityTimer = window.setInterval(() => {
      if (phase === 'ready' && tab === 'activity') {
        void refreshActivity().catch((cause) => { error = (cause as Error).message; });
      }
    }, 5_000);
    return () => window.clearInterval(activityTimer);
  });
</script>

<svelte:head>
  <title>QSONaut · Amateur Radio Mission Control</title>
  <meta name="description" content="QSONaut amateur radio mission control for operators, clubs, contests, and shared station activity." />
</svelte:head>

{#if phase === 'loading'}
  <main class="center mono cyan">ESTABLISHING CONTROL LINK…</main>
  {:else if phase === 'error'}
  <div class="public-page">
    <main class="center">
      <section class="auth"><a class="auth-brand" href="https://qsonaut.com"><img src="/qsonaut-icon.png" alt="" /><span><strong>QSONaut</strong><small>AMATEUR RADIO MISSION CONTROL</small></span></a>
        <p class="eyebrow amber">CONTROL LINK UNAVAILABLE</p>
        <h1>Server did not respond</h1>
        <p class="error">{error}</p>
        <button onclick={() => { phase = 'loading'; error = ''; void initialize(); }}>RETRY CONNECTION</button>
        <p class="form-help">If this persists, inspect the server process logs and the PostgreSQL connection.</p>
      </section>
    </main>
    <SiteFooter />
  </div>
{:else if phase === 'setup' || phase === 'login'}
  <div class="public-page">
    <main class="center">
    <section class="auth"><a class="auth-brand" href="https://qsonaut.com"><img src="/qsonaut-icon.png" alt="" /><span><strong>QSONaut</strong><small>AMATEUR RADIO MISSION CONTROL</small></span></a>
      <p class="eyebrow amber">{phase === 'setup' ? 'FIRST RUN / COMMAND AUTHORITY' : 'AUTHENTICATION REQUIRED'}</p>
      <h1>{phase === 'setup' ? 'Initialize server' : 'Operator sign in'}</h1>
      <form onsubmit={(event) => { event.preventDefault(); authenticate(); }}>
        <label>Callsign<input maxlength="16" bind:value={callsign} required /></label>
        {#if phase === 'setup'}<label>Display name<input maxlength="100" bind:value={displayName} required /></label>{/if}
        <label>Password<input type="password" minlength="12" maxlength="256" bind:value={password} required /></label>
        {#if error}<p class="error">{error}</p>{/if}
        <button disabled={busy}>{busy ? 'WORKING…' : phase === 'setup' ? 'INITIALIZE' : 'SIGN IN'}</button>
      </form>
      {#if phase === 'login'}
        <button class="secondary" type="button" onclick={() => void beginJoin()}>REQUEST ACCESS</button>
        {#if joinOpen}
          <section class="join-request">
            <p class="eyebrow cyan">NEW OPERATOR / REQUEST ACCESS</p>
            <h2>Join QSONaut</h2>
            <p class="form-help">Use a valid amateur callsign. We verify it with HamDB before sending the request to an administrator.</p>
            <form onsubmit={(event) => { event.preventDefault(); void submitJoin(); }}>
              <label>Callsign <div class="inline-form"><input maxlength="16" bind:value={joinCalls} required /><button class="secondary" type="button" onclick={() => void lookupJoinCall()} disabled={joinBusy || !joinCalls.trim()}>LOOK UP</button></div></label>
              {#if joinLookup}<p class="notice">HamDB: {joinLookup.display_name || joinLookup.callsign} · {joinLookup.license_class} · {joinLookup.grid || 'grid unavailable'}</p>{/if}
              <label>Email address<input type="email" maxlength="254" bind:value={joinEmail} required /></label>
              <label>Club <span class="form-help">optional</span><input maxlength="160" bind:value={joinClub} placeholder="Club name, if applicable" /></label>
              <label>Where did you hear about QSONaut?<input maxlength="240" bind:value={joinReferral} /></label>
              {#if joinChallenge}<label>Technical check: {joinChallenge.question}<input maxlength="80" bind:value={joinAnswer} required /></label>{/if}
              {#if joinNotice}<p class:error={joinNotice.toLowerCase().includes('error') || joinNotice.toLowerCase().includes('invalid')}>{joinNotice}</p>{/if}
              <button disabled={joinBusy || !joinChallenge}>{joinBusy ? 'WORKING…' : 'SEND ACCESS REQUEST'}</button>
            </form>
          </section>
        {/if}
      {/if}
    </section>
    </main>
    <SiteFooter />
  </div>
{:else}
  <div class="console-shell">
    <aside class="console-sidebar">
      <a class="console-brand" href="https://qsonaut.com" aria-label="QSONaut home"><img src="/qsonaut-icon.png" alt="" /><span><strong>QSONaut</strong><small>OPERATIONS CONSOLE</small></span></a>
      <div class="console-edition"><span>DEPLOYMENT</span><b>{capabilities.edition}</b></div>
      <nav class="console-nav" aria-label="Management console">
        <p>MY WORKSPACE</p>
        <button class:active={tab === 'overview'} onclick={() => openTab('overview')}><span>◫</span> Home</button>
        <button class:active={tab === 'events' && !clubScopeId} onclick={() => openTab('events')}><span>◌</span> My operations</button>
        <button class:active={tab === 'identities'} onclick={() => openTab('identities')}><span>⌁</span> My identities</button>
        <p>MY ORGANIZATIONS</p>
        {#each activeMemberships as club}<button class:active={tab === 'events' && clubScopeId === club.id} class="club-nav-item" onclick={() => openClubOperations(club)}><span>◉</span><span>{club.name}<small>{club.my_role}</small></span></button>{#if club.can_manage}<button class:active={tab === 'governance' && clubScopeId === club.id} class="club-nav-item sub-nav" onclick={() => openGovernance(club)}><span>◇</span><span>Governance<small>{club.name}</small></span></button>{/if}{/each}
        {#if capabilities.edition !== 'community'}<button class:active={tab === 'payments'} onclick={() => openTab('payments')}><span>¤</span> Club payments</button>{/if}
        <button class:active={tab === 'clubs'} onclick={() => openTab('clubs')}><span>＋</span> Find / request</button>
        <p>MY STATION &amp; DATA</p>
        <button class:active={tab === 'activity'} onclick={() => openTab('activity')}><span>≋</span> Sharing center</button>
        <button class:active={tab === 'station'} onclick={() => openTab('station')}><span>⌘</span> My Stations</button>
        {#if user?.global_role === 'administrator'}<p>ADMINISTRATION</p><button class:active={tab === 'admin'} onclick={() => openTab('admin')}><span>♙</span> Accounts &amp; access</button><button class:active={tab === 'admin-operations'} onclick={() => openTab('admin-operations')}><span>◌</span> Organization repair</button><button class:active={tab === 'admin-identities'} onclick={() => openTab('admin-identities')}><span>⌁</span> Identity review</button>{#if capabilities.edition !== 'community'}<button class:active={tab === 'server-management'} onclick={() => openTab('server-management')}><span>¤</span> Server management</button>{/if}<p>SERVER DATA</p><button class:active={tab === 'server-data'} onclick={() => openTab('server-data')}><span>⌁</span> Hardware validation</button><button class:active={tab === 'server-messages'} onclick={() => openTab('server-messages')}><span>›_</span> Channel traffic</button>{/if}
      </nav>
      <div class="sidebar-foot"><a href={qsonautDesktopUrl} target="_blank" rel="noreferrer">Desktop app ↗</a><a href={qsonautServerUrl} target="_blank" rel="noreferrer">Server docs ↗</a></div>
    </aside>
    <div class="console-content">
      <header class="console-header"><div><p class="eyebrow">QSONAUT SERVER</p><b>{tab === 'events' ? scopedClub ? scopedClub.name + ' operations' : 'My operations' : tab === 'clubs' ? 'Find organizations' : tab === 'payments' ? 'Club payments' : tab === 'identities' ? 'My operating identities' : tab === 'activity' ? 'My activity and station data' : tab === 'admin' ? 'Global administration' : tab === 'server-data' ? 'Server logs and hardware validation' : tab === 'station' ? 'Desktop connection' : 'My workspace'}</b></div><div class="site-header-right"><div class="operator"><i></i><div class="operator-menu-wrap"><button class="operator-trigger" aria-expanded={operatorMenuOpen} onclick={() => operatorMenuOpen = !operatorMenuOpen}><b>{user?.callsign}</b><span>⌄</span></button>{#if operatorMenuOpen}<div class="operator-menu"><button onclick={() => openTab('profile')}>PROFILE &amp; SECURITY</button><button onclick={() => openTab('clubs')}>FIND / REQUEST ORGANIZATION</button><button onclick={() => openTab('activity')}>MY LOGS / SHARING</button><button onclick={() => openTab('station')}>STATION LINK</button><button onclick={logout}>SIGN OUT</button></div>{/if}</div></div></div></header>
        <nav class="compact-nav" aria-label="Compact management navigation"><button class:active={tab === 'overview'} onclick={() => openTab('overview')}>Home</button><button class:active={tab === 'clubs'} onclick={() => openTab('clubs')}>Find clubs</button><button class:active={tab === 'events'} onclick={() => openTab('events')}>Operations</button>{#if capabilities.edition !== 'community'}<button class:active={tab === 'payments'} onclick={() => openTab('payments')}>Payments</button>{/if}{#if capabilities.edition !== 'community' && activeMemberships.some((club) => club.can_manage)}<button class:active={tab === 'governance'} onclick={() => openGovernance(activeMemberships.find((club) => club.can_manage)!)}>{scopedClub ? scopedClub.name + ' governance' : 'Governance'}</button>{/if}<button class:active={tab === 'identities'} onclick={() => openTab('identities')}>Identities</button><button class:active={tab === 'activity'} onclick={() => openTab('activity')}>Activity</button>{#if user?.global_role === 'administrator'}<button class:active={tab === 'admin'} onclick={() => openTab('admin')}>Admin</button>{#if capabilities.edition !== 'community'}<button class:active={tab === 'server-management'} onclick={() => openTab('server-management')}>Server management</button>{/if}<button class:active={tab === 'server-data'} onclick={() => openTab('server-data')}>Server data</button>{/if}</nav>
      {#if error}<p class="error banner">{error}</p>{/if}
      <main class="console-main">
      {#if tab === 'overview'}
        <section class="overview-workspace">
          <div class="overview-intro"><div><p class="eyebrow amber">MY OPERATING WORKSPACE</p><h1>Operate with your organizations.</h1><p>Your home shows only the clubs you actively belong to and the operations they run. Server-wide data is reserved for administrators in Server oversight.</p></div><div class="overview-actions"><button onclick={() => openTab('events')}>MY OPERATIONS</button><button class="secondary" onclick={() => openTab('station')}>CONNECT A STATION</button></div></div>
          <div class="metrics"><button onclick={() => openTab('events')}><b>{workspaceEvents.filter((event) => event.status === 'active').length}</b><span>ACTIVE OPERATIONS</span><small>Across your organizations</small></button><button onclick={() => openTab('clubs')}><b>{activeMemberships.length}</b><span>MY ORGANIZATIONS</span><small>Active memberships</small></button><button onclick={() => openTab('clubs')}><b>{activeMemberships.filter((club) => club.can_manage).reduce((sum, club) => sum + club.renewal_attention_count, 0)}</b><span>RENEWALS DUE</span><small>In organizations you manage</small></button><button onclick={() => openTab('events')}><b>{workspaceEvents.filter((event) => event.status === 'scheduled').length}</b><span>UPCOMING</span><small>Your scheduled windows</small></button></div>
          <div class="overview-grid"><section class="command-card"><p class="eyebrow">NEXT ACTIONS</p><h2>My operations queue</h2>{#if workspaceEvents.filter((event) => event.status === 'scheduled').length}<button class="queue-item" onclick={() => openTab('events')}><span>◌</span><div><b>{workspaceEvents.filter((event) => event.status === 'scheduled').length} scheduled operation{workspaceEvents.filter((event) => event.status === 'scheduled').length === 1 ? '' : 's'}</b><small>Review lineups, callsigns, and rules before activation.</small></div><em>Open →</em></button>{/if}{#if activeMemberships.some((club) => club.can_manage && club.renewal_attention_count)}<button class="queue-item" onclick={() => openTab('clubs')}><span>◎</span><div><b>Membership records need attention</b><small>Review renewal and dues status in your organization workspace.</small></div><em>Open →</em></button>{/if}{#if !workspaceEvents.filter((event) => event.status === 'scheduled').length && !activeMemberships.some((club) => club.can_manage && club.renewal_attention_count)}<p class="empty">Nothing needs immediate action. Use an organization in the sidebar to plan its next activity.</p>{/if}</section><section class="command-card identity-summary"><p class="eyebrow">OPERATING AUTHORITY</p><h2>My identity readiness</h2><b>{workspaceIdentities.filter((identity) => identity.status === 'active' && identity.verification_status === 'verified').length} ready callsigns</b><p>{workspaceIdentities.filter((identity) => identity.verification_status === 'pending').length} identity request{workspaceIdentities.filter((identity) => identity.verification_status === 'pending').length === 1 ? '' : 's'} awaiting verification.</p><button class="secondary" onclick={() => openTab('identities')}>OPEN MY IDENTITIES</button></section></div>
          <ActivitySummaryPanel clubs={activeMemberships} events={workspaceEvents} />
          <MyStationsPanel stations={stations} compact={true} />
        </section>
        {#if capabilities.edition !== 'community'}
          <div class="boundary hosted-boundary"><b>HOSTED EXTENSIONS · {capabilities.edition.toUpperCase()}</b><p>{capabilities.features.length} hosted capabilities are active.</p><div class="feature-list">{#each capabilities.features as feature}<span class="pill">{feature}</span>{/each}</div></div>
        {/if}
      {:else if tab === 'profile'}
        <section class="profile-panel"><p class="eyebrow amber">OPERATOR / IDENTITY</p><h2>{user?.callsign}</h2><p class="lede">Your operator identity powers individual activity, club participation, and contest reporting. License and address details stay in your private profile unless a future sharing surface explicitly says otherwise.</p><ActivitySummaryPanel clubs={activeMemberships} events={workspaceEvents} />
          {#if profile}<form class="profile-form" onsubmit={(event) => { event.preventDefault(); void saveProfile(); }}><div class="profile-section"><div><p class="eyebrow cyan">IDENTITY</p><h3>Operator profile</h3></div><button type="button" class="secondary" onclick={refreshHamdb} disabled={profileBusy}>↻ REFRESH FROM HAMDB</button><label>Callsign<input value={profile.user.callsign} readonly /></label><label>Display name<input maxlength="100" bind:value={profileName} required /></label><label>First name<input maxlength="80" bind:value={profile.first_name} /></label><label>Middle name<input maxlength="80" bind:value={profile.middle_name} /></label><label>Surname<input maxlength="120" bind:value={profile.surname} /></label><label>Suffix<input maxlength="40" bind:value={profile.suffix} /></label><label>Grid locator<input maxlength="16" bind:value={profile.grid} placeholder="e.g. CN87" /></label><label>QTH / station location<input maxlength="160" bind:value={profile.qth} placeholder="City, region" /></label></div><div class="profile-section"><div><p class="eyebrow cyan">LICENSE RECORD</p><h3>License details</h3></div><label>Class<input maxlength="40" bind:value={profile.license_class} /></label><label>Status<input maxlength="40" bind:value={profile.license_status} /></label><label>Expiration<input type="date" bind:value={profile.license_expires_on} /></label><label>Country<input maxlength="80" bind:value={profile.country} /></label><label>Latitude<input maxlength="32" bind:value={profile.latitude} /></label><label>Longitude<input maxlength="32" bind:value={profile.longitude} /></label></div><div class="profile-section address-section"><div><p class="eyebrow cyan">PRIVATE CONTACT</p><h3>Mailing address</h3><p class="form-help">Used for your account record and future club workflows. It is not included in activity summaries or copy links.</p></div><label>Address line 1<input maxlength="160" bind:value={profile.address_line_1} /></label><label>Address line 2<input maxlength="160" bind:value={profile.address_line_2} /></label><label>State / region<input maxlength="80" bind:value={profile.state} /></label><label>Postal code<input maxlength="32" bind:value={profile.postal_code} /></label></div><button disabled={profileBusy}>{profileBusy ? 'WORKING…' : 'SAVE OPERATOR PROFILE'}</button></form>{:else}<p class="notice">Loading operator profile…</p>{/if}
          {#if profile?.hamdb_last_error}<p class="error">HamDB: {profile.hamdb_last_error}</p>{/if}<form class="compact" onsubmit={(event) => { event.preventDefault(); void savePassword(); }}><h3>Change password</h3><label>New password<input type="password" minlength="12" maxlength="256" bind:value={newPassword} required /></label><button class="danger" disabled={profileBusy}>CHANGE PASSWORD</button></form>{#if profileNotice}<p class="notice">{profileNotice}</p>{/if}<div class="profile-grid"><div><small>GLOBAL ACCESS</small><b>{user?.global_role}</b></div><div><small>HAMDB SYNC</small><b>{profile?.hamdb_fetched_at ? new Date(profile.hamdb_fetched_at).toLocaleString() : 'not synced'}</b></div><div><small>ACTIVITY DATA</small><b>summarized overall, by club, or by contest</b></div></div></section>
      {:else if tab === 'clubs'}<ClubsPanel {clubs} {capabilities} currentUser={user} refresh={load} />
      {:else if tab === 'events'}<EventsPanel events={scopedEvents} clubs={activeMemberships} {templates} administrator={user?.global_role === 'administrator'} refresh={load} />
      {:else if tab === 'governance' && capabilities.edition !== 'community'}<GovernancePanel clubs={activeMemberships} initialClubId={clubScopeId} />
      {:else if tab === 'payments' && capabilities.edition !== 'community'}<PaymentsPanel clubs={activeMemberships} />
      {:else if tab === 'identities'}<IdentityPanel identities={workspaceIdentities} clubs={activeMemberships} events={workspaceEvents} />
      {:else if tab === 'activity'}<section class="sharing-workspace"><div class="admin-intro"><p class="eyebrow amber">MY ACTIVITY / PRIVACY</p><h1>Activity and privacy</h1><p>Review permitted activity and manage the callsign, club, and event policies you own. Uploading logs from QSONaut remains a separate station setting.</p></div><ActivityMapPanel clubs={activeMemberships} identities={workspaceIdentities} events={workspaceEvents} /><ActivityVisibilityPanel identities={workspaceIdentities} clubs={activeMemberships} events={workspaceEvents} /><ActivityPanel administrator={false} {capabilities} {stations} logs={myLogs} {messages} {diagnostics} refresh={refreshActivity} /></section>
      {:else if tab === 'admin' && user?.global_role === 'administrator'}<section class="admin-workspace"><div class="admin-intro"><p class="eyebrow amber">ADMINISTRATION / ACCOUNTS</p><h1>Accounts &amp; access</h1><p>Review access requests, operator accounts, and account recovery. Organization repair and identity review are separate administrator workspaces.</p></div><MembersPanel {members} {clubs} {accessRequests} refresh={load} /></section>
      {:else if tab === 'admin-operations' && user?.global_role === 'administrator'}<section class="admin-workspace"><div class="admin-intro"><p class="eyebrow amber">ADMINISTRATION / ORGANIZATIONS</p><h1>Organization repair</h1><p>Resolve cross-organization operation, contest, roster, and assignment issues without mixing them with account or identity administration.</p></div><EventsPanel {events} {clubs} {templates} administrator={true} refresh={load} /></section>
      {:else if tab === 'admin-identities' && user?.global_role === 'administrator'}<section class="admin-workspace"><div class="admin-intro"><p class="eyebrow amber">ADMINISTRATION / IDENTITIES</p><h1>Identity review</h1><p>Review global callsign records and their organization or event authority separately from operator accounts and contest operations.</p></div><IdentityPanel {identities} {clubs} {events} /></section>
      {:else if tab === 'server-management' && user?.global_role === 'administrator' && capabilities.edition !== 'community'}<section class="admin-workspace"><ServerManagementPanel /></section>
      {:else if tab === 'server-data' && user?.global_role === 'administrator'}<section class="admin-workspace"><div class="admin-intro"><p class="eyebrow amber">SERVER DATA / ADMINISTRATOR ONLY</p><h1>Hardware validation</h1><p>Review opt-in hardware validation snapshots alongside their connected-station context. Submitted QSO logs and automation traffic are retained separately and are not part of this validation workspace.</p><div class="actions"><button onclick={() => void exportDiagnostics()} disabled={diagnosticsAdminBusy}>EXPORT REPORTS</button><button class="danger" onclick={() => void purgeDiagnostics()} disabled={diagnosticsAdminBusy}>PURGE EXPIRED DATA</button><button class="danger" onclick={() => void purgeAllDiagnostics()} disabled={diagnosticsAdminBusy}>PURGE ALL SUBMITTED REPORTS</button></div>{#if diagnosticsAdminNotice}<p class="notice">{diagnosticsAdminNotice}</p>{/if}</div><ActivityPanel administrator={true} {capabilities} {stations} {logs} {messages} {diagnostics} refresh={refreshActivity} showLogs={false} showMessages={false} /></section>
      {:else if tab === 'server-messages' && user?.global_role === 'administrator'}<section class="admin-workspace"><ChannelMessagesPanel {messages} /></section>
      {:else}<MyStationsPanel {stations} /><StationLinkPanel currentUser={user} />{/if}
    </main>
    </div>
  </div>
  <SiteFooter />
{/if}

<style>
  .public-page { min-height:100vh; width:min(1280px,calc(100% - 40px)); margin:auto; display:flex; flex-direction:column; }
  .public-page .center { flex:1; min-height:auto; padding:48px 0; }
  .auth-brand { display:flex; align-items:center; gap:12px; color:inherit; text-decoration:none; }
  .auth-brand { margin-bottom:28px; }
  .auth-brand img { width:58px; height:58px; object-fit:contain; }
  .auth-brand strong { display:block; color:var(--cyan); font-size:clamp(24px,4vw,38px); font-weight:400; letter-spacing:-.04em; }
  .auth-brand small { display:block; margin-top:3px; color:#ef826e; font:600 9px ui-monospace; letter-spacing:.06em; }
  .overview-workspace { display:grid; gap:28px; }
  .club-nav-item { align-items:start !important; }.club-nav-item > span:last-child { display:grid; gap:2px; text-align:left; }.club-nav-item small { color:var(--muted); font-size:.68rem; text-transform:uppercase; }.club-nav-item.sub-nav { padding-left:28px; color:var(--muted); }
  .admin-workspace { display:grid; gap:22px; }.admin-intro { border:1px solid color-mix(in srgb,var(--amber) 40%,var(--line)); background:color-mix(in srgb,var(--amber) 8%,var(--panel)); border-radius:12px; padding:20px; }.admin-intro h1,.admin-intro p { margin:4px 0; }.admin-intro p:last-child { color:var(--muted); max-width:760px; }
  .overview-intro { display:flex; justify-content:space-between; align-items:end; gap:24px; }
  .overview-intro h1 { max-width:700px; margin:4px 0 10px; font-size:clamp(36px,5vw,66px); }
  .overview-intro p { max-width:670px; margin:0; color:var(--muted); font-size:16px; line-height:1.55; }
  .overview-actions { display:flex; flex-wrap:wrap; gap:9px; }
  .overview-grid { display:grid; grid-template-columns:minmax(0,1.35fr) minmax(280px,.65fr); gap:16px; }
  .command-card { padding:22px; border:1px solid var(--line); background:var(--panel); }
  .command-card h2 { margin:4px 0 17px; }
  .queue-item { width:100%; display:grid; grid-template-columns:auto 1fr auto; align-items:center; gap:13px; padding:13px 0; border:0; border-top:1px solid var(--line); border-radius:0; background:transparent; color:inherit; text-align:left; cursor:pointer; }
  .queue-item:first-of-type { border-top:0; }
  .queue-item:hover b { color:var(--cyan); }
  .queue-item > span { color:var(--amber); font-size:20px; }
  .queue-item div { display:grid; gap:4px; }.queue-item small,.queue-item em { color:var(--muted); font-style:normal; font-size:12px; }.queue-item em { color:var(--cyan); }
  .identity-summary > b { display:block; margin:8px 0; color:var(--cyan); font:500 27px ui-monospace; }.identity-summary p { color:var(--muted); line-height:1.5; }
  .profile-panel { max-width: 850px; }
  .lede { max-width: 760px; color: var(--muted); line-height: 1.6; }
  .profile-form { display: grid; gap: 14px; margin-top: 28px; }
  .profile-section { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; padding: 22px; border: 1px solid var(--line); background: #07161c; }
  .profile-section > div { grid-column: 1 / -1; display: flex; align-items: baseline; justify-content: space-between; gap: 18px; }
  .profile-section h3 { margin: 4px 0 0; }
  .profile-section label { display: grid; gap: 7px; }
  .profile-section input { width: 100%; box-sizing: border-box; }
  .profile-section input[readonly] { color: var(--muted); }
  .profile-form > button { justify-self: start; }
  .secondary { color: var(--cyan); border: 1px solid var(--line); padding: 9px 12px; font: 600 10px ui-monospace; letter-spacing: .08em; }
  .secondary:hover { background: #0d3038; }
  .address-section { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .address-section .form-help { grid-column: 1 / -1; }
  .profile-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1px; margin-top: 32px; border: 1px solid var(--line); background: var(--line); }
  .profile-grid div { display: grid; gap: 10px; padding: 22px; background: #07161c; }
  .profile-grid small { color: var(--muted); font: 600 10px ui-monospace; letter-spacing: .1em; }
  .profile-grid b { color: var(--cyan); }
  @media (max-width: 650px) { .profile-grid { grid-template-columns: 1fr; } }
  @media (max-width: 650px) { .profile-section, .address-section { grid-template-columns: 1fr; } .profile-section > div { display: block; } }
  @media (max-width: 850px) { .overview-intro { align-items:stretch; flex-direction:column; } .overview-grid { grid-template-columns:1fr; } }
  @media (max-width: 520px) { .public-page { width:calc(100% - 24px); } }
</style>
