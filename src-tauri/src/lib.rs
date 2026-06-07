use regex::Regex;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct AppState {
    ytdlp_path: Mutex<Option<PathBuf>>,
    ffmpeg_path: Mutex<Option<PathBuf>>,
    active_download: Mutex<Option<std::process::Child>>,
}

// ---------------------------------------------------------------------------
// Data types sent to the frontend
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
struct VideoInfo {
    title: String,
    thumbnail: String,
    duration: i64,
    uploader: String,
    webpage_url: String,
    formats: Vec<FormatOption>,
}

#[derive(Serialize, Clone)]
struct FormatOption {
    id: String,
    description: String,
    note: String,
    is_audio_only: bool,
}

#[derive(Serialize, Clone, Debug)]
struct DownloadProgress {
    percent: f64,
    speed: String,
    eta: String,
    total_size: String,
    status: String, // "downloading" | "processing" | "done" | "error" | "already_exists"
}

// ---------------------------------------------------------------------------
// Binary helpers
// ---------------------------------------------------------------------------

fn get_bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {e}"))?
        .join("binaries");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    Ok(dir)
}

async fn download_ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    let bin_dir = get_bin_dir(app)?;
    let name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    let dest = bin_dir.join(name);

    let url = if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    };

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download yt-dlp: {e}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read yt-dlp body: {e}"))?;
    std::fs::write(&dest, &bytes).map_err(|e| format!("Failed to save yt-dlp: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)
            .map_err(|e| format!("metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).map_err(|e| format!("chmod: {e}"))?;
    }

    Ok(dest)
}

async fn download_ffmpeg(app: &AppHandle) -> Result<PathBuf, String> {
    let bin_dir = get_bin_dir(app)?;
    let name = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let dest = bin_dir.join(name);

    if dest.is_file() {
        return Ok(dest);
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: evermeet.cx provides a zip with just the ffmpeg binary inside
        let url = "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip";
        let tmp_zip = bin_dir.join("ffmpeg_tmp.zip");

        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("ffmpeg indirilemedi: {e}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("ffmpeg okunamadı: {e}"))?;
        std::fs::write(&tmp_zip, &bytes)
            .map_err(|e| format!("ffmpeg kaydedilemedi: {e}"))?;

        // Extract with system unzip
        let status = std::process::Command::new("unzip")
            .args(["-o", tmp_zip.to_str().unwrap(), "-d", bin_dir.to_str().unwrap()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("unzip çalıştırılamadı: {e}"))?;

        std::fs::remove_file(&tmp_zip).ok();

        if !status.success() || !dest.is_file() {
            return Err("ffmpeg arşivden çıkarılamadı".into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: johnvansickle.com static build (.tar.xz)
        let url = "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz";
        let tmp_tar = bin_dir.join("ffmpeg_tmp.tar.xz");
        let tmp_dir = bin_dir.join("ffmpeg_extract");

        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("ffmpeg indirilemedi: {e}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("ffmpeg okunamadı: {e}"))?;
        std::fs::write(&tmp_tar, &bytes)
            .map_err(|e| format!("ffmpeg kaydedilemedi: {e}"))?;

        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("geçici dizin oluşturulamadı: {e}"))?;

        let status = std::process::Command::new("tar")
            .args(["-xf", tmp_tar.to_str().unwrap(), "-C", tmp_dir.to_str().unwrap()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("tar çalıştırılamadı: {e}"))?;

        // Find ffmpeg binary inside extracted dir
        if status.success() {
            if let Some(found) = find_file(&tmp_dir, "ffmpeg") {
                std::fs::copy(&found, &dest)
                    .map_err(|e| format!("ffmpeg kopyalanamadı: {e}"))?;
            }
        }

        std::fs::remove_file(&tmp_tar).ok();
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: gyan.dev essentials build
        let url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
        let tmp_zip = bin_dir.join("ffmpeg_tmp.zip");
        let tmp_dir = bin_dir.join("ffmpeg_extract");

        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("ffmpeg indirilemedi: {e}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("ffmpeg okunamadı: {e}"))?;
        std::fs::write(&tmp_zip, &bytes)
            .map_err(|e| format!("ffmpeg kaydedilemedi: {e}"))?;

        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("geçici dizin oluşturulamadı: {e}"))?;

        // Use PowerShell Expand-Archive
        let status = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    tmp_zip.display(),
                    tmp_dir.display()
                ),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("PowerShell çalıştırılamadı: {e}"))?;

        if status.success() {
            if let Some(found) = find_file(&tmp_dir, "ffmpeg.exe") {
                std::fs::copy(&found, &dest)
                    .map_err(|e| format!("ffmpeg kopyalanamadı: {e}"))?;
            }
        }

        std::fs::remove_file(&tmp_zip).ok();
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if dest.is_file() {
            let mut perms = std::fs::metadata(&dest)
                .map_err(|e| format!("metadata: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms).map_err(|e| format!("chmod: {e}"))?;
        }
    }

    if dest.is_file() {
        Ok(dest)
    } else {
        Err("ffmpeg indirilemedi — lütfen manuel olarak yükleyin".into())
    }
}

/// Recursively find a file by name in a directory
fn find_file(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map_or(false, |n| n == name) {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = find_file(&path, name) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let exe = dir.join(name);
            if exe.is_file() {
                Some(exe)
            } else {
                None
            }
        })
    })
}

async fn ensure_ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    let state = app.state::<AppState>();
    {
        let cached = state.ytdlp_path.lock().unwrap();
        if let Some(ref p) = *cached {
            if p.is_file() {
                return Ok(p.clone());
            }
        }
    }

    // Try PATH first
    let name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    if let Some(p) = find_on_path(name) {
        let mut cached = state.ytdlp_path.lock().unwrap();
        *cached = Some(p.clone());
        return Ok(p);
    }

    // Try app data dir
    let bin_dir = get_bin_dir(app)?;
    let local = bin_dir.join(name);
    if local.is_file() {
        let mut cached = state.ytdlp_path.lock().unwrap();
        *cached = Some(local.clone());
        return Ok(local);
    }

    // Download
    let p = download_ytdlp(app).await?;
    let mut cached = state.ytdlp_path.lock().unwrap();
    *cached = Some(p.clone());
    Ok(p)
}

async fn ensure_ffmpeg(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let state = app.state::<AppState>();
    {
        let cached = state.ffmpeg_path.lock().unwrap();
        if let Some(ref p) = *cached {
            if p.is_file() {
                return Ok(Some(p.clone()));
            }
        }
    }

    let name = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    if let Some(p) = find_on_path(name) {
        let mut cached = state.ffmpeg_path.lock().unwrap();
        *cached = Some(p.clone());
        return Ok(Some(p));
    }

    // Check app data dir
    if let Ok(bin_dir) = get_bin_dir(app) {
        let local = bin_dir.join(name);
        if local.is_file() {
            let mut cached = state.ffmpeg_path.lock().unwrap();
            *cached = Some(local.clone());
            return Ok(Some(local));
        }
    }

    // Try downloading ffmpeg
    match download_ffmpeg(app).await {
        Ok(p) => {
            let mut cached = state.ffmpeg_path.lock().unwrap();
            *cached = Some(p.clone());
            Ok(Some(p))
        }
        Err(e) => {
            eprintln!("ffmpeg download failed: {e}");
            Ok(None) // ffmpeg is optional — yt-dlp works without it for basic downloads
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn fetch_video_info(url: String, app: AppHandle) -> Result<VideoInfo, String> {
    let ytdlp = ensure_ytdlp(&app).await?;

    let output = Command::new(&ytdlp)
        .args([url.as_str(), "--dump-json", "--no-playlist"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("yt-dlp çalıştırılamadı: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        // Try to extract useful error message
        let err_msg = stderr.lines().last().unwrap_or("Bilinmeyen hata");
        return Err(format!("yt-dlp hatası: {err_msg}"));
    }

    // yt-dlp may output multiple JSON lines (first line = video info)
    let first_line = stdout
        .lines()
        .next()
        .ok_or("yt-dlp boş çıktı verdi")?;

    let raw: serde_json::Value =
        serde_json::from_str(first_line).map_err(|e| format!("JSON parse hatası: {e}"))?;

    let title = raw["title"].as_str().unwrap_or("Bilinmeyen").to_string();
    let thumbnail = raw["thumbnail"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let duration = raw["duration"].as_i64().unwrap_or(0);
    let uploader = raw["uploader"].as_str().unwrap_or("Bilinmeyen").to_string();
    let webpage_url = raw["webpage_url"]
        .as_str()
        .unwrap_or(&url)
        .to_string();

    // Build a sensible format list
    let mut formats: Vec<FormatOption> = Vec::new();
    let raw_formats = raw["formats"].as_array();

    // -- Best combined (video+audio) --
    formats.push(FormatOption {
        id: "bestvideo+bestaudio/best".into(),
        description: "En İyi Kalite (Video + Ses)".into(),
        note: "Otomatik".into(),
        is_audio_only: false,
    });

    // -- Audio only --
    formats.push(FormatOption {
        id: "bestaudio/best".into(),
        description: "Sadece Ses (MP3)".into(),
        note: "En iyi ses kalitesi".into(),
        is_audio_only: true,
    });

    // -- Resolution-specific formats --
    if let Some(arr) = raw_formats {
        let mut seen_res: Vec<String> = Vec::new();
        for f in arr {
            let height = f["height"].as_i64().unwrap_or(0);
            let vcodec = f["vcodec"].as_str().unwrap_or("none");
            let acodec = f["acodec"].as_str().unwrap_or("none");
            let _format_id = f["format_id"].as_str().unwrap_or("");

            if height >= 360 && vcodec != "none" && !seen_res.contains(&height.to_string()) {
                seen_res.push(height.to_string());
                let has_audio = acodec != "none";
                let label = match height {
                    h if h >= 2160 => "4K (2160p)",
                    h if h >= 1440 => "2K (1440p)",
                    h if h >= 1080 => "1080p (Full HD)",
                    h if h >= 720 => "720p (HD)",
                    _ => "SD",
                };

                let id = if has_audio {
                    format!(
                        "bestvideo[height<={height}]+bestaudio/best[height<={height}]",
                        height = height
                    )
                } else {
                    format!(
                        "bestvideo[height<={height}]+bestaudio/best[height<={height}]",
                        height = height
                    )
                };

                formats.push(FormatOption {
                    id,
                    description: format!("{label} — Video + Ses"),
                    note: format!("Maks {height}p"),
                    is_audio_only: false,
                });
            }
        }
    }

    Ok(VideoInfo {
        title,
        thumbnail,
        duration,
        uploader,
        webpage_url,
        formats,
    })
}

#[tauri::command]
async fn start_download(
    url: String,
    format_id: String,
    output_dir: String,
    audio_only: bool,
    app: AppHandle,
) -> Result<String, String> {
    let ytdlp = ensure_ytdlp(&app).await?;

    let output_template = format!("{}/%(title)s.%(ext)s", output_dir);

    let mut args: Vec<String> = vec![
        url.clone(),
        "-o".into(),
        output_template,
        "--newline".into(),
        "--progress".into(),
        "--no-playlist".into(),
        "--no-mtime".into(),
    ];

    if audio_only {
        args.push("-x".into());
        args.push("--audio-format".into());
        args.push("mp3".into());
        args.push("-f".into());
        args.push("bestaudio/best".into());
    } else if !format_id.is_empty() {
        args.push("-f".into());
        args.push(format_id.clone());
    } else {
        args.push("-f".into());
        args.push("bestvideo+bestaudio/best".into());
    }

    // Try to use ffmpeg if available for merging
    if let Ok(Some(_ff)) = ensure_ffmpeg(&app).await {
        args.push("--ffmpeg-location".into());
        args.push(_ff.to_string_lossy().to_string());
    }

    let mut child = Command::new(&ytdlp)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("yt-dlp başlatılamadı: {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Store child in state so cancel_download can kill it
    {
        let state = app.state::<AppState>();
        let mut download = state.active_download.lock().unwrap();
        // Kill any previous download
        if let Some(ref mut old) = *download {
            let _ = old.kill();
            let _ = old.wait();
        }
        *download = Some(child);
    }

    // Read yt-dlp stdout on a background thread, emit progress events
    let app_stdout = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let progress_re =
            Regex::new(r"\[download\]\s+(\d+\.?\d*)%\s+of\s+(.+?)\s+at\s+(.+?)/s\s+ETA\s+(.+)")
                .unwrap();
        let already_re = Regex::new(r"(?i)(has already been downloaded|already exists)").unwrap();

        for line in reader.lines() {
            match line {
                Ok(ref l) if progress_re.is_match(l) => {
                    let caps = progress_re.captures(l).unwrap();
                    let progress = DownloadProgress {
                        percent: caps[1].parse().unwrap_or(0.0),
                        total_size: caps[2].to_string(),
                        speed: caps[3].to_string(),
                        eta: caps[4].to_string(),
                        status: "downloading".into(),
                    };
                    app_stdout.emit("download-progress", &progress).ok();
                }
                Ok(ref l) if already_re.is_match(l) => {
                    let progress = DownloadProgress {
                        percent: 100.0,
                        speed: "—".into(),
                        eta: "00:00".into(),
                        total_size: "—".into(),
                        status: "already_exists".into(),
                    };
                    app_stdout.emit("download-progress", &progress).ok();
                }
                Ok(ref l)
                    if l.contains("Merging")
                        || l.contains("ExtractAudio")
                        || l.contains("[ffmpeg]") =>
                {
                    let progress = DownloadProgress {
                        percent: 99.0,
                        speed: "—".into(),
                        eta: "—".into(),
                        total_size: "—".into(),
                        status: "processing".into(),
                    };
                    app_stdout.emit("download-progress", &progress).ok();
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // Read stderr on a background thread, emit errors
    let app_stderr = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                if l.contains("ERROR") || l.contains("Error") || l.contains("WARNING") {
                    app_stderr.emit("download-error", &l).ok();
                }
            }
        }
        // When stderr closes, the process has finished
        app_stderr
            .emit(
                "download-progress",
                &DownloadProgress {
                    percent: 100.0,
                    speed: "—".into(),
                    eta: "00:00".into(),
                    total_size: "—".into(),
                    status: "done".into(),
                },
            )
            .ok();
    });

    // Waiter thread: clean up child from state when done
    let app_waiter = app.clone();
    std::thread::spawn(move || {
        let state = app_waiter.state::<AppState>();
        let mut download = state.active_download.lock().unwrap();
        if let Some(ref mut child) = *download {
            let _ = child.wait();
        }
        *download = None;
    });

    Ok("İndirme başladı".into())
}

#[tauri::command]
async fn pick_directory(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app.dialog().file().blocking_pick_folder();
    match folder {
        Some(path) => Ok(path.to_string()),
        None => Err("Klasör seçilmedi".into()),
    }
}

#[tauri::command]
fn cancel_download(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut download = state.active_download.lock().unwrap();
    if let Some(ref mut child) = *download {
        child.kill().map_err(|e| format!("Durdurulamadı: {e}"))?;
        *download = None;
        app.emit(
            "download-progress",
            &DownloadProgress {
                percent: 0.0,
                speed: "—".into(),
                eta: "—".into(),
                total_size: "—".into(),
                status: "cancelled".into(),
            },
        )
        .ok();
        Ok("İndirme durduruldu".into())
    } else {
        Err("Aktif indirme yok".into())
    }
}

#[tauri::command]
fn check_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    let state = app.state::<AppState>();

    let ytdlp_ready = {
        let cached = state.ytdlp_path.lock().unwrap();
        cached.is_some()
    };

    let ffmpeg_ready = {
        let cached = state.ffmpeg_path.lock().unwrap();
        cached.is_some()
    };

    Ok(BinaryStatus {
        ytdlp: ytdlp_ready,
        ffmpeg: ffmpeg_ready,
    })
}

#[derive(Serialize)]
struct BinaryStatus {
    ytdlp: bool,
    ffmpeg: bool,
}

// ---------------------------------------------------------------------------
// App entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            ytdlp_path: Mutex::new(None),
            ffmpeg_path: Mutex::new(None),
            active_download: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            fetch_video_info,
            start_download,
            cancel_download,
            pick_directory,
            check_binaries,
        ])
        .setup(|app| {
            // Pre-warm: try to locate binaries on startup
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = ensure_ytdlp(&handle).await;
                let _ = ensure_ffmpeg(&handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ytGUI başlatılırken hata oluştu");
}
