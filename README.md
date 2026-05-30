# 🖋️ Typst Writer

**Typst Writer** is a state-of-the-art, high-performance, and feature-rich cross-platform IDE dedicated to the modern [Typst](https://typst.app) typesetting language. Built atop GPUI, it offers a breathtakingly fast, responsive, and ergonomic editing experience with real-time feedback.

---

## ✨ Features

- **🚀 Instant Compile & Live Preview**: Real-time PDF rendering virtualized at 60FPS to support monolithic documents seamlessly without any UI latency.
- **🎨 Modern Ribbon Interface**: A highly responsive, context-aware MS Word-style ribbon toolbar with quick-access tabs (Home, Insert, Math, Developer) that adjust dynamically to your cursor context.
- **🖥️ Smart Split-View Docking Workspace**: A robust, focus-aware multi-split window docking architecture that groups editing and preview panels dynamically.
- **⚙️ Integrated Preference Panel**: Adjust system themes (Light/Dark mode), live editor font sizes, toggle auto-compilation, and easily check/copy path locations of your active JSON configuration files.
- **📦 Project Asset Dropzone**: Drag and drop custom `.ttf`/`.otf` font resources directly into the assets tab to instantly render custom typography in your Typst outputs.

---

## 💻 Platforms Supported

- **🐧 Linux**: X11, Wayland, and Vulkan-accelerated GPU render layers.
- **🪟 Windows**: Fully native DirectX/MSVC execution.
- **🍎 macOS**: Native Apple Silicon (`aarch64`) & Intel (`x86_64`) support.

---

## 🚀 Precompiled Binaries

Optimized cross-platform releases are built automatically via **GitHub Actions**. To download the latest version for your platform:
1. Navigate to the **Releases** tab on the GitHub repository.
2. Select your platform-specific package (`.zip` for Windows, `.tar.gz` for Linux and macOS).
3. Extract and run `typst-writer` immediately!

---

## 🛠️ Building From Source

### 1. Prerequisites

Ensure you have [Rust](https://rustup.rs/) installed (edition 2024 or later).

#### 🐧 Linux (Ubuntu / Debian / Fedora)
Install build essentials and GPUI graphics rendering dependencies:
```bash
sudo apt-get update
sudo apt-get install -y \
  pkg-config \
  libssl-dev \
  libdbus-1-dev \
  libfontconfig1-dev \
  libfreetype6-dev \
  libx11-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libwayland-dev \
  libasound2-dev \
  libvulkan-dev \
  libegl1-mesa-dev \
  libgl1-mesa-dev
```

#### 🍎 macOS
Make sure Xcode Command Line Tools are installed:
```bash
xcode-select --install
```

#### 🪟 Windows
Ensure **C++ Build Tools for Visual Studio** are installed along with the latest Windows SDK.

---

### 2. Build Commands

To build the most optimized release binary:
```bash
# Build the highly optimized release version
cargo build --release
```

The resulting optimized executable will be located in:
- **Linux/macOS**: `target/release/typst-writer`
- **Windows**: `target/release/typst-writer.exe`

To run the application directly from source:
```bash
cargo run --release
```

---

## ⚡ High-Performance Optimization Profile

Typst Writer uses aggressive compiler settings inside the `release` profile to achieve the smallest executable size and maximum runtime speed:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

- **`opt-level = 3`**: Enables full compiler optimizations.
- **`lto = true`**: Enables Link-Time Optimization (LTO) across all crates for maximum binary speed.
- **`strip = true`**: Automatically strips symbol tables and debug information, slashing executable sizes by up to 80% for seamless distribution.


## NOTICE: I will not be able to maintain this project anymore. I am handing over the project to the community. Please fork this repository and continue the development. Thanks for your support.