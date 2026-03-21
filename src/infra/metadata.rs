use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct PackageMetadata {
    #[allow(dead_code)]
    pub name: String,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
    pub npm_url: Option<String>,
}

#[derive(Deserialize)]
struct NpmRegistryResponse {
    repository: Option<NpmRepository>,
    homepage: Option<String>,
}

#[derive(Deserialize)]
struct NpmRepository {
    url: Option<String>,
}

/// Fetch package metadata from the npm registry.
/// Works for both npm and bun projects since the registry is the same.
pub async fn fetch_metadata(
    client: &reqwest::Client,
    package_name: &str,
) -> Result<PackageMetadata> {
    let url = format!("https://registry.npmjs.org/{}/latest", package_name);

    let resp: NpmRegistryResponse = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("failed to fetch registry metadata for {}", package_name))?
        .json()
        .await
        .with_context(|| format!("failed to parse registry metadata for {}", package_name))?;

    Ok(PackageMetadata {
        name: package_name.to_string(),
        repository_url: resp.repository.and_then(|r| r.url),
        homepage_url: resp.homepage,
        npm_url: Some(format!("https://www.npmjs.com/package/{}", package_name)),
    })
}
