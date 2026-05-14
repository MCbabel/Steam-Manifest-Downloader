const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

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
  searchRepos: [],
  selectedRepo: null,
  searchAppId: null,
  searchRepo: null,
  searchSha: null,
  searchKeyVdfKeys: null,
  speedTracker: {
    lastPercent: 0,
    lastTime: 0,
    samples: [],
    currentDepotSize: 0,
    currentDepotId: null,
    depotStartTime: 0,
    staleTimer: null,
    lastUpdateTime: 0,
  },
};

const SVG_BASE = 'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"';
const ICONS = {
  upload: `<svg class="btn-icon" ${SVG_BASE}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`,
  folderOpen: `<svg class="btn-icon" ${SVG_BASE}><path d="M6 14l1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"/></svg>`,
  refresh: `<svg class="btn-icon" ${SVG_BASE}><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>`,
  trash: `<svg class="btn-icon" ${SVG_BASE}><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>`,
  x: `<svg class="btn-icon" ${SVG_BASE}><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`,
  check: `<svg class="btn-icon" ${SVG_BASE}><polyline points="20 6 9 17 4 12"/></svg>`,
  checkCircle: `<svg class="btn-icon" ${SVG_BASE}><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`,
  sun: `<svg class="theme-icon theme-icon--sun" id="theme-icon" width="16" height="16" ${SVG_BASE}><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>`,
  moon: `<svg class="theme-icon theme-icon--moon" id="theme-icon" width="16" height="16" ${SVG_BASE}><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`,
};

const MH_APIKEY_STORAGE_KEY = 'manifestHubApiKey';
let autoRedownloadPending = false;
let autoSelectAllOnStep2 = false;
let autoSelectDepotIds = null; // specific depot IDs to re-select, or null for all
let defaultDownloadDir = '';

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const els = {
  stepUpload: $('#step-upload'),
  stepSelect: $('#step-select'),
  stepProgress: $('#step-progress'),
  tabUpload: $('#tab-upload'),
  tabSearch: $('#tab-search'),
  tabContentUpload: $('#tab-content-upload'),
  tabContentSearch: $('#tab-content-search'),
  dropZone: $('#drop-zone'),
  fileInfo: $('#file-info'),
  fileName: $('#file-name'),
  fileRemove: $('#file-remove'),
  uploadError: $('#upload-error'),
  uploadLoading: $('#upload-loading'),
  searchAppIdInput: $('#search-appid-input'),
  searchAutocomplete: $('#search-autocomplete'),
  btnSearch: $('#btn-search'),
  searchInputBlock: $('#search-input-block'),
  sourcesEmpty: $('#sources-empty-state'),
  sourcesEmptyInput: $('#sources-empty-input'),
  btnSourcesEmptyAdd: $('#btn-sources-empty-add'),
  sourcesEmptyError: $('#sources-empty-error'),
  sourcesList: $('#sources-list'),
  sourcesAddInput: $('#sources-add-input'),
  btnSourcesAdd: $('#btn-sources-add'),
  sourcesAddError: $('#sources-add-error'),
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
  appIdDisplay: $('#app-id-display'),
  depotCount: $('#depot-count'),
  depotList: $('#depot-list'),
  btnSelectAll: $('#btn-select-all'),
  btnDeselectAll: $('#btn-deselect-all'),
  btnBack: $('#btn-back'),
  btnDownload: $('#btn-download'),
  depotProgressFill: $('#depot-progress-fill'),
  depotProgressText: $('#depot-progress-text'),
  downloadSpeedInfo: $('#download-speed-info'),
  downloadSpeed: $('#download-speed'),
  downloadEta: $('#download-eta'),
  progressHeader: $('#progress-header'),
  progressBarFill: $('#progress-bar-fill'),
  progressStatus: $('#progress-status'),
  depotProgressList: $('#depot-progress-list'),
  terminalOutput: $('#terminal-output'),
  completionMessage: $('#completion-message'),
  btnCancel: $('#btn-cancel'),
  mhApiKey: $('#mh-apikey'),
  downloadDirInput: $('#download-dir'),
  btnBrowseDir: $('#btn-browse-dir'),
  diskSpaceInfo: $('#disk-space-info'),
  diskSpaceText: $('#disk-space-text'),
  gameInfoBanner: $('#game-info-banner'),
  gameInfoLoading: $('#game-info-loading'),
  gameHeaderImage: $('#game-header-image'),
  gameName: $('#game-name'),
  gameDescription: $('#game-description'),
  cancelModal: $('#cancel-modal'),
  btnCancelYes: $('#btn-cancel-yes'),
  btnCancelNo: $('#btn-cancel-no'),
  btnThemeToggle: $('#btn-theme-toggle'),
  depotSearch: $('#depotSearch'),
  showSelectedOnly: $('#showSelectedOnly'),
  btnSettings: $('#btn-settings'),
  settingsModal: $('#settings-modal'),
  btnSettingsSave: $('#btn-settings-save'),
  btnSettingsCancel: $('#btn-settings-cancel'),
  autoUpdateToggle: $('#auto-update-toggle'),
  btnToggleAdvanced: $('#btn-toggle-advanced'),
  advancedSettingsContent: $('#advanced-settings-content'),
  ddExtraArgsInput: $('#dd-extra-args-input'),
  maxRetriesInput: $('#max-retries-input'),
  speedLimitInput: $('#speed-limit-input'),
  proxyInput: $('#proxy-input'),
  notificationSoundToggle: $('#notification-sound-toggle'),
  telemetryModal: $('#telemetry-modal'),
  btnTelemetryAccept: $('#btn-telemetry-accept'),
  btnTelemetryDecline: $('#btn-telemetry-decline'),
  telemetryToggle: $('#telemetry-toggle'),
  buildInfoChannel: $('#build-info-channel'),
  buildInfoVersion: $('#build-info-version'),
  buildInfoSha: $('#build-info-sha'),
  buildInfoDate: $('#build-info-date'),
  buildInfoProfile: $('#build-info-profile'),
  buildInfoPlatform: $('#build-info-platform'),
  btnCopyBuildInfo: $('#btn-copy-build-info'),
  btnHistory: $('#btn-history'),
  historyModal: $('#history-modal'),
  historyList: $('#history-list'),
  btnHistoryClear: $('#btn-history-clear'),
  btnHistoryClose: $('#btn-history-close'),
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

function goToStep(step) {
  state.currentStep = step;

  [els.stepUpload, els.stepSelect, els.stepProgress, els.stepShortcut].forEach((el, i) => {
    if (!el) return;
    el.classList.toggle('active', i + 1 === step);
    el.classList.toggle('hidden', i + 1 !== step);
  });

  $$('.steps__item').forEach((el) => {
    const s = parseInt(el.dataset.step);
    el.classList.toggle('active', s === step);
    el.classList.toggle('completed', s < step);
  });

  const stepsIndicator = document.getElementById('steps-indicator');
  if (stepsIndicator) {
    const visibleMax = document.getElementById('step4-indicator')?.classList.contains('hidden') === false ? 4 : 3;
    stepsIndicator.setAttribute('aria-valuemax', String(visibleMax));
    stepsIndicator.setAttribute('aria-valuenow', String(Math.min(step, visibleMax)));
  }
}

function switchTab(tabName) {
  state.mode = tabName;

  els.tabUpload.classList.toggle('active', tabName === 'upload');
  els.tabSearch.classList.toggle('active', tabName === 'search');

  els.tabContentUpload.classList.toggle('active', tabName === 'upload');
  els.tabContentSearch.classList.toggle('active', tabName === 'search');
}

function initUpload() {
  const dropZone = els.dropZone;

  dropZone.addEventListener('click', openFileDialog);

  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });

  dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('drag-over');
  });

  // HTML5 drag-drop inside a WebView doesn't surface file paths; we rely on the
  // 'tauri://drag-drop' event below instead of e.dataTransfer.
  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
  });

  listen('tauri://drag-drop', (event) => {
    const paths = event.payload.paths || event.payload;
    if (Array.isArray(paths) && paths.length > 0) {
      handleFilePath(paths[0]);
    }
  });

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

async function handleDepotManifestFile(depotId) {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      filters: [{ name: 'Manifest Files', extensions: ['manifest'] }]
    });
    if (!filePath) return;

    const fileName = filePath.split(/[\\/]/).pop();

    state.depotManifests[depotId] = {
      originalName: fileName,
      storedPath: filePath
    };

    const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
    const btnEl = document.querySelector(`.depot-manifest-btn[data-depot-id="${depotId}"]`);
    if (statusEl) statusEl.innerHTML = `<span class="manifest-uploaded">${ICONS.check} ${escapeHtml(fileName)}</span>`;
    if (btnEl) btnEl.innerHTML = `${ICONS.upload} Replace`;
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
  if (btnEl) btnEl.innerHTML = `${ICONS.upload} Upload .manifest`;
}

async function handleFilePath(filePath) {
  const ext = filePath.split('.').pop().toLowerCase();
  if (ext !== 'lua' && ext !== 'st') {
    showUploadError('Please select a .lua or .st file');
    return;
  }

  const fileName = filePath.split(/[\\/]/).pop();

  els.dropZone.classList.add('hidden');
  els.fileInfo.classList.remove('hidden');
  els.fileName.textContent = fileName;
  els.uploadError.classList.add('hidden');
  els.uploadLoading.classList.remove('hidden');

  try {
    const raw = await invoke('parse_lua_file', { path: filePath });

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

  if (autocompleteDebounceTimer) {
    clearTimeout(autocompleteDebounceTimer);
    autocompleteDebounceTimer = null;
  }

  // If empty or numeric → no autocomplete
  if (!val || isNumericInput(val)) {
    hideAutocomplete();
    return;
  }

  if (val.length < 2) {
    hideAutocomplete();
    return;
  }

  // Debounce: 400ms
  autocompleteDebounceTimer = setTimeout(() => {
    triggerAutocomplete(val);
  }, 400);
}

async function loadDepotSources() {
  try {
    const settings = await invoke('get_settings');
    return Array.isArray(settings.depot_sources) ? settings.depot_sources : [];
  } catch {
    return [];
  }
}

async function saveDepotSources(sources) {
  const settings = await invoke('get_settings');
  settings.depot_sources = sources;
  await invoke('save_settings', { settings });
}

function validateSourceUrl(raw) {
  const url = (raw || '').trim();
  if (!url) return i18n.t('errors.sourceEmpty');
  if (!/^https?:\/\//i.test(url)) return i18n.t('errors.sourceProtocol');
  return null;
}

async function refreshSourcesUI() {
  const sources = await loadDepotSources();
  const empty = sources.length === 0;
  if (els.sourcesEmpty) els.sourcesEmpty.classList.toggle('hidden', !empty);
  if (els.searchInputBlock) els.searchInputBlock.classList.toggle('hidden', empty);
  if (els.sourcesList) {
    const removeLabel = escapeHtml(i18n.t('settings.removeSource'));
    els.sourcesList.innerHTML = sources
      .map((s, i) => {
        const safe = escapeHtml(s);
        return `<li><span title="${safe}">${safe}</span><button data-source-idx="${i}">${removeLabel}</button></li>`;
      })
      .join('');
  }
}

async function addDepotSource(rawUrl, errorEl) {
  if (errorEl) errorEl.classList.add('hidden');
  const err = validateSourceUrl(rawUrl);
  if (err) {
    if (errorEl) {
      errorEl.textContent = err;
      errorEl.classList.remove('hidden');
    }
    return false;
  }
  const url = rawUrl.trim();
  const sources = await loadDepotSources();
  if (sources.includes(url)) {
    if (errorEl) {
      errorEl.textContent = 'Source already added';
      errorEl.classList.remove('hidden');
    }
    return false;
  }
  sources.push(url);
  await saveDepotSources(sources);
  await refreshSourcesUI();
  return true;
}

async function removeDepotSource(index) {
  const settings = await invoke('get_settings');
  const sources = Array.isArray(settings.depot_sources) ? settings.depot_sources : [];
  if (index < 0 || index >= sources.length) return;
  const target = sources[index];
  const pristine = Array.isArray(settings.pristine_default_sources) ? settings.pristine_default_sources : [];

  if (pristine.includes(target)) {
    const confirmed = await confirmRemoveDefaultSource();
    if (!confirmed) return;
  }

  sources.splice(index, 1);
  await saveDepotSources(sources);
  await refreshSourcesUI();
}

function confirmRemoveDefaultSource() {
  const modal = document.getElementById('remove-source-modal');
  const yes = document.getElementById('btn-remove-source-yes');
  const no = document.getElementById('btn-remove-source-no');
  if (!modal || !yes || !no) return Promise.resolve(true);

  return new Promise((resolve) => {
    const cleanup = () => {
      yes.removeEventListener('click', onYes);
      no.removeEventListener('click', onNo);
      modal.classList.add('hidden');
    };
    const onYes = () => { cleanup(); resolve(true); };
    const onNo = () => { cleanup(); resolve(false); };
    yes.addEventListener('click', onYes);
    no.addEventListener('click', onNo);
    modal.classList.remove('hidden');
  });
}

async function performSearch() {
  hideAutocomplete();
  const appIdStr = els.searchAppIdInput.value.trim();
  if (!appIdStr) return;

  const appId = parseInt(appIdStr, 10);
  if (isNaN(appId) || appId <= 0) {
    showSearchError('Please enter a valid App ID');
    return;
  }

  els.searchError.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  state.selectedRepo = null;
  state.searchRepos = [];
  state.searchAppId = appId;

  els.searchLoading.classList.remove('hidden');
  els.btnSearch.disabled = true;

  emitEvent('search_performed');

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
      showSearchError('No manifests found for this App ID in any configured depot source. Add or enable more sources in Settings.');
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
  }
}

function renderRepoList(repos) {
  els.repoList.innerHTML = '';

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

  if (repos.length === 1) {
    selectRepo(0);
  }

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
  $$('.repo-card').forEach(c => c.classList.remove('selected'));

  state.selectedRepo = state.searchRepos[index];

  const card = els.repoList.querySelector(`[data-repo-index="${index}"]`);
  if (card) card.classList.add('selected');

  els.searchNextRow.classList.remove('hidden');
}

async function proceedFromSearch() {
  if (!state.selectedRepo) return;

  const repo = state.selectedRepo;
  const appId = state.searchAppId;

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
      showSearchError('No manifests found for this App ID in any configured depot source. Add or enable more sources in Settings.');
      els.searchNextRow.classList.remove('hidden');
      return;
    }

    state.parsedData = {
      mainAppId: appId,
      depots: depots
    };

    showSelectionStep();
  } catch (error) {
    els.manifestLoading.classList.add('hidden');
    showSearchError(String(error));
    els.searchNextRow.classList.remove('hidden');
  }
}

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

function goBackToSelect() {
  cleanupProgressListener();
  state.jobId = null;
  // Go back to Step 2 (select) — parsedData and selectedDepots are still intact
  goToStep(2);
}

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
  }

  els.gameInfoLoading.classList.add('hidden');
}

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

  const savedApiKey = localStorage.getItem(MH_APIKEY_STORAGE_KEY);
  if (savedApiKey) els.mhApiKey.value = savedApiKey;

  if (els.downloadDirInput && defaultDownloadDir) {
    els.downloadDirInput.value = els.downloadDirInput.value || defaultDownloadDir;
  }

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
            ${ICONS.upload} Upload .manifest
          </button>
          <span class="depot-manifest-status" data-depot-id="${safeDepotId}"></span>
        </div>
      </div>
    `;

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

  if (els.depotSearch) els.depotSearch.value = '';
  if (els.showSelectedOnly) els.showSelectedOnly.checked = false;

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

async function startDownload() {
  const data = state.parsedData;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));

  if (selectedDepots.length === 0) return;

  emitEvent('download_started', { depot_count: selectedDepots.length });

  requestNotificationPermission();

  const mhApiKey = els.mhApiKey.value.trim();
  if (mhApiKey) localStorage.setItem(MH_APIKEY_STORAGE_KEY, mhApiKey);
  saveDownloadDir();

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

  goToStep(3);
  initProgressUI(depotsWithCustomManifests);

  try {
    const downloadConfig = {
      mainAppId: String(data.mainAppId),
      selectedDepots: depotsWithCustomManifests,
      manifestHubApiKey: mhApiKey || null,
      downloadDir: getDownloadDir() || null,
      gameName: state.gameName || null,
      headerImage: state.headerImage || null
    };

    if (state.mode === 'search') {
      if (state.searchRepo) downloadConfig.repo = state.searchRepo;
      if (state.searchSha) downloadConfig.sha = state.searchSha;
      if (state.searchKeyVdfKeys) downloadConfig.keyVdfKeys = state.searchKeyVdfKeys;
    }

    const result = await invoke('start_download', { config: downloadConfig });

    state.jobId = result.jobId;
    state.downloadDir = result.downloadDir;

    connectProgressListener();
  } catch (error) {
    appendTerminalLine(`Error: ${error}`, 'error');
    showCompletion(false, String(error));
  }
}

function initProgressUI(depots) {
  els.progressBarFill.style.width = '0%';
  els.progressStatus.textContent = 'Initializing...';
  els.terminalOutput.innerHTML = '';
  els.completionMessage.classList.add('hidden');
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  els.btnCancel.classList.remove('hidden');
  els.btnCancel.disabled = false;
  els.btnCancel.innerHTML = `${ICONS.x} Cancel Download`;
  els.diskSpaceInfo.classList.add('hidden');
  if (els.depotProgressFill) els.depotProgressFill.style.width = '0%';
  if (els.depotProgressText) els.depotProgressText.textContent = '0%';
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;

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

async function connectProgressListener() {
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
      state.speedTracker.samples = [];
      state.speedTracker.lastTime = 0;
      state.speedTracker.lastPercent = 0;
      state.speedTracker.depotStartTime = Date.now();
      state.speedTracker.lastUpdateTime = Date.now();
      clearInterval(state.speedTracker.staleTimer);
      state.speedTracker.staleTimer = null;
      startStaleTimer();
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

  tracker.lastUpdateTime = now;

  if (tracker.lastTime === 0) {
    tracker.lastTime = now;
    tracker.lastPercent = percent;
    return;
  }

  tracker.samples.push({ percent, time: now });

  if (tracker.samples.length > 10) {
    tracker.samples.shift();
  }

  if (tracker.samples.length < 2) return;

  const oldest = tracker.samples[0];
  const newest = tracker.samples[tracker.samples.length - 1];
  const timeDelta = (newest.time - oldest.time) / 1000; // seconds
  const percentDelta = newest.percent - oldest.percent;

  if (timeDelta <= 0 || percentDelta <= 0) return;

  const percentPerSecond = percentDelta / timeDelta;
  const remainingPercent = 100 - percent;
  const etaSeconds = remainingPercent / percentPerSecond;

  let speedText = '';
  if (tracker.currentDepotSize > 0) {
    const bytesPerSecond = (tracker.currentDepotSize * percentDelta / 100) / timeDelta;
    const mbPerSecond = bytesPerSecond / (1024 * 1024);
    speedText = `↓ ${mbPerSecond.toFixed(1)} MB/s`;
  } else {
    speedText = `↓ ${percentPerSecond.toFixed(2)}%/s`;
  }

  const etaText = formatEta(etaSeconds);

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
  els.progressStatus.innerHTML = `<span class="status-success">${ICONS.checkCircle} Complete!</span>`;
  updateDepotDownloadProgress(100);
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  emitEvent('download_completed', { success: true });
  showCompletion(true, msg.message);

  if (msg.results) {
    const results = Array.isArray(msg.results) ? msg.results : [];
    results.forEach((r) => {
      updateDepotStatus(r.depotId, r.success ? 'done' : 'error', r.success ? 'Complete' : 'Failed');
    });
  }

  appendTerminalLine(`\n${msg.message}`, 'success');

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

function updateDepotStatus(depotId, status, text) {
  const item = document.getElementById(`depot-progress-${depotId}`);
  if (!item) return;

  const icon = item.querySelector('.depot-progress-item__icon');
  const statusEl = item.querySelector('.depot-progress-item__status');

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
  if (els.shortcutStatus) els.shortcutStatus.classList.add('hidden');
  if (els.shortcutDetectedSection) els.shortcutDetectedSection.classList.add('hidden');
  if (els.shortcutDetectedList) els.shortcutDetectedList.innerHTML = '';
  if (els.shortcutExePath) els.shortcutExePath.value = '';
  if (els.btnCreateShortcuts) { els.btnCreateShortcuts.disabled = false; els.btnCreateShortcuts.textContent = 'Create Shortcuts'; }
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  els.gameInfoBanner.classList.add('hidden');
  els.gameInfoLoading.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchError.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  els.manifestLoading.classList.add('hidden');
  resetUpload();
  goToStep(1);
}

async function openSettings() {
  try {
    const settings = await invoke('get_settings');
    els.autoUpdateToggle.checked = settings.auto_update !== false;

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
  refreshSourcesUI();
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

async function initTelemetryConsent() {
  try {
    const status = await invoke('get_telemetry_status');
    if (status.consent === 'pending') {
      const modal = els.telemetryModal;
      await new Promise((resolve) => {
        const observer = new MutationObserver(() => {
          if (modal.classList.contains('hidden')) {
            observer.disconnect();
            resolve();
          }
        });
        observer.observe(modal, { attributes: true, attributeFilter: ['class'] });
        modal.classList.remove('hidden');
      });
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

    if (els.btnSettingsSave && els.btnSettingsSave.dataset.languageRestart === '1') {
      await invoke('restart_app');
      return;
    }
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
  let html = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  html = html.replace(/^### (.+)$/gm, '<h4>$1</h4>');
  html = html.replace(/^## (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^# (.+)$/gm, '<h2>$1</h2>');
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>');
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

  const externallyManaged = info.installMethod && info.installMethod !== 'self';
  const systemHint = document.getElementById('update-system-hint');
  if (systemHint) {
    systemHint.classList.toggle('hidden', !externallyManaged);
    if (externallyManaged) {
      renderUpdateCommands(info.installMethod);
    }
  }
  els.btnUpdateNow.classList.toggle('hidden', externallyManaged);
  els.btnUpdateSkip.classList.toggle('hidden', externallyManaged);

  els.updateModal.classList.remove('hidden');
}

function updateCommandsFor(method) {
  switch (method) {
    case 'flatpak':
      return [{ label: '', cmd: 'flatpak update de.mcbabel.SteamManifestDownloader' }];
    case 'snap':
      return [{ label: '', cmd: 'sudo snap refresh steam-manifest-downloader' }];
    case 'system':
    default:
      return [
        { label: i18n.t('modals.update.aurBinLabel'), cmd: 'paru -Syu steam-manifest-downloader-bin' },
        { label: i18n.t('modals.update.aurSourceLabel'), cmd: 'paru -Syu steam-manifest-downloader' },
      ];
  }
}

function renderUpdateCommands(method) {
  const list = document.getElementById('update-system-cmd-list');
  if (!list) return;
  const commands = updateCommandsFor(method);
  const copyLabel = i18n.t('modals.update.copyCmd');
  list.innerHTML = commands.map((c, i) => {
    const labelHtml = c.label
      ? `<div class="update-system-cmd__label">${escapeHtml(c.label)}</div>`
      : '';
    return `
      <div class="update-system-cmd__row">
        ${labelHtml}
        <div class="update-system-cmd__inner">
          <pre><code>${escapeHtml(c.cmd)}</code></pre>
          <button type="button" class="btn btn--outline btn--small update-system-cmd__copy" data-cmd-idx="${i}">${escapeHtml(copyLabel)}</button>
        </div>
      </div>
    `;
  }).join('');

  list.querySelectorAll('.update-system-cmd__copy').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const idx = parseInt(btn.dataset.cmdIdx, 10);
      const cmd = commands[idx]?.cmd;
      if (!cmd) return;
      try {
        await navigator.clipboard.writeText(cmd);
        const original = btn.textContent;
        btn.textContent = 'Copied!';
        setTimeout(() => { btn.textContent = original; }, 1500);
      } catch {}
    });
  });
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
    els.btnThemeToggle.innerHTML = theme === 'dark' ? ICONS.moon : ICONS.sun;
    els.btnThemeToggle.title = theme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';
    els.btnThemeToggle.setAttribute('aria-label', theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode');
  }
}

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
  }
}

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

  const dismissBtn = document.getElementById('dotnet-warning-dismiss');
  if (dismissBtn) {
    dismissBtn.addEventListener('click', () => {
      banner.classList.add('hidden');
      // Remember dismissal for this session
      sessionStorage.setItem('dotnetWarningDismissed', 'true');
    });
  }

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

function initEvents() {
  els.tabUpload.addEventListener('click', () => switchTab('upload'));
  els.tabSearch.addEventListener('click', () => { switchTab('search'); refreshSourcesUI(); });

  if (els.btnSourcesEmptyAdd) {
    els.btnSourcesEmptyAdd.addEventListener('click', async () => {
      const ok = await addDepotSource(els.sourcesEmptyInput.value, els.sourcesEmptyError);
      if (ok) els.sourcesEmptyInput.value = '';
    });
  }
  if (els.btnSourcesAdd) {
    els.btnSourcesAdd.addEventListener('click', async () => {
      const ok = await addDepotSource(els.sourcesAddInput.value, els.sourcesAddError);
      if (ok) els.sourcesAddInput.value = '';
    });
  }
  if (els.sourcesList) {
    els.sourcesList.addEventListener('click', async (e) => {
      const btn = e.target.closest('button[data-source-idx]');
      if (!btn) return;
      const idx = parseInt(btn.dataset.sourceIdx, 10);
      if (Number.isInteger(idx)) await removeDepotSource(idx);
    });
  }

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
  document.addEventListener('click', (e) => {
    if (!els.searchAppIdInput.contains(e.target) && !els.searchAutocomplete.contains(e.target)) {
      hideAutocomplete();
    }
  });
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

  els.btnSelectAll.addEventListener('click', selectAll);
  els.btnDeselectAll.addEventListener('click', deselectAll);
  els.btnBack.addEventListener('click', () => goToStep(1));
  els.btnDownload.addEventListener('click', startDownload);
  els.btnCancel.addEventListener('click', showCancelModal);
  if (els.btnBrowseDir) {
    els.btnBrowseDir.addEventListener('click', browseDownloadDir);
  }
  els.btnCancelYes.addEventListener('click', cancelDownload);
  els.btnCancelNo.addEventListener('click', hideCancelModal);
  els.cancelModal.querySelector('.modal__backdrop').addEventListener('click', hideCancelModal);

  els.btnHistory.addEventListener('click', openHistory);
  els.btnHistoryClose.addEventListener('click', closeHistory);
  els.btnHistoryClear.addEventListener('click', clearHistory);
  els.historyModal.querySelector('.modal__backdrop').addEventListener('click', closeHistory);

  els.btnSettings.addEventListener('click', openSettings);
  els.btnSettingsSave.addEventListener('click', saveSettings);
  els.btnSettingsCancel.addEventListener('click', closeSettings);
  els.settingsModal.querySelector('.modal__backdrop').addEventListener('click', closeSettings);

  if (els.btnToggleAdvanced) {
    els.btnToggleAdvanced.addEventListener('click', toggleAdvancedSettings);
  }

  if (els.btnCopyBuildInfo) {
    els.btnCopyBuildInfo.addEventListener('click', copyBuildInfo);
  }

  if (els.btnTelemetryAccept) els.btnTelemetryAccept.addEventListener('click', acceptTelemetry);
  if (els.btnTelemetryDecline) els.btnTelemetryDecline.addEventListener('click', declineTelemetry);
  if (els.telemetryToggle) els.telemetryToggle.addEventListener('change', onTelemetryToggleChanged);

  els.btnUpdateNow.addEventListener('click', performUpdate);
  els.btnUpdateLater.addEventListener('click', hideUpdateModal);
  els.btnUpdateSkip.addEventListener('click', skipUpdateVersion);
  els.updateModal.querySelector('.modal__backdrop').addEventListener('click', hideUpdateModal);

  els.btnThemeToggle.addEventListener('click', toggleTheme);

  if (els.depotSearch) {
    els.depotSearch.addEventListener('input', applyDepotFilters);
  }
  if (els.showSelectedOnly) {
    els.showSelectedOnly.addEventListener('change', applyDepotFilters);
  }

  if (els.btnNextStep) {
    els.btnNextStep.addEventListener('click', () => {
      if (state.shortcutSupported) {
        goToShortcutStep();
      } else {
        resetApp();
      }
    });
  }

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

function initTauri() {
  document.getElementById('btn-minimize').addEventListener('click', () => invoke('minimize_window'));
  document.getElementById('btn-maximize').addEventListener('click', () => invoke('maximize_window'));
  document.getElementById('btn-close').addEventListener('click', () => invoke('close_window'));

  // data-tauri-drag-region and -webkit-app-region:drag do NOT work
  // reliably on Linux/WebKitGTK. This manual mousedown handler ensures
  // window dragging works on all platforms by directly calling startDragging().
  const titleBar = document.getElementById('title-bar');
  if (titleBar) {
    titleBar.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      if (e.target.closest('.title-bar__controls')) return;

      // Reserve the top edge for resize; without this the drag eats the resize handle.
      const resizeThreshold = 5;
      if (e.clientY <= resizeThreshold) return;

      // Fire-and-forget. Under Wayland the compositor rejects drag requests that
      // arrive after the event tick, so we can't await here.
      window.__TAURI__.window.getCurrentWindow().startDragging();
    });

    titleBar.addEventListener('dblclick', (e) => {
      if (e.target.closest('.title-bar__controls')) return;
      invoke('maximize_window');
    });
  }

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

  closeModal.querySelector('.modal__backdrop').addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

  checkDotNet();

  checkShortcutSupport();
}

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
          <button class="btn btn--small btn--outline history-action-redownload" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" title="Re-download" aria-label="Re-download">${ICONS.refresh}</button>
          <button class="btn btn--small btn--outline history-action-folder" data-path="${escapeHtml(entry.download_dir)}" title="Open Folder" aria-label="Open download folder"${entry.status === 'cancelled' ? ' disabled' : ''}>${ICONS.folderOpen}</button>
          <button class="btn btn--small btn--outline history-action-remove" data-entry-id="${escapeHtml(entry.id)}" title="Remove" aria-label="Remove entry">${ICONS.trash}</button>
        </div>
      </div>
    `;
  }).join('');

  els.historyList.querySelectorAll('.history-action-redownload').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const appId = btn.dataset.appId;
      const depotIds = (btn.dataset.depotIds || '').split(',').filter(Boolean);
      closeHistory();
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

    const recommended = exes.find(e => e.recommended) || exes[0];
    els.shortcutExePath.value = recommended.path;
    els.btnCreateShortcuts.disabled = false;

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

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

const modalFocusReturn = new WeakMap();

function getFocusable(modal) {
  return Array.from(modal.querySelectorAll(FOCUSABLE_SELECTOR))
    .filter((el) => el.offsetParent !== null || el === document.activeElement);
}

function topmostVisibleDialog() {
  const dialogs = document.querySelectorAll('[role="dialog"]');
  for (let i = dialogs.length - 1; i >= 0; i--) {
    if (!dialogs[i].classList.contains('hidden')) return dialogs[i];
  }
  return null;
}

function setupModalA11y() {
  document.querySelectorAll('[role="dialog"]').forEach((modal) => {
    modal.setAttribute('aria-hidden', modal.classList.contains('hidden') ? 'true' : 'false');

    const observer = new MutationObserver(() => {
      const isHidden = modal.classList.contains('hidden');
      modal.setAttribute('aria-hidden', isHidden ? 'true' : 'false');

      if (!isHidden) {
        modalFocusReturn.set(modal, document.activeElement);
        const focusables = getFocusable(modal);
        if (focusables.length > 0) {
          requestAnimationFrame(() => focusables[0].focus());
        }
      } else {
        const returnTo = modalFocusReturn.get(modal);
        if (returnTo && typeof returnTo.focus === 'function') {
          returnTo.focus();
        }
        modalFocusReturn.delete(modal);
      }
    });

    observer.observe(modal, { attributes: true, attributeFilter: ['class'] });
  });

  document.addEventListener('keydown', (e) => {
    const modal = topmostVisibleDialog();
    if (!modal) return;

    if (e.key === 'Escape') {
      const cancelBtn = modal.querySelector('[data-modal-cancel]')
        || modal.querySelector('.btn--outline')
        || modal.querySelector('button');
      if (cancelBtn) {
        e.preventDefault();
        cancelBtn.click();
      }
      return;
    }

    if (e.key === 'Tab') {
      const focusables = getFocusable(modal);
      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  });
}

const LANGUAGE_FLAGS = {
  en: `<svg viewBox="0 0 60 30" preserveAspectRatio="xMidYMid slice">
    <rect width="60" height="30" fill="#012169"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#fff" stroke-width="6"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#C8102E" stroke-width="2.5"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#fff" stroke-width="10"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#C8102E" stroke-width="6"/>
  </svg>`,
  de: `<svg viewBox="0 0 5 3" preserveAspectRatio="none">
    <rect width="5" height="1" y="0" fill="#000"/>
    <rect width="5" height="1" y="1" fill="#DD0000"/>
    <rect width="5" height="1" y="2" fill="#FFCE00"/>
  </svg>`,
};

function renderLanguageCards(container, activeCode, onSelect) {
  if (!container) return;
  container.innerHTML = '';
  for (const locale of i18n.getAvailableLocales()) {
    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'language-card' + (locale.code === activeCode ? ' is-active' : '');
    card.setAttribute('data-lang', locale.code);
    card.setAttribute('aria-label', locale.label);
    card.innerHTML = `
      <span class="language-card__flag" aria-hidden="true">${LANGUAGE_FLAGS[locale.code] || ''}</span>
      <span class="language-card__label">${escapeHtml(locale.label)}</span>
    `;
    card.addEventListener('click', () => onSelect(locale.code));
    container.appendChild(card);
  }
}

async function initI18n() {
  let settings = null;
  try {
    settings = await invoke('get_settings');
  } catch (e) {
    console.error('Failed to read settings during i18n init:', e);
  }
  const stored = settings && settings.language ? settings.language : '';
  const code = stored || i18n.detectBrowserLocale();
  try {
    await i18n.loadLocale(code);
  } catch (e) {
    console.error('Failed to load locale, falling back to en:', e);
    await i18n.loadLocale(i18n.FALLBACK);
  }
  i18n.applyTranslations(document);
  return { settings, hasStored: !!stored };
}

async function showLanguagePickerIfNeeded(initSettings, hasStored) {
  if (hasStored) return;
  const picker = document.getElementById('language-picker');
  const cards = document.getElementById('language-picker-cards');
  if (!picker || !cards) return;

  await new Promise((resolve) => {
    renderLanguageCards(cards, i18n.getCurrentLocale(), async (code) => {
      try {
        const fresh = await invoke('get_settings');
        fresh.language = code;
        await invoke('save_settings', { settings: fresh });
      } catch (e) {
        console.error('Failed to save language choice:', e);
      }
      if (code !== i18n.getCurrentLocale()) {
        try { await i18n.loadLocale(code); } catch {}
        i18n.applyTranslations(document);
      }
      picker.classList.add('hidden');
      resolve();
    });

    picker.classList.remove('hidden');
  });
}

function bindSettingsLanguageCards() {
  const cards = document.getElementById('settings-language-cards');
  if (!cards) return;

  let pendingCode = i18n.getCurrentLocale();
  const initial = i18n.getCurrentLocale();

  const updateSaveLabel = () => {
    if (!els.btnSettingsSave) return;
    if (pendingCode !== initial) {
      els.btnSettingsSave.textContent = i18n.t('settings.languageRestartButton');
      els.btnSettingsSave.dataset.languageRestart = '1';
    } else {
      els.btnSettingsSave.textContent = i18n.t('settings.save');
      delete els.btnSettingsSave.dataset.languageRestart;
    }
  };

  const render = () => {
    renderLanguageCards(cards, pendingCode, async (code) => {
      pendingCode = code;
      try {
        const fresh = await invoke('get_settings');
        fresh.language = code;
        await invoke('save_settings', { settings: fresh });
      } catch (e) {
        console.error('Failed to save language:', e);
      }
      render();
      updateSaveLabel();
    });
  };

  render();
}

document.addEventListener('DOMContentLoaded', async () => {
  const { settings: initSettings, hasStored } = await initI18n();

  initTheme();
  initUpload();
  initEvents();
  loadSettingsAndDefaults();
  initTauri();
  refreshSourcesUI();
  setupModalA11y();

  bindSettingsLanguageCards();
  await showLanguagePickerIfNeeded(initSettings, hasStored);
  await initTelemetryConsent();
  setTimeout(checkForUpdates, 1500);
});
