/**
 * Steam Manifest Downloader - Frontend Application (Tauri v2)
 */

// ============ Tauri API ============
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ============ State ============
const state = {
  currentStep: 1,
  mode: 'upload', // 'upload' or 'search'
  parsedData: null,
  selectedDepots: new Set(),
  jobId: null,
  unlistenProgress: null,
  gameName: null,
  headerImage: null,
  downloadDir: null,
  shortcutSupported: false,
  notificationsEnabled: false,
  notificationSoundEnabled: true,
  depotManifests: {}, // depotId -> { originalName, storedPath }
  // Search mode state
  searchRepos: [],
  selectedRepo: null,
  searchAppId: null,
  searchRepo: null,
  searchSha: null,
  searchKeyVdfKeys: null,
  // Speed tracking
  speedTracker: {
    lastPercent: 0,
    lastTime: 0,
    samples: [],     // Array of { percent, time } für gleitenden Durchschnitt
    currentDepotSize: 0,  // Größe des aktuell herunterladenen Depots in Bytes
    currentDepotId: null,
    depotStartTime: 0,      // Wann der aktuelle Depot-Download gestartet hat
    staleTimer: null,        // setInterval für Elapsed-Timer wenn kein Fortschritt
    lastUpdateTime: 0,       // Letzter Zeitpunkt mit echtem Prozent-Update
  },
};

// ============ Constants ============
const MH_APIKEY_STORAGE_KEY = 'manifestHubApiKey';
let autoRedownloadPending = false;
let autoSelectAllOnStep2 = false;
let autoSelectDepotIds = null; // specific depot IDs to re-select, or null for all
let defaultDownloadDir = '';

// ============ DOM Elements ============
const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const els = {
  // Steps
  stepUpload: $('#step-upload'),
  stepSelect: $('#step-select'),
  stepProgress: $('#step-progress'),
  // Tabs
  tabUpload: $('#tab-upload'),
  tabSearch: $('#tab-search'),
  tabContentUpload: $('#tab-content-upload'),
  tabContentSearch: $('#tab-content-search'),
  // Upload
  dropZone: $('#drop-zone'),
  fileInfo: $('#file-info'),
  fileName: $('#file-name'),
  fileRemove: $('#file-remove'),
  uploadError: $('#upload-error'),
  uploadLoading: $('#upload-loading'),
  // Search
  searchAppIdInput: $('#search-appid-input'),
  searchAutocomplete: $('#search-autocomplete'),
  btnSearch: $('#btn-search'),
  searchError: $('#search-error'),
  searchLoading: $('#search-loading'),
  searchResults: $('#search-results'),
  repoList: $('#repo-list'),
  searchNextRow: $('#search-next-row'),
  btnSearchNext: $('#btn-search-next'),
  manifestLoading: $('#manifest-loading'),
  searchGameBanner: $('#search-game-banner'),
  searchGameImage: $('#search-game-image'),
  searchGameName: $('#search-game-name'),
  searchGameDescription: $('#search-game-description'),
  // Select
  appIdDisplay: $('#app-id-display'),
  depotCount: $('#depot-count'),
  depotList: $('#depot-list'),
  btnSelectAll: $('#btn-select-all'),
  btnDeselectAll: $('#btn-deselect-all'),
  btnBack: $('#btn-back'),
  btnDownload: $('#btn-download'),
  // Progress (depot download)
  depotProgressFill: $('#depot-progress-fill'),
  depotProgressText: $('#depot-progress-text'),
  // Speed / ETA
  downloadSpeedInfo: $('#download-speed-info'),
  downloadSpeed: $('#download-speed'),
  downloadEta: $('#download-eta'),
  // Progress
  progressHeader: $('#progress-header'),
  progressBarFill: $('#progress-bar-fill'),
  progressStatus: $('#progress-status'),
  depotProgressList: $('#depot-progress-list'),
  terminalOutput: $('#terminal-output'),
  completionMessage: $('#completion-message'),
  // btnNew removed - now in Step 4
  btnCancel: $('#btn-cancel'),
  // btnStartOver removed - now in Step 4
  mhApiKey: $('#mh-apikey'),
  downloadDirInput: $('#download-dir'),
  btnBrowseDir: $('#btn-browse-dir'),
  // Disk Space
  diskSpaceInfo: $('#disk-space-info'),
  diskSpaceText: $('#disk-space-text'),
  // Game Info (Step 2)
  gameInfoBanner: $('#game-info-banner'),
  gameInfoLoading: $('#game-info-loading'),
  gameHeaderImage: $('#game-header-image'),
  gameName: $('#game-name'),
  gameDescription: $('#game-description'),
  // Modal
  cancelModal: $('#cancel-modal'),
  btnCancelYes: $('#btn-cancel-yes'),
  btnCancelNo: $('#btn-cancel-no'),
  // Theme
  btnThemeToggle: $('#btn-theme-toggle'),
  // Depot Filters
  depotSearch: $('#depotSearch'),
  showSelectedOnly: $('#showSelectedOnly'),
  // Settings
  btnSettings: $('#btn-settings'),
  settingsModal: $('#settings-modal'),
  btnSettingsSave: $('#btn-settings-save'),
  btnSettingsCancel: $('#btn-settings-cancel'),
  autoUpdateToggle: $('#auto-update-toggle'),
  // Advanced Settings
  btnToggleAdvanced: $('#btn-toggle-advanced'),
  advancedSettingsContent: $('#advanced-settings-content'),
  ddExtraArgsInput: $('#dd-extra-args-input'),
  maxRetriesInput: $('#max-retries-input'),
  speedLimitInput: $('#speed-limit-input'),
  proxyInput: $('#proxy-input'),
  notificationSoundToggle: $('#notification-sound-toggle'),
  // Telemetry
  telemetryModal: $('#telemetry-modal'),
  btnTelemetryAccept: $('#btn-telemetry-accept'),
  btnTelemetryDecline: $('#btn-telemetry-decline'),
  telemetryToggle: $('#telemetry-toggle'),
  // Debug / Build Info
  buildInfoChannel: $('#build-info-channel'),
  buildInfoVersion: $('#build-info-version'),
  buildInfoSha: $('#build-info-sha'),
  buildInfoDate: $('#build-info-date'),
  buildInfoProfile: $('#build-info-profile'),
  buildInfoPlatform: $('#build-info-platform'),
  btnCopyBuildInfo: $('#btn-copy-build-info'),
  // History
  btnHistory: $('#btn-history'),
  historyModal: $('#history-modal'),
  historyList: $('#history-list'),
  btnHistoryClear: $('#btn-history-clear'),
  btnHistoryClose: $('#btn-history-close'),
  // Update modal
  updateModal: $('#update-modal'),
  updateVersion: $('#update-version'),
  updateDate: $('#update-date'),
  updateDateRow: $('#update-date-row'),
  updateNotes: $('#update-notes'),
  updateProgressWrap: $('#update-progress-wrap'),
  updateProgressFill: $('#update-progress-fill'),
  updateProgressText: $('#update-progress-text'),
  updateActions: $('#update-actions'),
  btnUpdateNow: $('#btn-update-now'),
  btnUpdateLater: $('#btn-update-later'),
  btnUpdateSkip: $('#btn-update-skip'),
  // Step 4: Shortcuts
  stepShortcut: $('#step-shortcut'),
  step4Connector: $('#step4-connector'),
  step4Indicator: $('#step4-indicator'),
  btnNextStep: $('#btn-next-step'),
  shortcutExePath: $('#shortcut-exe-path'),
  btnBrowseExe: $('#btn-browse-exe'),
  shortcutDetectedSection: $('#shortcut-detected-section'),
  btnToggleDetected: $('#btn-toggle-detected'),
  shortcutDetectedList: $('#shortcut-detected-list'),
  shortcutDesktop: $('#shortcut-desktop'),
  shortcutStartMenu: $('#shortcut-startmenu'),
  shortcutStatus: $('#shortcut-status'),
  btnCreateShortcuts: $('#btn-create-shortcuts'),
  btnShortcutNew: $('#btn-shortcut-new'),
  btnShortcutStartOver: $('#btn-shortcut-start-over'),
};

// ============ Step Navigation ============
function goToStep(step) {
  state.currentStep = step;

  // Update step sections
  [els.stepUpload, els.stepSelect, els.stepProgress, els.stepShortcut].forEach((el, i) => {
    if (!el) return;
    el.classList.toggle('active', i + 1 === step);
    el.classList.toggle('hidden', i + 1 !== step);
  });

  // Update step indicators
  $$('.steps__item').forEach((el) => {
    const s = parseInt(el.dataset.step);
    el.classList.toggle('active', s === step);
    el.classList.toggle('completed', s < step);
  });
}

// ============ Tab Switching ============
function switchTab(tabName) {
  state.mode = tabName;

  // Update tab buttons
  els.tabUpload.classList.toggle('active', tabName === 'upload');
  els.tabSearch.classList.toggle('active', tabName === 'search');

  // Update tab content
  els.tabContentUpload.classList.toggle('active', tabName === 'upload');
  els.tabContentSearch.classList.toggle('active', tabName === 'search');
}

// ============ File Upload (Tauri File Dialog) ============
function initUpload() {
  const dropZone = els.dropZone;

  // Click to open Tauri file dialog
  dropZone.addEventListener('click', openFileDialog);

  // Drag and drop visual feedback (actual file path from Tauri drag-drop event)
  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });

  dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('drag-over');
  });

  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    // HTML5 drag events in webview don't provide full file paths
    // Tauri drag-drop is handled via the 'tauri://drag-drop' event below
  });

  // Listen for Tauri native drag-drop events (provides file paths)
  listen('tauri://drag-drop', (event) => {
    const paths = event.payload.paths || event.payload;
    if (Array.isArray(paths) && paths.length > 0) {
      handleFilePath(paths[0]);
    }
  });

  // Remove file
  els.fileRemove.addEventListener('click', (e) => {
    e.stopPropagation();
    resetUpload();
  });
}

async function openFileDialog() {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      filters: [{ name: 'Lua/ST Files', extensions: ['lua', 'st'] }]
    });
    if (filePath) {
      await handleFilePath(filePath);
    }
  } catch (e) {
    console.error('File dialog error:', e);
  }
}

function resetUpload() {
  els.fileInfo.classList.add('hidden');
  els.dropZone.classList.remove('hidden');
  els.uploadError.classList.add('hidden');
  els.uploadLoading.classList.add('hidden');
  state.parsedData = null;
}

// ============ Per-Depot Manifest File Upload (Tauri Dialog) ============
async function handleDepotManifestFile(depotId) {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      filters: [{ name: 'Manifest Files', extensions: ['manifest'] }]
    });
    if (!filePath) return;

    const fileName = filePath.split(/[\\/]/).pop();

    // Store the file path — the Rust backend will copy it during download
    state.depotManifests[depotId] = {
      originalName: fileName,
      storedPath: filePath
    };

    const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
    const btnEl = document.querySelector(`.depot-manifest-btn[data-depot-id="${depotId}"]`);
    if (statusEl) statusEl.innerHTML = `<span class="manifest-uploaded">✓ ${escapeHtml(fileName)}</span>`;
    if (btnEl) btnEl.textContent = '📁 Replace';
  } catch (error) {
    console.error('Failed to select manifest file:', error);
    alert('Failed to select manifest file: ' + error);
    delete state.depotManifests[depotId];
  }
}

function removeDepotManifest(depotId) {
  delete state.depotManifests[depotId];
  const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
  const btnEl = document.querySelector(`.depot-manifest-btn[data-depot-id="${depotId}"]`);
  if (statusEl) statusEl.innerHTML = '';
  if (btnEl) btnEl.textContent = '📁 Upload .manifest';
}

async function handleFilePath(filePath) {
  // Validate extension
  const ext = filePath.split('.').pop().toLowerCase();
  if (ext !== 'lua' && ext !== 'st') {
    showUploadError('Please select a .lua or .st file');
    return;
  }

  const fileName = filePath.split(/[\\/]/).pop();

  // Show file info
  els.dropZone.classList.add('hidden');
  els.fileInfo.classList.remove('hidden');
  els.fileName.textContent = fileName;
  els.uploadError.classList.add('hidden');
  els.uploadLoading.classList.remove('hidden');

  try {
    const raw = await invoke('parse_lua_file', { path: filePath });

    // Normalize snake_case response to camelCase for internal use
    state.parsedData = {
      mainAppId: raw.main_app_id,
      depots: (raw.depots || []).map(d => ({
        depotId: String(d.depot_id),
        manifestId: d.manifest_id || 'N/A',
        depotKey: d.depot_key || null,
        sizeBytes: d.size_bytes || null
      }))
    };
    state.mode = 'upload';
    els.uploadLoading.classList.add('hidden');
    emitEvent('lua_parsed', { depot_count: state.parsedData.depots.length });

    // Auto-advance to Step 2
    showSelectionStep();
  } catch (error) {
    els.uploadLoading.classList.add('hidden');
    showUploadError(String(error));
  }
}

function showUploadError(message) {
  els.uploadError.textContent = message;
  els.uploadError.classList.remove('hidden');
}

// ============ Autocomplete Search ============
let autocompleteDebounceTimer = null;

function isNumericInput(str) {
  return /^\d+$/.test(str.trim());
}

function hideAutocomplete() {
  els.searchAutocomplete.classList.add('hidden');
  els.searchAutocomplete.innerHTML = '';
}

function showAutocompleteLoading() {
  els.searchAutocomplete.innerHTML = '<div class="search-autocomplete__loading">Searching...</div>';
  els.searchAutocomplete.classList.remove('hidden');
}

function renderAutocompleteResults(results) {
  if (!results || results.length === 0) {
    els.searchAutocomplete.innerHTML = '<div class="search-autocomplete__empty">No games found</div>';
    els.searchAutocomplete.classList.remove('hidden');
    return;
  }

  els.searchAutocomplete.innerHTML = results.map(item => `
    <div class="search-autocomplete__item" data-appid="${escapeHtml(String(item.appId))}">
      <img class="search-autocomplete__img" src="${escapeHtml(item.image || '')}" alt="" loading="lazy" onerror="this.style.display='none'">
      <span class="search-autocomplete__name">${escapeHtml(item.name || '')}</span>
      <span class="search-autocomplete__appid">${escapeHtml(String(item.appId))}</span>
    </div>
  `).join('');
  els.searchAutocomplete.classList.remove('hidden');
}

async function triggerAutocomplete(query) {
  showAutocompleteLoading();
  try {
    const results = await invoke('search_steam_games', { query });
    // Only render if the input still matches (user may have typed more)
    const currentVal = els.searchAppIdInput.value.trim();
    if (currentVal === query || (!isNumericInput(currentVal) && currentVal.length >= 2)) {
      renderAutocompleteResults(results);
    }
  } catch (err) {
    console.error('[Autocomplete]', err);
    hideAutocomplete();
  }
}

function onSearchInput() {
  const val = els.searchAppIdInput.value.trim();

  // Clear any pending debounce
  if (autocompleteDebounceTimer) {
    clearTimeout(autocompleteDebounceTimer);
    autocompleteDebounceTimer = null;
  }

  // If empty or numeric → no autocomplete
  if (!val || isNumericInput(val)) {
    hideAutocomplete();
    return;
  }

  // Need at least 2 chars for search
  if (val.length < 2) {
    hideAutocomplete();
    return;
  }

  // Debounce: 400ms
  autocompleteDebounceTimer = setTimeout(() => {
    triggerAutocomplete(val);
  }, 400);
}

// ============ App ID Search ============
async function performSearch() {
  hideAutocomplete();
  const appIdStr = els.searchAppIdInput.value.trim();
  if (!appIdStr) return;

  const appId = parseInt(appIdStr, 10);
  if (isNaN(appId) || appId <= 0) {
    showSearchError('Please enter a valid App ID');
    return;
  }

  // Reset previous results
  els.searchError.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  state.selectedRepo = null;
  state.searchRepos = [];
  state.searchAppId = appId;

  // Show loading
  els.searchLoading.classList.remove('hidden');
  els.btnSearch.disabled = true;

  emitEvent('search_performed');

  // Fetch game info in parallel
  fetchSearchGameInfo(appId);

  try {
    const raw = await invoke('search_repos', {
      appId: String(appId),
    });

    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;

    const repos = (raw.repos || []).map(r => ({
      name: r.repo,
      date: r.date,
      sha: r.sha,
      type: r.type || 'unknown',
      source: r.source || r.type || 'unknown'
    }));
    state.searchRepos = repos;

    if (repos.length === 0) {
      showSearchError('No manifests found for this App ID on the Internet Archive.');
      return;
    }

    renderRepoList(repos);
    els.searchResults.classList.remove('hidden');
  } catch (error) {
    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;
    showSearchError(String(error));
  }
}

function showSearchError(message) {
  els.searchError.textContent = message;
  els.searchError.classList.remove('hidden');
}

async function fetchSearchGameInfo(appId) {
  els.searchGameBanner.classList.add('hidden');

  try {
    const info = await invoke('get_steam_app_info', { appId: String(appId) });

    if (info) {
      const { name, headerImage, shortDescription } = info;

      if (headerImage) {
        els.searchGameImage.src = headerImage;
        els.searchGameImage.alt = name || 'Game Cover';
        state.headerImage = headerImage;
      }

      if (name) {
        els.searchGameName.textContent = name;
        state.gameName = name;
      }

      if (shortDescription) {
        els.searchGameDescription.textContent = shortDescription;
      }

      els.searchGameBanner.classList.remove('hidden');
    }
  } catch (e) {
    // Silently fail
  }
}

function renderRepoList(repos) {
  els.repoList.innerHTML = '';

  // Individual repo cards
  repos.forEach((repo, index) => {
    const card = document.createElement('div');
    card.className = 'repo-card';
    card.dataset.repoIndex = index;

    const dateHtml = repo.date ? `<div class="repo-card__date">Updated: ${formatRepoDate(repo.date)}</div>` : '';

    card.innerHTML = `
      <div class="repo-card__radio"></div>
      <div class="repo-card__info">
        <div class="repo-card__name">${escapeHtml(repo.name)}</div>
        ${dateHtml}
      </div>
      <span class="repo-card__badge repo-card__badge--archive">${escapeHtml(repo.source || repo.type || 'unknown')}</span>
    `;
    card.addEventListener('click', () => selectRepo(index));
    els.repoList.appendChild(card);
  });

  // Auto-select the first (and likely only) repo if there's exactly one
  if (repos.length === 1) {
    selectRepo(0);
  }

  // Auto-proceed for re-download from history
  if (autoRedownloadPending && repos.length > 0) {
    autoRedownloadPending = false;
    selectRepo(0);
    proceedFromSearch();
  }
}

function formatRepoDate(dateStr) {
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) +
      ' at ' + d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
  } catch {
    return dateStr;
  }
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function selectRepo(index) {
  // Deselect all
  $$('.repo-card').forEach(c => c.classList.remove('selected'));

  state.selectedRepo = state.searchRepos[index];

  // Highlight the selected card
  const card = els.repoList.querySelector(`[data-repo-index="${index}"]`);
  if (card) card.classList.add('selected');

  // Show Next button
  els.searchNextRow.classList.remove('hidden');
}

async function proceedFromSearch() {
  if (!state.selectedRepo) return;

  const repo = state.selectedRepo;
  const appId = state.searchAppId;

  // Hide next button, show manifest loading
  els.searchNextRow.classList.add('hidden');
  els.manifestLoading.classList.remove('hidden');
  els.searchError.classList.add('hidden');

  try {
    const mRaw = await invoke('get_repo_manifests', {
      appId: String(appId),
      repo: repo.name,
      sha: repo.sha || null,
    });

    const depots = (mRaw.manifests || []).map(m => ({
      depotId: String(m.depot_id),
      manifestId: m.manifest_id || 'N/A',
      depotKey: m.depot_key || null,
      sizeBytes: m.size_bytes || null
    }));

    state.searchRepo = repo.name;
    state.searchSha = repo.sha;
    state.searchKeyVdfKeys = mRaw.depot_keys || null;

    els.manifestLoading.classList.add('hidden');

    if (depots.length === 0) {
      showSearchError('No manifests found for this App ID on the Internet Archive.');
      els.searchNextRow.classList.remove('hidden');
      return;
    }

    // Build parsedData for Step 2
    state.parsedData = {
      mainAppId: appId,
      depots: depots
    };

    // Transition to Step 2
    showSelectionStep();
  } catch (error) {
    els.manifestLoading.classList.add('hidden');
    showSearchError(String(error));
    els.searchNextRow.classList.remove('hidden');
  }
}

// ============ Download Directory ============
async function loadSettingsAndDefaults() {
  try {
    const settings = await invoke('get_settings');
    defaultDownloadDir = settings.download_location || '';
    state.notificationSoundEnabled = settings.notification_sound !== false;
    if (els.downloadDirInput) {
      els.downloadDirInput.value = defaultDownloadDir;
    }
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function getDownloadDir() {
  const val = els.downloadDirInput ? els.downloadDirInput.value.trim() : '';
  return val || defaultDownloadDir;
}

async function saveDownloadDir() {
  const dir = getDownloadDir();
  if (dir) {
    try {
      const settings = await invoke('get_settings');
      settings.download_location = dir;
      await invoke('save_settings', { settings });
    } catch (e) {
      console.error('Failed to save download dir:', e);
    }
  }
}

async function browseDownloadDir() {
  try {
    const { open } = window.__TAURI__.dialog;
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Download Location'
    });
    if (selected) {
      els.downloadDirInput.value = selected;
    }
  } catch (e) {
    console.error('Failed to open folder dialog:', e);
  }
}

// ============ "Start Over" — go back to Step 2 with preserved selections ============
function goBackToSelect() {
  // Clean up progress listener
  cleanupProgressListener();
  state.jobId = null;
  // Go back to Step 2 (select) — parsedData and selectedDepots are still intact
  goToStep(2);
}

// ============ Game Info ============
async function fetchGameInfo(appId) {
  els.gameInfoBanner.classList.add('hidden');
  els.gameInfoLoading.classList.remove('hidden');

  try {
    const info = await invoke('get_steam_app_info', { appId: String(appId) });

    if (info) {
      const { name, headerImage, shortDescription } = info;

      if (headerImage) {
        els.gameHeaderImage.src = headerImage;
        els.gameHeaderImage.alt = name || 'Game Cover';
        state.headerImage = headerImage;
      }

      if (name) {
        els.gameName.textContent = name;
        state.gameName = name;
      }

      if (shortDescription) {
        els.gameDescription.textContent = shortDescription;
      }

      els.gameInfoLoading.classList.add('hidden');
      els.gameInfoBanner.classList.remove('hidden');
      return;
    }
  } catch (e) {
    // Silently fail - just hide the banner
  }

  els.gameInfoLoading.classList.add('hidden');
}

// ============ Depot Selection ============
function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return null;
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(2)} GB`;
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(2)} MB`;
  const kb = bytes / 1024;
  return `${kb.toFixed(2)} KB`;
}

function showSelectionStep() {
  const data = state.parsedData;
  if (!data) return;

  els.appIdDisplay.textContent = data.mainAppId;
  els.depotCount.textContent = `${data.depots.length} depot(s) found`;

  // Fetch game info from Steam (async, non-blocking) — only if not already fetched by search
  if (state.mode !== 'search' || !state.gameName) {
    fetchGameInfo(data.mainAppId);
  } else {
    // Copy search game info to step 2 banner
    if (state.headerImage) {
      els.gameHeaderImage.src = state.headerImage;
      els.gameHeaderImage.alt = state.gameName || 'Game Cover';
    }
    if (state.gameName) els.gameName.textContent = state.gameName;
    els.gameInfoLoading.classList.add('hidden');
    if (state.headerImage || state.gameName) {
      els.gameInfoBanner.classList.remove('hidden');
    }
  }

  // Restore saved API key from localStorage (MH key is still local)
  const savedApiKey = localStorage.getItem(MH_APIKEY_STORAGE_KEY);
  if (savedApiKey) els.mhApiKey.value = savedApiKey;

  // Restore download directory
  if (els.downloadDirInput && defaultDownloadDir) {
    els.downloadDirInput.value = els.downloadDirInput.value || defaultDownloadDir;
  }

  // Render depot list
  els.depotList.innerHTML = '';
  state.selectedDepots.clear();

  state.depotManifests = {};

  data.depots.forEach((depot) => {
    const sizeFormatted = formatBytes(depot.sizeBytes);
    const item = document.createElement('div');
    item.className = 'depot-item';
    item.dataset.depotId = depot.depotId;
    if (depot.sizeBytes) item.dataset.sizeBytes = depot.sizeBytes;
    const safeDepotId = escapeHtml(String(depot.depotId));
    const safeManifestId = escapeHtml(String(depot.manifestId || 'N/A'));
    const safeSize = sizeFormatted ? escapeHtml(sizeFormatted) : '';
    item.innerHTML = `
      <div class="depot-item__checkbox"></div>
      <div class="depot-item__info">
        <div class="depot-item__depot-id">Depot ${safeDepotId}${safeSize ? `<span class="depot-item__size">${safeSize}</span>` : ''}</div>
        <div class="depot-item__manifest-id">Manifest: ${safeManifestId}</div>
        <div class="depot-item__custom-manifest">
          <label>Custom:</label>
          <input type="text" data-depot-id="${safeDepotId}" class="custom-manifest-input"
            placeholder="Custom manifest ID (optional)"
            onclick="event.stopPropagation()">
        </div>
        <div class="depot-item__manifest-upload">
          <button type="button" class="btn btn--small btn--outline depot-manifest-btn" data-depot-id="${safeDepotId}">
            📁 Upload .manifest
          </button>
          <span class="depot-manifest-status" data-depot-id="${safeDepotId}"></span>
        </div>
      </div>
    `;

    // Attach manifest upload button handler
    const manifestBtn = item.querySelector('.depot-manifest-btn');
    if (manifestBtn) {
      manifestBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        handleDepotManifestFile(depot.depotId);
      });
    }

    item.addEventListener('click', (e) => {
      // Don't toggle when clicking input or upload button
      if (e.target.tagName === 'INPUT') return;
      if (e.target.tagName === 'BUTTON' || e.target.closest('.depot-manifest-btn')) return;
      toggleDepot(depot.depotId, item);
    });
    els.depotList.appendChild(item);
  });

  // Reset depot filters
  if (els.depotSearch) els.depotSearch.value = '';
  if (els.showSelectedOnly) els.showSelectedOnly.checked = false;

  // Auto-select depots for re-download from history
  if (autoSelectAllOnStep2) {
    autoSelectAllOnStep2 = false;
    if (autoSelectDepotIds && autoSelectDepotIds.length > 0) {
      document.querySelectorAll('.depot-item').forEach(item => {
        const depotId = item.dataset.depotId;
        if (autoSelectDepotIds.includes(depotId)) {
          state.selectedDepots.add(depotId);
          item.classList.add('selected');
        }
      });
      autoSelectDepotIds = null;
    } else {
      selectAll();
    }
  }

  updateDownloadButton();
  goToStep(2);
}

function toggleDepot(depotId, element) {
  if (state.selectedDepots.has(depotId)) {
    state.selectedDepots.delete(depotId);
    element.classList.remove('selected');
  } else {
    state.selectedDepots.add(depotId);
    element.classList.add('selected');
  }
  updateDownloadButton();
}

function selectAll() {
  state.parsedData.depots.forEach((depot) => {
    state.selectedDepots.add(depot.depotId);
  });
  $$('.depot-item').forEach((el) => {
    el.classList.add('selected');
  });
  updateDownloadButton();
}

function deselectAll() {
  state.selectedDepots.clear();
  $$('.depot-item').forEach((el) => el.classList.remove('selected'));
  updateDownloadButton();
}

function updateDownloadButton() {
  const count = state.selectedDepots.size;
  els.btnDownload.disabled = count === 0;

  // Calculate total size of selected depots
  let totalBytes = 0;
  let hasSizeInfo = false;
  if (state.parsedData && state.parsedData.depots) {
    for (const depot of state.parsedData.depots) {
      if (state.selectedDepots.has(depot.depotId) && depot.sizeBytes) {
        totalBytes += depot.sizeBytes;
        hasSizeInfo = true;
      }
    }
  }
  const sizeLabel = hasSizeInfo ? ` — ~${formatBytes(totalBytes)} total` : '';

  els.btnDownload.innerHTML = `
    Download${count > 0 ? ` (${count})${sizeLabel}` : ''}
    <svg class="btn__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
      <polyline points="7 10 12 15 17 10"/>
      <line x1="12" y1="15" x2="12" y2="3"/>
    </svg>
  `;
}

// ============ Download Process ============
async function startDownload() {
  const data = state.parsedData;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));

  if (selectedDepots.length === 0) return;

  emitEvent('download_started', { depot_count: selectedDepots.length });

  // Request notification permission on first download
  requestNotificationPermission();

  // Get and save settings
  const mhApiKey = els.mhApiKey.value.trim();
  if (mhApiKey) localStorage.setItem(MH_APIKEY_STORAGE_KEY, mhApiKey);
  saveDownloadDir();

  // Collect custom manifest IDs and uploaded manifest files from inputs
  const depotsWithCustomManifests = selectedDepots.map(depot => {
    const input = document.querySelector(`.custom-manifest-input[data-depot-id="${depot.depotId}"]`);
    const customManifestId = input ? input.value.trim() : '';
    const depotManifest = state.depotManifests[depot.depotId];
    const result = {
      ...depot,
      customManifestId: customManifestId || null
    };
    if (depotManifest) {
      result.uploadedManifestPath = depotManifest.storedPath;
      // If no custom manifest ID typed, try to extract from filename
      if (!result.customManifestId) {
        const nameMatch = depotManifest.originalName.match(/^(\d+)_(\d+)\.manifest$/);
        if (nameMatch) {
          result.customManifestId = nameMatch[2];
        }
      }
    }
    return result;
  });

  // Go to progress step
  goToStep(3);
  initProgressUI(depotsWithCustomManifests);

  try {
    // Build download config
    const downloadConfig = {
      mainAppId: String(data.mainAppId),
      selectedDepots: depotsWithCustomManifests,
      manifestHubApiKey: mhApiKey || null,
      downloadDir: getDownloadDir() || null,
      gameName: state.gameName || null,
      headerImage: state.headerImage || null
    };

    // Add search-mode specific fields
    if (state.mode === 'search') {
      if (state.searchRepo) downloadConfig.repo = state.searchRepo;
      if (state.searchSha) downloadConfig.sha = state.searchSha;
      if (state.searchKeyVdfKeys) downloadConfig.keyVdfKeys = state.searchKeyVdfKeys;
    }

    // Start download via Tauri invoke
    const result = await invoke('start_download', { config: downloadConfig });

    state.jobId = result.jobId;
    state.downloadDir = result.downloadDir;

    // Listen for progress events (replaces WebSocket)
    connectProgressListener();
  } catch (error) {
    appendTerminalLine(`Error: ${error}`, 'error');
    showCompletion(false, String(error));
  }
}

function initProgressUI(depots) {
  // Reset progress
  els.progressBarFill.style.width = '0%';
  els.progressStatus.textContent = 'Initializing...';
  els.terminalOutput.innerHTML = '';
  els.completionMessage.classList.add('hidden');
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  els.btnCancel.classList.remove('hidden');
  els.btnCancel.disabled = false;
  els.btnCancel.innerHTML = '✕ Cancel Download';
  els.diskSpaceInfo.classList.add('hidden');
  // Reset depot download progress bar
  if (els.depotProgressFill) els.depotProgressFill.style.width = '0%';
  if (els.depotProgressText) els.depotProgressText.textContent = '0%';
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;

  // Build depot progress items
  els.depotProgressList.innerHTML = '';
  depots.forEach((depot) => {
    const item = document.createElement('div');
    item.className = 'depot-progress-item';
    item.id = `depot-progress-${depot.depotId}`;
    item.innerHTML = `
      <div class="depot-progress-item__icon depot-progress-item__icon--pending">●</div>
      <div class="depot-progress-item__label">Depot ${escapeHtml(String(depot.depotId))}</div>
      <div class="depot-progress-item__status">Waiting...</div>
    `;
    els.depotProgressList.appendChild(item);
  });
}

// ============ Tauri Progress Events (replaces WebSocket) ============
async function connectProgressListener() {
  // Clean up any previous listener
  if (state.unlistenProgress) {
    state.unlistenProgress();
    state.unlistenProgress = null;
  }

  const unlisten = await listen('download-progress', (event) => {
    handleProgressMessage(event.payload);
  });
  state.unlistenProgress = unlisten;
  appendTerminalLine('Connected to download engine...', 'info');
}

function cleanupProgressListener() {
  if (state.unlistenProgress) {
    state.unlistenProgress();
    state.unlistenProgress = null;
  }
}

function handleProgressMessage(msg) {
  switch (msg.type) {
    case 'status':
      if (msg.step === 'disk_space') {
        showDiskSpace(msg.freeGB, msg.drive);
      } else {
        handleStatusUpdate(msg);
      }
      break;

    case 'output':
      handleOutput(msg);
      break;

    case 'depot_complete':
      updateDepotStatus(msg.depotId, 'done', 'Complete');
      updateOverallProgress(msg.current, msg.total);
      // Reset depot progress bar for next depot
      updateDepotDownloadProgress(100);
      break;

    case 'complete':
      handleComplete(msg);
      break;

    case 'error':
      handleError(msg);
      break;

    case 'cancelled':
      handleCancelled(msg);
      break;
  }
}

function handleStatusUpdate(msg) {
  switch (msg.step) {
    case 'checking_branch':
      els.progressStatus.textContent = `Checking GitHub branch for App ${msg.appId}...`;
      appendTerminalLine(`Checking branch for App ${msg.appId}...`, 'info');
      break;

    case 'branch_found':
      appendTerminalLine(`✓ Branch found. Last updated: ${msg.lastUpdated || 'unknown'}`, 'success');
      break;

    case 'downloading_manifests':
      els.progressStatus.textContent = `Downloading manifests (0/${msg.total})...`;
      break;

    case 'downloading_manifest':
      if (msg.current && msg.total) {
        els.progressStatus.textContent = `Downloading manifest ${msg.current}/${msg.total} (Depot ${msg.depotId})...`;
        updateOverallProgress(msg.current - 1, msg.total * 2);
      }
      updateDepotStatus(msg.depotId, 'active', 'Downloading manifest...');
      if (msg.filename) {
        appendTerminalLine(`Downloading ${msg.filename}...`, 'info');
      }
      break;

    case 'downloading_manifest_hub':
      els.progressStatus.textContent = `Downloading custom manifest for Depot ${msg.depotId} via ManifestHub API...`;
      updateDepotStatus(msg.depotId, 'active', `Custom manifest: ${msg.manifestId}`);
      appendTerminalLine(`Downloading custom manifest for depot ${msg.depotId} (ID: ${msg.manifestId}) via ManifestHub API...`, 'info');
      break;

    case 'generating_keys':
      els.progressStatus.textContent = 'Generating depot keys file...';
      appendTerminalLine('Generating steam.keys file...', 'info');
      break;

    case 'keys_generated':
      appendTerminalLine(`✓ Generated keys for ${msg.depotCount} depots`, 'success');
      break;

    case 'starting_downloader':
      els.progressStatus.textContent = `Running DepotDownloader (0/${msg.total})...`;
      break;

    case 'running_downloader':
      // Reset speed tracker for new depot
      state.speedTracker.samples = [];
      state.speedTracker.lastTime = 0;
      state.speedTracker.lastPercent = 0;
      state.speedTracker.depotStartTime = Date.now();
      state.speedTracker.lastUpdateTime = Date.now();
      // Stop previous stale timer and start new one
      clearInterval(state.speedTracker.staleTimer);
      state.speedTracker.staleTimer = null;
      startStaleTimer();
      // Set current depot size if available
      if (msg.depotId) {
        const depot = state.parsedData?.depots?.find(d => d.depotId === String(msg.depotId));
        state.speedTracker.currentDepotSize = depot?.sizeBytes || 0;
        state.speedTracker.currentDepotId = msg.depotId;
      }
      if (msg.current && msg.total) {
        els.progressStatus.textContent = `Running DepotDownloader ${msg.current}/${msg.total} (Depot ${msg.depotId})...`;
        const baseProgress = state.parsedData ? state.selectedDepots.size : 0;
        updateOverallProgress(baseProgress + msg.current - 1, baseProgress + msg.total);
      }
      updateDepotStatus(msg.depotId, 'active', 'Downloading...');
      if (msg.command) {
        appendTerminalLine(`> ${msg.command}`, 'info');
      }
      break;
  }
}

function handleOutput(msg) {
  const cls = msg.stream === 'stderr' ? 'stderr' : 'stdout';
  // Tauri events use 'output' field instead of 'line'
  const text = msg.output || msg.line;
  if (text) {
    appendTerminalLine(text, cls);
    // Parse depot download percentage from output (e.g. "01.83% depots\...")
    const percentMatch = text.match(/^\s*(\d{1,3}(?:\.\d{1,2})?)%/);
    if (percentMatch) {
      const percent = parseFloat(percentMatch[1]);
      updateDepotDownloadProgress(percent);
    }
  }
}

function updateDepotDownloadProgress(percent) {
  if (els.depotProgressFill) {
    els.depotProgressFill.style.width = `${Math.min(percent, 100)}%`;
  }
  if (els.depotProgressText) {
    els.depotProgressText.textContent = `${percent.toFixed(1)}%`;
  }
  updateSpeedAndEta(percent);
}

function updateSpeedAndEta(percent) {
  const now = Date.now();
  const tracker = state.speedTracker;

  // Record last update time for stale detection
  tracker.lastUpdateTime = now;

  // Initialize on first call
  if (tracker.lastTime === 0) {
    tracker.lastTime = now;
    tracker.lastPercent = percent;
    return;
  }

  // Add sample
  tracker.samples.push({ percent, time: now });

  // Keep only last 10 samples (gleitender Durchschnitt)
  if (tracker.samples.length > 10) {
    tracker.samples.shift();
  }

  // Need at least 2 samples
  if (tracker.samples.length < 2) return;

  // Calculate rate from oldest to newest sample
  const oldest = tracker.samples[0];
  const newest = tracker.samples[tracker.samples.length - 1];
  const timeDelta = (newest.time - oldest.time) / 1000; // seconds
  const percentDelta = newest.percent - oldest.percent;

  if (timeDelta <= 0 || percentDelta <= 0) return;

  const percentPerSecond = percentDelta / timeDelta;
  const remainingPercent = 100 - percent;
  const etaSeconds = remainingPercent / percentPerSecond;

  // Calculate speed in MB/s if we know the depot size
  let speedText = '';
  if (tracker.currentDepotSize > 0) {
    const bytesPerSecond = (tracker.currentDepotSize * percentDelta / 100) / timeDelta;
    const mbPerSecond = bytesPerSecond / (1024 * 1024);
    speedText = `↓ ${mbPerSecond.toFixed(1)} MB/s`;
  } else {
    speedText = `↓ ${percentPerSecond.toFixed(2)}%/s`;
  }

  // Format ETA
  const etaText = formatEta(etaSeconds);

  // Show the info
  const infoEl = els.downloadSpeedInfo;
  if (infoEl) {
    infoEl.classList.remove('hidden');
    els.downloadSpeed.textContent = speedText;
    els.downloadEta.textContent = etaText;
  }
}

function formatEta(seconds) {
  if (!isFinite(seconds) || seconds < 0 || seconds > 86400) {
    return 'calculating...';
  }

  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) {
    return `~${h}h ${m}m remaining`;
  } else if (m > 0) {
    return `~${m}m ${s}s remaining`;
  } else {
    return `~${s}s remaining`;
  }
}

function startStaleTimer() {
  if (state.speedTracker.staleTimer) return; // Already running

  state.speedTracker.staleTimer = setInterval(() => {
    const now = Date.now();
    const timeSinceLastUpdate = (now - state.speedTracker.lastUpdateTime) / 1000;

    if (timeSinceLastUpdate > 5) {
      // Switch to elapsed time mode — show time since last progress update
      const waitingFor = (now - state.speedTracker.lastUpdateTime) / 1000;
      const infoEl = els.downloadSpeedInfo;
      if (infoEl) {
        infoEl.classList.remove('hidden');
        els.downloadSpeed.textContent = '⏳ Downloading large file...';
        els.downloadEta.textContent = `waiting ${formatElapsed(waitingFor)}`;
      }
    }
  }, 1000);
}

function formatElapsed(seconds) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) return `${h}h ${m}m ${s}s`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function handleComplete(msg) {
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;
  els.progressBarFill.style.width = '100%';
  els.progressStatus.textContent = '✅ Complete!';
  updateDepotDownloadProgress(100);
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  emitEvent('download_completed', { success: true });
  showCompletion(true, msg.message);

  // Mark remaining depots as done
  if (msg.results) {
    const results = Array.isArray(msg.results) ? msg.results : [];
    results.forEach((r) => {
      updateDepotStatus(r.depotId, r.success ? 'done' : 'error', r.success ? 'Complete' : 'Failed');
    });
  }

  appendTerminalLine(`\n${msg.message}`, 'success');

  // Browser notification + sound
  const gameName = state.gameName || 'Game';
  showBrowserNotification('Download Complete!', `${gameName} has been downloaded successfully.`, state.headerImage);
  playNotificationSound();

  cleanupProgressListener();
}

function handleError(msg) {
  if (msg.depotId) {
    updateDepotStatus(msg.depotId, 'error', 'Error');
  }
  appendTerminalLine(`Error: ${msg.message}`, 'error');

  // If it's a fatal error (no depotId = pipeline-level error), show Start Over and notify
  if (!msg.depotId) {
    clearInterval(state.speedTracker.staleTimer);
    state.speedTracker.staleTimer = null;
    if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
    emitEvent('download_completed', { success: false });
    showCompletion(false, msg.message);
    showBrowserNotification('Download Failed!', `Error: ${msg.message}`);
    playNotificationSound();
    cleanupProgressListener();
  }
}

function handleCancelled(msg) {
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;
  els.progressBarFill.style.width = '0%';
  els.progressStatus.textContent = 'Cancelled';
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  appendTerminalLine(`\n${msg.message}`, 'error');
  showCompletion(false, msg.message);
  cleanupProgressListener();
}

// ============ UI Helpers ============
function updateDepotStatus(depotId, status, text) {
  const item = document.getElementById(`depot-progress-${depotId}`);
  if (!item) return;

  const icon = item.querySelector('.depot-progress-item__icon');
  const statusEl = item.querySelector('.depot-progress-item__status');

  // Remove all status classes
  icon.className = 'depot-progress-item__icon';

  switch (status) {
    case 'active':
      icon.classList.add('depot-progress-item__icon--active');
      icon.textContent = '◉';
      break;
    case 'done':
      icon.classList.add('depot-progress-item__icon--done');
      icon.textContent = '✓';
      break;
    case 'error':
      icon.classList.add('depot-progress-item__icon--error');
      icon.textContent = '✗';
      break;
    default:
      icon.classList.add('depot-progress-item__icon--pending');
      icon.textContent = '●';
  }

  statusEl.textContent = text;
}

function updateOverallProgress(current, total) {
  if (total <= 0) return;
  const percent = Math.min(Math.round((current / total) * 100), 99);
  els.progressBarFill.style.width = `${percent}%`;
}

function appendTerminalLine(text, type = 'stdout') {
  const line = document.createElement('div');
  line.className = `terminal__line--${type}`;
  line.textContent = text;
  els.terminalOutput.appendChild(line);
  els.terminalOutput.scrollTop = els.terminalOutput.scrollHeight;
}

function showCompletion(success, message) {
  els.completionMessage.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  els.completionMessage.classList.add(success ? 'completion-message--success' : 'completion-message--error');
  els.completionMessage.textContent = message;
  els.btnCancel.classList.add('hidden');
  // Show Next button on success (goes to Step 4 if shortcuts supported, otherwise acts as reset)
  if (els.btnNextStep) {
    els.btnNextStep.classList.toggle('hidden', !success);
    els.btnNextStep.textContent = state.shortcutSupported ? 'Next' : 'Start New Download';
  }
}

function resetApp() {
  state.parsedData = null;
  state.selectedDepots.clear();
  state.jobId = null;
  state.gameName = null;
  state.headerImage = null;
  state.downloadDir = null;
  state.depotManifests = {};
  state.searchRepos = [];
  state.selectedRepo = null;
  state.searchAppId = null;
  state.searchSha = null;
  state.searchKeyVdfKeys = null;
  cleanupProgressListener();
  // Reset shortcut UI
  if (els.shortcutStatus) els.shortcutStatus.classList.add('hidden');
  if (els.shortcutDetectedSection) els.shortcutDetectedSection.classList.add('hidden');
  if (els.shortcutDetectedList) els.shortcutDetectedList.innerHTML = '';
  if (els.shortcutExePath) els.shortcutExePath.value = '';
  if (els.btnCreateShortcuts) { els.btnCreateShortcuts.disabled = false; els.btnCreateShortcuts.textContent = 'Create Shortcuts'; }
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  // Reset game info banner
  els.gameInfoBanner.classList.add('hidden');
  els.gameInfoLoading.classList.add('hidden');
  // Reset search UI
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchError.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  els.manifestLoading.classList.add('hidden');
  resetUpload();
  goToStep(1);
}

// ============ Settings Modal ============
async function openSettings() {
  // Load fresh settings from backend
  try {
    const settings = await invoke('get_settings');
    els.autoUpdateToggle.checked = settings.auto_update !== false;

    // Advanced settings
    els.ddExtraArgsInput.value = (settings.dd_extra_args || []).join(' ');
    els.maxRetriesInput.value = settings.max_retries ?? 3;
    els.speedLimitInput.value = settings.download_speed_limit || '';
    els.proxyInput.value = settings.proxy || '';
    els.notificationSoundToggle.checked = settings.notification_sound !== false;
    els.telemetryToggle.checked = settings.telemetry_consent === 'accepted';
  } catch (e) {
    els.autoUpdateToggle.checked = true;
  }
  loadBuildInfo();
  emitEvent('settings_opened');
  els.settingsModal.classList.remove('hidden');
}

function channelLabel(channel) {
  switch (channel) {
    case 'stable': return { text: 'Stable', cls: 'build-info__badge--stable' };
    case 'dev':    return { text: 'Dev',    cls: 'build-info__badge--dev' };
    case 'dev-local': return { text: 'Dev (local)', cls: 'build-info__badge--local' };
    default:       return { text: channel || '—', cls: 'build-info__badge--local' };
  }
}

async function loadBuildInfo() {
  try {
    const info = await invoke('get_build_info');
    state.buildInfo = info;

    const { text, cls } = channelLabel(info.channel);
    els.buildInfoChannel.textContent = text;
    els.buildInfoChannel.className = `build-info__badge ${cls}`;

    els.buildInfoVersion.textContent = info.version || '—';
    els.buildInfoSha.textContent = info.gitSha || 'unknown';
    els.buildInfoDate.textContent = info.buildDate || 'unknown';
    els.buildInfoProfile.textContent = info.profile || '—';
    els.buildInfoPlatform.textContent = `${info.targetOs || '?'} / ${info.targetArch || '?'}`;
  } catch (e) {
    console.error('Failed to load build info:', e);
  }
}

// ============ Telemetry ============
async function initTelemetryConsent() {
  try {
    const status = await invoke('get_telemetry_status');
    if (status.consent === 'pending') {
      els.telemetryModal.classList.remove('hidden');
    } else if (status.consent === 'accepted') {
      invoke('emit_telemetry_event', { kind: 'app_start' }).catch(() => {});
    }
  } catch (e) {
    console.error('Failed to load telemetry status:', e);
  }
}

async function acceptTelemetry() {
  try {
    await invoke('set_telemetry_consent', { accept: true });
    invoke('emit_telemetry_event', { kind: 'app_start' }).catch(() => {});
  } catch (e) {
    console.error('Failed to accept telemetry:', e);
  }
  els.telemetryModal.classList.add('hidden');
}

async function declineTelemetry() {
  try {
    await invoke('set_telemetry_consent', { accept: false });
  } catch (e) {
    console.error('Failed to decline telemetry:', e);
  }
  els.telemetryModal.classList.add('hidden');
}

async function onTelemetryToggleChanged() {
  const accept = els.telemetryToggle.checked;
  try {
    await invoke('set_telemetry_consent', { accept });
  } catch (e) {
    console.error('Failed to update telemetry consent:', e);
    els.telemetryToggle.checked = !accept;
  }
}

function emitEvent(kind, props) {
  invoke('emit_telemetry_event', { kind, props: props ?? null }).catch(() => {});
}

async function copyBuildInfo() {
  const info = state.buildInfo;
  if (!info) return;
  const text = [
    `Channel: ${info.channel}`,
    `Version: ${info.version}`,
    `Commit: ${info.gitSha}`,
    `Build date: ${info.buildDate}`,
    `Profile: ${info.profile}`,
    `Platform: ${info.targetOs}/${info.targetArch}`,
  ].join('\n');

  try {
    await navigator.clipboard.writeText(text);
    const btn = els.btnCopyBuildInfo;
    const original = btn.textContent;
    btn.textContent = 'Copied!';
    setTimeout(() => { btn.textContent = original; }, 1500);
  } catch (e) {
    console.error('Clipboard write failed:', e);
  }
}

function closeSettings() {
  els.settingsModal.classList.add('hidden');
}

async function saveSettings() {
  const autoUpdate = els.autoUpdateToggle.checked;
  try {
    const currentSettings = await invoke('get_settings');
    currentSettings.auto_update = autoUpdate;

    // Advanced settings
    const argsStr = els.ddExtraArgsInput.value.trim();
    if (argsStr) {
      currentSettings.dd_extra_args = argsStr.split(/\s+/).filter(a => a.length > 0);
    } else {
      currentSettings.dd_extra_args = ["-max-downloads", "8", "-verify-all"];
    }
    currentSettings.max_retries = parseInt(els.maxRetriesInput.value) || 3;
    currentSettings.download_speed_limit = els.speedLimitInput.value.trim();
    currentSettings.proxy = els.proxyInput.value.trim();
    currentSettings.notification_sound = els.notificationSoundToggle.checked;

    await invoke('save_settings', { settings: currentSettings });
    state.notificationSoundEnabled = currentSettings.notification_sound;
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
  closeSettings();
}

function toggleAdvancedSettings() {
  const content = els.advancedSettingsContent;
  const arrow = els.btnToggleAdvanced.querySelector('.settings-advanced__arrow');
  const isHidden = content.classList.contains('hidden');

  if (isHidden) {
    content.classList.remove('hidden');
    arrow.classList.add('expanded');
  } else {
    content.classList.add('hidden');
    arrow.classList.remove('expanded');
  }
}

// ============ Auto-Update ============
const SKIPPED_VERSION_KEY = 'skippedUpdateVersion';

async function checkForUpdates() {
  try {
    const enabled = await invoke('get_auto_update_enabled');
    if (!enabled) return;

    const result = await invoke('check_for_updates');

    emitEvent('update_checked', { available: !!result.available });

    if (result.error) {
      console.error('[AutoUpdate] Error:', result.error);
    }
    if (!result.available) return;

    // Check if user has skipped this version
    const skipped = localStorage.getItem(SKIPPED_VERSION_KEY);
    if (skipped === result.version) return;

    showUpdateModal(result);
  } catch (e) {
    console.error('[AutoUpdate] Check failed:', e);
  }
}

/** Simple Markdown → HTML renderer for release notes */
function renderMarkdown(md) {
  // Escape HTML
  let html = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  // Headings
  html = html.replace(/^### (.+)$/gm, '<h4>$1</h4>');
  html = html.replace(/^## (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^# (.+)$/gm, '<h2>$1</h2>');
  // Bold + Italic
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  // Unordered list items
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>');
  // Line breaks (remaining)
  html = html.replace(/\n/g, '<br>');
  // Clean up double <br> after block elements
  html = html.replace(/<\/(h[234]|ul|li)><br>/g, '</$1>');
  return html;
}

function showUpdateModal(info) {
  pendingUpdateInfo = info;
  els.updateVersion.textContent = `v${info.version}`;
  if (info.date) {
    try {
      const d = new Date(info.date);
      els.updateDate.textContent = isNaN(d.getTime()) ? info.date : d.toLocaleDateString();
    } catch { els.updateDate.textContent = info.date; }
    els.updateDateRow.style.display = '';
  } else {
    els.updateDateRow.style.display = 'none';
  }
  els.updateNotes.innerHTML = info.body
    ? renderMarkdown(info.body)
    : '<em>No release notes available.</em>';
  els.updateProgressWrap.classList.add('hidden');
  els.updateActions.style.display = '';
  els.btnUpdateNow.disabled = false;
  els.updateModal.classList.remove('hidden');
}

function hideUpdateModal() {
  els.updateModal.classList.add('hidden');
}

let pendingUpdateInfo = null;

async function performUpdate() {
  if (!pendingUpdateInfo || !pendingUpdateInfo.installerUrl) {
    // No direct installer — open release page in browser
    if (pendingUpdateInfo && pendingUpdateInfo.releaseUrl) {
      window.__TAURI__.shell.open(pendingUpdateInfo.releaseUrl);
    }
    hideUpdateModal();
    return;
  }

  els.btnUpdateNow.disabled = true;
  els.btnUpdateLater.style.display = 'none';
  els.btnUpdateSkip.style.display = 'none';
  els.btnUpdateNow.textContent = 'Downloading...';
  els.updateProgressWrap.classList.remove('hidden');
  els.updateProgressText.textContent = 'Downloading update installer...';
  els.updateProgressFill.style.width = '100%';
  els.updateProgressFill.classList.add('progress-bar__fill--indeterminate');

  emitEvent('update_installed');
  try {
    await invoke('install_update', { installerUrl: pendingUpdateInfo.installerUrl });
    // App will exit — this line may not be reached
  } catch (e) {
    console.error('[AutoUpdate] Install failed:', e);
    els.updateProgressText.textContent = `Update failed: ${e}`;
    els.updateProgressFill.classList.remove('progress-bar__fill--indeterminate');
    els.updateProgressFill.style.width = '0%';
    els.btnUpdateNow.textContent = 'Retry';
    els.btnUpdateNow.disabled = false;
    els.btnUpdateLater.style.display = '';
  }
}

function skipUpdateVersion() {
  const version = els.updateVersion.textContent.replace(/^v/, '');
  localStorage.setItem(SKIPPED_VERSION_KEY, version);
  hideUpdateModal();
}

// DEV: Test function — call window.testUpdateModal() in browser console
window.testUpdateModal = function() {
  showUpdateModal({
    available: true,
    version: '2.0.0',
    currentVersion: '1.1.0',
    date: new Date().toISOString(),
    body: '### What\'s New\n- ✨ Auto-Update feature\n- 🔧 Bug fixes\n- 🚀 Performance improvements\n\nThis is a **test** update dialog.'
  });
};

// ============ Depot Search/Filter ============
function applyDepotFilters() {
  const searchText = (els.depotSearch ? els.depotSearch.value.trim() : '');
  const showSelectedOnly = els.showSelectedOnly ? els.showSelectedOnly.checked : false;

  const items = document.querySelectorAll('.depot-item');
  items.forEach(item => {
    const depotId = item.dataset.depotId || '';
    const matchesSearch = !searchText || depotId.includes(searchText);
    const matchesSelected = !showSelectedOnly || state.selectedDepots.has(depotId);
    item.style.display = (matchesSearch && matchesSelected) ? '' : 'none';
  });
}

// ============ Theme Toggle ============
function initTheme() {
  const saved = localStorage.getItem('theme') || 'dark';
  document.documentElement.setAttribute('data-theme', saved);
  updateThemeButton(saved);
}

function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme') || 'dark';
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', next);
  localStorage.setItem('theme', next);
  updateThemeButton(next);
  emitEvent('theme_toggled', { to: next });
}

function updateThemeButton(theme) {
  if (els.btnThemeToggle) {
    els.btnThemeToggle.textContent = theme === 'dark' ? '🌙' : '☀️';
    els.btnThemeToggle.title = theme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';
  }
}

// ============ Cancel Download ============
function showCancelModal() {
  els.cancelModal.classList.remove('hidden');
}

function hideCancelModal() {
  els.cancelModal.classList.add('hidden');
}

async function cancelDownload() {
  hideCancelModal();

  if (!state.jobId) return;

  els.btnCancel.disabled = true;
  els.btnCancel.innerHTML = 'Cancelling...';
  appendTerminalLine('Cancelling download...', 'info');

  try {
    await invoke('cancel_download', { jobId: state.jobId });
  } catch (error) {
    const errStr = String(error);
    // If job is not running, the download already finished or errored — show Start Over
    if (errStr.toLowerCase().includes('not found') || errStr.toLowerCase().includes('not running')) {
      appendTerminalLine('Job is no longer running.', 'info');
      showCompletion(false, 'Download ended. You can start over.');
    } else {
      appendTerminalLine(`Cancel request failed: ${errStr}`, 'error');
      // Still show Next so user isn't stuck
      if (els.btnNextStep) els.btnNextStep.classList.remove('hidden');
      els.btnCancel.classList.add('hidden');
    }
  }
}

// ============ Disk Space ============
function showDiskSpace(freeGB, drive) {
  els.diskSpaceInfo.classList.remove('hidden', 'disk-space-info--warning', 'disk-space-info--danger');

  if (freeGB < 2) {
    els.diskSpaceInfo.classList.add('disk-space-info--danger');
    els.diskSpaceText.textContent = `Free disk space: ${freeGB} GB on ${drive} — CRITICALLY LOW!`;
  } else if (freeGB < 10) {
    els.diskSpaceInfo.classList.add('disk-space-info--warning');
    els.diskSpaceText.textContent = `Free disk space: ${freeGB} GB on ${drive} — Low space warning`;
  } else {
    els.diskSpaceText.textContent = `Free disk space: ${freeGB} GB on ${drive}`;
  }
}

// ============ Browser Notifications ============
function requestNotificationPermission() {
  if (!('Notification' in window)) return;
  if (Notification.permission === 'default') {
    Notification.requestPermission().then(perm => {
      state.notificationsEnabled = perm === 'granted';
    });
  } else {
    state.notificationsEnabled = Notification.permission === 'granted';
  }
}

function showBrowserNotification(title, body, icon) {
  if (!('Notification' in window)) return;
  if (Notification.permission !== 'granted') return;
  if (!document.hidden) return; // Only show when tab is not focused

  try {
    new Notification(title, {
      body,
      icon: icon || undefined
    });
  } catch (e) {
    // Fallback: ignore errors (e.g. service worker requirement)
  }
}

function playNotificationSound() {
  if (!state.notificationSoundEnabled) return;
  try {
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    oscillator.connect(gainNode);
    gainNode.connect(ctx.destination);
    oscillator.frequency.value = 800;
    oscillator.type = 'sine';
    gainNode.gain.setValueAtTime(0.3, ctx.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.5);
    oscillator.start(ctx.currentTime);
    oscillator.stop(ctx.currentTime + 0.5);
  } catch (e) {
    // Audio not available
  }
}

// ============ .NET Check ============
async function checkDotNet() {
  try {
    // Skip if user already dismissed the warning this session
    if (sessionStorage.getItem('dotnetWarningDismissed') === 'true') return;

    const result = await invoke('check_dotnet');
    if (!result.installed) {
      console.warn('.NET 9 runtime not found. DepotDownloader requires .NET 9.');
      showDotNetWarning();
    }
  } catch (e) {
    console.error('Failed to check .NET:', e);
  }
}

function showDotNetWarning() {
  const banner = document.getElementById('dotnet-warning');
  if (!banner) return;
  banner.classList.remove('hidden');

  // Dismiss button
  const dismissBtn = document.getElementById('dotnet-warning-dismiss');
  if (dismissBtn) {
    dismissBtn.addEventListener('click', () => {
      banner.classList.add('hidden');
      // Remember dismissal for this session
      sessionStorage.setItem('dotnetWarningDismissed', 'true');
    });
  }

  // Open link in external browser via Tauri shell
  const installLink = document.getElementById('dotnet-install-link');
  if (installLink) {
    installLink.addEventListener('click', (e) => {
      e.preventDefault();
      try {
        window.__TAURI__.shell.open('https://dotnet.microsoft.com/en-us/download/dotnet/9.0');
      } catch {
        // Fallback: just let the link work normally
        window.open('https://dotnet.microsoft.com/en-us/download/dotnet/9.0', '_blank');
      }
    });
  }
}

// ============ Event Listeners ============
function initEvents() {
  // Tabs
  els.tabUpload.addEventListener('click', () => switchTab('upload'));
  els.tabSearch.addEventListener('click', () => switchTab('search'));

  // Search
  els.btnSearch.addEventListener('click', performSearch);
  els.searchAppIdInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      hideAutocomplete();
      performSearch();
    }
    if (e.key === 'Escape') {
      hideAutocomplete();
    }
  });
  els.searchAppIdInput.addEventListener('input', onSearchInput);
  // Close autocomplete on click outside
  document.addEventListener('click', (e) => {
    if (!els.searchAppIdInput.contains(e.target) && !els.searchAutocomplete.contains(e.target)) {
      hideAutocomplete();
    }
  });
  // Delegated autocomplete-item clicks so each re-render doesn't stack listeners
  els.searchAutocomplete.addEventListener('click', (e) => {
    const item = e.target.closest('.search-autocomplete__item');
    if (!item) return;
    const appId = item.dataset.appid;
    if (!appId) return;
    els.searchAppIdInput.value = appId;
    hideAutocomplete();
    performSearch();
  });
  els.btnSearchNext.addEventListener('click', proceedFromSearch);

  // Select
  els.btnSelectAll.addEventListener('click', selectAll);
  els.btnDeselectAll.addEventListener('click', deselectAll);
  els.btnBack.addEventListener('click', () => goToStep(1));
  els.btnDownload.addEventListener('click', startDownload);
  // btnNew and btnStartOver moved to Step 4
  els.btnCancel.addEventListener('click', showCancelModal);
  // Browse folder button
  if (els.btnBrowseDir) {
    els.btnBrowseDir.addEventListener('click', browseDownloadDir);
  }
  els.btnCancelYes.addEventListener('click', cancelDownload);
  els.btnCancelNo.addEventListener('click', hideCancelModal);
  // Close modal on backdrop click
  els.cancelModal.querySelector('.modal__backdrop').addEventListener('click', hideCancelModal);

  // History
  els.btnHistory.addEventListener('click', openHistory);
  els.btnHistoryClose.addEventListener('click', closeHistory);
  els.btnHistoryClear.addEventListener('click', clearHistory);
  els.historyModal.querySelector('.modal__backdrop').addEventListener('click', closeHistory);

  // Settings
  els.btnSettings.addEventListener('click', openSettings);
  els.btnSettingsSave.addEventListener('click', saveSettings);
  els.btnSettingsCancel.addEventListener('click', closeSettings);
  els.settingsModal.querySelector('.modal__backdrop').addEventListener('click', closeSettings);

  // Advanced settings toggle
  if (els.btnToggleAdvanced) {
    els.btnToggleAdvanced.addEventListener('click', toggleAdvancedSettings);
  }

  // Copy build/debug info
  if (els.btnCopyBuildInfo) {
    els.btnCopyBuildInfo.addEventListener('click', copyBuildInfo);
  }

  // Telemetry consent
  if (els.btnTelemetryAccept) els.btnTelemetryAccept.addEventListener('click', acceptTelemetry);
  if (els.btnTelemetryDecline) els.btnTelemetryDecline.addEventListener('click', declineTelemetry);
  if (els.telemetryToggle) els.telemetryToggle.addEventListener('change', onTelemetryToggleChanged);

  // Update modal
  els.btnUpdateNow.addEventListener('click', performUpdate);
  els.btnUpdateLater.addEventListener('click', hideUpdateModal);
  els.btnUpdateSkip.addEventListener('click', skipUpdateVersion);
  els.updateModal.querySelector('.modal__backdrop').addEventListener('click', hideUpdateModal);

  // Theme
  els.btnThemeToggle.addEventListener('click', toggleTheme);

  // Depot Filters
  if (els.depotSearch) {
    els.depotSearch.addEventListener('input', applyDepotFilters);
  }
  if (els.showSelectedOnly) {
    els.showSelectedOnly.addEventListener('change', applyDepotFilters);
  }

  // Step 3 → Step 4 / Reset
  if (els.btnNextStep) {
    els.btnNextStep.addEventListener('click', () => {
      if (state.shortcutSupported) {
        goToShortcutStep();
      } else {
        resetApp();
      }
    });
  }

  // Step 4: Shortcuts
  if (els.btnBrowseExe) {
    els.btnBrowseExe.addEventListener('click', browseExe);
  }
  if (els.btnCreateShortcuts) {
    els.btnCreateShortcuts.addEventListener('click', createShortcuts);
  }
  if (els.shortcutDesktop) {
    els.shortcutDesktop.addEventListener('change', updateCreateShortcutsButton);
  }
  if (els.shortcutStartMenu) {
    els.shortcutStartMenu.addEventListener('change', updateCreateShortcutsButton);
  }
  if (els.btnShortcutNew) {
    els.btnShortcutNew.addEventListener('click', resetApp);
  }
  if (els.btnShortcutStartOver) {
    els.btnShortcutStartOver.addEventListener('click', () => goToStep(2));
  }
  if (els.btnToggleDetected) {
    els.btnToggleDetected.addEventListener('click', () => {
      const list = els.shortcutDetectedList;
      const arrow = els.btnToggleDetected.querySelector('.settings-advanced__arrow');
      list.classList.toggle('hidden');
      if (arrow) arrow.textContent = list.classList.contains('hidden') ? '\u25B6' : '\u25BC';
    });
  }
}

// ============ Tauri Integration (replaces Electron) ============
function initTauri() {
  // Window control buttons (custom title bar on all platforms)
  document.getElementById('btn-minimize').addEventListener('click', () => invoke('minimize_window'));
  document.getElementById('btn-maximize').addEventListener('click', () => invoke('maximize_window'));
  document.getElementById('btn-close').addEventListener('click', () => invoke('close_window'));

  // ===== Manual Window Drag for Linux (WebKitGTK) =====
  // data-tauri-drag-region and -webkit-app-region:drag do NOT work
  // reliably on Linux/WebKitGTK. This manual mousedown handler ensures
  // window dragging works on all platforms by directly calling startDragging().
  const titleBar = document.getElementById('title-bar');
  if (titleBar) {
    titleBar.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      if (e.target.closest('.title-bar__controls')) return;

      // Don't start dragging if mouse is in the resize zone (top edge of window)
      // This allows the window resize handle to work properly
      const resizeThreshold = 5; // pixels from window edge
      if (e.clientY <= resizeThreshold) return;

      // Fire-and-forget: startDragging() muss synchron im selben Event-Tick initiiert werden
      // await würde auf Linux/Wayland zu spät sein (Window-Manager lehnt verspätete Drag-Anfragen ab)
      window.__TAURI__.window.getCurrentWindow().startDragging();
    });

    // Double-click on title bar to maximize/restore
    titleBar.addEventListener('dblclick', (e) => {
      if (e.target.closest('.title-bar__controls')) return;
      invoke('maximize_window');
    });
  }

  // Close confirmation modal (during downloads)
  const closeModal = document.getElementById('close-modal');
  const btnCloseYes = document.getElementById('btn-close-yes');
  const btnCloseNo = document.getElementById('btn-close-no');

  listen('close-requested', () => {
    closeModal.classList.remove('hidden');
  });

  btnCloseNo.addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

  btnCloseYes.addEventListener('click', () => {
    closeModal.classList.add('hidden');
    invoke('close_window');
  });

  // Close modal on backdrop click
  closeModal.querySelector('.modal__backdrop').addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

  // Check .NET at startup
  checkDotNet();

  // Check shortcut support (Windows only)
  checkShortcutSupport();

  // Check for updates after a short delay (non-blocking)
  setTimeout(checkForUpdates, 1500);
}

// ============ Download History ============
async function openHistory() {
  els.historyModal.classList.remove('hidden');
  await loadHistory();
}

function closeHistory() {
  els.historyModal.classList.add('hidden');
}

async function loadHistory() {
  els.historyList.innerHTML = '<div class="history-loading"><div class="spinner"></div><span>Loading history...</span></div>';

  try {
    const entries = await invoke('get_history');
    renderHistory(entries);
  } catch (e) {
    els.historyList.innerHTML = '<div class="history-empty">Failed to load history.</div>';
    console.error('Failed to load history:', e);
  }
}

function renderHistory(entries) {
  if (!entries || entries.length === 0) {
    els.historyList.innerHTML = '<div class="history-empty">No downloads yet. Your download history will appear here.</div>';
    els.btnHistoryClear.style.display = 'none';
    return;
  }

  els.btnHistoryClear.style.display = '';
  els.historyList.innerHTML = entries.map(entry => {
    const date = entry.completed_at ? formatHistoryDate(entry.completed_at) : formatHistoryDate(entry.started_at);
    const badgeClass = entry.status === 'complete' ? 'history-entry__badge--complete'
      : entry.status === 'partial' ? 'history-entry__badge--partial'
      : entry.status === 'cancelled' ? 'history-entry__badge--cancelled'
      : 'history-entry__badge--failed';
    const statusLabel = entry.status === 'complete' ? 'Complete'
      : entry.status === 'partial' ? 'Partial'
      : entry.status === 'cancelled' ? 'Cancelled'
      : 'Failed';
    const imgHtml = entry.header_image
      ? `<img class="history-entry__image" src="${escapeHtml(entry.header_image)}" alt="" loading="lazy" onerror="this.style.display='none'">`
      : '<div class="history-entry__image history-entry__image--placeholder"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></div>';
    const name = entry.game_name ? escapeHtml(entry.game_name) : `App ${escapeHtml(entry.app_id)}`;

    return `
      <div class="history-entry" data-entry-id="${escapeHtml(entry.id)}">
        ${imgHtml}
        <div class="history-entry__info">
          <div class="history-entry__name">${name}</div>
          <div class="history-entry__meta">
            <span class="history-entry__appid">App ${escapeHtml(entry.app_id)}</span>
            <span class="history-entry__date">${date}</span>
            <span class="history-entry__badge ${badgeClass}">${statusLabel}</span>
          </div>
          <div class="history-entry__depots">${entry.depots_downloaded}/${entry.depot_count} depots downloaded</div>
        </div>
        <div class="history-entry__actions">
          <button class="btn btn--small btn--outline history-action-redownload" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" title="Re-download">🔄</button>
          <button class="btn btn--small btn--outline history-action-folder" data-path="${escapeHtml(entry.download_dir)}" title="Open Folder"${entry.status === 'cancelled' ? ' disabled' : ''}>📂</button>
          <button class="btn btn--small btn--outline history-action-remove" data-entry-id="${escapeHtml(entry.id)}" title="Remove">🗑</button>
        </div>
      </div>
    `;
  }).join('');

  // Attach event handlers
  els.historyList.querySelectorAll('.history-action-redownload').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const appId = btn.dataset.appId;
      const depotIds = (btn.dataset.depotIds || '').split(',').filter(Boolean);
      closeHistory();
      // Reset state for new search
      state.parsedData = null;
      state.selectedDepots.clear();
      state.jobId = null;
      state.depotManifests = {};
      state.searchRepos = [];
      state.selectedRepo = null;
      state.searchAppId = null;
      state.searchSha = null;
      state.searchKeyVdfKeys = null;
      cleanupProgressListener();
      els.gameInfoBanner.classList.add('hidden');
      els.searchResults.classList.add('hidden');
      els.searchNextRow.classList.add('hidden');
      els.searchError.classList.add('hidden');
      els.searchGameBanner.classList.add('hidden');
      els.manifestLoading.classList.add('hidden');
      resetUpload();
      // Set flags for auto-proceed to step 2 with previously downloaded depots
      autoRedownloadPending = true;
      autoSelectAllOnStep2 = true;
      autoSelectDepotIds = depotIds.length > 0 ? depotIds : null;
      switchTab('search');
      els.searchAppIdInput.value = appId;
      goToStep(1);
      performSearch();
    });
  });

  els.historyList.querySelectorAll('.history-action-folder').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await invoke('open_folder', { path: btn.dataset.path });
      } catch (err) {
        console.error('Failed to open folder:', err);
      }
    });
  });

  els.historyList.querySelectorAll('.history-action-remove').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await invoke('remove_history_entry', { entryId: btn.dataset.entryId });
        await loadHistory();
      } catch (err) {
        console.error('Failed to remove history entry:', err);
      }
    });
  });
}

function formatHistoryDate(dateStr) {
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) +
      ', ' + d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
  } catch {
    return dateStr;
  }
}

async function clearHistory() {
  try {
    await invoke('clear_history');
    await loadHistory();
  } catch (e) {
    console.error('Failed to clear history:', e);
  }
}

// ============ Step 4: Shortcut Creation ============
async function checkShortcutSupport() {
  try {
    const result = await invoke('is_shortcut_supported');
    state.shortcutSupported = result.supported;
    if (state.shortcutSupported) {
      if (els.step4Connector) els.step4Connector.classList.remove('hidden');
      if (els.step4Indicator) els.step4Indicator.classList.remove('hidden');
    }
  } catch (e) {
    console.error('Failed to check shortcut support:', e);
  }
}

async function goToShortcutStep() {
  goToStep(4);
  await detectExecutables();
}

async function detectExecutables() {
  if (!state.downloadDir) return;
  els.shortcutExePath.value = 'Scanning for executables...';
  els.btnCreateShortcuts.disabled = true;
  els.shortcutDetectedSection.classList.add('hidden');

  try {
    const result = await invoke('detect_executables', { downloadDir: state.downloadDir });
    const exes = result.executables || [];

    if (exes.length === 0) {
      els.shortcutExePath.value = '';
      els.shortcutExePath.placeholder = 'No executables found. Browse manually.';
      els.btnCreateShortcuts.disabled = false;
      return;
    }

    // Set the recommended exe
    const recommended = exes.find(e => e.recommended) || exes[0];
    els.shortcutExePath.value = recommended.path;
    els.btnCreateShortcuts.disabled = false;

    // Show other detected exes if more than 1
    if (exes.length > 1) {
      els.shortcutDetectedSection.classList.remove('hidden');
      els.shortcutDetectedList.innerHTML = exes.map(exe => {
        const sizeStr = formatShortcutFileSize(exe.size);
        const recBadge = exe.recommended ? ' <span class="shortcut-exe-badge">Recommended</span>' : '';
        return `<div class="shortcut-exe-item" data-path="${escapeHtml(exe.path)}">
          <span class="shortcut-exe-item__name">${escapeHtml(exe.name)}${recBadge}</span>
          <span class="shortcut-exe-item__size">${sizeStr}</span>
        </div>`;
      }).join('');

      // Click to select
      els.shortcutDetectedList.querySelectorAll('.shortcut-exe-item').forEach(item => {
        item.addEventListener('click', () => {
          els.shortcutExePath.value = item.dataset.path;
          els.btnCreateShortcuts.disabled = false;
        });
      });
    }
  } catch (e) {
    console.error('Failed to detect executables:', e);
    els.shortcutExePath.value = '';
    els.shortcutExePath.placeholder = 'Detection failed. Browse manually.';
    els.btnCreateShortcuts.disabled = false;
  }
}

async function browseExe() {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      defaultPath: state.downloadDir || undefined,
      filters: [{ name: 'Executables', extensions: ['exe'] }],
      title: 'Select Game Executable'
    });
    if (filePath) {
      els.shortcutExePath.value = filePath;
      els.btnCreateShortcuts.disabled = false;
    }
  } catch (e) {
    console.error('Failed to browse for exe:', e);
  }
}

async function createShortcuts() {
  const exePath = els.shortcutExePath.value.trim();
  if (!exePath) return;

  const createDesktop = els.shortcutDesktop.checked;
  const createStartMenu = els.shortcutStartMenu.checked;

  if (!createDesktop && !createStartMenu) {
    showShortcutStatus(false, 'Please select at least one shortcut location.');
    return;
  }

  els.btnCreateShortcuts.disabled = true;
  els.btnCreateShortcuts.textContent = 'Creating...';

  try {
    const gameName = state.gameName || 'Game';
    const result = await invoke('create_shortcuts', {
      exePath,
      gameName,
      iconPath: null,
      createDesktop,
      createStartMenu
    });

    const messages = [];
    if (result.desktop) messages.push('Desktop shortcut created');
    if (result.startMenu) messages.push('Start menu shortcut created');
    if (result.errors && result.errors.length > 0) {
      messages.push('Errors: ' + result.errors.join(', '));
    }

    const allGood = (!createDesktop || result.desktop) && (!createStartMenu || result.startMenu);
    if (allGood) emitEvent('shortcut_created', { desktop: !!result.desktop, start_menu: !!result.startMenu });
    showShortcutStatus(allGood, messages.join('. ') + '.');

    if (allGood) {
      els.btnCreateShortcuts.disabled = true;
      els.btnCreateShortcuts.textContent = 'Shortcuts Created';
    } else {
      els.btnCreateShortcuts.disabled = false;
      els.btnCreateShortcuts.textContent = 'Create Shortcuts';
    }
  } catch (e) {
    showShortcutStatus(false, `Failed to create shortcuts: ${e}`);
    els.btnCreateShortcuts.disabled = false;
    els.btnCreateShortcuts.textContent = 'Create Shortcuts';
  }
}

function showShortcutStatus(success, message) {
  els.shortcutStatus.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  els.shortcutStatus.classList.add(success ? 'completion-message--success' : 'completion-message--error');
  els.shortcutStatus.textContent = message;
}

function updateCreateShortcutsButton() {
  if (!els.btnCreateShortcuts) return;
  const anySelected = els.shortcutDesktop.checked || els.shortcutStartMenu.checked;
  els.btnCreateShortcuts.disabled = !anySelected;
}

function formatShortcutFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(2) + ' GB';
}

// ============ Init ============
document.addEventListener('DOMContentLoaded', () => {
  initTheme();
  initUpload();
  initEvents();
  loadSettingsAndDefaults();
  initTauri();
  initTelemetryConsent();
});
