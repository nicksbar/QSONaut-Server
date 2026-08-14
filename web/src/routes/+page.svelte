<script lang="ts">
  import { onMount } from 'svelte';
  import ActivityPanel from '$lib/ActivityPanel.svelte';
  import ClubsPanel from '$lib/ClubsPanel.svelte';
  import EventsPanel from '$lib/EventsPanel.svelte';
  import MembersPanel from '$lib/MembersPanel.svelte';
  import { api } from '$lib/api';
  import type { Club, ContestTemplate, Event, QsoLog, Station, User } from '$lib/types';

  type Phase = 'loading' | 'setup' | 'login' | 'ready';
  type Tab = 'overview' | 'clubs' | 'members' | 'events' | 'activity';

  let phase = $state<Phase>('loading');
  let tab = $state<Tab>('overview');
  let user = $state<User | null>(null);
  let error = $state('');
  let busy = $state(false);
  let callsign = $state('');
  let displayName = $state('');
  let password = $state('');
  let clubs = $state<Club[]>([]);
  let members = $state<User[]>([]);
  let events = $state<Event[]>([]);
  let templates = $state<ContestTemplate[]>([]);
  let stations = $state<Station[]>([]);
  let logs = $state<QsoLog[]>([]);

  async function load() {
    [clubs, members, events, templates, stations, logs] = await Promise.all([
      api<Club[]>('/api/v1/clubs'),
      api<User[]>('/api/v1/members'),
      api<Event[]>('/api/v1/events'),
      api<ContestTemplate[]>('/api/v1/contest-templates'),
      api<Station[]>('/api/v1/stations'),
      api<QsoLog[]>('/api/v1/logs'),
    ]);
  }

  async function initialize() {
    try {
      const setup = await api<{ setup_required: boolean }>('/api/v1/auth/setup');
      if (setup.setup_required) { phase = 'setup'; return; }
      const current = await api<User>('/api/v1/auth/me');
      if (current.global_role !== 'administrator') {
        await api('/api/v1/auth/logout', { method: 'POST' });
        phase = 'login'; return;
      }
      user = current; phase = 'ready'; await load();
    } catch { phase = 'login'; }
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
      if (current.global_role !== 'administrator') {
        await api('/api/v1/auth/logout', { method: 'POST' });
        throw new Error('The web console requires administrator access. Use QSONaut for event activity.');
      }
      user = current; password = ''; phase = 'ready'; await load();
    } catch (cause) { error = (cause as Error).message; } finally { busy = false; }
  }

  async function logout() {
    await api('/api/v1/auth/logout', { method: 'POST' });
    phase = 'login'; user = null;
  }

  onMount(initialize);
</script>

<svelte:head><title>QSONaut Server</title></svelte:head>

{#if phase === 'loading'}
  <main class="center mono cyan">ESTABLISHING CONTROL LINK…</main>
{:else if phase === 'setup' || phase === 'login'}
  <main class="center">
    <section class="auth">
      <p class="eyebrow amber">{phase === 'setup' ? 'FIRST RUN / COMMAND AUTHORITY' : 'AUTHENTICATION REQUIRED'}</p>
      <h1>{phase === 'setup' ? 'Initialize server' : 'Operator sign in'}</h1>
      <form onsubmit={(event) => { event.preventDefault(); authenticate(); }}>
        <label>Callsign<input maxlength="16" bind:value={callsign} required /></label>
        {#if phase === 'setup'}<label>Display name<input maxlength="100" bind:value={displayName} required /></label>{/if}
        <label>Password<input type="password" minlength="12" maxlength="256" bind:value={password} required /></label>
        {#if error}<p class="error">{error}</p>{/if}
        <button disabled={busy}>{busy ? 'WORKING…' : phase === 'setup' ? 'INITIALIZE' : 'SIGN IN'}</button>
      </form>
    </section>
  </main>
{:else}
  <div class="shell">
    <header><div><p class="eyebrow cyan">QSONAUT / GROUP OPERATIONS</p><h1>Command registry</h1></div><div class="operator"><i></i><b>{user?.callsign}</b><button onclick={logout}>SIGN OUT</button></div></header>
    <nav>{#each ['overview','clubs','members','events','activity'] as item}<button class:active={tab === item} onclick={() => tab = item as Tab}>{item}</button>{/each}</nav>
    {#if error}<p class="error banner">{error}</p>{/if}
    <main>
      {#if tab === 'overview'}
        <p class="eyebrow amber">CONTROL PLANE / ONLINE</p>
        <h2 class="hero">{events.filter((event) => event.status === 'active').length} active operations</h2>
        <div class="metrics"><article><b>{clubs.length}</b>CLUBS</article><article><b>{members.length}</b>OPERATORS</article><article><b>{stations.filter((station) => station.status === 'online').length}</b>ONLINE</article><article><b>{logs.length}</b>LOGS</article></div>
        <div class="boundary"><b>Management and coordination live here.</b><p>Radio control, decoding, audio, PTT, and operating workflows remain in QSONaut.</p></div>
      {:else if tab === 'clubs'}<ClubsPanel {clubs} refresh={load} />
      {:else if tab === 'members'}<MembersPanel {members} {clubs} refresh={load} />
      {:else if tab === 'events'}<EventsPanel {events} {clubs} {templates} refresh={load} />
      {:else}<ActivityPanel {stations} {logs} />{/if}
    </main>
    <footer class="mono">MANAGEMENT ONLY <span>NO RADIO CONTROL · NO AUDIO · NO PTT · NO MODEM</span></footer>
  </div>
{/if}
