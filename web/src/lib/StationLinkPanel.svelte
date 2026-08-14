<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { DeviceToken, DeviceTokenRecord, User } from '$lib/types';

  let { currentUser }: { currentUser: User | null } = $props();
  let deviceName = $state('My QSONaut station');
  let issued = $state<DeviceToken | null>(null);
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  let devices = $state<DeviceTokenRecord[]>([]);

  async function loadDevices() {
    devices = await api<DeviceTokenRecord[]>('/api/v1/auth/devices');
  }

  async function createToken() {
    busy = true;
    error = '';
    notice = '';
    try {
      issued = await api<DeviceToken>('/api/v1/auth/device/session', {
        method: 'POST',
        body: JSON.stringify({ device_name: deviceName }),
      });
      await loadDevices();
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      busy = false;
    }
  }

  async function revokeToken(id: string) {
    if (!confirm('Revoke this station token? QSONaut will lose access on its next connection.')) return;
    await api(`/api/v1/auth/devices/${id}`, { method: 'DELETE' });
    issued = null;
    notice = 'Token revoked.';
    await loadDevices();
  }

  async function reissueToken(id: string) {
    if (!confirm('Replace this token? Paste the replacement into QSONaut before its next connection.')) return;
    issued = await api<DeviceToken>(`/api/v1/auth/devices/${id}/reissue`, { method: 'POST' });
    notice = 'Replacement token issued. Copy it now and update QSONaut.';
    await loadDevices();
  }

  onMount(() => loadDevices().catch((cause) => error = (cause as Error).message));

  async function copyToken() {
    if (!issued) return;
    try {
      await navigator.clipboard.writeText(issued.token);
      notice = 'Token copied to clipboard.';
    } catch {
      error = 'Clipboard access was blocked; select and copy the token manually.';
    }
  }
</script>

<div class="grid">
  <section>
    <p class="eyebrow amber">QSONAUT / STATION ASSOCIATION</p>
    <h2>Link this operator to QSONaut</h2>
    <p class="section-intro">
      Create a device token for {currentUser?.callsign}. Each QSONaut installation should have its
      own named token.
    </p>
    <form onsubmit={(event) => { event.preventDefault(); createToken(); }}>
      <label>
        Station name
        <input maxlength="100" bind:value={deviceName} placeholder="Shack PC, Field laptop…" required />
        <small>This name identifies the installation when managing access later.</small>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <button disabled={busy}>{busy ? 'CREATING…' : 'CREATE STATION TOKEN'}</button>
    </form>
  </section>

  <section class="detail-panel">
    {#if issued}
      <p class="eyebrow cyan">TOKEN ISSUED / SHOWN ONCE</p>
      <h3>{deviceName}</h3>
      <p class="section-intro">
        Copy this now. The server stores only its hash and cannot display this token again.
      </p>
      <textarea class="token-value mono" readonly value={issued.token}></textarea>
      <div class="actions"><button onclick={copyToken}>COPY TOKEN</button></div>
      {#if notice}<p class="notice">{notice}</p>{/if}
      <div class="boundary">
        <b>In QSONaut desktop</b>
        <p>Open SERVER, enter this server's address (for example http://localhost:8080 locally or its hosted HTTPS address), paste the token, enable the connection, then choose Save &amp; reconnect.</p>
      </div>
      <p class="form-help">Expires {new Date(issued.expires_at).toLocaleString()}.</p>
    {:else}
      <div class="empty-state">
        <span>⌁</span>
        <h3>No token created in this session</h3>
        <p>A newly issued token will appear here exactly once.</p>
      </div>
    {/if}
  </section>
</div>

<section class="subsection">
  <div class="section-head"><div><p class="eyebrow cyan">ACTIVE ACCESS</p><h2>Issued station tokens</h2></div><button onclick={loadDevices}>REFRESH</button></div>
  {#if devices.length === 0}<p class="empty">No active device tokens.</p>{/if}
  {#each devices as device}
    <article class="record">
      <div><strong>{device.device_name}</strong><p>Created {new Date(device.created_at).toLocaleString()} · expires {new Date(device.expires_at).toLocaleString()}</p><small>Last used {device.last_used_at ? new Date(device.last_used_at).toLocaleString() : 'never'}</small></div>
      <span class="pill">{new Date(device.expires_at) > new Date() ? 'active' : 'expired'}</span>
      <div class="actions"><button onclick={() => reissueToken(device.id)}>REISSUE</button><button class="danger" onclick={() => revokeToken(device.id)}>REVOKE</button></div>
    </article>
  {/each}
</section>
