/**
 * NodeBook OS — Modern Telegram Mini App Client
 * Fullscreen Spotify Audio Player, Creator Video Hub with Custom Player, Download Queue & Knowledge Vault
 */

document.addEventListener('DOMContentLoaded', () => {
  // ── 0. Telegram WebApp SDK Initialization ─────────────────────────────────
  if (window.Telegram && window.Telegram.WebApp) {
    try {
      window.Telegram.WebApp.ready();
      window.Telegram.WebApp.expand();
      if (window.Telegram.WebApp.setHeaderColor) {
        window.Telegram.WebApp.setHeaderColor('#090a0c');
      }
    } catch (e) {
      console.warn('Telegram WebApp init:', e);
    }
  }

  // ── 1. App State ──────────────────────────────────────────────────────────
  const state = {
    activeTab: 'music',

    // Audio Player State
    tracks: [],
    playlists: [],
    musicFilterPlaylist: null,
    musicFilterArtist: null,
    musicSearchQuery: '',
    currentTrackIndex: -1,
    isPlaying: false,
    isShuffle: false,
    isRepeat: false,

    // Media & Video State
    videos: [],
    mediaFilterPlaylist: null,
    mediaFilterAuthor: null,
    mediaSearchQuery: '',
    activeVideo: null,

    // Vault State
    notes: [],
    vaultSearchQuery: '',
    vaultViewMode: 'tree', // 'tree' | 'list'
    vaultFilterCategory: null, // null = all
    collapsedBranches: new Set(),
    isAllTreeCollapsed: false,

    // Pins (Pinterest & Photos) State
    pins: [],
    pinsSearchQuery: '',
    activePin: null,

    // Download Queue Manager
    downloadQueue: [], // { id, url, isAudio, title, status: 'pending'|'downloading'|'done'|'error', progressPct, errorMsg }
    isProcessingQueue: false,

    // Playlist Modal Target
    pendingPlaylistItemId: null,
  };

  // ── 2. DOM Elements Cache ─────────────────────────────────────────────────
  const el = {
    // Nav
    tabBtns: document.querySelectorAll('.nav-tab'),
    tabPanes: document.querySelectorAll('.tab-pane'),

    // Audio Engine & Mini Player
    audio: document.getElementById('audio-engine'),
    miniPlayer: document.getElementById('mini-player'),
    miniClickable: document.getElementById('mini-player-clickable'),
    miniProgressFill: document.getElementById('mini-progress-fill'),
    miniCoverImg: document.getElementById('mini-cover-img'),
    miniTrackTitle: document.getElementById('mini-track-title'),
    miniTrackArtist: document.getElementById('mini-track-artist'),
    miniBtnPlay: document.getElementById('mini-btn-play'),
    miniIconPlay: document.getElementById('mini-icon-play'),
    miniIconPause: document.getElementById('mini-icon-pause'),
    miniBtnNext: document.getElementById('mini-btn-next'),

    // Fullscreen Audio Player
    fsPlayer: document.getElementById('fullscreen-player'),
    fsBtnClose: document.getElementById('fs-btn-close'),
    fsCoverImg: document.getElementById('fs-cover-img'),
    fsTrackTitle: document.getElementById('fs-track-title'),
    fsTrackArtist: document.getElementById('fs-track-artist'),
    fsContextTitle: document.getElementById('fs-context-title'),
    fsBtnHeart: document.getElementById('fs-btn-heart'),
    fsBtnAddPlaylist: document.getElementById('fs-btn-add-playlist'),
    fsScrubberBar: document.getElementById('fs-scrubber-bar'),
    fsScrubberFill: document.getElementById('fs-scrubber-fill'),
    fsScrubberHandle: document.getElementById('fs-scrubber-handle'),
    fsTimeCurrent: document.getElementById('fs-time-current'),
    fsTimeTotal: document.getElementById('fs-time-total'),
    fsBtnShuffle: document.getElementById('fs-btn-shuffle'),
    fsBtnPrev: document.getElementById('fs-btn-prev'),
    fsBtnPlay: document.getElementById('fs-btn-play'),
    fsIconPlay: document.getElementById('fs-icon-play'),
    fsIconPause: document.getElementById('fs-icon-pause'),
    fsBtnNext: document.getElementById('fs-btn-next'),
    fsBtnRepeat: document.getElementById('fs-btn-repeat'),
    fsVolumeSlider: document.getElementById('fs-volume-slider'),

    // Lists & Containers
    musicTrackList: document.getElementById('music-track-list'),
    musicPlaylistChips: document.getElementById('music-playlist-chips'),
    musicSearchInput: document.getElementById('music-search-input'),
    musicArtistCard: document.getElementById('music-artist-card'),
    btnMusicArtistBack: document.getElementById('btn-music-artist-back'),
    musicArtistName: document.getElementById('music-artist-name'),

    mediaGrid: document.getElementById('media-grid'),
    mediaPlaylistChips: document.getElementById('media-playlist-chips'),
    mediaSearchInput: document.getElementById('media-search-input'),
    mediaSearchHeader: document.getElementById('media-search-header'),

    // Creator Profile View
    creatorProfileCard: document.getElementById('creator-profile-card'),
    btnCreatorBack: document.getElementById('btn-creator-back'),
    creatorAvatar: document.getElementById('creator-avatar'),
    creatorAvatarLetter: document.getElementById('creator-avatar-letter'),
    creatorName: document.getElementById('creator-name'),
    creatorStats: document.getElementById('creator-stats'),

    // Knowledge Vault Hierarchical Tree
    vaultTreeContainer: document.getElementById('vault-tree-container'),
    vaultSearchInput: document.getElementById('vault-search-input'),

    // Pins Tab & Lightbox Modal
    pinsGrid: document.getElementById('pins-grid'),
    pinsSearchInput: document.getElementById('pins-search-input'),
    pinModal: document.getElementById('pin-modal'),
    pinModalImg: document.getElementById('pin-modal-img'),
    pinModalTitle: document.getElementById('pin-modal-title'),
    btnPinDownload: document.getElementById('btn-pin-download'),
    btnPinDelete: document.getElementById('btn-pin-delete'),
    btnClosePinModal: document.getElementById('btn-close-pin-modal'),

    // FAB & Action Modal
    fabAddBtn: document.getElementById('fab-add-btn'),
    addContentModal: document.getElementById('add-content-modal'),
    btnCloseAddModal: document.getElementById('btn-close-add-modal'),
    sheetTabs: document.querySelectorAll('.sheet-tab'),
    actionTabPanes: document.querySelectorAll('.action-tab-pane'),
    actionStatusMsg: document.getElementById('action-status-msg'),

    // Action Form Fields
    addMusicUrl: document.getElementById('add-music-url'),
    btnSubmitMusicDl: document.getElementById('btn-submit-music-dl'),
    addVideoUrl: document.getElementById('add-video-url'),
    btnSubmitVideoDl: document.getElementById('btn-submit-video-dl'),
    uploadDropzone: document.getElementById('upload-dropzone'),
    localFileInput: document.getElementById('local-file-input'),
    btnTriggerFilePick: document.getElementById('btn-trigger-file-pick'),
    newPlaylistName: document.getElementById('new-playlist-name'),
    btnSubmitCreatePl: document.getElementById('btn-submit-create-pl'),

    // Download Queue & History
    downloadQueueSection: document.getElementById('download-queue-section'),
    queueTaskList: document.getElementById('queue-task-list'),
    queueCountBadge: document.getElementById('queue-count-badge'),
    btnOpenHistoryModal: document.getElementById('btn-open-history-modal'),
    downloadHistoryModal: document.getElementById('download-history-modal'),
    btnCloseHistoryModal: document.getElementById('btn-close-history-modal'),
    btnClearHistory: document.getElementById('btn-clear-history'),
    historyTaskList: document.getElementById('history-task-list'),
    historyCountBadge: document.getElementById('history-count-badge'),

    // Custom Video Player & Modal
    videoPlayerModal: document.getElementById('video-player-modal'),
    btnCloseVideoModal: document.getElementById('btn-close-video-modal'),
    customVideoWrapper: document.getElementById('custom-video-wrapper'),
    html5Video: document.getElementById('html5-video-player'),
    videoCenterOverlay: document.getElementById('video-center-overlay'),
    videoBigPlayBtn: document.getElementById('video-big-play-btn'),
    cvIconBigPlay: document.getElementById('cv-icon-big-play'),
    cvIconBigPause: document.getElementById('cv-icon-big-pause'),
    videoControlsBar: document.getElementById('video-controls-bar'),
    videoScrubberTrack: document.getElementById('video-scrubber-track'),
    videoScrubberFill: document.getElementById('video-scrubber-fill'),
    videoScrubberHandle: document.getElementById('video-scrubber-handle'),
    cvBtnPlay: document.getElementById('cv-btn-play'),
    cvIconPlay: document.getElementById('cv-icon-play'),
    cvIconPause: document.getElementById('cv-icon-pause'),
    cvTimeCurrent: document.getElementById('cv-time-current'),
    cvTimeTotal: document.getElementById('cv-time-total'),
    cvBtnPip: document.getElementById('cv-btn-pip'),
    cvBtnFullscreen: document.getElementById('cv-btn-fullscreen'),

    // Video Modal Meta & Actions
    vmTitle: document.getElementById('vm-title'),
    vmAuthorBtn: document.getElementById('vm-author-btn'),
    vmAuthorName: document.getElementById('vm-author-name'),
    vmBtnHeart: document.getElementById('vm-btn-heart'),
    vmBtnAddPl: document.getElementById('vm-btn-add-pl'),
    vmBtnDelete: document.getElementById('vm-btn-delete'),
    vmMoreTitle: document.getElementById('vm-more-title'),
    vmMoreCarousel: document.getElementById('vm-more-carousel'),

    // Properties Modal
    notePropertiesModal: document.getElementById('note-properties-modal'),
    btnClosePropertiesModal: document.getElementById('btn-close-properties-modal'),
    propertiesContent: document.getElementById('properties-content'),

    // Note Document Reader Modal
    noteReaderModal: document.getElementById('note-reader-modal'),
    btnCloseReaderModal: document.getElementById('btn-close-reader-modal'),
    btnReaderProp: document.getElementById('btn-reader-prop'),
    readerCrumbFolder: document.getElementById('reader-crumb-folder'),
    readerCrumbFile: document.getElementById('reader-crumb-file'),
    readerTitle: document.getElementById('reader-title'),
    readerMetaRow: document.getElementById('reader-meta-row'),
    readerSummaryCard: document.getElementById('reader-summary-card'),
    readerSummaryText: document.getElementById('reader-summary-text'),
    readerContent: document.getElementById('reader-content'),

    // Playlist Picker Modal
    playlistPickerModal: document.getElementById('playlist-picker-modal'),
    btnClosePickerModal: document.getElementById('btn-close-picker-modal'),
    pickerPlaylistList: document.getElementById('picker-playlist-list'),
  };

  // ── 2.1 Telegram WebApp Authentication & Security ─────────────────────────
  function resolveTelegramAuth() {
    if (window.Telegram && window.Telegram.WebApp && window.Telegram.WebApp.initData) {
      return window.Telegram.WebApp.initData;
    }
    const urlParams = new URLSearchParams(window.location.search);
    const fromSearch = urlParams.get('initData') || urlParams.get('tgWebAppData') || urlParams.get('token');
    if (fromSearch) return fromSearch;

    const rawHash = window.location.hash.replace(/^#/, '');
    if (rawHash) {
      const hashParams = new URLSearchParams(rawHash);
      const fromHash = hashParams.get('tgWebAppData') || hashParams.get('initData') || hashParams.get('token');
      if (fromHash) return fromHash;
    }

    return localStorage.getItem('nb_tg_init_data') || sessionStorage.getItem('nb_tg_init_data') || 'desktop_5887915765_owner';
  }

  let tgInitData = resolveTelegramAuth();
  if (tgInitData) {
    localStorage.setItem('nb_tg_init_data', tgInitData);
    sessionStorage.setItem('nb_tg_init_data', tgInitData);
  }

  function showSecurityLockdown(errorMsg) {
    const secScreen = document.getElementById('access-denied-screen');
    const appContainer = document.getElementById('app-container');
    const secUserInfo = document.getElementById('security-user-info');

    if (appContainer) appContainer.style.display = 'none';
    if (secScreen) {
      secScreen.style.display = 'flex';
      let userDetails = 'Telegram ID: Не авторизован';
      try {
        if (window.Telegram?.WebApp?.initDataUnsafe?.user) {
          const u = window.Telegram.WebApp.initDataUnsafe.user;
          userDetails = `Telegram ID: ${u.id} (@${u.username || u.first_name || 'user'}) — Доступ запрещён`;
        }
      } catch (e) {}
      if (secUserInfo) {
        secUserInfo.textContent = errorMsg ? `${userDetails} (${errorMsg})` : userDetails;
      }
    }
  }

  async function authFetch(url, options = {}) {
    options.headers = options.headers || {};
    if (tgInitData) {
      options.headers['X-Telegram-Init-Data'] = tgInitData;
    }
    const res = await fetch(url, options);
    if (res.status === 401 || res.status === 403) {
      let errMsg = '';
      try {
        const data = await res.json();
        errMsg = data.error || '';
      } catch {}
      showSecurityLockdown(errMsg);
      throw new Error(`Auth failed: HTTP ${res.status} ${errMsg}`);
    }
    return res;
  }

  function authMediaUrl(url) {
    if (!tgInitData) return url;
    const separator = url.includes('?') ? '&' : '?';
    return `${url}${separator}initData=${encodeURIComponent(tgInitData)}`;
  }

  // ── 3. Utility Helpers ────────────────────────────────────────────────────
  function formatTime(seconds) {
    if (isNaN(seconds) || seconds < 0) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function escapeHtml(str) {
    if (!str) return '';
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  function showStatus(text, type = 'success') {
    el.actionStatusMsg.textContent = text;
    el.actionStatusMsg.className = `action-status-msg ${type}`;
    el.actionStatusMsg.style.display = 'block';
    setTimeout(() => {
      el.actionStatusMsg.style.display = 'none';
    }, 4500);
  }

  // ── 4. URL Validation Helpers ─────────────────────────────────────────────
  function isPureVideoUrl(url) {
    const u = url.toLowerCase();
    return u.includes('tiktok.com') ||
           u.includes('instagram.com/reel') ||
           u.includes('instagram.com/p/') ||
           u.includes('pinterest.com') ||
           u.includes('youtube.com/shorts') ||
           u.includes('youtu.be') ||
           u.includes('youtube.com/watch');
  }

  function isAudioResource(url) {
    const u = url.toLowerCase();
    return u.includes('spotify.com') ||
           u.includes('soundcloud.com') ||
           u.includes('music.youtube.com') ||
           u.endsWith('.mp3') ||
           u.endsWith('.wav') ||
           u.endsWith('.flac') ||
           u.endsWith('.m4a');
  }

  // ── 5. Data Fetching ──────────────────────────────────────────────────────
  async function fetchAllData() {
    try {
      // 1. Verify Authentication First
      await authFetch('/api/auth/verify');

      // 2. Load Core Data
      await Promise.all([
        fetchTracks(),
        fetchVideos(),
        fetchPins(),
        fetchPlaylists(),
        fetchNotes(),
      ]);
    } catch (e) {
      console.warn('App initialization stopped by security policy:', e);
    }
  }

  async function fetchTracks() {
    try {
      const res = await authFetch('/api/player/tracks');
      if (res.ok) {
        state.tracks = await res.json();
        renderMusicTab();
        renderPlaylistChips();
      }
    } catch (e) {
      console.error('Fetch tracks failed:', e);
    }
  }

  async function fetchVideos() {
    try {
      const res = await authFetch('/api/media/videos');
      if (res.ok) {
        state.videos = await res.json();
        renderMediaTab();
        renderPlaylistChips();
      }
    } catch (e) {
      console.error('Fetch videos failed:', e);
    }
  }

  async function fetchPins() {
    try {
      const res = await authFetch('/api/media/pins');
      if (res.ok) {
        state.pins = await res.json();
        renderPinsTab();
      }
    } catch (e) {
      console.error('Fetch pins failed:', e);
    }
  }

  async function fetchPlaylists() {
    try {
      const res = await authFetch('/api/playlists');
      if (res.ok) {
        state.playlists = await res.json();
        renderPlaylistChips();
      }
    } catch (e) {
      console.error('Fetch playlists failed:', e);
    }
  }

  async function fetchNotes() {
    try {
      const res = await authFetch('/api/vault/notes');
      if (res.ok) {
        state.notes = await res.json();
        renderVaultTab();
      }
    } catch (e) {
      console.error('Fetch notes failed:', e);
    }
  }

  // ── 6. Download Queue & History System ───────────────────────────────────
  function getStoredHistory() {
    try {
      return JSON.parse(localStorage.getItem('nb_dl_history') || '[]');
    } catch {
      return [];
    }
  }

  function appendToHistory(item) {
    const history = getStoredHistory();
    history.unshift(item);
    // Keep last 50 downloads
    if (history.length > 50) history.pop();
    localStorage.setItem('nb_dl_history', JSON.stringify(history));
    renderDownloadHistory();
  }

  function renderDownloadHistory() {
    const history = getStoredHistory();
    el.historyCountBadge.textContent = `${history.length} файлов`;

    if (history.length === 0) {
      el.historyTaskList.innerHTML = '<p class="action-desc" style="text-align: center; padding: 24px 0;">История загрузок пуста</p>';
      return;
    }

    el.historyTaskList.innerHTML = history.map(item => `
      <div class="history-item">
        <div class="history-item-top">
          <span class="history-item-title">${escapeHtml(item.title)}</span>
          <span class="history-tag ${item.isAudio ? 'audio' : 'video'}">${item.isAudio ? 'Аудио' : 'Видео'}</span>
        </div>
        <div class="history-item-meta">
          <span>${escapeHtml(item.time || '')}</span>
          <span style="color: ${item.status === 'done' ? '#10b981' : '#ef4444'}">${item.status === 'done' ? '✓ Загружено' : '✕ Ошибка'}</span>
        </div>
      </div>
    `).join('');
  }

  function enqueueDownload(url, isAudio) {
    const taskId = 'task_' + Math.random().toString(36).substr(2, 9);
    state.downloadQueue.push({
      id: taskId,
      url,
      isAudio,
      title: url.replace(/^https?:\/\/(www\.)?/, '').substring(0, 38),
      status: 'pending',
      progressPct: 0,
      errorMsg: null,
    });

    renderDownloadQueue();
    processDownloadQueue();
  }

  async function processDownloadQueue() {
    if (state.isProcessingQueue) return;
    state.isProcessingQueue = true;

    while (true) {
      const task = state.downloadQueue.find(t => t.status === 'pending');
      if (!task) break;

      task.status = 'downloading';
      task.progressPct = 5;
      task.statusText = 'Инициализация загрузки...';
      renderDownloadQueue();

      const startTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

      try {
        const startRes = await authFetch('/api/media/download', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ url: task.url, is_audio: task.isAudio }),
        });

        if (!startRes.ok) throw new Error('Ошибка запуска задачи');

        const { task_id } = await startRes.json();

        // Poll task status until done or error
        let isDone = false;
        while (!isDone) {
          await new Promise(r => setTimeout(r, 450));
          const statusRes = await authFetch(`/api/media/download/task/${task_id}`);
          if (!statusRes.ok) break;

          const data = await statusRes.json();
          task.progressPct = data.percent || task.progressPct;

          if (data.title && data.title !== task.title) {
            task.title = data.title;
          }

          if (data.is_playlist) {
            if (data.current_track) {
              const artistStr = data.current_artist ? ` — ${data.current_artist}` : '';
              task.statusText = `[${data.completed_tracks + 1}/${data.total_tracks}] ${data.current_track}${artistStr}`;
            } else {
              task.statusText = `Плейлист: скачано ${data.completed_tracks} из ${data.total_tracks}`;
            }
          } else {
            task.statusText = data.status === 'downloading' ? 'Скачивание и конвертация...' : 'Обработка...';
          }

          renderDownloadQueue();

          if (data.status === 'done') {
            isDone = true;
            task.progressPct = 100;
            task.status = 'done';
            task.statusText = data.is_playlist ? `Завершено (${data.total_tracks} треков)` : 'Загрузка завершена';
            renderDownloadQueue();

            await new Promise(r => setTimeout(r, 400));

            appendToHistory({
              id: task.id,
              title: task.title,
              url: task.url,
              isAudio: task.isAudio,
              time: startTime,
              status: 'done',
            });

            state.downloadQueue = state.downloadQueue.filter(t => t.id !== task.id);
            await fetchAllData();
          } else if (data.status === 'error') {
            throw new Error(data.error || 'Ошибка загрузки');
          }
        }
      } catch (err) {
        task.progressPct = 100;
        task.status = 'error';
        task.statusText = err.message || 'Ошибка загрузки';
        renderDownloadQueue();

        await new Promise(r => setTimeout(r, 800));

        appendToHistory({
          id: task.id,
          title: task.title,
          url: task.url,
          isAudio: task.isAudio,
          time: startTime,
          status: 'error',
        });

        state.downloadQueue = state.downloadQueue.filter(t => t.id !== task.id);
      }

      renderDownloadQueue();
    }

    state.isProcessingQueue = false;
  }

  function renderDownloadQueue() {
    const activeTasks = state.downloadQueue.filter(t => t.status === 'pending' || t.status === 'downloading' || t.status === 'done');

    // Auto-hide queue if no active tasks
    if (activeTasks.length === 0) {
      if (el.downloadQueueSection) el.downloadQueueSection.style.display = 'none';
      return;
    }

    if (el.downloadQueueSection) el.downloadQueueSection.style.display = 'flex';
    el.queueCountBadge.textContent = `${activeTasks.length} в процессе`;

    el.queueTaskList.innerHTML = activeTasks.map(t => {
      let statusLabel = 'В очереди';
      if (t.status === 'downloading') statusLabel = 'Загрузка';
      else if (t.status === 'done') statusLabel = 'Готово';
      else if (t.status === 'error') statusLabel = 'Ошибка';

      return `
        <div class="queue-item" data-id="${t.id}">
          <div class="queue-item-top">
            <span class="queue-item-url">${escapeHtml(t.title)}</span>
            <div class="queue-status-wrap">
              <span class="queue-pct">${t.progressPct}%</span>
              <span class="queue-item-status ${t.status}">${statusLabel}</span>
            </div>
          </div>
          ${t.statusText ? `<div class="queue-item-sub" style="font-size: 11px; color: var(--text-muted); margin-top: 3px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(t.statusText)}</div>` : ''}
          <div class="queue-progress-bar" style="margin-top: 6px;">
            <div class="queue-progress-fill" style="width: ${t.progressPct}%"></div>
          </div>
        </div>
      `;
    }).join('');
  }

  // History Modal Handlers
  el.btnOpenHistoryModal.addEventListener('click', () => {
    renderDownloadHistory();
    el.downloadHistoryModal.style.display = 'flex';
  });

  el.btnCloseHistoryModal.addEventListener('click', () => {
    el.downloadHistoryModal.style.display = 'none';
  });

  el.btnClearHistory.addEventListener('click', () => {
    localStorage.removeItem('nb_dl_history');
    renderDownloadHistory();
  });

  // Submit Music Download
  el.btnSubmitMusicDl.addEventListener('click', () => {
    const raw = el.addMusicUrl.value.trim();
    if (!raw) return;

    const urls = raw.split(/\s+/).filter(u => u.startsWith('http'));
    if (urls.length === 0) {
      showStatus('Введите корректную ссылку', 'error');
      return;
    }

    for (const url of urls) {
      if (isPureVideoUrl(url) && !url.includes('spotify.com') && !url.includes('soundcloud.com')) {
        showStatus('⚠️ Эта ссылка содержит видео. Используйте вкладку «Видео»', 'error');
        return;
      }
      enqueueDownload(url, true);
    }

    showStatus('Добавлено в очередь загрузки!', 'success');
    el.addMusicUrl.value = '';
  });

  // Submit Video Download
  el.btnSubmitVideoDl.addEventListener('click', () => {
    const raw = el.addVideoUrl.value.trim();
    if (!raw) return;

    const urls = raw.split(/\s+/).filter(u => u.startsWith('http'));
    if (urls.length === 0) {
      showStatus('Введите корректную ссылку', 'error');
      return;
    }

    for (const url of urls) {
      if (url.includes('spotify.com') || url.includes('soundcloud.com')) {
        showStatus('⚠️ Spotify и SoundCloud не содержат видео. Используйте вкладку «Музыка»', 'error');
        return;
      }
      enqueueDownload(url, false);
    }

    showStatus('Добавлено в очередь загрузки!', 'success');
    el.addVideoUrl.value = '';
  });

  // ── 7. Music Player Logic ─────────────────────────────────────────────────
  function getFilteredTracks() {
    let list = [...state.tracks];

    if (state.musicFilterPlaylist) {
      const pl = state.playlists.find(p => p.id === state.musicFilterPlaylist);
      if (pl) {
        list = list.filter(t => pl.item_ids.includes(t.id));
      }
    }

    if (state.musicFilterArtist) {
      list = list.filter(t => (t.uploader || '').toLowerCase() === state.musicFilterArtist.toLowerCase());
    }

    if (state.musicSearchQuery.trim()) {
      const q = state.musicSearchQuery.toLowerCase();
      list = list.filter(t =>
        t.title.toLowerCase().includes(q) ||
        (t.uploader && t.uploader.toLowerCase().includes(q))
      );
    }

    return list;
  }

  function playTrackByIndex(index) {
    const list = getFilteredTracks();
    if (index < 0 || index >= list.length) return;

    // Pause video if playing
    if (!el.html5Video.paused) {
      el.html5Video.pause();
      updateVideoPlayState(false);
    }

    state.currentTrackIndex = index;
    const track = list[index];
    const streamUrl = authMediaUrl(`/api/player/stream/${track.id}`);
    const coverUrl = authMediaUrl(`/api/player/cover/${track.id}`);

    el.audio.src = streamUrl;
    el.audio.play().then(() => {
      state.isPlaying = true;
      updateAudioPlayerUI(track, coverUrl);
    }).catch(err => {
      console.warn('Playback error:', err);
    });
  }

  function toggleAudioPlayPause() {
    if (!el.audio.src || state.currentTrackIndex === -1) {
      const list = getFilteredTracks();
      if (list.length > 0) playTrackByIndex(0);
      return;
    }

    if (el.audio.paused) {
      // Pause video if playing
      if (!el.html5Video.paused) {
        el.html5Video.pause();
        updateVideoPlayState(false);
      }
      el.audio.play();
      state.isPlaying = true;
    } else {
      el.audio.pause();
      state.isPlaying = false;
    }
    updateAudioPlayPauseIcons();
  }

  function playNextAudioTrack() {
    const list = getFilteredTracks();
    if (list.length === 0) return;

    if (state.isShuffle) {
      const nextIdx = Math.floor(Math.random() * list.length);
      playTrackByIndex(nextIdx);
    } else {
      let nextIdx = state.currentTrackIndex + 1;
      if (nextIdx >= list.length) nextIdx = 0;
      playTrackByIndex(nextIdx);
    }
  }

  function playPrevAudioTrack() {
    const list = getFilteredTracks();
    if (list.length === 0) return;

    if (el.audio.currentTime > 3) {
      el.audio.currentTime = 0;
      return;
    }

    let prevIdx = state.currentTrackIndex - 1;
    if (prevIdx < 0) prevIdx = list.length - 1;
    playTrackByIndex(prevIdx);
  }

  function updateAudioPlayerUI(track, coverUrl) {
    el.miniPlayer.style.display = 'flex';
    el.miniTrackTitle.textContent = track.title;
    el.miniTrackArtist.textContent = track.uploader || 'Исполнитель';
    el.miniCoverImg.src = coverUrl;

    el.fsTrackTitle.textContent = track.title;
    el.fsTrackArtist.textContent = track.uploader || 'Исполнитель';
    el.fsCoverImg.src = coverUrl;

    if (state.musicFilterPlaylist) {
      const pl = state.playlists.find(p => p.id === state.musicFilterPlaylist);
      el.fsContextTitle.textContent = pl ? pl.name : 'Плейлист';
    } else {
      el.fsContextTitle.textContent = 'Медиатека';
    }

    const favPl = state.playlists.find(p => p.id === 'favorites_audio');
    if (favPl && favPl.item_ids.includes(track.id)) {
      el.fsBtnHeart.classList.add('active');
    } else {
      el.fsBtnHeart.classList.remove('active');
    }

    updateAudioPlayPauseIcons();
    highlightActiveTrackCard();
  }

  function updateAudioPlayPauseIcons() {
    if (state.isPlaying) {
      el.miniIconPlay.style.display = 'none';
      el.miniIconPause.style.display = 'block';
      el.fsIconPlay.style.display = 'none';
      el.fsIconPause.style.display = 'block';
    } else {
      el.miniIconPlay.style.display = 'block';
      el.miniIconPause.style.display = 'none';
      el.fsIconPlay.style.display = 'block';
      el.fsIconPause.style.display = 'none';
    }
  }

  function highlightActiveTrackCard() {
    const list = getFilteredTracks();
    const currentTrack = list[state.currentTrackIndex];
    document.querySelectorAll('.track-card').forEach(card => {
      if (currentTrack && card.dataset.id === currentTrack.id) {
        card.classList.add('active');
      } else {
        card.classList.remove('active');
      }
    });
  }

  // Audio Engine Events
  el.audio.addEventListener('timeupdate', () => {
    const current = el.audio.currentTime;
    const total = el.audio.duration || 0;
    const pct = total > 0 ? (current / total) * 100 : 0;

    el.miniProgressFill.style.width = `${pct}%`;
    el.fsScrubberFill.style.width = `${pct}%`;
    el.fsScrubberHandle.style.left = `${pct}%`;
    el.fsTimeCurrent.textContent = formatTime(current);
    el.fsTimeTotal.textContent = formatTime(total);
  });

  el.audio.addEventListener('ended', () => {
    if (state.isRepeat) {
      el.audio.currentTime = 0;
      el.audio.play();
    } else {
      playNextAudioTrack();
    }
  });

  el.audio.addEventListener('play', () => {
    state.isPlaying = true;
    updateAudioPlayPauseIcons();
  });

  el.audio.addEventListener('pause', () => {
    state.isPlaying = false;
    updateAudioPlayPauseIcons();
  });

  // Fullscreen open & close
  el.miniClickable.addEventListener('click', () => {
    el.fsPlayer.classList.add('active');
  });

  el.fsBtnClose.addEventListener('click', () => {
    el.fsPlayer.classList.remove('active');
  });

  // Audio Scrubber
  el.fsScrubberBar.addEventListener('click', (e) => {
    const rect = el.fsScrubberBar.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const pct = Math.max(0, Math.min(1, clickX / rect.width));
    if (el.audio.duration) {
      el.audio.currentTime = pct * el.audio.duration;
    }
  });

  // Audio Controls
  el.miniBtnPlay.addEventListener('click', (e) => {
    e.stopPropagation();
    toggleAudioPlayPause();
  });

  el.miniBtnNext.addEventListener('click', (e) => {
    e.stopPropagation();
    playNextAudioTrack();
  });

  el.fsBtnPlay.addEventListener('click', toggleAudioPlayPause);
  el.fsBtnNext.addEventListener('click', playNextAudioTrack);
  el.fsBtnPrev.addEventListener('click', playPrevAudioTrack);

  el.fsBtnShuffle.addEventListener('click', () => {
    state.isShuffle = !state.isShuffle;
    el.fsBtnShuffle.classList.toggle('active', state.isShuffle);
  });

  el.fsBtnRepeat.addEventListener('click', () => {
    state.isRepeat = !state.isRepeat;
    el.fsBtnRepeat.classList.toggle('active', state.isRepeat);
  });

  el.fsVolumeSlider.addEventListener('input', (e) => {
    el.audio.volume = parseFloat(e.target.value);
  });

  el.fsBtnHeart.addEventListener('click', async () => {
    const list = getFilteredTracks();
    const track = list[state.currentTrackIndex];
    if (!track) return;

    await toggleFavoriteTrack(track.id);
  });

  el.fsBtnAddPlaylist.addEventListener('click', () => {
    const list = getFilteredTracks();
    const track = list[state.currentTrackIndex];
    if (track) openPlaylistPicker(track.id, 'audio');
  });

  async function toggleFavoriteTrack(trackId) {
    const favPl = state.playlists.find(p => p.id === 'favorites_audio');
    if (!favPl) return;

    const isFav = favPl.item_ids.includes(trackId);
    if (isFav) {
      await authFetch(`/api/playlists/${favPl.id}/items/${trackId}`, { method: 'DELETE' });
      favPl.item_ids = favPl.item_ids.filter(id => id !== trackId);
      el.fsBtnHeart.classList.remove('active');
    } else {
      await authFetch(`/api/playlists/${favPl.id}/items`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ item_id: trackId }),
      });
      favPl.item_ids.push(trackId);
      el.fsBtnHeart.classList.add('active');
    }
    renderMusicTab();
  }

  // ── 8. Render Music Tab ───────────────────────────────────────────────────
  function renderMusicTab() {
    const list = getFilteredTracks();
    const favPl = state.playlists.find(p => p.id === 'favorites_audio');

    if (state.musicFilterArtist && el.musicArtistCard) {
      el.musicArtistCard.style.display = 'flex';
      if (el.musicArtistName) el.musicArtistName.textContent = `👤 ${state.musicFilterArtist}`;
      if (el.musicPlaylistChips) el.musicPlaylistChips.style.display = 'none';
    } else if (el.musicArtistCard) {
      el.musicArtistCard.style.display = 'none';
      if (el.musicPlaylistChips) el.musicPlaylistChips.style.display = 'flex';
    }

    if (list.length === 0) {
      el.musicTrackList.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">
            <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M9 18V5l12-2v13"></path>
              <circle cx="6" cy="18" r="3"></circle>
              <circle cx="18" cy="16" r="3"></circle>
            </svg>
          </div>
          <p class="empty-title">Треки не найдены</p>
          <p class="empty-subtitle">Нажмите + для добавления музыки или сбросьте фильтр</p>
        </div>
      `;
      return;
    }

    el.musicTrackList.innerHTML = list.map((track, idx) => {
      const coverUrl = authMediaUrl(`/api/player/cover/${track.id}`);
      const durationStr = track.duration_secs ? formatTime(track.duration_secs) : '3:20';
      const isCurrent = state.currentTrackIndex === idx;
      const isFav = favPl && favPl.item_ids.includes(track.id);
      const artist = track.uploader || 'Исполнитель';

      return `
        <div class="track-card ${isCurrent ? 'active' : ''}" data-id="${escapeHtml(track.id)}" data-index="${idx}">
          <div class="track-cover-wrap">
            <img src="${coverUrl}" alt="Cover" loading="lazy">
            <div class="track-play-overlay">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
            </div>
          </div>
          <div class="track-info">
            <div class="track-title">${escapeHtml(track.title)}</div>
            <button class="track-artist-badge" data-artist="${escapeHtml(artist)}" title="Все треки исполнителя">
              ${escapeHtml(artist)}
            </button>
          </div>
          <div class="track-right-meta">
            <span class="track-duration">${durationStr}</span>
            <button class="track-menu-btn favorite-btn ${isFav ? 'active' : ''}" data-action="fav" data-id="${escapeHtml(track.id)}" title="В любимые">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="${isFav ? 'currentColor' : 'none'}" stroke="currentColor" stroke-width="2">
                <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"></path>
              </svg>
            </button>
            <button class="track-menu-btn" data-action="add-pl" data-id="${escapeHtml(track.id)}" title="В плейлист">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
              </svg>
            </button>
            <button class="track-menu-btn" data-action="delete" data-id="${escapeHtml(track.id)}" title="Удалить">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
            </button>
          </div>
        </div>
      `;
    }).join('');

    el.musicTrackList.querySelectorAll('.track-card').forEach(card => {
      card.addEventListener('click', (e) => {
        if (e.target.closest('.track-menu-btn') || e.target.closest('.track-artist-badge')) return;
        const idx = parseInt(card.dataset.index, 10);
        playTrackByIndex(idx);
      });
    });

    el.musicTrackList.querySelectorAll('.track-artist-badge').forEach(badge => {
      badge.addEventListener('click', (e) => {
        e.stopPropagation();
        state.musicFilterArtist = badge.dataset.artist;
        renderMusicTab();
      });
    });

    el.musicTrackList.querySelectorAll('.track-menu-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const action = btn.dataset.action;
        const id = btn.dataset.id;

        if (action === 'fav') {
          await toggleFavoriteTrack(id);
        } else if (action === 'add-pl') {
          openPlaylistPicker(id, 'audio');
        } else if (action === 'delete') {
          if (confirm('Удалить этот трек из медиатеки?')) {
            await authFetch(`/api/media/${id}`, { method: 'DELETE' });
            await fetchAllData();
          }
        }
      });
    });
  }

  if (el.btnMusicArtistBack) {
    el.btnMusicArtistBack.addEventListener('click', () => {
      state.musicFilterArtist = null;
      renderMusicTab();
    });
  }

  // ── 9. Render Media Hub Tab & Creator Profile ──────────────────────────────
  function getFilteredVideos() {
    let list = [...state.videos];

    if (state.mediaFilterPlaylist) {
      const pl = state.playlists.find(p => p.id === state.mediaFilterPlaylist);
      if (pl) {
        list = list.filter(v => pl.item_ids.includes(v.id));
      }
    }

    if (state.mediaFilterAuthor) {
      list = list.filter(v => (v.uploader || '').toLowerCase() === state.mediaFilterAuthor.toLowerCase());
    }

    if (state.mediaSearchQuery.trim()) {
      const q = state.mediaSearchQuery.toLowerCase();
      list = list.filter(v =>
        v.title.toLowerCase().includes(q) ||
        (v.uploader && v.uploader.toLowerCase().includes(q))
      );
    }

    return list;
  }

  function renderMediaTab() {
    const list = getFilteredVideos();
    const favPl = state.playlists.find(p => p.id === 'favorites_video');

    // Render Creator Profile Card if author is selected
    if (state.mediaFilterAuthor) {
      const authorVideos = state.videos.filter(v => (v.uploader || '').toLowerCase() === state.mediaFilterAuthor.toLowerCase());
      const authorName = state.mediaFilterAuthor;
      const initial = (authorName.replace(/^@/, '')[0] || 'A').toUpperCase();

      el.creatorAvatarLetter.textContent = initial;
      el.creatorName.textContent = authorName;
      el.creatorStats.textContent = `${authorVideos.length} видео сохранено в NodeBook`;
      el.creatorProfileCard.style.display = 'flex';

      // Hide global search and playlist chips for a clean channel view
      if (el.mediaSearchHeader) el.mediaSearchHeader.style.display = 'none';
      if (el.mediaPlaylistChips) el.mediaPlaylistChips.style.display = 'none';
    } else {
      el.creatorProfileCard.style.display = 'none';
      if (el.mediaSearchHeader) el.mediaSearchHeader.style.display = 'flex';
      if (el.mediaPlaylistChips) el.mediaPlaylistChips.style.display = 'flex';
    }

    if (list.length === 0) {
      el.mediaGrid.innerHTML = `
        <div class="empty-state" style="grid-column: 1 / -1;">
          <div class="empty-icon">
            <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5">
              <polygon points="23 7 16 12 23 17 23 7"></polygon>
              <rect x="1" y="5" width="15" height="14" rx="2" ry="2"></rect>
            </svg>
          </div>
          <p class="empty-title">Видео не найдены</p>
          <p class="empty-subtitle">Нажмите + для добавления видео или сбросьте фильтр</p>
        </div>
      `;
      return;
    }

    el.mediaGrid.innerHTML = list.map(item => {
      const thumbUrl = authMediaUrl(`/api/media/thumb/${item.id}`);
      const durationStr = item.duration_secs ? formatTime(item.duration_secs) : '0:45';
      const uploaderName = item.uploader || 'Автор';
      const isFav = favPl && favPl.item_ids.includes(item.id);

      let platform = 'VIDEO';
      if (item.source_url.includes('tiktok')) platform = 'TIKTOK';
      else if (item.source_url.includes('youtu')) platform = 'YOUTUBE';
      else if (item.source_url.includes('instagram')) platform = 'REELS';

      return `
        <div class="video-card" data-id="${escapeHtml(item.id)}">
          <div class="video-thumb-wrap">
            <img src="${thumbUrl}" alt="Thumbnail" loading="lazy">
            <span class="video-platform-tag">${platform}</span>
            <span class="video-duration-tag">${durationStr}</span>
          </div>
          <div class="video-card-body">
            <div class="video-card-title">${escapeHtml(item.title)}</div>
            <div class="video-card-footer">
              ${state.mediaFilterAuthor ? `
                <span class="video-card-date">${escapeHtml(item.created_at.split(' ')[0])}</span>
              ` : `
                <button class="video-author-badge" data-author="${escapeHtml(uploaderName)}" title="Профиль автора">
                  ${escapeHtml(uploaderName)}
                </button>
              `}
              <div class="video-card-actions">
                <button class="video-action-btn ${isFav ? 'fav-active' : ''}" data-action="fav" data-id="${escapeHtml(item.id)}" title="Любимое видео">
                  <svg viewBox="0 0 24 24" width="15" height="15" fill="${isFav ? '#f43f5e' : 'none'}" stroke="currentColor" stroke-width="2">
                    <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"></path>
                  </svg>
                </button>
                <button class="video-action-btn" data-action="add-pl" data-id="${escapeHtml(item.id)}" title="В плейлист">
                  <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
                  </svg>
                </button>
                <button class="video-action-btn del-btn" data-action="del" data-id="${escapeHtml(item.id)}" title="Удалить видео">
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      `;
    }).join('');

    // Attach Video Card Clicks
    el.mediaGrid.querySelectorAll('.video-card').forEach(card => {
      card.addEventListener('click', (e) => {
        if (e.target.closest('.video-author-badge') || e.target.closest('.video-action-btn')) return;
        const id = card.dataset.id;
        const item = state.videos.find(v => v.id === id);
        if (item) openVideoPlayerModal(item);
      });
    });

    // Author Badge Click -> Open Creator Profile View
    el.mediaGrid.querySelectorAll('.video-author-badge').forEach(badge => {
      badge.addEventListener('click', (e) => {
        e.stopPropagation();
        state.mediaFilterAuthor = badge.dataset.author;
        renderMediaTab();
      });
    });

    // Card Action Buttons
    el.mediaGrid.querySelectorAll('.video-action-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const action = btn.dataset.action;
        const id = btn.dataset.id;

        if (action === 'fav') {
          await toggleFavoriteVideo(id);
        } else if (action === 'add-pl') {
          openPlaylistPicker(id, 'video');
        } else if (action === 'del') {
          if (confirm('Удалить это видео из медиатеки?')) {
            await authFetch(`/api/media/${id}`, { method: 'DELETE' });
            await fetchVideos();
          }
        }
      });
    });
  }

  el.btnCreatorBack.addEventListener('click', () => {
    state.mediaFilterAuthor = null;
    renderMediaTab();
  });

  async function toggleFavoriteVideo(videoId) {
    const favPl = state.playlists.find(p => p.id === 'favorites_video');
    if (!favPl) return;

    const isFav = favPl.item_ids.includes(videoId);
    if (isFav) {
      await authFetch(`/api/playlists/${favPl.id}/items/${videoId}`, { method: 'DELETE' });
      favPl.item_ids = favPl.item_ids.filter(id => id !== videoId);
      el.vmBtnHeart.classList.remove('active');
    } else {
      await authFetch(`/api/playlists/${favPl.id}/items`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ item_id: videoId }),
      });
      favPl.item_ids.push(videoId);
      el.vmBtnHeart.classList.add('active');
    }
    renderMediaTab();
  }

  // ── 9.1 Pins Tab (Pinterest / Photo Gallery) ──────────────────────────────
  function renderPinsTab() {
    if (!el.pinsGrid) return;
    let list = state.pins;

    if (state.pinsSearchQuery.trim()) {
      const q = state.pinsSearchQuery.toLowerCase();
      list = list.filter(p => p.title.toLowerCase().includes(q) || (p.uploader && p.uploader.toLowerCase().includes(q)));
    }

    if (list.length === 0) {
      el.pinsGrid.innerHTML = `
        <div class="empty-state" style="grid-column: 1 / -1;">
          <div class="empty-icon">
            <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              <circle cx="8.5" cy="8.5" r="1.5"></circle>
              <polyline points="21 15 16 10 5 21"></polyline>
            </svg>
          </div>
          <p class="empty-title">Пинов пока нет</p>
          <p class="empty-subtitle">Отправьте боту ссылку на Pinterest или фото-пост, чтобы сохранить в Pin</p>
        </div>
      `;
      return;
    }

    el.pinsGrid.innerHTML = list.map(item => {
      const thumbUrl = authMediaUrl(`/api/media/thumb/${item.id}`);
      return `
        <div class="pin-card" data-id="${escapeHtml(item.id)}">
          <div class="pin-img-wrap">
            <img src="${thumbUrl}" alt="Pin image" loading="lazy">
            <div class="pin-overlay">
              <span class="pin-tag">PIN</span>
              <button class="pin-action-btn del-btn" data-action="del" data-id="${escapeHtml(item.id)}" title="Удалить пин">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
              </button>
            </div>
          </div>
          <div class="pin-body">
            <div class="pin-title">${escapeHtml(item.title)}</div>
            <div class="pin-date">${escapeHtml(item.created_at.split(' ')[0])}</div>
          </div>
        </div>
      `;
    }).join('');

    // Attach click to open modal
    el.pinsGrid.querySelectorAll('.pin-card').forEach(card => {
      card.addEventListener('click', (e) => {
        if (e.target.closest('.pin-action-btn')) return;
        const id = card.dataset.id;
        const pin = state.pins.find(p => p.id === id);
        if (pin) openPinModal(pin);
      });
    });

    // Delete pin button
    el.pinsGrid.querySelectorAll('.pin-action-btn[data-action="del"]').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const id = btn.dataset.id;
        if (confirm('Удалить этот пин из коллекции?')) {
          await authFetch(`/api/media/${id}`, { method: 'DELETE' });
          await fetchPins();
        }
      });
    });
  }

  function openPinModal(pin) {
    state.activePin = pin;
    if (!el.pinModal) return;
    const fullImgUrl = authMediaUrl(`/api/media/thumb/${pin.id}`);
    el.pinModalImg.src = fullImgUrl;
    el.pinModalTitle.textContent = pin.title || '📌 Pin';
    if (el.btnPinDownload) {
      el.btnPinDownload.href = fullImgUrl;
      el.btnPinDownload.setAttribute('download', `${pin.title || 'pin'}.jpg`);
    }
    el.pinModal.style.display = 'flex';
  }

  function closePinModal() {
    if (el.pinModal) el.pinModal.style.display = 'none';
    state.activePin = null;
  }

  if (el.btnClosePinModal) {
    el.btnClosePinModal.addEventListener('click', closePinModal);
  }
  if (el.pinModal) {
    el.pinModal.addEventListener('click', (e) => {
      if (e.target === el.pinModal) closePinModal();
    });
  }
  if (el.btnPinDelete) {
    el.btnPinDelete.addEventListener('click', async () => {
      if (state.activePin && confirm('Удалить этот пин?')) {
        const id = state.activePin.id;
        closePinModal();
        await authFetch(`/api/media/${id}`, { method: 'DELETE' });
        await fetchPins();
      }
    });
  }

  // ── 10. Custom Styled Video Player Controller & Progress Memory ───────────
  let controlsHideTimeout = null;
  let lastProgressSaveTime = 0;

  function saveVideoProgress(videoId, currentTime, duration) {
    if (!videoId || isNaN(currentTime) || isNaN(duration)) return;
    if (duration > 60 && currentTime >= 5 && currentTime < (duration - 5)) {
      localStorage.setItem('nb_video_pos_' + videoId, Math.floor(currentTime).toString());
    } else if (currentTime >= (duration - 5)) {
      localStorage.removeItem('nb_video_pos_' + videoId);
    }
  }

  function showVideoResumeToast(seconds) {
    const existing = document.querySelector('.resume-toast');
    if (existing) existing.remove();

    const toast = document.createElement('div');
    toast.className = 'resume-toast';
    toast.innerHTML = `<span>▶ Продолжено с ${formatTime(seconds)}</span>`;
    el.customVideoWrapper.appendChild(toast);

    setTimeout(() => {
      if (toast.parentNode) toast.remove();
    }, 3200);
  }

  function openVideoPlayerModal(videoItem) {
    state.activeVideo = videoItem;
    el.vmTitle.textContent = videoItem.title;
    el.vmAuthorName.textContent = videoItem.uploader || 'Автор';

    // Mutual exclusion: pause audio player if playing
    if (!el.audio.paused) {
      el.audio.pause();
      state.isPlaying = false;
      updateAudioPlayPauseIcons();
    }

    // Heart status
    const favPl = state.playlists.find(p => p.id === 'favorites_video');
    if (favPl && favPl.item_ids.includes(videoItem.id)) {
      el.vmBtnHeart.classList.add('active');
    } else {
      el.vmBtnHeart.classList.remove('active');
    }

    const savedPos = parseFloat(localStorage.getItem('nb_video_pos_' + videoItem.id) || '0');

    el.html5Video.src = authMediaUrl(`/api/media/stream/${videoItem.id}`);
    
    // Progress resume handler
    const onMetadata = () => {
      const dur = el.html5Video.duration || 0;
      if (savedPos >= 5 && dur > 60 && savedPos < (dur - 5)) {
        el.html5Video.currentTime = savedPos;
        showVideoResumeToast(savedPos);
      }
      el.html5Video.removeEventListener('loadedmetadata', onMetadata);
    };
    el.html5Video.addEventListener('loadedmetadata', onMetadata);

    el.html5Video.play().then(() => {
      updateVideoPlayState(true);
    }).catch(() => {
      updateVideoPlayState(false);
    });

    renderVideoCarousel(videoItem);
    el.videoPlayerModal.style.display = 'flex';
    resetControlsHideTimer();
  }

  function toggleCustomVideoPlay() {
    if (el.html5Video.paused) {
      // Pause background music before playing video
      if (!el.audio.paused) {
        el.audio.pause();
        state.isPlaying = false;
        updateAudioPlayPauseIcons();
      }
      el.html5Video.play();
      updateVideoPlayState(true);
    } else {
      el.html5Video.pause();
      updateVideoPlayState(false);
    }
    resetControlsHideTimer();
  }

  function updateVideoPlayState(isPlaying) {
    if (isPlaying) {
      el.cvIconPlay.style.display = 'none';
      el.cvIconPause.style.display = 'block';
      el.cvIconBigPlay.style.display = 'none';
      el.cvIconBigPause.style.display = 'block';
    } else {
      el.cvIconPlay.style.display = 'block';
      el.cvIconPause.style.display = 'none';
      el.cvIconBigPlay.style.display = 'block';
      el.cvIconBigPause.style.display = 'none';
      el.videoCenterOverlay.classList.add('visible');
      el.videoControlsBar.classList.add('visible');
    }
  }

  function resetControlsHideTimer() {
    el.videoControlsBar.classList.add('visible');
    el.videoCenterOverlay.classList.add('visible');
    clearTimeout(controlsHideTimeout);

    if (!el.html5Video.paused) {
      controlsHideTimeout = setTimeout(() => {
        el.videoControlsBar.classList.remove('visible');
        el.videoCenterOverlay.classList.remove('visible');
      }, 2500);
    }
  }

  el.customVideoWrapper.addEventListener('mousemove', resetControlsHideTimer);
  el.customVideoWrapper.addEventListener('touchstart', resetControlsHideTimer);

  el.videoCenterOverlay.addEventListener('click', toggleCustomVideoPlay);
  el.cvBtnPlay.addEventListener('click', toggleCustomVideoPlay);

  el.html5Video.addEventListener('timeupdate', () => {
    const cur = el.html5Video.currentTime;
    const dur = el.html5Video.duration || 0;
    const pct = dur > 0 ? (cur / dur) * 100 : 0;

    el.videoScrubberFill.style.width = `${pct}%`;
    el.videoScrubberHandle.style.left = `${pct}%`;
    el.cvTimeCurrent.textContent = formatTime(cur);
    el.cvTimeTotal.textContent = formatTime(dur);

    // Save progress every 2 seconds for long videos (> 60s)
    const now = Date.now();
    if (now - lastProgressSaveTime > 2000 && state.activeVideo) {
      lastProgressSaveTime = now;
      saveVideoProgress(state.activeVideo.id, cur, dur);
    }
  });

  el.html5Video.addEventListener('play', () => {
    // Pause audio whenever video starts
    if (!el.audio.paused) {
      el.audio.pause();
      state.isPlaying = false;
      updateAudioPlayPauseIcons();
    }
    updateVideoPlayState(true);
  });

  el.html5Video.addEventListener('pause', () => {
    updateVideoPlayState(false);
    if (state.activeVideo) {
      saveVideoProgress(state.activeVideo.id, el.html5Video.currentTime, el.html5Video.duration || 0);
    }
  });

  el.html5Video.addEventListener('ended', () => {
    if (state.activeVideo) {
      localStorage.removeItem('nb_video_pos_' + state.activeVideo.id);
    }
  });

  el.videoScrubberTrack.addEventListener('click', (e) => {
    const rect = el.videoScrubberTrack.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const pct = Math.max(0, Math.min(1, clickX / rect.width));
    if (el.html5Video.duration) {
      el.html5Video.currentTime = pct * el.html5Video.duration;
    }
  });

  el.cvBtnPip.addEventListener('click', async () => {
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else if (el.html5Video.requestPictureInPicture) {
        await el.html5Video.requestPictureInPicture();
      }
    } catch (e) {
      console.warn('PiP error:', e);
    }
  });

  el.cvBtnFullscreen.addEventListener('click', () => {
    if (!document.fullscreenElement) {
      el.customVideoWrapper.requestFullscreen().catch(() => {});
    } else {
      document.exitFullscreen().catch(() => {});
    }
  });

  // Modal Actions
  el.vmBtnHeart.addEventListener('click', async () => {
    if (state.activeVideo) {
      await toggleFavoriteVideo(state.activeVideo.id);
    }
  });

  el.vmBtnAddPl.addEventListener('click', () => {
    if (state.activeVideo) {
      openPlaylistPicker(state.activeVideo.id, 'video');
    }
  });

  el.vmBtnDelete.addEventListener('click', async () => {
    if (state.activeVideo) {
      if (confirm(`Удалить «${state.activeVideo.title}» из медиатеки?`)) {
        localStorage.removeItem('nb_video_pos_' + state.activeVideo.id);
        await authFetch(`/api/media/${state.activeVideo.id}`, { method: 'DELETE' });
        el.videoPlayerModal.style.display = 'none';
        el.html5Video.pause();
        el.html5Video.src = '';
        await fetchVideos();
      }
    }
  });

  el.vmAuthorBtn.addEventListener('click', () => {
    if (state.activeVideo && state.activeVideo.uploader) {
      state.mediaFilterAuthor = state.activeVideo.uploader;
      el.videoPlayerModal.style.display = 'none';
      el.html5Video.pause();
      renderMediaTab();
    }
  });

  el.btnCloseVideoModal.addEventListener('click', () => {
    if (state.activeVideo) {
      saveVideoProgress(state.activeVideo.id, el.html5Video.currentTime, el.html5Video.duration || 0);
    }
    el.videoPlayerModal.style.display = 'none';
    el.html5Video.pause();
    el.html5Video.src = '';
  });

  function renderVideoCarousel(currentVideo) {
    const creatorVideos = state.videos.filter(v =>
      v.id !== currentVideo.id &&
      v.uploader &&
      currentVideo.uploader &&
      v.uploader.toLowerCase() === currentVideo.uploader.toLowerCase()
    );

    const otherVideos = state.videos.filter(v =>
      v.id !== currentVideo.id && !creatorVideos.some(cv => cv.id === v.id)
    );

    const queue = [...creatorVideos, ...otherVideos];

    if (queue.length === 0) {
      el.vmMoreTitle.textContent = 'Нет других видео';
      el.vmMoreCarousel.innerHTML = '';
      return;
    }

    if (creatorVideos.length > 0) {
      el.vmMoreTitle.textContent = `Другие видео от ${currentVideo.uploader}`;
    } else {
      el.vmMoreTitle.textContent = 'Следующие видео в медиатеке';
    }

    el.vmMoreCarousel.innerHTML = queue.map(item => `
      <div class="carousel-video-card" data-id="${escapeHtml(item.id)}">
        <div class="carousel-thumb-wrap">
          <img src="${authMediaUrl('/api/media/thumb/' + item.id)}" alt="Thumb" loading="lazy">
        </div>
        <div class="carousel-video-title">${escapeHtml(item.title)}</div>
      </div>
    `).join('');

    el.vmMoreCarousel.querySelectorAll('.carousel-video-card').forEach(card => {
      card.addEventListener('click', () => {
        const id = card.dataset.id;
        const nextVideo = state.videos.find(v => v.id === id);
        if (nextVideo) openVideoPlayerModal(nextVideo);
      });
    });
  }

  // ── 11. Playlist Chips & Picker ───────────────────────────────────────────
  function renderPlaylistChips() {
    // Music Chips
    const musicPlaylists = state.playlists.filter(p => p.playlist_type === 'audio');
    let musicHtml = `
      <button class="chip-btn ${state.musicFilterPlaylist === null ? 'active' : ''}" data-pl="all">
        Все треки (${state.tracks.length})
      </button>
    `;
    musicPlaylists.forEach(pl => {
      musicHtml += `
        <button class="chip-btn ${state.musicFilterPlaylist === pl.id ? 'active' : ''}" data-pl="${escapeHtml(pl.id)}">
          ${escapeHtml(pl.name)} (${pl.item_ids.length})
        </button>
      `;
    });
    el.musicPlaylistChips.innerHTML = musicHtml;

    el.musicPlaylistChips.querySelectorAll('.chip-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const pl = btn.dataset.pl;
        state.musicFilterPlaylist = pl === 'all' ? null : pl;
        renderPlaylistChips();
        renderMusicTab();
      });
    });

    // Media Chips
    const mediaPlaylists = state.playlists.filter(p => p.playlist_type === 'video');
    let mediaHtml = `
      <button class="chip-btn ${state.mediaFilterPlaylist === null ? 'active' : ''}" data-pl="all">
        Все видео (${state.videos.length})
      </button>
    `;
    mediaPlaylists.forEach(pl => {
      mediaHtml += `
        <button class="chip-btn ${state.mediaFilterPlaylist === pl.id ? 'active' : ''}" data-pl="${escapeHtml(pl.id)}">
          ${escapeHtml(pl.name)} (${pl.item_ids.length})
        </button>
      `;
    });
    el.mediaPlaylistChips.innerHTML = mediaHtml;

    el.mediaPlaylistChips.querySelectorAll('.chip-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const pl = btn.dataset.pl;
        state.mediaFilterPlaylist = pl === 'all' ? null : pl;
        renderPlaylistChips();
        renderMediaTab();
      });
    });
  }

  function openPlaylistPicker(itemId, itemType) {
    state.pendingPlaylistItemId = itemId;
    const matching = state.playlists.filter(p => p.playlist_type === itemType);

    if (matching.length === 0) {
      el.pickerPlaylistList.innerHTML = '<p class="action-desc">Нет доступных плейлистов. Создайте плейлист через кнопку +.</p>';
    } else {
      el.pickerPlaylistList.innerHTML = matching.map(pl => {
        const isAlreadyIn = pl.item_ids.includes(itemId);
        return `
          <button class="picker-item-btn" data-pl-id="${escapeHtml(pl.id)}">
            <span>${escapeHtml(pl.name)}</span>
            <span>${isAlreadyIn ? '✓ Добавлено' : '+ Добавить'}</span>
          </button>
        `;
      }).join('');

      el.pickerPlaylistList.querySelectorAll('.picker-item-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
          const plId = btn.dataset.plId;
          await authFetch(`/api/playlists/${plId}/items`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ item_id: state.pendingPlaylistItemId }),
          });
          el.playlistPickerModal.style.display = 'none';
          await fetchPlaylists();
        });
      });
    }

    el.playlistPickerModal.style.display = 'flex';
  }

  el.btnClosePickerModal.addEventListener('click', () => {
    el.playlistPickerModal.style.display = 'none';
  });

  // ── 12. Knowledge Vault Multi-Level Hierarchical Tree System ──────────────
  const PARA_METADATA = {
    'projects': { key: 'Projects', label: '01_Projects (Проекты)', icon: '🚀' },
    'areas': { key: 'Areas', label: '02_Areas (Сферы ответственности)', icon: '🌱' },
    'resources': { key: 'Resources', label: '03_Resources (База знаний и ресурсы)', icon: '📚' },
    'archive': { key: 'Archive', label: '04_Archive (Архив)', icon: '📦' },
    'daily': { key: 'Daily', label: '00_Daily (Ежедневный дневник)', icon: '📅' },
  };

  function resolveCategoryMeta(rawCat) {
    if (!rawCat) return PARA_METADATA['daily'];
    const lower = rawCat.toLowerCase().trim();
    if (PARA_METADATA[lower]) return PARA_METADATA[lower];
    return { key: rawCat, label: rawCat, icon: '📁' };
  }

  function getFilteredNotes() {
    let list = [...state.notes];

    if (state.vaultSearchQuery.trim()) {
      const q = state.vaultSearchQuery.toLowerCase();
      list = list.filter(n =>
        n.title.toLowerCase().includes(q) ||
        n.clean_text.toLowerCase().includes(q) ||
        (n.file_path && n.file_path.toLowerCase().includes(q)) ||
        (n.para_category && n.para_category.toLowerCase().includes(q)) ||
        (n.ai_summary && n.ai_summary.toLowerCase().includes(q))
      );
    }

    return list;
  }

  function renderVaultTab() {
    const list = getFilteredNotes();

    if (list.length === 0) {
      el.vaultTreeContainer.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">
            <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
              <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
            </svg>
          </div>
          <p class="empty-title">Заметок не найдено</p>
          <p class="empty-subtitle">Записи из дневника и базы знаний сохраняются сюда автоматически</p>
        </div>
      `;
      return;
    }

    renderVaultTreeView(list);
  }

  // Pure Multi-Level Hierarchical Tree Explorer
  function renderVaultTreeView(list) {
    el.vaultTreeContainer.style.display = 'flex';

    // Build hierarchical tree: Category -> Subfolders -> Files
    const tree = {};

    list.forEach(note => {
      const catMeta = resolveCategoryMeta(note.para_category);
      if (!tree[catMeta.key]) {
        tree[catMeta.key] = {
          meta: catMeta,
          subfolders: {},
          rootNotes: [],
          totalCount: 0,
        };
      }

      tree[catMeta.key].totalCount++;

      // Parse subfolder from file_path or note.area
      const pathParts = (note.file_path || '').split('/');
      let subfolderName = null;

      if (pathParts.length > 2) {
        subfolderName = pathParts[1];
      } else if (pathParts.length === 2 && !pathParts[0].toLowerCase().includes(catMeta.key.toLowerCase())) {
        subfolderName = pathParts[0];
      } else if (note.area) {
        subfolderName = note.area;
      }

      if (subfolderName) {
        if (!tree[catMeta.key].subfolders[subfolderName]) {
          tree[catMeta.key].subfolders[subfolderName] = [];
        }
        tree[catMeta.key].subfolders[subfolderName].push(note);
      } else {
        tree[catMeta.key].rootNotes.push(note);
      }
    });

    const categoryKeys = Object.keys(tree);

    let treeHtml = `<div class="tree-explorer" style="width:100%;">`;

    categoryKeys.forEach(catKey => {
      const node = tree[catKey];
      const isCollapsed = state.collapsedBranches.has(catKey);

      treeHtml += `
        <!-- Level 1 Category Folder -->
        <div class="tree-folder ${isCollapsed ? 'collapsed' : ''}" data-node-id="${escapeHtml(catKey)}">
          <div class="tree-folder-head">
            <div class="tree-folder-left">
              <svg class="tree-chevron" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="6 9 12 15 18 9"></polyline>
              </svg>
              <span class="tree-folder-icon">${node.meta.icon}</span>
              <span class="tree-folder-name">${escapeHtml(node.meta.label)}</span>
            </div>
            <span class="tree-folder-count">${node.totalCount} ${node.totalCount === 1 ? 'файл' : 'файлов'}</span>
          </div>

          <div class="tree-folder-children">
      `;

      // Level 2 Subfolders
      Object.keys(node.subfolders).forEach(subName => {
        const subNotes = node.subfolders[subName];
        const subId = `${catKey}_${subName}`;
        const isSubCollapsed = state.collapsedBranches.has(subId);

        treeHtml += `
          <div class="tree-folder ${isSubCollapsed ? 'collapsed' : ''}" data-node-id="${escapeHtml(subId)}">
            <div class="tree-folder-head">
              <div class="tree-folder-left">
                <svg class="tree-chevron" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.5">
                  <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
                <span class="tree-folder-icon">${isSubCollapsed ? '📁' : '📂'}</span>
                <span class="tree-folder-name">${escapeHtml(subName)}</span>
              </div>
              <span class="tree-folder-count">${subNotes.length}</span>
            </div>

            <div class="tree-folder-children">
              ${subNotes.map(note => `
                <div class="tree-file-item" data-note-id="${escapeHtml(note.id)}">
                  <div class="tree-file-left">
                    <span class="tree-file-icon">📄</span>
                    <span class="tree-file-title">${escapeHtml(note.title)}</span>
                    <span class="tree-file-ext">.md</span>
                  </div>
                  <div class="tree-file-right">
                    ${note.ai_summary ? '<span class="tree-file-has-ai">⚡ ИИ</span>' : ''}
                    <span class="tree-file-date">${escapeHtml(note.created_at || '')}</span>
                  </div>
                </div>
              `).join('')}
            </div>
          </div>
        `;
      });

      // Direct Category Notes (Root of category)
      node.rootNotes.forEach(note => {
        treeHtml += `
          <div class="tree-file-item" data-note-id="${escapeHtml(note.id)}">
            <div class="tree-file-left">
              <span class="tree-file-icon">📄</span>
              <span class="tree-file-title">${escapeHtml(note.title)}</span>
              <span class="tree-file-ext">.md</span>
            </div>
            <div class="tree-file-right">
              ${note.ai_summary ? '<span class="tree-file-has-ai">⚡ ИИ</span>' : ''}
              <span class="tree-file-date">${escapeHtml(note.created_at || '')}</span>
            </div>
          </div>
        `;
      });

      treeHtml += `
          </div>
        </div>
      `;
    });

    treeHtml += `</div>`;
    el.vaultTreeContainer.innerHTML = treeHtml;

    // Folder collapse/expand handler
    el.vaultTreeContainer.querySelectorAll('.tree-folder-head').forEach(head => {
      head.addEventListener('click', (e) => {
        e.stopPropagation();
        const folder = head.closest('.tree-folder');
        const nodeId = folder.dataset.nodeId;
        if (state.collapsedBranches.has(nodeId)) {
          state.collapsedBranches.delete(nodeId);
          folder.classList.remove('collapsed');
        } else {
          state.collapsedBranches.add(nodeId);
          folder.classList.add('collapsed');
        }
      });
    });

    // File click -> Open Note Document Reader
    el.vaultTreeContainer.querySelectorAll('.tree-file-item').forEach(item => {
      item.addEventListener('click', () => {
        const noteId = item.dataset.noteId;
        const note = state.notes.find(n => n.id === noteId);
        if (note) openNoteReader(note);
      });
    });
  }

  function formatMarkdownNote(text) {
    if (!text) return '';
    let raw = escapeHtml(text);

    // Markdown tables converter
    const lines = raw.split('\n');
    let formattedLines = [];
    let inTable = false;
    let tableRows = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line.startsWith('|') && line.endsWith('|')) {
        if (line.includes('---')) continue; // skip table separator
        const cells = line.split('|').slice(1, -1).map(c => c.trim());
        if (!inTable) {
          inTable = true;
          tableRows.push(`<thead><tr>${cells.map(c => `<th>${c}</th>`).join('')}</tr></thead><tbody>`);
        } else {
          tableRows.push(`<tr>${cells.map(c => `<td>${c}</td>`).join('')}</tr>`);
        }
      } else {
        if (inTable) {
          inTable = false;
          tableRows.push('</tbody></table>');
          formattedLines.push(`<table class="markdown-table">${tableRows.join('')}`);
          tableRows = [];
        }
        formattedLines.push(line);
      }
    }
    if (inTable) {
      tableRows.push('</tbody></table>');
      formattedLines.push(`<table class="markdown-table">${tableRows.join('')}`);
    }

    return formattedLines.join('\n')
      .replace(/^### (.*$)/gim, '<h4 style="color:#60a5fa;margin:14px 0 6px;">$1</h4>')
      .replace(/^## (.*$)/gim, '<h3 style="color:#fff;margin:18px 0 8px;border-bottom:1px solid rgba(255,255,255,0.1);padding-bottom:4px;">$1</h3>')
      .replace(/^# (.*$)/gim, '<h2 style="color:#fff;margin:20px 0 10px;">$1</h2>')
      .replace(/^\- (.*$)/gim, '<div style="padding-left:14px;position:relative;margin:3px 0;"><span style="position:absolute;left:0;color:#60a5fa;">•</span>$1</div>')
      .replace(/\*\*(.*?)\*\*/gim, '<strong>$1</strong>')
      .replace(/\[\[(.*?)\|(.*?)\]\]/gim, '<span style="color:#60a5fa;background:rgba(59,130,246,0.15);padding:2px 6px;border-radius:4px;font-size:12px;">$2</span>')
      .replace(/\[\[(.*?)\]\]/gim, '<span style="color:#60a5fa;background:rgba(59,130,246,0.15);padding:2px 6px;border-radius:4px;font-size:12px;">$1</span>')
      .replace(/\n\n/g, '<div style="height:10px;"></div>');
  }

  // 2. Note Document Reader Modal
  function openNoteReader(note) {
    const meta = resolveCategoryMeta(note.para_category);
    el.readerCrumbFolder.textContent = meta.label;
    el.readerCrumbFile.textContent = `${note.title}.md`;
    el.readerTitle.textContent = note.title;

    el.readerMetaRow.innerHTML = `
      <span class="reader-para-badge">${escapeHtml(meta.label)}</span>
      <span class="tree-file-date">📅 ${escapeHtml(note.created_at || '')}</span>
      <span class="tree-file-date">📝 ${note.clean_text.split(/\s+/).length} слов</span>
    `;

    if (note.ai_summary) {
      el.readerSummaryCard.style.display = 'flex';
      el.readerSummaryText.textContent = note.ai_summary;
    } else {
      el.readerSummaryCard.style.display = 'none';
    }

    el.readerContent.innerHTML = formatMarkdownNote(note.clean_text);

    el.btnReaderProp.onclick = () => {
      openPropertiesModal(note.id);
    };

    el.noteReaderModal.style.display = 'flex';
  }

  el.btnCloseReaderModal.addEventListener('click', () => {
    el.noteReaderModal.style.display = 'none';
  });

  async function openPropertiesModal(noteId) {
    try {
      const res = await authFetch(`/api/vault/note/${noteId}/properties`);
      if (res.ok) {
        const p = await res.json();
        el.propertiesContent.innerHTML = `
          <div class="prop-row"><span class="prop-label">Категория PARA</span><span class="prop-val">${escapeHtml(p.para_category)}</span></div>
          <div class="prop-row"><span class="prop-label">Область жизни</span><span class="prop-val">${escapeHtml(p.area || 'Личное')}</span></div>
          <div class="prop-row"><span class="prop-label">Количество слов</span><span class="prop-val">${p.clean_text ? p.clean_text.split(/\s+/).length : 0}</span></div>
          <div class="prop-row"><span class="prop-label">Создано</span><span class="prop-val">${escapeHtml(p.created_at)}</span></div>
          <div class="prop-row"><span class="prop-label">Файл хранилища</span><span class="prop-val">${escapeHtml(p.file_path || p.title + '.md')}</span></div>
          <div class="prop-row"><span class="prop-label">Теги</span><span class="prop-val">${p.tags.join(', ') || 'нет'}</span></div>
          <div class="prop-row"><span class="prop-label">Сущности</span><span class="prop-val">${p.entities.join(', ') || 'нет'}</span></div>
        `;
        el.notePropertiesModal.style.display = 'flex';
      }
    } catch (e) {
      console.error('Failed to load properties:', e);
    }
  }

  el.btnClosePropertiesModal.addEventListener('click', () => {
    el.notePropertiesModal.style.display = 'none';
  });

  // ── 13. FAB Action Sheet & Modal Navigation ───────────────────────────────
  el.fabAddBtn.addEventListener('click', () => {
    el.addContentModal.style.display = 'flex';
    renderDownloadQueue();
  });

  el.btnCloseAddModal.addEventListener('click', () => {
    el.addContentModal.style.display = 'none';
  });

  el.sheetTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      el.sheetTabs.forEach(t => t.classList.remove('active'));
      el.actionTabPanes.forEach(p => p.classList.remove('active'));

      tab.classList.add('active');
      const pane = document.getElementById(`action-${tab.dataset.actionTab}`);
      if (pane) pane.classList.add('active');
    });
  });

  // File Upload
  el.btnTriggerFilePick.addEventListener('click', () => el.localFileInput.click());
  el.uploadDropzone.addEventListener('click', () => el.localFileInput.click());

  el.localFileInput.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const formData = new FormData();
    formData.append('file', file);

    showStatus('Загрузка файла...', 'success');
    try {
      const res = await authFetch('/api/media/upload', { method: 'POST', body: formData });
      if (res.ok) {
        showStatus('Файл успешно сохранен!', 'success');
        await fetchAllData();
      } else {
        showStatus('Ошибка загрузки файла', 'error');
      }
    } catch (err) {
      showStatus('Сетевая ошибка при загрузке', 'error');
    }
  });

  // Create Playlist
  el.btnSubmitCreatePl.addEventListener('click', async () => {
    const name = el.newPlaylistName.value.trim();
    if (!name) return;

    const plTypeRadio = document.querySelector('input[name="pl_type"]:checked');
    const plType = plTypeRadio ? plTypeRadio.value : 'audio';

    try {
      const res = await authFetch('/api/playlists', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, playlist_type: plType }),
      });

      if (res.ok) {
        showStatus('Плейлист успешно создан!', 'success');
        el.newPlaylistName.value = '';
        await fetchPlaylists();
      }
    } catch (err) {
      showStatus('Ошибка создания плейлиста', 'error');
    }
  });

  // ── 14. Search Inputs ─────────────────────────────────────────────────────
  el.musicSearchInput.addEventListener('input', (e) => {
    state.musicSearchQuery = e.target.value;
    renderMusicTab();
  });

  el.mediaSearchInput.addEventListener('input', (e) => {
    state.mediaSearchQuery = e.target.value;
    renderMediaTab();
  });

  el.vaultSearchInput.addEventListener('input', (e) => {
    state.vaultSearchQuery = e.target.value;
    renderVaultTab();
  });

  if (el.pinsSearchInput) {
    el.pinsSearchInput.addEventListener('input', (e) => {
      state.pinsSearchQuery = e.target.value;
      renderPinsTab();
    });
  }

  // ── 15. Nav Tabs Switching ────────────────────────────────────────────────
  el.tabBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      el.tabBtns.forEach(b => b.classList.remove('active'));
      el.tabPanes.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const tabId = `pane-${btn.dataset.tab}`;
      const pane = document.getElementById(tabId);
      if (pane) pane.classList.add('active');
      state.activeTab = btn.dataset.tab;
    });
  });

  // ── 16. Initial Data Load ─────────────────────────────────────────────────
  fetchAllData();
});
