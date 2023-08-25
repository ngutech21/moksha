pub fn read_fixture(name: &str) -> anyhow::Result<String> {
    let base_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let raw_token = std::fs::read_to_string(format!("{base_dir}/src/fixtures/{name}"))?;
    Ok(raw_token.trim().to_string())
}

pub fn read_fixture_as<T>(name: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    Ok(serde_json::from_str::<T>(&read_fixture(name)?)?)
}
