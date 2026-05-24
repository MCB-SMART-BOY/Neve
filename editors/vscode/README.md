# Neve Language Support for VS Code

Syntax highlighting, auto-completion, diagnostics, formatting, and code navigation for the [Neve](https://github.com/neve-lang/neve) programming language.

## Features

- **Syntax highlighting** — Full TextMate grammar for `.neve` files
- **Diagnostics** — Real-time parse and type errors as you type
- **Auto-completion** — Keywords, stdlib functions, types, and type-aware method completion (54 methods across 5 receiver types)
- **Hover** — Type information and documentation on hover
- **Go to Definition** — Jump to symbol definitions
- **Find References** — Find all references to a symbol
- **Rename** — Rename symbols across files
- **Signature Help** — Function parameter hints (80+ builtin signatures)
- **Code Formatting** — Format documents with `neve fmt`
- **Code Lens** — Reference counts above function/struct/trait definitions
- **Document Symbols** — Breadcrumb and outline support
- **Workspace Symbols** — Search symbols across the workspace
- **Semantic Tokens** — AST-based syntax highlighting (10 token types, 3 modifiers)
- **Inlay Hints** — Inline type annotations
- **Folding Ranges** — Code folding for blocks, structs, enums, traits, impls, match
- **Code Actions** — Quick-fix suggestions for parse and type errors

## Requirements

- [Neve CLI](https://github.com/neve-lang/neve) installed and available on `$PATH`
- Run `neve setup vscode` after installation for optimal configuration

## Quick Start

1. Install Neve: follow the [installation guide](https://github.com/neve-lang/neve#installation)
2. Install this extension from the VS Code marketplace
3. Open any `.neve` file — syntax highlighting and diagnostics activate automatically

## Configuration

This extension contributes the following settings (configurable in VS Code settings):

| Setting | Default | Description |
|---------|---------|-------------|
| `editor.tabSize` | 4 | Tab size for Neve files |
| `editor.insertSpaces` | true | Use spaces instead of tabs |
| `editor.codeLens` | true | Show reference counts |

## License

MPL-2.0 — same as the Neve language project.
