use serde::{Deserialize, Deserializer, Serializer};
use url::Url;

pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str: Option<String> = Option::deserialize(deserializer)?;
    url_str.map_or_else(
        || Ok(None),
        |s| Url::parse(&s).map_err(serde::de::Error::custom).map(Some),
    )
}

pub fn serialize_url<S>(url: &Option<Url>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match url {
        Some(url) => serializer.serialize_str(url.as_str()),
        None => serializer.serialize_none(),
    }
}
