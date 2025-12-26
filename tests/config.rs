//! Integration tests for neve-config crate.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use neve_config::{SystemConfig, UserConfig};
use neve_config::module::{Module, OptionDecl, OptionType};
use neve_config::generate::{Generator, GeneratedConfig};
use neve_config::activate::{Activator, TestResult};
use neve_config::generation::{GenerationManager, GenerationMetadata};
use neve_eval::Value;
use neve_derive::{Hash, StorePath};

// SystemConfig tests

#[test]
fn test_system_config_builder() {
    let config = SystemConfig::new("my-system")
        .hostname("neve-host")
        .timezone("UTC")
        .service("sshd")
        .package("vim");

    assert_eq!(config.name, "my-system");
    assert_eq!(config.options.hostname, Some("neve-host".to_string()));
    assert_eq!(config.options.services, vec!["sshd"]);
    assert_eq!(config.options.packages, vec!["vim"]);
}

#[test]
fn test_system_config_multiple_packages() {
    let config = SystemConfig::new("test")
        .package("vim")
        .package("git")
        .package("tmux");

    assert_eq!(config.options.packages, vec!["vim", "git", "tmux"]);
}

// UserConfig tests

#[test]
fn test_user_config_builder() {
    let user = UserConfig::new("alice")
        .shell("/bin/zsh")
        .group("wheel")
        .package("git");

    assert_eq!(user.name, "alice");
    assert_eq!(user.home, PathBuf::from("/home/alice"));
    assert_eq!(user.shell, Some("/bin/zsh".to_string()));
    assert_eq!(user.groups, vec!["wheel"]);
}

#[test]
fn test_user_config_default_home() {
    let user = UserConfig::new("testuser")
        .shell("/bin/bash");

    // Default home is /home/<username>
    assert_eq!(user.home, PathBuf::from("/home/testuser"));
}

// Module tests

#[test]
fn test_module_builder() {
    let module = Module::new("test")
        .import("./base.neve")
        .set("hostname", Value::String(Rc::new("test-host".to_string())));

    assert_eq!(module.name, "test");
    assert_eq!(module.imports, vec!["./base.neve"]);
}

#[test]
fn test_option_decl() {
    let opt = OptionDecl::new("networking.hostName", OptionType::String)
        .description("The hostname of the system")
        .example("my-host");

    assert_eq!(opt.name, "networking.hostName");
    assert_eq!(opt.ty, OptionType::String);
    assert!(opt.description.is_some());
}

#[test]
fn test_module_to_system_config() {
    let module = Module::new("test")
        .set("hostname", Value::String(Rc::new("test-host".to_string())))
        .set("timezone", Value::String(Rc::new("UTC".to_string())))
        .set("packages", Value::List(Rc::new(vec![
            Value::String(Rc::new("vim".to_string())),
            Value::String(Rc::new("git".to_string())),
        ])));

    let config = module.to_system_config().unwrap();
    assert_eq!(config.options.hostname, Some("test-host".to_string()));
    assert_eq!(config.options.timezone, Some("UTC".to_string()));
    assert_eq!(config.options.packages, vec!["vim", "git"]);
}

// Generator tests

fn temp_dir(suffix: &str) -> PathBuf {
    env::temp_dir().join(format!("neve-config-test-{}-{}", std::process::id(), suffix))
}

#[test]
fn test_generator() {
    let dir = temp_dir("gen");

    let config = SystemConfig::new("test")
        .hostname("test-host")
        .timezone("UTC")
        .service("sshd");

    let generator = Generator::new(dir.clone());
    let generated = generator.generate(&config).unwrap();

    assert!(!generated.files.is_empty());
    assert!(generated.activation_script.is_some());
    assert_eq!(generated.services, vec!["sshd"]);

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_to_derivation() {
    let dir = temp_dir("drv");
    let config = SystemConfig::new("my-config")
        .hostname("my-host")
        .package("vim")
        .service("sshd");

    let generator = Generator::new(dir);
    let drv = generator.to_derivation(&config);

    assert_eq!(drv.name, "my-config");
    assert!(drv.env.contains_key("hostname"));
    assert!(drv.env.contains_key("packages"));
}

// Activator tests

#[test]
fn test_activator_dry_run() {
    let activator = Activator::new()
        .dry_run(true)
        .verbose(false);

    let generated = GeneratedConfig::new();
    let result = activator.activate(&generated).unwrap();

    assert!(result.success);
    assert_eq!(result.files_installed, 0);
}

#[test]
fn test_test_result() {
    let mut result = TestResult::new();
    result.files_checked = 5;
    result.warnings.push("test warning".to_string());
    result.success = true;

    assert!(result.success);
    assert_eq!(result.warnings.len(), 1);
}

// Generation manager tests

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
