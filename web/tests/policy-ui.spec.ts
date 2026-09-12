import { expect, test, type Page, type Route } from '@playwright/test';

const member = {
  id: 'user-member',
  callsign: 'N7UF',
  display_name: 'N7UF Operator',
  global_role: 'member',
};

const club = {
  id: 'club-cascade',
  name: 'Cascade Radio Club',
  callsign: 'W7CRC',
  description: 'A local radio club',
  member_count: 12,
  renewal_attention_count: 0,
  my_role: 'member',
  my_membership_status: 'active',
  join_request_status: null,
  can_manage: false,
};

const event = {
  id: 'event-field-day',
  club_id: club.id,
  name: 'Cascade Field Day',
  contest_name: 'ARRL Field Day',
  special_callsign: null,
  starts_at: '2026-06-27T18:00:00Z',
  ends_at: '2026-06-28T21:00:00Z',
  status: 'scheduled',
  contest_template_id: null,
  contest_definition_version: null,
  contest_config: {},
  participant_count: 3,
};

const identity = {
  id: 'identity-n7uf',
  owner_user_id: member.id,
  club_id: null,
  event_id: null,
  callsign: member.callsign,
  identity_type: 'personal',
  authority: 'N7UF Operator',
  verification_status: 'verified',
  status: 'active',
  effective_from: '2026-01-01T00:00:00Z',
  expires_at: null,
};

const clubIdentity = {
  ...identity,
  id: 'identity-w7crc',
  owner_user_id: null,
  club_id: club.id,
  callsign: club.callsign,
  identity_type: 'club',
  authority: club.name,
};

const profile = {
  user: member,
  grid: 'CN87',
  qth: 'Seattle, WA',
  first_name: 'N7UF',
  middle_name: '',
  surname: 'Operator',
  suffix: '',
  license_class: 'Extra',
  license_status: 'active',
  license_expires_on: null,
  address_line_1: '',
  address_line_2: '',
  state: 'WA',
  postal_code: '',
  country: 'US',
  latitude: '',
  longitude: '',
  hamdb_fetched_at: null,
  hamdb_last_error: '',
};

const mapPoint = {
  grid: 'CN87XX',
  latitude: 47.65,
  longitude: -122.35,
  qso_count: 2,
  last_qso_at: '2026-09-10T12:00:00Z',
};

function response(route: Route, body: unknown, status = 200) {
  return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
}

async function installApiMock(page: Page, role: 'member' | 'administrator' = 'member', membershipRole: 'member' | 'coordinator' = 'member') {
  const user = role === 'administrator'
    ? { ...member, id: 'user-admin', callsign: 'K1ADM', display_name: 'K1ADM Administrator', global_role: role }
    : member;
  const policies = [
    { id: 'policy-identity', scope: 'identity', scope_id: identity.id, visibility: 'private', updated_by_user_id: user.id, can_edit: role === 'member', updated_at: '2026-09-10T00:00:00Z' },
    { id: 'policy-club', scope: 'club', scope_id: club.id, visibility: 'members', updated_by_user_id: user.id, can_edit: role === 'member', updated_at: '2026-09-10T00:00:00Z' },
    { id: 'policy-event', scope: 'event', scope_id: event.id, visibility: 'private', updated_by_user_id: user.id, can_edit: role === 'member', updated_at: '2026-09-10T00:00:00Z' },
  ];
  const calls: { method: string; path: string; url: string; body?: string }[] = [];
  let authenticated = false;

  await page.route('**/api/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    calls.push({ method: request.method(), path, url: request.url(), body: request.postData() || undefined });

    if (path === '/api/v1/auth/setup' && request.method() === 'GET') return response(route, { setup_required: false });
    if (path === '/api/v1/auth/login' && request.method() === 'POST') {
      authenticated = true;
      return response(route, user);
    }
    if (path === '/api/v1/auth/me') return authenticated ? response(route, user) : response(route, { error: 'authentication required' }, 401);
    if (path === '/api/v1/auth/profile') return response(route, { ...profile, user });
    if (path === '/api/v1/clubs') return response(route, [{ ...club, my_role: membershipRole, can_manage: membershipRole === 'coordinator' }, { ...club, id: 'club-inactive', name: 'Inactive Club', my_membership_status: 'inactive', my_role: 'member', can_manage: false }]);
    if (path === '/api/v1/events') return response(route, [event, { ...event, id: 'event-hidden', club_id: 'club-inactive', name: 'Inactive Club Event' }]);
    if (path === '/api/v1/contest-templates') return response(route, []);
    if (path === '/api/v1/identities') return response(route, [identity, clubIdentity]);
    if (path === '/api/v1/capabilities') return response(route, { edition: 'community', max_clubs: 5, features: [] });
    if (path === '/api/v1/logs') return response(route, [{
      id: 'log-1', user_id: user.id, callsign_id: identity.id, operating_callsign: identity.callsign,
      operator_callsign: identity.callsign, callsign: 'W1AW', band: '20m', mode: 'FT8', frequency_hz: 14_074_000,
      occurred_at: '2026-09-10T12:00:00Z', exchange: { grid: mapPoint.grid }, event_name: null, source: 'qsonaut',
    }]);
    if (path === '/api/v1/stations') return response(route, []);
    if (path === '/api/v1/diagnostics') return response(route, []);
    if (path === '/api/v1/channel-messages') return response(route, []);
    if (path === '/api/v1/members') return response(route, []);
    if (path === '/api/v1/access/requests') return response(route, []);
    if (path === '/api/v1/activity/summary') return response(route, { scope: 'overall', scope_id: null, period_days: 30, qso_count: 1, unique_callsigns: 1, band_count: 1, mode_count: 1, points: 1, last_qso_at: mapPoint.last_qso_at, status: 'active' });
    if (path === '/api/v1/activity/map') return response(route, [mapPoint]);
    if (path === '/api/v1/activity/visibility' && request.method() === 'GET') return response(route, policies);
    if (path === '/api/v1/activity/visibility' && request.method() === 'PUT') {
      const body = JSON.parse(request.postData() || '{}') as { scope: string; scope_id: string; visibility: string };
      const policy = policies.find((item) => item.scope === body.scope && item.scope_id === body.scope_id);
      if (!policy || (role === 'member' && body.scope !== 'identity' && membershipRole !== 'coordinator')) return response(route, { error: 'forbidden' }, 403);
      policy.visibility = body.visibility;
      return response(route, policy);
    }
    return response(route, { error: `Unhandled mocked API route: ${request.method()} ${path}` }, 404);
  });

  return { calls };
}

async function signIn(page: Page, role: 'member' | 'administrator' = 'member') {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Operator sign in' })).toBeVisible();
  await page.getByLabel('Callsign').fill(role === 'administrator' ? 'K1ADM' : 'N7UF');
  await page.getByLabel('Password').fill('correct-horse-battery-staple');
  await page.getByRole('button', { name: 'SIGN IN' }).click();
  await expect(page.getByRole('heading', { name: 'Operate with your organizations.' })).toBeVisible();
}

test.describe('policy-driven operator workspace', () => {
  test('member sees only active organizations and can edit owned identity policy', async ({ page }) => {
    const { calls } = await installApiMock(page);
    await signIn(page);

    if ((page.viewportSize()?.width || 0) > 900) {
      await expect(page.getByRole('button', { name: /Cascade Radio Club/ })).toBeVisible();
    } else {
      await expect(page.getByRole('button', { name: /MY ORGANIZATIONS/ })).toBeVisible();
    }
    await expect(page.getByText('Inactive Club', { exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: 'Hardware validation' }).or(page.getByRole('button', { name: 'Server data' }))).not.toBeVisible();

    const activityButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByRole('button', { name: 'Sharing center' })
      : page.getByRole('button', { name: 'Activity' });
    await activityButton.click();
    await expect(page.getByRole('heading', { name: 'Activity and privacy' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Worked grid squares' })).toBeVisible();
    await expect(page.getByText('CN87XX', { exact: true })).toBeVisible();

    const mapControls = page.locator('.map-controls');
    await expect(mapControls.getByLabel('U.S. county boundaries')).not.toBeChecked();
    await expect(mapControls.getByLabel('Six-character grid detail')).toBeChecked();
    await expect(mapControls.getByLabel('Labels')).toBeChecked();

    const identityPolicy = page.locator('.policy-panel select').first();
    await expect(identityPolicy).toBeEnabled();
    await identityPolicy.selectOption('global');
    await expect(page.getByText('Activity policy saved.')).toBeVisible();
    expect(calls.some((call) => call.method === 'PUT' && call.path === '/api/v1/activity/visibility' && call.body?.includes('"scope":"identity"'))).toBe(true);

    const clubPolicy = page.locator('.policy-panel select').nth(2);
    await expect(clubPolicy).toBeDisabled();
  });

  test('member operations and map scopes stay limited to active club membership', async ({ page }) => {
    const { calls } = await installApiMock(page);
    await signIn(page);

    const operationsButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByLabel('Management console').getByRole('button', { name: /My operations/ })
      : page.getByLabel('Compact management navigation').getByRole('button', { name: 'Operations', exact: true });
    await operationsButton.click();
    await expect(page.getByRole('heading', { name: 'Cascade Field Day' })).toBeVisible();
    await expect(page.getByText('Inactive Club Event', { exact: true })).not.toBeVisible();

    const activityButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByRole('button', { name: 'Sharing center' })
      : page.getByRole('button', { name: 'Activity' });
    await activityButton.click();
    const mapScope = page.getByLabel('Activity scope');
    await expect(mapScope).toBeVisible();
    await mapScope.selectOption(`identity:${identity.id}`);
    await expect.poll(() => calls.some((call) => call.path === '/api/v1/activity/map' && call.url.includes(`scope=identity&scope_id=${identity.id}`))).toBe(true);
    await mapScope.selectOption(`event:${event.id}`);
    await expect.poll(() => calls.some((call) => call.path === '/api/v1/activity/map' && call.url.includes(`scope=event&scope_id=${event.id}`))).toBe(true);
  });

  test('organization policy owners can edit club and event activity policies', async ({ page }) => {
    const { calls } = await installApiMock(page, 'member', 'coordinator');
    await signIn(page);

    const activityButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByRole('button', { name: 'Sharing center' })
      : page.getByRole('button', { name: 'Activity' });
    await activityButton.click();
    await expect(page.getByRole('heading', { name: 'Who controls each activity stream?' })).toBeVisible();

    const policySelects = page.locator('.policy-panel select');
    await expect(policySelects.nth(2)).toBeEnabled();
    await expect(policySelects.nth(3)).toBeEnabled();
    await policySelects.nth(2).selectOption('global');
    await policySelects.nth(3).selectOption('global');
    await expect.poll(() => calls.filter((call) => call.method === 'PUT' && call.path === '/api/v1/activity/visibility').length).toBe(2);
  });

  test('operator can inspect identities and reach the station workspace', async ({ page }) => {
    await installApiMock(page);
    await signIn(page);

    const identitiesButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByLabel('Management console').getByRole('button', { name: /My identities/ })
      : page.getByLabel('Compact management navigation').getByRole('button', { name: 'Identities', exact: true });
    await identitiesButton.click();
    await expect(page.getByRole('heading', { name: 'Callsign registry' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'N7UF' }).first()).toBeVisible();
    await expect(page.getByRole('heading', { name: 'N7UF' })).toBeVisible();
    await expect(page.getByText('Personal operating identity tied to this operator account.')).toBeVisible();

    const stationButton = (page.viewportSize()?.width || 0) > 900
      ? page.getByRole('button', { name: 'My Stations' })
      : page.getByRole('button', { name: 'Home' });
    await stationButton.click();
    if ((page.viewportSize()?.width || 0) <= 900) await page.getByRole('button', { name: 'CONNECT A STATION' }).click();
    await expect(page.getByText('No station has reported in yet.')).toBeVisible();
  });

  test('administrator gets server data controls while member-facing policy controls stay scoped', async ({ page }) => {
    await installApiMock(page, 'administrator');
    await signIn(page, 'administrator');

    const serverDataButton = page.getByRole('button', { name: 'Hardware validation' }).or(page.getByRole('button', { name: 'Server data' }));
    await expect(serverDataButton).toBeVisible();
    await serverDataButton.click();
    await expect(page.getByRole('heading', { name: 'Hardware validation' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'PURGE ALL SUBMITTED REPORTS' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'QSO logs' })).not.toBeVisible();
    await expect(page.getByText('Submitted QSO logs and automation traffic are retained separately')).toBeVisible();
  });
});
