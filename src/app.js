// ===================================================================
// ytGUI — Frontend Logic (ES Module)
// ===================================================================

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ===================================================================
// DOM refs
// ===================================================================
const $ = (sel) => document.querySelector(sel);

const dom = {
  disclaimerModal: $("#disclaimer-modal"),
  disclaimerAccept: $("#disclaimer-accept"),
  disclaimerBtn: $("#disclaimer-btn"),
  themeToggle: $("#theme-toggle"),
  urlInput: $("#url-input"),
  fetchBtn: $("#fetch-btn"),
  loading: $("#loading"),
  videoInfo: $("#video-info"),
  thumbnail: $("#thumbnail"),
  videoTitle: $("#video-title"),
  videoUploader: $("#video-uploader"),
  videoDuration: $("#video-duration"),
  videoLink: $("#video-link"),
  formatSelect: $("#format-select"),
  outputDir: $("#output-dir"),
  dirBtn: $("#dir-btn"),
  downloadBtn: $("#download-btn"),
  progressSection: $("#progress-section"),
  progressStatusText: $("#progress-status-text"),
  progressPercent: $("#progress-percent"),
  progressFill: $("#progress-fill"),
  progressSpeed: $("#progress-speed"),
  progressEta: $("#progress-eta"),
  progressSize: $("#progress-size"),
  cancelBtn: $("#cancel-btn"),
  historyList: $("#history-list"),
  toastContainer: $("#toast-container"),
};

// ===================================================================
// State
// ===================================================================
let currentVideoInfo = null;
let selectedFormatId = "";
let selectedAudioOnly = false;
let isDownloading = false;

// ===================================================================
// Disclaimer Modal
// ===================================================================
function initDisclaimer() {
  const accepted = localStorage.getItem("ytgui-disclaimer-accepted");
  if (accepted === "true") {
    dom.disclaimerModal.classList.add("hidden");
    return;
  }
  dom.disclaimerModal.classList.remove("hidden");
}

dom.disclaimerAccept.addEventListener("change", () => {
  dom.disclaimerBtn.disabled = !dom.disclaimerAccept.checked;
});

dom.disclaimerBtn.addEventListener("click", () => {
  if (dom.disclaimerAccept.checked) {
    localStorage.setItem("ytgui-disclaimer-accepted", "true");
    dom.disclaimerModal.classList.add("hidden");
    toast("Uygulamaya hoş geldiniz! 🎉", "success");
  }
});

// ===================================================================
// Theme
// ===================================================================
function initTheme() {
  const saved = localStorage.getItem("ytgui-theme");
  if (saved === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
  }
}

dom.themeToggle.addEventListener("click", () => {
  const current = document.documentElement.getAttribute("data-theme");
  if (current === "dark") {
    document.documentElement.removeAttribute("data-theme");
    localStorage.setItem("ytgui-theme", "light");
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
    localStorage.setItem("ytgui-theme", "dark");
  }
});

// ===================================================================
// Toast
// ===================================================================
function toast(message, type = "info") {
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.textContent = message;
  dom.toastContainer.appendChild(el);
  setTimeout(() => el.remove(), 3200);
}

// ===================================================================
// Helpers
// ===================================================================
function formatDuration(seconds) {
  if (!seconds || seconds <= 0) return "—";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function formatDate(iso) {
  const d = new Date(iso);
  return d.toLocaleDateString("tr-TR", {
    day: "2-digit", month: "2-digit", year: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ===================================================================
// History
// ===================================================================
function getHistory() {
  try { return JSON.parse(localStorage.getItem("ytgui-history") || "[]"); }
  catch { return []; }
}

function addToHistory(title, url, status) {
  const history = getHistory();
  history.unshift({ title, url, status, date: new Date().toISOString() });
  localStorage.setItem("ytgui-history", JSON.stringify(history.slice(0, 20)));
  renderHistory();
}

function renderHistory() {
  const history = getHistory();
  if (!history.length) {
    dom.historyList.innerHTML = '<p class="empty">Henüz indirme yapılmadı</p>';
    return;
  }
  dom.historyList.innerHTML = history
    .map(h => `
      <div class="history-item">
        <span class="hi-title" title="${esc(h.title)}">${esc(h.title)}</span>
        <span class="hi-date">${formatDate(h.date)}</span>
        <span class="hi-status ${h.status}">${h.status === "done" ? "✓" : "✗"}</span>
      </div>`)
    .join("");
}

// ===================================================================
// Fetch Video Info
// ===================================================================
async function fetchVideoInfo() {
  const url = dom.urlInput.value.trim();
  if (!url) return toast("Lütfen bir URL girin", "warning");
  if (!url.startsWith("http://") && !url.startsWith("https://"))
    return toast("Geçerli bir URL girin", "warning");

  dom.loading.classList.remove("hidden");
  dom.videoInfo.classList.add("hidden");
  dom.progressSection.classList.add("hidden");

  try {
    currentVideoInfo = await invoke("fetch_video_info", { url });
    renderVideoInfo(currentVideoInfo);
    dom.loading.classList.add("hidden");
    dom.videoInfo.classList.remove("hidden");
    toast("Video bilgisi alındı ✓", "success");
  } catch (err) {
    dom.loading.classList.add("hidden");
    toast(`Hata: ${err}`, "error");
    console.error("fetch_video_info:", err);
  }
}

function renderVideoInfo(info) {
  dom.thumbnail.src = info.thumbnail || "";
  dom.thumbnail.onerror = () => {
    dom.thumbnail.src =
      "data:image/svg+xml," +
      encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" fill="%23333"><rect width="320" height="180"/><text x="160" y="95" text-anchor="middle" fill="%23666" font-size="14">No Thumbnail</text></svg>');
  };
  dom.videoTitle.textContent = info.title;
  dom.videoUploader.textContent = info.uploader;
  dom.videoDuration.textContent = formatDuration(info.duration);
  dom.videoLink.href = info.webpage_url;

  dom.formatSelect.innerHTML = info.formats
    .map((f, i) =>
      `<option value="${esc(f.id)}" data-audio="${f.is_audio_only ? "1" : "0"}" ${i === 0 ? "selected" : ""}>
        ${esc(f.description)} — ${esc(f.note)}
      </option>`)
    .join("");

  updateFormatSelection();
}

function updateFormatSelection() {
  const opt = dom.formatSelect.selectedOptions[0];
  if (opt) {
    selectedFormatId = opt.value;
    selectedAudioOnly = opt.dataset.audio === "1";
  }
}

dom.formatSelect.addEventListener("change", updateFormatSelection);

// ===================================================================
// Directory Picker
// ===================================================================
async function pickDirectory() {
  try {
    const dir = await invoke("pick_directory");
    dom.outputDir.value = dir;
  } catch (err) {
    if (err !== "Klasör seçilmedi") toast(`Hata: ${err}`, "error");
  }
}

dom.dirBtn.addEventListener("click", pickDirectory);

// ===================================================================
// Download
// ===================================================================
async function startDownload() {
  if (isDownloading) return toast("Zaten bir indirme devam ediyor", "warning");
  if (!currentVideoInfo) return toast("Önce video bilgisi getirin", "warning");

  const outputDir = dom.outputDir.value.trim();
  if (!outputDir) return toast("Lütfen indirme klasörü seçin", "warning");

  updateFormatSelection();

  isDownloading = true;
  dom.downloadBtn.disabled = true;
  dom.downloadBtn.textContent = "⏳ İndiriliyor...";
  dom.progressSection.classList.remove("hidden");
  dom.cancelBtn.classList.remove("hidden");
  dom.progressFill.style.width = "0%";
  dom.progressPercent.textContent = "%0";
  dom.progressSpeed.textContent = "";
  dom.progressEta.textContent = "";
  dom.progressSize.textContent = "";
  dom.progressStatusText.textContent = "Başlatılıyor...";

  try {
    await invoke("start_download", {
      url: currentVideoInfo.webpage_url,
      formatId: selectedFormatId,
      outputDir: outputDir,
      audioOnly: selectedAudioOnly,
    });
  } catch (err) {
    toast(`İndirme hatası: ${err}`, "error");
    resetDownload();
    addToHistory(currentVideoInfo.title, currentVideoInfo.webpage_url, "error");
  }
}

function resetDownload() {
  isDownloading = false;
  dom.downloadBtn.disabled = false;
  dom.downloadBtn.textContent = "⬇️ İndirmeyi Başlat";
  dom.cancelBtn.classList.add("hidden");
}

async function cancelDownload() {
  try {
    await invoke("cancel_download");
    toast("İndirme durduruldu", "warning");
    resetDownload();
  } catch (err) {
    toast(`Durdurulamadı: ${err}`, "error");
  }
}

dom.downloadBtn.addEventListener("click", startDownload);
dom.cancelBtn.addEventListener("click", cancelDownload);

// ===================================================================
// Enter → fetch
// ===================================================================
dom.urlInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    fetchVideoInfo();
  }
});
dom.fetchBtn.addEventListener("click", fetchVideoInfo);

// ===================================================================
// Progress Events
// ===================================================================
function updateProgressUI(p) {
  dom.progressFill.style.width = `${p.percent}%`;
  dom.progressPercent.textContent = `%${Math.round(p.percent)}`;
  dom.progressSpeed.textContent = p.speed !== "—" ? `⚡ ${p.speed}` : "";
  dom.progressEta.textContent = p.eta !== "—" ? `⏱ ${p.eta}` : "";
  dom.progressSize.textContent = p.total_size !== "—" ? `📦 ${p.total_size}` : "";

  switch (p.status) {
    case "downloading":
      dom.progressStatusText.textContent = "İndiriliyor...";
      break;
    case "processing":
      dom.progressStatusText.textContent = "İşleniyor (ffmpeg)...";
      break;
    case "already_exists":
      dom.progressStatusText.textContent = "⚠️ Dosya zaten mevcut";
      toast("Bu dosya zaten indirilmiş", "warning");
      break;
    case "done":
      dom.progressStatusText.textContent = "✅ Tamamlandı!";
      dom.progressFill.style.width = "100%";
      dom.progressPercent.textContent = "%100";
      toast("İndirme tamamlandı! 🎉", "success");
      if (currentVideoInfo) {
        addToHistory(currentVideoInfo.title, currentVideoInfo.webpage_url, "done");
      }
      resetDownload();
      break;
    case "cancelled":
      dom.progressStatusText.textContent = "⏹ Durduruldu";
      dom.progressFill.style.width = "0%";
      dom.progressPercent.textContent = "%0";
      resetDownload();
      break;
    case "error":
      dom.progressStatusText.textContent = "❌ Hata oluştu";
      resetDownload();
      break;
  }
}

// ===================================================================
// Init
// ===================================================================
async function init() {
  initTheme();
  initDisclaimer();
  renderHistory();

  await listen("download-progress", (event) => updateProgressUI(event.payload));
  await listen("download-error", (event) => console.warn("yt-dlp:", event.payload));
}

init().catch(console.error);
