<script lang="ts">
  const qsonautUrl = 'https://github.com/nicksbar/QSONaut';
  const serverUrl = 'https://github.com/nicksbar/QSONaut-Server';
  const issuesUrl = `${serverUrl}/issues`;
</script>

<footer class="site-footer">
  <div class="footer-brand">
    <img src="/qsonaut-icon.png" alt="" />
    <div>
      <strong>QSONaut</strong>
      <p>Amateur radio mission control for operators, clubs, contests, and shared station activity.</p>
    </div>
  </div>
  <div class="footer-links">
    <a href={qsonautUrl} target="_blank" rel="noreferrer">Desktop project ↗</a>
    <a href={serverUrl} target="_blank" rel="noreferrer">Server project ↗</a>
    <a href={issuesUrl} target="_blank" rel="noreferrer">Report an issue ↗</a>
    <a href="https://qsonaut.com" target="_blank" rel="noreferrer">qsonaut.com ↗</a>
  </div>
  <div class="footer-legal">
    <span>© 2026 N7UF / QSONaut</span>
    <span>Open-source amateur radio software</span>
  </div>
</footer>

<style>
  .site-footer { display:grid; grid-template-columns:1.4fr 1fr; gap:24px 48px; margin-top:auto; padding:28px 0 22px; border-top:1px solid var(--line); color:var(--muted); }
  .footer-brand { display:flex; align-items:center; gap:12px; }
  .footer-brand img { width:42px; height:42px; object-fit:contain; }
  .footer-brand strong { display:block; color:var(--cyan); font-size:18px; font-weight:500; }
  .footer-brand p { max-width:450px; margin:5px 0 0; font-size:12px; line-height:1.5; }
  .footer-links { display:flex; flex-wrap:wrap; justify-content:flex-end; align-content:center; gap:8px 18px; font-size:12px; }
  .footer-links a { color:var(--cyan); text-decoration:none; }
  .footer-links a:hover { color:#fff; }
  .footer-legal { grid-column:1/-1; display:flex; justify-content:space-between; gap:12px; padding-top:14px; border-top:1px solid #16313a; font:10px ui-monospace; letter-spacing:.06em; }
  @media (max-width:700px) { .site-footer { grid-template-columns:1fr; } .footer-links { justify-content:flex-start; } .footer-legal { display:grid; } }
</style>
