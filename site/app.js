const translations = {
  en: {
    nav_features: "Features", nav_downloads: "Downloads", nav_docs: "Documentation",
    eyebrow: "Desktop game-localization toolbox",
    hero_title: "TBX Translator automates game localization.",
    hero_text: "It extracts and translates text, supports manual review, and prepares translations for injection into the game.",
    download_latest: "Download latest version", read_docs: "Read documentation", latest_release: "Latest release",
    renpy_desc: "Dialogue, language menus and fonts", unity_desc: "Mono, IL2CPP and XUnity workflows", godot_desc: "PCK, PO and native catalogs",
    why_tbx: "Why TBX Translator", features_title: "A complete translation workflow",
    feature_extract_title: "Broad extraction", feature_extract_text: "Find interface text, dialogue and story content across supported engines.",
    feature_format_title: "Formatting protection", feature_format_text: "Keep variables, BBCode, tags, whitespace and placeholders in their original positions.",
    feature_review_title: "Review before install", feature_review_text: "Inspect and adjust generated translations in the built-in editor.",
    feature_tasks_title: "Independent tasks", feature_tasks_text: "Run engines separately with progress, logs, cache and cancellation.",
    feature_future_title: "More formats in the future", feature_future_text: "Planned expansion for RPG Maker, Wolf RPG Editor, Unreal Engine and other game formats.",
    get_app: "Get the application", downloads_title: "Downloads", downloads_text: "Files are loaded directly from the latest GitHub release.",
    loading_downloads: "Loading available downloads…", download_error: "Could not load individual files.", open_release: "Open the latest release",
    learn_more: "Learn more", docs_title: "All documentation in one place",
    docs_text: "Browse guides, architecture, API notes, changelog and development records without leaving the site.", open_docs: "Open documentation",
    download: "Download", installer: "Installer", portable_zip: "Portable ZIP", package: "Package", appimage: "AppImage",
    back_home: "Home", documentation: "Documentation", docs_library: "Library", loading_document: "Loading documentation…",
    document_error: "Documentation could not be loaded."
  },
  "pt-BR": {
    nav_features: "Recursos", nav_downloads: "Downloads", nav_docs: "Documentação",
    eyebrow: "Ferramenta de localização de jogos",
    hero_title: "O TBX Translator automatiza a localização de jogos.",
    hero_text: "Ele extrai e traduz textos, permite revisão manual e prepara as traduções para injeção no jogo.",
    download_latest: "Baixar versão mais recente", read_docs: "Ler documentação", latest_release: "Release mais recente",
    renpy_desc: "Diálogos, menus de idioma e fontes", unity_desc: "Fluxos Mono, IL2CPP e XUnity", godot_desc: "PCK, PO e catálogos nativos",
    why_tbx: "Por que usar o TBX Translator", features_title: "Um fluxo completo de tradução",
    feature_extract_title: "Extração abrangente", feature_extract_text: "Encontre interface, diálogos e histórias nas engines compatíveis.",
    feature_format_title: "Proteção da formatação", feature_format_text: "Preserve variáveis, BBCode, tags, espaços e marcadores nas posições originais.",
    feature_review_title: "Revise antes de instalar", feature_review_text: "Confira e ajuste as traduções no editor integrado.",
    feature_tasks_title: "Tarefas independentes", feature_tasks_text: "Execute engines separadamente com progresso, logs, cache e cancelamento.",
    feature_future_title: "Mais formatos no futuro", feature_future_text: "Expansão planejada para RPG Maker, Wolf RPG Editor, Unreal Engine e outros formatos de jogos.",
    get_app: "Obtenha o aplicativo", downloads_title: "Downloads", downloads_text: "Os arquivos são carregados diretamente da release mais recente do GitHub.",
    loading_downloads: "Carregando downloads disponíveis…", download_error: "Não foi possível carregar os arquivos individuais.", open_release: "Abrir a release mais recente",
    learn_more: "Saiba mais", docs_title: "Toda a documentação em um só lugar",
    docs_text: "Consulte guias, arquitetura, API, changelog e registros de desenvolvimento sem sair do site.", open_docs: "Abrir documentação",
    download: "Baixar", installer: "Instalador", portable_zip: "ZIP portátil", package: "Pacote", appimage: "AppImage",
    back_home: "Início", documentation: "Documentação", docs_library: "Biblioteca", loading_document: "Carregando documentação…",
    document_error: "Não foi possível carregar a documentação."
  }
};

let currentLanguage = localStorage.getItem("tbx-site-language") || "en";
let latestRelease = null;

function t(key) {
  return translations[currentLanguage]?.[key] || translations.en[key] || key;
}

function resetMachineTranslation() {
  const active = document.cookie.split(";").some(item => item.trim().startsWith("googtrans="));
  if (!active) return false;
  const rootDomain = `.${location.hostname.split(".").slice(-2).join(".")}`;
  for (const domain of ["", rootDomain]) {
    document.cookie = `googtrans=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;${domain ? `domain=${domain};` : ""}`;
  }
  return true;
}

function applyLanguage(language) {
  currentLanguage = translations[language] ? language : "en";
  localStorage.setItem("tbx-site-language", currentLanguage);
  document.documentElement.lang = currentLanguage;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = t(element.dataset.i18n);
  });
  const picker = document.querySelector("#native-language");
  if (picker) picker.value = currentLanguage;
  renderDownloads();
  document.dispatchEvent(new CustomEvent("tbx-language", { detail: currentLanguage }));
}

function formatBytes(bytes) {
  if (!bytes) return "";
  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`;
}

const platforms = [
  [/Setup\.exe$/i, "Windows", "installer"],
  [/Windows-x64\.zip$/i, "Windows", "portable_zip"],
  [/Debian-Ubuntu.*\.deb$/i, "Debian / Ubuntu", "package"],
  [/Fedora.*\.rpm$/i, "Fedora", "package"],
  [/Arch.*\.pkg\.tar\.zst$/i, "Arch Linux", "package"],
  [/\.AppImage$/i, "Linux", "appimage"]
];

function renderDownloads() {
  const grid = document.querySelector("#download-grid");
  if (!grid || !latestRelease) return;
  const cards = [];
  for (const [pattern, platform, typeKey] of platforms) {
    const asset = latestRelease.assets.find(item => pattern.test(item.name));
    if (!asset) continue;
    cards.push(`<article class="card download-card"><span class="platform">${platform}</span><div class="asset-name">${asset.name}</div><div class="asset-meta">${t(typeKey)} · ${formatBytes(asset.size)}</div><a class="button secondary" href="${asset.browser_download_url}">${t("download")}</a></article>`);
  }
  grid.innerHTML = cards.join("");
}

async function loadRelease() {
  const version = document.querySelector("#release-version");
  const grid = document.querySelector("#download-grid");
  const error = document.querySelector("#download-error");
  if (!grid) return;
  try {
    const endpoint = `https://api.github.com/repos/Samwns/TBX-Translator/releases/latest?ts=${Date.now()}`;
    const response = await fetch(endpoint, { cache: "no-store", headers: { Accept: "application/vnd.github+json" } });
    if (!response.ok) throw new Error(response.status);
    latestRelease = await response.json();
    if (version) version.textContent = latestRelease.name || latestRelease.tag_name;
    renderDownloads();
    if (!grid.children.length) throw new Error("no assets");
    error?.classList.add("hidden");
  } catch (_) {
    grid.innerHTML = "";
    error?.classList.remove("hidden");
    if (version) version.textContent = "GitHub Releases";
  }
}

document.addEventListener("DOMContentLoaded", () => {
  const picker = document.querySelector("#native-language");
  picker?.addEventListener("change", event => {
    const selected = event.target.value;
    localStorage.setItem("tbx-site-language", selected);
    if (resetMachineTranslation()) {
      location.reload();
      return;
    }
    applyLanguage(selected);
  });
  document.querySelector(".menu-button")?.addEventListener("click", event => {
    const nav = document.querySelector(".main-nav");
    const open = nav.classList.toggle("open");
    event.currentTarget.setAttribute("aria-expanded", String(open));
  });
  applyLanguage(currentLanguage);
  loadRelease();
});
