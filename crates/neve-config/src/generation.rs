//! Configuration generations.
//!
//! Manages configuration history for rollback support.

use crate::ConfigError;
use neve_derive::StorePath;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Generations directory name.
const GENERATIONS_DIR: &str = "generations";

/// Generation manager.
pub struct GenerationManager {
    /// Base directory for generations.
    base_dir: PathBuf,
}

impl GenerationManager {
    /// Create a new generation manager.
    pub fn new(base_dir: PathBuf) -> Result<Self, ConfigError> {
        let gen_dir = base_dir.join(GENERATIONS_DIR);
        fs::create_dir_all(&gen_dir)?;
        
        Ok(Self { base_dir })
    }

    /// Get the generations directory.
    fn generations_dir(&self) -> PathBuf {
        self.base_dir.join(GENERATIONS_DIR)
    }

    /// Get the current generation link path.
    fn current_link(&self) -> PathBuf {
        self.generations_dir().join("current")
    }

    /// Get the path for a specific generation.
    fn generation_path(&self, num: u64) -> PathBuf {
        self.generations_dir().join(format!("generation-{}", num))
    }

    /// Get the current generation number.
    pub fn current_generation(&self) -> Result<Option<u64>, ConfigError> {
        let current = self.current_link();
        if !current.exists() {
            return Ok(None);
        }

        let target = fs::read_link(&current)?;
        let name = target.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ConfigError::Invalid("invalid generation link".to_string()))?;

        if let Some(gen_str) = name.strip_prefix("generation-") {
            let num = gen_str.parse::<u64>()
                .map_err(|_| ConfigError::Invalid("invalid generation number".to_string()))?;
            Ok(Some(num))
        } else {
            Err(ConfigError::Invalid("invalid generation link format".to_string()))
        }
    }

    /// Get the next generation number.
    pub fn next_generation(&self) -> Result<u64, ConfigError> {
        Ok(self.current_generation()?.unwrap_or(0) + 1)
    }

    /// Create a new generation.
    pub fn create_generation(&self, store_path: &StorePath, metadata: GenerationMetadata) -> Result<Generation, ConfigError> {
        let gen_num = self.next_generation()?;
        let gen_path = self.generation_path(gen_num);
        
        fs::create_dir_all(&gen_path)?;
        
        // Save metadata
        let meta_path = gen_path.join("metadata.json");
        let meta_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| ConfigError::Invalid(format!("JSON error: {}", e)))?;
        fs::write(&meta_path, meta_json)?;
        
        // Create link to store path
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
                    && let Ok(generation) = self.load_generation(gen_num) {
                        generations.push(generation);
                    }
        }
        
        generations.sort_by_key(|g| g.number);
        Ok(generations)
    }

    /// Load a specific generation.
    pub fn load_generation(&self, number: u64) -> Result<Generation, ConfigError> {
        let gen_path = self.generation_path(number);
        
        if !gen_path.exists() {
            return Err(ConfigError::NotFound(format!("generation {}", number)));
        }
        
        // Load metadata
        let meta_path = gen_path.join("metadata.json");
        let metadata = if meta_path.exists() {
            let content = fs::read_to_string(&meta_path)?;
            serde_json::from_str(&content)
                .map_err(|e| ConfigError::Invalid(format!("JSON error: {}", e)))?
        } else {
            GenerationMetadata::default()
        };
        
        // Load store path
        let store_link = gen_path.join("system");
        let store_path_str = if store_link.is_symlink() {
            fs::read_link(&store_link)?
                .to_string_lossy()
                .into_owned()
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
    pub fn switch_to(&self, number: u64) -> Result<Generation, ConfigError> {
        let generation = self.load_generation(number)?;
        
        // Update current link
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
    pub fn collect_garbage(&self, keep: usize) -> Result<usize, ConfigError> {
        let mut generations = self.list_generations()?;
        
        if generations.len() <= keep {
            return Ok(0);
        }
        
        // Sort by number descending
        generations.sort_by_key(|g| std::cmp::Reverse(g.number));
        
        // Remove old generations
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
#[derive(Debug, Clone)]
pub struct Generation {
    /// Generation number.
    pub number: u64,
    /// Path to the generation directory.
    pub path: PathBuf,
    /// Store path of the configuration.
    pub store_path: StorePath,
    /// Generation metadata.
    pub metadata: GenerationMetadata,
}

/// Generation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Creation timestamp.
    pub created_at: u64,
    /// Configuration name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Git commit (if applicable).
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Get current Unix timestamp.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use neve_derive::Hash;

    fn temp_dir(suffix: &str) -> PathBuf {
        env::temp_dir().join(format!("neve-gen-mgr-{}-{}", std::process::id(), suffix))
    }

    #[test]
    fn test_generation_manager() {
        let dir = temp_dir("mgr");
        let manager = GenerationManager::new(dir.clone()).unwrap();
        
        assert_eq!(manager.current_generation().unwrap(), None);
        assert_eq!(manager.next_generation().unwrap(), 1);
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_generation() {
        let dir = temp_dir("create");
        let manager = GenerationManager::new(dir.clone()).unwrap();
        
        let hash = Hash::of(b"test config");
        let store_path = StorePath::new(hash, "test-config".to_string());
        let metadata = GenerationMetadata::new()
            .name("test")
            .description("Test configuration");
        
        let generation = manager.create_generation(&store_path, metadata).unwrap();
        
        assert_eq!(generation.number, 1);
        assert!(generation.path.exists());
        
        // Check current points to it
        assert_eq!(manager.current_generation().unwrap(), Some(1));
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_generations() {
        let dir = temp_dir("list");
        let manager = GenerationManager::new(dir.clone()).unwrap();
        
        // Create a few generations
        for i in 1..=3 {
            let hash = Hash::of(format!("config-{}", i).as_bytes());
            let store_path = StorePath::new(hash, format!("config-{}", i));
            let metadata = GenerationMetadata::new().name(format!("gen-{}", i));
            manager.create_generation(&store_path, metadata).unwrap();
        }
        
        let gens = manager.list_generations().unwrap();
        assert_eq!(gens.len(), 3);
        assert_eq!(gens[0].number, 1);
        assert_eq!(gens[2].number, 3);
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}
