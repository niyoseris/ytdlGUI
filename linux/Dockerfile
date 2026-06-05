# Build ytGUI for Linux using Docker
# Usage: docker build -t ytgui-linux -f linux/Dockerfile .

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Install system deps
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libfuse2 \
    file \
    xz-utils \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20.x
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# Copy project files
COPY package.json .
COPY vite.config.js .
COPY index.html .
COPY src/ src/
COPY src-tauri/ src-tauri/
COPY scripts/ scripts/

# Install npm deps (with platform-specific packages)
RUN npm install

# Download yt-dlp for Linux
RUN mkdir -p src-tauri/binaries && \
    wget -O src-tauri/binaries/yt-dlp-x86_64-unknown-linux-gnu \
    https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux && \
    chmod +x src-tauri/binaries/yt-dlp-x86_64-unknown-linux-gnu

# Download ffmpeg for Linux
RUN mkdir -p /tmp/ffmpeg && \
    cd /tmp/ffmpeg && \
    wget -q https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz && \
    tar -xf ffmpeg-release-amd64-static.tar.xz && \
    cp ffmpeg-*-static/ffmpeg /build/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu && \
    chmod +x /build/src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu && \
    rm -rf /tmp/ffmpeg

# Add binaries to tauri config as resources
RUN if [ -f src-tauri/tauri.conf.json ]; then \
    sed -i 's/"resources": \[\]/"resources": ["binaries\/*"]/' src-tauri/tauri.conf.json; \
    fi

# Build
RUN npx vite build && cargo tauri build

# Create output directory
RUN mkdir -p /output && \
    cp -r src-tauri/target/release/bundle/deb/*.deb /output/ 2>/dev/null || true && \
    cp -r src-tauri/target/release/bundle/appimage/*.AppImage /output/ 2>/dev/null || true && \
    echo "Linux build complete. Check /output/"

VOLUME ["/output"]

CMD ["echo", "Build complete. Run: docker cp <container>:/output ./linux-output"]
