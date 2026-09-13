<script lang="ts">
  import { api, formatFrequency } from './api';
  import type { ChannelMessage, DiagnosticReport, QsoLog, ServerCapabilities, ShareLinkRecord, Station } from './types';
  import { onMount } from 'svelte';
  let { administrator, capabilities, stations, logs, messages, diagnostics, refresh, showLogs = true, showMessages = true }: { administrator: boolean; capabilities: ServerCapabilities; stations: Station[]; logs: QsoLog[]; messages: ChannelMessage[]; diagnostics: DiagnosticReport[]; refresh: () => Promise<void>; showLogs?: boolean; showMessages?: boolean } = $props();
  let externalSharingEnabled = $derived(capabilities.features.includes('external sharing'));
  let shareStatus = $state('');
  let shares = $state<ShareLinkRecord[]>([]);
  let stationSearch = $state('');
  let diagnosticSearch = $state('');
  let messageSearch = $state('');
  let logSearch = $state('');
  let logMode = $state('all');
  let shareFilter = $state('active');
  let visibleStations = $derived(stations.filter((station) => {
    const query = stationSearch.trim().toLowerCase();
    return !query || `${station.callsign} ${station.station_label} ${station.radio_model || ''} ${station.mode || ''}`.toLowerCase().includes(query);
  }));
  let visibleDiagnostics = $derived(diagnostics.filter((report) => {
    const query = diagnosticSearch.trim().toLowerCase();
    return !query || `${report.operator_callsign} ${report.category} ${report.summary}`.toLowerCase().includes(query);
  }));
  let visibleMessages = $derived(messages.filter((message) => {
    const query = messageSearch.trim().toLowerCase();
    return !query || `${message.channel} ${message.author_callsign} ${message.message}`.toLowerCase().includes(query);
  }));
  let visibleLogs = $derived(logs.filter((log) => {
    const query = logSearch.trim().toLowerCase();
    return (!query || `${log.operator_callsign} ${log.callsign} ${log.band} ${log.mode} ${log.event_name || ''}`.toLowerCase().includes(query))
      && (logMode === 'all' || log.mode === logMode);
  }));
  let logModes = $derived([...new Set(logs.map((log) => log.mode))].sort());
  let visibleShares = $derived(shares.filter((share) => {
    if (shareFilter === 'all') return true;
    if (shareFilter === 'revoked') return Boolean(share.revoked_at);
    if (shareFilter === 'expired') return !share.revoked_at && new Date(share.expires_at) <= new Date();
    return !share.revoked_at && new Date(share.expires_at) > new Date();
  }));

  async function loadShares() {
    if (!externalSharingEnabled) return;
    shares = await api<ShareLinkRecord[]>('/api/v1/shares');
  }

  async function revokeShare(id: string) {
    try {
      await api(`/api/v1/shares/${id}`, { method: 'DELETE' });
      await loadShares();
      shareStatus = 'SHARE REVOKED';
    } catch (cause) { shareStatus = (cause as Error).message; }
  }

  async function copyShareLink(log: QsoLog) {
    shareStatus = 'CREATING SHARE LINK…';
    try {
      const share = await api<{ share_path: string; expires_at: string }>(`/api/v1/logs/${log.id}/share`, {
        method: 'POST',
        body: JSON.stringify({ expires_in_days: 7 }),
      });
      await navigator.clipboard.writeText(`${window.location.origin}${share.share_path}`);
      shareStatus = `COPIED · EXPIRES ${new Date(share.expires_at).toLocaleDateString()}`;
      await loadShares();
    } catch (cause) {
      shareStatus = (cause as Error).message;
    }
  }

  onMount(() => { if (externalSharingEnabled) void loadShares(); });

</script>

<section>
  {#if administrator}
  <div class="section-head"><div><p class="eyebrow">QSONAUT / LIVE STATION NETWORK</p><h2>Connected stations</h2></div><div><small>{visibleStations.length} of {stations.length} · LIVE · 5 s</small> <button onclick={refresh}>REFRESH ACTIVITY</button></div></div>
  <div class="list-tools"><input aria-label="Search connected stations" placeholder="Search callsign, radio, or mode" bind:value={stationSearch} /></div>
  <p class="section-intro">Live operator, station, frequency, band, and mode context shared by connected QSONaut clients.</p>
  {#if stations.length === 0}
    <div class="empty-state shallow"><span>◌</span><h3>No station presence yet</h3><p>Connected QSONaut stations will appear here as operators come online.</p></div>
  {:else if visibleStations.length === 0}
    <p class="empty">No stations match the current search.</p>
  {:else}
    <div class="station-grid">
    {#each visibleStations as station}
        <article class="station-card large"><i class:online={station.status === 'online'}></i><div><div class="detail-head"><b>{station.callsign} · {station.station_label || 'QSONaut'}</b><span class="pill">{station.status}</span></div><strong>{formatFrequency(station.frequency_hz)}</strong><p>{station.band || 'station band'} · {station.mode || 'station mode'} · {[station.radio_manufacturer, station.radio_model].filter(Boolean).join(' ') || 'QSONaut station'}</p><small>{station.display_name} · QSONaut {station.qsonaut_version} · {station.platform} · seen {new Date(station.last_seen).toLocaleString()}</small><details><summary>Full shared station metadata</summary><pre>{JSON.stringify(station.metadata, null, 2)}</pre></details></div></article>
      {/each}
    </div>
  {/if}

  <div class="section-head"><div><p class="eyebrow">SUPPORT / OPT-IN SNAPSHOTS</p><h2>Radio diagnostics</h2></div><b>{visibleDiagnostics.length} of {diagnostics.length} reports</b></div>
  <div class="list-tools"><input aria-label="Search diagnostics" placeholder="Search operator, category, or summary" bind:value={diagnosticSearch} /></div>
  {#if diagnostics.length === 0}
    <div class="empty-state shallow"><span>⌁</span><h3>No diagnostic snapshots</h3><p>Operators must explicitly enable and send snapshots from QSONaut.</p></div>
  {:else if visibleDiagnostics.length === 0}
    <p class="empty">No diagnostics match the current search.</p>
  {:else}
    {#each visibleDiagnostics as report}
      <article class="diagnostic-card"><div class="detail-head"><div><b>{report.operator_callsign} · {report.summary}</b><p>{report.category} · {new Date(report.created_at).toLocaleString()}</p></div><span class="pill">{report.instance_id.slice(0,8)}</span></div><details><summary>Inspect full report</summary><pre>{JSON.stringify(report.payload, null, 2)}</pre></details></article>
    {/each}
  {/if}

  {#if showMessages}
  <div class="section-head"><div><p class="eyebrow">AUTOMATION / CHANNEL TRAFFIC</p><h2>Messages</h2></div><b>{visibleMessages.length} of {messages.length} messages</b></div>
  <div class="list-tools"><input aria-label="Search channel messages" placeholder="Search channel, author, or message" bind:value={messageSearch} /></div>
  {#if messages.length === 0}
    <div class="empty-state shallow"><span>›_</span><h3>No channel traffic yet</h3><p>Operator tools and permissioned automations can publish into shared channels.</p></div>
  {:else if visibleMessages.length === 0}
    <p class="empty">No messages match the current search.</p>
  {:else}
    <div class="station-grid">
    {#each visibleMessages as message}
        <article class="station-card large"><div><div class="detail-head"><b>#{message.channel} · {message.author_callsign}</b><small>{new Date(message.created_at).toLocaleString()}</small></div><p>{message.message}</p></div></article>
      {/each}
    </div>
  {/if}
  {/if}

  {/if}

  {#if !administrator}
    <div class="section-head"><div><p class="eyebrow">MY SUBMISSIONS / SUPPORT HISTORY</p><h2>Hardware validation snapshots</h2></div><b>{visibleDiagnostics.length} submitted</b></div>
    <p class="section-intro">These are the opt-in support snapshots submitted by your QSONaut stations. They are visible to you and to server administrators; cross-operator review is administrator-only.</p>
    {#if diagnostics.length === 0}<p class="empty">No hardware validation snapshots submitted yet.</p>{:else if visibleDiagnostics.length === 0}<p class="empty">No submitted snapshots match the current search.</p>{:else}{#each visibleDiagnostics as report}<article class="diagnostic-card"><div class="detail-head"><div><b>{report.summary}</b><p>{report.category} · submitted {new Date(report.created_at).toLocaleString()}</p></div><span class="pill">{report.instance_id.slice(0,8)}</span></div><details><summary>View my submitted snapshot</summary><pre>{JSON.stringify(report.payload, null, 2)}</pre></details></article>{/each}{/if}
  {/if}

  {#if showLogs}
  <div class="section-head"><div><p class="eyebrow">{administrator ? 'COLLECTED ACTIVITY / SERVER REVIEW' : 'MY SUBMISSIONS / LOG HISTORY'}</p><h2>{administrator ? 'QSO logs' : 'My QSO logs'}</h2></div><div><b>{visibleLogs.length} of {logs.length} records</b>{#if shareStatus}<small class="share-status">{shareStatus}</small>{/if}</div></div>
  <div class="list-tools"><input aria-label="Search QSO logs" placeholder="Search callsign, band, mode, or event" bind:value={logSearch} /><select aria-label="Filter QSO logs by mode" bind:value={logMode}><option value="all">All modes</option>{#each logModes as mode}<option value={mode}>{mode}</option>{/each}</select></div>
  {#if logs.length === 0}
    <div class="empty-state shallow"><span>≋</span><h3>No uploaded logs</h3><p>Logs will appear here when an authenticated QSONaut client submits its idempotent QSO records. Manual logging remains in QSONaut.</p></div>
  {:else}
    {#if visibleLogs.length === 0}<p class="empty">No logs match the current filters.</p>{:else}<div class="table-wrap"><table><thead><tr><th>UTC</th><th>Operator</th><th>Worked</th><th>Band</th><th>Mode</th><th>Frequency</th><th>Event</th><th>Source</th>{#if externalSharingEnabled}<th></th>{/if}</tr></thead><tbody>{#each visibleLogs as log}<tr><td>{new Date(log.occurred_at).toISOString().replace('T', ' ').slice(0, 19)}</td><td>{log.operator_callsign}</td><td><b>{log.callsign}</b></td><td>{log.band}</td><td>{log.mode}</td><td>{formatFrequency(log.frequency_hz)}</td><td>{log.event_name || '—'}</td><td>{log.source}</td>{#if externalSharingEnabled}<td><button onclick={() => copyShareLink(log)}>COPY LINK</button></td>{/if}</tr>{/each}</tbody></table></div>{/if}
  {/if}

  {#if externalSharingEnabled}
    <div class="section-head"><div><p class="eyebrow">SHARING / ACTIVE LINKS</p><h2>Copyable detail links</h2></div><b>{visibleShares.length} of {shares.length}</b></div>
    <div class="list-tools"><select aria-label="Filter share links" bind:value={shareFilter}><option value="active">Active links</option><option value="expired">Expired links</option><option value="revoked">Revoked links</option><option value="all">All links</option></select></div>
    {#if shares.length === 0}
      <p class="section-intro">Create a link from any QSO record above. Links expire automatically and can be revoked here.</p>
    {:else if visibleShares.length === 0}
      <p class="empty">No share links match this filter.</p>
    {:else}
      <div class="share-list">{#each visibleShares as share}<article><div><b>{share.worked_callsign}</b><small>{new Date(share.occurred_at).toLocaleString()} · expires {new Date(share.expires_at).toLocaleDateString()}</small></div>{#if share.revoked_at}<span class="pill">revoked</span>{:else if new Date(share.expires_at) <= new Date()}<span class="pill">expired</span>{:else}<button onclick={() => void revokeShare(share.id)}>REVOKE</button>{/if}</article>{/each}</div>
    {/if}
  {/if}
  {/if}
</section>

<style>
  .list-tools { display:flex; align-items:center; gap:8px; margin:10px 0 14px; }
  .list-tools input { flex:1; min-width:0; }
  .list-tools select { min-width:150px; }
  details { margin-top: 12px; color: var(--cyan); }
  summary { cursor: pointer; font: 600 10px ui-monospace; letter-spacing: .08em; text-transform: uppercase; }
  pre { max-height: 420px; overflow: auto; white-space: pre-wrap; word-break: break-word; padding: 14px; background: #040b0e; border: 1px solid var(--line); color: #b8d6dc; font: 11px ui-monospace; }
  .diagnostic-card { padding: 18px; margin-bottom: 10px; border: 1px solid var(--line); background: var(--panel); }
  .diagnostic-card p { margin: 5px 0; color: var(--muted); }
  .share-status { display: block; color: var(--cyan); margin-top: 4px; }
  .share-list { display: grid; gap: 8px; margin-bottom: 34px; }
  .share-list article { display: flex; align-items: center; justify-content: space-between; gap: 14px; padding: 14px 16px; border: 1px solid var(--line); background: var(--panel); }
  .share-list article div { display: grid; gap: 5px; }
  .share-list small { color: var(--muted); }
</style>
