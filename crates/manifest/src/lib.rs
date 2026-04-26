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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
