<script lang="ts">
  import { onMount } from 'svelte';
  import ActivityPanel from '$lib/ActivityPanel.svelte';
  import ActivitySummaryPanel from '$lib/ActivitySummaryPanel.svelte';
  import ActivityVisibilityPanel from '$lib/ActivityVisibilityPanel.svelte';
  import ClubsPanel from '$lib/ClubsPanel.svelte';
  import EventsPanel from '$lib/EventsPanel.svelte';
  import MembersPanel from '$lib/MembersPanel.svelte';
  import StationLinkPanel from '$lib/StationLinkPanel.svelte';
  import SiteFooter from '$lib/SiteFooter.svelte';
  import { ApiError, api } from '$lib/api';
  import type { AccessCallsignLookup, AccessChallenge, AccessRequest, ChannelMessage, Club, ContestTemplate, DiagnosticReport, Event, QsoLog, ServerCapabilities, Station, User, UserProfile } from '$lib/types';

  type Phase = 'loading' | 'setup' | 'login' | 'ready' | 'error';
  type Tab = 'overview' | 'clubs' | 'members' | 'events' | 'activity' | 'station' | 'profile';

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
  let stations = $state<Station[]>([]);
  let logs = $state<QsoLog[]>([]);
  let messages = $state<ChannelMessage[]>([]);
  let diagnostics = $state<DiagnosticReport[]>([]);
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
    } finally {
      activityRefreshInFlight = false;
    }
  }

  async function load() {
    [clubs, events, templates, capabilities] = await Promise.all([
      api<Club[]>('/api/v1/clubs'),
      api<Event[]>('/api/v1/events'),
      api<ContestTemplate[]>('/api/v1/contest-templates'),
      api<ServerCapabilities>('/api/v1/capabilities'),
    ]);
    if (user?.global_role === 'administrator') {
      members = await api<User[]>('/api/v1/members');
      accessRequests = await api<AccessRequest[]>('/api/v1/access/requests');
      await refreshActivity();
    } else {
      members = []; accessRequests = []; stations = []; messages = []; diagnostics = [];
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
    operatorMenuOpen = false;
    if (next === 'profile' && user) void loadProfile().catch((cause) => { profileNotice = (cause as Error).message; });
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
  <div class="shell">
    <header class="site-header"><a class="site-brand" href="https://qsonaut.com" aria-label="QSONaut home"><img src="/qsonaut-icon.png" alt="" /><span><strong>QSONaut</strong><small>AMATEUR RADIO MISSION CONTROL</small></span></a><div class="site-header-right"><nav class="site-links"><a href={qsonautDesktopUrl} target="_blank" rel="noreferrer">PROJECT</a><a href={qsonautServerUrl} target="_blank" rel="noreferrer">SERVER</a><a href="https://qsonaut.com" target="_blank" rel="noreferrer">QSONAUT.COM</a></nav><div class="operator"><i></i><div class="operator-menu-wrap"><button class="operator-trigger" aria-expanded={operatorMenuOpen} onclick={() => operatorMenuOpen = !operatorMenuOpen}><b>{user?.callsign}</b><span>⌄</span></button>{#if operatorMenuOpen}<div class="operator-menu"><button onclick={() => openTab('profile')}>PROFILE</button><button onclick={() => openTab('activity')}>MY LOGS / SHARING</button><button onclick={() => openTab('station')}>STATION LINK</button><button onclick={logout}>SIGN OUT</button></div>{/if}</div></div></div></header>
    <nav>{#each (user?.global_role === 'administrator' ? ['overview','clubs','members','events','activity','station'] : ['overview','clubs','activity','station']) as item}<button class:active={tab === item} onclick={() => tab = item as Tab}>{item === 'station' ? 'station link' : item === 'activity' ? 'my activity' : item}</button>{/each}</nav>
    {#if error}<p class="error banner">{error}</p>{/if}
    <main>
      {#if tab === 'overview'}
        <p class="eyebrow amber">CONTROL PLANE / ONLINE</p>
        <h2 class="hero">{events.filter((event) => event.status === 'active').length} active operations</h2>
        <div class="metrics"><article><b>{clubs.length}</b>CLUBS</article><article><b>{clubs.filter((club) => club.my_role).length}</b>MY CLUBS</article><article><b>{clubs.filter((club) => club.can_manage).reduce((sum, club) => sum + club.renewal_attention_count, 0)}</b>RENEWALS DUE</article><article><b>{events.filter((event) => event.status === 'scheduled').length}</b>UPCOMING</article></div>
        <ActivitySummaryPanel {clubs} {events} /><ActivityVisibilityPanel {clubs} {events} />
        <div class="boundary"><b>One home for group operations.</b><p>Configure contests, coordinate operators, follow station activity, collect logs, and build club reports.</p></div>
      {:else if tab === 'profile'}
        <section class="profile-panel"><p class="eyebrow amber">OPERATOR / IDENTITY</p><h2>{user?.callsign}</h2><p class="lede">Your operator identity powers individual activity, club participation, and contest reporting. License and address details stay in your private profile unless a future sharing surface explicitly says otherwise.</p><ActivitySummaryPanel {clubs} {events} />
          {#if profile}<form class="profile-form" onsubmit={(event) => { event.preventDefault(); void saveProfile(); }}><div class="profile-section"><div><p class="eyebrow cyan">IDENTITY</p><h3>Operator profile</h3></div><button type="button" class="secondary" onclick={refreshHamdb} disabled={profileBusy}>↻ REFRESH FROM HAMDB</button><label>Callsign<input value={profile.user.callsign} readonly /></label><label>Display name<input maxlength="100" bind:value={profileName} required /></label><label>First name<input maxlength="80" bind:value={profile.first_name} /></label><label>Middle name<input maxlength="80" bind:value={profile.middle_name} /></label><label>Surname<input maxlength="120" bind:value={profile.surname} /></label><label>Suffix<input maxlength="40" bind:value={profile.suffix} /></label><label>Grid locator<input maxlength="16" bind:value={profile.grid} placeholder="e.g. CN87" /></label><label>QTH / station location<input maxlength="160" bind:value={profile.qth} placeholder="City, region" /></label></div><div class="profile-section"><div><p class="eyebrow cyan">LICENSE RECORD</p><h3>License details</h3></div><label>Class<input maxlength="40" bind:value={profile.license_class} /></label><label>Status<input maxlength="40" bind:value={profile.license_status} /></label><label>Expiration<input type="date" bind:value={profile.license_expires_on} /></label><label>Country<input maxlength="80" bind:value={profile.country} /></label><label>Latitude<input maxlength="32" bind:value={profile.latitude} /></label><label>Longitude<input maxlength="32" bind:value={profile.longitude} /></label></div><div class="profile-section address-section"><div><p class="eyebrow cyan">PRIVATE CONTACT</p><h3>Mailing address</h3><p class="form-help">Used for your account record and future club workflows. It is not included in activity summaries or copy links.</p></div><label>Address line 1<input maxlength="160" bind:value={profile.address_line_1} /></label><label>Address line 2<input maxlength="160" bind:value={profile.address_line_2} /></label><label>State / region<input maxlength="80" bind:value={profile.state} /></label><label>Postal code<input maxlength="32" bind:value={profile.postal_code} /></label></div><button disabled={profileBusy}>{profileBusy ? 'WORKING…' : 'SAVE OPERATOR PROFILE'}</button></form>{:else}<p class="notice">Loading operator profile…</p>{/if}
          {#if profile?.hamdb_last_error}<p class="error">HamDB: {profile.hamdb_last_error}</p>{/if}<form class="compact" onsubmit={(event) => { event.preventDefault(); void savePassword(); }}><h3>Change password</h3><label>New password<input type="password" minlength="12" maxlength="256" bind:value={newPassword} required /></label><button class="danger" disabled={profileBusy}>CHANGE PASSWORD</button></form>{#if profileNotice}<p class="notice">{profileNotice}</p>{/if}<div class="profile-grid"><div><small>GLOBAL ACCESS</small><b>{user?.global_role}</b></div><div><small>HAMDB SYNC</small><b>{profile?.hamdb_fetched_at ? new Date(profile.hamdb_fetched_at).toLocaleString() : 'not synced'}</b></div><div><small>ACTIVITY DATA</small><b>summarized overall, by club, or by contest</b></div></div></section>
      {:else if tab === 'clubs'}<ClubsPanel {clubs} {capabilities} currentUser={user} refresh={load} />
      {:else if tab === 'members'}<MembersPanel {members} {clubs} {accessRequests} refresh={load} />
      {:else if tab === 'events'}<EventsPanel {events} {clubs} {templates} refresh={load} />
      {:else if tab === 'activity'}<ActivitySummaryPanel {clubs} {events} /><ActivityVisibilityPanel {clubs} {events} /><ActivityPanel administrator={user?.global_role === 'administrator'} {stations} {logs} {messages} {diagnostics} refresh={refreshActivity} />
      {:else}<StationLinkPanel currentUser={user} />{/if}
    </main>
    <SiteFooter />
  </div>
{/if}

<style>
  .public-page { min-height:100vh; width:min(1280px,calc(100% - 40px)); margin:auto; display:flex; flex-direction:column; }
  .public-page .center { flex:1; min-height:auto; padding:48px 0; }
  .auth-brand, .site-brand { display:flex; align-items:center; gap:12px; color:inherit; text-decoration:none; }
  .auth-brand { margin-bottom:28px; }
  .auth-brand img { width:58px; height:58px; object-fit:contain; }
  .auth-brand strong, .site-brand strong { display:block; color:var(--cyan); font-size:clamp(24px,4vw,38px); font-weight:400; letter-spacing:-.04em; }
  .auth-brand small, .site-brand small { display:block; margin-top:3px; color:#ef826e; font:600 9px ui-monospace; letter-spacing:.06em; }
  .site-header { gap:24px; padding:18px 0; }
  .site-brand img { width:56px; height:56px; object-fit:contain; }
  .site-header-right { display:flex; align-items:center; gap:22px; }
  .site-links { display:flex; align-items:center; gap:4px; border:0; overflow:visible; }
  .site-links a { padding:8px 10px; color:var(--muted); font:600 10px ui-monospace; letter-spacing:.08em; text-decoration:none; }
  .site-links a:hover { color:var(--cyan); }
  .site-header .operator { white-space:nowrap; }
  .operator-menu-wrap { position: relative; }
  .operator-trigger { display: flex; align-items: center; gap: 7px; padding: 8px !important; color: #dce9eb !important; }
  .operator-trigger span { color: var(--cyan); }
  .operator-menu { position: absolute; right: 0; top: 100%; z-index: 5; min-width: 190px; border: 1px solid var(--line); background: #07161cf5; box-shadow: 0 12px 30px #0008; }
  .operator-menu button { display: block; width: 100%; padding: 12px 15px; text-align: left; color: var(--cyan); font: 600 10px ui-monospace; letter-spacing: .1em; }
  .operator-menu button:hover { background: #0d3038; }
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
  @media (max-width: 850px) { .site-header { align-items:flex-start; flex-direction:column; } .site-header-right { width:100%; justify-content:space-between; gap:10px; } }
  @media (max-width: 520px) { .public-page { width:calc(100% - 24px); } .site-header-right { align-items:flex-start; flex-direction:column; } .site-links { flex-wrap:wrap; } .site-brand img { width:48px; height:48px; } }
</style>
