# Neve Package Registry

> *Package distribution and binary cache for Neve.*  
> Neve 软件包分发和二进制缓存。

---

## Overview / 概述

The Neve registry (`registry.neve.dev`) provides package discovery, distribution, and binary caching for the Neve ecosystem. It implements a v1 HTTP API with JSON metadata, NAR archives for build outputs, and optional narinfo signing.

The registry is currently in **internal validation** (Q6 gate). The server and client implementations are ready; the remaining work is operational — domain setup, public hosting, and signing key infrastructure.

## Current State / 当前状态

### Server (`neve registry-serve`)
- **Location**: `neve-cli/src/commands/registry_serve.rs`
- **Protocol**: HTTP v1 API with JSON responses
- **Endpoints**: Package index, version metadata, search, NAR download
- **Data model**: Index entries with version lists, per-version metadata with nar_hash and file_hash

### Client (`neve registry-client`)
- **Location**: `neve-cli/src/registry_client.rs`
- **Capabilities**: Package index fetching, version resolution, NAR download
- **CLI commands**: `neve registry-update`, `neve search`, `neve install`

### Binary Cache
- **Location**: `neve-cli/src/commands/build.rs`, `crates/neve-store/src/cache.rs`
- **Features**: Content-addressed NAR storage, narinfo signing (ed25519), multi-cache priority
- **CLI flags**: `--cache-url`, `--cache-dir`, `--cache-public-key`, `--cache-private-key`, `--no-substitute`, `--cache-upload`

## v1 API / v1 API

### GET `/packages.json`
Returns the full package index:
```json
[
  {
    "name": "hello",
    "versions": ["1.0.0", "1.2.0"],
    "description": "A friendly greeting program"
  }
]
```

### GET `/packages/<name>.json`
Returns version metadata:
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

### GET `/nar/<hash>.nar`
Downloads a NAR archive by content hash.

### GET `/search?q=<query>`
Case-insensitive search across package names and descriptions.

## Public Launch Plan / 公开启动计划

### Gate: Internal Validation (Current)

| Step | Status | Description |
|------|--------|-------------|
| Server implementation | ✅ | v1 API functional |
| Client implementation | ✅ | Index fetch, search, install |
| Binary cache | ✅ | NAR signing, multi-cache |
| Local testing | ✅ | `neve registry-serve` + `neve install` |
| Domain & hosting | ⬜ | `registry.neve.dev` setup |
| Signing key generation | ⬜ | Production ed25519 keys |
| Rate limiting | ⬜ | Per-IP throttling |
| Terms of service | ⬜ | Package submission policy |

### Phase: Public Beta

1. Deploy server at `registry.neve.dev`
2. Publish initial package set (core libraries and tools)
3. Open package submission with review process
4. Monitor for 4-6 weeks

### Phase: General Availability

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
export NEVE_REGISTRY="https://registry.neve.dev"

# CLI usage
neve search hello
neve install hello@1.0.0
neve registry-update  # refresh local index

# Binary cache
neve build --cache-url https://cache.neve.dev \
           --cache-public-key ed25519:AAA... \
           --cache-upload \
           --cache-private-key ed25519:BBB...
```

## Related Files / 相关文件

| File | Purpose |
|------|---------|
| `neve-cli/src/registry_client.rs` | HTTP client for v1 API |
| `neve-cli/src/commands/registry_serve.rs` | Registry server (local dev) |
| `neve-cli/src/commands/registry_publish.rs` | Package publisher |
| `neve-cli/src/commands/registry.rs` | `neve registry-update` command |
| `neve-cli/src/commands/search.rs` | `neve search` command |
| `neve-cli/src/commands/install.rs` | `neve install` command |
| `neve-cli/src/commands/build.rs` | Binary cache integration |
| `crates/neve-store/src/cache.rs` | Content-addressed cache |
| `crates/neve-store/src/nar.rs` | NAR archive format |
