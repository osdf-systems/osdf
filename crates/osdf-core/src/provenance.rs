use serde::{Deserialize, Serialize};

pub const TRANSFORM_RECEIPT_TYPE: &str = "osdf.transform.v1";
pub const INSPECTION_RESULT_TYPE: &str = "osdf.inspection.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformReceipt {
    #[serde(default = "default_transform_receipt_type")]
    pub receipt_type: String,
    #[serde(default = "default_receipt_version")]
    pub receipt_version: String,
    pub source_package_digest: String,
    pub output_package_digest: String,
    pub generated_at: String,
    pub transformer: TransformActor,
    pub policy_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<ObjectTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransformActor {
    pub actor_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub software_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTransform {
    pub source_object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_object: Option<String>,
    pub action: String,
    pub source_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scanner_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectionResult {
    #[serde(default = "default_inspection_result_type")]
    pub result_type: String,
    pub object_id: String,
    pub object_path: String,
    pub scanner_id: String,
    pub scanner_version: String,
    pub verdict: InspectionVerdict,
    pub inspected_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<InspectionFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InspectionVerdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectionFinding {
    pub code: String,
    pub severity: InspectionVerdict,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

fn default_transform_receipt_type() -> String {
    TRANSFORM_RECEIPT_TYPE.to_string()
}

fn default_inspection_result_type() -> String {
    INSPECTION_RESULT_TYPE.to_string()
}

fn default_receipt_version() -> String {
    "1".to_string()
}
