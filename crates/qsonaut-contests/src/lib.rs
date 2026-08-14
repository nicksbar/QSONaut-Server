//! Server-owned, versioned contest definitions.
//!
//! These definitions deliberately do not depend on the Yahaml codebase. They
//! are normalized domain data which the store publishes to `PostgreSQL`.

use qsonaut_protocol::ContestTemplate;
use serde_json::{Value, json};
use uuid::Uuid;

pub const CATALOG_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    YearRound,
    Annual {
        summary: &'static str,
        duration_hours: u16,
    },
    Manual {
        summary: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateRule {
    Band,
    BandMode,
    None,
}

impl DuplicateRule {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Band => "band",
            Self::BandMode => "band-mode",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SetupField {
    pub key: &'static str,
    pub description: &'static str,
    pub options: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct Definition {
    pub id: &'static str,
    pub contest_type: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub organization: &'static str,
    pub icon: &'static str,
    pub rules_url: &'static str,
    pub formula: &'static str,
    pub points_per_qso: u8,
    pub multiplier: Option<&'static str>,
    pub setup_fields: &'static [SetupField],
    pub bands: &'static [&'static str],
    pub modes: &'static [&'static str],
    pub duplicate_rule: DuplicateRule,
    pub exchange: &'static [&'static str],
    pub schedule: Schedule,
}

const HF: &[&str] = &["160", "80", "40", "20", "15", "10"];
const HF_NO_160: &[&str] = &["80", "40", "20", "15", "10"];
const VHF: &[&str] = &[
    "6", "2", "1.25", "70CM", "33CM", "23CM", "13CM", "6CM", "3CM", "1.25CM",
];
const FIELD_BANDS: &[&str] = &[
    "160", "80", "40", "20", "15", "10", "6", "2", "1.25", "70CM", "SAT",
];
const ACTIVATION_BANDS: &[&str] = &[
    "160", "80", "60", "40", "30", "20", "17", "15", "12", "10", "6", "2", "1.25", "70CM",
];
const MIXED: &[&str] = &["CW", "SSB", "FM", "DIGITAL"];
const FIELD_MODES: &[&str] = &["CW", "PHONE", "DIGITAL"];

const CLASS_SECTION: &[SetupField] = &[
    SetupField {
        key: "class",
        description: "Operating class, including transmitter count",
        options: &[
            "1A", "2A", "3A", "4A", "5A", "6A", "1B", "1C", "1D", "1E", "2E", "3E", "1F", "2F",
            "3F",
        ],
    },
    SetupField {
        key: "section",
        description: "ARRL or RAC section",
        options: &[],
    },
    SetupField {
        key: "power",
        description: "Power category",
        options: &["HIGH", "LOW", "QRP"],
    },
];
const WFD_FIELDS: &[SetupField] = &[
    SetupField {
        key: "class",
        description: "Transmitter count and category: Outdoor, Indoor, or Home",
        options: &["1O", "2O", "3O", "1I", "2I", "3I", "1H"],
    },
    SetupField {
        key: "section",
        description: "ARRL or RAC section",
        options: &[],
    },
];
const GRID: &[SetupField] = &[SetupField {
    key: "grid",
    description: "Your four-character Maidenhead grid square",
    options: &[],
}];
const EXCHANGE: &[SetupField] = &[SetupField {
    key: "exchange",
    description: "Exchange this station will send",
    options: &[],
}];
const PARK: &[SetupField] = &[SetupField {
    key: "park",
    description: "Activator park reference, for example US-4566",
    options: &[],
}];
const SUMMIT: &[SetupField] = &[SetupField {
    key: "summit",
    description: "Activator summit reference, for example W7W/LC-001",
    options: &[],
}];

// Stable IDs are part of the server protocol and must never be recycled.
const DEFINITIONS: &[Definition] = &[
    Definition {
        id: "10000000-0000-4000-8000-000000000001",
        contest_type: "ARRL_FD",
        name: "ARRL Field Day",
        description: "Emergency preparedness exercise and public operating event",
        organization: "ARRL",
        icon: "🎯",
        rules_url: "https://www.arrl.org/field-day",
        formula: "Mode QSO points, power multiplier, and earned bonuses",
        points_per_qso: 1,
        multiplier: Some("power"),
        setup_fields: CLASS_SECTION,
        bands: FIELD_BANDS,
        modes: FIELD_MODES,
        duplicate_rule: DuplicateRule::BandMode,
        exchange: &["class", "section"],
        schedule: Schedule::Annual {
            summary: "Fourth full weekend of June; confirm against the current rules",
            duration_hours: 27,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000002",
        contest_type: "WINTER_FD",
        name: "Winter Field Day",
        description: "Emergency communications exercise held in winter conditions",
        organization: "Winter Field Day Association",
        icon: "❄️",
        rules_url: "https://winterfieldday.org/",
        formula: "Mode QSO points plus bonuses defined by the current rules",
        points_per_qso: 1,
        multiplier: None,
        setup_fields: WFD_FIELDS,
        bands: FIELD_BANDS,
        modes: FIELD_MODES,
        duplicate_rule: DuplicateRule::BandMode,
        exchange: &["class", "section"],
        schedule: Schedule::Annual {
            summary: "Last full weekend of January; confirm against the current rules",
            duration_hours: 24,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000003",
        contest_type: "POTA",
        name: "Parks on the Air",
        description: "Year-round portable park activation and hunting program",
        organization: "POTA",
        icon: "🏞️",
        rules_url: "https://docs.pota.app/",
        formula: "Program credit is calculated under the current POTA rules",
        points_per_qso: 1,
        multiplier: None,
        setup_fields: PARK,
        bands: ACTIVATION_BANDS,
        modes: MIXED,
        duplicate_rule: DuplicateRule::None,
        exchange: &["rst", "park"],
        schedule: Schedule::YearRound,
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000004",
        contest_type: "SOTA",
        name: "Summits on the Air",
        description: "Year-round mountain summit activation and chasing program",
        organization: "SOTA",
        icon: "⛰️",
        rules_url: "https://www.sota.org.uk/Joining-In/General-Rules",
        formula: "Activator and chaser points depend on the summit under current SOTA rules",
        points_per_qso: 1,
        multiplier: None,
        setup_fields: SUMMIT,
        bands: ACTIVATION_BANDS,
        modes: MIXED,
        duplicate_rule: DuplicateRule::None,
        exchange: &["rst", "summit"],
        schedule: Schedule::YearRound,
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000005",
        contest_type: "ARRL_10M",
        name: "ARRL 10 Meter Contest",
        description: "Worldwide CW and phone competition on the 10-meter band",
        organization: "ARRL",
        icon: "☀️",
        rules_url: "https://www.arrl.org/10-meter",
        formula: "CW and phone QSO points multiplied by location multipliers",
        points_per_qso: 2,
        multiplier: Some("location"),
        setup_fields: EXCHANGE,
        bands: &["10"],
        modes: &["CW", "PHONE"],
        duplicate_rule: DuplicateRule::BandMode,
        exchange: &["rst", "stateOrSerial"],
        schedule: Schedule::Annual {
            summary: "Second full weekend of December; confirm against the current rules",
            duration_hours: 48,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000006",
        contest_type: "ARRL_DX_CW",
        name: "ARRL International DX Contest — CW",
        description: "Worldwide ARRL DX competition, CW weekend",
        organization: "ARRL",
        icon: "🌍",
        rules_url: "https://www.arrl.org/arrl-dx",
        formula: "DX QSO points multiplied by country or location multipliers",
        points_per_qso: 3,
        multiplier: Some("dxcc"),
        setup_fields: EXCHANGE,
        bands: HF,
        modes: &["CW"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["rst", "stateOrPower"],
        schedule: Schedule::Annual {
            summary: "Third full weekend of February",
            duration_hours: 48,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000007",
        contest_type: "ARRL_DX_PHONE",
        name: "ARRL International DX Contest — Phone",
        description: "Worldwide ARRL DX competition, phone weekend",
        organization: "ARRL",
        icon: "📣",
        rules_url: "https://www.arrl.org/arrl-dx",
        formula: "DX QSO points multiplied by country or location multipliers",
        points_per_qso: 3,
        multiplier: Some("dxcc"),
        setup_fields: EXCHANGE,
        bands: HF,
        modes: &["PHONE"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["rs", "stateOrPower"],
        schedule: Schedule::Annual {
            summary: "First full weekend of March",
            duration_hours: 48,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000008",
        contest_type: "ARRL_RTTY",
        name: "ARRL RTTY Roundup",
        description: "Worldwide RTTY contest",
        organization: "ARRL",
        icon: "⌨️",
        rules_url: "https://www.arrl.org/rtty-roundup",
        formula: "QSO points multiplied by state, province, and country multipliers",
        points_per_qso: 1,
        multiplier: Some("location"),
        setup_fields: EXCHANGE,
        bands: HF_NO_160,
        modes: &["RTTY"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["rst", "stateOrSerial"],
        schedule: Schedule::Annual {
            summary: "First full weekend of January, but never January 1; confirm the date",
            duration_hours: 30,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000009",
        contest_type: "ARRL_160M",
        name: "ARRL 160 Meter Contest",
        description: "CW competition on the 160-meter band",
        organization: "ARRL",
        icon: "🌙",
        rules_url: "https://www.arrl.org/160-meter",
        formula: "QSO points multiplied by section and country multipliers",
        points_per_qso: 2,
        multiplier: Some("section-country"),
        setup_fields: EXCHANGE,
        bands: &["160"],
        modes: &["CW"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["rst", "sectionOrCountry"],
        schedule: Schedule::Annual {
            summary: "First full weekend of December; confirm against the current rules",
            duration_hours: 42,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000010",
        contest_type: "ARRL_VHF",
        name: "ARRL VHF Contest (Generic)",
        description: "Manual template for an ARRL VHF event when the specific season is not known",
        organization: "ARRL",
        icon: "📡",
        rules_url: "https://www.arrl.org/vhf",
        formula: "Band-weighted QSO points multiplied by unique grid squares",
        points_per_qso: 1,
        multiplier: Some("grid"),
        setup_fields: GRID,
        bands: VHF,
        modes: MIXED,
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Manual {
            summary: "Choose the exact date from the current contest calendar",
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000011",
        contest_type: "ARRL_JAN_VHF",
        name: "ARRL January VHF Contest",
        description: "Winter VHF competition on 6 meters and above",
        organization: "ARRL",
        icon: "❄️",
        rules_url: "https://www.arrl.org/january-vhf",
        formula: "Band-weighted QSO points multiplied by unique grid squares",
        points_per_qso: 1,
        multiplier: Some("grid"),
        setup_fields: GRID,
        bands: VHF,
        modes: MIXED,
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Annual {
            summary: "Third or fourth weekend of January; use the current calendar",
            duration_hours: 33,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000012",
        contest_type: "ARRL_JUNE_VHF",
        name: "ARRL June VHF Contest",
        description: "Summer VHF competition on 6 meters and above",
        organization: "ARRL",
        icon: "🌤️",
        rules_url: "https://www.arrl.org/june-vhf",
        formula: "Band-weighted QSO points multiplied by unique grid squares",
        points_per_qso: 1,
        multiplier: Some("grid"),
        setup_fields: GRID,
        bands: VHF,
        modes: MIXED,
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Annual {
            summary: "Second full weekend of June",
            duration_hours: 33,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000013",
        contest_type: "ARRL_SEPT_VHF",
        name: "ARRL September VHF Contest",
        description: "Fall VHF competition on 6 meters and above",
        organization: "ARRL",
        icon: "🍂",
        rules_url: "https://www.arrl.org/september-vhf",
        formula: "Band-weighted QSO points multiplied by unique grid squares",
        points_per_qso: 1,
        multiplier: Some("grid"),
        setup_fields: GRID,
        bands: VHF,
        modes: MIXED,
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Annual {
            summary: "Second full weekend of September; confirm against the current rules",
            duration_hours: 33,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000014",
        contest_type: "ARRL_10GHZ_UP",
        name: "ARRL 10 GHz & Up Contest",
        description: "Microwave competition on 10 GHz and higher bands",
        organization: "ARRL",
        icon: "🛰️",
        rules_url: "https://www.arrl.org/10-ghz-and-up",
        formula: "Distance-based scoring under the current event rules",
        points_per_qso: 1,
        multiplier: None,
        setup_fields: GRID,
        bands: &[
            "10GHZ", "24GHZ", "47GHZ", "76GHZ", "122GHZ", "134GHZ", "241GHZ",
        ],
        modes: MIXED,
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Annual {
            summary: "Two weekend segments in August and September; use the current calendar",
            duration_hours: 24,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000015",
        contest_type: "ARRL_RR_CW",
        name: "ARRL Rookie Roundup — CW",
        description: "Short CW event centered on newly licensed operators",
        organization: "ARRL",
        icon: "🧭",
        rules_url: "https://www.arrl.org/rookie-roundup",
        formula: "QSO points and rookie multipliers under the current rules",
        points_per_qso: 1,
        multiplier: Some("rookie"),
        setup_fields: EXCHANGE,
        bands: HF_NO_160,
        modes: &["CW"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["name", "check", "state"],
        schedule: Schedule::Manual {
            summary: "This mode is not held every year; use the current ARRL calendar",
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000016",
        contest_type: "ARRL_RR_PHONE",
        name: "ARRL Rookie Roundup — Phone",
        description: "Short phone event centered on newly licensed operators",
        organization: "ARRL",
        icon: "🎙️",
        rules_url: "https://www.arrl.org/rookie-roundup",
        formula: "QSO points and rookie multipliers under the current rules",
        points_per_qso: 1,
        multiplier: Some("rookie"),
        setup_fields: EXCHANGE,
        bands: HF_NO_160,
        modes: &["PHONE"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["name", "check", "state"],
        schedule: Schedule::Annual {
            summary: "Third Sunday of April; confirm against the current rules",
            duration_hours: 6,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000017",
        contest_type: "ARRL_RR_RTTY",
        name: "ARRL Rookie Roundup — RTTY",
        description: "Short RTTY event centered on newly licensed operators",
        organization: "ARRL",
        icon: "🖥️",
        rules_url: "https://www.arrl.org/rookie-roundup",
        formula: "QSO points and rookie multipliers under the current rules",
        points_per_qso: 1,
        multiplier: Some("rookie"),
        setup_fields: EXCHANGE,
        bands: HF_NO_160,
        modes: &["RTTY"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["name", "check", "state"],
        schedule: Schedule::Annual {
            summary: "Third Sunday of August; confirm against the current rules",
            duration_hours: 6,
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000018",
        contest_type: "ARRL_SCR",
        name: "ARRL School Club Roundup",
        description: "School-oriented operating event encouraging youth participation",
        organization: "ARRL",
        icon: "🏫",
        rules_url: "https://www.arrl.org/school-club-roundup",
        formula: "QSO points multiplied by station-class multipliers",
        points_per_qso: 1,
        multiplier: Some("station-class"),
        setup_fields: EXCHANGE,
        bands: HF_NO_160,
        modes: &["CW", "PHONE", "DIGITAL"],
        duplicate_rule: DuplicateRule::BandMode,
        exchange: &["rs", "class", "state"],
        schedule: Schedule::Manual {
            summary: "Typically February and October; use the current calendar for the exact five-day period",
        },
    },
    Definition {
        id: "10000000-0000-4000-8000-000000000019",
        contest_type: "ARRL_INTL_DIGITAL",
        name: "ARRL International Digital Contest",
        description: "Worldwide non-RTTY digital competition using grid-square exchanges",
        organization: "ARRL",
        icon: "💻",
        rules_url: "https://www.arrl.org/arrl-digital-contest",
        formula: "One QSO point plus one point per 500 km of distance, rounded up",
        points_per_qso: 1,
        multiplier: None,
        setup_fields: GRID,
        bands: &["160", "80", "40", "20", "15", "10", "6"],
        modes: &["DIGITAL_NO_RTTY"],
        duplicate_rule: DuplicateRule::Band,
        exchange: &["grid"],
        schedule: Schedule::Annual {
            summary: "First full weekend of June",
            duration_hours: 30,
        },
    },
];

fn required_fields(fields: &[SetupField]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|field| {
                let mut value = json!({ "required": true, "description": field.description });
                if !field.options.is_empty() {
                    value["options"] = json!(field.options);
                }
                (field.key.to_owned(), value)
            })
            .collect(),
    )
}

fn schedule_value(schedule: Schedule) -> Value {
    match schedule {
        Schedule::YearRound => {
            json!({ "type": "year-round", "summary": "Available year-round", "timezone": "UTC", "advisory": true })
        }
        Schedule::Annual {
            summary,
            duration_hours,
        } => {
            json!({ "type": "annual-pattern", "summary": summary, "durationHours": duration_hours, "timezone": "UTC", "advisory": true })
        }
        Schedule::Manual { summary } => {
            json!({ "type": "manual", "summary": summary, "timezone": "UTC", "advisory": true })
        }
    }
}

#[must_use]
/// Builds every server-owned contest template.
///
/// # Panics
///
/// Panics only if a developer introduces an invalid stable UUID into the
/// compile-time catalog. Catalog tests exercise every definition.
pub fn builtin_templates() -> Vec<ContestTemplate> {
    DEFINITIONS
        .iter()
        .map(|definition| ContestTemplate {
            id: Uuid::parse_str(definition.id).expect("built-in contest UUID must be valid"),
            contest_type: definition.contest_type.to_owned(),
            name: definition.name.to_owned(),
            description: definition.description.to_owned(),
            organization: definition.organization.to_owned(),
            icon: definition.icon.to_owned(),
            rules_url: definition.rules_url.to_owned(),
            definition_version: CATALOG_VERSION,
            is_builtin: true,
            scoring_rules: json!({
                "pointsPerQso": definition.points_per_qso,
                "formula": definition.formula,
                "multiplier": definition.multiplier,
            }),
            required_fields: required_fields(definition.setup_fields),
            validation_rules: json!({
                "bands": definition.bands,
                "modes": definition.modes,
                "duplicateRule": definition.duplicate_rule.as_str(),
                "exchange": { "sent": definition.exchange, "received": definition.exchange },
            }),
            schedule: schedule_value(definition.schedule),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_nineteen_unique_stable_definitions() {
        let templates = builtin_templates();
        assert_eq!(templates.len(), 19);
        assert_eq!(
            templates.iter().map(|t| t.id).collect::<HashSet<_>>().len(),
            19
        );
        assert_eq!(
            templates
                .iter()
                .map(|t| &t.contest_type)
                .collect::<HashSet<_>>()
                .len(),
            19
        );
    }

    #[test]
    fn definitions_have_safe_sources_and_advisory_schedules() {
        for template in builtin_templates() {
            assert!(template.rules_url.starts_with("https://"));
            assert_eq!(template.schedule["advisory"], true);
            assert!(
                !template.validation_rules["bands"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                !template.validation_rules["modes"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn international_digital_is_not_the_stale_yahaml_definition() {
        let template = builtin_templates()
            .into_iter()
            .find(|t| t.contest_type == "ARRL_INTL_DIGITAL")
            .unwrap();
        assert_eq!(template.schedule["summary"], "First full weekend of June");
        assert_eq!(template.required_fields["grid"]["required"], true);
        assert_eq!(
            template.validation_rules["bands"],
            json!(["160", "80", "40", "20", "15", "10", "6"])
        );
        assert_eq!(
            template.validation_rules["modes"],
            json!(["DIGITAL_NO_RTTY"])
        );
    }
}
