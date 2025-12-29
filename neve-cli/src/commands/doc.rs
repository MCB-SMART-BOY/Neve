//! Documentation viewer command.
//!
//! Provides man-like access to embedded documentation.

use std::io::Write;

// Embed documentation at compile time
const DOC_QUICKSTART: &str = include_str!("../../../docs/quickstart.md");
const DOC_TUTORIAL: &str = include_str!("../../../docs/tutorial.md");
const DOC_SPEC: &str = include_str!("../../../docs/spec.md");
const DOC_API: &str = include_str!("../../../docs/api.md");
const DOC_PHILOSOPHY: &str = include_str!("../../../docs/philosophy.md");
const DOC_INSTALL: &str = include_str!("../../../docs/install.md");
const DOC_CHANGELOG: &str = include_str!("../../../docs/changelog.md");

/// Available documentation topics
const TOPICS: &[(&str, &str, &str)] = &[
    ("quickstart", DOC_QUICKSTART, "5-minute quick start guide"),
    ("tutorial", DOC_TUTORIAL, "Complete language tutorial"),
    ("spec", DOC_SPEC, "Language specification"),
    ("api", DOC_API, "Standard library API reference"),
    ("philosophy", DOC_PHILOSOPHY, "Design philosophy"),
    ("install", DOC_INSTALL, "Installation guide"),
    ("changelog", DOC_CHANGELOG, "Version history"),
];

/// List available documentation topics
pub fn list() -> Result<(), String> {
    println!(
        r#"
╔═══════════════════════════════════════════════════════════════╗
║                    NEVE DOCUMENTATION                         ║
╚═══════════════════════════════════════════════════════════════╝

Available topics:
"#
    );

    for (name, _, desc) in TOPICS {
        println!("  {:12} - {}", name, desc);
    }

    println!(
        r#"
Usage:
  neve doc <topic>          View a topic (e.g., neve doc quickstart)
  neve doc <topic> --en     View English section only
  neve doc <topic> --zh     View Chinese section only
  neve doc --list           List all topics

Examples:
  neve doc quickstart       Full quickstart guide
  neve doc api --en         API reference (English)
  neve doc spec --zh        Language spec (Chinese)
"#
    );

    Ok(())
}

/// View a documentation topic
pub fn view(topic: &str, lang: Option<&str>) -> Result<(), String> {
    // Find the topic
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
    let output = match lang {
        Some("en") => extract_section(content, "english"),
        Some("zh") => extract_section(content, "chinese"),
        _ => content.to_string(),
    };

    // Try to use a pager for better reading experience
    if try_pager(&output).is_err() {
        // Fallback to direct output
        println!("{}", output);
    }

    Ok(())
}

/// Extract a specific language section from the document
fn extract_section(content: &str, section: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_section = false;

    // Find the anchor for this section
    let anchor = format!("<a name=\"{}\"></a>", section);
    let other_anchor = if section == "english" {
        "<a name=\"chinese\"></a>"
    } else {
        "<a name=\"english\"></a>"
    };

    // First, include the header (everything before the first anchor)
    for line in &lines {
        if line.contains("<a name=") {
            break;
        }
        result.push(*line);
    }

    // Then extract the requested section
    for line in &lines {
        if line.contains(&anchor) {
            in_section = true;
            continue;
        }

        if in_section && line.contains(other_anchor) {
            // Stop at the other section
            break;
        }

        if in_section {
            result.push(*line);
        }
    }

    result.join("\n")
}

/// Try to display content using a pager (less, more, etc.)
fn try_pager(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try to find a pager
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());

    // Try 'less' with some nice options for markdown
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
    Err("No pager available".into())
}
