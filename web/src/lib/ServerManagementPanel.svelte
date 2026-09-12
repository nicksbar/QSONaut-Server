<script lang="ts">
  import { api } from './api';
  import type { DeploymentReadiness, HostedChangeRecord, PaymentFlowStatus, PaymentOperationsSummary, ServerBillingSettings } from './types';

  let settings = $state<ServerBillingSettings | null>(null);
  let feePercent = $state('0');
  let currency = $state('USD');
  let donationsEnabled = $state(false);
  let clubDuesEnabled = $state(false);
  let loading = $state(true);
  let saving = $state(false);
  let notice = $state('');
  let error = $state('');
  let readiness = $state<DeploymentReadiness | null>(null);
  let ledger = $state<HostedChangeRecord[]>([]);
  let operationsLoading = $state(true);
  let paymentStatus = $state<PaymentFlowStatus | null>(null);
  let paymentOperations = $state<PaymentOperationsSummary | null>(null);

  async function load() {
    loading = true; error = '';
    try {
      settings = await api<ServerBillingSettings>('/api/v1/admin/server-billing');
      feePercent = (settings.platform_fee_basis_points / 100).toFixed(2);
      currency = settings.currency;
      donationsEnabled = settings.donations_enabled;
      clubDuesEnabled = settings.club_dues_enabled;
    } catch (cause) { error = (cause as Error).message; }
    finally { loading = false; }
  }

  async function save() {
    saving = true; error = ''; notice = '';
    const percent = Number(feePercent);
    if (!Number.isFinite(percent) || percent < 0 || percent > 100) { error = 'Platform fee must be between 0% and 100%.'; saving = false; return; }
    try {
      settings = await api<ServerBillingSettings>('/api/v1/admin/server-billing', {
        method: 'PUT',
        body: JSON.stringify({ platform_fee_basis_points: Math.round(percent * 100), donations_enabled: donationsEnabled, club_dues_enabled: clubDuesEnabled, currency }),
      });
      notice = 'Server commercial policy saved. No payments are collected until a provider is connected.';
    } catch (cause) { error = (cause as Error).message; }
    finally { saving = false; }
  }

  async function loadOperations() {
    operationsLoading = true;
    try {
      [readiness, ledger, paymentStatus, paymentOperations] = await Promise.all([
        api<DeploymentReadiness>('/api/v1/admin/deployment-readiness'),
        api<HostedChangeRecord[]>('/api/v1/admin/change-ledger'),
        api<PaymentFlowStatus>('/api/v1/payments/status'),
        api<PaymentOperationsSummary>('/api/v1/admin/payments/operations'),
      ]);
    } catch (cause) { error = (cause as Error).message; }
    finally { operationsLoading = false; }
  }

  $effect(() => { void load(); void loadOperations(); });
</script>

<section class="server-management">
  <div class="section-head"><div><p class="eyebrow">SERVER MANAGEMENT / COMMERCIAL POLICY</p><h1>Platform and organization payments</h1><p class="section-intro">Set the rules that a future payment provider will enforce. This workspace does not charge cards, create checkout sessions, or move club funds.</p></div><button class="secondary" onclick={() => void load()} disabled={loading}>REFRESH</button></div>
  {#if loading}<p class="empty">Loading server policy…</p>
  {:else}
    <div class="status-card"><div><span class="status-dot"></span><b>PAYMENT COLLECTION {settings?.provider_status === 'live' ? 'CONNECTED' : 'NOT CONNECTED'}</b><p>{settings?.provider_status === 'live' ? 'Provider configuration is live.' : 'Policy can be prepared safely, but no money can be collected yet.'}</p></div><span class="pill">{settings?.provider_status || 'not_configured'}</span></div>
    <form class="policy-form" onsubmit={(event) => { event.preventDefault(); void save(); }}>
      <div class="form-section"><div><p class="eyebrow">PLATFORM REVENUE</p><h2>Server fee</h2><p class="form-help">The percentage retained by the QSONaut service from supported transactions. Use 0% for a donation-only or pass-through model.</p></div><label>Platform fee (%)<input type="number" min="0" max="100" step="0.01" bind:value={feePercent} /></label></div>
      <div class="form-section"><div><p class="eyebrow">SUPPORT THE SERVICE</p><h2>Donations</h2><p class="form-help">Enables a future donation destination for the server. It does not expose a payment button until a provider is configured.</p></div><label class="check"><input type="checkbox" bind:checked={donationsEnabled} /> Allow server donations</label></div>
      <div class="form-section"><div><p class="eyebrow">ORGANIZATION FUNDS</p><h2>Club dues</h2><p class="form-help">Allows clubs to define dues and receive their own payments. Platform fees remain separate from club funds.</p></div><label class="check"><input type="checkbox" bind:checked={clubDuesEnabled} /> Enable club dues capability</label></div>
      <div class="form-section"><div><p class="eyebrow">SETTLEMENT DEFAULT</p><h2>Currency</h2><p class="form-help">Three-letter ISO currency used for future server and club payment records.</p></div><label>Currency code<input maxlength="3" bind:value={currency} /></label></div>
      {#if error}<p class="error">{error}</p>{/if}{#if notice}<p class="notice">{notice}</p>{/if}
      <div class="actions"><button disabled={saving}>{saving ? 'SAVING…' : 'SAVE COMMERCIAL POLICY'}</button></div>
    </form>
    <div class="boundary-note"><b>Safety boundary</b><p>These controls describe policy only. Provider credentials, webhooks, reconciliation, refunds, tax receipts, and club payout destinations must be implemented and audited before enabling live collection.</p></div>
    <section class="operations-section">
      <div class="section-head"><div><p class="eyebrow">DEPLOYMENT / READINESS</p><h2>Hosted service posture</h2><p class="section-intro">Read-only checks for proxy and provider preparation. Values are status signals, not credentials.</p></div><button class="secondary" onclick={() => void loadOperations()} disabled={operationsLoading}>REFRESH STATUS</button></div>
      {#if readiness}<div class="readiness-grid"><span class:good={readiness.secure_cookies}><b>Cookies</b><small>{readiness.secure_cookies ? 'secure' : 'insecure HTTP mode'}</small></span><span class="good"><b>Database</b><small>{readiness.database}</small></span><span class="good"><b>Migrations</b><small>{readiness.private_migrations}</small></span><span class:good={readiness.online_map_tiles}><b>Online map</b><small>{readiness.online_map_tiles ? 'enabled' : 'offline basemap'}</small></span><span><b>WebSocket</b><small>{readiness.websocket_path}</small></span><span><b>Bind</b><small>{readiness.bind_address}</small></span><span><b>Payments</b><small>{readiness.payment_provider}</small></span><span><b>Email / OAuth</b><small>{readiness.email_provider} / {readiness.oauth_provider}</small></span><span class="good"><b>Abuse protection</b><small>{readiness.abuse_protection}</small></span><span class:good={readiness.trusted_proxy_mode}><b>Client address</b><small>{readiness.trusted_proxy_mode ? 'trusted proxy configured' : 'socket peer only'}</small></span></div>{/if}
    </section>
    <section class="operations-section"><div><p class="eyebrow">PAYMENT FLOWS / BLOCKERS</p><h2>Money movement gates</h2><p class="section-intro">The flows are installed, but checkout is deliberately closed until the provider, accounting, webhook, refund, tax, and payout boundaries are approved.</p></div>{#if paymentOperations}<div class="payment-metrics"><span><b>{paymentOperations.blocked_intents}</b><small>blocked intents</small></span><span><b>{paymentOperations.pending_intents}</b><small>pending intents</small></span><span><b>{paymentOperations.succeeded_intents}</b><small>succeeded intents</small></span><span><b>{paymentOperations.unprocessed_provider_events}</b><small>provider events needing action</small></span><span><b>{paymentOperations.ready_club_destinations}</b><small>ready club destinations</small></span><span><b>{paymentOperations.active_entitlements}</b><small>active entitlements</small></span></div>{/if}{#if paymentStatus}<div class="payment-gates">{#each paymentStatus.gates as gate}<span class:included={gate.status === 'included'}><b>{gate.label}</b><small>{gate.status}</small></span>{/each}</div><div class="blockers">{#each paymentStatus.blockers as blocker}<div><b>{blocker.code.replaceAll('_', ' ')}</b><p>{blocker.message}</p></div>{/each}</div><div class="offer-list"><b>Installed flow offers</b>{#each paymentStatus.offers as offer}<span><strong>{offer.label}</strong><small>{offer.ownership} · {offer.purpose}</small></span>{/each}</div>{/if}</section>
    <section class="operations-section"><div><p class="eyebrow">ADMINISTRATION / CHANGE LEDGER</p><h2>Recent hosted changes</h2><p class="section-intro">Append-only records of hosted policy and governance changes. Sensitive provider payloads are excluded.</p></div>{#if ledger.length}<div class="ledger">{#each ledger as entry}<div class="ledger-row"><b>{entry.action}</b><span>{entry.resource_type}{entry.resource_id ? ` · ${entry.resource_id}` : ''}</span><time>{new Date(entry.created_at).toLocaleString()}</time></div>{/each}</div>{:else}<p class="empty">No hosted changes recorded yet.</p>{/if}</section>
  {/if}
</section>

<style>
  .server-management { display:grid; gap:18px; }
  .section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }
  .section-head h1 { margin:4px 0; }.section-intro { max-width:760px; color:var(--muted); line-height:1.55; }
  .status-card, .form-section, .boundary-note { border:1px solid var(--line); background:var(--panel); padding:18px; border-radius:10px; }
  .status-card { display:flex; justify-content:space-between; align-items:center; gap:14px; }.status-card p, .boundary-note p { margin:6px 0 0; color:var(--muted); }.status-dot { display:inline-block; width:9px; height:9px; margin-right:8px; border-radius:50%; background:var(--amber); }.pill { padding:4px 9px; border-radius:999px; background:var(--chip); color:var(--muted); font:600 10px ui-monospace; text-transform:uppercase; }
  .policy-form { display:grid; gap:12px; }.form-section { display:grid; grid-template-columns:minmax(0,1fr) minmax(180px,280px); gap:20px; align-items:center; }.form-section h2 { margin:4px 0; }.form-help { color:var(--muted); }.form-section label { display:grid; gap:6px; }.form-section .check { display:flex; align-items:center; gap:8px; }.form-section .check input { width:auto; }
  .actions { display:flex; gap:8px; }.error, .notice { padding:10px 12px; border-radius:8px; }.error { color:var(--red); background:color-mix(in srgb,var(--red) 12%,transparent); }.notice { color:var(--green); background:color-mix(in srgb,var(--green) 12%,transparent); }.empty { color:var(--muted); }.boundary-note { border-color:color-mix(in srgb,var(--amber) 40%,var(--line)); background:color-mix(in srgb,var(--amber) 8%,var(--panel)); }.boundary-note b { color:var(--amber); }
  .operations-section { display:grid; gap:12px; margin-top:8px; }.operations-section h2 { margin:4px 0; }.readiness-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:10px; }.readiness-grid span { display:grid; gap:5px; padding:13px; border:1px solid var(--line); background:var(--panel); border-radius:9px; }.readiness-grid small { color:var(--muted); }.readiness-grid .good b { color:var(--green); }.ledger { border:1px solid var(--line); background:var(--panel); border-radius:9px; overflow:hidden; }.ledger-row { display:grid; grid-template-columns:1.2fr 1fr 1fr; gap:12px; padding:11px 13px; border-bottom:1px solid var(--line); font-size:12px; }.ledger-row:last-child { border-bottom:0; }.ledger-row span,.ledger-row time { color:var(--muted); }
  .payment-gates,.offer-list { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:10px; }.payment-gates span,.offer-list span,.blockers div { display:grid; gap:5px; padding:13px; border:1px solid var(--line); background:var(--panel); border-radius:9px; }.payment-gates small,.offer-list small,.blockers p { color:var(--muted); margin:0; }.payment-gates .included b { color:var(--green); }.payment-gates span:not(.included) b,.blockers b { color:var(--amber); }.blockers { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:10px; }.offer-list { grid-template-columns:repeat(2,minmax(0,1fr)); }.offer-list > b { grid-column:1/-1; }
  .payment-metrics { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:10px; }.payment-metrics span { display:grid; gap:4px; padding:13px; border:1px solid var(--line); background:var(--panel); border-radius:9px; }.payment-metrics b { color:var(--cyan); font:24px ui-monospace; }.payment-metrics small { color:var(--muted); }
  @media (max-width:700px) { .section-head, .status-card { align-items:start; flex-direction:column; }.form-section, .readiness-grid, .payment-metrics, .payment-gates, .blockers, .offer-list { grid-template-columns:1fr; }.ledger-row { grid-template-columns:1fr; gap:4px; } }
</style>
