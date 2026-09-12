<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api';
  import type { Club, ClubPaymentMethod, ClubPaymentRecord, PaymentFlowStatus, PaymentIntent, PaymentStatusEvent } from './types';

  let { clubs }: { clubs: Club[] } = $props();
  let selectedClubId = $state('');
  let options = $state<ClubPaymentMethod[]>([]);
  let methodId = $state('');
  let amount = $state('');
  let history = $state<PaymentIntent[]>([]);
  let records = $state<ClubPaymentRecord[]>([]);
  let flow = $state<PaymentFlowStatus | null>(null);
  let createdMethod = $state<ClubPaymentMethod | null>(null);
  let working = $state(false);
  let loading = $state(true);
  let error = $state('');
  let notice = $state('');
  let reconcileId = $state('');
  let reconciliationReference = $state('');
  let reconciliationNote = $state('');
  let markDuesCurrent = $state(false);
  let openReceiptId = $state('');
  let timelines = $state<Record<string, PaymentStatusEvent[]>>({});

  let selectedClub = $derived(clubs.find((club) => club.id === selectedClubId));
  let selectedMethod = $derived(options.find((method) => method.id === methodId));
  let clubHistory = $derived(history.filter((intent) => intent.club_id === selectedClubId));

  function money(minor: number, currency: string) {
    return new Intl.NumberFormat(undefined, { style: 'currency', currency: currency.trim() }).format(minor / 100);
  }

  function canCancel(intent: PaymentIntent) {
    const method = options.find((option) => option.id === intent.payment_destination_id);
    return intent.status === 'pending' && Boolean(method) && method?.method_kind !== 'automated';
  }

  async function loadClub() {
    if (!selectedClubId) return;
    loading = true; error = ''; createdMethod = null;
    try {
      options = await api<ClubPaymentMethod[]>(`/api/v1/clubs/${selectedClubId}/payment-options`);
      methodId = options.some((method) => method.id === methodId) ? methodId : options.find((method) => method.is_default)?.id || options[0]?.id || '';
      records = selectedClub?.can_manage ? await api<ClubPaymentRecord[]>(`/api/v1/clubs/${selectedClubId}/payment-records`) : [];
    } catch (cause) { error = (cause as Error).message; }
    finally { loading = false; }
  }

  async function loadAll() {
    loading = true; error = '';
    try {
      [flow, history] = await Promise.all([
        api<PaymentFlowStatus>('/api/v1/payments/status'),
        api<PaymentIntent[]>('/api/v1/payments/intents'),
      ]);
      selectedClubId = clubs.find((club) => club.my_membership_status === 'active')?.id || '';
      await loadClub();
    } catch (cause) { error = (cause as Error).message; loading = false; }
  }

  async function createPayment() {
    const amountMinor = Math.round(Number(amount) * 100);
    if (!Number.isFinite(amountMinor) || amountMinor <= 0 || !methodId || !flow) return;
    working = true; error = ''; notice = ''; createdMethod = null;
    try {
      const intent = await api<PaymentIntent>('/api/v1/payments/intents', { method: 'POST', body: JSON.stringify({ purpose: 'club_contribution', offer_key: 'club_contribution', club_id: selectedClubId, payment_method_id: methodId, amount_minor: amountMinor, currency: flow.currency, idempotency_key: crypto.randomUUID(), return_url: null, cancel_url: null }) });
      createdMethod = options.find((method) => method.id === intent.payment_destination_id) || null;
      notice = 'Payment record created. Follow the club method below; it remains pending until the club confirms receipt.';
      amount = '';
      history = await api<PaymentIntent[]>('/api/v1/payments/intents');
      if (selectedClub?.can_manage) records = await api<ClubPaymentRecord[]>(`/api/v1/clubs/${selectedClubId}/payment-records`);
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  async function toggleReceipt(intentId: string) {
    if (openReceiptId === intentId) { openReceiptId = ''; return; }
    working = true; error = '';
    try {
      if (!timelines[intentId]) timelines = { ...timelines, [intentId]: await api<PaymentStatusEvent[]>(`/api/v1/payments/intents/${intentId}/events`) };
      openReceiptId = intentId;
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  async function cancelPayment(intentId: string) {
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/payments/intents/${intentId}/cancel`, { method: 'PATCH' });
      notice = 'Pending payment record cancelled.';
      history = await api<PaymentIntent[]>('/api/v1/payments/intents');
      timelines = { ...timelines, [intentId]: await api<PaymentStatusEvent[]>(`/api/v1/payments/intents/${intentId}/events`) };
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  function beginReconcile(record: ClubPaymentRecord) {
    reconcileId = record.id; reconciliationReference = ''; reconciliationNote = ''; markDuesCurrent = false;
  }

  async function reconcile(outcome: 'confirmed' | 'rejected') {
    if (!reconcileId) return;
    working = true; error = ''; notice = '';
    try {
      await api(`/api/v1/clubs/${selectedClubId}/payment-records/${reconcileId}/reconciliation`, { method: 'PATCH', body: JSON.stringify({ outcome, reference: reconciliationReference || null, note: reconciliationNote, mark_dues_current: outcome === 'confirmed' && markDuesCurrent }) });
      notice = outcome === 'confirmed' ? 'Payment confirmed and recorded.' : 'Payment record rejected.';
      reconcileId = '';
      [history, records] = await Promise.all([api<PaymentIntent[]>('/api/v1/payments/intents'), api<ClubPaymentRecord[]>(`/api/v1/clubs/${selectedClubId}/payment-records`)]);
    } catch (cause) { error = (cause as Error).message; }
    finally { working = false; }
  }

  onMount(() => void loadAll());
</script>

<section class="payments">
  <div class="section-head"><div><p class="eyebrow">MY ORGANIZATIONS / PAYMENTS</p><h1>Club payments</h1><p class="intro">Money goes to the club method you select. QSONaut records the workflow but does not reroute direct club payments.</p></div><button class="secondary" onclick={() => void loadAll()} disabled={loading}>REFRESH</button></div>
  {#if error}<p class="error">{error}</p>{/if}{#if notice}<p class="notice">{notice}</p>{/if}
  {#if clubs.length === 0}<p class="empty">You do not have an active club membership.</p>{:else}
    <label class="club-picker">Organization<select bind:value={selectedClubId} onchange={() => void loadClub()}>{#each clubs as club}<option value={club.id}>{club.name}</option>{/each}</select></label>
    <div class="payment-grid">
      <form onsubmit={(event) => { event.preventDefault(); void createPayment(); }}>
        <p class="eyebrow">MEMBER PAYMENT</p><h2>Pay dues or contribute</h2>
        {#if !flow?.club_dues_enabled}<p class="warning">Club payments are disabled by server policy. Methods may be prepared, but payment records cannot be created.</p>{/if}
        <label>Club payment method<select bind:value={methodId} required>{#each options as method}<option value={method.id}>{method.display_label}{method.is_default ? ' · default' : ''}</option>{/each}</select></label>
        <label>Amount ({flow?.currency || 'USD'})<input type="number" min="0.01" step="0.01" bind:value={amount} required /></label>
        {#if selectedMethod}<div class="instructions"><b>{selectedMethod.display_label}</b>{#if selectedMethod.public_reference}<span>{selectedMethod.public_reference}</span>{/if}<p>{selectedMethod.instructions || (selectedMethod.method_kind === 'external_link' ? 'Continue to the club payment page after creating the record.' : 'Provider instructions will appear here.')}</p></div>{/if}
        <button disabled={working || loading || !flow?.club_dues_enabled || !methodId}>{working ? 'WORKING…' : 'CREATE PENDING PAYMENT'}</button>
        {#if options.length === 0}<p class="empty">This club has not published a ready payment method.</p>{/if}
      </form>
      <section class="history"><p class="eyebrow">MY HISTORY</p><h2>Payment records</h2>{#each clubHistory as intent}<article><div><b>{money(intent.amount_minor, intent.currency)}</b><small>{new Date(intent.created_at).toLocaleString()}</small></div><span class:success={intent.status === 'succeeded'}>{intent.status.replaceAll('_', ' ')}</span><div class="record-actions"><button class="secondary" onclick={() => void toggleReceipt(intent.id)} disabled={working}>{openReceiptId === intent.id ? 'HIDE' : 'RECEIPT'}</button>{#if canCancel(intent)}<button class="danger" onclick={() => void cancelPayment(intent.id)} disabled={working}>CANCEL</button>{/if}</div>{#if openReceiptId === intent.id}<ol class="timeline">{#each timelines[intent.id] || [] as event}<li><b>{event.next_status.replaceAll('_', ' ')}</b><small>{new Date(event.created_at).toLocaleString()} · {event.source}</small>{#if event.note}<span>{event.note}</span>{/if}</li>{/each}</ol>{/if}</article>{:else}<p class="empty">No payment records for this club.</p>{/each}</section>
    </div>
    {#if createdMethod}<div class="next-step"><b>Next step · {createdMethod.display_label}</b><p>{createdMethod.instructions || 'Complete payment using the club destination.'}</p>{#if createdMethod.public_reference}<p>Reference: {createdMethod.public_reference}</p>{/if}{#if createdMethod.payment_url}<a href={createdMethod.payment_url} target="_blank" rel="noreferrer">OPEN CLUB PAYMENT PAGE ↗</a>{/if}</div>{/if}
    {#if selectedClub?.can_manage}<section class="reconciliation"><div><p class="eyebrow">CLUB MANAGER / RECONCILIATION</p><h2>Pending member payments</h2><p class="intro">Confirm only after the club independently verifies receipt. Automated methods cannot be confirmed here.</p></div>{#each records as record}<article><div><b>{record.callsign} · {money(record.amount_minor, record.currency)}</b><small>{record.method_label} · {new Date(record.created_at).toLocaleString()}</small></div><span class:success={record.status === 'succeeded'}>{record.status}</span>{#if record.status === 'pending' && record.method_kind !== 'automated'}<button class="secondary" onclick={() => beginReconcile(record)}>RECONCILE</button>{/if}{#if reconcileId === record.id}<div class="reconcile-form"><label>Club reference<input maxlength="200" bind:value={reconciliationReference} placeholder="Optional receipt, memo, or ledger reference" /></label><label>Internal note<textarea maxlength="2000" bind:value={reconciliationNote}></textarea></label><label class="check"><input type="checkbox" bind:checked={markDuesCurrent} /> Also mark this member's dues current</label><div><button onclick={() => void reconcile('confirmed')} disabled={working}>CONFIRM RECEIVED</button><button class="danger" onclick={() => void reconcile('rejected')} disabled={working}>REJECT</button><button class="secondary" onclick={() => reconcileId = ''}>CANCEL</button></div></div>{/if}</article>{:else}<p class="empty">No club payment records.</p>{/each}</section>{/if}
  {/if}
</section>

<style>
  .payments { display:grid; gap:18px; }.section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }.section-head h1,h2 { margin:0; }.eyebrow { color:var(--cyan); font-size:.72rem; letter-spacing:.12em; margin:0 0 4px; }.intro,.empty,.history small,.reconciliation small { color:var(--muted); }.club-picker { display:grid; gap:5px; max-width:420px; }.payment-grid { display:grid; grid-template-columns:minmax(280px,.8fr) minmax(320px,1.2fr); gap:14px; }.payments form,.history,.reconciliation,.next-step { display:grid; align-content:start; gap:10px; border:1px solid var(--line); background:var(--panel); border-radius:10px; padding:16px; }.payments label { display:grid; gap:5px; }.instructions,.warning { padding:11px; border-left:2px solid var(--amber); background:color-mix(in srgb,var(--amber) 7%,var(--panel)); }.instructions { display:grid; gap:4px; }.instructions p,.warning { margin:0; }.history article,.reconciliation article { display:grid; grid-template-columns:1fr auto auto; gap:10px; align-items:center; border-top:1px solid var(--line); padding-top:10px; }.history article div,.reconciliation article div:first-child { display:grid; gap:3px; }.history span,.reconciliation article > span { color:var(--amber); font:700 10px ui-monospace; text-transform:uppercase; }.history span.success,.reconciliation span.success { color:var(--green); }.reconciliation { gap:12px; }.reconcile-form { grid-column:1/-1; display:grid; gap:8px; padding:12px; border:1px solid var(--line); }.reconcile-form textarea { min-height:60px; }.reconcile-form .check { display:flex; align-items:center; gap:8px; }.reconcile-form .check input { width:auto; }.reconcile-form div { display:flex; gap:8px; }.next-step { border-color:color-mix(in srgb,var(--cyan) 45%,var(--line)); }.next-step p { margin:0; }.next-step a { color:var(--cyan); }.error,.notice { padding:10px 12px; border-radius:8px; }.error { color:var(--red); background:color-mix(in srgb,var(--red) 12%,transparent); }.notice { color:var(--green); background:color-mix(in srgb,var(--green) 12%,transparent); }.danger { color:var(--red); }@media (max-width:800px) { .payment-grid { grid-template-columns:1fr; }.section-head { align-items:start; flex-direction:column; }.history article,.reconciliation article { grid-template-columns:1fr; } }
  .record-actions { display:flex !important; grid-auto-flow:column; gap:6px !important; }
  .timeline { grid-column:1/-1; margin:0; padding:10px 10px 10px 28px; border-left:2px solid var(--cyan); }
  .timeline li { padding:4px 0; }
  .timeline li b,.timeline li small,.timeline li span { display:block; }
  .timeline li span { color:var(--muted); font-size:.85rem; }
</style>
