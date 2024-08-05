use std::fs;
use zed_extension_api::{self as zed, Result};

struct TypstExtension {
    cached_binary_path: Option<String>,
}

#[derive(Clone)]
struct TypstLspBinary {
    path: String,
    environment: Option<Vec<(String, String)>>,
}

impl TypstExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<TypstLspBinary> {
        if let Some(path) = worktree.which("typst_lsp") {
            let env = worktree.shell_env();
            return Ok(TypstLspBinary {
                path,
                environment: Some(env),
            });
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(TypstLspBinary {
                    path: path.clone(),
                    environment: None,
                });
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "nvarner/typst-lsp",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, _arch) = zed::current_platform();
        let asset_name = format!(
            "typst-lsp-x86_64-{os}",
            os = match platform {
                zed::Os::Mac => "apple-darwin",
                zed::Os::Linux => "unknown-linux-gnu",
                zed::Os::Windows => "pc-windows-msvc.exe",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("typst_lsp_{}", release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = format!("{version_dir}/typst_lsp");

        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(&entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(TypstLspBinary {
            path: binary_path,
            environment: None,
        })
    }
}

impl zed::Extension for TypstExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let typst_lsp_binary = self.language_server_binary(language_server_id, worktree)?;
        Ok(zed::Command {
            command: typst_lsp_binary.path,
            args: vec![],
            env: typst_lsp_binary.environment.unwrap_or_default(),
        })
    }
}

zed::register_extension!(TypstExtension);
