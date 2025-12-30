//! Build analytics and visualization for Neve.
//! Neve 的构建分析和可视化。
//!
//! This module provides advanced build analytics that go beyond Nix:
//! 此模块提供超越 Nix 的高级构建分析：
//!
//! - Build timing and performance analysis
//!   构建时间和性能分析
//! - Dependency graph visualization
//!   依赖图可视化
//! - Cache hit/miss statistics
//!   缓存命中/未命中统计
//! - Build parallelization opportunities
//!   构建并行化机会
//! - Resource usage tracking
//!   资源使用跟踪

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Unique identifier for a build.
/// 构建的唯一标识符。
pub type BuildId = String;

/// Build event for tracking.
/// 用于跟踪的构建事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildEvent {
    /// Build started. / 构建开始。
    Started { id: BuildId, name: String },
    /// Build completed successfully. / 构建成功完成。
    Completed { id: BuildId, duration_ms: u64 },
    /// Build failed. / 构建失败。
    Failed { id: BuildId, error: String },
    /// Cache hit (no rebuild needed). / 缓存命中（无需重建）。
    CacheHit { id: BuildId },
    /// Phase started. / 阶段开始。
    PhaseStarted { id: BuildId, phase: String },
    /// Phase completed. / 阶段完成。
    PhaseCompleted {
        id: BuildId,
        phase: String,
        duration_ms: u64,
    },
    /// Download started. / 下载开始。
    DownloadStarted { id: BuildId, url: String, size: u64 },
    /// Download completed. / 下载完成。
    DownloadCompleted { id: BuildId, url: String },
}

/// Build statistics for a single derivation.
/// 单个推导的构建统计。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildStats {
    /// Derivation name. / 推导名称。
    pub name: String,
    /// Total build time. / 总构建时间。
    pub total_duration: Duration,
    /// Time spent in each phase. / 每个阶段花费的时间。
    pub phase_times: BTreeMap<String, Duration>,
    /// Whether this was a cache hit. / 是否为缓存命中。
    pub cache_hit: bool,
    /// Download time (if applicable). / 下载时间（如果适用）。
    pub download_time: Option<Duration>,
    /// Download size in bytes. / 下载大小（字节）。
    pub download_size: u64,
    /// Number of dependencies. / 依赖数量。
    pub dependency_count: usize,
    /// Peak memory usage in bytes. / 峰值内存使用量（字节）。
    pub peak_memory: u64,
    /// CPU time in seconds. / CPU 时间（秒）。
    pub cpu_time: f64,
}

/// Aggregated build analytics.
/// 聚合的构建分析。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildAnalytics {
    /// Individual build statistics. / 单个构建统计。
    pub builds: HashMap<BuildId, BuildStats>,
    /// Total build time. / 总构建时间。
    pub total_time: Duration,
    /// Number of cache hits. / 缓存命中数。
    pub cache_hits: usize,
    /// Number of cache misses (actual builds). / 缓存未命中数（实际构建）。
    pub cache_misses: usize,
    /// Total download size. / 总下载大小。
    pub total_download_size: u64,
    /// Total download time. / 总下载时间。
    pub total_download_time: Duration,
    /// Build events log. / 构建事件日志。
    pub events: Vec<BuildEvent>,
    /// Dependency graph edges. / 依赖图边。
    pub dependency_edges: Vec<(BuildId, BuildId)>,
    /// Critical path (longest path through dependency graph).
    /// 关键路径（通过依赖图的最长路径）。
    pub critical_path: Vec<BuildId>,
    /// Parallelization efficiency (0.0 to 1.0).
    /// 并行化效率（0.0 到 1.0）。
    pub parallelization_efficiency: f64,
}

impl BuildAnalytics {
    /// Create new empty analytics.
    /// 创建新的空分析。
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a build event.
    /// 记录构建事件。
    pub fn record_event(&mut self, event: BuildEvent) {
        self.events.push(event.clone());

        match event {
            BuildEvent::Started { id, name } => {
                self.builds.entry(id).or_default().name = name;
            }
            BuildEvent::Completed { id, duration_ms } => {
                if let Some(stats) = self.builds.get_mut(&id) {
                    stats.total_duration = Duration::from_millis(duration_ms);
                    self.cache_misses += 1;
                }
            }
            BuildEvent::Failed { .. } => {
                self.cache_misses += 1;
            }
            BuildEvent::CacheHit { id } => {
                if let Some(stats) = self.builds.get_mut(&id) {
                    stats.cache_hit = true;
                }
                self.cache_hits += 1;
            }
            BuildEvent::PhaseCompleted {
                id,
                phase,
                duration_ms,
            } => {
                if let Some(stats) = self.builds.get_mut(&id) {
                    stats
                        .phase_times
                        .insert(phase, Duration::from_millis(duration_ms));
                }
            }
            BuildEvent::DownloadStarted { id, size, .. } => {
                if let Some(stats) = self.builds.get_mut(&id) {
                    stats.download_size = size;
                }
                self.total_download_size += size;
            }
            BuildEvent::DownloadCompleted { .. } => {}
            BuildEvent::PhaseStarted { .. } => {}
        }
    }

    /// Add a dependency edge.
    /// 添加依赖边。
    pub fn add_dependency(&mut self, from: BuildId, to: BuildId) {
        self.dependency_edges.push((from, to));
    }

    /// Calculate the critical path through the build graph.
    /// 计算通过构建图的关键路径。
    pub fn calculate_critical_path(&mut self) {
        // Build adjacency list
        // 构建邻接表
        let mut adj: HashMap<&BuildId, Vec<&BuildId>> = HashMap::new();
        let mut in_degree: HashMap<&BuildId, usize> = HashMap::new();

        for (from, to) in &self.dependency_edges {
            adj.entry(from).or_default().push(to);
            *in_degree.entry(to).or_default() += 1;
            in_degree.entry(from).or_default();
        }

        // Find longest path using dynamic programming
        // 使用动态规划找到最长路径
        let mut longest_path: HashMap<&BuildId, (Duration, Vec<&BuildId>)> = HashMap::new();

        // Topological sort
        // 拓扑排序
        let mut queue: Vec<&BuildId> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();

        while let Some(node) = queue.pop() {
            let node_duration = self
                .builds
                .get(node)
                .map(|s| s.total_duration)
                .unwrap_or_default();

            let (best_duration, best_path) = adj
                .get(node)
                .into_iter()
                .flat_map(|neighbors| neighbors.iter())
                .filter_map(|&neighbor| longest_path.get(neighbor))
                .max_by_key(|(d, _)| *d)
                .cloned()
                .unwrap_or((Duration::ZERO, Vec::new()));

            let mut path = vec![node];
            path.extend(best_path);
            longest_path.insert(node, (best_duration + node_duration, path));

            for neighbor in adj.get(node).into_iter().flat_map(|n| n.iter()) {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }

        // Find the longest path overall
        // 找到总体最长路径
        if let Some((_, (_, path))) = longest_path.iter().max_by_key(|(_, (d, _))| *d) {
            self.critical_path = path.iter().map(|&s| s.clone()).collect();
        }
    }

    /// Calculate parallelization efficiency.
    /// 计算并行化效率。
    pub fn calculate_efficiency(&mut self) {
        let total_work: Duration = self
            .builds
            .values()
            .filter(|s| !s.cache_hit)
            .map(|s| s.total_duration)
            .sum();

        if self.total_time.as_secs_f64() > 0.0 {
            self.parallelization_efficiency =
                total_work.as_secs_f64() / self.total_time.as_secs_f64();
        }
    }

    /// Get cache hit rate as percentage.
    /// 获取缓存命中率（百分比）。
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get the slowest builds.
    /// 获取最慢的构建。
    pub fn slowest_builds(&self, n: usize) -> Vec<(&BuildId, &BuildStats)> {
        let mut builds: Vec<_> = self.builds.iter().filter(|(_, s)| !s.cache_hit).collect();
        builds.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));
        builds.into_iter().take(n).collect()
    }

    /// Get builds that could benefit from caching.
    /// 获取可以从缓存中受益的构建。
    pub fn cache_opportunities(&self) -> Vec<(&BuildId, &BuildStats)> {
        self.builds
            .iter()
            .filter(|(_, s)| !s.cache_hit && s.total_duration > Duration::from_secs(60))
            .collect()
    }

    /// Generate a summary report.
    /// 生成摘要报告。
    pub fn summary_report(&self) -> String {
        let mut report = String::new();

        report.push_str("╔══════════════════════════════════════════════════════════════╗\n");
        report.push_str("║                    Neve Build Analytics                      ║\n");
        report.push_str("║                    Neve 构建分析报告                         ║\n");
        report.push_str("╠══════════════════════════════════════════════════════════════╣\n");

        report.push_str(&format!(
            "║ Total builds 总构建数: {:>40} ║\n",
            self.builds.len()
        ));
        report.push_str(&format!(
            "║ Cache hits 缓存命中: {:>42} ║\n",
            self.cache_hits
        ));
        report.push_str(&format!(
            "║ Cache misses 缓存未命中: {:>38} ║\n",
            self.cache_misses
        ));
        report.push_str(&format!(
            "║ Cache hit rate 缓存命中率: {:>34.1}% ║\n",
            self.cache_hit_rate()
        ));
        report.push_str(&format!(
            "║ Total time 总时间: {:>43} ║\n",
            format_duration(self.total_time)
        ));
        report.push_str(&format!(
            "║ Download size 下载大小: {:>39} ║\n",
            format_size(self.total_download_size)
        ));
        report.push_str(&format!(
            "║ Parallelization efficiency 并行化效率: {:>20.1}% ║\n",
            self.parallelization_efficiency * 100.0
        ));

        report.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        report.push_str("║ Slowest builds 最慢构建:                                     ║\n");

        for (_id, stats) in self.slowest_builds(5) {
            let name = if stats.name.len() > 30 {
                format!("{}...", &stats.name[..27])
            } else {
                stats.name.clone()
            };
            report.push_str(&format!(
                "║   {:<35} {:>20} ║\n",
                name,
                format_duration(stats.total_duration)
            ));
        }

        if !self.critical_path.is_empty() {
            report.push_str("╠══════════════════════════════════════════════════════════════╣\n");
            report.push_str("║ Critical path 关键路径:                                      ║\n");

            for (i, id) in self.critical_path.iter().take(5).enumerate() {
                let name = self
                    .builds
                    .get(id)
                    .map(|s| s.name.as_str())
                    .unwrap_or(id.as_str());
                let display_name = if name.len() > 50 {
                    format!("{}...", &name[..47])
                } else {
                    name.to_string()
                };
                report.push_str(&format!("║   {}. {:<55} ║\n", i + 1, display_name));
            }
        }

        report.push_str("╚══════════════════════════════════════════════════════════════╝\n");
        report
    }

    /// Export analytics as JSON.
    /// 将分析导出为 JSON。
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export dependency graph in DOT format for visualization.
    /// 以 DOT 格式导出依赖图以供可视化。
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph neve_build {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Add nodes with styling based on cache hit
        // 根据缓存命中添加带样式的节点
        for (id, stats) in &self.builds {
            let color = if stats.cache_hit { "green" } else { "blue" };
            let label = format!("{}\\n{}", stats.name, format_duration(stats.total_duration));
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", color=\"{}\"];\n",
                id, label, color
            ));
        }

        dot.push_str("\n");

        // Add edges
        // 添加边
        for (from, to) in &self.dependency_edges {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", from, to));
        }

        dot.push_str("}\n");
        dot
    }

    /// Save analytics to a file.
    /// 将分析保存到文件。
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(path, json)
    }

    /// Load analytics from a file.
    /// 从文件加载分析。
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }
}

/// Build analytics collector for real-time tracking.
/// 用于实时跟踪的构建分析收集器。
pub struct AnalyticsCollector {
    analytics: BuildAnalytics,
    start_time: Instant,
    phase_starts: HashMap<(BuildId, String), Instant>,
    build_starts: HashMap<BuildId, Instant>,
}

impl AnalyticsCollector {
    /// Create a new collector.
    /// 创建新的收集器。
    pub fn new() -> Self {
        Self {
            analytics: BuildAnalytics::new(),
            start_time: Instant::now(),
            phase_starts: HashMap::new(),
            build_starts: HashMap::new(),
        }
    }

    /// Start tracking a build.
    /// 开始跟踪构建。
    pub fn start_build(&mut self, id: BuildId, name: String) {
        self.build_starts.insert(id.clone(), Instant::now());
        self.analytics
            .record_event(BuildEvent::Started { id, name });
    }

    /// Record a cache hit.
    /// 记录缓存命中。
    pub fn cache_hit(&mut self, id: BuildId) {
        self.build_starts.remove(&id);
        self.analytics.record_event(BuildEvent::CacheHit { id });
    }

    /// Complete a build successfully.
    /// 成功完成构建。
    pub fn complete_build(&mut self, id: BuildId) {
        let duration_ms = self
            .build_starts
            .remove(&id)
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);

        self.analytics
            .record_event(BuildEvent::Completed { id, duration_ms });
    }

    /// Record a build failure.
    /// 记录构建失败。
    pub fn fail_build(&mut self, id: BuildId, error: String) {
        self.build_starts.remove(&id);
        self.analytics
            .record_event(BuildEvent::Failed { id, error });
    }

    /// Start a build phase.
    /// 开始构建阶段。
    pub fn start_phase(&mut self, id: BuildId, phase: String) {
        self.phase_starts
            .insert((id.clone(), phase.clone()), Instant::now());
        self.analytics
            .record_event(BuildEvent::PhaseStarted { id, phase });
    }

    /// Complete a build phase.
    /// 完成构建阶段。
    pub fn complete_phase(&mut self, id: BuildId, phase: String) {
        let duration_ms = self
            .phase_starts
            .remove(&(id.clone(), phase.clone()))
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);

        self.analytics.record_event(BuildEvent::PhaseCompleted {
            id,
            phase,
            duration_ms,
        });
    }

    /// Add a dependency relationship.
    /// 添加依赖关系。
    pub fn add_dependency(&mut self, from: BuildId, to: BuildId) {
        self.analytics.add_dependency(from, to);
    }

    /// Finalize and get analytics.
    /// 完成并获取分析。
    pub fn finish(mut self) -> BuildAnalytics {
        self.analytics.total_time = self.start_time.elapsed();
        self.analytics.calculate_critical_path();
        self.analytics.calculate_efficiency();
        self.analytics
    }
}

impl Default for AnalyticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a duration for display.
/// 格式化持续时间以供显示。
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format a byte size for display.
/// 格式化字节大小以供显示。
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_basic() {
        let mut analytics = BuildAnalytics::new();

        analytics.record_event(BuildEvent::Started {
            id: "pkg1".to_string(),
            name: "package-1".to_string(),
        });
        analytics.record_event(BuildEvent::Completed {
            id: "pkg1".to_string(),
            duration_ms: 5000,
        });
        analytics.record_event(BuildEvent::CacheHit {
            id: "pkg2".to_string(),
        });

        assert_eq!(analytics.cache_hits, 1);
        assert_eq!(analytics.cache_misses, 1);
        assert_eq!(analytics.cache_hit_rate(), 50.0);
    }

    #[test]
    fn test_analytics_phases() {
        let mut analytics = BuildAnalytics::new();

        analytics.record_event(BuildEvent::Started {
            id: "pkg1".to_string(),
            name: "package-1".to_string(),
        });
        analytics.record_event(BuildEvent::PhaseCompleted {
            id: "pkg1".to_string(),
            phase: "configure".to_string(),
            duration_ms: 1000,
        });
        analytics.record_event(BuildEvent::PhaseCompleted {
            id: "pkg1".to_string(),
            phase: "build".to_string(),
            duration_ms: 3000,
        });

        let stats = analytics.builds.get("pkg1").unwrap();
        assert_eq!(stats.phase_times.len(), 2);
        assert_eq!(
            stats.phase_times.get("configure"),
            Some(&Duration::from_secs(1))
        );
    }

    #[test]
    fn test_collector() {
        let mut collector = AnalyticsCollector::new();

        collector.start_build("pkg1".to_string(), "package-1".to_string());
        collector.start_phase("pkg1".to_string(), "configure".to_string());
        std::thread::sleep(Duration::from_millis(10));
        collector.complete_phase("pkg1".to_string(), "configure".to_string());
        collector.complete_build("pkg1".to_string());

        let analytics = collector.finish();
        assert_eq!(analytics.cache_misses, 1);
        assert!(analytics.total_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_dot_export() {
        let mut analytics = BuildAnalytics::new();
        analytics.builds.insert(
            "pkg1".to_string(),
            BuildStats {
                name: "package-1".to_string(),
                total_duration: Duration::from_secs(10),
                ..Default::default()
            },
        );
        analytics.builds.insert(
            "pkg2".to_string(),
            BuildStats {
                name: "package-2".to_string(),
                cache_hit: true,
                ..Default::default()
            },
        );
        analytics.add_dependency("pkg1".to_string(), "pkg2".to_string());

        let dot = analytics.to_dot();
        assert!(dot.contains("digraph neve_build"));
        assert!(dot.contains("package-1"));
        assert!(dot.contains("package-2"));
        assert!(dot.contains("->"));
    }
}
