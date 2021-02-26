use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use dunce::simplified;
use path_slash::PathExt;

use crate::{
    artifacts_emitter::ArtifactEmitter, error::BuildError, BuildResult, FileOperation, IOResultExt,
};

// User configurable options during flutter build
#[derive(Default)]
pub struct FlutterBuildOptions {
    // lib/main.dart by default
    pub target_file: Option<PathBuf>,

    pub local_engine: Option<String>,
    pub local_engine_src_path: Option<PathBuf>,
}

pub enum TargetOS {
    Mac,
    Windows,
}

pub struct FlutterBuild {
    pub(super) root_dir: PathBuf,
    pub(super) out_dir: PathBuf,
    pub(super) options: FlutterBuildOptions,
    pub(super) build_mode: String,
    pub(super) target_os: TargetOS,
    pub(super) target_platform: String,
}

impl FlutterBuild {
    pub fn new(options: FlutterBuildOptions) -> FlutterBuild {
        FlutterBuild {
            root_dir: std::env::var("CARGO_MANIFEST_DIR").unwrap().into(),
            out_dir: std::env::var("OUT_DIR").unwrap().into(),
            options: FlutterBuildOptions {
                target_file: options.target_file.or_else(|| Some("lib/main.dart".into())),
                local_engine: options.local_engine,
                local_engine_src_path: options.local_engine_src_path,
            },
            build_mode: FlutterBuild::build_mode(),
            target_os: FlutterBuild::target_os(),
            target_platform: FlutterBuild::target_platform(),
        }
    }

    pub fn build(&self) -> BuildResult<()> {
        let flutter_out_root = self.out_dir.join("flutter");
        let flutter_out_dart_tool = flutter_out_root.join(".dart_tool");
        fs::create_dir_all(&flutter_out_dart_tool)
            .wrap_error(FileOperation::CreateDir, flutter_out_dart_tool.clone())?;

        let package_config = self.root_dir.join(".dart_tool").join("package_config.json");
        let package_config_out = flutter_out_dart_tool.join("package_config.json");


        if !Path::exists(&package_config) {
            return Err(BuildError::OtherError(
                "package_config.json does not exist. Run flutter pub get first".to_owned(),
            ));
        }

        Self::copy(&package_config, &package_config_out)?;

        let mut local_roots = HashSet::<PathBuf>::new();

        self.update_package_config_paths(package_config, package_config_out, &mut local_roots)?;

        Self::copy(
            self.root_dir
                .join(".dart_tool")
                .join("package_config_subset"),
            flutter_out_root
                .join(".dart_tool")
                .join("package_config_subset"),
        )?;

        Self::copy(
            self.root_dir.join("pubspec.yaml"),
            flutter_out_root.join("pubspec.yaml"),
        )?;

        self.run_flutter_assemble(&flutter_out_root)?;
        self.emit_flutter_artifacts(&flutter_out_root)?;
        self.emit_flutter_checks(&local_roots).unwrap();

        Ok(())
    }

    pub fn build_mode() -> String {
        let mut build_mode: String = std::env::var("PROFILE").unwrap().into();
        let profile = std::env::var("FLUTTER_PROFILE").unwrap_or("false".into()) == "true";
        if profile && build_mode != "release" {
            panic!("Profile option (FLUTTER_PROFILE) must only be enabled for release builds")
        }
        if profile {
            build_mode = "profile".into();
        }
        build_mode
    }

    fn target_platform() -> String {
        let env_arch = std::env::var("CARGO_CFG_TARGET_ARCH");
        let arch = match env_arch.as_deref() {
            Ok("x86_64") => "x64",
            Ok("aarch64") => "arm64",
            _ => panic!("Unsupported target architecture {:?}", env_arch),
        };
        match FlutterBuild::target_os() {
            TargetOS::Mac => format!("darwin-{}", arch),
            TargetOS::Windows => format!("windows-{}", arch),
        }
    }

    fn target_os() -> TargetOS {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS");
        match target_os.as_deref() {
            Ok("macos") => TargetOS::Mac,
            Ok("windows") => TargetOS::Windows,
            _ => panic!("Unsupported target operating system {:?}", target_os),
        }
    }

    fn update_package_config_paths<PathRef: AsRef<Path>>(
        &self,
        original: PathRef,
        new: PathRef,
        local_roots: &mut HashSet<PathBuf>,
    ) -> BuildResult<()> {
        let string =
            fs::read_to_string(&new).wrap_error(FileOperation::Read, new.as_ref().into())?;

        let mut package_config: PackageConfig =
            serde_json::from_str(&string).map_err(|e| BuildError::JsonError {
                text: Some(string),
                source: e,
            })?;

        for package in &mut package_config.packages {
            if package.root_uri.starts_with("..") {
                // relative path
                let absolute = original.as_ref().parent().unwrap().join(&package.root_uri);

                let absolute = absolute
                    .canonicalize()
                    .wrap_error(FileOperation::Canonicalize, absolute)?;

                {
                    // joining posix path with windows path results in posix
                    // path used as last segment verbatim; this is a workaround
                    let mut local_root = absolute.clone();
                    local_root.extend(Path::new(&package.package_uri).iter());
                    local_roots.insert(local_root);
                }

                // remove unc from windows canonicalize
                let absolute = simplified(&absolute);
                let mut absolute = absolute.to_slash_lossy();
                if !absolute.starts_with("/") {
                    absolute = format!("/{}", absolute);
                }
                absolute = format!("file://{}", absolute);
                package.root_uri = absolute;
            }
        }

        let serialized =
            serde_json::to_string_pretty(&package_config).map_err(|e| BuildError::JsonError {
                text: None,
                source: e,
            })?;

        fs::write(&new, &serialized).wrap_error(FileOperation::Write, new.as_ref().into())?;

        Ok(())
    }

    fn run_flutter_assemble<PathRef: AsRef<Path>>(&self, working_dir: PathRef) -> BuildResult<()> {
        let rebased = pathdiff::diff_paths(&self.root_dir, &working_dir).unwrap();

        let actions: Vec<String> = match (&self.target_os, self.build_mode.as_str()) {
            (TargetOS::Mac, _) => vec![format!("{}_macos_bundle_flutter_assets", self.build_mode)],

            // quicker, no need to copy flutter artifacts, we'll do it ourselves
            (TargetOS::Windows, "debug") => vec!["kernel_snapshot".into(), "copy_assets".into()],

            // the only published action that builds desktop aot is *_bundle_*_assets; it also
            // produces other artifacts (i.e. flutter dll), but we ignore those and handle
            // artifacts on our own
            (TargetOS::Windows, _) => vec![format!("{}_bundle_windows_assets", self.build_mode)],
        };

        let mut command: Command = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(&["/C", "flutter"]);
            c
        } else {
            Command::new("flutter")
        };
        command.current_dir(&working_dir);

        if let Some(local_engine) = &self.options.local_engine {
            command.arg(format!("--local-engine={}", local_engine));
        }
        if let Some(local_src_engine_path) = &self.options.local_engine_src_path {
            command.arg(format!(
                "--local-engine-src-path={}",
                local_src_engine_path.to_str().unwrap()
            ));
        }
        command
            .arg("assemble")
            .arg("--output=.")
            .arg(format!("--define=BuildMode={}", self.build_mode))
            .arg(format!("--define=TargetPlatform={}", self.target_platform))
            .arg(format!(
                "--define=TargetFile={}",
                rebased
                    .join(self.options.target_file.as_ref().unwrap())
                    .to_str()
                    .unwrap()
            ))
            .arg("-v")
            .arg("--suppress-analytics")
            .args(actions);
        let output = command
            .output()
            .wrap_error(FileOperation::Command, "flutter".into())?;

        if !output.status.success() {
            Err(BuildError::FlutterToolError {
                command: format!("{:?}", command),
                status: output.status,
                stderr: String::from_utf8_lossy(&output.stderr).into(),
                stdout: String::from_utf8_lossy(&output.stdout).into(),
            })
        } else {
            Ok(())
        }
    }

    fn emit_flutter_artifacts<PathRef: AsRef<Path>>(
        &self,
        working_dir: PathRef,
    ) -> BuildResult<()> {
        let artifacts_dir = self.out_dir.join("../../../");
        let artifacts_dir = artifacts_dir
            .canonicalize()
            .wrap_error(FileOperation::Canonicalize, artifacts_dir)?;
        let flutter_out_root = self.out_dir.join("flutter");
        let emitter = ArtifactEmitter::new(&self, flutter_out_root, artifacts_dir)?;

        match self.target_os {
            TargetOS::Mac => {
                cargo_emit::rustc_link_search! {
                    working_dir.as_ref().to_str().unwrap() => "framework",
                };
                emitter.emit_app_framework()?;
                emitter.emit_external_libraries()?;
                emitter.emit_linker_flags()?;
            }
            TargetOS::Windows => {
                emitter.emit_flutter_data()?;
                emitter.emit_external_libraries()?;
                emitter.emit_linker_flags()?;
            }
        }
        Ok(())
    }

    fn emit_flutter_checks(&self, roots: &HashSet<PathBuf>) -> BuildResult<()> {
        cargo_emit::rerun_if_changed! {
            self.root_dir.join("pubspec.yaml").to_str().unwrap(),
            self.root_dir.join("pubspec.lock").to_str().unwrap(),
        };

        for path in roots {
            self.emit_checks_for_dir(path)?;
        }
        Ok(())
    }

    fn emit_checks_for_dir(&self, path: &Path) -> BuildResult<()> {
        for entry in fs::read_dir(path).wrap_error(FileOperation::ReadDir, path.into())? {
            let entry = entry.wrap_error(FileOperation::ReadDir, path.into())?;
            let metadata = entry
                .metadata()
                .wrap_error(FileOperation::MetaData, entry.path())?;
            if metadata.is_dir() {
                self.emit_checks_for_dir(entry.path().as_path())?;
            } else {
                cargo_emit::rerun_if_changed! {
                    entry.path().to_string_lossy(),
                }
            }
        }
        Ok(())
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64, BuildError> {
        fs::copy(&from, &to).wrap_error_with_src(
            FileOperation::Copy,
            to.as_ref().into(),
            from.as_ref().into(),
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Package {
    root_uri: String,
    package_uri: String,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PackageConfig {
    packages: Vec<Package>,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}
