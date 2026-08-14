<script lang="ts">
  import { formatFrequency } from './api';
  import type { QsoLog, Station } from './types';
  let { stations, logs }: { stations: Station[]; logs: QsoLog[] } = $props();
</script>

<section>
  <p class="eyebrow">QSONAUT / OPT-IN REMOTE STATE</p>
  <h2>Connected stations</h2>
  <p class="section-intro">QSONaut publishes this operational snapshot when its server connection and presence sharing are enabled. It is coordination data, never remote-control authority.</p>
  {#if stations.length === 0}
    <div class="empty-state shallow"><span>◌</span><h3>No station presence yet</h3><p>The server endpoint is ready; the QSONaut desktop publisher is the next client-side integration.</p></div>
  {:else}
    <div class="station-grid">
      {#each stations as station}
        <article class="station-card large"><i class:online={station.status === 'online'}></i><div><div class="detail-head"><b>{station.callsign} · {station.station_label || 'QSONaut'}</b><span class="pill">{station.status}</span></div><strong>{formatFrequency(station.frequency_hz)}</strong><p>{station.band || 'band unknown'} · {station.mode || 'mode unknown'} · {[station.radio_manufacturer, station.radio_model].filter(Boolean).join(' ') || 'radio not shared'}</p><small>{station.display_name} · QSONaut {station.qsonaut_version} · {station.platform} · seen {new Date(station.last_seen).toLocaleString()}</small></div></article>
      {/each}
    </div>
  {/if}

  <div class="section-head"><div><p class="eyebrow">COLLECTED ACTIVITY</p><h2>QSO logs</h2></div><b>{logs.length} records</b></div>
  {#if logs.length === 0}
    <div class="empty-state shallow"><span>≋</span><h3>No uploaded logs</h3><p>Logs will appear here when an authenticated QSONaut client submits its idempotent QSO records. Manual logging remains in QSONaut.</p></div>
  {:else}
    <div class="table-wrap"><table><thead><tr><th>UTC</th><th>Operator</th><th>Worked</th><th>Band</th><th>Mode</th><th>Frequency</th><th>Event</th><th>Source</th></tr></thead><tbody>{#each logs as log}<tr><td>{new Date(log.occurred_at).toISOString().replace('T',' ').slice(0,19)}</td><td>{log.operator_callsign}</td><td><b>{log.callsign}</b></td><td>{log.band}</td><td>{log.mode}</td><td>{formatFrequency(log.frequency_hz)}</td><td>{log.event_name || '—'}</td><td>{log.source}</td></tr>{/each}</tbody></table></div>
  {/if}
</section>
