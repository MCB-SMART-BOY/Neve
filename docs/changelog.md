# Changelog

基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式。

## [0.3.0] - 2025-12-29

### 跨平台支持
- **Windows/macOS**: 完整语言支持 (eval, repl, check, fmt, lsp)
- **Docker 构建后端**: `--backend docker` 选项，在非 Linux 平台实现沙箱构建
- **平台感知 CLI**: `neve info --platform` 显示平台能力

### 平台特性矩阵

| 特性 | Linux | macOS | Windows |
|------|-------|-------|---------|
| 语言核心 | ✅ | ✅ | ✅ |
| 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker 构建 | ✅ | ✅ | ✅ |
| 系统配置 | ✅ | ❌ | ❌ |

## [0.2.0] - 2025-12-28

### 主要特性
- **REPL 环境持久化**: 变量和函数在会话中保持
- **模块重导出修复**: 修复 `pub import` 无限循环 bug

### REPL 增强
- `:env` - 显示当前绑定
- `:load <file>` - 加载外部文件
- `:clear` - 清空环境
- 多行输入支持 (行尾 `\`)

### Bug 修复
- 模块重导出无限循环
- Import 冲突解析

## [0.1.0] - 2024

### 初始版本

#### 语言核心 (95%)
- 完整词法分析器 (logos)
- 递归下降解析器 (LL(1)) + 错误恢复
- Hindley-Milner 类型推导 + Trait 支持
- 树遍历解释器 + 惰性求值
- 模块系统

#### 标准库
- 9 个模块: io, list, map, math, option, path, result, set, string

#### 工具链 (80%)
- LSP 服务器
- 代码格式化器
- 交互式 REPL
- 诊断系统

#### 包管理 (60%)
- Derivation 模型 + 哈希验证
- 内容寻址存储 (BLAKE3)
- 沙箱构建器 (Linux 命名空间)
- 源码获取 (URL, Git, 本地)

#### 系统配置 (40%)
- 配置框架
- 代际管理

[0.3.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MCB-SMART-BOY/neve/releases/tag/v0.1.0
