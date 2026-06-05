# 🎬 ytGUI — Cross-Platform YouTube Downloader

**ytGUI**, yt-dlp ve ffmpeg kullanan, Tauri tabanlı, platform bağımsız bir
masaüstü video indiricidir.

> ⚠️ **YASAL UYARI:** Bu yazılım eğitim amaçlıdır. Sadece telif hakkı size ait
> olan veya indirme izniniz bulunan içerikleri indirin. YouTube ve diğer
> platformların kullanım şartlarını ihlal etmekten doğacak hukuki sorumluluk
> tamamen kullanıcıya aittir.

---

## ✨ Özellikler

- 🎥 YouTube, TikTok, Instagram ve yt-dlp'in desteklediği tüm platformlar
- 🎵 Sadece ses (MP3) veya video indirme
- 📺 Çözünürlük seçimi (4K, 1080p, 720p, 480p)
- 🌙 Koyu / Açık tema
- 📋 İndirme geçmişi
- 📊 Canlı ilerleme çubuğu (hız, ETA, boyut)
- 📦 **Tek dosya** — son kullanıcı hiçbir şey kurmaz
- 🍎 Windows / macOS / Linux

## 🔧 Geliştirme

### Gereksinimler

- [Rust](https://rustup.rs) (1.70+)
- [Node.js](https://nodejs.org) (18+)
- [npm](https://www.npmjs.com) (9+)

macOS'ta ek olarak:
```bash
xcode-select --install
```

Linux'ta ek olarak:
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
```

### Kurulum

```bash
# Repo'yu klonla
cd ytGUI

# npm bağımlılıklarını yükle
npm install

# Binary'leri indir (opsiyonel — uygulama ilk açılışta otomatik indirir)
npm run download-binaries
```

### Geliştirme modunda çalıştır

```bash
cargo tauri dev
```

### Build (tek dosya)

```bash
cargo tauri build
```

Build çıktısı:
- **Windows:** `src-tauri/target/release/bundle/msi/ytGUI_x.x.x_x64_en-US.msi`
- **macOS:** `src-tauri/target/release/bundle/dmg/ytGUI_x.x.x_x64.dmg`
- **Linux:** `src-tauri/target/release/bundle/appimage/ytgui_x.x.x_amd64.AppImage`

## 🧠 Nasıl Çalışır?

```
[HTML/CSS/JS Arayüz]
        ↕ (IPC)
[Rust Backend (main.rs)]
        ↕ (std::process::Command)
[yt-dlp + ffmpeg binary]
        ↕
[YouTube / Platform API]
```

1. Kullanıcı URL girer → Rust backend yt-dlp ile `--dump-json` çalıştırır
2. Format listesi frontend'e gönderilir
3. Kullanıcı format seçer → Rust yt-dlp'yi indirme için başlatır
4. yt-dlp çıktısı anlık parse edilip frontend'e event olarak gönderilir
5. Gerekirse ffmpeg ile ses/video birleştirilir

## 📁 Proje Yapısı

```
ytGUI/
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── style.css
│   └── app.js
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── binaries/           # yt-dlp & ffmpeg (otomatik indirilir)
│   └── src/
│       ├── main.rs
│       └── lib.rs
├── package.json
└── README.md
```

## 📜 Lisans

MIT License — bakınız LICENSE dosyası.

yt-dlp ve ffmpeg kendi lisanslarına tabidir. Bu proje onları dağıtmaz,
kullanıcının sisteminden veya otomatik indirme ile GitHub/ffmpeg.org'dan temin eder.

---

**Made with ❤️, Rust, and Tauri**
