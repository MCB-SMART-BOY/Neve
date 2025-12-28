# Neve Installation Guide

Complete installation instructions for Neve on all supported platforms.

---

## 📋 System Requirements

- **Operating System**:
  - Linux (x86_64, ARM64)
  - macOS (Intel, Apple Silicon)
  - Windows (x86_64)
- **Memory**: 512 MB minimum
- **Disk Space**: 100 MB for installation

---

## 🚀 Quick Install

### Option 1: Pre-built Binaries (Recommended)

#### Linux / macOS

```bash
# Download the latest release for your platform
# Linux x86_64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz

# Linux ARM64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-aarch64-unknown-linux-gnu.tar.gz

# macOS Intel
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-apple-darwin.tar.gz

# macOS Apple Silicon
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-aarch64-apple-darwin.tar.gz

# Extract and install
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Or install to user directory (no sudo needed)
mkdir -p ~/.local/bin
mv neve ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc

# Verify installation
neve --version
```

#### Windows

1. Download the latest release:
   - [neve-x86_64-pc-windows-msvc.zip](https://github.com/MCB-SMART-BOY/neve/releases/latest)

2. Extract the ZIP file

3. Add to PATH:
   ```powershell
   # Move to a permanent location
   Move-Item neve.exe C:\Program Files\Neve\

   # Add to PATH (PowerShell as Administrator)
   $env:Path += ";C:\Program Files\Neve"
   [Environment]::SetEnvironmentVariable("Path", $env:Path, [System.EnvironmentVariableTarget]::Machine)
   ```

4. Verify installation:
   ```powershell
   neve --version
   ```

---

### Option 2: Build from Source

#### Prerequisites

- **Rust**: 1.75 or later
  ```bash
  # Install rustup if not already installed
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

  # On Windows, download from: https://rustup.rs
  ```

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve

# Build release binary
cargo build --release

# The binary will be at: target/release/neve

# Install system-wide (Linux/macOS)
sudo cp target/release/neve /usr/local/bin/

# Or install to user directory
mkdir -p ~/.local/bin
cp target/release/neve ~/.local/bin/

# On Windows
copy target\release\neve.exe C:\Program Files\Neve\
```

#### Platform-Specific Notes

**Linux:**
- No additional dependencies required
- Sandbox features require Linux kernel 3.8+ (namespaces support)

**macOS:**
- Xcode Command Line Tools required: `xcode-select --install`
- Apple Silicon Macs: Use native ARM64 binary for best performance

**Windows:**
- Visual Studio Build Tools required for building from source
- Some features (sandboxing) are limited on Windows

---

### Option 3: Package Managers

#### Arch Linux (AUR)

```bash
# Using yay
yay -S neve-git

# Using paru
paru -S neve-git
```

#### Homebrew (Coming Soon)

```bash
brew install neve
```

#### Chocolatey (Windows - Coming Soon)

```powershell
choco install neve
```

#### Scoop (Windows - Coming Soon)

```powershell
scoop install neve
```

---

## 🔧 Post-Installation Setup

### Configure Your Shell

Add Neve to your PATH permanently:

**Bash** (`~/.bashrc`):
```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Zsh** (`~/.zshrc`):
```zsh
export PATH="$HOME/.local/bin:$PATH"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
set -gx PATH $HOME/.local/bin $PATH
```

**Windows PowerShell** (Profile):
```powershell
$env:Path += ";$env:USERPROFILE\.local\bin"
```

### Verify Installation

```bash
# Check version
neve --version
# Expected output: Neve 0.2.0

# Start REPL
neve repl
# You should see: Neve REPL v0.2.0

# Run a simple expression
neve eval "1 + 2"
# Expected output: 3
```

---

## 🎮 Getting Started

### Interactive REPL

```bash
neve repl
```

Try these commands:
```neve
neve> let x = 42;
neve> let y = 100;
neve> x + y
Int(142)

neve> fn double(n) = n * 2;
neve> double(21)
Int(42)

neve> :help    # Show all commands
neve> :env     # Show current bindings
neve> :quit    # Exit REPL
```

### Run a Neve File

Create `hello.neve`:
```neve
fn main() = {
    let name = "World";
    print(`Hello, {name}!`)
};
```

Run it:
```bash
neve run hello.neve
```

### Check Type Errors

```bash
neve check myfile.neve
```

### Format Code

```bash
neve fmt file myfile.neve
neve fmt dir ./src
```

---

## 🛠️ IDE Integration

### VS Code

Install the Neve extension (coming soon):
```bash
code --install-extension neve-lang.neve
```

### Vim/Neovim

Add to your config:
```vim
" Install vim-plug if not already installed
Plug 'neve-lang/neve.vim'
```

### Emacs

```elisp
(use-package neve-mode
  :ensure t)
```

---

## 🐛 Troubleshooting

### "Command not found: neve"

**Solution**: Ensure Neve is in your PATH:
```bash
echo $PATH
which neve  # Should show the location
```

### Permission Denied (Linux/macOS)

**Solution**: Make the binary executable:
```bash
chmod +x /path/to/neve
```

### Windows SmartScreen Warning

**Solution**: Click "More info" → "Run anyway"
- This is expected for new unsigned binaries
- We're working on code signing

### Build Errors on Windows

**Solution**: Install Visual Studio Build Tools:
```powershell
# Install via winget
winget install Microsoft.VisualStudio.2022.BuildTools

# Or download from:
# https://visualstudio.microsoft.com/downloads/
```

### "Linker error" on Linux

**Solution**: Install build essentials:
```bash
# Debian/Ubuntu
sudo apt install build-essential

# Fedora/RHEL
sudo dnf install gcc

# Arch Linux
sudo pacman -S base-devel
```

---

## 🆘 Getting Help

- **Documentation**: https://github.com/MCB-SMART-BOY/neve
- **Issues**: https://github.com/MCB-SMART-BOY/neve/issues
- **Discussions**: https://github.com/MCB-SMART-BOY/neve/discussions

---

## 🚀 Next Steps

- Read the [Tutorial](docs/TUTORIAL.md)
- Check out [Examples](examples/)
- Join the community discussions
- Contribute to the project!

---

**Note**: Neve is under active development. Some features may be incomplete or change in future versions.
