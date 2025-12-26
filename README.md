# Neve

> A pure functional language for system configuration and package management.

Neve is my attempt to create a modern replacement for the Nix language. While Nix is incredibly powerful, I've always felt it could be more approachable with cleaner syntax and a proper type system.

**This project is still in early development.** Many features are incomplete or missing entirely. If you're interested in functional package management or language design, I'd love to hear your thoughts and suggestions!

## What's Working

- **Lexer & Parser** - Can parse most Neve syntax
- **Type Checker** - Basic Hindley-Milner inference
- **Evaluator** - Simple expressions work
- **LSP** - Basic editor support

## What's Not (Yet)

- **Package Building** - Derivations are defined but don't actually build anything
- **Store** - No content-addressed storage yet
- **CLI** - Very incomplete
- **Standard Library** - Just a skeleton
- **Documentation** - You're looking at most of it

## A Taste of Neve

```neve
-- Define a simple package
let hello = derivation #{
    name = "hello",
    version = "2.12",
    src = fetchurl #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz",
        sha256 = "cf04af86dc085268c5f4470fbae49b18...",
    },
    build = fn(src) #{
        configure = "./configure --prefix=$out",
        make = "make install",
    },
};

-- System configuration
let mySystem = #{
    hostname = "wonderland",
    users = [
        #{ name = "alice", shell = "/bin/zsh" },
    ],
    packages = [hello, git, vim],
};
```

## Why Another Nix?

I love Nix's ideas but struggle with its syntax:

| Pain Point | Nix | Neve |
|------------|-----|------|
| Is this a record or function? | `{ x = 1; }` | `#{ x = 1 }` (always a record) |
| Lambda syntax conflicts with types | `x: x + 1` | `fn(x) x + 1` |
| Implicit recursion | `rec { }` | Automatic detection |
| No type safety | Runtime errors | Catch errors early |

## Building from Source

```bash
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
cargo test  # ~500 tests, most pass!
```

## Arch Linux (AUR)

```bash
yay -S neve-git
```

## Project Structure

```
neve/
├── crates/
│   ├── neve-lexer      # Tokenizer
│   ├── neve-parser     # Recursive descent parser
│   ├── neve-hir        # Name resolution
│   ├── neve-typeck     # Type inference
│   ├── neve-eval       # Tree-walking interpreter
│   ├── neve-derive     # Derivation model (WIP)
│   ├── neve-store      # Content-addressed store (WIP)
│   └── ...
└── tests/              # Integration tests
```

## Contributing

This is a learning project and I'm figuring things out as I go. If you:

- Find bugs (there are many)
- Have ideas for better syntax
- Want to help implement features
- Just want to chat about language design

Please open an issue or PR! I'm especially interested in feedback on the syntax design.

## Name

*Neve* means "snow" in Italian and Portuguese - a nod to Nix (Latin for "snow"), but representing a fresh start.

## License

[MPL-2.0](LICENSE)

---

*This is a hobby project. Use at your own risk, and expect breaking changes.*

---

# Neve

> 一门用于系统配置与包管理的纯函数式语言。

Neve 是我尝试为 Nix 语言创造一个现代替代品。虽然 Nix 非常强大，但我一直觉得它可以更加友好——更清晰的语法，更完善的类型系统。

**这个项目仍处于早期开发阶段。** 很多功能还不完整，甚至完全缺失。如果你对函数式包管理或语言设计感兴趣，非常欢迎提出你的想法和建议！

## 已经能用的

- **词法分析 & 语法分析** - 能解析大部分 Neve 语法
- **类型检查** - 基础的 Hindley-Milner 推导
- **求值器** - 简单表达式可以运行
- **LSP** - 基本的编辑器支持

## 还不能用的

- **包构建** - Derivation 定义了但还不能真正构建
- **Store** - 还没有内容寻址存储
- **命令行工具** - 非常不完整
- **标准库** - 只是个骨架
- **文档** - 你现在看到的就是大部分了

## Neve 长什么样

```neve
-- 定义一个简单的包
let hello = derivation #{
    name = "hello",
    version = "2.12",
    src = fetchurl #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz",
        sha256 = "cf04af86dc085268c5f4470fbae49b18...",
    },
    build = fn(src) #{
        configure = "./configure --prefix=$out",
        make = "make install",
    },
};

-- 系统配置
let mySystem = #{
    hostname = "wonderland",
    users = [
        #{ name = "alice", shell = "/bin/zsh" },
    ],
    packages = [hello, git, vim],
};
```

## 为什么要再造一个 Nix？

我喜欢 Nix 的理念，但总是被它的语法困扰：

| 痛点 | Nix | Neve |
|------|-----|------|
| 这是记录还是函数？ | `{ x = 1; }` | `#{ x = 1 }` (永远是记录) |
| Lambda 语法和类型冲突 | `x: x + 1` | `fn(x) x + 1` |
| 隐式递归 | `rec { }` | 自动检测 |
| 没有类型安全 | 运行时报错 | 提前发现错误 |

## 从源码构建

```bash
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
cargo test  # 约 500 个测试，大部分能过！
```

## Arch Linux (AUR)

```bash
yay -S neve-git
```

## 项目结构

```
neve/
├── crates/
│   ├── neve-lexer      # 词法分析
│   ├── neve-parser     # 递归下降解析器
│   ├── neve-hir        # 名称解析
│   ├── neve-typeck     # 类型推导
│   ├── neve-eval       # 树遍历解释器
│   ├── neve-derive     # 推导模型 (WIP)
│   ├── neve-store      # 内容寻址存储 (WIP)
│   └── ...
└── tests/              # 集成测试
```

## 参与贡献

这是一个学习项目，我也在边做边摸索。如果你：

- 发现了 bug（肯定很多）
- 对语法设计有更好的想法
- 想帮忙实现某些功能
- 只是想聊聊语言设计

欢迎开 issue 或 PR！我特别希望能收到关于语法设计的反馈。

## 名字的由来

*Neve* 在意大利语和葡萄牙语中意为"雪"——呼应 Nix（拉丁语的"雪"），但代表着一个全新的开始。

## 许可证

[MPL-2.0](LICENSE)

---

*这是个业余项目。使用风险自负，随时可能有破坏性更改。*
