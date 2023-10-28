use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct MintInfoSettings {
    pub name: Option<String>,
    #[serde(default)]
    pub version: bool,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
}
