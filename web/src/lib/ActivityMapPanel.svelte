<script lang="ts">
  import { api } from './api';
  import type { ActivityMapPoint, Club, ManagedCallsign } from './types';

  let { clubs, identities }: { clubs: Club[]; identities: ManagedCallsign[] } = $props();
  let scope = $state('overall');
  let callsignId = $state('');
  let points = $state<ActivityMapPoint[]>([]);
  let loading = $state(false);
  let notice = $state('');
  let selected = $state<ActivityMapPoint | null>(null);
  let activeClubs = $derived(clubs.filter((club) => club.my_membership_status === 'active'));

  function position(point: ActivityMapPoint) {
    return { left: `${((point.longitude + 180) / 360) * 100}%`, top: `${((90 - point.latitude) / 180) * 100}%` };
  }

  async function load() {
    loading = true; notice = '';
    try {
      const query = scope === 'overall'
        ? `scope=overall${callsignId ? `&callsign_id=${encodeURIComponent(callsignId)}` : ''}`
        : `scope=club&scope_id=${encodeURIComponent(scope)}`;
      points = await api<ActivityMapPoint[]>(`/api/v1/activity/map?${query}`);
      selected = points[0] || null;
    } catch (cause) { notice = (cause as Error).message; }
    finally { loading = false; }
  }

  $effect(() => { void load(); });
</script>

<section class="activity-map">
  <div class="section-head"><div><p class="eyebrow amber">COMMUNITY / QSO MAP</p><h2>Worked grid squares</h2><p class="section-intro">A basic view of valid Maidenhead grids submitted in visible QSO exchanges. Markers are grid-square centres, never precise operator locations.</p></div><b>{points.length} grids</b></div>
  <div class="map-tools"><label>Activity scope<select bind:value={scope} onchange={() => { if (scope !== 'overall') callsignId = ''; }}><option value="overall">My owned callsigns</option>{#each activeClubs as club}<option value={club.id}>{club.name} activity</option>{/each}</select></label>{#if scope === 'overall' && identities.length}<label>Callsign<select bind:value={callsignId}><option value="">All owned callsigns</option>{#each identities as identity}<option value={identity.id}>{identity.callsign}</option>{/each}</select></label>{/if}<button class="secondary" onclick={() => void load()} disabled={loading}>{loading ? 'LOADING…' : 'REFRESH MAP'}</button></div>
  <div class="world-map" aria-label="QSO activity map">
    <div class="equator"></div><div class="meridian one"></div><div class="meridian two"></div><div class="meridian three"></div><div class="latitude north"></div><div class="latitude south"></div>
    {#each points as point}<button class:selected={selected?.grid === point.grid} class="map-point" style:left={position(point).left} style:top={position(point).top} onclick={() => selected = point} aria-label={`${point.grid}: ${point.qso_count} QSOs`}><span style:width={`${Math.min(34, 10 + point.qso_count * 3)}px`} style:height={`${Math.min(34, 10 + point.qso_count * 3)}px`}></span></button>{/each}
    {#if points.length === 0 && !loading}<p class="map-empty">No map-ready exchanges were found. The community map uses server-shared QSOs with a Maidenhead grid; publish a new grid-bearing contact or choose a different scope.</p>{/if}
  </div>
  {#if selected}<div class="map-detail"><b>{selected.grid}</b><span>{selected.qso_count} QSO{selected.qso_count === 1 ? '' : 's'} · last {new Date(selected.last_qso_at).toLocaleString()}</span></div>{/if}
  {#if notice}<p class="error">{notice}</p>{/if}
</section>

<style>
  .activity-map { display:grid; gap:16px; padding:25px; border:1px solid var(--line); background:var(--panel); margin-bottom:34px; }.section-head { display:flex; justify-content:space-between; gap:16px; align-items:end; }.section-head h2 { margin:4px 0; }.section-head b { color:var(--cyan); font:500 24px ui-monospace; }.map-tools { display:flex; gap:10px; align-items:end; }.map-tools label { display:grid; gap:5px; color:var(--muted); font:600 10px ui-monospace; text-transform:uppercase; }.world-map { position:relative; min-height:330px; overflow:hidden; border:1px solid var(--line); background:radial-gradient(circle at 50% 48%, color-mix(in srgb,var(--cyan) 12%,transparent),transparent 48%),#07161c; }.equator,.meridian,.latitude { position:absolute; background:color-mix(in srgb,var(--line) 80%,transparent); }.equator { left:0; right:0; top:50%; height:1px; }.meridian { top:0; bottom:0; width:1px; }.meridian.one { left:25%; }.meridian.two { left:50%; }.meridian.three { left:75%; }.latitude { left:0; right:0; height:1px; }.latitude.north { top:25%; }.latitude.south { top:75%; }.map-point { position:absolute; z-index:2; padding:0; border:0; background:transparent; transform:translate(-50%,-50%); cursor:pointer; }.map-point span { display:block; border-radius:999px; border:2px solid var(--panel); background:var(--cyan); box-shadow:0 0 0 4px color-mix(in srgb,var(--cyan) 20%,transparent),0 0 18px color-mix(in srgb,var(--cyan) 65%,transparent); }.map-point.selected span { background:var(--amber); box-shadow:0 0 0 5px color-mix(in srgb,var(--amber) 25%,transparent),0 0 20px color-mix(in srgb,var(--amber) 70%,transparent); }.map-empty { position:absolute; inset:0; display:grid; place-items:center; margin:0; padding:28px; color:var(--muted); text-align:center; }.map-detail { display:flex; gap:10px; align-items:baseline; padding:12px; background:var(--chip); }.map-detail b { color:var(--cyan); font:600 18px ui-monospace; }.map-detail span { color:var(--muted); }.error { color:var(--red); margin:0; } @media (max-width:700px) { .section-head { align-items:start; flex-direction:column; }.map-tools { align-items:stretch; flex-direction:column; }.world-map { min-height:240px; } }
</style>
