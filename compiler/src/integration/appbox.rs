//! AppBox Integration for BoxLang
//!
//! This module provides integration between BoxLang compiler and AppBox
//! application container format, allowing BoxLang applications to be
//! packaged and distributed as AppBox packages (.bx files).

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const BX_EXTENSION: &str = "bx";
pub const MANIFEST_FILE: &str = "appbox.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppBoxManifest {
    pub format_version: String,
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub runtime: RuntimeConfig,
    pub platforms: Vec<PlatformConfig>,
    pub permissions: Vec<Permission>,
    pub resources: ResourceLimits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boxlang: Option<BoxLangConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub language: String,
    pub version: String,
    pub entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub os: String,
    pub arch: String,
    pub binary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<String>,
    #[serde(default)]
    pub network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxLangConfig {
    pub edition: String,
    pub target: String,
    pub opt_level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip: Option<bool>,
}

impl Default for AppBoxManifest {
    fn default() -> Self {
        Self {
            format_version: "1.0.0".to_string(),
            app_id: "com.example.app".to_string(),
            name: "MyApp".to_string(),
            version: "1.0.0".to_string(),
            author: "Anonymous".to_string(),
            description: "A BoxLang application".to_string(),
            icon: None,
            runtime: RuntimeConfig {
                language: "boxlang".to_string(),
                version: "0.1.0".to_string(),
                entry: "main".to_string(),
                args: None,
                env: None,
            },
            platforms: vec![
                PlatformConfig {
                    os: "linux".to_string(),
                    arch: "amd64".to_string(),
                    binary: "BIN/linux_amd64/main".to_string(),
                    dependencies: vec![],
                },
                PlatformConfig {
                    os: "darwin".to_string(),
                    arch: "arm64".to_string(),
                    binary: "BIN/darwin_arm64/main".to_string(),
                    dependencies: vec![],
                },
                PlatformConfig {
                    os: "windows".to_string(),
                    arch: "amd64".to_string(),
                    binary: "BIN/windows_amd64/main.exe".to_string(),
                    dependencies: vec![],
                },
            ],
            permissions: vec![],
            resources: ResourceLimits {
                memory: Some("128MB".to_string()),
                cpu: Some("50%".to_string()),
                disk: Some("100MB".to_string()),
                network: true,
            },
            boxlang: Some(BoxLangConfig::default()),
        }
    }
}

impl Default for BoxLangConfig {
    fn default() -> Self {
        Self {
            edition: "2024".to_string(),
            target: "native".to_string(),
            opt_level: 2,
            features: None,
            dependencies: None,
            lto: Some(false),
            strip: Some(true),
        }
    }
}

impl AppBoxManifest {
    pub fn new(name: &str) -> Self {
        let mut manifest = Self::default();
        manifest.name = name.to_string();
        manifest.app_id = format!("com.boxlang.{}", name.to_lowercase().replace(' ', "_"));
        manifest
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.app_id.is_empty() {
            anyhow::bail!("app_id is required");
        }
        if self.name.is_empty() {
            anyhow::bail!("name is required");
        }
        if self.version.is_empty() {
            anyhow::bail!("version is required");
        }
        if self.format_version != "1.0.0" {
            anyhow::bail!("Unsupported format version: {}", self.format_version);
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize manifest")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse manifest")
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read manifest file")?;
        Self::from_json(&content)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path.as_ref(), json).context("Failed to write manifest file")
    }
}

pub struct AppBoxBuilder {
    manifest: AppBoxManifest,
    source_files: Vec<PathBuf>,
    output_dir: PathBuf,
    sign: bool,
    key_path: Option<PathBuf>,
    verbose: bool,
}

impl AppBoxBuilder {
    pub fn new(name: &str, output_dir: impl AsRef<Path>) -> Self {
        Self {
            manifest: AppBoxManifest::new(name),
            source_files: vec![],
            output_dir: output_dir.as_ref().to_path_buf(),
            sign: false,
            key_path: None,
            verbose: false,
        }
    }

    pub fn from_manifest(manifest: AppBoxManifest, output_dir: impl AsRef<Path>) -> Self {
        Self {
            manifest,
            source_files: vec![],
            output_dir: output_dir.as_ref().to_path_buf(),
            sign: false,
            key_path: None,
            verbose: false,
        }
    }

    pub fn version(mut self, version: &str) -> Self {
        self.manifest.version = version.to_string();
        self
    }

    pub fn author(mut self, author: &str) -> Self {
        self.manifest.author = author.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.manifest.description = description.to_string();
        self
    }

    pub fn app_id(mut self, app_id: &str) -> Self {
        self.manifest.app_id = app_id.to_string();
        self
    }

    pub fn icon(mut self, icon: &str) -> Self {
        self.manifest.icon = Some(icon.to_string());
        self
    }

    pub fn add_source(mut self, path: impl AsRef<Path>) -> Self {
        self.source_files.push(path.as_ref().to_path_buf());
        self
    }

    pub fn sources(mut self, paths: Vec<PathBuf>) -> Self {
        self.source_files = paths;
        self
    }

    pub fn opt_level(mut self, level: u8) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.opt_level = level.min(3);
        }
        self
    }

    pub fn target(mut self, target: &str) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.target = target.to_string();
        }
        self
    }

    pub fn add_platform(mut self, os: &str, arch: &str, binary: &str) -> Self {
        self.manifest.platforms.push(PlatformConfig {
            os: os.to_string(),
            arch: arch.to_string(),
            binary: binary.to_string(),
            dependencies: vec![],
        });
        self
    }

    pub fn add_permission(mut self, name: &str, description: &str, required: bool) -> Self {
        self.manifest.permissions.push(Permission {
            name: name.to_string(),
            description: description.to_string(),
            required,
        });
        self
    }

    pub fn memory_limit(mut self, memory: &str) -> Self {
        self.manifest.resources.memory = Some(memory.to_string());
        self
    }

    pub fn cpu_limit(mut self, cpu: &str) -> Self {
        self.manifest.resources.cpu = Some(cpu.to_string());
        self
    }

    pub fn network(mut self, enabled: bool) -> Self {
        self.manifest.resources.network = enabled;
        self
    }

    pub fn sign(mut self, sign: bool) -> Self {
        self.sign = sign;
        self
    }

    pub fn key_path(mut self, path: impl AsRef<Path>) -> Self {
        self.key_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn feature(mut self, feature: &str) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.features.get_or_insert_with(Vec::new).push(feature.to_string());
        }
        self
    }

    pub fn dependency(mut self, name: &str, version: &str) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.dependencies.get_or_insert_with(HashMap::new)
                .insert(name.to_string(), version.to_string());
        }
        self
    }

    pub fn lto(mut self, enabled: bool) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.lto = Some(enabled);
        }
        self
    }

    pub fn strip(mut self, enabled: bool) -> Self {
        if let Some(ref mut config) = self.manifest.boxlang {
            config.strip = Some(enabled);
        }
        self
    }

    pub fn build(self) -> Result<PathBuf> {
        self.manifest.validate()?;

        std::fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("Failed to create output directory: {}", self.output_dir.display()))?;

        let manifest_path = self.output_dir.join(MANIFEST_FILE);
        self.manifest.save(&manifest_path)?;

        for source in &self.source_files {
            let file_name = source.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid source file path: {}", source.display()))?;
            let dest = self.output_dir.join(file_name);
            std::fs::copy(source, &dest)
                .with_context(|| format!("Failed to copy {} to {}", source.display(), dest.display()))?;
        }

        let package_name = format!("{}.{}", self.manifest.name.to_lowercase().replace(' ', "_"), BX_EXTENSION);
        let package_path = self.output_dir.join(&package_name);
        self.create_package(&package_path)?;

        if self.verbose {
            println!("Created package: {}", package_path.display());
            println!("App ID: {}", self.manifest.app_id);
            println!("Version: {}", self.manifest.version);
        }

        Ok(package_path)
    }

    fn create_package(&self, output: &Path) -> Result<()> {
        use std::fs::File;
        use zip::{ZipWriter, write::FileOptions};

        let file = File::create(output)
            .with_context(|| format!("Failed to create package file: {}", output.display()))?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(3));

        let manifest_content = self.manifest.to_json()?;
        zip.start_file(format!("META/{}", MANIFEST_FILE), options)?;
        zip.write_all(manifest_content.as_bytes())?;

        for source in &self.source_files {
            let file_name = source.file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid source file"))?;
            zip.start_file(format!("SRC/{}", file_name.to_string_lossy()), options)?;
            let content = std::fs::read(source)?;
            zip.write_all(&content)?;
        }

        zip.finish()?;
        Ok(())
    }

    pub fn manifest(&self) -> &AppBoxManifest {
        &self.manifest
    }

    pub fn manifest_mut(&mut self) -> &mut AppBoxManifest {
        &mut self.manifest
    }
}

pub fn generate_default_main(name: &str) -> String {
    format!(r#"module {};

/// Main entry point for {} AppBox application
pub fn main() {{
    println("Hello from {}!");
    println("This is a BoxLang application packaged with AppBox.");
}}
"#, name.to_lowercase().replace(' ', "_"), name, name)
}

pub fn convert_box_toml_to_appbox(box_toml_path: impl AsRef<Path>) -> Result<AppBoxManifest> {
    let content = std::fs::read_to_string(box_toml_path.as_ref())?;
    let box_config: toml::Value = toml::from_str(&content)?;
    
    let mut manifest = AppBoxManifest::default();
    
    if let Some(package) = box_config.get("package") {
        if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
            manifest.name = name.to_string();
            manifest.app_id = format!("com.boxlang.{}", name.to_lowercase().replace(' ', "_"));
        }
        if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
            manifest.version = version.to_string();
        }
        if let Some(description) = package.get("description").and_then(|v| v.as_str()) {
            manifest.description = description.to_string();
        }
        if let Some(authors) = package.get("authors").and_then(|v| v.as_array()) {
            if let Some(author) = authors.first().and_then(|a| a.as_str()) {
                manifest.author = author.to_string();
            }
        }
    }

    if let Some(dependencies) = box_config.get("dependencies").and_then(|v| v.as_table()) {
        let deps: HashMap<String, String> = dependencies.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();
        
        if !deps.is_empty() {
            if let Some(ref mut config) = manifest.boxlang {
                config.dependencies = Some(deps);
            }
        }
    }

    if let Some(features) = box_config.get("features").and_then(|v| v.as_array()) {
        let features: Vec<String> = features.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        
        if !features.is_empty() {
            if let Some(ref mut config) = manifest.boxlang {
                config.features = Some(features);
            }
        }
    }
    
    Ok(manifest)
}

pub fn create_boxlang_project(name: &str, output_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref().join(name);
    std::fs::create_dir_all(&output_dir)?;
    std::fs::create_dir_all(output_dir.join("src"))?;
    std::fs::create_dir_all(output_dir.join("resources"))?;
    std::fs::create_dir_all(output_dir.join("config"))?;

    let main_content = generate_default_main(name);
    std::fs::write(output_dir.join("src/main.box"), main_content)?;

    let manifest = AppBoxManifest::new(name);
    manifest.save(output_dir.join(MANIFEST_FILE))?;

    let box_toml = format!(r#"[package]
name = "{}"
version = "1.0.0"
description = "A BoxLang application"
authors = ["Your Name"]

[dependencies]

[features]
"#, name);
    std::fs::write(output_dir.join("box.toml"), box_toml)?;

    let gitignore = r#"dist/
*.bx
*.log
"#;
    std::fs::write(output_dir.join(".gitignore"), gitignore)?;

    Ok(output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_manifest() {
        let manifest = AppBoxManifest::default();
        assert_eq!(manifest.format_version, "1.0.0");
        assert_eq!(manifest.runtime.language, "boxlang");
    }

    #[test]
    fn test_manifest_validation() {
        let manifest = AppBoxManifest::new("TestApp");
        assert!(manifest.validate().is_ok());
        
        let mut invalid = manifest.clone();
        invalid.app_id = "".to_string();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_builder() {
        let builder = AppBoxBuilder::new("test-app", "/tmp/test")
            .version("1.0.0")
            .author("Test Author")
            .opt_level(2)
            .add_permission("network", "Network access", true);
        
        assert_eq!(builder.manifest.name, "test-app");
        assert_eq!(builder.manifest.version, "1.0.0");
        assert_eq!(builder.manifest.permissions.len(), 1);
    }

    #[test]
    fn test_generate_default_main() {
        let main = generate_default_main("MyApp");
        assert!(main.contains("module myapp"));
        assert!(main.contains("Hello from MyApp"));
    }

    #[test]
    fn test_manifest_json_roundtrip() {
        let manifest = AppBoxManifest::new("TestApp")
            .version("2.0.0")
            .author("Test Author");
        
        let json = manifest.to_json().unwrap();
        let parsed = AppBoxManifest::from_json(&json).unwrap();
        
        assert_eq!(manifest.name, parsed.name);
        assert_eq!(manifest.version, parsed.version);
        assert_eq!(manifest.author, parsed.author);
    }
}
