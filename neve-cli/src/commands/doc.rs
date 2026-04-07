//! Documentation viewer command.
//! 文档查看器命令。
//!
//! Provides man-like access to embedded documentation with terminal rendering.
//! 提供类似 man 的嵌入式文档访问，带有终端渲染。

use std::io::Write;
use termimad::MadSkin;

// Embed documentation at compile time
// 在编译时嵌入文档
const DOC_INDEX: &str = include_str!("../../../docs/README.md");
const DOC_QUICKSTART: &str = include_str!("../../../docs/user/quickstart.md");
const DOC_TUTORIAL: &str = include_str!("../../../docs/user/tutorial.md");
const DOC_SPEC: &str = include_str!("../../../docs/reference/spec.md");
const DOC_API: &str = include_str!("../../../docs/reference/api.md");
const DOC_DIAGNOSTICS: &str = include_str!("../../../docs/reference/diagnostics.md");
const DOC_PHILOSOPHY: &str = include_str!("../../../docs/project/philosophy.md");
const DOC_INSTALL: &str = include_str!("../../../docs/user/install.md");
const DOC_ARCHITECTURE: &str = include_str!("../../../docs/contributor/architecture.md");
const DOC_ONBOARDING: &str = include_str!("../../../docs/contributor/onboarding.md");
const DOC_CONTRIBUTING: &str = include_str!("../../../docs/contributor/contributing.md");
const DOC_BOOTSTRAP: &str = include_str!("../../../docs/contributor/bootstrap.md");
const DOC_ROADMAP: &str = include_str!("../../../docs/project/roadmap.md");
const DOC_LANGUAGE_ROADMAP: &str = include_str!("../../../docs/project/language-roadmap.md");
const DOC_FEATURE_MATRIX: &str = include_str!("../../../docs/project/feature-matrix.md");
const DOC_CHANGELOG: &str = include_str!("../../../docs/project/changelog.md");

/// Available documentation topics.
/// 可用的文档主题。
const TOPICS: &[(&str, &str, &str)] = &[
    ("index", DOC_INDEX, "Documentation hub / 文档首页"),
    (
        "quickstart",
        DOC_QUICKSTART,
        "5-minute quick start / 5 分钟快速上手",
    ),
    ("tutorial", DOC_TUTORIAL, "Complete tutorial / 完整教程"),
    ("spec", DOC_SPEC, "Language spec / 语言规范"),
    ("api", DOC_API, "Standard library API / 标准库 API"),
    (
        "diagnostics",
        DOC_DIAGNOSTICS,
        "Diagnostic codes / 诊断错误码",
    ),
    ("philosophy", DOC_PHILOSOPHY, "Design philosophy / 设计哲学"),
    ("install", DOC_INSTALL, "Installation guide / 安装指南"),
    ("architecture", DOC_ARCHITECTURE, "Architecture / 内部架构"),
    (
        "onboarding",
        DOC_ONBOARDING,
        "Contributor onboarding / 贡献者入门",
    ),
    (
        "contributing",
        DOC_CONTRIBUTING,
        "Contributor guide / 贡献指南",
    ),
    (
        "bootstrap",
        DOC_BOOTSTRAP,
        "Bootstrap examples / bootstrap 示例",
    ),
    ("roadmap", DOC_ROADMAP, "Project roadmap / 项目路线图"),
    (
        "language-roadmap",
        DOC_LANGUAGE_ROADMAP,
        "Language completion roadmap / 语言完备化路线图",
    ),
    (
        "feature-matrix",
        DOC_FEATURE_MATRIX,
        "Feature support matrix / 功能支持矩阵",
    ),
    ("changelog", DOC_CHANGELOG, "Changelog / 更新日志"),
];

/// Resolve topic name with aliases and prefix matching.
/// 解析主题名称（支持别名与前缀匹配）。
fn resolve_topic(input: &str) -> Option<&'static str> {
    let mut topic = input.trim().to_lowercase();
    topic = topic.replace('_', "-");

    let aliases: &[(&str, &str)] = &[
        ("qs", "quickstart"),
        ("docs", "index"),
        ("index", "index"),
        ("home", "index"),
        ("hub", "index"),
        ("quick", "quickstart"),
        ("getting-started", "quickstart"),
        ("intro", "quickstart"),
        ("start", "quickstart"),
        ("use", "quickstart"),
        ("learn", "tutorial"),
        ("ref", "spec"),
        ("reference", "spec"),
        ("stdlib", "api"),
        ("errors", "diagnostics"),
        ("diag", "diagnostics"),
        ("design", "philosophy"),
        ("guide", "tutorial"),
        ("arch", "architecture"),
        ("onboard", "onboarding"),
        ("contrib", "contributing"),
        ("contributors", "contributing"),
        ("boot", "bootstrap"),
        ("matrix", "feature-matrix"),
        ("features", "feature-matrix"),
        ("status", "feature-matrix"),
        ("project", "roadmap"),
        ("contribute", "contributing"),
        ("lang-roadmap", "language-roadmap"),
        ("change", "changelog"),
        ("changes", "changelog"),
        ("install", "install"),
    ];

    for (alias, name) in aliases {
        if topic == *alias {
            return Some(*name);
        }
    }

    if let Some((name, _, _)) = TOPICS.iter().find(|(name, _, _)| *name == topic) {
        return Some(*name);
    }

    let matches: Vec<&str> = TOPICS
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| name.starts_with(&topic))
        .collect();

    if matches.len() == 1 {
        return Some(matches[0]);
    }

    None
}

/// Create a styled skin for terminal rendering.
/// 为终端渲染创建样式化的皮肤。
fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default();

    // Customize colors for better readability
    // 自定义颜色以提高可读性
    skin.bold.set_fg(termimad::crossterm::style::Color::Cyan);
    skin.italic
        .set_fg(termimad::crossterm::style::Color::Magenta);
    skin.inline_code
        .set_fg(termimad::crossterm::style::Color::Green);
    skin.code_block
        .set_fg(termimad::crossterm::style::Color::Green);

    skin
}

/// List available documentation topics.
/// 列出可用的文档主题。
pub fn list() -> Result<(), String> {
    let skin = create_skin();
    let mut content = String::new();
    content.push_str("# NEVE DOCUMENTATION / 文档导航\n\n");
    content.push_str("## Usage / 用法\n\n");
    content.push_str("```\n");
    content.push_str("neve doc <topic>          View a topic / 查看主题\n");
    content.push_str("neve doc --list           List all topics / 列出主题\n");
    content.push_str("neve doc                 List topics (same as --list) / 同上\n");
    content.push_str("```\n\n");
    content.push_str("## Start Here / 从这里开始\n\n");
    content.push_str("```\n");
    content.push_str("neve doc index            Documentation hub / 文档首页\n");
    content.push_str("neve doc install          Installation guide / 安装指南\n");
    content.push_str("neve doc quickstart       Full quickstart guide / 完整快速入门\n");
    content.push_str("neve doc tutorial         Complete tutorial / 完整教程\n");
    content.push_str("```\n\n");
    content.push_str("## Reference / 参考\n\n");
    content.push_str("```\n");
    content.push_str("neve doc spec             Language spec / 语言规范\n");
    content.push_str("neve doc api              API reference / 标准库 API\n");
    content.push_str("neve doc diagnostics      Diagnostic codes / 诊断错误码\n");
    content.push_str("```\n\n");
    content.push_str("## Project / 项目现状\n\n");
    content.push_str("```\n");
    content.push_str("neve doc feature-matrix   Real support matrix / 真实功能矩阵\n");
    content.push_str("neve doc roadmap          Product roadmap / 项目路线图\n");
    content.push_str("neve doc language-roadmap Language completion roadmap / 语言完备化路线图\n");
    content.push_str("neve doc changelog        Released changes / 更新日志\n");
    content.push_str("```\n\n");
    content.push_str("## Contributor / 贡献者\n\n");
    content.push_str("```\n");
    content.push_str("neve doc contributing     Contributor guide / 贡献指南\n");
    content.push_str("neve doc onboarding       Codebase onboarding / 贡献者入门\n");
    content.push_str("neve doc architecture     Internal architecture / 内部架构\n");
    content.push_str("neve doc bootstrap        Bootstrap examples / bootstrap 示例\n");
    content.push_str("```\n");

    println!("{}", skin.term_text(&content));
    Ok(())
}

/// View a documentation topic.
/// 查看文档主题。
pub fn view(topic: &str) -> Result<(), String> {
    if matches!(topic, "list" | "help" | "topics") {
        return list();
    }

    let resolved = resolve_topic(topic);
    let content = resolved
        .and_then(|name| TOPICS.iter().find(|(n, _, _)| *n == name))
        .map(|(_, content, _)| *content);

    let content = match content {
        Some(c) => c,
        None => {
            eprintln!("Unknown topic: {}", topic);
            eprintln!("未知主题：{}", topic);
            eprintln!();
            eprintln!("Available topics / 可用主题:");
            for (name, _, desc) in TOPICS {
                eprintln!("  {:12} - {}", name, desc);
            }
            return Ok(());
        }
    };

    let cleaned = clean_markdown(content);

    // Render with termimad
    // 使用 termimad 渲染
    let skin = create_skin();
    let rendered = skin.term_text(&cleaned);

    // Try to use a pager for better reading experience
    // 尝试使用分页器以获得更好的阅读体验
    if try_pager(&rendered.to_string()).is_err() {
        // Fallback to direct output
        // 回退到直接输出
        println!("{}", rendered);
    }

    Ok(())
}

/// Clean up markdown for better terminal rendering.
/// 清理 markdown 以获得更好的终端渲染效果。
fn clean_markdown(content: &str) -> String {
    let mut out = Vec::new();
    let mut in_code_block = false;
    let mut last_was_hr = false;

    for raw in content.lines() {
        let line = raw.trim_end();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            out.push(line.to_string());
            last_was_hr = false;
            continue;
        }

        if !in_code_block {
            let trimmed = line.trim();
            if trimmed.starts_with('<') {
                continue;
            }
            if trimmed == "---" {
                if out.is_empty() || last_was_hr {
                    continue;
                }
                last_was_hr = true;
                out.push("---".to_string());
                continue;
            }
            if trimmed.is_empty() && out.is_empty() {
                continue;
            }
            last_was_hr = false;
        }

        out.push(line.to_string());
    }

    out.join("\n")
}

/// Try to display content using a pager (less, more, etc.).
/// 尝试使用分页器（less、more 等）显示内容。
fn try_pager(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try to find a pager
    // 尝试查找分页器
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());

    // Try 'less' with some nice options for colored output
    // 尝试使用带有彩色输出选项的 'less'
    let pagers = [
        (pager.as_str(), vec!["-R", "-S"]),
        ("less", vec!["-R", "-S"]),
        ("more", vec![]),
    ];

    for (cmd, args) in pagers {
        if let Ok(mut child) = std::process::Command::new(cmd)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(content.as_bytes());
            }
            let _ = child.wait();
            return Ok(());
        }
    }

    // No pager found, return error to trigger fallback
    // 未找到分页器，返回错误以触发回退
    Err("No pager available".into())
}
