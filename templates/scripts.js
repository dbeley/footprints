        const state = {
            currentOffset: 0,
            currentPage: 1,
            limit: 50,
            totalScrobbles: 0,
            currentPeriod: 'alltime',
            customRange: null,
        };

        function periodLabel(period) {
            switch (period) {
                case 'today': return 'Today';
                case 'week': return 'This Week';
                case 'month': return 'This Month';
                case 'year': return 'This Year';
                default: return 'All Time';
            }
        }

        async function loadStats() {
            try {
                const response = await fetch('/api/stats');
                const data = await response.json();
                document.getElementById('totalScrobbles').textContent = data.total_scrobbles.toLocaleString();
                state.totalScrobbles = data.total_scrobbles;
                updatePaginationInfo();
            } catch (error) {
                console.error('Error loading stats:', error);
            }
        }

        async function loadStatsUI(period, range) {
            try {
                const params = new URLSearchParams({ period });
                if (range && range.start && range.end) {
                    params.set('start', range.start);
                    params.set('end', range.end);
                }

                const response = await fetch(`/api/stats/ui?${params.toString()}`);
                const data = await response.json();

                state.lastArtists = data.top_artists;
                state.lastTracks = data.top_tracks;
                state.lastAlbums = data.top_albums;

                const label = period === 'custom' && range
                    ? formatCustomLabel(range)
                    : periodLabel(period);

                document.getElementById('activePeriodLabel').textContent = label;
                document.getElementById('periodScrobbles').textContent = data.period_scrobbles.toLocaleString();

                renderGrid('topArtistsGrid', data.top_artists, 'artist');
                renderGrid('topTracksGrid', data.top_tracks, 'track');
                renderGrid('topAlbumsGrid', data.top_albums, 'album');
            } catch (error) {
                console.error('Error loading UI stats:', error);
            }
        }

        function renderGrid(containerId, items, type) {
            const container = document.getElementById(containerId);

            if (!items || items.length === 0) {
                container.innerHTML = `
                    <div class="empty-state">
                        <div class="muted">No scrobbles in this period.</div>
                    </div>
                `;
                return;
            }

            const limit = 15;
            const html = items.slice(0, limit).map((item, idx) => {
                const rank = idx + 1;
                let name, details, count, imageUrl, artist, album, track;

                if (type === 'artist') {
                    name = item.name;
                    artist = item.name;
                    count = item.count;
                    imageUrl = item.image_url;
                } else if (type === 'track') {
                    name = item.track;
                    details = item.artist;
                    artist = item.artist;
                    track = item.track;
                    count = item.count;
                    imageUrl = item.image_url;
                } else {
                    name = item.album;
                    details = item.artist;
                    artist = item.artist;
                    album = item.album;
                    count = item.count;
                    imageUrl = item.image_url;
                }

                const placeholder = !imageUrl
                    ? `<div class="placeholder">${getInitials(name)}</div>`
                    : '';
                const cover = imageUrl
                    ? `<img class="cover" src="${escapeHtml(imageUrl)}" alt="" loading="lazy" referrerpolicy="no-referrer" onerror="this.style.display='none'">`
                    : '';

                const clickHandler = type === 'artist'
                    ? `onclick="openEntityModal('artist', '${escapeHtml(artist).replace(/'/g, "\\'")}')"`
                    : type === 'album'
                    ? `onclick="openEntityModal('album', '${escapeHtml(artist).replace(/'/g, "\\'")}', '${escapeHtml(album).replace(/'/g, "\\'")}')"`
                    : `onclick="openEntityModal('track', '${escapeHtml(artist).replace(/'/g, "\\'")}', '${escapeHtml(track).replace(/'/g, "\\'")}')"`;

                return `
                    <div class="grid-item" ${clickHandler}>
                        ${cover}
                        ${placeholder}
                        <div class="overlay">
                            <span class="rank-badge">#${rank}</span>
                            <div class="name">${escapeHtml(name)}</div>
                            ${details ? `<div class="details">${escapeHtml(details)}</div>` : ''}
                            <div class="count">${count.toLocaleString()} plays</div>
                        </div>
                    </div>
                `;
            }).join('');

            container.innerHTML = html;
        }

        async function loadTimeline(offset = 0) {
            try {
                const response = await fetch(`/api/timeline?limit=${state.limit}&offset=${offset}`);
                const data = await response.json();

                const timeline = document.getElementById('timelineList');
                timeline.innerHTML = '';

                if (data.length === 0) {
                    timeline.innerHTML = '<div class="muted" style="padding: 12px;">No scrobbles found</div>';
                    return;
                }

                data.forEach(scrobble => {
                    const date = new Date(scrobble.timestamp);
                    const item = document.createElement('div');
                    item.className = 'timeline-item';
                    item.innerHTML = `
                        <div class="timestamp">${date.toLocaleString()}</div>
                        <div class="track-name">${escapeHtml(scrobble.track)}</div>
                        <div class="item-details">
                            ${escapeHtml(scrobble.artist)}
                            ${scrobble.album ? ' • ' + escapeHtml(scrobble.album) : ''}
                        </div>
                    `;
                    timeline.appendChild(item);
                });

                state.currentOffset = offset;
                state.currentPage = Math.floor(offset / state.limit) + 1;
                updatePaginationInfo();
            } catch (error) {
                console.error('Error loading timeline:', error);
            }
        }

        function updatePaginationInfo() {
            const totalPages = Math.ceil(state.totalScrobbles / state.limit);
            document.getElementById('pageInfo').textContent = `of ${totalPages.toLocaleString()}`;
            document.getElementById('pageInput').value = state.currentPage;
            document.getElementById('pageInput').max = totalPages;

            // Update button states
            document.getElementById('firstPageBtn').disabled = state.currentPage === 1;
            document.getElementById('prevPageBtn').disabled = state.currentPage === 1;
            document.getElementById('nextPageBtn').disabled = state.currentPage >= totalPages;
            document.getElementById('lastPageBtn').disabled = state.currentPage >= totalPages;
        }

        function goToFirstPage() {
            loadTimeline(0);
        }

        function goToPrevPage() {
            if (state.currentPage > 1) {
                loadTimeline((state.currentPage - 2) * state.limit);
            }
        }

        function goToNextPage() {
            const totalPages = Math.ceil(state.totalScrobbles / state.limit);
            if (state.currentPage < totalPages) {
                loadTimeline(state.currentPage * state.limit);
            }
        }

        function goToLastPage() {
            const totalPages = Math.ceil(state.totalScrobbles / state.limit);
            const lastPageOffset = (totalPages - 1) * state.limit;
            loadTimeline(lastPageOffset);
        }

        function goToPage() {
            const pageInput = document.getElementById('pageInput');
            const pageNumber = parseInt(pageInput.value, 10);
            const totalPages = Math.ceil(state.totalScrobbles / state.limit);

            if (pageNumber >= 1 && pageNumber <= totalPages) {
                loadTimeline((pageNumber - 1) * state.limit);
            } else {
                pageInput.value = state.currentPage;
            }
        }

        async function importLastFm() {
            const username = document.getElementById('lastfmUsername').value;
            const apiKey = document.getElementById('lastfmApiKey').value;
            const messageDiv = document.getElementById('importMessage');
            const btn = document.getElementById('lastfmBtn');

            if (!username || !apiKey) {
                messageDiv.innerHTML = '<div class="error">Please provide both username and API key</div>';
                return;
            }

            btn.disabled = true;
            messageDiv.innerHTML = '<div class="loading">Importing from Last.fm...</div>';

            try {
                const response = await fetch('/api/import', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        source: 'lastfm',
                        username,
                        api_key: apiKey
                    })
                });

                    const result = await response.json();
                    if (result.success) {
                        messageDiv.innerHTML = `<div class="success">${result.message}</div>`;
                        loadStats();
                        loadStatsUI(state.currentPeriod, state.customRange);
                    } else {
                        messageDiv.innerHTML = `<div class="error">${result.message}</div>`;
                    }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Import failed</div>';
                console.error('Error:', error);
            } finally {
                btn.disabled = false;
            }
        }

        async function importListenBrainz() {
            const username = document.getElementById('lbUsername').value;
            const token = document.getElementById('lbToken').value;
            const messageDiv = document.getElementById('importMessage');
            const btn = document.getElementById('lbBtn');

            if (!username) {
                messageDiv.innerHTML = '<div class="error">Please provide username</div>';
                return;
            }

            btn.disabled = true;
            messageDiv.innerHTML = '<div class="loading">Importing from ListenBrainz...</div>';

            try {
                const response = await fetch('/api/import', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        source: 'listenbrainz',
                        username,
                        token: token || null
                    })
                });

                    const result = await response.json();
                    if (result.success) {
                        messageDiv.innerHTML = `<div class="success">${result.message}</div>`;
                        loadStats();
                        loadStatsUI(state.currentPeriod, state.customRange);
                    } else {
                        messageDiv.innerHTML = `<div class="error">${result.message}</div>`;
                    }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Import failed</div>';
                console.error('Error:', error);
            } finally {
                btn.disabled = false;
            }
        }

        // Sync configuration functions
        async function loadSyncConfigs() {
            try {
                const response = await fetch('/api/sync/config');
                const configs = await response.json();

                const container = document.getElementById('syncConfigList');
                if (configs.length === 0) {
                    container.innerHTML = '<div class="muted" style="padding: 12px;">No sync configurations yet. Add one below to keep your history automatically updated.</div>';
                    return;
                }

                container.innerHTML = configs.map(config => `
                    <div class="sync-config-item">
                        <div class="sync-config-info">
                            <div class="sync-config-name">
                                ${config.source === 'lastfm' ? '🎵 Last.fm' : '🎧 ListenBrainz'}: ${escapeHtml(config.username)}
                                <span class="sync-config-status ${config.enabled ? 'enabled' : 'disabled'}">
                                    ${config.enabled ? 'Active' : 'Paused'}
                                </span>
                            </div>
                            <div class="sync-config-details">
                                Syncs every ${config.sync_interval_minutes} minutes
                                ${config.last_sync_timestamp ? ' • Last synced: ' + new Date(config.last_sync_timestamp).toLocaleString() : ' • Never synced'}
                            </div>
                        </div>
                        <div class="sync-config-actions">
                            <button class="ghost" onclick="triggerSync(${config.id})">Sync Now</button>
                            <button class="ghost" onclick="toggleSyncConfig(${config.id}, ${!config.enabled})">${config.enabled ? 'Pause' : 'Resume'}</button>
                            <button class="ghost" onclick="deleteSyncConfig(${config.id})">Delete</button>
                        </div>
                    </div>
                `).join('');
            } catch (error) {
                console.error('Error loading sync configs:', error);
                document.getElementById('syncConfigList').innerHTML = '<div class="error">Failed to load sync configurations</div>';
            }
        }

        async function addSyncConfig(source) {
            const messageDiv = document.getElementById('syncMessage');
            let username, apiKey, token, interval, btn;

            if (source === 'lastfm') {
                username = document.getElementById('syncLastfmUsername').value;
                apiKey = document.getElementById('syncLastfmApiKey').value;
                interval = parseInt(document.getElementById('syncLastfmInterval').value);
                btn = document.getElementById('addSyncLastfmBtn');

                if (!username || !apiKey) {
                    messageDiv.innerHTML = '<div class="error">Please provide both username and API key</div>';
                    return;
                }
            } else {
                username = document.getElementById('syncLbUsername').value;
                token = document.getElementById('syncLbToken').value;
                interval = parseInt(document.getElementById('syncLbInterval').value);
                btn = document.getElementById('addSyncLbBtn');

                if (!username) {
                    messageDiv.innerHTML = '<div class="error">Please provide username</div>';
                    return;
                }
            }

            if (!interval || interval < 15 || interval > 1440) {
                messageDiv.innerHTML = '<div class="error">Sync interval must be between 15 and 1440 minutes</div>';
                return;
            }

            btn.disabled = true;
            messageDiv.innerHTML = '<div class="loading">Adding sync configuration...</div>';

            try {
                const body = {
                    source,
                    username,
                    sync_interval_minutes: interval,
                    enabled: true
                };

                if (source === 'lastfm') {
                    body.api_key = apiKey;
                } else if (token) {
                    body.token = token;
                }

                const response = await fetch('/api/sync/config', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(body)
                });

                const result = await response.json();
                if (result.success) {
                    messageDiv.innerHTML = `<div class="success">${result.message}</div>`;
                    loadSyncConfigs();
                    // Clear form
                    if (source === 'lastfm') {
                        document.getElementById('syncLastfmUsername').value = '';
                        document.getElementById('syncLastfmApiKey').value = '';
                        document.getElementById('syncLastfmInterval').value = '60';
                    } else {
                        document.getElementById('syncLbUsername').value = '';
                        document.getElementById('syncLbToken').value = '';
                        document.getElementById('syncLbInterval').value = '60';
                    }
                } else {
                    messageDiv.innerHTML = `<div class="error">${result.message || 'Failed to add sync configuration'}</div>`;
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Failed to add sync configuration</div>';
                console.error('Error:', error);
            } finally {
                btn.disabled = false;
            }
        }

        async function triggerSync(configId) {
            const messageDiv = document.getElementById('syncMessage');
            messageDiv.innerHTML = '<div class="loading">Triggering sync...</div>';

            try {
                const response = await fetch(`/api/sync/config/${configId}/trigger`, {
                    method: 'POST'
                });

                const result = await response.json();
                if (result.success) {
                    messageDiv.innerHTML = `<div class="success">${result.message}</div>`;
                    loadSyncConfigs();
                    loadStats();
                    loadStatsUI(state.currentPeriod, state.customRange);
                } else {
                    messageDiv.innerHTML = `<div class="error">${result.message || 'Sync failed'}</div>`;
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Failed to trigger sync</div>';
                console.error('Error:', error);
            }
        }

        async function toggleSyncConfig(configId, enable) {
            const messageDiv = document.getElementById('syncMessage');

            try {
                // Get current config
                const getResponse = await fetch(`/api/sync/config/${configId}`);
                const config = await getResponse.json();

                // Update enabled status
                config.enabled = enable;

                const response = await fetch(`/api/sync/config/${configId}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(config)
                });

                const result = await response.json();
                if (result.success) {
                    messageDiv.innerHTML = `<div class="success">Sync configuration ${enable ? 'resumed' : 'paused'}</div>`;
                    loadSyncConfigs();
                } else {
                    messageDiv.innerHTML = `<div class="error">Failed to update sync configuration</div>`;
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Failed to update sync configuration</div>';
                console.error('Error:', error);
            }
        }

        async function deleteSyncConfig(configId) {
            if (!confirm('Are you sure you want to delete this sync configuration?')) {
                return;
            }

            const messageDiv = document.getElementById('syncMessage');

            try {
                const response = await fetch(`/api/sync/config/${configId}`, {
                    method: 'DELETE'
                });

                const result = await response.json();
                if (result.success) {
                    messageDiv.innerHTML = `<div class="success">${result.message}</div>`;
                    loadSyncConfigs();
                } else {
                    messageDiv.innerHTML = `<div class="error">Failed to delete sync configuration</div>`;
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Failed to delete sync configuration</div>';
                console.error('Error:', error);
            }
        }

        // Export functions
        async function exportData(format) {
            const messageDiv = document.getElementById('exportMessage');
            const btn = document.getElementById(`export${format.charAt(0).toUpperCase() + format.slice(1)}Btn`);

            btn.disabled = true;
            messageDiv.innerHTML = '<div class="loading">Preparing export...</div>';

            try {
                const response = await fetch(`/api/export?format=${format}`);

                if (!response.ok) {
                    throw new Error('Export failed');
                }

                const blob = await response.blob();
                const url = window.URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `footprints_export_${new Date().toISOString().split('T')[0]}.${format}`;
                document.body.appendChild(a);
                a.click();
                window.URL.revokeObjectURL(url);
                document.body.removeChild(a);

                messageDiv.innerHTML = `<div class="success">Export completed successfully</div>`;
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Export failed</div>';
                console.error('Error:', error);
            } finally {
                btn.disabled = false;
            }
        }

        function switchTab(tabName) {
            document.querySelectorAll('.tab').forEach(tab => {
                tab.classList.toggle('active', tab.dataset.tab === tabName);
            });

            document.querySelectorAll('.tab-content').forEach(content => {
                content.classList.toggle('active', content.id === tabName);
            });

            // Show/hide controls panel based on tab
            const controlsPanel = document.querySelector('.panel .controls').parentElement;
            if (tabName === 'overview') {
                controlsPanel.style.display = 'block';
            } else {
                controlsPanel.style.display = 'none';
            }

            if (tabName === 'timeline' && state.currentOffset === 0) {
                loadTimeline(0);
            } else if (tabName === 'import') {
                loadSyncConfigs();
            } else if (tabName === 'sessions') {
                loadSessionsReport();
            } else if (tabName === 'heatmap') {
                loadHeatmapReport();
            } else if (tabName === 'novelty') {
                loadNoveltyReport();
            } else if (tabName === 'transitions') {
                loadTransitionsReport();
            } else if (tabName === 'diversity') {
                loadDiversityReport();
            } else if (tabName === 'yearly') {
                initYearlyTab();
            }
        }

        function getInitials(name) {
            return name
                .split(' ')
                .map(word => word[0])
                .filter(Boolean)
                .slice(0, 2)
                .join('')
                .toUpperCase();
        }

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text ?? '';
            return div.innerHTML;
        }

        function formatCustomLabel(range) {
            const start = new Date(range.start);
            const end = new Date(range.end);
            return `${start.toLocaleDateString()} – ${end.toLocaleDateString()}`;
        }

        function applyCustomRange() {
            const startInput = document.getElementById('customStart').value;
            const endInput = document.getElementById('customEnd').value;
            const message = document.getElementById('customRangeMessage');

            if (!startInput || !endInput) {
                message.textContent = 'Select both start and end dates';
                return;
            }

            const startIso = new Date(`${startInput}T00:00:00Z`).toISOString();
            const endIso = new Date(`${endInput}T23:59:59Z`).toISOString();

            state.customRange = { start: startIso, end: endIso };
            state.currentPeriod = 'custom';
            document.querySelectorAll('.time-btn').forEach(b => b.classList.remove('active'));
            const customBtn = document.querySelector('.time-btn[data-period="custom"]');
            if (customBtn) customBtn.classList.add('active');
            message.textContent = `Using ${formatCustomLabel(state.customRange)}`;
            loadStatsUI('custom', state.customRange);
        }

        document.addEventListener('DOMContentLoaded', function() {
            document.querySelectorAll('.tab').forEach(tab => {
                tab.addEventListener('click', function() {
                    switchTab(this.dataset.tab);
                });
            });

            document.querySelectorAll('.time-btn').forEach(btn => {
                btn.addEventListener('click', function() {
                    document.querySelectorAll('.time-btn').forEach(b => b.classList.remove('active'));
                    this.classList.add('active');

                    state.currentPeriod = this.dataset.period;

                    const customRangeEl = document.getElementById('customRange');
                    if (state.currentPeriod === 'custom') {
                        customRangeEl.style.display = 'grid';
                        document.getElementById('customRangeMessage').textContent = 'Pick a start and end date, then apply.';
                    } else {
                        customRangeEl.style.display = 'none';
                        state.customRange = null;
                        loadStatsUI(state.currentPeriod, state.customRange);
                    }
                });
            });

            const today = new Date();
            const monthAgo = new Date();
            monthAgo.setDate(today.getDate() - 30);
            document.getElementById('customEnd').value = today.toISOString().slice(0, 10);
            document.getElementById('customStart').value = monthAgo.toISOString().slice(0, 10);

            loadStats();
            loadStatsUI(state.currentPeriod, state.customRange);

            // Add Enter key support for page input
            document.getElementById('pageInput').addEventListener('keypress', (e) => {
                if (e.key === 'Enter') {
                    goToPage();
                }
            });
        });

        // Novelty Report Functions
        async function loadNoveltyReport() {
            const granularity = document.getElementById('noveltyGranularity').value;
            const message = document.getElementById('noveltyMessage');

            // Reset pagination when loading new report
            window.noveltyDiscoveriesPage = 1;

            message.innerHTML = `<div style="text-align: center; padding: 40px;">
                <div class="loading-spinner" style="margin: 0 auto 12px;"></div>
                <p class="muted">Analyzing your listening patterns...</p>
            </div>`;

            try {
                const params = new URLSearchParams({ granularity });
                if (state.customRange) {
                    params.append('start', state.customRange.start);
                    params.append('end', state.customRange.end);
                }

                const response = await fetch(`/api/reports/novelty?${params}`);
                const data = await response.json();

                message.innerHTML = '';
                renderNoveltySummary(data.summary);
                renderNoveltyTimeline(data.timeline);
                renderNoveltyDiscoveries(data.new_artists_discovered);
            } catch (error) {
                message.innerHTML = `<div style="text-align: center; padding: 40px; color: #ef4444;">
                    <div style="font-size: 48px; margin-bottom: 12px;">⚠️</div>
                    <p>Error loading novelty report</p>
                    <p class="muted" style="margin-top: 8px;">${error.message}</p>
                </div>`;
            }
        }

        function renderNoveltySummary(summary) {
            const container = document.getElementById('noveltySummary');
            const noveltyPercent = (summary.avg_novelty_ratio * 100).toFixed(1);

            container.innerHTML = `
                <div class="metric-cards">
                    <div class="metric-card">
                        <div class="metric-label">Avg Novelty Ratio</div>
                        <div class="metric-value">${noveltyPercent}%</div>
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${noveltyPercent}%"></div>
                        </div>
                        <div class="metric-subtext">New tracks vs. repeats</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Unique Tracks</div>
                        <div class="metric-value">${summary.total_unique_tracks.toLocaleString()}</div>
                        <div class="metric-subtext">Discovered songs</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Unique Artists</div>
                        <div class="metric-value">${summary.total_unique_artists.toLocaleString()}</div>
                        <div class="metric-subtext">Different artists</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Total Scrobbles</div>
                        <div class="metric-value">${summary.total_scrobbles.toLocaleString()}</div>
                        <div class="metric-subtext">Listening sessions</div>
                    </div>
                </div>
                <div style="text-align: center; margin: 16px 0; padding: 12px; background: var(--tile); border-radius: 8px; border: 1px solid var(--border);">
                    <span style="color: #10b981; font-weight: 600;">🎯 Most exploratory:</span>
                    <span style="color: var(--text); margin-right: 20px;">${escapeHtml(summary.most_exploratory_period)}</span>
                    <span style="color: var(--accent); font-weight: 600;">🔁 Least exploratory:</span>
                    <span style="color: var(--text);">${escapeHtml(summary.least_exploratory_period)}</span>
                </div>
            `;
        }

        function renderNoveltyTimeline(timeline) {
            const container = document.getElementById('noveltyTimeline');
            if (timeline.length === 0) {
                container.innerHTML = '<div class="empty-state"><div class="empty-state-icon">📊</div><p>No timeline data available</p></div>';
                return;
            }

            let html = '<div class="section-header"><h3>📈 Novelty Timeline <span class="badge">' + timeline.length + ' periods</span></h3></div>';
            html += '<div class="timeline-viz">';

            timeline.forEach(point => {
                const noveltyPercent = (point.novelty_ratio * 100).toFixed(1);
                const newPercent = point.total_scrobbles > 0 ? (point.new_tracks / point.total_scrobbles * 100) : 0;
                const repeatPercent = 100 - newPercent;

                html += `
                    <div class="timeline-item">
                        <div class="timeline-period">${escapeHtml(point.period)}</div>
                        <div class="timeline-bar">
                            <div class="timeline-bar-new" style="width: ${newPercent}%"></div>
                            <div class="timeline-bar-repeat" style="width: ${repeatPercent}%"></div>
                        </div>
                        <div class="timeline-stats">
                            <strong>${noveltyPercent}%</strong> new<br>
                            <span class="muted">${point.total_scrobbles} total</span>
                        </div>
                    </div>
                `;
            });

            html += '</div>';
            container.innerHTML = html;
        }

        function renderNoveltyDiscoveries(discoveries) {
            const container = document.getElementById('noveltyDiscoveries');
            if (discoveries.length === 0) {
                container.innerHTML = '';
                return;
            }

            // Store discoveries for pagination
            window.noveltyDiscoveries = discoveries;
            if (!window.noveltyDiscoveriesPage) {
                window.noveltyDiscoveriesPage = 1;
            }

            const itemsPerPage = 20;
            const totalPages = Math.ceil(discoveries.length / itemsPerPage);
            const currentPage = Math.min(window.noveltyDiscoveriesPage, totalPages);
            const startIdx = (currentPage - 1) * itemsPerPage;
            const endIdx = Math.min(startIdx + itemsPerPage, discoveries.length);
            const pageDiscoveries = discoveries.slice(startIdx, endIdx);

            let html = '<div class="section-header"><h3>✨ New Artists Discovered <span class="badge">' + discoveries.length + '</span></h3></div>';

            pageDiscoveries.forEach(disc => {
                const date = new Date(disc.first_heard);
                html += `
                    <div class="discovery-card">
                        <div class="discovery-icon">🎵</div>
                        <div class="discovery-info">
                            <h4>${escapeHtml(disc.artist)}</h4>
                            <div class="discovery-meta">
                                First heard: ${date.toLocaleDateString()} • Period: ${escapeHtml(disc.period)}
                            </div>
                        </div>
                        <div class="discovery-plays">
                            <span class="discovery-plays-count">${disc.total_plays}</span>
                            <span class="discovery-plays-label">plays</span>
                        </div>
                    </div>
                `;
            });

            if (discoveries.length > itemsPerPage) {
                html += `
                    <div class="inline-actions" style="align-items: center; gap: 12px; margin-top: 16px;">
                        <button class="ghost" onclick="noveltyDiscoveriesFirstPage()" ${currentPage === 1 ? 'disabled' : ''}>First</button>
                        <button class="ghost" onclick="noveltyDiscoveriesPrevPage()" ${currentPage === 1 ? 'disabled' : ''}>Previous</button>
                        <span class="muted">Page ${currentPage} of ${totalPages}</span>
                        <button class="ghost" onclick="noveltyDiscoveriesNextPage()" ${currentPage === totalPages ? 'disabled' : ''}>Next</button>
                        <button class="ghost" onclick="noveltyDiscoveriesLastPage()" ${currentPage === totalPages ? 'disabled' : ''}>Last</button>
                    </div>
                    <div style="text-align: center; margin-top: 8px; color: var(--muted); font-size: 0.9em;">
                        Showing ${startIdx + 1}-${endIdx} of ${discoveries.length} discoveries
                    </div>
                `;
            }

            container.innerHTML = html;
        }

        function noveltyDiscoveriesFirstPage() {
            window.noveltyDiscoveriesPage = 1;
            renderNoveltyDiscoveries(window.noveltyDiscoveries);
        }

        function noveltyDiscoveriesPrevPage() {
            if (window.noveltyDiscoveriesPage > 1) {
                window.noveltyDiscoveriesPage--;
                renderNoveltyDiscoveries(window.noveltyDiscoveries);
            }
        }

        function noveltyDiscoveriesNextPage() {
            const totalPages = Math.ceil(window.noveltyDiscoveries.length / 20);
            if (window.noveltyDiscoveriesPage < totalPages) {
                window.noveltyDiscoveriesPage++;
                renderNoveltyDiscoveries(window.noveltyDiscoveries);
            }
        }

        function noveltyDiscoveriesLastPage() {
            const totalPages = Math.ceil(window.noveltyDiscoveries.length / 20);
            window.noveltyDiscoveriesPage = totalPages;
            renderNoveltyDiscoveries(window.noveltyDiscoveries);
        }

        // Transitions Report Functions
        async function loadTransitionsReport() {
            const gapMinutes = document.getElementById('transitionsGap').value;
            const minCount = document.getElementById('transitionsMinCount').value;
            const includeSelf = document.getElementById('transitionsSelfTransitions').checked;
            const message = document.getElementById('transitionsMessage');

            message.innerHTML = `<div style="text-align: center; padding: 40px;">
                <div class="loading-spinner" style="margin: 0 auto 12px;"></div>
                <p class="muted">Analyzing artist transitions...</p>
            </div>`;

            try {
                const params = new URLSearchParams({
                    gap_minutes: gapMinutes,
                    min_count: minCount,
                    include_self_transitions: includeSelf
                });
                if (state.customRange) {
                    params.append('start', state.customRange.start);
                    params.append('end', state.customRange.end);
                }

                const response = await fetch(`/api/reports/transitions?${params}`);
                const data = await response.json();

                message.innerHTML = '';
                renderTransitionsSummary(data.summary);
                renderTransitionsTop(data.top_transitions);
                renderTransitionsNetwork(data.network_data);
            } catch (error) {
                message.innerHTML = `<div style="text-align: center; padding: 40px; color: #ef4444;">
                    <div style="font-size: 48px; margin-bottom: 12px;">⚠️</div>
                    <p>Error loading transitions report</p>
                    <p class="muted" style="margin-top: 8px;">${error.message}</p>
                </div>`;
            }
        }

        function renderTransitionsSummary(summary) {
            const container = document.getElementById('transitionsSummary');
            const avgTransitions = summary.avg_transitions_per_session ? summary.avg_transitions_per_session.toFixed(1) : '0';

            container.innerHTML = `
                <div class="metric-cards">
                    <div class="metric-card">
                        <div class="metric-label">Total Transitions</div>
                        <div class="metric-value">${summary.total_transitions.toLocaleString()}</div>
                        <div class="metric-subtext">Artist changes</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Unique Transitions</div>
                        <div class="metric-value">${summary.unique_transitions.toLocaleString()}</div>
                        <div class="metric-subtext">Different flows</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Most Connected</div>
                        <div class="metric-value" style="font-size: 1.3em;">${escapeHtml(summary.most_connected_artist || 'N/A')}</div>
                        <div class="metric-subtext">Hub artist</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Avg per Session</div>
                        <div class="metric-value">${avgTransitions}</div>
                        <div class="metric-subtext">Transitions</div>
                    </div>
                </div>
            `;
        }

        function renderTransitionsTop(transitions) {
            const container = document.getElementById('transitionsTop');
            if (transitions.length === 0) {
                container.innerHTML = '<div class="empty-state"><div class="empty-state-icon">🔀</div><p>No transitions found</p></div>';
                return;
            }

            let html = '<div class="section-header"><h3>🔥 Top Transitions <span class="badge">' + transitions.length + '</span></h3></div>';

            transitions.slice(0, 30).forEach(trans => {
                html += `
                    <div class="transition-flow">
                        <div class="transition-artist">${escapeHtml(trans.from_artist)}</div>
                        <div class="transition-arrow">→</div>
                        <div class="transition-artist">${escapeHtml(trans.to_artist)}</div>
                        <div class="transition-count">${trans.count}×</div>
                    </div>
                `;
            });

            if (transitions.length > 30) {
                html += `<div style="text-align: center; margin-top: 12px; color: var(--muted);">
                    Showing 30 of ${transitions.length} transitions
                </div>`;
            }

            container.innerHTML = html;
        }

        function renderTransitionsNetwork(network) {
            const container = document.getElementById('transitionsNetwork');

            let html = '<div class="section-header"><h3>🕸️ Network Graph</h3></div>';
            html += `
                <div class="network-placeholder">
                    <div class="network-placeholder-icon">🌐</div>
                    <h3 style="color: var(--text); margin-bottom: 8px;">Interactive Network Visualization</h3>
                    <p class="muted" style="margin-bottom: 12px;">
                        ${network.nodes.length} artists • ${network.edges.length} connections
                    </p>
                    <p class="muted">
                        D3.js force-directed graph coming soon!<br>
                        This will show how artists connect through your listening patterns.
                    </p>
                </div>
            `;

            container.innerHTML = html;
        }

        // Heatmap Report Functions
        async function loadHeatmapReport() {
            const timezone = document.getElementById('heatmapTimezone').value;
            const normalize = document.getElementById('heatmapNormalize').checked;
            const message = document.getElementById('heatmapMessage');

            message.innerHTML = `<div style="text-align: center; padding: 40px;">
                <div class="loading-spinner" style="margin: 0 auto 12px;"></div>
                <p class="muted">Generating heatmap...</p>
            </div>`;

            try {
                const params = new URLSearchParams({
                    timezone: timezone,
                    normalize: normalize
                });

                if (state.customRange) {
                    params.append('start', state.customRange.start);
                    params.append('end', state.customRange.end);
                }

                const response = await fetch(`/api/reports/heatmap?${params}`);
                const data = await response.json();

                message.innerHTML = '';
                renderHeatmapSummary(data);
                renderHeatmapViz(data);
            } catch (error) {
                message.innerHTML = `<div style="text-align: center; padding: 40px; color: #ef4444;">
                    <div style="font-size: 48px; margin-bottom: 12px;">⚠️</div>
                    <p>Error loading heatmap</p>
                    <p class="muted" style="margin-top: 8px;">${error.message}</p>
                </div>`;
            }
        }

        function renderHeatmapSummary(data) {
            const container = document.getElementById('heatmapSummary');

            const peakDay = data.peak_day || {};
            const peakHour = data.peak_hour || {};
            const totalScrobbles = data.total_scrobbles || 0;

            container.innerHTML = `
                <div class="heatmap-peak-times">
                    <div class="heatmap-peak">
                        <div class="heatmap-peak-label">Peak Day</div>
                        <div class="heatmap-peak-value">${getDayName(peakDay.day_of_week)}</div>
                        <div class="heatmap-peak-count">${(peakDay.count || 0).toLocaleString()} scrobbles</div>
                    </div>
                    <div class="heatmap-peak">
                        <div class="heatmap-peak-label">Peak Hour</div>
                        <div class="heatmap-peak-value">${formatHour(peakHour.hour)}</div>
                        <div class="heatmap-peak-count">${(peakHour.count || 0).toLocaleString()} scrobbles</div>
                    </div>
                    <div class="heatmap-peak">
                        <div class="heatmap-peak-label">Total Scrobbles</div>
                        <div class="heatmap-peak-value">${totalScrobbles.toLocaleString()}</div>
                        <div class="heatmap-peak-count">in heatmap</div>
                    </div>
                </div>
            `;
        }

        function renderHeatmapViz(data) {
            const container = document.getElementById('heatmapViz');
            const grid = data.grid || [];

            if (grid.length === 0) {
                container.innerHTML = '<div class="empty-state"><div class="empty-state-icon">🔥</div><p>No data available</p></div>';
                return;
            }

            // Find max value for color scaling
            const allCounts = grid.flatMap(row => row.hours.map(h => h.count));
            const maxCount = Math.max(...allCounts);

            const days = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

            let html = '<div class="section-header"><h3>🔥 Activity Heatmap</h3></div>';
            html += '<div class="heatmap-grid"><table class="heatmap-table"><thead><tr><th></th>';

            // Hour headers
            for (let h = 0; h < 24; h++) {
                html += `<th>${h}h</th>`;
            }
            html += '</tr></thead><tbody>';

            // Rows for each day
            grid.forEach(dayData => {
                const dayName = days[dayData.day_of_week] || 'Unknown';
                html += `<tr><td class="heatmap-row-label">${dayName}</td>`;

                dayData.hours.forEach(hourData => {
                    const intensity = maxCount > 0 ? hourData.count / maxCount : 0;
                    const color = getHeatmapColor(intensity);
                    const percentage = data.is_normalized ? `${(hourData.count * 100).toFixed(1)}%` : hourData.count;

                    html += `
                        <td>
                            <div class="heatmap-cell"
                                 style="background: ${color}; color: ${intensity > 0.5 ? '#fff' : 'var(--text)'}"
                                 title="${dayName} ${hourData.hour}:00 - ${percentage}">
                                ${hourData.count > 0 ? (data.is_normalized ? Math.round(hourData.count * 100) : hourData.count) : ''}
                            </div>
                        </td>
                    `;
                });

                html += '</tr>';
            });

            html += '</tbody></table></div>';

            // Legend
            html += `
                <div class="heatmap-legend">
                    <span class="muted">Activity Level:</span>
                    <div class="heatmap-legend-item">
                        <div class="heatmap-legend-box" style="background: ${getHeatmapColor(0)}"></div>
                        <span class="muted">None</span>
                    </div>
                    <div class="heatmap-legend-item">
                        <div class="heatmap-legend-box" style="background: ${getHeatmapColor(0.25)}"></div>
                        <span class="muted">Low</span>
                    </div>
                    <div class="heatmap-legend-item">
                        <div class="heatmap-legend-box" style="background: ${getHeatmapColor(0.5)}"></div>
                        <span class="muted">Medium</span>
                    </div>
                    <div class="heatmap-legend-item">
                        <div class="heatmap-legend-box" style="background: ${getHeatmapColor(0.75)}"></div>
                        <span class="muted">High</span>
                    </div>
                    <div class="heatmap-legend-item">
                        <div class="heatmap-legend-box" style="background: ${getHeatmapColor(1)}"></div>
                        <span class="muted">Peak</span>
                    </div>
                </div>
            `;

            container.innerHTML = html;
        }

        function getHeatmapColor(intensity) {
            if (intensity === 0) return 'var(--tile)';

            // Color scale from dark blue to bright cyan
            const colors = [
                'rgba(125, 211, 252, 0.1)',  // 0-20%
                'rgba(125, 211, 252, 0.3)',  // 20-40%
                'rgba(125, 211, 252, 0.5)',  // 40-60%
                'rgba(125, 211, 252, 0.7)',  // 60-80%
                'rgba(125, 211, 252, 0.9)',  // 80-100%
            ];

            const index = Math.min(Math.floor(intensity * 5), 4);
            return colors[index];
        }

        function getDayName(dayIndex) {
            const days = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
            return days[dayIndex] || 'Unknown';
        }

        function formatHour(hour) {
            if (hour === 0) return '12 AM';
            if (hour < 12) return `${hour} AM`;
            if (hour === 12) return '12 PM';
            return `${hour - 12} PM`;
        }

        // Diversity Report Functions
        async function loadDiversityReport() {
            const granularity = document.getElementById('diversityGranularity').value;
            const message = document.getElementById('diversityMessage');

            message.innerHTML = `<div style="text-align: center; padding: 40px;">
                <div class="loading-spinner" style="margin: 0 auto 12px;"></div>
                <p class="muted">Analyzing diversity...</p>
            </div>`;

            try {
                const params = new URLSearchParams({ granularity });

                if (state.customRange) {
                    params.append('start', state.customRange.start);
                    params.append('end', state.customRange.end);
                }

                const response = await fetch(`/api/reports/diversity?${params}`);
                const data = await response.json();

                message.innerHTML = '';
                renderDiversitySummary(data.summary);
                renderDiversityTimeline(data.timeline);
            } catch (error) {
                message.innerHTML = `<div style="text-align: center; padding: 40px; color: #ef4444;">
                    <div style="font-size: 48px; margin-bottom: 12px;">⚠️</div>
                    <p>Error loading diversity report</p>
                    <p class="muted" style="margin-top: 8px;">${error.message}</p>
                </div>`;
            }
        }

        function renderDiversitySummary(summary) {
            const container = document.getElementById('diversitySummary');

            container.innerHTML = `
                <div class="metric-cards">
                    <div class="metric-card">
                        <div class="metric-label">Diversity Score</div>
                        <div class="metric-value">${summary.avg_diversity_score.toFixed(1)}</div>
                        <div class="metric-subtext">Average (0-100)</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Shannon Entropy</div>
                        <div class="metric-value">${summary.avg_shannon_entropy.toFixed(2)}</div>
                        <div class="metric-subtext">Average variety</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Gini Coefficient</div>
                        <div class="metric-value">${summary.avg_gini_coefficient.toFixed(2)}</div>
                        <div class="metric-subtext">Concentration (0-1)</div>
                    </div>
                    <div class="metric-card">
                        <div class="metric-label">Unique Artists</div>
                        <div class="metric-value">${summary.total_unique_artists.toLocaleString()}</div>
                        <div class="metric-subtext">All time</div>
                    </div>
                </div>
                <div style="margin-top: 20px; padding: 16px; background: var(--tile); border-radius: 8px; border: 1px solid var(--border);">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div>
                            <div style="color: var(--muted); font-size: 0.9em;">Most Diverse Period</div>
                            <div style="font-size: 1.5em; font-weight: 700; color: #22c55e;">${summary.most_diverse_period}</div>
                        </div>
                        <div style="text-align: right;">
                            <div style="color: var(--muted); font-size: 0.9em;">Least Diverse Period</div>
                            <div style="font-size: 1.5em; font-weight: 700; color: #ef4444;">${summary.least_diverse_period}</div>
                        </div>
                    </div>
                </div>
            `;
        }

        function renderDiversityTimeline(timeline) {
            const container = document.getElementById('diversityTimeline');

            if (timeline.length === 0) {
                container.innerHTML = '<div class="empty-state"><div class="empty-state-icon">📊</div><p>No data available</p></div>';
                return;
            }

            let html = '<div class="section-header"><h3>📈 Diversity Timeline</h3></div>';
            html += '<div class="diversity-chart">';

            timeline.forEach(point => {
                const scorePercent = point.diversity_score;

                html += `
                    <div class="diversity-bar-container">
                        <div class="diversity-label">${point.period}</div>
                        <div class="diversity-bar">
                            <div class="diversity-bar-fill" style="width: ${scorePercent}%">
                                ${scorePercent > 20 ? scorePercent.toFixed(1) : ''}
                            </div>
                        </div>
                        <div class="diversity-value">${scorePercent.toFixed(1)}</div>
                    </div>
                `;
            });

            html += '</div>';

            container.innerHTML = html;
        }

        // Yearly Report Functions
        function initYearlyTab() {
            loadAvailableYears();
        }

        async function loadAvailableYears() {
            try {
                const response = await fetch('/api/years');
                const years = await response.json();

                if (years.length === 0) {
                    // No data, use current year as fallback
                    years.push(new Date().getFullYear());
                }

                populateYearSelectors(years);

                // Set default to most recent year
                document.getElementById('yearlyYear').value = years[0];
                loadYearlyReport();
            } catch (error) {
                console.error('Error loading available years:', error);
                // Fallback to current year
                const currentYear = new Date().getFullYear();
                populateYearSelectors([currentYear]);
                document.getElementById('yearlyYear').value = currentYear;
                loadYearlyReport();
            }
        }

        function populateYearSelectors(years) {
            const selectors = ['yearlyYear', 'compareYear1', 'compareYear2'];
            selectors.forEach(id => {
                const select = document.getElementById(id);
                select.innerHTML = years.map(y => `<option value="${y}">${y}</option>`).join('');
            });

            // Set defaults for comparison
            if (years.length > 0) {
                document.getElementById('compareYear1').value = years[0];
                if (years.length > 1) {
                    document.getElementById('compareYear2').value = years[1];
                }
            }
        }

        async function loadYearlyReport() {
            const year = document.getElementById('yearlyYear').value;
            const message = document.getElementById('yearlyMessage');
            const content = document.getElementById('yearlyContent');

            message.innerHTML = `<div style="text-align: center; padding: 40px;">
                <div class="loading-spinner" style="margin: 0 auto 12px;"></div>
                <p class="muted">Generating your ${year} wrapped...</p>
            </div>`;
            content.innerHTML = '';

            try {
                const response = await fetch(`/api/reports/yearly/${year}`);
                const data = await response.json();

                message.innerHTML = '';
                renderYearlyReport(data);
            } catch (error) {
                message.innerHTML = `<div style="text-align: center; padding: 40px; color: #ef4444;">
                    <div style="font-size: 48px; margin-bottom: 12px;">⚠️</div>
                    <p>Error loading yearly report</p>
                    <p class="muted" style="margin-top: 8px;">${error.message}</p>
                </div>`;
            }
        }

        function renderYearlyReport(report) {
            const container = document.getElementById('yearlyContent');

            if (report.overview.total_scrobbles === 0) {
                container.innerHTML = '<div class="empty-state"><div class="empty-state-icon">🎉</div><p>No data for this year</p></div>';
                return;
            }

            const hours = Math.floor(report.overview.total_minutes / 60);

            let html = `
                <div class="yearly-hero">
                    <div class="yearly-hero-title">${report.year}</div>
                    <div class="yearly-hero-subtitle">Your Year in Music 🎵</div>
                </div>

                <div class="yearly-stats-grid">
                    <div class="yearly-stat-card">
                        <div class="yearly-stat-value">${report.overview.total_scrobbles.toLocaleString()}</div>
                        <div class="yearly-stat-label">Songs Played</div>
                    </div>
                    <div class="yearly-stat-card">
                        <div class="yearly-stat-value">${hours.toLocaleString()}</div>
                        <div class="yearly-stat-label">Hours Listened</div>
                    </div>
                    <div class="yearly-stat-card">
                        <div class="yearly-stat-value">${report.overview.total_artists.toLocaleString()}</div>
                        <div class="yearly-stat-label">Artists</div>
                    </div>
                    <div class="yearly-stat-card">
                        <div class="yearly-stat-value">${Math.round(report.overview.average_per_day)}</div>
                        <div class="yearly-stat-label">Songs per Day</div>
                    </div>
                </div>
            `;

            // Milestones
            if (report.milestones.length > 0) {
                html += '<div class="yearly-section">';
                html += '<div class="yearly-section-title">🏆 Your Milestones</div>';
                html += '<div class="yearly-milestone-grid">';
                report.milestones.forEach(milestone => {
                    html += `
                        <div class="yearly-milestone">
                            <div class="yearly-milestone-icon">${milestone.icon}</div>
                            <div class="yearly-milestone-title">${milestone.title}</div>
                            <div class="yearly-milestone-desc">${milestone.description}</div>
                            <div class="yearly-milestone-value">${milestone.value}</div>
                        </div>
                    `;
                });
                html += '</div></div>';
            }

            // Top Artists
            html += '<div class="yearly-section">';
            html += '<div class="yearly-section-title">🎤 Your Top Artists</div>';
            html += '<div class="yearly-top-list">';
            report.top_content.top_artists.slice(0, 10).forEach(artist => {
                html += `
                    <div class="yearly-top-item">
                        <div class="yearly-rank">#${artist.rank}</div>
                        <div class="yearly-top-info">
                            <div class="yearly-top-name">${escapeHtml(artist.artist)}</div>
                            <div class="yearly-top-meta">${artist.percentage.toFixed(1)}% of your listening</div>
                        </div>
                        <div class="yearly-top-count">${artist.play_count} plays</div>
                    </div>
                `;
            });
            html += '</div></div>';

            // Top Tracks
            html += '<div class="yearly-section">';
            html += '<div class="yearly-section-title">🎵 Your Top Tracks</div>';
            html += '<div class="yearly-top-list">';
            report.top_content.top_tracks.slice(0, 10).forEach(track => {
                html += `
                    <div class="yearly-top-item">
                        <div class="yearly-rank">#${track.rank}</div>
                        <div class="yearly-top-info">
                            <div class="yearly-top-name">${escapeHtml(track.track)}</div>
                            <div class="yearly-top-meta">by ${escapeHtml(track.artist)}</div>
                        </div>
                        <div class="yearly-top-count">${track.play_count} plays</div>
                    </div>
                `;
            });
            html += '</div></div>';

            // Discoveries
            html += '<div class="yearly-section">';
            html += '<div class="yearly-section-title">🗺️ New Discoveries</div>';
            html += `<div style="padding: 20px; background: var(--tile); border-radius: 8px; border: 1px solid var(--border);">`;
            html += `<div style="font-size: 2em; font-weight: 700; color: var(--accent); margin-bottom: 8px;">
                ${report.discoveries.new_artists} new artists
            </div>`;
            if (report.discoveries.first_artist) {
                html += `<p class="muted">First discovery: ${escapeHtml(report.discoveries.first_artist.artist)}
                    on ${new Date(report.discoveries.first_artist.timestamp).toLocaleDateString()}</p>`;
            }
            if (report.discoveries.top_discovery) {
                html += `<p class="muted" style="margin-top: 8px;">Biggest find: ${escapeHtml(report.discoveries.top_discovery.artist)}
                    (${report.discoveries.top_discovery.plays_this_year} plays)</p>`;
            }
            html += `</div></div>`;

            container.innerHTML = html;
        }

        // Entity Detail Modal Functions
        function openEntityModal(type, artist, nameOrAlbum) {
            const modal = document.getElementById('entityModal');
            modal.classList.add('active');
            document.body.style.overflow = 'hidden';

            if (type === 'artist') {
                loadArtistDetail(artist);
            } else if (type === 'album') {
                loadAlbumDetail(artist, nameOrAlbum);
            } else if (type === 'track') {
                loadTrackDetail(artist, nameOrAlbum);
            }
        }

        function closeEntityModal() {
            const modal = document.getElementById('entityModal');
            modal.classList.remove('active');
            document.body.style.overflow = '';
        }

        // Close modal when clicking outside
        document.addEventListener('click', (e) => {
            const modal = document.getElementById('entityModal');
            if (e.target === modal) {
                closeEntityModal();
            }
        });

        // Close modal with Escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                closeEntityModal();
            }
        });

        async function loadArtistDetail(artist) {
            document.getElementById('entityModalTitle').textContent = artist;
            document.getElementById('entityModalSubtitle').textContent = 'Artist';
            document.getElementById('entityModalBody').innerHTML = '<div class="loading-overlay">Loading artist details...</div>';

            try {
                const response = await fetch(`/api/artist/${encodeURIComponent(artist)}`);
                const data = await response.json();

                renderEntityImage(data.image_url, artist);
                renderArtistDetail(data);
            } catch (error) {
                console.error('Error loading artist details:', error);
                document.getElementById('entityModalBody').innerHTML = '<div class="error">Failed to load artist details.</div>';
            }
        }

        async function loadAlbumDetail(artist, album) {
            document.getElementById('entityModalTitle').textContent = album;
            document.getElementById('entityModalSubtitle').textContent = artist;
            document.getElementById('entityModalBody').innerHTML = '<div class="loading-overlay">Loading album details...</div>';

            try {
                const response = await fetch(`/api/album/${encodeURIComponent(artist)}/${encodeURIComponent(album)}`);
                const data = await response.json();

                renderEntityImage(data.image_url, album);
                renderAlbumDetail(data);
            } catch (error) {
                console.error('Error loading album details:', error);
                document.getElementById('entityModalBody').innerHTML = '<div class="error">Failed to load album details.</div>';
            }
        }

        async function loadTrackDetail(artist, track) {
            document.getElementById('entityModalTitle').textContent = track;
            document.getElementById('entityModalSubtitle').textContent = artist;
            document.getElementById('entityModalBody').innerHTML = '<div class="loading-overlay">Loading track details...</div>';

            try {
                const response = await fetch(`/api/track/${encodeURIComponent(artist)}/${encodeURIComponent(track)}`);
                const data = await response.json();

                renderEntityImage(data.image_url, track);
                renderTrackDetail(data);
            } catch (error) {
                console.error('Error loading track details:', error);
                document.getElementById('entityModalBody').innerHTML = '<div class="error">Failed to load track details.</div>';
            }
        }

        function renderEntityImage(imageUrl, name) {
            const container = document.getElementById('entityModalImage');
            if (imageUrl) {
                container.innerHTML = `<img class="modal-image" src="${escapeHtml(imageUrl)}" alt="" loading="lazy" referrerpolicy="no-referrer">`;
            } else {
                container.innerHTML = `<div class="modal-image-placeholder">${getInitials(name)}</div>`;
            }
        }

        function renderArtistDetail(data) {
            const stats = data.stats;
            const firstDate = stats.first_scrobble ? new Date(stats.first_scrobble * 1000).toLocaleDateString() : 'N/A';

            let html = '<div class="entity-stats-grid">';
            html += `
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.total_scrobbles.toLocaleString()}</div>
                    <div class="entity-stat-label">Total Plays</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.unique_tracks.toLocaleString()}</div>
                    <div class="entity-stat-label">Tracks</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.unique_albums.toLocaleString()}</div>
                    <div class="entity-stat-label">Albums</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${firstDate}</div>
                    <div class="entity-stat-label">First Played</div>
                </div>
            `;
            html += '</div>';

            // Top tracks
            if (data.top_tracks && data.top_tracks.length > 0) {
                html += '<div class="entity-section">';
                html += '<div class="entity-section-title">🎵 Top Tracks</div>';
                html += '<div class="entity-list">';
                data.top_tracks.slice(0, 10).forEach((track, idx) => {
                    html += `
                        <div class="entity-list-item clickable" onclick="openEntityModal('track', '${escapeHtml(stats.artist).replace(/'/g, "\\'")}', '${escapeHtml(track.name).replace(/'/g, "\\'")}')">
                            <div class="entity-list-rank">#${idx + 1}</div>
                            <div class="entity-list-info">
                                <div class="entity-list-name">${escapeHtml(track.name)}</div>
                            </div>
                            <div class="entity-list-count">${track.count.toLocaleString()} plays</div>
                        </div>
                    `;
                });
                html += '</div></div>';
            }

            // Top albums
            if (data.top_albums && data.top_albums.length > 0) {
                html += '<div class="entity-section">';
                html += '<div class="entity-section-title">💿 Top Albums</div>';
                html += '<div class="entity-list">';
                data.top_albums.slice(0, 10).forEach((album, idx) => {
                    const imageHtml = album.image_url
                        ? `<img class="entity-list-image" src="${escapeHtml(album.image_url)}" alt="" loading="lazy" referrerpolicy="no-referrer">`
                        : '';
                    html += `
                        <div class="entity-list-item clickable" onclick="openEntityModal('album', '${escapeHtml(stats.artist).replace(/'/g, "\\'")}', '${escapeHtml(album.name).replace(/'/g, "\\'")}')">
                            ${imageHtml}
                            <div class="entity-list-rank">#${idx + 1}</div>
                            <div class="entity-list-info">
                                <div class="entity-list-name">${escapeHtml(album.name)}</div>
                            </div>
                            <div class="entity-list-count">${album.count.toLocaleString()} plays</div>
                        </div>
                    `;
                });
                html += '</div></div>';
            }

            document.getElementById('entityModalBody').innerHTML = html;
        }

        function renderAlbumDetail(data) {
            const stats = data.stats;
            const firstDate = stats.first_scrobble ? new Date(stats.first_scrobble * 1000).toLocaleDateString() : 'N/A';

            let html = '<div class="entity-stats-grid">';
            html += `
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.total_scrobbles.toLocaleString()}</div>
                    <div class="entity-stat-label">Total Plays</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.unique_tracks.toLocaleString()}</div>
                    <div class="entity-stat-label">Tracks</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${firstDate}</div>
                    <div class="entity-stat-label">First Played</div>
                </div>
            `;
            html += '</div>';

            // Track list
            if (data.tracks && data.tracks.length > 0) {
                html += '<div class="entity-section">';
                html += '<div class="entity-section-title">🎵 Tracks</div>';
                html += '<div class="entity-list">';
                data.tracks.forEach((track, idx) => {
                    html += `
                        <div class="entity-list-item clickable" onclick="openEntityModal('track', '${escapeHtml(stats.artist).replace(/'/g, "\\'")}', '${escapeHtml(track.name).replace(/'/g, "\\'")}')">
                            <div class="entity-list-rank">#${idx + 1}</div>
                            <div class="entity-list-info">
                                <div class="entity-list-name">${escapeHtml(track.name)}</div>
                            </div>
                            <div class="entity-list-count">${track.count.toLocaleString()} plays</div>
                        </div>
                    `;
                });
                html += '</div></div>';
            }

            document.getElementById('entityModalBody').innerHTML = html;
        }

        function renderTrackDetail(data) {
            const stats = data.stats;
            const firstDate = stats.first_scrobble ? new Date(stats.first_scrobble * 1000).toLocaleDateString() : 'N/A';
            const lastDate = stats.last_scrobble ? new Date(stats.last_scrobble * 1000).toLocaleDateString() : 'N/A';

            let html = '<div class="entity-stats-grid">';
            html += `
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${stats.total_scrobbles.toLocaleString()}</div>
                    <div class="entity-stat-label">Total Plays</div>
                </div>
            `;
            if (stats.album) {
                html += `
                    <div class="entity-stat-card">
                        <div class="entity-stat-value" style="font-size: 1.2em;">${escapeHtml(stats.album)}</div>
                        <div class="entity-stat-label">Album</div>
                    </div>
                `;
            }
            html += `
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${firstDate}</div>
                    <div class="entity-stat-label">First Played</div>
                </div>
                <div class="entity-stat-card">
                    <div class="entity-stat-value">${lastDate}</div>
                    <div class="entity-stat-label">Last Played</div>
                </div>
            `;
            html += '</div>';

            document.getElementById('entityModalBody').innerHTML = html;
        }
