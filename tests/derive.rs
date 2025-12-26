//! Integration tests for neve-derive crate.

use neve_derive::{
    Hash, Hasher, Derivation, StorePath, Output, HashMode,
    Version, VersionConstraint, Dependency, PackageId, PackageMetadata,
    Resolver, MemoryRegistry, ResolveError,
};

// Hash tests

#[test]
fn test_hash_data() {
    let hash = Hash::of(b"hello world");
    assert!(!hash.is_null());
    assert_eq!(hash.to_hex().len(), 64);
}

#[test]
fn test_hash_roundtrip() {
    let hash = Hash::of(b"test data");
    let hex = hash.to_hex();
    let parsed = Hash::from_hex(&hex).unwrap();
    assert_eq!(hash, parsed);
}

#[test]
fn test_hasher_incremental() {
    let mut hasher = Hasher::new();
    hasher.update(b"hello ");
    hasher.update(b"world");
    let hash1 = hasher.finalize();

    let hash2 = Hash::of(b"hello world");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_deterministic() {
    let hash1 = Hash::of(b"same content");
    let hash2 = Hash::of(b"same content");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_different_data() {
    let hash1 = Hash::of(b"content a");
    let hash2 = Hash::of(b"content b");
    assert_ne!(hash1, hash2);
}

// Derivation tests

#[test]
fn test_derivation_builder() {
    let drv = Derivation::builder("hello", "2.12.1")
        .system("x86_64-linux")
        .builder_path("/bin/bash")
        .arg("-c")
        .arg("echo hello")
        .env("PATH", "/bin")
        .build();

    assert_eq!(drv.name, "hello");
    assert_eq!(drv.version, "2.12.1");
    assert_eq!(drv.system, "x86_64-linux");
    assert_eq!(drv.builder, "/bin/bash");
    assert_eq!(drv.args, vec!["-c", "echo hello"]);
    assert!(drv.outputs.contains_key("out"));
}

#[test]
fn test_derivation_hash() {
    let drv1 = Derivation::builder("hello", "1.0")
        .system("x86_64-linux")
        .build();
    
    let drv2 = Derivation::builder("hello", "1.0")
        .system("x86_64-linux")
        .build();
    
    let drv3 = Derivation::builder("hello", "1.1")
        .system("x86_64-linux")
        .build();

    // Same derivation should have same hash
    assert_eq!(drv1.hash(), drv2.hash());
    
    // Different version should have different hash
    assert_ne!(drv1.hash(), drv3.hash());
}

#[test]
fn test_derivation_json() {
    let drv = Derivation::builder("test", "1.0")
        .env("FOO", "bar")
        .build();

    let json = drv.to_json().unwrap();
    let parsed = Derivation::from_json(&json).unwrap();

    assert_eq!(drv.name, parsed.name);
    assert_eq!(drv.version, parsed.version);
    assert_eq!(drv.env.get("FOO"), parsed.env.get("FOO"));
}

// StorePath and Output tests

#[test]
fn test_store_path() {
    let hash = Hash::of(b"test derivation");
    let path = StorePath::new(hash, "hello-2.12.1".to_string());
    
    assert_eq!(path.name(), "hello-2.12.1");
    assert!(path.path().to_string_lossy().contains("hello-2.12.1"));
}

#[test]
fn test_store_path_from_derivation() {
    let drv_hash = Hash::of(b"derivation content");
    let path = StorePath::from_derivation(drv_hash, "mypackage-1.0");
    
    assert_eq!(path.name(), "mypackage-1.0");
}

#[test]
fn test_output() {
    let out = Output::new("out");
    assert!(!out.is_fixed());
    
    let fixed = Output::fixed("out", Hash::of(b"expected"), HashMode::Flat);
    assert!(fixed.is_fixed());
}

// Version and resolution tests

fn make_pkg(name: &str, version: &str, deps: Vec<(&str, &str)>) -> PackageMetadata {
    PackageMetadata {
        id: PackageId::new(name, Version::parse(version).unwrap()),
        dependencies: deps.into_iter()
            .map(|(n, c)| Dependency::new(n, VersionConstraint::parse(c).unwrap()))
            .collect(),
        derivation_path: None,
    }
}

#[test]
fn test_version_parse() {
    let v = Version::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);

    let v = Version::parse("1.0").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);

    let v = Version::parse("2.0.0-beta").unwrap();
    assert_eq!(v.major, 2);
    assert_eq!(v.pre, Some("beta".to_string()));
}

#[test]
fn test_version_constraint() {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.3.0").unwrap();
    let v3 = Version::parse("2.0.0").unwrap();

    let c = VersionConstraint::parse("^1.2.0").unwrap();
    assert!(c.matches(&v1));
    assert!(c.matches(&v2));
    assert!(!c.matches(&v3));

    let c = VersionConstraint::parse(">=1.2.3").unwrap();
    assert!(c.matches(&v1));
    assert!(c.matches(&v2));
    assert!(c.matches(&v3));

    let c = VersionConstraint::parse("<2.0.0").unwrap();
    assert!(c.matches(&v1));
    assert!(c.matches(&v2));
    assert!(!c.matches(&v3));
}

#[test]
fn test_simple_resolution() {
    let mut registry = MemoryRegistry::new();
    registry.add(make_pkg("foo", "1.0.0", vec![]));
    registry.add(make_pkg("bar", "2.0.0", vec![("foo", "^1.0")]));

    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("bar", VersionConstraint::parse("^2.0").unwrap()),
    ];

    let resolution = resolver.resolve(&deps).unwrap();
    
    assert!(resolution.packages.contains_key("foo"));
    assert!(resolution.packages.contains_key("bar"));
    assert_eq!(resolution.packages["foo"].version, Version::parse("1.0.0").unwrap());
    assert_eq!(resolution.packages["bar"].version, Version::parse("2.0.0").unwrap());
}

#[test]
fn test_version_selection() {
    let mut registry = MemoryRegistry::new();
    registry.add(make_pkg("foo", "1.0.0", vec![]));
    registry.add(make_pkg("foo", "1.1.0", vec![]));
    registry.add(make_pkg("foo", "1.2.0", vec![]));
    registry.add(make_pkg("foo", "2.0.0", vec![]));

    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("foo", VersionConstraint::parse("^1.0").unwrap()),
    ];

    let resolution = resolver.resolve(&deps).unwrap();
    
    // Should select latest compatible version
    assert_eq!(resolution.packages["foo"].version, Version::parse("1.2.0").unwrap());
}

#[test]
fn test_build_order() {
    let mut registry = MemoryRegistry::new();
    registry.add(make_pkg("a", "1.0.0", vec![]));
    registry.add(make_pkg("b", "1.0.0", vec![("a", "^1.0")]));
    registry.add(make_pkg("c", "1.0.0", vec![("b", "^1.0")]));

    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("c", VersionConstraint::parse("^1.0").unwrap()),
    ];

    let resolution = resolver.resolve(&deps).unwrap();
    
    // Build order should be a, b, c
    let names: Vec<_> = resolution.build_order.iter().map(|p| p.name.as_str()).collect();
    let a_pos = names.iter().position(|n| *n == "a").unwrap();
    let b_pos = names.iter().position(|n| *n == "b").unwrap();
    let c_pos = names.iter().position(|n| *n == "c").unwrap();
    
    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}

#[test]
fn test_cycle_detection() {
    let mut registry = MemoryRegistry::new();
    registry.add(make_pkg("a", "1.0.0", vec![("b", "^1.0")]));
    registry.add(make_pkg("b", "1.0.0", vec![("a", "^1.0")]));

    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("a", VersionConstraint::parse("^1.0").unwrap()),
    ];

    let result = resolver.resolve(&deps);
    assert!(matches!(result, Err(ResolveError::CyclicDependency(_))));
}

#[test]
fn test_package_not_found() {
    let registry = MemoryRegistry::new();
    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("nonexistent", VersionConstraint::Any),
    ];

    let result = resolver.resolve(&deps);
    assert!(matches!(result, Err(ResolveError::PackageNotFound(_))));
}

#[test]
fn test_diamond_dependency() {
    // A depends on B and C
    // B depends on D ^1.0
    // C depends on D ^1.1
    // Should resolve to D 1.1 or higher
    let mut registry = MemoryRegistry::new();
    registry.add(make_pkg("d", "1.0.0", vec![]));
    registry.add(make_pkg("d", "1.1.0", vec![]));
    registry.add(make_pkg("d", "1.2.0", vec![]));
    registry.add(make_pkg("b", "1.0.0", vec![("d", "^1.0")]));
    registry.add(make_pkg("c", "1.0.0", vec![("d", "^1.1")]));
    registry.add(make_pkg("a", "1.0.0", vec![("b", "^1.0"), ("c", "^1.0")]));

    let resolver = Resolver::new(&registry);
    let deps = vec![
        Dependency::new("a", VersionConstraint::parse("^1.0").unwrap()),
    ];

    let resolution = resolver.resolve(&deps).unwrap();
    
    // D should be >= 1.1.0
    assert!(resolution.packages["d"].version >= Version::parse("1.1.0").unwrap());
}
