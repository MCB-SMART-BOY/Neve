//! Documentation viewer command.
//! 文档查看器命令。
//!
//! Provides man-like access to embedded documentation with terminal rendering.
//! 提供类似 man 的嵌入式文档访问，带有终端渲染。

use std::io::Write;
use termimad::MadSkin;

// Embed documentation at compile time
// 在编译时嵌入文档
const DOC_QUICKSTART: &str = include_str!("../../../docs/quickstart.md");
const DOC_TUTORIAL: &str = include_str!("../../../docs/tutorial.md");
const DOC_SPEC: &str = include_str!("../../../docs/spec.md");
const DOC_API: &str = include_str!("../../../docs/api.md");
const DOC_PHILOSOPHY: &str = include_str!("../../../docs/philosophy.md");
const DOC_INSTALL: &str = include_str!("../../../docs/install.md");
const DOC_CHANGELOG: &str = include_str!("../../../docs/changelog.md");

/// Available documentation topics.
/// 可用的文档主题。
const TOPICS: &[(&str, &str, &str)] = &[
    ("quickstart", DOC_QUICKSTART, "5-minute quick start guide"),
    ("tutorial", DOC_TUTORIAL, "Complete language tutorial"),
    ("spec", DOC_SPEC, "Language specification"),
    ("api", DOC_API, "Standard library API reference"),
    ("philosophy", DOC_PHILOSOPHY, "Design philosophy"),
    ("install", DOC_INSTALL, "Installation guide"),
    ("changelog", DOC_CHANGELOG, "Version history"),
];

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

    let content = r#"
# NEVE DOCUMENTATION

## Available topics:

| Topic | Description |
|-------|-------------|
| quickstart | 5-minute quick start guide |
| tutorial | Complete language tutorial |
| spec | Language specification |
| api | Standard library API reference |
| philosophy | Design philosophy |
| install | Installation guide |
| changelog | Version history |

## Usage:

```
neve doc <topic>          View a topic
neve doc <topic> --en     View English section only
neve doc <topic> --zh     View Chinese section only
neve doc --list           List all topics
```

## Examples:

```
neve doc quickstart       Full quickstart guide
neve doc api --en         API reference (English)
neve doc spec --zh        Language spec (Chinese)
```
"#;

    println!("{}", skin.term_text(content));
    Ok(())
}

/// View a documentation topic.
/// 查看文档主题。
pub fn view(topic: &str, lang: Option<&str>) -> Result<(), String> {
    // Find the topic
    // 查找主题
    let content = TOPICS
        .iter()
        .find(|(name, _, _)| *name == topic)
        .map(|(_, content, _)| *content);

    let content = match content {
        Some(c) => c,
        None => {
            eprintln!("Unknown topic: {}", topic);
            eprintln!();
            eprintln!("Available topics:");
            for (name, _, desc) in TOPICS {
                eprintln!("  {:12} - {}", name, desc);
            }
            return Ok(());
        }
    };

    // Filter by language if requested
    // 如果请求，按语言过滤
    let output = match lang {
        Some("en") => extract_section(content, "english"),
        Some("zh") => extract_section(content, "chinese"),
        _ => content.to_string(),
    };

    // Clean up HTML anchors and render
    // 清理 HTML 锚点并渲染
    let cleaned = clean_markdown(&output);

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
    content
        .lines()
        .filter(|line| {
            // Remove HTML anchor tags
            // 移除 HTML 锚点标签
            !line.contains("<a name=")
                && !line.contains("</a>")
                && !line.contains("<div")
                && !line.contains("</div>")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract a specific language section from the document.
/// 从文档中提取特定语言的部分。
fn extract_section(content: &str, section: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_section = false;

    // Find the anchor for this section
    // 查找此部分的锚点
    let anchor = format!("<a name=\"{}\"></a>", section);
    let other_anchor = if section == "english" {
        "<a name=\"chinese\"></a>"
    } else {
        "<a name=\"english\"></a>"
    };

    // First, include the header (everything before the first anchor)
    // 首先，包含标题（第一个锚点之前的所有内容）
    for line in &lines {
        if line.contains("<a name=") {
            break;
        }
        result.push(*line);
    }

    // Then extract the requested section
    // 然后提取请求的部分
    for line in &lines {
        if line.contains(&anchor) {
            in_section = true;
            continue;
        }

        if in_section && line.contains(other_anchor) {
            // Stop at the other section
            // 在另一个部分停止
            break;
        }

        if in_section {
            result.push(*line);
        }
    }

    result.join("\n")
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
