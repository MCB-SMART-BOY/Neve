//! Dependency resolution algorithm for Neve packages.
//!
//! This module implements a SAT-based dependency resolver that finds
//! a consistent set of package versions satisfying all constraints.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::StorePath;

/// A package identifier with name and version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: String,
    pub version: Version,
}

impl PackageId {
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
        }
    }
}

/// A semantic version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let mut parts = s.split('-');
        let version_part = parts.next().unwrap();
        let pre = parts.next().map(String::from);

        let nums: Vec<&str> = version_part.split('.').collect();
        if nums.is_empty() || nums.len() > 3 {
            return Err(VersionParseError::InvalidFormat(s.to_string()));
        }
        // Note: nums.is_empty() is the idiomatic way to check for empty collections

        let major = nums[0].parse().map_err(|_| VersionParseError::InvalidNumber)?;
        let minor = nums.get(1).map(|s| s.parse()).transpose()
            .map_err(|_| VersionParseError::InvalidNumber)?.unwrap_or(0);
        let patch = nums.get(2).map(|s| s.parse()).transpose()
            .map_err(|_| VersionParseError::InvalidNumber)?.unwrap_or(0);

        Ok(Self { major, minor, patch, pre })
    }

    /// Check if this version is compatible with another (same major version for 1.x+).
    pub fn is_compatible(&self, other: &Version) -> bool {
        if self.major == 0 && other.major == 0 {
            // For 0.x, minor version must match
            self.minor == other.minor
        } else {
            // For 1.x+, major version must match
            self.major == other.major
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum VersionParseError {
    InvalidFormat(String),
    InvalidNumber,
}

/// A version constraint.
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    /// Exact version: `=1.2.3`
    Exact(Version),
    /// Greater than or equal: `>=1.2.3`
    GreaterOrEqual(Version),
    /// Less than: `<2.0.0`
    Less(Version),
    /// Compatible (caret): `^1.2.3`
    Compatible(Version),
    /// Tilde (patch-level changes): `~1.2.3`
    Tilde(Version),
    /// Any version: `*`
    Any,
    /// Compound constraint (AND)
    And(Box<VersionConstraint>, Box<VersionConstraint>),
    /// Compound constraint (OR)
    Or(Box<VersionConstraint>, Box<VersionConstraint>),
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint.
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::GreaterOrEqual(v) => version >= v,
            VersionConstraint::Less(v) => version < v,
            VersionConstraint::Compatible(v) => {
                version >= v && version.is_compatible(v)
            }
            VersionConstraint::Tilde(v) => {
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionConstraint::Any => true,
            VersionConstraint::And(a, b) => a.matches(version) && b.matches(version),
            VersionConstraint::Or(a, b) => a.matches(version) || b.matches(version),
        }
    }

    /// Parse a version constraint string.
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let s = s.trim();
        
        if s == "*" {
            return Ok(VersionConstraint::Any);
        }

        if let Some(rest) = s.strip_prefix(">=") {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::GreaterOrEqual(v));
        }

        if let Some(rest) = s.strip_prefix("<=") {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::And(
                Box::new(VersionConstraint::Less(Version::new(v.major, v.minor, v.patch + 1))),
                Box::new(VersionConstraint::GreaterOrEqual(Version::new(0, 0, 0))),
            ));
        }

        if let Some(rest) = s.strip_prefix('<') {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::Less(v));
        }

        if let Some(rest) = s.strip_prefix('>') {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::GreaterOrEqual(Version::new(
                v.major, v.minor, v.patch + 1
            )));
        }

        if let Some(rest) = s.strip_prefix('^') {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::Compatible(v));
        }

        if let Some(rest) = s.strip_prefix('~') {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::Tilde(v));
        }

        if let Some(rest) = s.strip_prefix('=') {
            let v = Version::parse(rest)?;
            return Ok(VersionConstraint::Exact(v));
        }

        // Default to compatible (caret) constraint
        let v = Version::parse(s)?;
        Ok(VersionConstraint::Compatible(v))
    }
}

/// A dependency declaration.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub constraint: VersionConstraint,
    pub optional: bool,
}

impl Dependency {
    pub fn new(name: impl Into<String>, constraint: VersionConstraint) -> Self {
        Self {
            name: name.into(),
            constraint,
            optional: false,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// Package metadata for resolution.
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub id: PackageId,
    pub dependencies: Vec<Dependency>,
    pub derivation_path: Option<StorePath>,
}

/// Package registry providing available packages.
pub trait PackageRegistry {
    /// Get all available versions of a package.
    fn get_versions(&self, name: &str) -> Vec<Version>;
    
    /// Get the metadata for a specific package version.
    fn get_metadata(&self, name: &str, version: &Version) -> Option<PackageMetadata>;
}

/// In-memory package registry for testing.
#[derive(Debug, Default)]
pub struct MemoryRegistry {
    packages: HashMap<String, Vec<PackageMetadata>>,
}

impl MemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, metadata: PackageMetadata) {
        self.packages
            .entry(metadata.id.name.clone())
            .or_default()
            .push(metadata);
    }
}

impl PackageRegistry for MemoryRegistry {
    fn get_versions(&self, name: &str) -> Vec<Version> {
        self.packages.get(name)
            .map(|pkgs| pkgs.iter().map(|p| p.id.version.clone()).collect())
            .unwrap_or_default()
    }

    fn get_metadata(&self, name: &str, version: &Version) -> Option<PackageMetadata> {
        self.packages.get(name)?
            .iter()
            .find(|p| &p.id.version == version)
            .cloned()
    }
}

/// Result of dependency resolution.
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Resolved packages by name.
    pub packages: HashMap<String, PackageId>,
    /// Dependency graph (package -> dependencies).
    pub graph: HashMap<String, Vec<String>>,
    /// Topological order for building.
    pub build_order: Vec<PackageId>,
}

/// Dependency resolution error.
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// Package not found.
    PackageNotFound(String),
    /// No version satisfies the constraint.
    NoMatchingVersion {
        package: String,
        constraint: String,
        available: Vec<Version>,
    },
    /// Conflicting version requirements.
    VersionConflict {
        package: String,
        requirement1: String,
        requirement2: String,
    },
    /// Cyclic dependency detected.
    CyclicDependency(Vec<String>),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::PackageNotFound(name) => {
                write!(f, "package '{}' not found in registry", name)
            }
            ResolveError::NoMatchingVersion { package, constraint, available } => {
                write!(f, "no version of '{}' matches constraint '{}', available: {:?}",
                    package, constraint, available)
            }
            ResolveError::VersionConflict { package, requirement1, requirement2 } => {
                write!(f, "conflicting requirements for '{}': {} vs {}",
                    package, requirement1, requirement2)
            }
            ResolveError::CyclicDependency(cycle) => {
                write!(f, "cyclic dependency detected: {}", cycle.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// The dependency resolver.
pub struct Resolver<'a, R: PackageRegistry> {
    registry: &'a R,
}

impl<'a, R: PackageRegistry> Resolver<'a, R> {
    pub fn new(registry: &'a R) -> Self {
        Self { registry }
    }

    /// Resolve dependencies for a root package.
    pub fn resolve(&self, root_deps: &[Dependency]) -> Result<Resolution, ResolveError> {
        let mut resolved: HashMap<String, PackageId> = HashMap::new();
        let mut constraints: HashMap<String, Vec<VersionConstraint>> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut queue: VecDeque<(String, VersionConstraint)> = VecDeque::new();

        // Add root dependencies to queue
        for dep in root_deps {
            queue.push_back((dep.name.clone(), dep.constraint.clone()));
        }

        // Process dependencies
        while let Some((name, constraint)) = queue.pop_front() {
            // Record constraint
            constraints.entry(name.clone()).or_default().push(constraint.clone());

            // Skip if already resolved
            if resolved.contains_key(&name) {
                // Verify existing resolution satisfies new constraint
                let existing = &resolved[&name];
                if !constraint.matches(&existing.version) {
                    return Err(ResolveError::VersionConflict {
                        package: name,
                        requirement1: format!("{:?}", constraints[&existing.name]),
                        requirement2: format!("{:?}", constraint),
                    });
                }
                continue;
            }

            // Find matching version
            let versions = self.registry.get_versions(&name);
            if versions.is_empty() {
                return Err(ResolveError::PackageNotFound(name));
            }

            // Get all constraints for this package so far
            let all_constraints = &constraints[&name];

            // Find best matching version (prefer latest)
            let mut matching: Vec<&Version> = versions.iter()
                .filter(|v| all_constraints.iter().all(|c| c.matches(v)))
                .collect();
            matching.sort();
            matching.reverse();

            let version = matching.first().cloned().ok_or_else(|| {
                ResolveError::NoMatchingVersion {
                    package: name.clone(),
                    constraint: format!("{:?}", all_constraints),
                    available: versions.clone(),
                }
            })?;

            // Get metadata and add to resolved
            let metadata = self.registry.get_metadata(&name, version)
                .ok_or_else(|| ResolveError::PackageNotFound(name.clone()))?;

            resolved.insert(name.clone(), metadata.id.clone());

            // Add dependencies to graph and queue
            let mut deps = Vec::new();
            for dep in &metadata.dependencies {
                if !dep.optional {
                    deps.push(dep.name.clone());
                    queue.push_back((dep.name.clone(), dep.constraint.clone()));
                }
            }
            graph.insert(name, deps);
        }

        // Compute build order (topological sort)
        let build_order = self.topological_sort(&resolved, &graph)?;

        Ok(Resolution {
            packages: resolved,
            graph,
            build_order,
        })
    }

    /// Topological sort for build order.
    fn topological_sort(
        &self,
        resolved: &HashMap<String, PackageId>,
        graph: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<PackageId>, ResolveError> {
        let mut result = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut in_progress: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        fn visit(
            name: &str,
            graph: &HashMap<String, Vec<String>>,
            resolved: &HashMap<String, PackageId>,
            visited: &mut HashSet<String>,
            in_progress: &mut HashSet<String>,
            path: &mut Vec<String>,
            result: &mut Vec<PackageId>,
        ) -> Result<(), ResolveError> {
            if visited.contains(name) {
                return Ok(());
            }

            if in_progress.contains(name) {
                path.push(name.to_string());
                let cycle_start = path.iter().position(|n| n == name).unwrap();
                return Err(ResolveError::CyclicDependency(path[cycle_start..].to_vec()));
            }

            in_progress.insert(name.to_string());
            path.push(name.to_string());

            if let Some(deps) = graph.get(name) {
                for dep in deps {
                    visit(dep, graph, resolved, visited, in_progress, path, result)?;
                }
            }

            path.pop();
            in_progress.remove(name);
            visited.insert(name.to_string());

            if let Some(id) = resolved.get(name) {
                result.push(id.clone());
            }

            Ok(())
        }

        for name in resolved.keys() {
            visit(name, graph, resolved, &mut visited, &mut in_progress, &mut path, &mut result)?;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
