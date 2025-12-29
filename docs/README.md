# Neve 文档中心 / Documentation Hub

欢迎来到 Neve 语言的文档中心！

## 文档结构

| 文档 | 描述 |
|------|------|
| [quickstart.md](quickstart.md) | 5分钟快速入门 |
| [tutorial.md](tutorial.md) | 完整教程 |
| [spec.md](spec.md) | 语言规范 |
| [api.md](api.md) | 标准库参考 |
| [philosophy.md](philosophy.md) | 设计哲学与路线图 |
| [install.md](install.md) | 安装指南 |
| [changelog.md](changelog.md) | 版本更新日志 |

## 快速链接

```bash
# 安装
cargo build --release

# 运行 REPL
neve repl

# 求值表达式
neve eval "1 + 2"

# 类型检查
neve check file.neve

# 格式化代码
neve fmt file.neve
```

## 语法速查

| 概念 | 语法 | 示例 |
|------|------|------|
| 记录 | `#{ }` | `#{ x = 1, y = 2 }` |
| 列表 | `[ ]` | `[1, 2, 3]` |
| Lambda | `fn(x) expr` | `fn(x) x + 1` |
| 函数 | `fn name(x) = expr;` | `fn add(a, b) = a + b;` |
| 管道 | `\|>` | `x \|> f \|> g` |
| 插值 | `` `{expr}` `` | `` `sum = {1 + 2}` `` |
| 注释 | `-- --` | `-- 这是注释 --` |

## 社区

- **GitHub**: [MCB-SMART-BOY/Neve](https://github.com/MCB-SMART-BOY/Neve)
- **Issues**: [Bug 报告和功能请求](https://github.com/MCB-SMART-BOY/Neve/issues)
