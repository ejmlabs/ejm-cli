use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use ejm_harness::{Hook, Provider, Tool};
use ejm_harness_packages::{
    InstalledPackage, PackageContext, PackageGrants, RuntimeConfig, RuntimePool, read_bounded,
};
use ratatui::style::Color;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{args::Args, packages};

#[derive(Clone, Copy)]
pub struct Palette {
    pub accent: Color,
    pub dim: Color,
    pub warning: Color,
    pub error: Color,
    pub positive: Color,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    accent: String,
    dim: String,
    warning: String,
    error: String,
    positive: String,
}

#[derive(Clone)]
pub struct Extensions {
    pub pool: RuntimePool,
    packages: Arc<Vec<(InstalledPackage, PackageGrants)>>,
    pub palette: Option<Palette>,
    widgets: Vec<(String, String)>,
    cancel: CancellationToken,
}

impl Extensions {
    pub fn empty() -> Result<Self> {
        Ok(Self {
            pool: RuntimePool::new(RuntimeConfig::default())?,
            packages: Arc::new(Vec::new()),
            palette: None,
            widgets: Vec::new(),
            cancel: CancellationToken::new(),
        })
    }

    pub async fn load(args: &Args) -> Result<Self> {
        ensure!(
            args.widget.len() <= 4,
            "at most four package widgets may be visible"
        );
        let packages = packages::manager(args)?.enabled()?;
        let pool = RuntimePool::new(RuntimeConfig::default())?;
        let result = async {
            for (package, grants) in &packages {
                pool.load(package.clone(), grants.clone(), CancellationToken::new())
                    .await?;
            }
            let mut extensions = Self {
                pool: pool.clone(),
                packages: Arc::new(packages),
                palette: None,
                widgets: Vec::new(),
                cancel: CancellationToken::new(),
            };
            if let Some(theme) = &args.theme {
                extensions.palette = Some(extensions.theme(theme)?);
            }
            for name in &args.widget {
                extensions
                    .widgets
                    .push(extensions.resolve(name, ExportKind::Widget)?);
            }
            Ok(extensions)
        }
        .await;
        if result.is_err() {
            pool.shutdown().await?;
        }
        result
    }

    pub fn tools(&self) -> Result<Vec<Arc<dyn Tool>>> {
        let mut tools = Vec::new();
        for (package, _) in self.packages.iter() {
            tools.extend(self.pool.tools(&package.manifest.name)?);
        }
        Ok(tools)
    }

    pub fn hooks(&self) -> Result<Vec<Arc<dyn Hook>>> {
        let mut hooks = Vec::new();
        for (package, _) in self.packages.iter() {
            hooks.extend(self.pool.hooks(&package.manifest.name)?);
        }
        Ok(hooks)
    }

    pub fn provider(&self, name: &str) -> Result<Arc<dyn Provider>> {
        let (package, provider) = self.resolve(name, ExportKind::Provider)?;
        self.pool.provider(&package, &provider)
    }

    pub fn summary(&self) -> String {
        if self.packages.is_empty() {
            return "No enabled packages. Use ejm-cli package install PATH.".into();
        }
        self.packages
            .iter()
            .map(|(package, grants)| {
                format!(
                    "{}@{}: tools [{}], commands [{}], {} hooks",
                    package.manifest.name,
                    package.manifest.version,
                    grants.tools.join(", "),
                    grants.commands.join(", "),
                    grants.hooks.len()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn has_widgets(&self) -> bool {
        !self.widgets.is_empty()
    }
    pub fn count(&self) -> usize {
        self.packages.len()
    }

    pub fn command(&self, name: &str) -> Result<(String, String)> {
        self.resolve(name, ExportKind::Command)
    }

    pub fn context(
        &self,
        harness: &ejm_harness::Harness,
        session: &ejm_harness::Session,
    ) -> PackageContext {
        PackageContext {
            session_id: session.id.clone(),
            cwd: session.cwd.clone(),
            governor: harness.governor(),
            cancel: self.cancel.child_token(),
        }
    }

    pub async fn widgets(&self, context: PackageContext) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        for (package, name) in &self.widgets {
            let output = self
                .pool
                .render_widget(package, name, json!({}), context.clone())
                .await?;
            let text = output
                .as_str()
                .or_else(|| output.get("text").and_then(Value::as_str))
                .context("widget must return text or {text:string}")?;
            ensure!(text.len() <= 2048, "widget output exceeds 2 KiB");
            lines.push(crate::render::safe_text(text).replace(['\n', '\r'], " "));
        }
        Ok(lines)
    }

    pub fn prompt(&self, kind: &str, name: &str) -> Result<String> {
        let mut matches = Vec::new();
        for (package, _) in self.packages.iter() {
            let paths = if kind == "skill" {
                &package.manifest.exports.skills
            } else {
                &package.manifest.exports.prompts
            };
            for path in paths {
                let asset = asset_name(path, kind)?;
                let qualified = format!("{}:{asset}", package.manifest.name);
                if name == asset || name == qualified {
                    matches.push(package.root.join(path));
                }
            }
        }
        ensure!(
            matches.len() == 1,
            "choose an unambiguous installed {kind}: @publisher/package:name"
        );
        Ok(String::from_utf8(read_bounded(&matches[0], 64 * 1024)?)?)
    }

    pub fn theme(&self, name: &str) -> Result<Palette> {
        let mut matches = Vec::new();
        for (package, _) in self.packages.iter() {
            for path in &package.manifest.exports.themes {
                let asset = asset_name(path, "theme")?;
                if name == asset || name == format!("{}:{asset}", package.manifest.name) {
                    matches.push(package.root.join(path));
                }
            }
        }
        ensure!(
            matches.len() == 1,
            "choose an unambiguous installed theme: @publisher/package:name"
        );
        let file: ThemeFile = serde_json::from_slice(&read_bounded(&matches[0], 16 * 1024)?)?;
        Ok(Palette {
            accent: color(&file.accent)?,
            dim: color(&file.dim)?,
            warning: color(&file.warning)?,
            error: color(&file.error)?,
            positive: color(&file.positive)?,
        })
    }

    pub async fn reload(&self, package: &str, context: PackageContext) -> Result<()> {
        let installation = self
            .packages
            .iter()
            .find(|(entry, _)| entry.manifest.name == package)
            .context("package is not enabled")?;
        let bytes = read_bounded(
            &installation
                .0
                .root
                .join(ejm_harness_package_spec::MANIFEST_FILE),
            ejm_harness_package_spec::MAX_MANIFEST_BYTES,
        )?;
        ensure!(
            ejm_harness_package_spec::Manifest::from_bytes(&bytes)? == installation.0.manifest,
            "manifest changed; restart to register changed exports"
        );
        self.pool.reload(package, context.cancel).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.cancel.cancel();
        self.pool.shutdown().await
    }

    fn resolve(&self, requested: &str, kind: ExportKind) -> Result<(String, String)> {
        let mut matches = Vec::new();
        for (package, grants) in self.packages.iter() {
            let names = match kind {
                ExportKind::Command => &grants.commands,
                ExportKind::Widget => &grants.widgets,
                ExportKind::Provider => &grants.providers,
            };
            for name in names {
                let qualified = format!("{}:{name}", package.manifest.name);
                if requested == name || requested == qualified {
                    matches.push((package.manifest.name.clone(), name.clone()));
                }
            }
        }
        if matches.is_empty() {
            bail!("no granted package export named {requested}");
        }
        ensure!(
            matches.len() == 1,
            "export name is ambiguous; use @publisher/package:name"
        );
        matches.pop().context("package export missing")
    }
}

#[derive(Clone, Copy)]
enum ExportKind {
    Command,
    Widget,
    Provider,
}

fn asset_name(path: &str, kind: &str) -> Result<String> {
    let path = PathBuf::from(path);
    let name = if kind == "skill" {
        path.parent().and_then(std::path::Path::file_name)
    } else {
        path.file_stem()
    };
    Ok(name
        .and_then(|value| value.to_str())
        .context("asset has no valid name")?
        .to_owned())
}

fn color(value: &str) -> Result<Color> {
    ensure!(
        value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit()),
        "theme colors must use #RRGGBB"
    );
    value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid theme color"))
}
