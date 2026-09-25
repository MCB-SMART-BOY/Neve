<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>n3v3 Package Registry</h1>

<p><em>软件包注册表</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

# n3v3 Package Registry

> *Package distribution and binary cache for n3v3.*  
> n3v3 软件包分发和二进制缓存。
This registry document describes the v5.0.0 implementation. The published CLI package is `n3v3`, and its binary is `n3v3`.
本文描述 v5.0.0 的注册表实现。已发布的 CLI 软件包名为 `n3v3`，其二进制文件名为 `n3v3`。

---

## Overview / 概述

The n3v3 registry (`registry.n3v3.dev`) provides package discovery, distribution, and binary caching for the n3v3 ecosystem. It implements a v1 HTTP API with JSON metadata, NAR archives for build outputs, and optional narinfo signing.

The registry is **Experimental** (internal validation). Server and client implementations are available; public hosting, production signing keys, rate limiting, and policy work remain **Planned**.

## Current State / 当前状态

### Server (`n3v3 registry-serve`)
- **Location**: `n3v3-cli/src/commands/registry_serve.rs`
- **Protocol**: HTTP v1 API with JSON responses
- **v1 routes**: `/v1/index.json`, `/v1/search`, `/v1/packages/<name>`, `/v1/packages/<name>/<version>`, `/v1/<hash>.narinfo`, `/v1/nar/<hash>.nar`; publishing uses `POST /v1/packages/<name>`
- **Data model**: Index entries with version lists, per-version metadata with nar_hash and file_hash

### Client (registry client library)
- **Location**: `n3v3-cli/src/registry_client.rs`
- **Capabilities**: Package index fetching, search, version resolution, NAR download
- **CLI commands**: `n3v3 registry-update`, `n3v3 search`, `n3v3 package install`

### Binary Cache
- **Location**: `n3v3-cli/src/commands/build.rs`, `crates/n3v3-store/src/cache.rs`
- **Features**: Content-addressed NAR storage, narinfo signing (ed25519), multi-cache priority
- **CLI flags**: `--cache-url`, `--cache-dir`, `--cache-public-key`, `--cache-private-key`, `--no-substitute`, `--cache-upload`


## v1 API / v1 API

The canonical server routes are under `/v1/`; legacy `/packages.json` and `/packages/<name>` routes remain compatibility endpoints.
规范服务器路由位于 `/v1/` 下；旧版 `/packages.json` 与 `/packages/<name>` 路由仍作为兼容端点保留。

### GET `/v1/index.json`
Returns the full package index:
返回完整软件包索引：
```json
[
  {
    "name": "hello",
    "versions": ["1.0.0", "1.2.0"],
    "description": "A friendly greeting program"
  }
]
```

### GET `/v1/packages/<name>`
Returns metadata for all versions of one package:
返回一个软件包的全部版本元数据：
```json
{
  "name": "hello",
  "versions": [
    {
      "version": "1.0.0",
      "nar_hash": "sha256:abc123...",
      "file_hash": "sha256:def456...",
      "dependencies": {},
      "description": "Initial release"
    }
  ]
}
```

### GET `/v1/packages/<name>/<version>`
Returns metadata for one package version.
返回一个软件包版本的元数据。

### GET `/v1/search?q=<query>`
Search is case-insensitive across package names.
搜索按软件包名称进行大小写不敏感匹配。

### GET `/v1/<hash>.narinfo`
Returns signed cache metadata when the corresponding narinfo file exists.
当对应 narinfo 文件存在时返回已签名的缓存元数据。

### GET `/v1/nar/<hash>.nar`
Downloads a NAR archive by content hash.
按内容哈希下载 NAR 归档。

### POST `/v1/packages/<name>`
Publishes version metadata for a package.
发布软件包版本元数据。

## Public Launch Plan / 公开启动计划

### Experimental: Internal Validation (Current)

| Step | Status | Description |
|------|--------|-------------|
| Server implementation | Implemented | v1 API routes in `registry_serve.rs` |
| Client implementation | Implemented | Index fetch, search, version resolution, and install integration |
| Binary cache | Implemented | NAR signing and multi-cache support |
| Local testing | Experimental | `n3v3 registry-serve` + `n3v3 package install` |
| Domain & hosting | Planned | `registry.n3v3.dev` setup |
| Signing key generation | Planned | Production ed25519 keys |
| Rate limiting | Planned | Per-IP throttling |
| Terms of service | Planned | Package submission policy |

### Planned: Public Beta

1. Deploy server at `registry.n3v3.dev`
2. Publish initial package set (core libraries and tools)
3. Open package submission with review process
4. Monitor for 4-6 weeks

### Planned: General Availability

1. Remove beta label
2. Document package authoring guide
3. Establish community package maintenance process
4. Set up mirror infrastructure

## Security Model / 安全模型

- **NAR integrity**: Build outputs are content-addressed by NAR hash
- **narinfo signing**: ed25519 signatures prevent cache poisoning
- **Substitution**: Users control which caches to trust via `--cache-public-key`
- **Upload signing**: Cache upload requires `--cache-private-key`

## Configuration / 配置

```bash
# Environment variable
export N3V3_REGISTRY="https://registry.n3v3.dev"

# CLI usage
n3v3 search hello
n3v3 package install hello
n3v3 registry-update  # refresh local index

# Binary cache
n3v3 build --cache-url https://cache.n3v3.dev \
           --cache-public-key ed25519:AAA... \
           --cache-upload \
           --cache-private-key ed25519:BBB...
```

## Related Files / 相关文件

| File | Purpose |
|------|---------|
| `n3v3-cli/src/registry_client.rs` | HTTP client for v1 API |
| `n3v3-cli/src/commands/registry_serve.rs` | Registry server (local dev) |
| `n3v3-cli/src/commands/registry_publish.rs` | Package publisher |
| `n3v3-cli/src/commands/registry.rs` | `n3v3 registry-update` command |
| `n3v3-cli/src/commands/search.rs` | `n3v3 search` command |
| `n3v3-cli/src/commands/install.rs` | `n3v3 package install` command |
| `n3v3-cli/src/commands/build.rs` | Binary cache integration |
| `crates/n3v3-store/src/cache.rs` | Content-addressed cache |
| `crates/n3v3-store/src/nar.rs` | NAR archive format |
