<script lang="ts">
  import { api } from './api';
  import type { Club } from './types';

  let { clubs, refresh }: { clubs: Club[]; refresh: () => Promise<void> } = $props();
  let name = $state('');
  let callsign = $state('');
  let description = $state('');
  let working = $state(false);
  let error = $state('');

  async function createClub() {
    working = true;
    error = '';
    try {
      await api<Club>('/api/v1/clubs', {
        method: 'POST',
        body: JSON.stringify({ name, callsign: callsign || null, description }),
      });
      name = callsign = description = '';
      await refresh();
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      working = false;
    }
  }
</script>

<section class="grid">
  <div>
    <p class="eyebrow">ORGANIZATIONS</p>
    <h2>Clubs</h2>
    {#if clubs.length === 0}<p class="empty">No clubs yet. Register the first one.</p>{/if}
    {#each clubs as club}
      <article class="record">
        <b>{club.name}</b><span>{club.callsign || 'NO CLUB CALL'}</span>
        <p>{club.description || 'No description'}</p>
      </article>
    {/each}
  </div>
  <form onsubmit={(event) => { event.preventDefault(); createClub(); }}>
    <h3>Register club</h3>
    <label>Name<input maxlength="120" bind:value={name} required /></label>
    <label>Club callsign<input maxlength="16" bind:value={callsign} /></label>
    <label>Description<textarea maxlength="1000" bind:value={description}></textarea></label>
    {#if error}<p class="error">{error}</p>{/if}
    <button disabled={working}>{working ? 'CREATING…' : 'CREATE CLUB'}</button>
  </form>
</section>
