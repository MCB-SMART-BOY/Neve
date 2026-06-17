# Neve Ecosystem Design

## 1. Architecture

Neve's ecosystem is built on a Nix-inspired content-addressed model:

```
flake.neve ──→ Flake (inputs + outputs)
    │
    ▼
flake.lock ──→ FlakeLock (pinned hashes)
    │
    ▼
neve-store ──→ Content-addressed /neve/store
    │
    ├── neve-fetch (URL, Git, local)
    ├── neve-builder (sandboxed builds)
    └── neve-config (system configuration)
```

### 1.1 Design Principles

**Content addressing**: Every artifact in the store is identified by a cryptographic hash of its contents. This guarantees:
- **Reproducibility**: Same inputs always produce the same output path.
- **Deduplication**: Identical artifacts are stored only once.
- **Integrity**: Tampered artifacts are detectable via hash mismatch.

**Sandboxed builds**: Every build runs in an isolated environment with:
- No network access unless explicitly declared.
- Restricted filesystem access (only declared inputs).
- Deterministic build outputs.

**Generational profiles**: User environments use atomic generation-based switching:
- Each `neve package install` creates a new generation.
- `neve package rollback` switches to the previous generation atomically.
- Generations are garbage-collection roots, protecting active packages.

### 1.2 Crate Architecture

| Crate | Purpose | Status |
|-------|---------|--------|
| `neve-fetch` | URL, Git, and local file fetching with hash verification | Implemented |
| `neve-store` | Content-addressed store with NAR archives, signatures, GC | Implemented |
| `neve-builder` | Sandboxed builds (native, Docker, simple backends) | Implemented |
| `neve-config` | System configuration with generation-based rollback | Implemented |

## 2. Flake System (already implemented)

### 2.1 `flake.neve`: Project Manifest

The flake manifest declares inputs (dependencies) and outputs (packages, modules, configurations):

```neve
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    neve-std.url = "github:neve-lang/std/v0.3";
  };

  outputs = { self, nixpkgs, neve-std }: {
    packages.hello = neve-std.buildNevePackage {
      name = "hello";
      src = ./src;
    };
  };
}
```

### 2.2 `flake.lock`: Dependency Lockfile

JSON lockfile pinning all dependency hashes for reproducible builds:

```json
{
  "nodes": {
    "nixpkgs": {
      "locked": {
        "narHash": "sha256-abc123...",
        "type": "github",
        "owner": "NixOS",
        "repo": "nixpkgs",
        "rev": "abc123..."
      }
    }
  }
}
```

### 2.3 Input Types

| Type | Syntax | Example |
|------|--------|---------|
| GitHub | `owner/repo` | `github:neve-lang/std` |
| Git | `git+https://...` | `git+https://git.example.com/repo` |
| URL | `https://...` | `https://example.com/pkg.tar.gz` |
| Local path | `./path` | `./lib/mylib` |

## 3. Store (already implemented)

### 3.1 Content-Addressed Storage

The store resides at `/neve/store` (configurable via `NEVE_STORE` environment variable). Each artifact is stored at a path derived from its content hash:

```
/neve/store/
  ├── abc123...-hello-1.0/
  │   ├── bin/
  │   │   └── hello
  │   └── share/
  ├── def456...-coreutils-9.0/
  └── ...
```

### 3.2 NAR Archives

Packages are serialized as NAR (Neve ARchive) archives, a deterministic archive format that preserves:
- File permissions and types (regular, directory, symlink)
- Modification times (normalized for reproducibility)
- File contents

NAR archives support Ed25519 signature verification for integrity.

### 3.3 Garbage Collection

The store supports generation-based garbage collection:
- **Generation roots**: Active profiles and their generations protect packages.
- **GC sweep**: `neve store gc` removes unreferenced store paths.
- **Dry-run mode**: Preview what would be removed before executing.

### 3.4 Binary Cache / Substituter

Store artifacts can be served via binary caches:
- **Cache URLs**: Remote HTTP(S) endpoints serving NAR archives.
- **narinfo files**: Metadata files with hashes and signatures.
- **Substitution**: `neve build` can download pre-built artifacts instead of building locally.
- **Upload**: Successful builds can be uploaded to writable caches.

## 4. Package Management (already implemented)

### 4.1 CLI Commands

```bash
# Install a package to the user profile
neve package install hello

# Remove a package from the user profile
neve package remove hello

# List installed packages
neve package list

# Search the store and package index
neve search <query>

# Rollback to previous generation
neve package rollback

# Build a package
neve build <package> --backend native

# Update dependencies
neve update
```

### 4.2 Profile Generations

Each installation or removal creates a new profile generation:

```
~/.neve/profile/
  ├── generation-1/
  │   ├── manifest
  │   └── bin/
  ├── generation-2/
  │   ├── manifest
  │   └── bin/
  └── current -> generation-2  (symlink)
```

Rollback atomically switches the `current` symlink to the previous generation.

### 4.3 Package Resolution

When installing a package, the installer:
1. Searches for exact matches in the store.
2. Falls back to prefix matching (e.g., `hello` matches `hello-1.0`).
3. Reports ambiguous matches when multiple versions exist.

### 4.4 System Configuration

The `neve config` subsystem manages system-wide configuration with the same generation-based model:
- `neve config build`: Build system configuration from flake.
- `neve config switch`: Atomically switch to new configuration.
- `neve config rollback`: Revert to previous configuration.
- `neve config list`: List configuration generations.
- `neve config verify`: Verify generation activation snapshot integrity.

### 4.5 Package Index

Neve supports a simple JSON package index for discovery:

```json
[
  {"name": "nevepkgs.git", "description": "Git version control"},
  {"name": "nevepkgs.curl", "description": "URL transfer tool"},
  {"name": "nevepkgs.neve", "description": "The Neve language"}
]
```

Location: `$HOME/.neve/package-index.json` or `$NEVE_PACKAGE_INDEX`

The index is searched by `neve search <query>` which matches against both package name and description.

## 5. Build System

### 5.1 Build Backends

| Backend | Description | Use Case |
|---------|-------------|----------|
| `native` | Build directly on host | Development, trusted packages |
| `docker` | Build in Docker container | CI, untrusted packages |
| `simple` | Minimal sandbox with seccomp | Lightweight isolation |
| `auto` | Auto-select best backend | Default |

### 5.2 Sandbox Features

- **Filesystem isolation**: Only declared inputs are visible.
- **Network isolation**: No network access unless declared in flake.
- **seccomp filtering**: System call filtering on Linux.
- **User namespace**: Unprivileged user mapping.

## 6. Future Directions

### 6.1 Package Index / Registry (Implemented ✅)

Neve supports a central package registry for discovering and publishing packages. The registry CLI is now available:

```bash
neve registry-update   # Update local registry index
neve registry-serve    # Start local registry server
neve registry-publish  # Publish package to registry
```

The package index is a simple JSON file at `$HOME/.neve/package-index.json` or `$NEVE_PACKAGE_INDEX`:

```json
[
  {"name": "nevepkgs.git", "description": "Git version control"},
  {"name": "nevepkgs.curl", "description": "URL transfer tool"},
  {"name": "nevepkgs.neve", "description": "The Neve language"}
]
```

The index is searched by `neve search <query>` which matches against both package name and description.

### 6.2 Module System Integration

Tighter integration between the flake system and Neve's module system, allowing `use` to resolve flake inputs.

### 6.3 Remote Registry & Publishing

Future expansion of the registry system to support remote publishing workflows, decentralized package discovery, and multi-registry federation.

### 6.4 Cross-Platform Support

Currently package management is Unix-only. Future phases may extend to Windows via a different store model.

## 7. References

- [Stability Tiers](../reference/stability.md) — Stdlib stability guarantees
- [Forward Plan](../../.claude/forward-plan.md) — Overall project phases
- [Feature Matrix](feature-matrix.md) — Capability assessment
- [API Reference](../reference/api.md) — Stdlib API documentation
- [Specification](../reference/spec.md) — Language specification
