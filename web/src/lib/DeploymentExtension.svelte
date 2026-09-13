<script lang="ts">
  import type { Club, ServerCapabilities, User } from './types';

  let { capabilities: _capabilities, clubs: _clubs, user: _user, initialClubId: _initialClubId = null }: { capabilities: ServerCapabilities; clubs: Club[]; user: User | null; initialClubId?: string | null } = $props();

  // Community builds intentionally provide no hosted deployment extension.
  // Hosted deployments replace this module during proprietary web assembly.
</script>
