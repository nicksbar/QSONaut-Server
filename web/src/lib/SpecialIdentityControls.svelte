<script lang="ts">
  import { api } from './api';
  import type { Event, ManagedCallsign } from './types';

  let { event, identity, administrator, canManage, changed }: {
    event: Event | undefined;
    identity: ManagedCallsign | undefined;
    administrator: boolean;
    canManage: boolean;
    changed: () => Promise<void>;
  } = $props();
  let callsign = $state('');
  let authority = $state('');
  let reference = $state('');
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');

  async function run(action: () => Promise<void>) {
    busy = true;
    error = '';
    notice = '';
    try {
      await action();
      await changed();
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      busy = false;
    }
  }

  function create() {
    if (!event) return;
    void run(async () => {
      await api<ManagedCallsign>('/api/v1/identities/special', {
        method: 'POST',
        body: JSON.stringify({ event_id: event.id, callsign, authority, authority_reference: reference }),
      });
      callsign = authority = reference = '';
      notice = 'Special identity requested.';
    });
  }

  function decide(decision: string) {
    if (!identity) return;
    void run(async () => {
      await api(`/api/v1/identities/${identity.id}/status`, {
        method: 'PATCH',
        body: JSON.stringify({ decision }),
      });
      notice = `Identity ${decision}d.`;
    });
  }
</script>

{#if canManage && event}
  <section class="identity-controls-panel">
    <div><p class="eyebrow">IDENTITY MANAGEMENT</p><h3>{identity ? identity.callsign : 'Special operating identity'}</h3></div>
    {#if identity}
      <p>{identity.status} · verification {identity.verification_status}</p>
      {#if administrator && identity.verification_status === 'pending'}
        <button onclick={() => decide('approve')} disabled={busy}>APPROVE</button>
        <button class="danger" onclick={() => decide('reject')} disabled={busy}>REJECT</button>
      {/if}
      {#if identity.status !== 'revoked'}
        <button class="secondary" onclick={() => decide('suspend')} disabled={busy || identity.status === 'suspended'}>SUSPEND</button>
        <button class="danger" onclick={() => decide('revoke')} disabled={busy}>REVOKE</button>
      {/if}
    {:else}
      <p class="form-help">Request a special callsign for this operation. The server will keep it pending until the appropriate authority verifies it.</p>
      <form onsubmit={(event) => { event.preventDefault(); create(); }}>
        <label>Callsign<input maxlength="16" bind:value={callsign} required placeholder="e.g. W1AW/7" /></label>
        <label>Authority<input maxlength="120" bind:value={authority} placeholder="Club authorization" /></label>
        <label>Reference<input maxlength="200" bind:value={reference} placeholder="License or approval reference" /></label>
        <button disabled={busy || !callsign.trim()}>REQUEST SPECIAL IDENTITY</button>
      </form>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}{#if notice}<p class="notice">{notice}</p>{/if}
  </section>
{/if}

<style>
  .identity-controls-panel { display:grid; gap:10px; margin:12px 0; padding:16px; border:1px solid var(--line); background:var(--chip); }
  .identity-controls-panel h3 { margin:3px 0; }.identity-controls-panel p { margin:0; color:var(--muted); }
  .identity-controls-panel form { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:10px; align-items:end; }
  .identity-controls-panel form button { grid-column:1 / -1; justify-self:start; }
  @media (max-width:700px) { .identity-controls-panel form { grid-template-columns:1fr; } }
</style>
