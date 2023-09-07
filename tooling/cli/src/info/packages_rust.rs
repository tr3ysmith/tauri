// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use super::{SectionItem, Status};
use crate::interface::rust::get_workspace_dir;
use colored::Colorize;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize)]
struct CargoLockPackage {
  name: String,
  version: String,
  source: Option<String>,
}

#[derive(Deserialize)]
struct CargoLock {
  package: Vec<CargoLockPackage>,
}

#[derive(Clone, Deserialize)]
struct CargoManifestDependencyPackage {
  version: Option<String>,
  git: Option<String>,
  branch: Option<String>,
  rev: Option<String>,
  path: Option<PathBuf>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum CargoManifestDependency {
  Version(String),
  Package(CargoManifestDependencyPackage),
}

#[derive(Deserialize)]
struct CargoManifestPackage {
  version: String,
}

#[derive(Deserialize)]
struct CargoManifest {
  package: CargoManifestPackage,
  dependencies: HashMap<String, CargoManifestDependency>,
}

fn crate_latest_version(name: &str) -> Option<String> {
  let url = format!("https://docs.rs/crate/{name}/");
  match ureq::get(&url).call() {
    Ok(response) => match (response.status(), response.header("location")) {
      (302, Some(location)) => Some(location.replace(&url, "")),
      _ => None,
    },
    Err(_) => None,
  }
}

fn crate_version(
  tauri_dir: &Path,
  manifest: Option<&CargoManifest>,
  lock: Option<&CargoLock>,
  name: &str,
) -> (String, Option<String>) {
  let crate_lock_packages: Vec<CargoLockPackage> = lock
    .as_ref()
    .map(|lock| {
      lock
        .package
        .iter()
        .filter(|p| p.name == name)
        .cloned()
        .collect()
    })
    .unwrap_or_default();
  let (crate_version_string, found_crate_versions) =
    match (&manifest, &lock, crate_lock_packages.len()) {
      (Some(_manifest), Some(_lock), 1) => {
        let crate_lock_package = crate_lock_packages.first().unwrap();
        let version_string = if let Some(s) = &crate_lock_package.source {
          if s.starts_with("git") {
            format!("{} ({})", s, crate_lock_package.version)
          } else {
            crate_lock_package.version.clone()
          }
        } else {
          crate_lock_package.version.clone()
        };
        (version_string, vec![crate_lock_package.version.clone()])
      }
      (None, Some(_lock), 1) => {
        let crate_lock_package = crate_lock_packages.first().unwrap();
        let version_string = if let Some(s) = &crate_lock_package.source {
          if s.starts_with("git") {
            format!("{} ({})", s, crate_lock_package.version)
          } else {
            crate_lock_package.version.clone()
          }
        } else {
          crate_lock_package.version.clone()
        };
        (
          format!("{version_string} (no manifest)"),
          vec![crate_lock_package.version.clone()],
        )
      }
      _ => {
        let mut found_crate_versions = Vec::new();
        let mut is_git = false;
        let manifest_version = match manifest.and_then(|m| m.dependencies.get(name).cloned()) {
          Some(tauri) => match tauri {
            CargoManifestDependency::Version(v) => {
              found_crate_versions.push(v.clone());
              v
            }
            CargoManifestDependency::Package(p) => {
              if let Some(v) = p.version {
                found_crate_versions.push(v.clone());
                v
              } else if let Some(p) = p.path {
                let manifest_path = tauri_dir.join(&p).join("Cargo.toml");
                let v = match read_to_string(manifest_path)
                  .map_err(|_| ())
                  .and_then(|m| toml::from_str::<CargoManifest>(&m).map_err(|_| ()))
                {
                  Ok(manifest) => manifest.package.version,
                  Err(_) => "unknown version".to_string(),
                };
                format!("path:{p:?} [{v}]")
              } else if let Some(g) = p.git {
                is_git = true;
                let mut v = format!("git:{g}");
                if let Some(branch) = p.branch {
                  let _ = write!(v, "&branch={branch}");
                } else if let Some(rev) = p.rev {
                  let _ = write!(v, "#{rev}");
                }
                v
              } else {
                "unknown manifest".to_string()
              }
            }
          },
          None => "no manifest".to_string(),
        };

        let lock_version = match (lock, crate_lock_packages.is_empty()) {
          (Some(_lock), false) => crate_lock_packages
            .iter()
            .map(|p| p.version.clone())
            .collect::<Vec<String>>()
            .join(", "),
          (Some(_lock), true) => "unknown lockfile".to_string(),
          _ => "no lockfile".to_string(),
        };

        (
          format!(
            "{} {}({})",
            manifest_version,
            if is_git { "(git manifest)" } else { "" },
            lock_version
          ),
          found_crate_versions,
        )
      }
    };

  let crate_version = found_crate_versions
    .into_iter()
    .map(|v| semver::Version::parse(&v).ok())
    .max();
  let suffix = match (crate_version, crate_latest_version(name)) {
    (Some(Some(version)), Some(target_version)) => {
      let target_version = semver::Version::parse(&target_version).unwrap();
      if version < target_version {
        Some(format!(
          " ({}, latest: {})",
          "outdated".yellow(),
          target_version.to_string().green()
        ))
      } else {
        None
      }
    }
    _ => None,
  };
  (crate_version_string, suffix)
}

pub fn items(app_dir: Option<&PathBuf>, tauri_dir: Option<PathBuf>) -> Vec<SectionItem> {
  let mut items = Vec::new();

  if tauri_dir.is_some() || app_dir.is_some() {
    if let Some(tauri_dir) = tauri_dir {
      let manifest: Option<CargoManifest> =
        if let Ok(manifest_contents) = read_to_string(tauri_dir.join("Cargo.toml")) {
          toml::from_str(&manifest_contents).ok()
        } else {
          None
        };
      let lock: Option<CargoLock> = get_workspace_dir()
        .ok()
        .and_then(|p| read_to_string(p.join("Cargo.lock")).ok())
        .and_then(|s| toml::from_str(&s).ok());

      for dep in ["tauri", "tauri-build", "wry", "tao"] {
        let (version_string, version_suffix) =
          crate_version(&tauri_dir, manifest.as_ref(), lock.as_ref(), dep);
        let dep = dep.to_string();
        let item = SectionItem::new(
          move || {
            Some((
              format!(
                "{} {}: {}{}",
                dep,
                "[RUST]".dimmed(),
                version_string,
                version_suffix
                  .clone()
                  .map(|s| format!(",{s}"))
                  .unwrap_or_else(|| "".into())
              ),
              Status::Neutral,
            ))
          },
          || None,
          false,
        );
        items.push(item);
      }
    }
  }

  if let Ok(rust_cli) = std::process::Command::new("cargo")
    .arg("tauri")
    .arg("-V")
    .output()
  {
    if rust_cli.status.success() {
      let stdout = String::from_utf8_lossy(rust_cli.stdout.as_slice()).to_string();
      let mut output = stdout.split(' ');
      let dep = output.next().unwrap_or_default().to_string();
      let version_string = output
        .next()
        .unwrap_or_default()
        .strip_suffix('\n')
        .unwrap_or_default()
        .to_string();

      let version_suffix = match crate_latest_version(&dep) {
        Some(target_version) => {
          let version = semver::Version::parse(&version_string).unwrap();
          let target_version = semver::Version::parse(&target_version).unwrap();
          if version < target_version {
            Some(format!(
              " ({}, latest: {})",
              "outdated".yellow(),
              target_version.to_string().green()
            ))
          } else {
            None
          }
        }
        None => None,
      };

      items.push(SectionItem::new(
        move || {
          Some((
            format!(
              "{} {}: {}{}",
              dep,
              "[RUST]".dimmed(),
              version_string,
              version_suffix
                .clone()
                .map(|s| format!(", {s}"))
                .unwrap_or_else(|| "".into())
            ),
            Status::Neutral,
          ))
        },
        || None,
        false,
      ));
    } else {
      items.push(SectionItem::new(
        move || {
          Some((
            format!("tauri-cli {}: not installed!", "[RUST]".dimmed()),
            Status::Neutral,
          ))
        },
        || None,
        false,
      ));
    }
  }

  items
}
