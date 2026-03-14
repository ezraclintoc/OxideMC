use crate::config::{read_oxide_config, write_oxide_config};
use crate::download::download_url;
use cliclack::input;
use ferinth::structures::{
    project::ProjectType,
    search::{Facet, SearchHit, Sort},
};
use std::env;
use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;

pub async fn search_modrinth(
    query: &str,
    project_type: ProjectType,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<SearchHit>, Box<dyn Error>> {
    let client = ferinth::Ferinth::<()>::new("OxideMC", Some(env!("CARGO_PKG_VERSION")), None);
    let mut facets: Vec<Vec<Facet>> = vec![vec![Facet::ProjectType(project_type)]];
    if let Some(l) = loader {
        facets.push(vec![Facet::Categories(l.to_string())]);
    }
    if let Some(gv) = game_version {
        facets.push(vec![Facet::Versions(gv.to_string())]);
    }
    let response = client.search(query, &Sort::Relevance, facets).await?;
    Ok(response.hits)
}

pub async fn install_modrinth(
    project_id: &str,
    loader: Option<&str>,
    game_version: &str,
    install_dir: &PathBuf,
) -> Result<String, Box<dyn Error>> {
    let client = ferinth::Ferinth::<()>::new("OxideMC", Some(env!("CARGO_PKG_VERSION")), None);
    let versions = client.version_list(project_id).await?;

    // Try best match: game version + loader, then fall back to loader only
    let best = versions
        .iter()
        .find(|v| {
            v.game_versions.iter().any(|gv| gv == game_version)
                && loader.map_or(true, |l| v.loaders.iter().any(|vl| vl == l))
        })
        .or_else(|| {
            versions
                .iter()
                .find(|v| loader.map_or(true, |l| v.loaders.iter().any(|vl| vl == l)))
        })
        .ok_or("No compatible version found")?;

    let file = best
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| best.files.first())
        .ok_or("No downloadable file")?;

    create_dir_all(install_dir)?;
    download_url(file.url.as_str(), install_dir, &file.filename).await?;
    Ok(file.filename.clone())
}

pub async fn install_curseforge(
    mod_id: i32,
    game_version: &str,
    install_dir: &PathBuf,
    api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let client = furse::Furse::new(api_key);
    let files = client.get_mod_files(mod_id).await?;

    // Find latest compatible file (matching game_version first, then any)
    let file = files
        .iter()
        .find(|f| f.game_versions.iter().any(|gv| gv == game_version))
        .or_else(|| files.first())
        .ok_or("No files found for this CurseForge project")?;

    let file_url = if let Some(url) = &file.download_url {
        url.to_string()
    } else {
        client.file_download_url(mod_id, file.id).await?.to_string()
    };

    create_dir_all(install_dir)?;
    download_url(&file_url, install_dir, &file.file_name).await?;
    Ok(file.file_name.clone())
}

pub fn get_curseforge_key(dir: &PathBuf) -> String {
    if let Ok(key) = read_oxide_config(dir, "curseforge_api_key") {
        return key;
    }
    let key: String = input(
        "Enter your CurseForge API key (get one at console.curseforge.com/api-keys):",
    )
    .required(true)
    .interact()
    .unwrap();
    let _ = write_oxide_config(dir, "curseforge_api_key", &key);
    key
}
