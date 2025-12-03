use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CredentialProperty {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub property_type: String, // text, password, etc.
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntegrationDefinition {
    pub name: String,
    pub description: String,
    pub credentials: Vec<CredentialProperty>,
}
