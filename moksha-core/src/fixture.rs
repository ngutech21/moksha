//! This module defines helper functions for loading fixtures in tests.
//!
//! The `read_fixture` function reads a fixture file from the `src/fixtures` directory relative to the Cargo manifest directory. The function takes a `name` argument that specifies the name of the fixture file to read. The function returns a `Result` containing the contents of the fixture file as a `String`.
//!
//! The `read_fixture_as` function is a generic function that reads a fixture file and deserializes its contents into a value of type `T`. The function takes a `name` argument that specifies the name of the fixture file to read, and a type parameter `T` that specifies the type to deserialize the fixture contents into. The function returns a `Result` containing the deserialized value.
//!
//! Both functions return an `anyhow::Result`, which allows for easy error handling using the `?` operator. The functions are intended to be used in tests to load fixture data for testing purposes.
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
