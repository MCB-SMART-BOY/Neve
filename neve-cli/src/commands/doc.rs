//! Documentation viewer command.
//! 文档查看器命令。

use std::io::Write;
use termimad::MadSkin;

// Embed documentation at compile time — paths relative to this source file
// (neve-cli/src/commands/) via symlink neve-cli/docs/ → docs/
const DOC_INDEX: &str = include_str!("../../docs/README.md");
const DOC_QUICKSTART: &str = include_str!("../../docs/user/quickstart.md");
const DOC_TUTORIAL: &str = include_str!("../../docs/user/tutorial.md");
const DOC_SPEC: &str = include_str!("../../docs/reference/spec.md");
const DOC_API: &str = include_str!("../../docs/reference/api.md");
const DOC_DIAGNOSTICS: &str = include_str!("../../docs/reference/diagnostics.md");
const DOC_PHILOSOPHY: &str = include_str!("../../docs/project/philosophy.md");
const DOC_INSTALL: &str = include_str!("../../docs/user/install.md");
const DOC_ARCHITECTURE: &str = include_str!("../../docs/contributor/architecture.md");
const DOC_ONBOARDING: &str = include_str!("../../docs/contributor/onboarding.md");
const DOC_CONTRIBUTING: &str = include_str!("../../docs/contributor/contributing.md");
const DOC_FEATURE_MATRIX: &str = include_str!("../../docs/project/feature-matrix.md");
const DOC_CHANGELOG: &str = include_str!("../../docs/project/changelog.md");

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
        "feature-matrix",
        DOC_FEATURE_MATRIX,
        "Feature support matrix / 功能支持矩阵",
    ),
    ("changelog", DOC_CHANGELOG, "Changelog / 更新日志"),
];

fn resolve_topic(input: &str) -> Option<&'static str> {
    let topic = input.trim().to_lowercase().replace('_', "-");
    let aliases: &[(&str, &str)] = &[
        ("qs", "quickstart"),
        ("docs", "index"),
        ("home", "index"),
        ("quick", "quickstart"),
        ("start", "quickstart"),
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
        ("matrix", "feature-matrix"),
        ("features", "feature-matrix"),
        ("status", "feature-matrix"),
        ("change", "changelog"),
        ("changes", "changelog"),
        ("install", "install"),
        ("contrib", "contributing"),
    ];
    for (alias, target) in aliases {
        if topic == *alias {
            return Some(target);
        }
    }
    TOPICS
        .iter()
        .map(|(name, _, _)| name)
        .find(|&name| *name == topic)
        .map(|v| v as _)
}

fn create_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.bold.set_fg(termimad::crossterm::style::Color::Cyan);
    skin.italic
        .set_fg(termimad::crossterm::style::Color::Magenta);
    skin.inline_code
        .set_fg(termimad::crossterm::style::Color::Green);
    skin.code_block
        .set_fg(termimad::crossterm::style::Color::Green);
    skin
}

pub fn list() -> Result<(), String> {
    let skin = create_skin();
    let mut content = String::from(
        "# NEVE DOCUMENTATION\n\n## Usage\n\n```\nneve doc <topic>\nneve doc --list\n```\n\n## Topics\n\n```\n",
    );
    for (name, _, desc) in TOPICS {
        content.push_str(&format!("  {:14} - {}\n", name, desc));
    }
    content.push_str("```\n");
    println!("{}", skin.term_text(&content));
    Ok(())
}

pub fn view(topic: &str) -> Result<(), String> {
    if matches!(topic, "list" | "help" | "topics") {
        return list();
    }
    let resolved = resolve_topic(topic);
    let (_, content, _) = TOPICS
        .iter()
        .find(|(n, _, _)| Some(*n) == resolved)
        .ok_or_else(|| {
            let mut msg = format!("Unknown topic: {}\n\nAvailable:\n", topic);
            for (name, _, desc) in TOPICS {
                msg.push_str(&format!("  {:12} - {}\n", name, desc));
            }
            msg
        })?;

    let cleaned = clean_markdown(content);
    let skin = create_skin();
    let rendered = skin.term_text(&cleaned);
    if try_pager(&rendered.to_string()).is_err() {
        println!("{}", rendered);
    }
    Ok(())
}

fn clean_markdown(content: &str) -> String {
    let mut out = Vec::new();
    let mut in_code_block = false;
    for raw in content.lines() {
        let line = raw.trim_end();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
        }
        if !in_code_block {
            let cleaned = line
                .replace("<div align=\"center\">", "")
                .replace("</div>", "")
                .replace("<br>", "")
                .replace("<strong>", "**")
                .replace("</strong>", "**")
                .replace("<em>", "*")
                .replace("</em>", "*");
            if cleaned.starts_with("<img")
                || cleaned.starts_with("<a name=")
                || cleaned.starts_with("<a href=")
                || cleaned.starts_with("<p>")
            {
                continue;
            }
            out.push(cleaned.to_string());
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

fn try_pager(content: &str) -> Result<(), String> {
    for pager in &["less", "more"] {
        if let Ok(mut child) = std::process::Command::new(pager)
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
    Err("no pager found".to_string())
}
