<script lang="ts">
  import { onMount } from 'svelte';
  import { formatFrequency } from '$lib/api';
  import type { SharedQsoDetail } from '$lib/types';
  import { page } from '$app/state';

  let log = $state<SharedQsoDetail | null>(null);
  let error = $state('');
  let loading = $state(true);

  onMount(async () => {
    try {
      const response = await fetch(`/api/v1/share/${encodeURIComponent(page.params.token ?? '')}`);
      if (!response.ok) throw new Error('This share link is expired, revoked, or invalid.');
      log = await response.json() as SharedQsoDetail;
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>{log ? `${log.operator_callsign} · ${log.callsign} · QSONaut` : 'Shared QSONaut detail'}</title>
  <meta name="description" content="Shared QSONaut contact detail" />
</svelte:head>

<main class="share-shell">
  {#if loading}
    <p class="eyebrow cyan">QSONAUT / SECURE SHARE</p><h1>Loading shared detail…</h1>
  {:else if error}
    <p class="eyebrow amber">QSONAUT / SHARE UNAVAILABLE</p><h1>Link unavailable</h1><p class="muted">{error}</p>
  {:else if log}
    <p class="eyebrow cyan">QSONAUT / SHARED CONTACT</p>
    <h1>{log.operator_callsign} <span>worked</span> {log.callsign}</h1>
    <p class="muted">{new Date(log.occurred_at).toISOString().replace('T', ' ').slice(0, 19)} UTC</p>
    <section class="detail-card">
      <div><small>MODE</small><strong>{log.mode}</strong></div>
      <div><small>BAND</small><strong>{log.band}</strong></div>
      <div><small>FREQUENCY</small><strong>{formatFrequency(log.frequency_hz)}</strong></div>
      <div><small>EVENT</small><strong>{log.event_name || 'General operation'}</strong></div>
      <div><small>REPORTS</small><strong>{log.rst_sent || '—'} / {log.rst_received || '—'}</strong></div>
      <div><small>SOURCE</small><strong>{log.source}</strong></div>
    </section>
    <section class="exchange"><h2>Exchange</h2><pre>{JSON.stringify(log.exchange, null, 2)}</pre></section>
    <footer>Shared from QSONaut Server · link access is time-limited and revocable</footer>
  {/if}
</main>

<style>
  .share-shell { width: min(760px, calc(100% - 32px)); margin: 0 auto; padding: 12vh 0 10vh; }
  h1 { font-size: clamp(34px, 7vw, 68px); line-height: 1.05; margin: 18px 0 12px; }
  h1 span { color: var(--muted); font-weight: 300; }
  .muted { color: var(--muted); }
  .detail-card { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1px; margin-top: 42px; border: 1px solid var(--line); background: var(--line); }
  .detail-card div { display: grid; gap: 8px; padding: 22px; background: var(--panel); }
  small { color: var(--muted); font: 600 10px ui-monospace; letter-spacing: .12em; }
  strong { color: var(--cyan); font-size: 18px; }
  .exchange { margin-top: 32px; border-top: 1px solid var(--line); padding-top: 22px; }
  pre { padding: 18px; overflow: auto; background: #040b0e; color: #b8d6dc; }
  footer { margin-top: 42px; padding-top: 18px; border-top: 1px solid var(--line); color: var(--muted); font-size: 12px; }
  @media (max-width: 600px) { .detail-card { grid-template-columns: repeat(2, 1fr); } }
</style>
