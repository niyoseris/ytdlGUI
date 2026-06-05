#!/bin/bash
set -e
# ===================================================================
# ytGUI Linux Build Script
# Builds .deb and .AppImage on Linux (native or via Docker)
# ===================================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🏗️  ytGUI Linux Builder"
echo "======================"
echo ""

# -- Docker mode ----------------------------------------------------
if [ "$1" = "--docker" ] || ! command -v apt-get &>/dev/null; then
    echo "🐳 Docker modunda build yapılıyor..."
    echo ""

    docker build -t ytgui-linux -f "$SCRIPT_DIR/Dockerfile" "$PROJECT_DIR"
    CONTAINER_ID=$(docker create ytgui-linux)

    OUTPUT="$PROJECT_DIR/linux/output"
    rm -rf "$OUTPUT"
    mkdir -p "$OUTPUT"

    docker cp "$CONTAINER_ID:/output/." "$OUTPUT"
    docker rm "$CONTAINER_ID"

    echo ""
    echo "✅ Linux build tamam! Çıktılar:"
    ls -lh "$OUTPUT/"
    echo ""
    echo "AppImage: ytGUI_x.x.x_amd64.AppImage → çift tıkla çalışır"
    echo "Debian:   ytgui_x.x.x_amd64.deb      → sudo dpkg -i ile kur"
    exit 0
fi

# -- Native Linux mode -----------------------------------------------
echo "🐧 Native Linux modunda build yapılıyor..."
echo ""

# Check deps
command -v rustc &>/dev/null || { echo "Rust kurulu değil. curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
command -v node &>/dev/null || { echo "Node.js kurulu değil."; exit 1; }

# System deps
echo "📦 Sistem paketleri kuruluyor..."
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev \
    libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
    librsvg2-dev patchelf libfuse2 wget unzip

cd "$PROJECT_DIR"

# Install npm deps
echo "📦 npm paketleri kuruluyor..."
npm install

# Download yt-dlp
echo "📥 yt-dlp indiriliyor..."
mkdir -p src-tauri/binaries
if [ ! -f src-tauri/binaries/yt-dlp-x86_64-unknown-linux-gnu ]; then
    ARCH=$(uname -m)
    if [ "$ARCH" = "aarch64" ]; then
        URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux_aarch64"
        TARGET="yt-dlp-aarch64-unknown-linux-gnu"
    else
        URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
        TARGET="yt-dlp-x86_64-unknown-linux-gnu"
    fi
    wget -O "src-tauri/binaries/$TARGET" "$URL"
    chmod +x "src-tauri/binaries/$TARGET"
fi

# Download ffmpeg
echo "📥 ffmpeg indiriliyor..."
if [ ! -f src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu ]; then
    TMPDIR=$(mktemp -d)
    cd "$TMPDIR"
    wget -q https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz
    tar -xf ffmpeg-release-amd64-static.tar.xz
    cp ffmpeg-*-static/ffmpeg "$PROJECT_DIR/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu"
    chmod +x "$PROJECT_DIR/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu"
    rm -rf "$TMPDIR"
    cd "$PROJECT_DIR"
fi

# Build
echo "📦 Tauri build (AppImage)..."
npx vite build
cargo tauri build

echo ""
echo "✅ Linux build tamam!"
echo ""
echo "Çıktılar:"
echo "  AppImage: src-tauri/target/release/bundle/appimage/ytgui_*_amd64.AppImage"
echo "  Debian:   src-tauri/target/release/bundle/deb/ytgui_*_amd64.deb"
echo ""
echo "AppImage → çift tıkla çalışır (hiçbir şey kurman gerekmez)"
echo "Debian   → sudo dpkg -i ytgui_*.deb ile kurulur"
