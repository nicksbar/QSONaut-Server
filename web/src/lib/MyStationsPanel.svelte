<script lang="ts">
  import { formatFrequency } from './api';
  import type { Station } from './types';
  let { stations, compact = false }: { stations: Station[]; compact?: boolean } = $props();
</script>

<section class:compact class="my-stations">
  <div class="section-head"><div><p class="eyebrow">MY STATIONS</p><h2>{compact ? 'Connected stations' : 'My connected stations'}</h2></div><b>{stations.length} reported</b></div>
  {#if stations.length === 0}<p class="empty">No station has reported in yet. Link a QSONaut installation to see it here.</p>
  {:else}<div class="station-list">{#each stations as station}<article><i class:online={station.status === 'online'}></i><div><b>{station.station_label || 'QSONaut station'}</b><p>{[station.radio_manufacturer, station.radio_model].filter(Boolean).join(' ') || 'Radio details not reported'} · {formatFrequency(station.frequency_hz)} · {station.band || 'band unknown'} · {station.mode || 'mode unknown'}</p><small>{station.platform} · QSONaut {station.qsonaut_version} · seen {new Date(station.last_seen).toLocaleString()}</small></div><span class="pill">{station.status}</span></article>{/each}</div>{/if}
</section>

<style>
  .my-stations { padding:20px; border:1px solid var(--line); background:var(--panel); }.section-head { display:flex; justify-content:space-between; align-items:end; gap:14px; }.section-head h2 { margin:3px 0 0; }.section-head b,.empty,small { color:var(--muted); }.station-list { display:grid; gap:8px; margin-top:14px; }.station-list article { display:grid; grid-template-columns:auto minmax(0,1fr) auto; align-items:start; gap:12px; padding:13px; border:1px solid var(--line); background:var(--chip); }.station-list i { width:9px; height:9px; border-radius:50%; margin-top:6px; background:var(--muted); }.station-list i.online { background:var(--green); box-shadow:0 0 10px color-mix(in srgb,var(--green) 60%,transparent); }.station-list p { margin:4px 0; color:var(--muted); }.compact { padding:16px; }
</style>
