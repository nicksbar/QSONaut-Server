export type User = {
  id: string;
  callsign: string;
  display_name: string;
  global_role: string;
};

export type ServerCapabilities = {
  edition: string;
  max_clubs: number | null;
  features: string[];
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
  user_id: string;
  scope: 'overall' | 'club' | 'contest';
  scope_id: string | null;
  visibility: 'private' | 'members' | 'global';
  updated_at: string;
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
  contest_config: Record<string, unknown>;
  participant_count: number;
};

export type QsoLog = {
  id: string;
  user_id: string;
  operator_callsign: string;
  event_id: string | null;
  event_name: string | null;
  visibility: 'private' | 'global' | 'club' | 'contest';
  visibility_club_id: string | null;
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
