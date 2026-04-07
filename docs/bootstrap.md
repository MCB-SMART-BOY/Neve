<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Bootstrap Package Examples</h1>

<p><em>Bootstrap 示例包</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./README.md">Docs</a></strong>
</p>

</div>

---

This document describes the bootstrap package examples kept under `examples/bootstrap/`.
本文档说明放在 `examples/bootstrap/` 下的 bootstrap 示例包。

## 什么是 Bootstrap 基础包? / What Are Bootstrap Packages?

这些示例代表从零启动工具链时最早期的一批基础构件，用来表达 Neve 将来如何描述自举过程。

These examples represent the earliest building blocks of a future bootstrap chain and show how Neve may describe that process.

## Bootstrap 顺序 / Bootstrap Order

```
1. musl libc        → C 标准库 / C standard library
2. binutils         → 二进制工具 (ld, as, ar) / Binary utilities
3. gcc              → C/C++ 编译器 / C/C++ compiler
4. make             → 构建工具 / Build tool
5. bash             → Shell 解释器 / Shell interpreter
6. coreutils        → 核心工具 (ls, cp, etc.) / Core utilities
```

## 包定义结构 / Package Definition Structure

每个 `.neve` 文件定义一个包,使用 Neve 的 derivation 语法:

```neve
{
    name = "package-name",
    version = "1.0.0",

    meta = #{
        description = "Package description",
        homepage = "https://...",
        license = "MIT",
        platforms = ["x86_64-linux"],
    },

    src = fetchurl {
        url = "https://...",
        hash = "sha256-...",
    },

    buildInputs = [ /* dependencies */ ],

    buildPhase = ''
        make -j$NIX_BUILD_CORES
    '',

    installPhase = ''
        make install PREFIX=$out
    '',
}
```

## 当前包列表 / Current Packages

### ✅ 已定义 / Defined

- **musl** (1.2.4) - Lightweight C standard library
- **binutils** (2.41) - GNU binary utilities (ld, as, ar, objdump, etc.)
- **gcc** (13.2.0) - GNU Compiler Collection (C, C++)

### 📋 计划中 / Planned

- **make** - GNU Make build tool
- **bash** - Bourne Again Shell
- **coreutils** - GNU core utilities
- **findutils** - GNU find, xargs, locate
- **diffutils** - GNU diff, cmp, diff3
- **patch** - GNU patch utility
- **sed** - Stream editor
- **grep** - Pattern matching
- **gawk** - GNU awk
- **gzip** - Compression utility
- **bzip2** - Compression utility
- **xz** - Compression utility
- **tar** - Archive tool

## 设计原则 / Design Principles

### 1. 最小化依赖 / Minimal Dependencies

Bootstrap 包应该尽可能少地依赖其他包，理想情况下只依赖更早阶段的基础构件。

### 2. 可复现构建 / Reproducible Builds

所有包必须:
- 使用固定版本
- 包含 SHA-256 校验和
- 避免网络访问(构建时)
- 使用确定性构建标志

### 3. 文档化 / Documentation

每个包应包含:
- 清晰的描述
- 构建步骤说明
- 依赖关系
- 许可证信息

### 4. 优化空间 / Space Optimization

- 移除不必要的文档和本地化文件
- Strip 二进制文件
- 分离开发文件到 `dev` 输出

## 使用方法 / Usage

### 构建单个包 / Build a Single Package

```bash
neve build examples/bootstrap/musl.neve
```

### 构建整个工具链 / Build Entire Toolchain

```bash
neve build examples/bootstrap/gcc.neve  # 会自动构建依赖
```

### 查看包信息 / Show Package Info

```bash
neve show examples/bootstrap/musl.neve
```

## 哈希值获取 / Getting Hashes

由于包定义中使用的哈希值是占位符,实际使用时需要获取真实哈希:

```bash
# 方法 1: 使用 nix-prefetch-url (如果可用)
nix-prefetch-url https://musl.libc.org/releases/musl-1.2.4.tar.gz

# 方法 2: 手动下载并计算
wget https://musl.libc.org/releases/musl-1.2.4.tar.gz
sha256sum musl-1.2.4.tar.gz
```

## 与 Nix 的区别 / Differences from Nix

虽然 Neve 参考了 Nix 的设计,但有关键区别:

1. **语法**: Neve 使用现代化的零歧义语法
2. **类型系统**: 强类型,Hindley-Milner 推导
3. **兼容性**: 不兼容 nixpkgs,从零构建生态

## 贡献指南 / Contributing

添加新的 bootstrap 示例包:

1. 在 `examples/bootstrap/` 创建 `.neve` 文件
2. 遵循现有包的结构
3. 确保包含所有必要的元数据
4. 测试构建过程
5. 提交 Pull Request

## 参考资料 / References

- [Linux From Scratch](http://www.linuxfromscratch.org/)
- [Nix Pills](https://nixos.org/guides/nix-pills/)
- [GNU Build System](https://www.gnu.org/software/automake/manual/html_node/Autotools-Introduction.html)

---

*Describe bootstrap packages in Neve.*
