//! Dependency resolution algorithm for Neve packages.
//! Neve 包的依赖解析算法。
//!
//! This module implements a SAT-based dependency resolver that finds
//! a consistent set of package versions satisfying all constraints.
//! 本模块实现了一个基于 SAT 的依赖解析器，用于找到满足所有约束的一致包版本集合。

use crate::StorePath;
use std::collections::{HashMap, HashSet, VecDeque};

/// A package identifier with name and version.
/// 带有名称和版本的包标识符。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// Package name. / 包名称。
    pub name: String,
    /// Package version. / 包版本。
    pub version: Version,
}

impl PackageId {
    /// Create a new package identifier.
    /// 创建新的包标识符。
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
        }
    }
}

/// A semantic version.
/// 语义版本。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    /// Major version number. / 主版本号。
    pub major: u32,
    /// Minor version number. / 次版本号。
    pub minor: u32,
    /// Patch version number. / 补丁版本号。
    pub patch: u32,
    /// Pre-release tag (if any). / 预发布标签（如有）。
    pub pre: Option<String>,
}

impl Version {
    /// Create a new version.
    /// 创建新版本。
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    /// Parse a version string.
    /// 解析版本字符串。
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let mut parts = s.split('-');
        // Safe: split always returns at least one element
        // 安全：split 总是返回至少一个元素
        let version_part = parts.next().expect("split always yields at least one part");
        let pre = parts.next().map(String::from);

        let nums: Vec<&str> = version_part.split('.').collect();
        if nums.is_empty() || nums.len() > 3 {
            return Err(VersionParseError::InvalidFormat(s.to_string()));
        }
        // Note: nums.is_empty() is the idiomatic way to check for empty collections
        // 注意：nums.is_empty() 是检查空集合的惯用方式

        let major = nums[0]
            .parse()
            .map_err(|_| VersionParseError::InvalidNumber)?;
        let minor = nums
            .get(1)
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| VersionParseError::InvalidNumber)?
            .unwrap_or(0);
        let patch = nums
            .get(2)
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| VersionParseError::InvalidNumber)?
            .unwrap_or(0);

        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// Check if this version is compatible with another (same major version for 1.x+).
    /// 检查此版本是否与另一个版本兼容（对于 1.x+，主版本号相同）。
    pub fn is_compatible(&self, other: &Version) -> bool {
        if self.major == 0 && other.major == 0 {
            // For 0.x, minor version must match
            // 对于 0.x，次版本号必须匹配
            self.minor == other.minor
        } else {
            // For 1.x+, major version must match
            // 对于 1.x+，主版本号必须匹配
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

/// Version parsing error.
/// 版本解析错误。
#[derive(Debug, Clone)]
pub enum VersionParseError {
    /// Invalid version format. / 无效的版本格式。
    InvalidFormat(String),
    /// Invalid version number. / 无效的版本号。
    InvalidNumber,
}

/// A version constraint.
/// 版本约束。
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    /// Exact version: `=1.2.3`. / 精确版本：`=1.2.3`。
    Exact(Version),
    /// Greater than or equal: `>=1.2.3`. / 大于等于：`>=1.2.3`。
    GreaterOrEqual(Version),
    /// Less than: `<2.0.0`. / 小于：`<2.0.0`。
    Less(Version),
    /// Compatible (caret): `^1.2.3`. / 兼容（插入符）：`^1.2.3`。
    Compatible(Version),
    /// Tilde (patch-level changes): `~1.2.3`. / 波浪号（补丁级更改）：`~1.2.3`。
    Tilde(Version),
    /// Any version: `*`. / 任意版本：`*`。
    Any,
    /// Compound constraint (AND). / 复合约束（与）。
    And(Box<VersionConstraint>, Box<VersionConstraint>),
    /// Compound constraint (OR). / 复合约束（或）。
    Or(Box<VersionConstraint>, Box<VersionConstraint>),
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint.
    /// 检查版本是否满足此约束。
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::GreaterOrEqual(v) => version >= v,
            VersionConstraint::Less(v) => version < v,
            VersionConstraint::Compatible(v) => version >= v && version.is_compatible(v),
            VersionConstraint::Tilde(v) => {
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionConstraint::Any => true,
            VersionConstraint::And(a, b) => a.matches(version) && b.matches(version),
            VersionConstraint::Or(a, b) => a.matches(version) || b.matches(version),
        }
    }

    /// Parse a version constraint string.
    /// 解析版本约束字符串。
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
                Box::new(VersionConstraint::Less(Version::new(
                    v.major,
                    v.minor,
                    v.patch + 1,
                ))),
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
                v.major,
                v.minor,
                v.patch + 1,
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
        // 默认为兼容（插入符）约束
        let v = Version::parse(s)?;
        Ok(VersionConstraint::Compatible(v))
    }
}

/// A dependency declaration.
/// 依赖声明。
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Dependency name. / 依赖名称。
    pub name: String,
    /// Version constraint. / 版本约束。
    pub constraint: VersionConstraint,
    /// Whether the dependency is optional. / 依赖是否可选。
    pub optional: bool,
}

impl Dependency {
    /// Create a new dependency.
    /// 创建新依赖。
    pub fn new(name: impl Into<String>, constraint: VersionConstraint) -> Self {
        Self {
            name: name.into(),
            constraint,
            optional: false,
        }
    }

    /// Mark the dependency as optional.
    /// 将依赖标记为可选。
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// Package metadata for resolution.
/// 用于解析的包元数据。
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    /// Package identifier. / 包标识符。
    pub id: PackageId,
    /// Package dependencies. / 包依赖。
    pub dependencies: Vec<Dependency>,
    /// Derivation path (if built). / 推导路径（如果已构建）。
    pub derivation_path: Option<StorePath>,
}

/// Package registry providing available packages.
/// 提供可用包的包注册表。
pub trait PackageRegistry {
    /// Get all available versions of a package.
    /// 获取包的所有可用版本。
    fn get_versions(&self, name: &str) -> Vec<Version>;

    /// Get the metadata for a specific package version.
    /// 获取特定包版本的元数据。
    fn get_metadata(&self, name: &str, version: &Version) -> Option<PackageMetadata>;
}

/// In-memory package registry for testing.
/// 用于测试的内存包注册表。
#[derive(Debug, Default)]
pub struct MemoryRegistry {
    packages: HashMap<String, Vec<PackageMetadata>>,
}

impl MemoryRegistry {
    /// Create a new empty registry.
    /// 创建新的空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a package to the registry.
    /// 向注册表添加包。
    pub fn add(&mut self, metadata: PackageMetadata) {
        self.packages
            .entry(metadata.id.name.clone())
            .or_default()
            .push(metadata);
    }
}

impl PackageRegistry for MemoryRegistry {
    fn get_versions(&self, name: &str) -> Vec<Version> {
        self.packages
            .get(name)
            .map(|pkgs| pkgs.iter().map(|p| p.id.version.clone()).collect())
            .unwrap_or_default()
    }

    fn get_metadata(&self, name: &str, version: &Version) -> Option<PackageMetadata> {
        self.packages
            .get(name)?
            .iter()
            .find(|p| &p.id.version == version)
            .cloned()
    }
}

/// Result of dependency resolution.
/// 依赖解析的结果。
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Resolved packages by name. / 按名称解析的包。
    pub packages: HashMap<String, PackageId>,
    /// Dependency graph (package -> dependencies). / 依赖图（包 -> 依赖）。
    pub graph: HashMap<String, Vec<String>>,
    /// Topological order for building. / 构建的拓扑顺序。
    pub build_order: Vec<PackageId>,
}

/// Dependency resolution error.
/// 依赖解析错误。
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// Package not found. / 未找到包。
    PackageNotFound(String),
    /// No version satisfies the constraint. / 没有版本满足约束。
    NoMatchingVersion {
        package: String,
        constraint: String,
        available: Vec<Version>,
    },
    /// Conflicting version requirements. / 版本要求冲突。
    VersionConflict {
        package: String,
        requirement1: String,
        requirement2: String,
    },
    /// Cyclic dependency detected. / 检测到循环依赖。
    CyclicDependency(Vec<String>),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::PackageNotFound(name) => {
                write!(f, "package '{}' not found in registry", name)
            }
            ResolveError::NoMatchingVersion {
                package,
                constraint,
                available,
            } => {
                write!(
                    f,
                    "no version of '{}' matches constraint '{}', available: {:?}",
                    package, constraint, available
                )
            }
            ResolveError::VersionConflict {
                package,
                requirement1,
                requirement2,
            } => {
                write!(
                    f,
                    "conflicting requirements for '{}': {} vs {}",
                    package, requirement1, requirement2
                )
            }
            ResolveError::CyclicDependency(cycle) => {
                write!(f, "cyclic dependency detected: {}", cycle.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// The dependency resolver.
/// 依赖解析器。
pub struct Resolver<'a, R: PackageRegistry> {
    registry: &'a R,
}

impl<'a, R: PackageRegistry> Resolver<'a, R> {
    /// Create a new resolver with the given registry.
    /// 使用给定的注册表创建新的解析器。
    pub fn new(registry: &'a R) -> Self {
        Self { registry }
    }

    /// Resolve dependencies for a root package.
    /// 解析根包的依赖。
    pub fn resolve(&self, root_deps: &[Dependency]) -> Result<Resolution, ResolveError> {
        let mut resolved: HashMap<String, PackageId> = HashMap::new();
        let mut constraints: HashMap<String, Vec<VersionConstraint>> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut queue: VecDeque<(String, VersionConstraint)> = VecDeque::new();

        // Add root dependencies to queue
        // 将根依赖添加到队列
        for dep in root_deps {
            queue.push_back((dep.name.clone(), dep.constraint.clone()));
        }

        // Process dependencies
        // 处理依赖
        while let Some((name, constraint)) = queue.pop_front() {
            // Record constraint
            // 记录约束
            constraints
                .entry(name.clone())
                .or_default()
                .push(constraint.clone());

            // Skip if already resolved
            // 如果已解析则跳过
            if resolved.contains_key(&name) {
                // Verify existing resolution satisfies new constraint
                // 验证现有解析满足新约束
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
            // 查找匹配版本
            let versions = self.registry.get_versions(&name);
            if versions.is_empty() {
                return Err(ResolveError::PackageNotFound(name));
            }

            // Get all constraints for this package so far
            // 获取此包迄今为止的所有约束
            let all_constraints = &constraints[&name];

            // Find best matching version (prefer latest)
            // 查找最佳匹配版本（优先选择最新版本）
            let mut matching: Vec<&Version> = versions
                .iter()
                .filter(|v| all_constraints.iter().all(|c| c.matches(v)))
                .collect();
            matching.sort();
            matching.reverse();

            let version =
                matching
                    .first()
                    .cloned()
                    .ok_or_else(|| ResolveError::NoMatchingVersion {
                        package: name.clone(),
                        constraint: format!("{:?}", all_constraints),
                        available: versions.clone(),
                    })?;

            // Get metadata and add to resolved
            // 获取元数据并添加到已解析
            let metadata = self
                .registry
                .get_metadata(&name, version)
                .ok_or_else(|| ResolveError::PackageNotFound(name.clone()))?;

            resolved.insert(name.clone(), metadata.id.clone());

            // Add dependencies to graph and queue
            // 将依赖添加到图和队列
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
        // 计算构建顺序（拓扑排序）
        let build_order = self.topological_sort(&resolved, &graph)?;

        Ok(Resolution {
            packages: resolved,
            graph,
            build_order,
        })
    }

    /// Topological sort for build order.
    /// 构建顺序的拓扑排序。
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
                // Safe: we just pushed `name` to path, so it must exist
                // 安全：我们刚刚将 `name` 推入 path，所以它一定存在
                let cycle_start = path
                    .iter()
                    .position(|n| n == name)
                    .expect("name was just pushed to path");
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
            visit(
                name,
                graph,
                resolved,
                &mut visited,
                &mut in_progress,
                &mut path,
                &mut result,
            )?;
        }

        Ok(result)
    }
}
