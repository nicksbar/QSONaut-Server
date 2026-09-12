<script lang="ts">
  import { api } from './api';
  import type { ClubPaymentMethod } from './types';

  let { clubId }: { clubId: string } = $props();
  let methods = $state<ClubPaymentMethod[]>([]);
  let loading = $state(false);
  let working = $state(false);
  let error = $state('');
  let notice = $state('');
  let provider = $state<'paypal' | 'amazon_pay' | 'zelle' | 'external_link' | 'offline' | 'other'>('paypal');
  let methodKind = $state<'automated' | 'external_link' | 'manual'>('external_link');
  let displayLabel = $state('PayPal');
  let publicReference = $state('');
  let paymentUrl = $state('');
  let instructions = $state('');
  let isDefault = $state(false);

  async function load() {
    if (!clubId) return;
    loading = true; error = '';
    try { methods = await api<ClubPaymentMethod[]>(`/api/v1/clubs/${clubId}/payment-methods`); }
    catch (cause) { error = (cause as Error).message; }
    finally { loading = false; }
  }

  function selectPreset() {
    if (provider === 'zelle') { methodKind = 'manual'; displayLabel = 'Zelle'; paymentUrl = ''; }
    else if (provider === 'amazon_pay') { methodKind = 'automated'; displayLabel = 'Amazon Pay'; }
    else if (provider === 'paypal') { methodKind = 'external_link'; displayLabel = 'PayPal'; }
    else if (provider === 'external_link') { methodKind = 'external_link'; displayLabel = 'Online payment link'; }
    else { methodKind = 'manual'; displayLabel = provider === 'offline' ? 'Pay by check or cash' : 'Other payment method'; paymentUrl = ''; }
  }

  async function createMethod() {
    working = true; error = ''; notice = '';
    try {
      await api<ClubPaymentMethod>(`/api/v1/clubs/${clubId}/payment-methods`, { method: 'POST', body: JSON.stringify({ provider, method_kind: methodKind, display_label: displayLabel, public_reference: publicReference || null, payment_url: paymentUrl || null, instructions, is_default: isDefault }) });
      publicReference = ''; paymentUrl = ''; instructions = ''; isDefault = false;
      notice = methodKind === 'automated' ? 'Method saved pending adapter verification.' : 'Payment method published.';
      await load();
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  async function updateMethod(method: ClubPaymentMethod, status = method.status, makeDefault = method.is_default) {
    working = true; error = ''; notice = '';
    try {
      await api<ClubPaymentMethod>(`/api/v1/clubs/${clubId}/payment-methods/${method.id}`, { method: 'PATCH', body: JSON.stringify({ display_label: method.display_label, public_reference: method.public_reference, payment_url: method.payment_url, instructions: method.instructions, status, is_default: makeDefault }) });
      notice = makeDefault ? 'Default payment method updated.' : `Payment method ${status}.`;
      await load();
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  $effect(() => { if (clubId) void load(); });
</script>

<section class="club-payments">
  <div class="section-head"><div><p class="eyebrow">CLUB FUNDS / PAYMENT METHODS</p><h2>Ways members can pay the club</h2><p class="help">Configure several club-owned destinations. QSONaut does not receive or redistribute these funds.</p></div><button class="secondary" onclick={() => void load()} disabled={loading}>REFRESH</button></div>
  {#if error}<p class="error">{error}</p>{/if}{#if notice}<p class="notice">{notice}</p>{/if}
  <div class="payment-layout">
    <form onsubmit={(event) => { event.preventDefault(); void createMethod(); }}>
      <label>Provider<select bind:value={provider} onchange={selectPreset}><option value="paypal">PayPal</option><option value="amazon_pay">Amazon Pay</option><option value="zelle">Zelle</option><option value="external_link">Other online link</option><option value="offline">Check or cash</option><option value="other">Other manual method</option></select></label>
      <label>Processing<select bind:value={methodKind} disabled={provider === 'zelle'}><option value="automated">Automated adapter</option><option value="external_link">External payment link</option><option value="manual">Manual instructions</option></select></label>
      <label>Member-facing label<input maxlength="120" bind:value={displayLabel} required /></label>
      <label>Public account/reference<input maxlength="200" bind:value={publicReference} placeholder="Optional public handle or memo guidance" /></label>
      {#if methodKind === 'external_link'}<label>HTTPS payment URL<input type="url" pattern="https://.*" bind:value={paymentUrl} required /></label>{/if}
      <label>Instructions<textarea maxlength="2000" bind:value={instructions} required={methodKind === 'manual'} placeholder="Instructions shown to club members"></textarea></label>
      <label class="check"><input type="checkbox" bind:checked={isDefault} /> Make this the default</label>
      <button disabled={working}>ADD PAYMENT METHOD</button>
      {#if methodKind === 'automated'}<p class="adapter-note">Automated methods remain pending until the matching provider adapter and webhook verification are installed.</p>{/if}
    </form>
    <div class="method-list">
      {#if loading}<p class="empty">Loading payment methods…</p>
      {:else if methods.length === 0}<p class="empty">No club payment methods configured.</p>
      {:else}{#each methods as method}<article><div><b>{method.display_label}</b><small>{method.provider.replaceAll('_', ' ')} · {method.method_kind.replaceAll('_', ' ')}</small></div><span class:ready={method.status === 'ready'}>{method.status.replaceAll('_', ' ')}</span>{#if method.public_reference}<p>{method.public_reference}</p>{/if}{#if method.instructions}<p>{method.instructions}</p>{/if}<div class="method-actions">{#if !method.is_default && method.status !== 'disabled'}<button class="secondary" onclick={() => void updateMethod(method, method.status, true)} disabled={working}>MAKE DEFAULT</button>{/if}{#if method.status !== 'disabled'}<button class="secondary danger" onclick={() => void updateMethod(method, 'disabled', false)} disabled={working}>DISABLE</button>{:else}<button class="secondary" onclick={() => void updateMethod(method, method.method_kind === 'automated' ? 'pending_verification' : 'ready', false)} disabled={working}>ENABLE</button>{/if}</div></article>{/each}{/if}
    </div>
  </div>
  <p class="boundary"><b>Reconciliation:</b> PayPal/Amazon automation requires a private adapter. Zelle, checks, cash, and other manual methods require a club manager to record payment separately; QSONaut must not infer success from instructions alone.</p>
</section>

<style>
  .club-payments { display:grid; gap:14px; border-top:1px solid var(--line); padding-top:20px; }.section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }.section-head h2 { margin:0; }.eyebrow { color:var(--cyan); font-size:.72rem; letter-spacing:.12em; margin:0 0 4px; }.help,.empty,.adapter-note,.boundary,.method-list p,.method-list small { color:var(--muted); }.payment-layout { display:grid; grid-template-columns:minmax(280px,.8fr) minmax(320px,1.2fr); gap:14px; }.club-payments form,.method-list article { display:grid; gap:9px; border:1px solid var(--line); background:var(--panel); border-radius:10px; padding:16px; }.club-payments label { display:grid; gap:5px; }.club-payments textarea { min-height:78px; resize:vertical; }.club-payments .check { display:flex; gap:8px; align-items:center; }.club-payments .check input { width:auto; }.method-list { display:grid; gap:10px; align-content:start; }.method-list article { grid-template-columns:1fr auto; }.method-list article div:first-child { display:grid; gap:3px; }.method-list article > p,.method-actions { grid-column:1/-1; margin:0; }.method-list span { color:var(--amber); font:700 10px ui-monospace; text-transform:uppercase; }.method-list span.ready { color:var(--green); }.method-actions { display:flex; gap:8px; }.danger { color:var(--red); }.error,.notice { padding:10px 12px; border-radius:8px; }.error { color:var(--red); background:color-mix(in srgb,var(--red) 12%,transparent); }.notice { color:var(--green); background:color-mix(in srgb,var(--green) 12%,transparent); }.boundary { border-left:2px solid var(--amber); padding:10px 14px; margin:0; }@media (max-width:800px) { .payment-layout { grid-template-columns:1fr; }.section-head { align-items:start; flex-direction:column; } }
</style>
