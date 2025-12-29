//! Configuration generations.
//! 配置代。
//!
//! Manages configuration history for rollback support.
//! 管理配置历史以支持回滚。

use crate::ConfigError;
use neve_derive::StorePath;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Generations directory name.
/// 代目录名称。
const GENERATIONS_DIR: &str = "generations";

/// Generation manager.
/// 代管理器。
pub struct GenerationManager {
    /// Base directory for generations. / 代的基础目录。
    base_dir: PathBuf,
}

impl GenerationManager {
    /// Create a new generation manager.
    /// 创建新的代管理器。
    pub fn new(base_dir: PathBuf) -> Result<Self, ConfigError> {
        let gen_dir = base_dir.join(GENERATIONS_DIR);
        fs::create_dir_all(&gen_dir)?;

        Ok(Self { base_dir })
    }

    /// Get the generations directory.
    /// 获取代目录。
    fn generations_dir(&self) -> PathBuf {
        self.base_dir.join(GENERATIONS_DIR)
    }

    /// Get the current generation link path.
    /// 获取当前代链接路径。
    fn current_link(&self) -> PathBuf {
        self.generations_dir().join("current")
    }

    /// Get the path for a specific generation.
    /// 获取特定代的路径。
    fn generation_path(&self, num: u64) -> PathBuf {
        self.generations_dir().join(format!("generation-{}", num))
    }

    /// Get the current generation number.
    /// 获取当前代号。
    pub fn current_generation(&self) -> Result<Option<u64>, ConfigError> {
        let current = self.current_link();
        if !current.exists() {
            return Ok(None);
        }

        let target = fs::read_link(&current)?;
        let name = target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ConfigError::Invalid("invalid generation link".to_string()))?;

        if let Some(gen_str) = name.strip_prefix("generation-") {
            let num = gen_str
                .parse::<u64>()
                .map_err(|_| ConfigError::Invalid("invalid generation number".to_string()))?;
            Ok(Some(num))
        } else {
            Err(ConfigError::Invalid(
                "invalid generation link format".to_string(),
            ))
        }
    }

    /// Get the next generation number.
    /// 获取下一个代号。
    pub fn next_generation(&self) -> Result<u64, ConfigError> {
        Ok(self.current_generation()?.unwrap_or(0) + 1)
    }

    /// Create a new generation.
    /// 创建新的代。
    pub fn create_generation(
        &self,
        store_path: &StorePath,
        metadata: GenerationMetadata,
    ) -> Result<Generation, ConfigError> {
        let gen_num = self.next_generation()?;
        let gen_path = self.generation_path(gen_num);

        fs::create_dir_all(&gen_path)?;

        // Save metadata
        // 保存元数据
        let meta_path = gen_path.join("metadata.json");
        let meta_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| ConfigError::Invalid(format!("JSON error: {}", e)))?;
        fs::write(&meta_path, meta_json)?;

        // Create link to store path
        // 创建到存储路径的链接
        let store_link = gen_path.join("system");
        #[cfg(unix)]
        {
            let store_path_str = store_path.display_name();
            std::os::unix::fs::symlink(&store_path_str, &store_link)?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&store_link, store_path.display_name())?;
        }

        // Update current link
        // 更新当前链接
        let current = self.current_link();
        if current.exists() || current.is_symlink() {
            fs::remove_file(&current)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&gen_path, &current)?;
        #[cfg(not(unix))]
        fs::write(&current, gen_path.to_string_lossy().as_bytes())?;

        Ok(Generation {
            number: gen_num,
            path: gen_path,
            store_path: store_path.clone(),
            metadata,
        })
    }

    /// List all generations.
    /// 列出所有代。
    pub fn list_generations(&self) -> Result<Vec<Generation>, ConfigError> {
        let mut generations = Vec::new();
        let dir = self.generations_dir();

        if !dir.exists() {
            return Ok(generations);
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if let Some(gen_str) = name_str.strip_prefix("generation-")
                && let Ok(gen_num) = gen_str.parse::<u64>()
                && let Ok(generation) = self.load_generation(gen_num)
            {
                generations.push(generation);
            }
        }

        generations.sort_by_key(|g| g.number);
        Ok(generations)
    }

    /// Load a specific generation.
    /// 加载特定的代。
    pub fn load_generation(&self, number: u64) -> Result<Generation, ConfigError> {
        let gen_path = self.generation_path(number);

        if !gen_path.exists() {
            return Err(ConfigError::NotFound(format!("generation {}", number)));
        }

        // Load metadata
        // 加载元数据
        let meta_path = gen_path.join("metadata.json");
        let metadata = if meta_path.exists() {
            let content = fs::read_to_string(&meta_path)?;
            serde_json::from_str(&content)
                .map_err(|e| ConfigError::Invalid(format!("JSON error: {}", e)))?
        } else {
            GenerationMetadata::default()
        };

        // Load store path
        // 加载存储路径
        let store_link = gen_path.join("system");
        let store_path_str = if store_link.is_symlink() {
            fs::read_link(&store_link)?.to_string_lossy().into_owned()
        } else if store_link.exists() {
            fs::read_to_string(&store_link)?
        } else {
            return Err(ConfigError::Invalid("missing system link".to_string()));
        };

        let store_path = StorePath::parse_name(&store_path_str)
            .ok_or_else(|| ConfigError::Invalid("invalid store path".to_string()))?;

        Ok(Generation {
            number,
            path: gen_path,
            store_path,
            metadata,
        })
    }

    /// Switch to a specific generation.
    /// 切换到特定的代。
    pub fn switch_to(&self, number: u64) -> Result<Generation, ConfigError> {
        let generation = self.load_generation(number)?;

        // Update current link
        // 更新当前链接
        let current = self.current_link();
        if current.exists() || current.is_symlink() {
            fs::remove_file(&current)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&generation.path, &current)?;
        #[cfg(not(unix))]
        fs::write(&current, generation.path.to_string_lossy().as_bytes())?;

        Ok(generation)
    }

    /// Delete old generations, keeping the last N.
    /// 删除旧的代，保留最后 N 个。
    pub fn collect_garbage(&self, keep: usize) -> Result<usize, ConfigError> {
        let mut generations = self.list_generations()?;

        if generations.len() <= keep {
            return Ok(0);
        }

        // Sort by number descending
        // 按代号降序排序
        generations.sort_by_key(|g| std::cmp::Reverse(g.number));

        // Remove old generations
        // 删除旧的代
        let mut deleted = 0;
        for generation in generations.into_iter().skip(keep) {
            if generation.path.exists() {
                fs::remove_dir_all(&generation.path)?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}

/// A configuration generation.
/// 配置代。
#[derive(Debug, Clone)]
pub struct Generation {
    /// Generation number. / 代号。
    pub number: u64,
    /// Path to the generation directory. / 代目录的路径。
    pub path: PathBuf,
    /// Store path of the configuration. / 配置的存储路径。
    pub store_path: StorePath,
    /// Generation metadata. / 代元数据。
    pub metadata: GenerationMetadata,
}

/// Generation metadata.
/// 代元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Creation timestamp. / 创建时间戳。
    pub created_at: u64,
    /// Configuration name. / 配置名称。
    pub name: Option<String>,
    /// Description. / 描述。
    pub description: Option<String>,
    /// Git commit (if applicable). / Git 提交（如果适用）。
    pub git_commit: Option<String>,
}

impl Default for GenerationMetadata {
    fn default() -> Self {
        Self {
            created_at: current_timestamp(),
            name: None,
            description: None,
            git_commit: None,
        }
    }
}

impl GenerationMetadata {
    /// Create new metadata.
    /// 创建新的元数据。
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    /// 设置名称。
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    /// 设置描述。
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Get current Unix timestamp.
/// 获取当前 Unix 时间戳。
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
