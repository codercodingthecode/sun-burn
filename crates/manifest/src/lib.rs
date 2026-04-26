use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("validation error: {0}")]
    Validation(String),
}

// ── FieldType ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldType {
    Text,
    Password,
    WifiPicker,
    SshKeyPicker,
    CountryPicker,
    TimezonePicker,
    Toggle,
    Select,
}

// ── Supporting types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowWhen {
    pub field: String,
    pub value: String,
}

// ── Core schema types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub show_when: Option<ShowWhen>,
    #[serde(default)]
    pub options: Option<Vec<SelectOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRule {
    /// Path relative to the boot partition root.
    pub path: String,
    /// Template string with `{{field_id}}` placeholders.
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub fields: Vec<Field>,
    pub writes: Vec<WriteRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<Step>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a `sunburn.json` string into a [`Manifest`].
pub fn parse(json: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = serde_json::from_str(json)?;
    validate(&manifest)?;
    Ok(manifest)
}

/// Validate a parsed [`Manifest`] for semantic correctness.
pub fn validate(manifest: &Manifest) -> Result<(), ManifestError> {
    if manifest.version != "1" {
        return Err(ManifestError::Validation(format!(
            "unsupported manifest version: {}",
            manifest.version
        )));
    }

    if manifest.name.is_empty() {
        return Err(ManifestError::Validation("manifest name must not be empty".into()));
    }

    // Collect all field IDs across all steps for show_when validation.
    let all_field_ids: std::collections::HashSet<&str> = manifest
        .steps
        .iter()
        .flat_map(|s| s.fields.iter().map(|f| f.id.as_str()))
        .collect();

    for step in &manifest.steps {
        if step.id.is_empty() {
            return Err(ManifestError::Validation("step id must not be empty".into()));
        }

        for field in &step.fields {
            if field.id.is_empty() {
                return Err(ManifestError::Validation(format!(
                    "field in step '{}' has an empty id",
                    step.id
                )));
            }

            // select fields must have options
            if field.field_type == FieldType::Select && field.options.is_none() {
                return Err(ManifestError::Validation(format!(
                    "field '{}' has type 'select' but no options",
                    field.id
                )));
            }

            // show_when must reference a known field
            if let Some(sw) = &field.show_when {
                if !all_field_ids.contains(sw.field.as_str()) {
                    return Err(ManifestError::Validation(format!(
                        "field '{}' show_when references unknown field '{}'",
                        field.id, sw.field
                    )));
                }
            }
        }

        for rule in &step.writes {
            if rule.path.is_empty() {
                return Err(ManifestError::Validation(format!(
                    "write rule in step '{}' has an empty path",
                    step.id
                )));
            }
        }
    }

    Ok(())
}

// ── Template substitution (public for tests) ──────────────────────────────────

/// Replace all `{{key}}` tokens in `template` using the provided `values` map.
pub fn substitute(template: &str, values: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── helpers ───────────────────────────────────────────────────────────────

    const SAMPLE: &str = r#"{
        "version": "1",
        "name": "Solar Monitor",
        "description": "Home solar monitoring stack",
        "steps": [
            {
                "id": "network",
                "title": "Network Setup",
                "fields": [
                    { "id": "ssid", "type": "wifi-picker", "label": "WiFi Network", "required": true },
                    { "id": "password", "type": "password", "label": "Password", "required": true },
                    { "id": "country", "type": "country-picker", "label": "Country", "required": false, "default": "US" }
                ],
                "writes": [
                    { "path": "wifi.txt", "template": "ssid={{ssid}}\npassword={{password}}\ncountry={{country}}" }
                ]
            }
        ]
    }"#;

    fn minimal_manifest(extra_fields: &str) -> String {
        format!(
            r#"{{
                "version": "1",
                "name": "Test",
                "steps": [{{
                    "id": "s1",
                    "title": "Step One",
                    "fields": [{}],
                    "writes": []
                }}]
            }}"#,
            extra_fields
        )
    }

    // ── existing tests (kept) ─────────────────────────────────────────────────

    #[test]
    fn parse_sample() {
        let m = parse(SAMPLE).unwrap();
        assert_eq!(m.name, "Solar Monitor");
        assert_eq!(m.steps.len(), 1);
        assert_eq!(m.steps[0].fields.len(), 3);
        assert_eq!(m.steps[0].fields[0].field_type, FieldType::WifiPicker);
    }

    #[test]
    fn bad_version_rejected() {
        let bad = SAMPLE.replace(r#""version": "1""#, r#""version": "99""#);
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn show_when_unknown_field_rejected() {
        let bad = r#"{
            "version": "1",
            "name": "Test",
            "steps": [{
                "id": "s1",
                "title": "T",
                "fields": [
                    { "id": "f1", "type": "toggle", "label": "L", "required": false,
                      "show_when": { "field": "nonexistent", "value": "true" } }
                ],
                "writes": []
            }]
        }"#;
        assert!(parse(bad).is_err());
    }

    #[test]
    fn select_without_options_rejected() {
        let bad = r#"{
            "version": "1",
            "name": "Test",
            "steps": [{
                "id": "s1",
                "title": "T",
                "fields": [
                    { "id": "f1", "type": "select", "label": "L", "required": false }
                ],
                "writes": []
            }]
        }"#;
        assert!(parse(bad).is_err());
    }

    // ── new tests ─────────────────────────────────────────────────────────────

    #[test]
    fn valid_manifest_all_field_types_parses() {
        let json = r#"{
            "version": "1",
            "name": "All Fields",
            "description": "exercises every field type",
            "steps": [{
                "id": "step1",
                "title": "All Types",
                "fields": [
                    { "id": "f_text",     "type": "text",           "label": "Text"     },
                    { "id": "f_pass",     "type": "password",       "label": "Pass"     },
                    { "id": "f_wifi",     "type": "wifi-picker",    "label": "Wifi"     },
                    { "id": "f_ssh",      "type": "ssh-key-picker", "label": "SSH"      },
                    { "id": "f_country",  "type": "country-picker", "label": "Country"  },
                    { "id": "f_tz",       "type": "timezone-picker","label": "TZ"       },
                    { "id": "f_toggle",   "type": "toggle",         "label": "Toggle"   },
                    { "id": "f_select",   "type": "select",         "label": "Select",
                      "options": [{"value":"a","label":"A"},{"value":"b","label":"B"}] }
                ],
                "writes": []
            }]
        }"#;
        let m = parse(json).unwrap();
        let fields = &m.steps[0].fields;
        assert_eq!(fields[0].field_type, FieldType::Text);
        assert_eq!(fields[1].field_type, FieldType::Password);
        assert_eq!(fields[2].field_type, FieldType::WifiPicker);
        assert_eq!(fields[3].field_type, FieldType::SshKeyPicker);
        assert_eq!(fields[4].field_type, FieldType::CountryPicker);
        assert_eq!(fields[5].field_type, FieldType::TimezonePicker);
        assert_eq!(fields[6].field_type, FieldType::Toggle);
        assert_eq!(fields[7].field_type, FieldType::Select);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse("not json at all {{{").is_err());
        assert!(parse("").is_err());
        assert!(parse("null").is_err());
    }

    #[test]
    fn parse_rejects_unknown_version() {
        for v in &["0", "2", "99", "1.0", ""] {
            let bad = format!(
                r#"{{"version":"{v}","name":"T","steps":[{{"id":"s","title":"T","fields":[],"writes":[]}}]}}"#
            );
            assert!(parse(&bad).is_err(), "version '{}' should be rejected", v);
        }
    }

    #[test]
    fn parse_rejects_empty_step_id() {
        let bad = r#"{
            "version": "1",
            "name": "Test",
            "steps": [{
                "id": "",
                "title": "Step",
                "fields": [],
                "writes": []
            }]
        }"#;
        let err = parse(bad).unwrap_err();
        assert!(err.to_string().contains("step id"));
    }

    #[test]
    fn parse_rejects_empty_field_id() {
        let bad = minimal_manifest(
            r#"{ "id": "", "type": "text", "label": "Oops" }"#,
        );
        let err = parse(&bad).unwrap_err();
        assert!(err.to_string().contains("empty id"));
    }

    #[test]
    fn select_with_options_passes() {
        let good = r#"{
            "version": "1",
            "name": "Test",
            "steps": [{
                "id": "s1",
                "title": "T",
                "fields": [
                    { "id": "f1", "type": "select", "label": "L",
                      "options": [{"value":"x","label":"X"}] }
                ],
                "writes": []
            }]
        }"#;
        assert!(parse(good).is_ok());
    }

    #[test]
    fn show_when_valid_reference_passes() {
        let good = r#"{
            "version": "1",
            "name": "Test",
            "steps": [{
                "id": "s1",
                "title": "T",
                "fields": [
                    { "id": "toggle1", "type": "toggle", "label": "Enable" },
                    { "id": "dep",     "type": "text",   "label": "Extra",
                      "show_when": { "field": "toggle1", "value": "true" } }
                ],
                "writes": []
            }]
        }"#;
        assert!(parse(good).is_ok());
    }

    #[test]
    fn field_type_serde_round_trip() {
        let types = vec![
            FieldType::WifiPicker,
            FieldType::SshKeyPicker,
            FieldType::CountryPicker,
            FieldType::TimezonePicker,
            FieldType::Toggle,
            FieldType::Select,
            FieldType::Text,
            FieldType::Password,
        ];
        for ft in types {
            let serialized = serde_json::to_string(&ft).unwrap();
            let deserialized: FieldType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(ft, deserialized, "round-trip failed for {:?}", ft);
        }
    }

    #[test]
    fn field_type_kebab_case_serialization() {
        assert_eq!(serde_json::to_string(&FieldType::WifiPicker).unwrap(),    r#""wifi-picker""#);
        assert_eq!(serde_json::to_string(&FieldType::SshKeyPicker).unwrap(),  r#""ssh-key-picker""#);
        assert_eq!(serde_json::to_string(&FieldType::CountryPicker).unwrap(), r#""country-picker""#);
        assert_eq!(serde_json::to_string(&FieldType::TimezonePicker).unwrap(),r#""timezone-picker""#);
        assert_eq!(serde_json::to_string(&FieldType::Toggle).unwrap(),        r#""toggle""#);
        assert_eq!(serde_json::to_string(&FieldType::Select).unwrap(),        r#""select""#);
    }

    #[test]
    fn optional_description_absent_still_parses() {
        // SAMPLE has description; this one intentionally omits it
        let json = r#"{
            "version": "1",
            "name": "No Description",
            "steps": [{
                "id": "s1",
                "title": "Only Step",
                "fields": [],
                "writes": []
            }]
        }"#;
        let m = parse(json).unwrap();
        assert!(m.description.is_none());
        assert_eq!(m.name, "No Description");
    }

    #[test]
    fn multi_step_cross_step_show_when_passes() {
        // show_when can reference a field in any step (all IDs collected globally)
        let json = r#"{
            "version": "1",
            "name": "Multi",
            "steps": [
                {
                    "id": "step_a",
                    "title": "A",
                    "fields": [
                        { "id": "flag", "type": "toggle", "label": "Flag" }
                    ],
                    "writes": []
                },
                {
                    "id": "step_b",
                    "title": "B",
                    "fields": [
                        { "id": "dep", "type": "text", "label": "Dep",
                          "show_when": { "field": "flag", "value": "true" } }
                    ],
                    "writes": []
                }
            ]
        }"#;
        assert!(parse(json).is_ok());
    }

    #[test]
    fn template_substitution_all_tokens_replaced() {
        let mut values = HashMap::new();
        values.insert("ssid".into(), "HomeNet".into());
        values.insert("password".into(), "hunter2".into());
        values.insert("country".into(), "DE".into());

        let template = "ssid={{ssid}}\npassword={{password}}\ncountry={{country}}";
        let result = substitute(template, &values);
        assert_eq!(result, "ssid=HomeNet\npassword=hunter2\ncountry=DE");
    }

    #[test]
    fn template_substitution_unknown_tokens_left_intact() {
        let values: HashMap<String, String> = HashMap::new();
        let result = substitute("key={{missing}}", &values);
        assert_eq!(result, "key={{missing}}");
    }

    #[test]
    fn template_substitution_partial_replacement() {
        let mut values = HashMap::new();
        values.insert("known".into(), "VALUE".into());
        let result = substitute("{{known}} and {{unknown}}", &values);
        assert_eq!(result, "VALUE and {{unknown}}");
    }
}
