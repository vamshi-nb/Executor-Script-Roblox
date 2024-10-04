use actix_web::{
    http::header::{ACCEPT, LOCATION},
    web, HttpRequest, HttpResponse, Responder,
};
use rusty_s3::{actions::GetObject, S3Action};
use semver::Version;
use serde::{Deserialize, Deserializer};

use pesde::{
    manifest::target::TargetKind,
    names::PackageName,
    source::{git_index::GitBasedSource, pesde::IndexFile},
};

use crate::{
    error::Error,
    package::{s3_name, PackageResponse, S3_SIGN_DURATION},
    AppState,
};

#[derive(Debug)]
pub enum VersionRequest {
    Latest,
    Specific(Version),
}

impl<'de> Deserialize<'de> for VersionRequest {
    fn deserialize<D>(deserializer: D) -> Result<VersionRequest, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.eq_ignore_ascii_case("latest") {
            return Ok(VersionRequest::Latest);
        }

        s.parse()
            .map(VersionRequest::Specific)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum TargetRequest {
    Any,
    Specific(TargetKind),
}

impl<'de> Deserialize<'de> for TargetRequest {
    fn deserialize<D>(deserializer: D) -> Result<TargetRequest, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.eq_ignore_ascii_case("any") {
            return Ok(TargetRequest::Any);
        }

        s.parse()
            .map(TargetRequest::Specific)
            .map_err(serde::de::Error::custom)
    }
}

pub async fn get_package_version(
    request: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(PackageName, VersionRequest, TargetRequest)>,
) -> Result<impl Responder, Error> {
    let (name, version, target) = path.into_inner();

    let (scope, name_part) = name.as_str();

    let entries: IndexFile = {
        let source = app_state.source.lock().unwrap();

        match source.read_file([scope, name_part], &app_state.project, None)? {
            Some(versions) => toml::de::from_str(&versions)?,
            None => return Ok(HttpResponse::NotFound().finish()),
        }
    };

    let Some((v_id, entry, targets)) = ({
        let version = match version {
            VersionRequest::Latest => match entries.keys().map(|k| k.version()).max() {
                Some(latest) => latest.clone(),
                None => return Ok(HttpResponse::NotFound().finish()),
            },
            VersionRequest::Specific(version) => version,
        };

        let versions = entries
            .iter()
            .filter(|(v_id, _)| *v_id.version() == version);

        match target {
            TargetRequest::Any => versions.clone().min_by_key(|(v_id, _)| *v_id.target()),
            TargetRequest::Specific(kind) => versions
                .clone()
                .find(|(_, entry)| entry.target.kind() == kind),
        }
        .map(|(v_id, entry)| {
            (
                v_id,
                entry,
                versions.map(|(_, entry)| (&entry.target).into()).collect(),
            )
        })
    }) else {
        return Ok(HttpResponse::NotFound().finish());
    };

    let accept = request
        .headers()
        .get(ACCEPT)
        .and_then(|accept| accept.to_str().ok())
        .and_then(|accept| match accept.to_lowercase().as_str() {
            "text/plain" => Some(true),
            "application/octet-stream" => Some(false),
            _ => None,
        });

    if let Some(readme) = accept {
        let object_url = GetObject::new(
            &app_state.s3_bucket,
            Some(&app_state.s3_credentials),
            &s3_name(&name, v_id, readme),
        )
        .sign(S3_SIGN_DURATION);

        return Ok(HttpResponse::TemporaryRedirect()
            .append_header((LOCATION, object_url.as_str()))
            .finish());
    }

    Ok(HttpResponse::Ok().json(PackageResponse {
        name: name.to_string(),
        version: v_id.version().to_string(),
        targets,
        description: entry.description.clone().unwrap_or_default(),
        published_at: entry.published_at,
        license: entry.license.clone().unwrap_or_default(),
        authors: entry.authors.clone(),
        repository: entry.repository.clone().map(|url| url.to_string()),
    }))
}
