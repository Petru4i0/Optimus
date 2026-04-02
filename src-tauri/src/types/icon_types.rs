use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IconBinaryDto {
    pub(crate) icon_key: String,
    pub(crate) content_type: String,
    pub(crate) bytes: Vec<u8>,
}

