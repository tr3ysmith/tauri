// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use super::category::AppCategory;
use crate::bundle::{common, platform::target_triple};
pub use tauri_utils::config::WebviewInstallMode;
use tauri_utils::{
  config::{BundleType, NSISInstallerMode, NsisCompression},
  resources::{external_binaries, ResourcePaths},
};

use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

/// The type of the package we're bundling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PackageType {
  /// The macOS application bundle (.app).
  MacOsBundle,
  /// The iOS app bundle.
  IosBundle,
  /// The Windows bundle (.msi).
  WindowsMsi,
  /// The NSIS bundle (.exe).
  Nsis,
  /// The Linux Debian package bundle (.deb).
  Deb,
  /// The Linux RPM bundle (.rpm).
  Rpm,
  /// The Linux AppImage bundle (.AppImage).
  AppImage,
  /// The macOS DMG bundle (.dmg).
  Dmg,
  /// The Updater bundle.
  Updater,
}

impl From<BundleType> for PackageType {
  fn from(bundle: BundleType) -> Self {
    match bundle {
      BundleType::Deb => Self::Deb,
      BundleType::AppImage => Self::AppImage,
      BundleType::Msi => Self::WindowsMsi,
      BundleType::Nsis => Self::Nsis,
      BundleType::App => Self::MacOsBundle,
      BundleType::Dmg => Self::Dmg,
      BundleType::Updater => Self::Updater,
    }
  }
}

impl PackageType {
  /// Maps a short name to a PackageType.
  /// Possible values are "deb", "ios", "msi", "app", "rpm", "appimage", "dmg", "updater".
  pub fn from_short_name(name: &str) -> Option<PackageType> {
    // Other types we may eventually want to support: apk.
    match name {
      "deb" => Some(PackageType::Deb),
      "ios" => Some(PackageType::IosBundle),
      "msi" => Some(PackageType::WindowsMsi),
      "nsis" => Some(PackageType::Nsis),
      "app" => Some(PackageType::MacOsBundle),
      "rpm" => Some(PackageType::Rpm),
      "appimage" => Some(PackageType::AppImage),
      "dmg" => Some(PackageType::Dmg),
      "updater" => Some(PackageType::Updater),
      _ => None,
    }
  }

  /// Gets the short name of this PackageType.
  #[allow(clippy::trivially_copy_pass_by_ref)]
  pub fn short_name(&self) -> &'static str {
    match *self {
      PackageType::Deb => "deb",
      PackageType::IosBundle => "ios",
      PackageType::WindowsMsi => "msi",
      PackageType::Nsis => "nsis",
      PackageType::MacOsBundle => "app",
      PackageType::Rpm => "rpm",
      PackageType::AppImage => "appimage",
      PackageType::Dmg => "dmg",
      PackageType::Updater => "updater",
    }
  }

  /// Gets the list of the possible package types.
  pub fn all() -> &'static [PackageType] {
    ALL_PACKAGE_TYPES
  }

  /// Gets a number representing priority which used to sort package types
  /// in an order that guarantees that if a certain package type
  /// depends on another (like Dmg depending on MacOsBundle), the dependency
  /// will be built first
  ///
  /// The lower the number, the higher the priority
  pub fn priority(&self) -> u32 {
    match self {
      PackageType::MacOsBundle => 0,
      PackageType::IosBundle => 0,
      PackageType::WindowsMsi => 0,
      PackageType::Nsis => 0,
      PackageType::Deb => 0,
      PackageType::Rpm => 0,
      PackageType::AppImage => 0,
      PackageType::Dmg => 1,
      PackageType::Updater => 2,
    }
  }
}

const ALL_PACKAGE_TYPES: &[PackageType] = &[
  #[cfg(target_os = "linux")]
  PackageType::Deb,
  #[cfg(target_os = "macos")]
  PackageType::IosBundle,
  #[cfg(target_os = "windows")]
  PackageType::WindowsMsi,
  #[cfg(target_os = "windows")]
  PackageType::Nsis,
  #[cfg(target_os = "macos")]
  PackageType::MacOsBundle,
  #[cfg(target_os = "linux")]
  PackageType::Rpm,
  #[cfg(target_os = "macos")]
  PackageType::Dmg,
  #[cfg(target_os = "linux")]
  PackageType::AppImage,
  PackageType::Updater,
];

/// The package settings.
#[derive(Debug, Clone)]
pub struct PackageSettings {
  /// the package's product name.
  pub product_name: String,
  /// the package's version.
  pub version: String,
  /// the package's description.
  pub description: String,
  /// the package's homepage.
  pub homepage: Option<String>,
  /// the package's authors.
  pub authors: Option<Vec<String>>,
  /// the default binary to run.
  pub default_run: Option<String>,
}

/// The updater settings.
#[derive(Debug, Default, Clone)]
pub struct UpdaterSettings {
  /// Whether the updater is active or not.
  pub active: bool,
  /// The updater endpoints.
  pub endpoints: Option<Vec<String>>,
  /// Signature public key.
  pub pubkey: String,
  /// Display built-in dialog or use event system if disabled.
  pub dialog: bool,
  /// Args to pass to `msiexec.exe` to run the updater on Windows.
  pub msiexec_args: Option<&'static [&'static str]>,
}

/// The Linux debian bundle settings.
#[derive(Clone, Debug, Default)]
pub struct DebianSettings {
  // OS-specific settings:
  /// the list of debian dependencies.
  pub depends: Option<Vec<String>>,
  /// List of custom files to add to the deb package.
  /// Maps the path on the debian package to the path of the file to include (relative to the current working directory).
  pub files: HashMap<PathBuf, PathBuf>,
  /// Path to a custom desktop file Handlebars template.
  ///
  /// Available variables: `categories`, `comment` (optional), `exec`, `icon` and `name`.
  ///
  /// Default file contents:
  /// ```text
  #[doc = include_str!("./linux/templates/main.desktop")]
  /// ```
  pub desktop_template: Option<PathBuf>,
}

/// The macOS bundle settings.
#[derive(Clone, Debug, Default)]
pub struct MacOsSettings {
  /// MacOS frameworks that need to be bundled with the app.
  ///
  /// Each string can either be the name of a framework (without the `.framework` extension, e.g. `"SDL2"`),
  /// in which case we will search for that framework in the standard install locations (`~/Library/Frameworks/`, `/Library/Frameworks/`, and `/Network/Library/Frameworks/`),
  /// or a path to a specific framework bundle (e.g. `./data/frameworks/SDL2.framework`).  Note that this setting just makes tauri-bundler copy the specified frameworks into the OS X app bundle
  /// (under `Foobar.app/Contents/Frameworks/`); you are still responsible for:
  ///
  /// - arranging for the compiled binary to link against those frameworks (e.g. by emitting lines like `cargo:rustc-link-lib=framework=SDL2` from your `build.rs` script)
  ///
  /// - embedding the correct rpath in your binary (e.g. by running `install_name_tool -add_rpath "@executable_path/../Frameworks" path/to/binary` after compiling)
  pub frameworks: Option<Vec<String>>,
  /// A version string indicating the minimum MacOS version that the bundled app supports (e.g. `"10.11"`).
  /// If you are using this config field, you may also want have your `build.rs` script emit `cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.11`.
  pub minimum_system_version: Option<String>,
  /// The path to the LICENSE file for macOS apps.
  /// Currently only used by the dmg bundle.
  pub license: Option<String>,
  /// The exception domain to use on the macOS .app bundle.
  ///
  /// This allows communication to the outside world e.g. a web server you're shipping.
  pub exception_domain: Option<String>,
  /// Code signing identity.
  pub signing_identity: Option<String>,
  /// Provider short name for notarization.
  pub provider_short_name: Option<String>,
  /// Path to the entitlements.plist file.
  pub entitlements: Option<String>,
  /// Path to the Info.plist file for the bundle.
  pub info_plist_path: Option<PathBuf>,
}

/// Configuration for a target language for the WiX build.
#[derive(Debug, Clone, Default)]
pub struct WixLanguageConfig {
  /// The path to a locale (`.wxl`) file. See <https://wixtoolset.org/documentation/manual/v3/howtos/ui_and_localization/build_a_localized_version.html>.
  pub locale_path: Option<PathBuf>,
}

/// The languages to build using WiX.
#[derive(Debug, Clone)]
pub struct WixLanguage(pub Vec<(String, WixLanguageConfig)>);

impl Default for WixLanguage {
  fn default() -> Self {
    Self(vec![("en-US".into(), Default::default())])
  }
}

/// Settings specific to the WiX implementation.
#[derive(Clone, Debug, Default)]
pub struct WixSettings {
  /// The app languages to build. See <https://docs.microsoft.com/en-us/windows/win32/msi/localizing-the-error-and-actiontext-tables>.
  pub language: WixLanguage,
  /// By default, the bundler uses an internal template.
  /// This option allows you to define your own wix file.
  pub template: Option<PathBuf>,
  /// A list of paths to .wxs files with WiX fragments to use.
  pub fragment_paths: Vec<PathBuf>,
  /// The ComponentGroup element ids you want to reference from the fragments.
  pub component_group_refs: Vec<String>,
  /// The Component element ids you want to reference from the fragments.
  pub component_refs: Vec<String>,
  /// The FeatureGroup element ids you want to reference from the fragments.
  pub feature_group_refs: Vec<String>,
  /// The Feature element ids you want to reference from the fragments.
  pub feature_refs: Vec<String>,
  /// The Merge element ids you want to reference from the fragments.
  pub merge_refs: Vec<String>,
  /// Disables the Webview2 runtime installation after app install. Will be removed in v2, use [`WindowsSettings::webview_install_mode`] instead.
  pub skip_webview_install: bool,
  /// The path to the LICENSE file.
  pub license: Option<PathBuf>,
  /// Create an elevated update task within Windows Task Scheduler.
  pub enable_elevated_update_task: bool,
  /// Path to a bitmap file to use as the installation user interface banner.
  /// This bitmap will appear at the top of all but the first page of the installer.
  ///
  /// The required dimensions are 493px × 58px.
  pub banner_path: Option<PathBuf>,
  /// Path to a bitmap file to use on the installation user interface dialogs.
  /// It is used on the welcome and completion dialogs.

  /// The required dimensions are 493px × 312px.
  pub dialog_image_path: Option<PathBuf>,
  /// Enables FIPS compliant algorithms.
  pub fips_compliant: bool,
}

/// Settings specific to the NSIS implementation.
#[derive(Clone, Debug, Default)]
pub struct NsisSettings {
  /// A custom .nsi template to use.
  pub template: Option<PathBuf>,
  /// The path to the license file to render on the installer.
  pub license: Option<PathBuf>,
  /// The path to a bitmap file to display on the header of installers pages.
  ///
  /// The recommended dimensions are 150px x 57px.
  pub header_image: Option<PathBuf>,
  /// The path to a bitmap file for the Welcome page and the Finish page.
  ///
  /// The recommended dimensions are 164px x 314px.
  pub sidebar_image: Option<PathBuf>,
  /// The path to an icon file used as the installer icon.
  pub installer_icon: Option<PathBuf>,
  /// Whether the installation will be for all users or just the current user.
  pub install_mode: NSISInstallerMode,
  /// A list of installer languages.
  /// By default the OS language is used. If the OS language is not in the list of languages, the first language will be used.
  /// To allow the user to select the language, set `display_language_selector` to `true`.
  ///
  /// See <https://github.com/kichik/nsis/tree/9465c08046f00ccb6eda985abbdbf52c275c6c4d/Contrib/Language%20files> for the complete list of languages.
  pub languages: Option<Vec<String>>,
  /// An key-value pair where the key is the language and the
  /// value is the path to a custom `.nsi` file that holds the translated text for tauri's custom messages.
  ///
  /// See <https://github.com/tauri-apps/tauri/blob/dev/tooling/bundler/src/bundle/windows/templates/nsis-languages/English.nsh> for an example `.nsi` file.
  ///
  /// **Note**: the key must be a valid NSIS language and it must be added to [`NsisConfig`]languages array,
  pub custom_language_files: Option<HashMap<String, PathBuf>>,
  /// Whether to display a language selector dialog before the installer and uninstaller windows are rendered or not.
  /// By default the OS language is selected, with a fallback to the first language in the `languages` array.
  pub display_language_selector: bool,
  /// Set compression algorithm used to compress files in the installer.
  pub compression: Option<NsisCompression>,
}

/// The Windows bundle settings.
#[derive(Clone, Debug)]
pub struct WindowsSettings {
  /// The file digest algorithm to use for creating file signatures. Required for code signing. SHA-256 is recommended.
  pub digest_algorithm: Option<String>,
  /// The SHA1 hash of the signing certificate.
  pub certificate_thumbprint: Option<String>,
  /// Server to use during timestamping.
  pub timestamp_url: Option<String>,
  /// Whether to use Time-Stamp Protocol (TSP, a.k.a. RFC 3161) for the timestamp server. Your code signing provider may
  /// use a TSP timestamp server, like e.g. SSL.com does. If so, enable TSP by setting to true.
  pub tsp: bool,
  /// WiX configuration.
  pub wix: Option<WixSettings>,
  /// Nsis configuration.
  pub nsis: Option<NsisSettings>,
  /// The path to the application icon. Defaults to `./icons/icon.ico`.
  pub icon_path: PathBuf,
  /// The installation mode for the Webview2 runtime.
  pub webview_install_mode: WebviewInstallMode,
  /// Path to the webview fixed runtime to use.
  ///
  /// Overwrites [`Self::webview_install_mode`] if set.
  ///
  /// Will be removed in v2, use [`Self::webview_install_mode`] instead.
  pub webview_fixed_runtime_path: Option<PathBuf>,
  /// Validates a second app installation, blocking the user from installing an older version if set to `false`.
  ///
  /// For instance, if `1.2.1` is installed, the user won't be able to install app version `1.2.0` or `1.1.5`.
  ///
  /// /// The default value of this flag is `true`.
  pub allow_downgrades: bool,
}

impl Default for WindowsSettings {
  fn default() -> Self {
    Self {
      digest_algorithm: None,
      certificate_thumbprint: None,
      timestamp_url: None,
      tsp: false,
      wix: None,
      nsis: None,
      icon_path: PathBuf::from("icons/icon.ico"),
      webview_install_mode: Default::default(),
      webview_fixed_runtime_path: None,
      allow_downgrades: true,
    }
  }
}

/// The bundle settings of the BuildArtifact we're bundling.
#[derive(Clone, Debug, Default)]
pub struct BundleSettings {
  /// the app's identifier.
  pub identifier: Option<String>,
  /// The app's publisher. Defaults to the second element in the identifier string.
  /// Currently maps to the Manufacturer property of the Windows Installer.
  pub publisher: Option<String>,
  /// the app's icon list.
  pub icon: Option<Vec<String>>,
  /// the app's resources to bundle.
  ///
  /// each item can be a path to a file or a path to a folder.
  ///
  /// supports glob patterns.
  pub resources: Option<Vec<String>>,
  /// The app's resources to bundle. Takes precedence over `Self::resources` when specified.
  ///
  /// Maps each resource path to its target directory in the bundle resources directory.
  ///
  /// Supports glob patterns.
  pub resources_map: Option<HashMap<String, String>>,
  /// the app's copyright.
  pub copyright: Option<String>,
  /// the app's category.
  pub category: Option<AppCategory>,
  /// the app's short description.
  pub short_description: Option<String>,
  /// the app's long description.
  pub long_description: Option<String>,
  // Bundles for other binaries:
  /// Configuration map for the apps to bundle.
  pub bin: Option<HashMap<String, BundleSettings>>,
  /// External binaries to add to the bundle.
  ///
  /// Note that each binary name should have the target platform's target triple appended,
  /// as well as `.exe` for Windows.
  /// For example, if you're bundling a sidecar called `sqlite3`, the bundler expects
  /// a binary named `sqlite3-x86_64-unknown-linux-gnu` on linux,
  /// and `sqlite3-x86_64-pc-windows-gnu.exe` on windows.
  ///
  /// Run `tauri build --help` for more info on targets.
  ///
  /// If you are building a universal binary for MacOS, the bundler expects
  /// your external binary to also be universal, and named after the target triple,
  /// e.g. `sqlite3-universal-apple-darwin`. See
  /// <https://developer.apple.com/documentation/apple-silicon/building-a-universal-macos-binary>
  pub external_bin: Option<Vec<String>>,
  /// Debian-specific settings.
  pub deb: DebianSettings,
  /// MacOS-specific settings.
  pub macos: MacOsSettings,
  /// Updater configuration.
  pub updater: Option<UpdaterSettings>,
  /// Windows-specific settings.
  pub windows: WindowsSettings,
}

/// A binary to bundle.
#[derive(Clone, Debug)]
pub struct BundleBinary {
  name: String,
  src_path: Option<String>,
  main: bool,
}

impl BundleBinary {
  /// Creates a new bundle binary.
  pub fn new(name: String, main: bool) -> Self {
    Self {
      name,
      src_path: None,
      main,
    }
  }

  /// Sets the src path of the binary.
  #[must_use]
  pub fn set_src_path(mut self, src_path: Option<String>) -> Self {
    self.src_path = src_path;
    self
  }

  /// Mark the binary as the main executable.
  pub fn set_main(&mut self, main: bool) {
    self.main = main;
  }

  /// Sets the binary name.
  pub fn set_name(&mut self, name: String) {
    self.name = name;
  }

  /// Returns the binary name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns the binary `main` flag.
  pub fn main(&self) -> bool {
    self.main
  }

  /// Returns the binary source path.
  pub fn src_path(&self) -> Option<&String> {
    self.src_path.as_ref()
  }
}

/// The Settings exposed by the module.
#[derive(Clone, Debug)]
pub struct Settings {
  /// The log level.
  log_level: log::Level,
  /// the package settings.
  package: PackageSettings,
  /// the package types we're bundling.
  ///
  /// if not present, we'll use the PackageType list for the target OS.
  package_types: Option<Vec<PackageType>>,
  /// the directory where the bundles will be placed.
  project_out_directory: PathBuf,
  /// the bundle settings.
  bundle_settings: BundleSettings,
  /// the binaries to bundle.
  binaries: Vec<BundleBinary>,
  /// The target triple.
  target: String,
}

/// A builder for [`Settings`].
#[derive(Default)]
pub struct SettingsBuilder {
  log_level: Option<log::Level>,
  project_out_directory: Option<PathBuf>,
  package_types: Option<Vec<PackageType>>,
  package_settings: Option<PackageSettings>,
  bundle_settings: BundleSettings,
  binaries: Vec<BundleBinary>,
  target: Option<String>,
}

impl SettingsBuilder {
  /// Creates the default settings builder.
  pub fn new() -> Self {
    Default::default()
  }

  /// Sets the project output directory. It's used as current working directory.
  #[must_use]
  pub fn project_out_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
    self
      .project_out_directory
      .replace(path.as_ref().to_path_buf());
    self
  }

  /// Sets the package types to create.
  #[must_use]
  pub fn package_types(mut self, package_types: Vec<PackageType>) -> Self {
    self.package_types = Some(package_types);
    self
  }

  /// Sets the package settings.
  #[must_use]
  pub fn package_settings(mut self, settings: PackageSettings) -> Self {
    self.package_settings.replace(settings);
    self
  }

  /// Sets the bundle settings.
  #[must_use]
  pub fn bundle_settings(mut self, settings: BundleSettings) -> Self {
    self.bundle_settings = settings;
    self
  }

  /// Sets the binaries to bundle.
  #[must_use]
  pub fn binaries(mut self, binaries: Vec<BundleBinary>) -> Self {
    self.binaries = binaries;
    self
  }

  /// Sets the target triple.
  #[must_use]
  pub fn target(mut self, target: String) -> Self {
    self.target.replace(target);
    self
  }

  /// Sets the log level for spawned commands. Defaults to [`log::Level::Error`].
  #[must_use]
  pub fn log_level(mut self, level: log::Level) -> Self {
    self.log_level.replace(level);
    self
  }

  /// Builds a Settings from the CLI args.
  ///
  /// Package settings will be read from Cargo.toml.
  ///
  /// Bundle settings will be read from $TAURI_DIR/tauri.conf.json if it exists and fallback to Cargo.toml's [package.metadata.bundle].
  pub fn build(self) -> crate::Result<Settings> {
    let target = if let Some(t) = self.target {
      t
    } else {
      target_triple()?
    };

    Ok(Settings {
      log_level: self.log_level.unwrap_or(log::Level::Error),
      package: self.package_settings.expect("package settings is required"),
      package_types: self.package_types,
      project_out_directory: self
        .project_out_directory
        .expect("out directory is required"),
      binaries: self.binaries,
      bundle_settings: BundleSettings {
        external_bin: self
          .bundle_settings
          .external_bin
          .as_ref()
          .map(|bins| external_binaries(bins, &target)),
        ..self.bundle_settings
      },
      target,
    })
  }
}

impl Settings {
  /// Sets the log level for spawned commands.
  pub fn set_log_level(&mut self, level: log::Level) {
    self.log_level = level;
  }

  /// Returns the log level for spawned commands.
  pub fn log_level(&self) -> log::Level {
    self.log_level
  }

  /// Returns the directory where the bundle should be placed.
  pub fn project_out_directory(&self) -> &Path {
    &self.project_out_directory
  }

  /// Returns the target triple.
  pub fn target(&self) -> &str {
    &self.target
  }

  /// Returns the architecture for the binary being bundled (e.g. "arm", "x86" or "x86_64").
  pub fn binary_arch(&self) -> &str {
    if self.target.starts_with("x86_64") {
      "x86_64"
    } else if self.target.starts_with('i') {
      "x86"
    } else if self.target.starts_with("arm") {
      "arm"
    } else if self.target.starts_with("aarch64") {
      "aarch64"
    } else if self.target.starts_with("universal") {
      "universal"
    } else {
      panic!("Unexpected target triple {}", self.target)
    }
  }

  /// Returns the file name of the binary being bundled.
  pub fn main_binary_name(&self) -> &str {
    self
      .binaries
      .iter()
      .find(|bin| bin.main)
      .expect("failed to find main binary")
      .name
      .as_str()
  }

  /// Returns the path to the specified binary.
  pub fn binary_path(&self, binary: &BundleBinary) -> PathBuf {
    let mut path = self.project_out_directory.clone();
    path.push(binary.name());
    path
  }

  /// Returns the list of binaries to bundle.
  pub fn binaries(&self) -> &Vec<BundleBinary> {
    &self.binaries
  }

  /// If a list of package types was specified by the command-line, returns
  /// that list filtered by the current target OS available targets.
  ///
  /// If a target triple was specified by the
  /// command-line, returns the native package type(s) for that target.
  ///
  /// Otherwise returns the native package type(s) for the host platform.
  ///
  /// Fails if the host/target's native package type is not supported.
  pub fn package_types(&self) -> crate::Result<Vec<PackageType>> {
    let target_os = self
      .target
      .split('-')
      .nth(2)
      .unwrap_or(std::env::consts::OS)
      .replace("darwin", "macos");

    let mut platform_types = match target_os.as_str() {
      "macos" => vec![PackageType::MacOsBundle, PackageType::Dmg],
      "ios" => vec![PackageType::IosBundle],
      "linux" => vec![PackageType::Deb, PackageType::AppImage],
      "windows" => vec![PackageType::WindowsMsi, PackageType::Nsis],
      os => {
        return Err(crate::Error::GenericError(format!(
          "Native {} bundles not yet supported.",
          os
        )))
      }
    };

    // add updater if needed
    if self.is_update_enabled() {
      platform_types.push(PackageType::Updater)
    }

    if let Some(package_types) = &self.package_types {
      let mut types = vec![];
      for package_type in package_types {
        let package_type = *package_type;
        if platform_types
          .clone()
          .into_iter()
          .any(|t| t == package_type)
        {
          types.push(package_type);
        }
      }
      Ok(types)
    } else {
      Ok(platform_types)
    }
  }

  /// Returns the product name.
  pub fn product_name(&self) -> &str {
    &self.package.product_name
  }

  /// Returns the bundle's identifier
  pub fn bundle_identifier(&self) -> &str {
    self.bundle_settings.identifier.as_deref().unwrap_or("")
  }

  /// Returns the bundle's identifier
  pub fn publisher(&self) -> Option<&str> {
    self.bundle_settings.publisher.as_deref()
  }

  /// Returns an iterator over the icon files to be used for this bundle.
  pub fn icon_files(&self) -> ResourcePaths<'_> {
    match self.bundle_settings.icon {
      Some(ref paths) => ResourcePaths::new(paths.as_slice(), false),
      None => ResourcePaths::new(&[], false),
    }
  }

  /// Returns an iterator over the resource files to be included in this
  /// bundle.
  pub fn resource_files(&self) -> ResourcePaths<'_> {
    match (
      &self.bundle_settings.resources,
      &self.bundle_settings.resources_map,
    ) {
      (Some(paths), None) => ResourcePaths::new(paths.as_slice(), true),
      (None, Some(map)) => ResourcePaths::from_map(map, true),
      (Some(_), Some(_)) => panic!("cannot use both `resources` and `resources_map`"),
      (None, None) => ResourcePaths::new(&[], true),
    }
  }

  /// Returns an iterator over the external binaries to be included in this
  /// bundle.
  pub fn external_binaries(&self) -> ResourcePaths<'_> {
    match self.bundle_settings.external_bin {
      Some(ref paths) => ResourcePaths::new(paths.as_slice(), true),
      None => ResourcePaths::new(&[], true),
    }
  }

  /// Copies external binaries to a path.
  pub fn copy_binaries(&self, path: &Path) -> crate::Result<()> {
    for src in self.external_binaries() {
      let src = src?;
      let dest = path.join(
        src
          .file_name()
          .expect("failed to extract external binary filename")
          .to_string_lossy()
          .replace(&format!("-{}", self.target), ""),
      );
      common::copy_file(&src, dest)?;
    }
    Ok(())
  }

  /// Copies resources to a path.
  pub fn copy_resources(&self, path: &Path) -> crate::Result<()> {
    for resource in self.resource_files().iter() {
      let resource = resource?;
      let dest = path.join(resource.target());
      common::copy_file(resource.path(), dest)?;
    }
    Ok(())
  }

  /// Returns the version string of the bundle.
  pub fn version_string(&self) -> &str {
    &self.package.version
  }

  /// Returns the copyright text.
  pub fn copyright_string(&self) -> Option<&str> {
    self.bundle_settings.copyright.as_deref()
  }

  /// Returns the list of authors name.
  pub fn author_names(&self) -> &[String] {
    match self.package.authors {
      Some(ref names) => names.as_slice(),
      None => &[],
    }
  }

  /// Returns the authors as a comma-separated string.
  pub fn authors_comma_separated(&self) -> Option<String> {
    let names = self.author_names();
    if names.is_empty() {
      None
    } else {
      Some(names.join(", "))
    }
  }

  /// Returns the package's homepage URL, defaulting to "" if not defined.
  pub fn homepage_url(&self) -> &str {
    self.package.homepage.as_deref().unwrap_or("")
  }

  /// Returns the app's category.
  pub fn app_category(&self) -> Option<AppCategory> {
    self.bundle_settings.category
  }

  /// Returns the app's short description.
  pub fn short_description(&self) -> &str {
    self
      .bundle_settings
      .short_description
      .as_ref()
      .unwrap_or(&self.package.description)
  }

  /// Returns the app's long description.
  pub fn long_description(&self) -> Option<&str> {
    self.bundle_settings.long_description.as_deref()
  }

  /// Returns the debian settings.
  pub fn deb(&self) -> &DebianSettings {
    &self.bundle_settings.deb
  }

  /// Returns the MacOS settings.
  pub fn macos(&self) -> &MacOsSettings {
    &self.bundle_settings.macos
  }

  /// Returns the Windows settings.
  pub fn windows(&self) -> &WindowsSettings {
    &self.bundle_settings.windows
  }

  /// Returns the Updater settings.
  pub fn updater(&self) -> Option<&UpdaterSettings> {
    self.bundle_settings.updater.as_ref()
  }

  /// Is update enabled
  pub fn is_update_enabled(&self) -> bool {
    match &self.bundle_settings.updater {
      Some(val) => val.active,
      None => false,
    }
  }
}
