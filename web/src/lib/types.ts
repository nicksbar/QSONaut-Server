export type User = {
  id: string;
  callsign: string;
  display_name: string;
  global_role: string;
};

export type DeviceAuthorizationApproval = {
  device_name: string;
  client_id: string;
  client_version: string;
  expires_at: string;
};

export type GovernancePosition = {
  id: string;
  club_id: string;
  name: string;
  position_type: 'officer' | 'board';
  seats: number;
  term_years: number;
  election_parity: 'any' | 'even' | 'odd';
  description: string;
};

export type GovernanceAssignment = {
  id: string;
  position_id: string;
  user_id: string;
  callsign: string;
  display_name: string;
  seat_number: number;
  starts_on: string;
  ends_on: string;
  selection_method: 'elected' | 'appointed' | 'acting';
};

export type GovernanceElection = {
  id: string;
  club_id: string;
  title: string;
  election_year: number;
  status: 'planned' | 'nominations' | 'voting' | 'closed' | 'certified' | 'cancelled';
  opens_at: string | null;
  closes_at: string | null;
  notes: string;
  position_ids: string[];
};

export type Governance = {
  positions: GovernancePosition[];
  assignments: GovernanceAssignment[];
  elections: GovernanceElection[];
};
export type ServerCapabilities = {
  edition: string;
  max_clubs: number | null;
  features: string[];
};

export type ServerBillingSettings = {
  platform_fee_basis_points: number;
  donations_enabled: boolean;
  club_dues_enabled: boolean;
  currency: string;
  provider_status: 'not_configured' | 'sandbox' | 'live';
  updated_by: string | null;
  updated_at: string;
};

export type DeploymentReadiness = {
  secure_cookies: boolean;
  bind_address: string;
  websocket_path: string;
  database: string;
  private_migrations: string;
  online_map_tiles: boolean;
  payment_provider: string;
  email_provider: string;
  oauth_provider: string;
};

export type HostedChangeRecord = {
  id: string;
  actor_user_id: string;
  action: string;
  resource_type: string;
  resource_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
};

export type PaymentFlowStatus = {
  provider_status: string;
  checkout_enabled: boolean;
  donations_enabled: boolean;
  club_dues_enabled: boolean;
  platform_fee_basis_points: number;
  currency: string;
  gates: { key: string; status: string; label: string }[];
  blockers: { code: string; message: string }[];
  offers: { key: string; purpose: string; label: string; ownership: string; description: string }[];
};

export type PaymentEntitlement = {
  id: string;
  capability_key: string;
  club_id: string | null;
  status: string;
  starts_at: string;
  ends_at: string | null;
};

export type PaymentOperationsSummary = {
  blocked_intents: number;
  pending_intents: number;
  succeeded_intents: number;
  unprocessed_provider_events: number;
  ready_club_destinations: number;
  active_entitlements: number;
};

export type ClubPaymentMethod = {
  id: string;
  club_id: string;
  provider: 'paypal' | 'amazon_pay' | 'zelle' | 'external_link' | 'offline' | 'other';
  method_kind: 'automated' | 'external_link' | 'manual';
  display_label: string;
  public_reference: string | null;
  payment_url: string | null;
  instructions: string;
  status: 'not_configured' | 'pending_verification' | 'ready' | 'disabled';
  is_default: boolean;
  created_at: string;
  updated_at: string;
};

export type PaymentIntent = {
  id: string;
  payer_user_id: string;
  club_id: string | null;
  payment_destination_id: string | null;
  purpose: string;
  offer_key: string;
  amount_minor: number;
  currency: string;
  platform_fee_minor: number;
  club_amount_minor: number;
  status: 'blocked_provider_not_configured' | 'created' | 'pending' | 'succeeded' | 'failed' | 'cancelled' | 'refunded';
  created_at: string;
  updated_at: string;
};

export type PaymentStatusEvent = {
  id: string;
  previous_status: string | null;
  next_status: string;
  source: 'system' | 'member' | 'manager' | 'provider' | 'migration';
  actor_user_id: string | null;
  note: string;
  created_at: string;
};

export type ClubPaymentRecord = {
  id: string;
  payer_user_id: string;
  callsign: string;
  display_name: string;
  payment_destination_id: string;
  method_label: string;
  method_kind: string;
  amount_minor: number;
  currency: string;
  platform_fee_minor: number;
  club_amount_minor: number;
  status: string;
  created_at: string;
  reconciliation_outcome: string | null;
  reconciliation_reference: string | null;
  reconciliation_note: string | null;
  mark_dues_current: boolean | null;
  reconciled_at: string | null;
};

export type AccessChallenge = {
  id: string;
  question: string;
  expires_at: string;
  attempts_remaining: number;
};

export type AccessCallsignLookup = {
  callsign: string;
  display_name: string;
  grid: string;
  license_class: string;
  license_status: string;
};

export type AccessRequest = {
  id: string;
  callsign: string;
  email: string;
  club_name: string;
  referral_source: string;
  hamdb_display_name: string;
  hamdb_grid: string;
  hamdb_license_class: string;
  hamdb_license_status: string;
  status: string;
  reviewed_at: string | null;
  reviewed_by: string | null;
  created_at: string;
};

export type UserProfile = {
  user: User;
  grid: string;
  qth: string;
  first_name: string;
  middle_name: string;
  surname: string;
  suffix: string;
  license_class: string;
  license_status: string;
  license_expires_on: string | null;
  address_line_1: string;
  address_line_2: string;
  state: string;
  postal_code: string;
  country: string;
  latitude: string;
  longitude: string;
  hamdb_fetched_at: string | null;
  hamdb_last_error: string;
};

export type ActivitySummary = {
  scope: string;
  scope_id: string | null;
  period_days: number;
  qso_count: number;
  unique_callsigns: number;
  band_count: number;
  mode_count: number;
  points: number;
  last_qso_at: string | null;
  status: string;
};

export type ActivityVisibility = {
  id: string;
  scope: 'identity' | 'club' | 'event';
  scope_id: string;
  visibility: 'private' | 'members' | 'global';
  updated_by_user_id: string;
  can_edit: boolean;
  updated_at: string;
};

export type ActivityMapPoint = {
  grid: string;
  latitude: number;
  longitude: number;
  qso_count: number;
  last_qso_at: string;
};

export type HostedMapConfig = {
  online_tiles: boolean;
  tile_url: string | null;
  attribution: string | null;
  max_zoom: number;
};

export type ShareLinkRecord = {
  id: string;
  qso_log_id: string;
  occurred_at: string;
  worked_callsign: string;
  expires_at: string;
  revoked_at: string | null;
  created_at: string;
};

export type DeviceToken = {
  token: string;
  user: User;
  expires_at: string;
  scopes: string[];
};

export type DeviceTokenRecord = {
  id: string;
  device_name: string;
  expires_at: string;
  last_used_at: string | null;
  created_at: string;
};

export type Club = {
  id: string;
  name: string;
  callsign: string | null;
  description: string;
  member_count: number;
  renewal_attention_count: number;
  my_role: string | null;
  my_membership_status: string | null;
  join_request_status: string | null;
  can_manage: boolean;
};

export type ClubMember = {
  club_id: string;
  user_id: string;
  callsign: string;
  display_name: string;
  role: string;
  membership_status: string;
  dues_status: string;
  membership_number: string | null;
  renewal_due_on: string | null;
  joined_at: string;
};

export type ClubJoinRequest = {
  id: string;
  club_id: string;
  user_id: string;
  callsign: string;
  display_name: string;
  status: string;
  requested_at: string;
  reviewed_at: string | null;
  reviewed_by: string | null;
};

export type Membership = {
  club_id: string;
  club_name: string;
  club_callsign: string | null;
  role: string;
};

export type Station = {
  id: string;
  user_id: string;
  callsign: string;
  display_name: string;
  instance_id: string;
  station_label: string;
  radio_manufacturer: string | null;
  radio_model: string | null;
  frequency_hz: number | null;
  band: string | null;
  mode: string | null;
  qsonaut_version: string;
  platform: string;
  status: string;
  metadata: Record<string, unknown>;
  last_seen: string;
};

export type DiagnosticReport = {
  id: string;
  user_id: string;
  operator_callsign: string;
  instance_id: string;
  category: string;
  summary: string;
  payload: Record<string, unknown>;
  created_at: string;
};

export type MemberDetail = {
  user: User;
  memberships: Membership[];
  stations: Station[];
};

export type ContestTemplate = {
  id: string;
  contest_type: string;
  name: string;
  description: string;
  organization: string;
  icon: string;
  rules_url: string;
  definition_version: number;
  is_builtin: boolean;
  scoring_rules: Record<string, unknown>;
  required_fields: Record<string, { required?: boolean; options?: string[]; description?: string }>;
  validation_rules: {
    bands?: string[];
    modes?: string[];
    duplicateRule?: string;
    exchange?: { sent?: string[]; received?: string[] };
  };
  schedule: Record<string, unknown>;
};

export type Event = {
  id: string;
  club_id: string;
  name: string;
  contest_name: string;
  special_callsign: string | null;
  starts_at: string;
  ends_at: string;
  status: string;
  contest_template_id: string | null;
  contest_definition_version: number | null;
  contest_config: Record<string, unknown>;
  participant_count: number;
};

export type ManagedCallsign = {
  id: string;
  callsign: string;
  identity_type: 'personal' | 'club' | 'special';
  owner_user_id: string | null;
  club_id: string | null;
  event_id: string | null;
  status: string;
  effective_from: string;
  expires_at: string | null;
  verification_status: string;
  authority: string;
};

export type EventParticipant = {
  id: string;
  event_id: string;
  user_id: string;
  club_id: string;
  callsign_id: string;
  operator_callsign: string;
  operating_callsign: string;
  role: string;
  status: string;
  starts_at: string | null;
  ends_at: string | null;
  station_label: string;
  band: string | null;
  mode: string | null;
};

export type EventScore = {
  event_id: string;
  total_points: number;
  qso_count: number;
  duplicate_count: number;
  multiplier_values: Record<string, unknown>;
};

export type QsoLog = {
  id: string;
  user_id: string;
  operator_callsign: string;
  operating_callsign: string | null;
  callsign_id: string | null;
  event_id: string | null;
  event_name: string | null;
  callsign: string;
  band: string;
  mode: string;
  frequency_hz: number | null;
  occurred_at: string;
  rst_sent: string | null;
  rst_received: string | null;
  exchange: Record<string, unknown>;
  points: number;
  source: string;
};

export type SharedQsoDetail = {
  operator_callsign: string;
  event_name: string | null;
  callsign: string;
  band: string;
  mode: string;
  frequency_hz: number | null;
  occurred_at: string;
  rst_sent: string | null;
  rst_received: string | null;
  exchange: Record<string, unknown>;
  points: number;
  source: string;
};

export type ChannelMessage = {
  id: string;
  user_id: string;
  author_callsign: string;
  event_id: string | null;
  channel: string;
  message: string;
  metadata: Record<string, unknown>;
  created_at: string;
};
