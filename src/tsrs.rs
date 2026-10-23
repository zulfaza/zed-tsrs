use std::fs;
use std::path::PathBuf;

use zed_extension_api::serde_json::{Value, json};
use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

struct TsrsExtension {
    cached_installation: Option<(String, Option<String>)>,
}

const PACKAGE_NAME: &str = "tsrs";

#[derive(Debug, Default)]
struct TsrsSettings {
    package_version: Option<String>,
}

impl TsrsSettings {
    fn from_lsp_settings(settings: &Value) -> Self {
        let package_version = settings
            .get("package_version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self { package_version }
    }
}

fn merge_json_value_into(source: Value, target: &mut Value) {
    match (source, target) {
        (Value::Object(source), Value::Object(target)) => {
            for (key, value) in source {
                if let Some(target) = target.get_mut(&key) {
                    merge_json_value_into(value, target);
                } else {
                    target.insert(key, value);
                }
            }
        }

        (Value::Array(source), Value::Array(target)) => {
            for value in source {
                target.push(value);
            }
        }

        (source, target) => *target = source,
    }
}

impl TsrsExtension {
    fn get_launcher_path() -> Result<PathBuf> {
        let launcher = PathBuf::from("node_modules/tsrs/bin/tsrs");
        if !launcher.is_file() {
            return Err(format!("tsrs launcher not found at {}", launcher.display()));
        }
        Ok(launcher)
    }

    fn binary_exists(&self) -> bool {
        Self::get_launcher_path().is_ok()
    }

    fn get_installed_version(&self) -> Option<String> {
        zed::npm_package_installed_version(PACKAGE_NAME)
            .ok()
            .flatten()
    }

    fn should_install_or_update(&self, target_version: &str) -> bool {
        if !self.binary_exists() {
            return true;
        }

        match self.get_installed_version() {
            Some(installed_version) => installed_version != target_version,
            None => true,
        }
    }

    fn install_package(
        &mut self,
        id: &LanguageServerId,
        custom_version: Option<&str>,
    ) -> Result<()> {
        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let target_version = match custom_version {
            Some(version) => version.to_string(),
            None => zed::npm_package_latest_version(PACKAGE_NAME)?,
        };

        if self.should_install_or_update(&target_version) {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::npm_install_package(PACKAGE_NAME, &target_version)?;
        }

        let binary_path = Self::get_launcher_path()
            .map_err(|e| format!("Failed to locate tsrs launcher after installation: {}", e))?;

        self.cached_installation = Some((
            binary_path.to_string_lossy().into_owned(),
            custom_version.map(str::to_owned),
        ));

        Ok(())
    }

    fn binary_path(
        &mut self,
        id: &LanguageServerId,
        package_version: Option<&str>,
    ) -> Result<String> {
        if let Some((cached_path, cached_version)) = self.cached_installation.as_ref()
            && cached_version.as_deref() == package_version
            && fs::metadata(cached_path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(cached_path.clone());
        }

        self.install_package(id, package_version)?;

        let binary_path = Self::get_launcher_path()
            .map_err(|e| format!("Failed to locate tsrs launcher: {}", e))?;

        Ok(binary_path.to_string_lossy().to_string())
    }
}

impl zed::Extension for TsrsExtension {
    fn new() -> Self {
        Self {
            cached_installation: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> zed_extension_api::Result<zed_extension_api::Command> {
        let lsp_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;

        let settings = lsp_settings
            .settings
            .as_ref()
            .map(TsrsSettings::from_lsp_settings)
            .unwrap_or_default();
        let (path, arguments, env) = match lsp_settings.binary {
            Some(binary) => (binary.path, binary.arguments, binary.env),
            None => (None, None, None),
        };
        let env = env.into_iter().flat_map(|env| env.into_iter()).collect();
        let arguments = arguments.unwrap_or_else(|| vec!["--lsp".into(), "--stdio".into()]);

        if let Some(command) = path {
            return Ok(zed::Command {
                command,
                args: arguments,
                env,
            });
        }

        let package_version = settings.package_version.as_deref();
        let executable_path = self.binary_path(language_server_id, package_version)?;

        let launcher = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(executable_path)
            .to_string_lossy()
            .into_owned();
        let mut args = vec![launcher];
        args.extend(arguments);

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args,
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> zed_extension_api::Result<Option<zed_extension_api::serde_json::Value>> {
        let mut initialization_options = json!({
            "codeLensShowLocationsCommandName": "editor.action.showReferences"
        });

        if let Some(user_initialization_options) =
            LspSettings::for_worktree(server_id.as_ref(), worktree)?.initialization_options
        {
            merge_json_value_into(user_initialization_options, &mut initialization_options);
        }

        Ok(Some(initialization_options))
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> zed_extension_api::Result<Option<zed_extension_api::serde_json::Value>> {
        let config = json!({
            "inlayHints": {
                "parameterNames": {
                    "enabled": "all",
                    "suppressWhenArgumentMatchesName": false
                },
                "parameterTypes": {
                    "enabled": true
                },
                "variableTypes": {
                    "enabled": true,
                    "suppressWhenTypeMatchesName": false
                },
                "propertyDeclarationTypes": {
                    "enabled": true
                },
                "functionLikeReturnTypes": {
                    "enabled": true
                },
                "enumMemberValues": {
                    "enabled": true
                }
            },
            "implementationsCodeLens": {
                "enabled": true,
                "showOnAllClassMethods": true,
                "showOnInterfaceMethods": true
            },
            "referencesCodeLens": {
                "enabled": true,
                "showOnAllFunctions": true
            }
        });

        let mut settings = json!({
            "typescript": config.clone(),
            "javascript": config
        });

        if let Some(user_settings) =
            LspSettings::for_worktree(server_id.as_ref(), worktree)?.settings
        {
            merge_json_value_into(user_settings, &mut settings);
        }

        Ok(Some(settings))
    }
}

zed::register_extension!(TsrsExtension);
