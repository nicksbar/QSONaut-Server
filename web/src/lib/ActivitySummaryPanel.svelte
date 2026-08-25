<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api';
  import type { ActivitySummary, Club, Event } from './types';

  let { clubs, events }: { clubs: Club[]; events: Event[] } = $props();
  let scope = $state('overall');
  let scopeId = $state('');
  let periodDays = $state(30);
  let summary = $state<ActivitySummary | null>(null);
  let error = $state('');
  let loading = $state(false);
  let myClubs = $derived(clubs.filter((club) => club.my_role));
  let myEvents = $derived(events.filter((event) => myClubs.some((club) => club.id === event.club_id)));

  async function load() {
    loading = true;
    error = '';
    const query = new URLSearchParams({ scope, period_days: String(periodDays) });
    if (scopeId) query.set('scope_id', scopeId);
    try {
      summary = await api<ActivitySummary>(`/api/v1/activity/summary?${query}`);
    } catch (cause) {
      summary = null;
      error = (cause as Error).message;
    } finally {
      loading = false;
    }
  }

  function changeScope(next: string) {
    scope = next;
    scopeId = next === 'club' ? myClubs[0]?.id || '' : next === 'contest' ? myEvents[0]?.id || '' : '';
    void load();
  }

  onMount(() => void load());
</script>

<section class="summary-panel">
  <div class="section-head"><div><p class="eyebrow amber">OPERATOR STATUS</p><h2>Activity status</h2></div>{#if summary}<span class:quiet={summary.status === 'quiet'} class="status-badge">{summary.status}</span>{/if}</div>
  <div class="summary-controls">
    <label>View<select value={scope} onchange={(event) => changeScope(event.currentTarget.value)}><option value="overall">overall</option><option value="club" disabled={myClubs.length === 0}>club</option><option value="contest" disabled={myEvents.length === 0}>contest</option></select></label>
    {#if scope === 'club'}<label>Club<select bind:value={scopeId} onchange={load}>{#each myClubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>{/if}
    {#if scope === 'contest'}<label>Contest<select bind:value={scopeId} onchange={load}>{#each myEvents as event}<option value={event.id}>{event.name}</option>{/each}</select></label>{/if}
    <label>Period<select bind:value={periodDays} onchange={load}><option value={7}>last 7 days</option><option value={30}>last 30 days</option><option value={90}>last 90 days</option><option value={365}>last year</option><option value={0}>all time</option></select></label>
  </div>
  {#if loading}<p class="empty">Refreshing activity…</p>
  {:else if error}<p class="error">{error}</p>
  {:else if summary}
    <div class="summary-metrics"><div><b>{summary.qso_count}</b><small>QSOS</small></div><div><b>{summary.unique_callsigns}</b><small>CALLSIGNS</small></div><div><b>{summary.points}</b><small>POINTS</small></div><div><b>{summary.band_count} / {summary.mode_count}</b><small>BANDS / MODES</small></div></div>
    <p class="summary-foot">{summary.last_qso_at ? `Last activity ${new Date(summary.last_qso_at).toLocaleString()}` : 'No activity recorded for this view.'}</p>
  {/if}
</section>

<style>
  .summary-panel { padding: 25px; border: 1px solid var(--line); background: #061217cc; margin-bottom: 34px; }
  .summary-controls { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin: 18px 0; }
  .status-badge { padding: 7px 12px; border: 1px solid var(--green); color: var(--green); font: 700 10px ui-monospace; text-transform: uppercase; }
  .status-badge.quiet { border-color: var(--muted); color: var(--muted); }
  .summary-metrics { display: grid; grid-template-columns: repeat(4, 1fr); border: 1px solid var(--line); }
  .summary-metrics div { display: grid; gap: 7px; padding: 18px; border-right: 1px solid var(--line); }
  .summary-metrics b { color: var(--cyan); font: 28px ui-monospace; }
  .summary-metrics small { color: var(--muted); font: 600 10px ui-monospace; }
  .summary-foot { color: var(--muted); font-size: 12px; }
  @media (max-width: 700px) { .summary-controls, .summary-metrics { grid-template-columns: 1fr 1fr; } }
</style>
