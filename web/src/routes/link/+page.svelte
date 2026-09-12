<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { api, ApiError } from '$lib/api';
  import type { DeviceAuthorizationApproval, User } from '$lib/types';

  type Phase = 'loading' | 'login' | 'ready' | 'complete' | 'error';

  let phase = $state<Phase>('loading');
  let currentUser = $state<User | null>(null);
  let approval = $state<DeviceAuthorizationApproval | null>(null);
  let callsign = $state('');
  let password = $state('');
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  let userCode = $derived((page.url.searchParams.get('user_code') ?? '').trim().toUpperCase());

  async function loadApproval() {
    if (!userCode) {
      phase = 'error';
      error = 'This link is missing its authorization code.';
      return;
    }
    try {
      currentUser = await api<User>('/api/v1/auth/me');
      approval = await api<DeviceAuthorizationApproval>(
        `/api/v1/auth/device/approval?user_code=${encodeURIComponent(userCode)}`,
      );
      phase = 'ready';
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 401) {
        phase = 'login';
        return;
      }
      phase = 'error';
      error = (cause as Error).message;
    }
  }

  async function login() {
    busy = true;
    error = '';
    try {
      currentUser = await api<User>('/api/v1/auth/login', {
        method: 'POST',
        body: JSON.stringify({ callsign, password }),
      });
      password = '';
      approval = await api<DeviceAuthorizationApproval>(
        `/api/v1/auth/device/approval?user_code=${encodeURIComponent(userCode)}`,
      );
      phase = 'ready';
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      busy = false;
    }
  }

  async function decide(approved: boolean) {
    busy = true;
    error = '';
    notice = '';
    try {
      await api('/api/v1/auth/device/approval', {
        method: 'POST',
        body: JSON.stringify({ user_code: userCode, approved }),
      });
      phase = 'complete';
      notice = approved
        ? 'QSONaut is approved. Return to the desktop application to finish connecting.'
        : 'The QSONaut linking request was denied.';
    } catch (cause) {
      error = (cause as Error).message;
    } finally {
      busy = false;
    }
  }

  onMount(loadApproval);
</script>

<svelte:head>
  <title>Link QSONaut</title>
</svelte:head>

<main class="shell">
  <div class="brand">QSONAUT <span>/ SECURE DEVICE LINK</span></div>
  <section class="card">
    {#if phase === 'loading'}
      <p class="eyebrow">CHECKING REQUEST</p>
      <h1>Preparing secure link…</h1>
      <p class="muted">This page will show the QSONaut installation requesting access.</p>
    {:else if phase === 'login'}
      <p class="eyebrow">SIGN IN REQUIRED</p>
      <h1>Link QSONaut to your account</h1>
      <p class="muted">Sign in to approve this device. Your password is sent only to this QSONaut server.</p>
      <form onsubmit={(event) => { event.preventDefault(); login(); }}>
        <label>Callsign<input bind:value={callsign} autocomplete="username" required /></label>
        <label>Password<input type="password" bind:value={password} autocomplete="current-password" required /></label>
        {#if error}<p class="error">{error}</p>{/if}
        <button disabled={busy}>{busy ? 'SIGNING IN…' : 'SIGN IN AND REVIEW'}</button>
      </form>
    {:else if phase === 'ready' && approval}
      <p class="eyebrow">AUTHORIZE INSTALLATION</p>
      <h1>Link QSONaut?</h1>
      <p class="muted">The following installation is asking for access to your QSONaut account.</p>
      <dl>
        <div><dt>Account</dt><dd>{currentUser?.callsign}</dd></div>
        <div><dt>Station</dt><dd>{approval.device_name}</dd></div>
        <div><dt>Client</dt><dd>{approval.client_id} · {approval.client_version}</dd></div>
        <div><dt>Request expires</dt><dd>{new Date(approval.expires_at).toLocaleString()}</dd></div>
      </dl>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button class="danger" disabled={busy} onclick={() => decide(false)}>DENY</button>
        <button disabled={busy} onclick={() => decide(true)}>{busy ? 'SAVING…' : 'APPROVE QSONAUT'}</button>
      </div>
    {:else if phase === 'complete'}
      <p class="eyebrow">REQUEST COMPLETE</p>
      <h1>{notice.includes('approved') ? 'QSONaut is linked' : 'Link denied'}</h1>
      <p class="muted">{notice}</p>
      <p class="small">You can close this window.</p>
    {:else}
      <p class="eyebrow">LINK UNAVAILABLE</p>
      <h1>We couldn’t load this request</h1>
      <p class="error">{error}</p>
      <p class="muted">Start linking again from QSONaut to create a fresh request.</p>
    {/if}
  </section>
</main>

<style>
  :global(body) { margin: 0; background: #081116; color: #eef7fb; font-family: Georgia, 'Times New Roman', serif; }
  .shell { min-height: 100vh; display: grid; align-content: center; justify-items: center; gap: 28px; padding: 32px 20px; box-sizing: border-box; }
  .brand { color: #ffb15c; letter-spacing: .16em; font-weight: 700; }
  .brand span { color: #77e7ff; font-weight: 400; }
  .card { width: min(100%, 620px); box-sizing: border-box; border: 1px solid #29414b; background: #0d1b21; padding: clamp(24px, 5vw, 52px); box-shadow: 0 22px 70px #0008; }
  .eyebrow { color: #ffb15c; letter-spacing: .13em; font-size: .82rem; margin: 0 0 12px; }
  h1 { font-size: clamp(2rem, 5vw, 3.6rem); line-height: 1; margin: 0 0 18px; font-weight: 500; }
  .muted, .small { color: #a4bac4; line-height: 1.55; }
  .small { font-size: .9rem; }
  form { display: grid; gap: 18px; margin-top: 28px; }
  label { display: grid; gap: 8px; color: #dcebf0; }
  input { border: 1px solid #38535f; background: #081116; color: #eef7fb; padding: 13px; font: inherit; font-size: 1rem; }
  button { border: 1px solid #77e7ff; background: #77e7ff; color: #071116; padding: 13px 17px; font: inherit; font-weight: 700; cursor: pointer; }
  button:disabled { opacity: .55; cursor: wait; }
  button.danger { border-color: #d87979; background: transparent; color: #ffaaa4; }
  .actions { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; }
  .error { color: #ffaaa4; line-height: 1.45; }
  dl { border-top: 1px solid #29414b; border-bottom: 1px solid #29414b; margin: 28px 0 0; }
  dl div { display: grid; grid-template-columns: minmax(120px, .7fr) 1.3fr; gap: 16px; padding: 14px 0; border-bottom: 1px solid #1d323a; }
  dl div:last-child { border-bottom: 0; }
  dt { color: #82a5b0; } dd { margin: 0; overflow-wrap: anywhere; }
</style>
