# Neve 文档中心 / Documentation Hub

欢迎来到 Neve 语言的文档中心!

## 快速开始 / Quick Start

- **[5分钟快速入门](quickstart.md)** - 快速了解 Neve 并运行第一个程序
- **[完整教程](TUTORIAL.md)** - 深入学习 Neve 的各个方面
- **[API 参考](API.md)** - 标准库函数和类型参考

## 核心文档 / Core Documentation

- **[语言规范](../neve-spec-v2.md)** - 完整的语法和语义定义
- **[设计哲学](../PHILOSOPHY.md)** - Neve 的设计原则和理念
- **[项目分析](../ANALYSIS.md)** - 项目定位和技术选型
- **[开发路线图](../ROADMAP.md)** - 功能规划和优先级

## 特性亮点 / Feature Highlights

### 🎯 零歧义语法

Neve 的语法经过精心设计,消除了所有歧义:

- `#{}` 永远是记录
- `{}` 永远是代码块
- `->` 只用于类型和 match 分支
- `=` 只用于值绑定

### 🔐 纯函数式

- 所有值都是不可变的
- 函数没有副作用(除了 I/O)
- 延迟求值优化
- 尾调用优化防止栈溢出

### 🧩 强大的类型系统

- Hindley-Milner 类型推导
- 泛型和高阶类型
- Trait 系统和关联类型
- 代数数据类型 (ADT)

### 📦 现代包管理

- 内容寻址存储 (CAS)
- 可复现构建
- 沙盒化构建环境
- 二进制缓存支持

## 学习路径 / Learning Path

### 初学者

1. 阅读 [5分钟快速入门](quickstart.md)
2. 运行示例代码
3. 浏览 [完整教程](TUTORIAL.md) 的基础部分

### 进阶开发者

1. 深入学习 [完整教程](TUTORIAL.md)
2. 阅读 [语言规范](../neve-spec-v2.md)
3. 参考 [API 文档](API.md)
4. 查看 [设计哲学](../PHILOSOPHY.md)

### 贡献者

1. 阅读 [开发路线图](../ROADMAP.md)
2. 理解 [项目分析](../ANALYSIS.md)
3. 查看 [GitHub Issues](https://github.com/MCB-SMART-BOY/Neve/issues)

## 代码示例 / Code Examples

### Hello World

```neve
"Hello, Neve!"
```

### 函数定义

```neve
fn factorial(n: Int) -> Int = {
    if n <= 1 then 1
    else n * factorial(n - 1)
};
```

### 数据处理

```neve
let numbers = [1, 2, 3, 4, 5];

let result = numbers
    |> filter(fn(x) x > 2)
    |> map(fn(x) x * 2)
    |> fold(0, fn(acc, x) acc + x);

-- result = 24
```

### 模式匹配

```neve
fn describe(opt) = match opt {
    Some(x) -> `Got: {x}`,
    None -> "Nothing",
};
```

## 工具 / Tools

### 命令行工具

```bash
neve eval "expression"    # 求值表达式
neve repl                  # 启动 REPL
neve run file.neve         # 运行文件
neve check file.neve       # 类型检查
neve fmt file.neve         # 格式化代码
neve build                 # 构建包
```

### LSP 支持

Neve 提供完整的 Language Server Protocol 支持:

- ✅ 语法高亮
- ✅ 代码补全
- ✅ 跳转到定义
- ✅ 实时诊断
- ✅ 代码格式化
- ✅ 重命名
- ✅ 查找引用

### 编辑器集成

- VS Code: 通过 LSP 扩展
- Neovim: 通过 nvim-lspconfig
- Emacs: 通过 lsp-mode

## 社区 / Community

- **GitHub**: [MCB-SMART-BOY/Neve](https://github.com/MCB-SMART-BOY/Neve)
- **Issues**: [Bug 报告和功能请求](https://github.com/MCB-SMART-BOY/Neve/issues)
- **讨论**: [GitHub Discussions](https://github.com/MCB-SMART-BOY/Neve/discussions)

## 常见问题 / FAQ

### Neve 和 Nix 有什么关系?

Neve 继承了 Nix 的核心理念(纯函数式、可复现、声明式),但用现代设计从零实现,**不兼容 nixpkgs**。我们的目标是"继承并且超越"。

### 为什么选择 Neve?

- **零歧义**: 语法清晰,没有意外
- **类型安全**: 强大的类型系统避免运行时错误
- **可复现**: 完全确定性的构建
- **现代化**: 使用现代编译器技术和最佳实践

### Neve 适合什么场景?

- 包管理和构建系统
- 配置管理
- 函数式编程学习
- 需要可复现性的任何场景

### 如何贡献?

1. Fork 项目
2. 创建功能分支
3. 提交 Pull Request
4. 参与代码审查

## 路线图 / Roadmap

查看 [ROADMAP.md](../ROADMAP.md) 了解:

- ✅ 已完成功能
- 🚧 正在开发
- 📋 计划中

## 许可证 / License

Neve 采用 MIT 许可证。详见项目根目录的 LICENSE 文件。

---

*快乐编程! Happy Hacking with Neve!* 🚀
