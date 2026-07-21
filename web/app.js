const app = document.getElementById('app');
const connectionBadge = document.getElementById('connectionBadge');
const newGameButton = document.getElementById('newGameButton');
const resultDialog = document.getElementById('resultDialog');
const dialogLabel = document.getElementById('dialogLabel');
const dialogTitle = document.getElementById('dialogTitle');
const dialogMessage = document.getElementById('dialogMessage');
const dialogDelta = document.getElementById('dialogDelta');
const dialogClose = document.getElementById('dialogClose');

let scenario = null;
let state = null;
let requestInFlight = false;
const sessionStorageKey = 'civic-sim-session-id';
const memoryStorage = new Map();

function storageGet(key) {
  try { return localStorage.getItem(key); }
  catch { return memoryStorage.get(key) ?? null; }
}

function storageSet(key, value) {
  try { localStorage.setItem(key, value); }
  catch { memoryStorage.set(key, String(value)); }
}

function storageRemove(key) {
  try { localStorage.removeItem(key); }
  catch { memoryStorage.delete(key); }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function formatMoney(value) {
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency: 'INR',
    maximumFractionDigits: 0,
  }).format(value);
}

function createClientActionId() {
  if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID();
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers || {}),
    },
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(body.error || `Request failed (${response.status})`);
  return body;
}

function setConnection(isOnline, label = null) {
  connectionBadge.textContent = label || (isOnline ? 'Server online' : 'Server unavailable');
  connectionBadge.className = `status-badge ${isOnline ? 'online' : 'offline'}`;
}

function renderError(message) {
  app.innerHTML = `
    <section class="empty-panel">
      <p class="eyebrow">CONNECTION ERROR</p>
      <h2>The game could not be loaded</h2>
      <p>${escapeHtml(message)}</p>
      <button id="retryButton" class="button button-primary" type="button">Retry</button>
    </section>`;
  document.getElementById('retryButton').addEventListener('click', boot);
}

function renderProfileSelection() {
  newGameButton.hidden = true;
  app.innerHTML = `
    <section class="hero-card">
      <p class="eyebrow">${escapeHtml(scenario.title)}</p>
      <h2>${escapeHtml(scenario.mission.title)}</h2>
      <p class="lead">${escapeHtml(scenario.description)}</p>
      <div class="profile-grid">
        ${scenario.citizens.map((citizen) => `
          <article class="profile-card">
            <span class="occupation">${escapeHtml(citizen.occupation)}</span>
            <h3>${escapeHtml(citizen.name)}</h3>
            <p>${escapeHtml(citizen.context)}</p>
            <div class="resource-preview">
              <span class="chip">${formatMoney(citizen.starting_resources.money)}</span>
              <span class="chip">${citizen.starting_resources.energy} energy</span>
              <span class="chip">${citizen.starting_resources.influence} influence</span>
              <span class="chip">${citizen.starting_resources.days_remaining} days</span>
            </div>
            <button class="button button-primary select-profile" data-citizen-id="${escapeHtml(citizen.id)}" type="button">Play as ${escapeHtml(citizen.name.split(' ')[0])}</button>
          </article>`).join('')}
      </div>
    </section>`;

  document.querySelectorAll('.select-profile').forEach((button) => {
    button.addEventListener('click', () => startSession(button.dataset.citizenId));
  });
}

function costHtml(cost) {
  const entries = [];
  if (cost.money) entries.push(`<span><strong>${formatMoney(cost.money)}</strong></span>`);
  if (cost.energy) entries.push(`<span>${cost.energy} energy</span>`);
  if (cost.influence) entries.push(`<span>${cost.influence} influence</span>`);
  if (cost.days) entries.push(`<span>${cost.days} days</span>`);
  return entries.join('');
}

function metricBar(label, value) {
  return `
    <div class="stat-row">
      <div class="stat-header"><span>${escapeHtml(label)}</span><strong>${value}/100</strong></div>
      <div class="mini-track"><div class="mini-fill" style="width:${Math.max(0, Math.min(100, value))}%"></div></div>
    </div>`;
}

function renderGame() {
  newGameButton.hidden = false;
  const statusClass = state.status === 'won' ? 'won' : 'lost';
  const finished = state.status !== 'active';
  const newestEvents = [...state.events].reverse();

  app.innerHTML = `
    <div class="game-layout">
      <div class="main-column">
        <section class="panel mission-panel">
          <div class="panel-inner">
            <p class="eyebrow">CURRENT MISSION</p>
            <h2>${escapeHtml(state.mission_title)}</h2>
            <p>${escapeHtml(state.objective)}</p>
            <div class="citizen-strip">
              <div><strong>${escapeHtml(state.citizen_name)}</strong><span>${escapeHtml(state.citizen_context)}</span></div>
              <div class="turn-number"><span>Turn</span><strong>${state.turn}</strong></div>
            </div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-inner">
            <div class="resource-grid">
              <div class="metric-card"><div class="label">Money</div><div class="value">${formatMoney(state.resources.money)}</div></div>
              <div class="metric-card"><div class="label">Energy</div><div class="value">${state.resources.energy}</div></div>
              <div class="metric-card"><div class="label">Influence</div><div class="value">${state.resources.influence}</div></div>
              <div class="metric-card"><div class="label">Days left</div><div class="value">${state.resources.days_remaining}</div></div>
            </div>
            <div class="progress-section">
              <div class="progress-heading"><span>Mission progress</span><span>${state.metrics.progress}%</span></div>
              <div class="progress-track"><div class="progress-fill" style="width:${state.metrics.progress}%"></div></div>
              <p class="status-text">${escapeHtml(state.current_status)}</p>
            </div>
          </div>
        </section>

        ${finished ? `
          <section class="end-banner ${statusClass}">
            <h3>${state.status === 'won' ? 'Mission completed' : 'Mission failed'}</h3>
            <p>${escapeHtml(state.current_status)}</p>
            <button id="restartButton" class="button ${state.status === 'won' ? 'button-primary' : 'button-danger'}" type="button">Restart with the same citizen</button>
          </section>` : `
          <section class="panel">
            <div class="panel-inner">
              <div class="section-title"><h3>Choose the next action</h3><span>Outcomes are uncertain</span></div>
              <div class="action-list">
                ${state.available_actions.map((action) => `
                  <button class="action-button" type="button" data-action-id="${escapeHtml(action.id)}" ${action.enabled && !requestInFlight ? '' : 'disabled'}>
                    <span>
                      <span class="action-title">${escapeHtml(action.title)}</span>
                      <span class="action-description">${escapeHtml(action.description)}</span>
                    </span>
                    <span class="action-cost">${costHtml(action.cost)}</span>
                    ${action.disabled_reason ? `<span class="disabled-reason">${escapeHtml(action.disabled_reason)}</span>` : ''}
                  </button>`).join('')}
              </div>
            </div>
          </section>`}
      </div>

      <aside class="side-column">
        <section class="panel">
          <div class="panel-inner">
            <div class="section-title"><h3>Civic capacity</h3><span>Public knowledge</span></div>
            <div class="secondary-metrics">
              ${metricBar('Documentation', state.metrics.documentation)}
              ${metricBar('Community support', state.metrics.community_support)}
              ${metricBar('Public attention', state.metrics.public_attention)}
              ${metricBar('Integrity', state.metrics.integrity)}
            </div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-inner">
            <div class="section-title"><h3>Case history</h3><span>${state.events.length} events</span></div>
            <div class="timeline">
              ${newestEvents.length ? newestEvents.map((event) => `
                <article class="timeline-item">
                  <h4>Turn ${event.turn}: ${escapeHtml(event.action_title)}</h4>
                  <p>${escapeHtml(event.message)}</p>
                  <div class="event-meta">${event.progress_change >= 0 ? '+' : ''}${event.progress_change}% progress · ${event.resources_after.days_remaining} days left</div>
                </article>`).join('') : '<p class="action-description">No official action has been taken yet.</p>'}
            </div>
          </div>
        </section>
      </aside>
    </div>`;

  document.querySelectorAll('.action-button[data-action-id]').forEach((button) => {
    button.addEventListener('click', () => takeAction(button.dataset.actionId));
  });
  document.getElementById('restartButton')?.addEventListener('click', resetSession);
}

async function startSession(citizenId) {
  if (requestInFlight) return;
  requestInFlight = true;
  try {
    const created = await api('/api/v1/sessions', {
      method: 'POST',
      body: JSON.stringify({ citizen_id: citizenId }),
    });
    state = created;
    storageSet(sessionStorageKey, state.id);
    requestInFlight = false;
    renderGame();
  } catch (error) {
    alert(error.message);
  } finally {
    requestInFlight = false;
  }
}

async function takeAction(actionId) {
  if (requestInFlight || state.status !== 'active') return;
  requestInFlight = true;
  renderGame();
  try {
    const result = await api(`/api/v1/sessions/${encodeURIComponent(state.id)}/actions`, {
      method: 'POST',
      body: JSON.stringify({
        action_id: actionId,
        client_action_id: createClientActionId(),
      }),
    });
    state = result.state;
    renderGame();
    dialogLabel.textContent = state.status === 'won' ? 'MISSION COMPLETE' : state.status === 'lost' ? 'MISSION ENDED' : 'STOCHASTIC OUTCOME';
    dialogTitle.textContent = result.progress_change > 12 ? 'Major movement' : result.progress_change > 0 ? 'The case moved' : 'No clear progress';
    dialogMessage.textContent = result.message;
    dialogDelta.textContent = `${result.progress_change >= 0 ? '+' : ''}${result.progress_change}% mission progress`;
    if (typeof resultDialog.showModal === 'function') resultDialog.showModal();
  } catch (error) {
    alert(error.message);
    await restoreSession();
  } finally {
    requestInFlight = false;
    renderGame();
  }
}

async function resetSession() {
  if (!state || requestInFlight) return;
  requestInFlight = true;
  try {
    state = await api(`/api/v1/sessions/${encodeURIComponent(state.id)}/reset`, { method: 'POST', body: '{}' });
    requestInFlight = false;
    renderGame();
  } catch (error) {
    alert(error.message);
  } finally {
    requestInFlight = false;
  }
}

async function restoreSession() {
  const sessionId = storageGet(sessionStorageKey);
  if (!sessionId) return false;
  try {
    state = await api(`/api/v1/sessions/${encodeURIComponent(sessionId)}`);
    renderGame();
    return true;
  } catch {
    storageRemove(sessionStorageKey);
    return false;
  }
}

async function boot() {
  app.innerHTML = '<section class="loading-panel"><div class="spinner" aria-hidden="true"></div><p>Loading the civic simulation…</p></section>';
  try {
    const [health, loadedScenario] = await Promise.all([
      api('/api/v1/health'),
      api('/api/v1/scenario'),
    ]);
    scenario = loadedScenario;
    setConnection(true, `${health.sessions} saved session${health.sessions === 1 ? '' : 's'}`);
    const restored = await restoreSession();
    if (!restored) renderProfileSelection();
  } catch (error) {
    setConnection(false);
    renderError(error.message);
  }
}

newGameButton.addEventListener('click', () => {
  if (!confirm('Start a new game? The current session remains saved on the server.')) return;
  state = null;
  storageRemove(sessionStorageKey);
  renderProfileSelection();
});

dialogClose.addEventListener('click', () => resultDialog.close());
window.addEventListener('online', () => setConnection(true));
window.addEventListener('offline', () => setConnection(false));

if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => navigator.serviceWorker.register('/sw.js').catch(() => {}));
}

boot();
