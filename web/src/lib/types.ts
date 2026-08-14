export type User = {
  id: string;
  callsign: string;
  display_name: string;
  global_role: string;
};

export type Club = {
  id: string;
  name: string;
  callsign: string | null;
  description: string;
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
};

export type QsoLog = {
  id: string;
  user_id: string;
  operator_callsign: string;
  event_id: string | null;
  event_name: string | null;
  callsign: string;
  band: string;
  mode: string;
  frequency_hz: number | null;
  occurred_at: string;
  points: number;
  source: string;
};
