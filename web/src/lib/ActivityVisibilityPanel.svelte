<script lang="ts">
  import { api } from './api';
  import type { ActivityVisibility, Club, Event, ManagedCallsign } from './types';

  let { identities, clubs, events }: { identities: ManagedCallsign[]; clubs: Club[]; events: Event[] } = $props();
  let policies = $state<ActivityVisibility[]>([]);
  let notice = $state('');
  let busy = $state(false);
  let activeClubs = $derived(clubs.filter((club) => club.my_membership_status === 'active'));
  let myIdentities = $derived(identities.filter((identity) => identity.owner_user_id || identity.club_id));
  let myEvents = $derived(events.filter((event) => activeClubs.some((club) => club.id === event.club_id)));

  function policy(scope: ActivityVisibility['scope'], scopeId: string) { return policies.find((item) => item.scope === scope && item.scope_id === scopeId)?.visibility || 'private'; }
  function club(clubId: string | null) { return clubs.find((item) => item.id === clubId); }
  function canManage(clubId: string | null) { const role = club(clubId)?.my_role; return role === 'owner' || role === 'coordinator'; }
  function canEditIdentity(identity: ManagedCallsign) { return !!identity.owner_user_id || canManage(identity.club_id); }
  function identityDescription(identity: ManagedCallsign) { return identity.identity_type === 'personal' ? 'You control this callsign.' : `${club(identity.club_id)?.name || 'The owning club'} controls this callsign.`; }

  async function load() { policies = await api<ActivityVisibility[]>('/api/v1/activity/visibility'); }
  async function save(scope: ActivityVisibility['scope'], scopeId: string, visibility: string) {
    busy = true; notice = '';
    try { await api<ActivityVisibility>('/api/v1/activity/visibility', { method: 'PUT', body: JSON.stringify({ scope, scope_id: scopeId, visibility }) }); await load(); notice = 'Activity policy saved.'; }
    catch (cause) { notice = (cause as Error).message; } finally { busy = false; }
  }
  $effect(() => { void load().catch((cause) => { notice = (cause as Error).message; }); });
</script>

<section class="policy-panel">
  <div class="section-head"><div><p class="eyebrow amber">ACTIVITY / POLICY OWNERSHIP</p><h2>Who controls each activity stream?</h2></div><small>Maps use the same effective permissions</small></div>
  <p class="section-intro">A policy belongs to the callsign, club, or event that produced the activity. You can change personal callsign policies; club and event policies require an owner or coordinator.</p>
  <section class="policy-group"><div class="group-heading"><div><p class="eyebrow cyan">CALLSIGNS</p><h3>Operating identities</h3></div><small>Individual logs follow the selected callsign.</small></div><div class="policy-grid">
    {#each myIdentities as identity}<label><span>{identity.callsign} <small>{identity.identity_type}</small></span><select value={policy('identity', identity.id)} disabled={busy || !canEditIdentity(identity)} onchange={(event) => void save('identity', identity.id, event.currentTarget.value)}><option value="private">Submitter only</option><option value="members" disabled={identity.identity_type === 'personal'}>Owning club members</option><option value="global">Anyone with access</option></select><em>{identityDescription(identity)}{canEditIdentity(identity) ? '' : ' Read-only policy.'}</em></label>{/each}
  </div></section>
  <section class="policy-group"><div class="group-heading"><div><p class="eyebrow cyan">CLUBS</p><h3>Organization activity</h3></div><small>Defaults for club-owned identities and club activity.</small></div><div class="policy-grid">
    {#each activeClubs as clubItem}<label><span>{clubItem.name} <small>club</small></span><select value={policy('club', clubItem.id)} disabled={busy || !canManage(clubItem.id)} onchange={(event) => void save('club', clubItem.id, event.currentTarget.value)}><option value="private">Submitter only</option><option value="members">Club members</option><option value="global">Anyone with access</option></select><em>{canManage(clubItem.id) ? 'You manage this organization policy.' : 'Controlled by club owners or coordinators.'}</em></label>{/each}
  </div></section>
  <section class="policy-group"><div class="group-heading"><div><p class="eyebrow cyan">EVENTS</p><h3>Event activity</h3></div><small>Event policy overrides identity and club defaults.</small></div><div class="policy-grid">
    {#each myEvents as event}<label><span>{event.name} <small>event</small></span><select value={policy('event', event.id)} disabled={busy || !canManage(event.club_id)} onchange={(change) => void save('event', event.id, change.currentTarget.value)}><option value="private">Submitter only</option><option value="members">Club members</option><option value="global">Anyone with access</option></select><em>{canManage(event.club_id) ? 'You manage this event policy.' : 'Controlled by the owning club.'}</em></label>{/each}
  </div></section>
  {#if notice}<p class="notice">{notice}</p>{/if}
</section>

<style>
  .policy-panel { display:grid; gap:22px; padding:25px; border:1px solid var(--line); background:#061217cc; margin-bottom:34px; }.section-head,.group-heading { display:flex; justify-content:space-between; gap:16px; align-items:end; }.section-head h2,.group-heading h3 { margin:4px 0; }.section-head small,.group-heading small { color:var(--muted); }.section-intro { max-width:850px; margin:0; color:var(--muted); line-height:1.55; }.policy-group { display:grid; gap:12px; padding-top:18px; border-top:1px solid var(--line); }.policy-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:12px; }.policy-grid label { display:grid; gap:8px; padding:14px; border:1px solid var(--line); background:#07161c; color:var(--muted); font:600 10px ui-monospace; text-transform:uppercase; letter-spacing:.06em; }.policy-grid label > span { color:var(--text); }.policy-grid label small { color:var(--cyan); }.policy-grid label em { color:var(--muted); font:400 11px system-ui; text-transform:none; letter-spacing:0; line-height:1.4; }.policy-grid select:disabled { opacity:.7; } @media (max-width:900px) { .policy-grid { grid-template-columns:repeat(2,minmax(0,1fr)); } } @media (max-width:700px) { .section-head,.group-heading { align-items:start; flex-direction:column; }.policy-grid { grid-template-columns:1fr; } }
</style>
