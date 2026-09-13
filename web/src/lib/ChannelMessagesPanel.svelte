<script lang="ts">
  import type { ChannelMessage } from './types';
  let { messages }: { messages: ChannelMessage[] } = $props();
  let query = $state('');
  let visible = $derived(messages.filter((message) => !query.trim() || `${message.channel} ${message.author_callsign} ${message.message}`.toLowerCase().includes(query.trim().toLowerCase())));
</script>

<section class="messages-panel">
  <div class="section-head"><div><p class="eyebrow">SERVER DATA / ADMINISTRATOR ONLY</p><h1>Channel traffic</h1><p class="section-intro">Review recent automation and operator channel messages retained by the server.</p></div><b>{visible.length} of {messages.length}</b></div>
  <div class="list-tools"><input aria-label="Search channel messages" placeholder="Search channel, author, or message" bind:value={query} /></div>
  {#if messages.length === 0}<div class="empty-state shallow"><span>›_</span><h2>No channel traffic</h2><p>Permissioned clients and automations have not published any retained messages.</p></div>
  {:else if visible.length === 0}<p class="empty">No messages match the current search.</p>
  {:else}<div class="message-list">{#each visible as message}<article><div class="detail-head"><b>#{message.channel} · {message.author_callsign}</b><small>{new Date(message.created_at).toLocaleString()}</small></div><p>{message.message}</p><details><summary>Metadata</summary><pre>{JSON.stringify(message.metadata, null, 2)}</pre></details></article>{/each}</div>{/if}
</section>

<style>
  .messages-panel { display:grid; gap:18px; }.section-head { display:flex; justify-content:space-between; align-items:end; gap:16px; }.section-head h1 { margin:4px 0; }.section-head b { color:var(--muted); }.list-tools { display:flex; gap:8px; }.list-tools input { width:100%; }.message-list { display:grid; gap:10px; }.message-list article { padding:16px; border:1px solid var(--line); background:var(--panel); }.message-list p { margin:10px 0 0; color:var(--text); }.detail-head { display:flex; justify-content:space-between; gap:12px; }.detail-head small { color:var(--muted); } details { margin-top:12px; } summary { cursor:pointer; color:var(--cyan); font:600 10px ui-monospace; text-transform:uppercase; } pre { max-height:240px; overflow:auto; padding:12px; background:#040b0e; color:#b8d6dc; white-space:pre-wrap; }
</style>
