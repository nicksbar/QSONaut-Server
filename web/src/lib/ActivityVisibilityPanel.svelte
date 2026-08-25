<script lang="ts">
  import { api } from './api';
  import type { ActivityVisibility, Club, Event } from './types';

  let { clubs, events }: { clubs: Club[]; events: Event[] } = $props();
  let policies = $state<ActivityVisibility[]>([]);
  let notice = $state('');
  let busy = $state(false);
  let myClubs = $derived(clubs.filter((club) => club.my_role));
  let myEvents = $derived(events.filter((event) => myClubs.some((club) => club.id === event.club_id)));

  function policy(scope: ActivityVisibility['scope'], scopeId: string | null) {
    return policies.find((item) => item.scope === scope && item.scope_id === scopeId)?.visibility || 'private';
  }

  async function load() {
    policies = await api<ActivityVisibility[]>('/api/v1/activity/visibility');
  }

  async function save(scope: ActivityVisibility['scope'], scopeId: string | null, visibility: string) {
    busy = true; notice = '';
    try {
      await api<ActivityVisibility>('/api/v1/activity/visibility', { method: 'PUT', body: JSON.stringify({ scope, scope_id: scopeId, visibility }) });
      await load();
      notice = 'Sharing policy saved.';
    } catch (cause) { notice = (cause as Error).message; } finally { busy = false; }
  }

  $effect(() => { void load().catch((cause) => { notice = (cause as Error).message; }); });
</script>

<section class="policy-panel">
  <div class="section-head"><div><p class="eyebrow amber">ACTIVITY / AUDIENCE CONTROL</p><h2>Who can see my activity?</h2></div><small>QSO records remain operator-owned</small></div>
  <p class="section-intro">Set the audience for your status and activity detail independently for your overall station, each club, and each contest. Private details such as address and license records are never included.</p>
  <div class="policy-grid">
    <label>Overall<select value={policy('overall', null)} disabled={busy} onchange={(event) => void save('overall', null, event.currentTarget.value)}><option value="private">Only me</option><option value="members">Relevant members</option><option value="global">Anyone with access</option></select></label>
    {#each myClubs as club}
      <label>{club.name}<select value={policy('club', club.id)} disabled={busy} onchange={(event) => void save('club', club.id, event.currentTarget.value)}><option value="private">Only me</option><option value="members">Club members</option><option value="global">Anyone with access</option></select></label>
    {/each}
    {#each myEvents as contest}
      <label>{contest.name}<select value={policy('contest', contest.id)} disabled={busy} onchange={(change) => void save('contest', contest.id, change.currentTarget.value)}><option value="private">Only me</option><option value="members">Club members</option><option value="global">Anyone with access</option></select></label>
    {/each}
  </div>
  {#if notice}<p class="notice">{notice}</p>{/if}
</section>

<style>
  .policy-panel { padding: 25px; border: 1px solid var(--line); background: #061217cc; margin-bottom: 34px; }
  .policy-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .policy-grid label { display: grid; gap: 7px; padding: 14px; border: 1px solid var(--line); background: #07161c; color: var(--muted); font: 600 10px ui-monospace; text-transform: uppercase; letter-spacing: .06em; }
  @media (max-width: 700px) { .policy-grid { grid-template-columns: 1fr; } }
</style>
