// Downloads yt-dlp and ffmpeg static binaries for the current platform.
// Run: node scripts/download-binaries.mjs
// Places binaries in src-tauri/binaries/ with target-triple suffixes for Tauri sidecar.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BIN_DIR = path.resolve(__dirname, "..", "src-tauri", "binaries");

// Detect platform
const platform = process.platform; // 'darwin' | 'win32' | 'linux'
const arch = process.arch; // 'x64' | 'arm64'

const TARGET_TRIPLES = {
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
};

const target = TARGET_TRIPLES[`${platform}-${arch}`];
if (!target) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

const isWin = platform === "win32";
const ytdlpSuffix = isWin ? ".exe" : "";
const ffmpegSuffix = isWin ? ".exe" : "";

// URLs
const YTDLP_URLS = {
  "darwin-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
  "darwin-arm64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
  "win32-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
  "linux-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
  "linux-arm64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux_aarch64",
};

// ffmpeg static builds
// macOS: from evermeet.cx (single binary)
// Linux: from johnvansickle.com
// Windows: from gyan.dev
const FFMPEG_URLS = {
  "darwin-x64": "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip",
  "darwin-arm64": "https://www.osxexperts.net/ffmpeg6arm.zip",
  "win32-x64": "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
  "linux-x64": "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz",
};

async function download(url, dest) {
  console.log(`  Downloading: ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  const buf = Buffer.from(await res.arrayBuffer());
  fs.writeFileSync(dest, buf);
  console.log(`  Saved: ${dest} (${(buf.length / 1024 / 1024).toFixed(1)} MB)`);
  return dest;
}

async function main() {
  fs.mkdirSync(BIN_DIR, { recursive: true });

  console.log("📦 Downloading binaries...");
  console.log(`   Platform: ${platform}-${arch} (${target})`);
  console.log(`   Output:   ${BIN_DIR}\n`);

  // --- yt-dlp ---
  const ytdlpUrl = YTDLP_URLS[`${platform}-${arch}`];
  if (!ytdlpUrl) {
    console.log("⚠️  No yt-dlp binary for this platform — skipping");
  } else {
    const ytDest = path.join(BIN_DIR, `yt-dlp-${target}${ytdlpSuffix}`);
    if (fs.existsSync(ytDest)) {
      console.log(`  ✓ yt-dlp already exists: ${ytdlpSuffix}`);
    } else {
      await download(ytdlpUrl, ytDest);
      // Make executable on Unix
      if (!isWin) {
        fs.chmodSync(ytDest, 0o755);
      }
    }
  }

  // --- ffmpeg ---
  const ffmpegUrl = FFMPEG_URLS[`${platform}-${arch}`];
  if (!ffmpegUrl) {
    console.log("⚠️  No ffmpeg binary URL for this platform — skipping");
  } else {
    const ffDest = path.join(BIN_DIR, `ffmpeg-${target}${ffmpegSuffix}`);
    if (fs.existsSync(ffDest)) {
      console.log("  ✓ ffmpeg already exists");
    } else {
      console.log("  Downloading ffmpeg...");
      // ffmpeg comes in archives — download and extract
      const tmpDir = path.join(BIN_DIR, "_tmp");
      fs.mkdirSync(tmpDir, { recursive: true });

      const archiveName = ffmpegUrl.endsWith(".zip")
        ? "ffmpeg.zip"
        : "ffmpeg.tar.xz";
      const archivePath = path.join(tmpDir, archiveName);
      await download(ffmpegUrl, archivePath);

      console.log("  Extracting ffmpeg...");
      if (archiveName.endsWith(".zip")) {
        execSync(`unzip -o "${archivePath}" -d "${tmpDir}"`, {
          stdio: "pipe",
        });
        // Find the ffmpeg binary inside the extracted directory
        const findFfmpeg = (dir) => {
          const entries = fs.readdirSync(dir, { withFileTypes: true });
          for (const e of entries) {
            const full = path.join(dir, e.name);
            if (e.isFile() && e.name.startsWith("ffmpeg")) {
              return full;
            }
            if (e.isDirectory()) {
              const found = findFfmpeg(full);
              if (found) return found;
            }
          }
          return null;
        };
        const ffmpegBin = findFfmpeg(tmpDir);
        if (ffmpegBin) {
          fs.copyFileSync(ffmpegBin, ffDest);
          if (!isWin) fs.chmodSync(ffDest, 0o755);
          console.log(`  ✓ ffmpeg extracted to: ${ffDest}`);
        }
      } else {
        execSync(`tar -xf "${archivePath}" -C "${tmpDir}"`, {
          stdio: "pipe",
        });
        const findFfmpeg = (dir) => {
          const entries = fs.readdirSync(dir, { withFileTypes: true });
          for (const e of entries) {
            const full = path.join(dir, e.name);
            if (e.isFile() && e.name === "ffmpeg") {
              return full;
            }
            if (e.isDirectory()) {
              const found = findFfmpeg(full);
              if (found) return found;
            }
          }
          return null;
        };
        const ffmpegBin = findFfmpeg(tmpDir);
        if (ffmpegBin) {
          fs.copyFileSync(ffmpegBin, ffDest);
          if (!isWin) fs.chmodSync(ffDest, 0o755);
          console.log(`  ✓ ffmpeg extracted to: ${ffDest}`);
        }
      }

      // Cleanup temp
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  }

  console.log("\n✅ Binary download complete!");
  console.log("   Run 'cargo tauri dev' to start the app.");
}

main().catch((err) => {
  console.error("❌", err.message);
  process.exit(1);
});
