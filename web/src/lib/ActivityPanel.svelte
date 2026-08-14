<script lang="ts">
  import { formatFrequency } from './api';
  import type { ChannelMessage, QsoLog, Station } from './types';
  let { stations, logs, messages, refresh }: { stations: Station[]; logs: QsoLog[]; messages: ChannelMessage[]; refresh: () => Promise<void> } = $props();
</script>

<section>
  <div class="section-head"><div><p class="eyebrow">QSONAUT / LIVE STATION NETWORK</p><h2>Connected stations</h2></div><button onclick={refresh}>REFRESH ACTIVITY</button></div>
  <p class="section-intro">Live operator, station, frequency, band, and mode context shared by connected QSONaut clients.</p>
  {#if stations.length === 0}
    <div class="empty-state shallow"><span>◌</span><h3>No station presence yet</h3><p>Connected QSONaut stations will appear here as operators come online.</p></div>
  {:else}
    <div class="station-grid">
      {#each stations as station}
        <article class="station-card large"><i class:online={station.status === 'online'}></i><div><div class="detail-head"><b>{station.callsign} · {station.station_label || 'QSONaut'}</b><span class="pill">{station.status}</span></div><strong>{formatFrequency(station.frequency_hz)}</strong><p>{station.band || 'station band'} · {station.mode || 'station mode'} · {[station.radio_manufacturer, station.radio_model].filter(Boolean).join(' ') || 'QSONaut station'}</p><small>{station.display_name} · QSONaut {station.qsonaut_version} · {station.platform} · seen {new Date(station.last_seen).toLocaleString()}</small></div></article>
      {/each}
    </div>
  {/if}

  <div class="section-head"><div><p class="eyebrow">AUTOMATION / CHANNEL TRAFFIC</p><h2>Messages</h2></div><b>{messages.length} messages</b></div>
  {#if messages.length === 0}
    <div class="empty-state shallow"><span>›_</span><h3>No channel traffic yet</h3><p>Operator tools and permissioned automations can publish into shared channels.</p></div>
  {:else}
    <div class="station-grid">
      {#each messages as message}
        <article class="station-card large"><div><div class="detail-head"><b>#{message.channel} · {message.author_callsign}</b><small>{new Date(message.created_at).toLocaleString()}</small></div><p>{message.message}</p></div></article>
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
