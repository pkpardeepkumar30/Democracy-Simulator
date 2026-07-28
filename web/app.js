const app = document.getElementById('app');
const connectionBadge = document.getElementById('connectionBadge');
const newGameButton = document.getElementById('newGameButton');
const resultDialog = document.getElementById('resultDialog');
const dialogLabel = document.getElementById('dialogLabel');
const dialogTitle = document.getElementById('dialogTitle');
const dialogMessage = document.getElementById('dialogMessage');
const dialogDelta = document.getElementById('dialogDelta');
const dialogClose = document.getElementById('dialogClose');

let scenarios = [];
let generatorCatalog = null;
let campaigns = [];
let activeCampaignId = null;
let campaignsSupported = true;
let scenario = null;
let state = null;
let requestInFlight = false;
let cityRenderer = null;
let pendingVisualEvent = null;
const sessionStorageKey = 'civic-sim-session-id';
const campaignStorageKey = 'civic-sim-campaign-id';
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

function destroyCityRenderer() {
  try { cityRenderer?.destroy(); }
  catch { /* Phaser cleanup must never block the playable DOM UI. */ }
  cityRenderer = null;
}

function mountCityRenderer() {
  const host = document.getElementById('cityCanvasHost');
  const fallback = document.getElementById('cityDomFallback');
  if (!host || typeof globalThis.CivicCityRenderer !== 'function') return;
  try {
    cityRenderer = new globalThis.CivicCityRenderer({
      parent: host,
      scenario,
      state,
      visualEvent: pendingVisualEvent,
      locked: requestInFlight,
      onAction: (actionId) => takeAction(actionId),
    });
    fallback?.setAttribute('hidden', '');
  } catch (error) {
    console.warn('Visual city unavailable; using DOM fallback.', error);
    host.remove();
    fallback?.removeAttribute('hidden');
  }
}

function formatMoney(value) {
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency: 'INR',
    maximumFractionDigits: 0,
  }).format(value);
}

function applyScenarioTheme() {
  const theme = scenario?.visual_theme;
  document.documentElement.style.setProperty('--navy', theme?.primary_color || '#172c3f');
  document.documentElement.style.setProperty('--saffron', theme?.accent_color || '#bb6b22');
  document.documentElement.style.setProperty('--paper', theme?.background_color || '#f4f1e9');
}

function renderScenarioSelection() {
  destroyCityRenderer();
  scenario = null;
  applyScenarioTheme();
  newGameButton.hidden = true;
  app.innerHTML = `
    <section class="hero-card">
      <p class="eyebrow">WORLD-SCALE CIVIC SIMULATION</p>
      <h2>Choose an environment</h2>
      <p class="lead">The same civic tools behave differently across political systems, institutions, economies and administrative cultures.</p>
      ${campaignsSupported ? `<div class="campaign-panel">
        <div>
          <strong>Cross-mission campaign</strong>
          <span>Link missions to preserve their full history and carry civic reputation, institutional knowledge and networks forward.</span>
        </div>
        <label><span>Current campaign</span><select id="campaignSelect">
          <option value="">One-off mission</option>
          ${campaigns.map((item) => `<option value="${escapeHtml(item.id)}" ${item.id === activeCampaignId ? 'selected' : ''}>${escapeHtml(item.name)} — ${item.mission_count} completed</option>`).join('')}
        </select></label>
        <form id="campaignForm"><input name="name" maxlength="80" required placeholder="New campaign name"><button class="button button-secondary" type="submit">Create</button></form>
      </div>` : ''}
      ${generatorCatalog?.templates?.length ? `<div class="generator-callout">
        <div><strong>Compose a new world</strong><span>Choose the civic setting yourself or let the server build a seeded random environment.</span></div>
        <button id="openGenerator" class="button button-primary" type="button">Generate world</button>
      </div>` : ''}
      <div class="scenario-grid">
        ${scenarios.map((item) => `
          <article class="scenario-card" style="--scenario-color:${escapeHtml(item.visual_theme?.primary_color || '#172c3f')};--scenario-accent:${escapeHtml(item.visual_theme?.accent_color || '#bb6b22')}">
            <span class="occupation">${escapeHtml(item.world_region || 'Configurable environment')}</span>
            ${item.generated ? '<span class="generated-badge">GENERATED</span>' : ''}
            <h3>${escapeHtml(item.title)}</h3>
            <p>${escapeHtml(item.description)}</p>
            <div class="resource-preview">
              <span class="chip">${escapeHtml(item.objective_type || 'Civic objective')}</span>
              <span class="chip">${item.role_count} player roles</span>
            </div>
            <button class="button button-primary select-scenario" data-scenario-id="${escapeHtml(item.id)}" type="button">Explore environment</button>
          </article>`).join('')}
      </div>
    </section>`;

  document.querySelectorAll('.select-scenario').forEach((button) => {
    button.addEventListener('click', () => selectScenario(button.dataset.scenarioId));
  });
  document.getElementById('openGenerator')?.addEventListener('click', renderGenerator);
  document.getElementById('campaignSelect')?.addEventListener('change', (event) => {
    activeCampaignId = event.target.value || null;
    if (activeCampaignId) storageSet(campaignStorageKey, activeCampaignId);
    else storageRemove(campaignStorageKey);
  });
  document.getElementById('campaignForm')?.addEventListener('submit', createCampaign);
}

async function createCampaign(event) {
  event.preventDefault();
  if (requestInFlight) return;
  const form = event.currentTarget;
  const name = new FormData(form).get('name');
  requestInFlight = true;
  try {
    const campaign = await api('/api/v1/campaigns', { method: 'POST', body: JSON.stringify({ name }) });
    campaigns.push({ id: campaign.id, name: campaign.name, mission_count: 0, values: {} });
    activeCampaignId = campaign.id;
    storageSet(campaignStorageKey, campaign.id);
    renderScenarioSelection();
  } catch (error) {
    alert(error.message);
  } finally {
    requestInFlight = false;
  }
}

const generatorFields = [
  ['city_plan', 'Real city plan'],
  ['world_region', 'World region'],
  ['political_system', 'Political system'],
  ['administrative_capacity', 'Administrative capacity'],
  ['corruption_structure', 'Corruption structure'],
  ['rule_of_law', 'Rule of law'],
  ['media_environment', 'Media environment'],
  ['player_role', 'Player role'],
  ['objective_type', 'Civic objective'],
];

function renderGenerator() {
  destroyCityRenderer();
  if (!generatorCatalog) return;
  newGameButton.hidden = true;
  app.innerHTML = `
    <section class="hero-card">
      <p class="eyebrow">SCENARIO COMPOSER</p>
      <h2>Build a civic environment</h2>
      <p class="lead">Leave any field on Random to compose it from the abstraction library. The seed, selected values, difficulty and modifiers are stored with the generated pack.</p>
      <form id="generatorForm" class="generator-form">
        ${generatorFields.map(([id, label]) => `
          <label><span>${escapeHtml(label)}</span><select name="${id}">
            <option value="">Random</option>
            ${(generatorCatalog.categories[id] || []).map((option) => `<option value="${escapeHtml(option.id)}">${escapeHtml(option.label)}</option>`).join('')}
          </select></label>`).join('')}
        <label><span>Difficulty</span><select name="difficulty">
          ${generatorCatalog.difficulties.map((option) => `<option value="${escapeHtml(option.id)}" ${option.id === 'standard' ? 'selected' : ''}>${escapeHtml(option.label)} — ${escapeHtml(option.description)}</option>`).join('')}
        </select></label>
        <label><span>Seed (optional)</span><input name="seed" type="number" min="0" step="1" placeholder="Random server seed"></label>
        <fieldset class="modifier-field"><legend>Modifiers (none means random)</legend>
          ${generatorCatalog.modifiers.map((option) => `<label class="check-label"><input type="checkbox" name="modifier" value="${escapeHtml(option.id)}"><span>${escapeHtml(option.label)}</span></label>`).join('')}
        </fieldset>
        <div class="generator-actions">
          <button class="button button-primary" type="submit">Generate selected world</button>
          <button id="randomWorld" class="button button-secondary" type="button">Surprise me</button>
          <button id="cancelGenerator" class="button button-secondary" type="button">Back</button>
        </div>
      </form>
    </section>`;
  document.getElementById('generatorForm').addEventListener('submit', (event) => generateWorld(event, false));
  document.getElementById('randomWorld').addEventListener('click', (event) => generateWorld(event, true));
  document.getElementById('cancelGenerator').addEventListener('click', renderScenarioSelection);
}

async function generateWorld(event, fullyRandom) {
  event.preventDefault();
  if (requestInFlight) return;
  const form = document.getElementById('generatorForm');
  const data = new FormData(form);
  const selections = {};
  if (!fullyRandom) {
    for (const [id] of generatorFields) {
      const value = data.get(id);
      if (value) selections[id] = value;
    }
  }
  const seedText = fullyRandom ? '' : data.get('seed');
  const payload = {
    selections,
    difficulty: fullyRandom ? 'standard' : data.get('difficulty'),
    modifiers: fullyRandom ? [] : data.getAll('modifier'),
    randomize_unspecified: true,
  };
  if (seedText) payload.seed = Number(seedText);
  requestInFlight = true;
  form.querySelectorAll('button,select,input').forEach((control) => { control.disabled = true; });
  try {
    scenario = await api('/api/v1/scenarios/generate', { method: 'POST', body: JSON.stringify(payload) });
    applyScenarioTheme();
    renderProfileSelection();
  } catch (error) {
    alert(error.message);
    renderGenerator();
  } finally {
    requestInFlight = false;
  }
}

async function selectScenario(scenarioId) {
  if (requestInFlight) return;
  requestInFlight = true;
  try {
    scenario = await api(`/api/v1/scenarios/${encodeURIComponent(scenarioId)}`);
    applyScenarioTheme();
    renderProfileSelection();
  } catch (error) {
    alert(error.message);
  } finally {
    requestInFlight = false;
  }
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
  destroyCityRenderer();
  newGameButton.hidden = true;
  app.innerHTML = `
    <section class="hero-card">
      <p class="eyebrow">${escapeHtml(scenario.title)}</p>
      <h2>${escapeHtml(scenario.mission.title)}</h2>
      <p class="lead">${escapeHtml(scenario.description)}</p>
      <div class="environment-strip">
        <span>${escapeHtml(scenario.environment.world_region)}</span>
        <span>${escapeHtml(scenario.environment.political_system)}</span>
        <span>${escapeHtml(scenario.environment.administrative_capacity)}</span>
      </div>
      ${activeCampaignId ? `<p class="campaign-link">Campaign: ${escapeHtml(campaigns.find((item) => item.id === activeCampaignId)?.name || 'linked civic history')}</p>` : ''}
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
      <button id="backToScenarios" class="button button-secondary back-button" type="button">Choose another environment</button>
    </section>`;

  document.querySelectorAll('.select-profile').forEach((button) => {
    button.addEventListener('click', () => startSession(button.dataset.citizenId));
  });
  document.getElementById('backToScenarios').addEventListener('click', renderScenarioSelection);
}

function costHtml(cost) {
  const entries = [];
  if (cost.money) entries.push(`<span><strong>${formatMoney(cost.money)}</strong></span>`);
  if (cost.energy) entries.push(`<span>${cost.energy} energy</span>`);
  if (cost.influence) entries.push(`<span>${cost.influence} influence</span>`);
  if (cost.days) entries.push(`<span>${cost.days} days</span>`);
  for (const [id, value] of Object.entries(cost.values || {})) {
    const definition = scenario?.value_definitions?.find((item) => item.id === id);
    entries.push(`<span>${value} ${escapeHtml(definition?.label || id)}</span>`);
  }
  return entries.join('');
}

function formatIndicator(indicator) {
  if (indicator.format === 'money') return formatMoney(indicator.value);
  if (indicator.format === 'days') return `${indicator.value}`;
  if (indicator.format === 'percent') return `${indicator.value}%`;
  return String(indicator.value);
}

function factorDisplay(indicator) {
  if (indicator.id === 'evidence_stage') {
    return ['Unverified', 'Independently verified', 'Corroborated', 'Chain of custody confirmed', 'Legally admissible'][indicator.value] || `Stage ${indicator.value}`;
  }
  return indicator.format === 'percent' ? `${indicator.value}%` : `${indicator.value}/${indicator.max}`;
}

function metricBar(indicator) {
  const { label, value, min = 0, max = 100 } = indicator;
  const range = Math.max(1, max - min);
  const percent = Math.max(0, Math.min(100, ((value - min) / range) * 100));
  return `
    <div class="stat-row">
      <div class="stat-header"><span>${escapeHtml(label)}</span><strong>${escapeHtml(factorDisplay(indicator))}</strong></div>
      <div class="mini-track"><div class="mini-fill" style="width:${percent}%"></div></div>
    </div>`;
}

function factorChangeLines(changes, currentState = state, includeCurrentValue = false) {
  const indicators = new Map(
    (currentState?.indicators || [])
      .filter((indicator) => indicator.group !== 'resource' && indicator.id !== 'progress')
      .map((indicator) => [indicator.id, indicator]),
  );
  return Object.entries(changes || {})
    .filter(([id, value]) => value !== 0 && indicators.has(id))
    .map(([id, value]) => {
      const indicator = indicators.get(id);
      if (id === 'evidence_stage') return `${indicator.label} → ${factorDisplay(indicator)}`;
      const signed = `${value > 0 ? '+' : ''}${value}${indicator.format === 'percent' ? '%' : ''}`;
      const current = includeCurrentValue ? ` → ${factorDisplay(indicator)}` : '';
      return `${indicator.label} ${signed}${current}`;
    });
}

function eventChangeSummary(event) {
  const changes = factorChangeLines(event.value_changes).slice(0, 3);
  return changes.length ? changes.join(' · ') : 'No visible civic factor changed';
}

function activeCityPlan() {
  const asset = scenario?.visual_theme?.map_asset;
  return asset ? globalThis.CivicCityPlans?.[asset] || null : null;
}

function cityPlanSvg(plan) {
  if (!plan) return '<div class="city-river" aria-hidden="true"></div>';
  const layers = ['water', 'rail', 'road'].map((kind) => `
    <g class="map-layer map-${kind}">
      ${plan.features.filter((feature) => feature.kind === kind).map((feature) =>
        `<polyline class="map-${escapeHtml(feature.class)}" points="${feature.points.map((point) => `${point[0]},${point[1]}`).join(' ')}"></polyline>`,
      ).join('')}
    </g>`).join('');
  return `<svg class="city-plan-svg" viewBox="0 0 1000 600" preserveAspectRatio="xMidYMid meet" aria-hidden="true">${layers}</svg>`;
}

function mapAttributionHtml(plan, mapAsset) {
  if (!plan && !mapAsset?.startsWith('osm:')) return '';
  const label = plan?.label || mapAsset.slice(4).replaceAll('-', ' ');
  const sourceUrl = plan?.source_url || 'https://www.openstreetmap.org/';
  const licenseUrl = globalThis.CivicCityMapLibrary?.license_url || 'https://www.openstreetmap.org/copyright';
  return `
    <div class="map-attribution">
      <span>Street geometry: <strong>${escapeHtml(label)}</strong>. Civic scenario and institution locations are fictional.</span>
      <span><a href="${escapeHtml(sourceUrl)}" target="_blank" rel="noopener">Map data © OpenStreetMap contributors</a> · <a href="${escapeHtml(licenseUrl)}" target="_blank" rel="noopener">ODbL</a></span>
    </div>`;
}

function cityMapHtml() {
  const locations = scenario?.visual_theme?.locations || [];
  const validActions = state.available_actions.filter((action) => action.enabled);
  if (!locations.length) return '';
  const rendererAvailable = typeof globalThis.CivicCityRenderer === 'function';
  const mapAsset = scenario?.visual_theme?.map_asset || null;
  const mapPlan = activeCityPlan();
  return `
    <section class="panel city-panel">
      <div class="panel-inner">
        <div class="section-title"><h3>Civic environment</h3><span>${escapeHtml(mapPlan?.label || scenario.environment.world_region)}</span></div>
        <div class="city-map" role="group" aria-label="Scenario institutions and action locations">
          ${rendererAvailable ? '<div id="cityCanvasHost" class="city-canvas-host" aria-hidden="true"></div>' : ''}
          <div id="cityDomFallback" class="city-dom-fallback" ${rendererAvailable ? 'hidden' : ''}>
            ${rendererAvailable ? '' : cityPlanSvg(mapPlan)}
            ${locations.map((location) => {
            const selectable = validActions.find((action) => action.location_id === location.id);
            if (!selectable) return `<span class="city-location context" style="left:${location.x}%;top:${location.y}%"><span>${escapeHtml(location.label)}</span></span>`;
            return `<button class="city-location available" style="left:${location.x}%;top:${location.y}%" type="button" ${requestInFlight ? 'disabled' : ''} data-action-id="${escapeHtml(selectable.id)}" aria-label="${escapeHtml(location.label)}: ${escapeHtml(selectable.title)}"><span>${escapeHtml(location.label)}</span></button>`;
            }).join('')}
          </div>
        </div>
        ${mapAttributionHtml(mapPlan, mapAsset)}
      </div>
    </section>`;
}

function renderGame() {
  destroyCityRenderer();
  newGameButton.hidden = false;
  const statusClass = state.status === 'won' ? 'won' : 'lost';
  const finished = state.status !== 'active';
  const newestEvents = [...state.events].reverse();
  const resourceIndicators = state.indicators.filter((indicator) => indicator.group === 'resource');
  const metricIndicators = state.indicators.filter((indicator) => indicator.group !== 'resource' && indicator.id !== 'progress');
  const validActions = state.available_actions.filter((action) => action.enabled);

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

        ${cityMapHtml()}

        <section class="panel">
          <div class="panel-inner">
            <div class="resource-grid">
              ${resourceIndicators.map((indicator) => `<div class="metric-card"><div class="label">${escapeHtml(indicator.label)}</div><div class="value">${escapeHtml(formatIndicator(indicator))}</div></div>`).join('')}
            </div>
            <div class="progress-section">
              <div class="progress-heading"><span>Current situation</span><span>${metricIndicators.length} tracked factors</span></div>
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
                ${validActions.map((action) => `
                  <button class="action-button" type="button" data-action-id="${escapeHtml(action.id)}" ${requestInFlight ? 'disabled' : ''}>
                    <span>
                      <span class="action-title">${escapeHtml(action.title)}</span>
                      <span class="action-description">${escapeHtml(action.description)}</span>
                    </span>
                    <span class="action-cost">${costHtml(action.cost)}</span>
                  </button>`).join('') || '<p class="action-description">No valid action is currently available.</p>'}
              </div>
            </div>
          </section>`}
      </div>

      <aside class="side-column">
        <section class="panel">
          <div class="panel-inner">
            <div class="section-title"><h3>Civic capacity</h3><span>Public knowledge</span></div>
            <div class="secondary-metrics">
              ${metricIndicators.map(metricBar).join('')}
            </div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-inner">
            <div class="section-title"><h3>Case history</h3><span>${state.events.length} events</span></div>
            <div class="timeline">
              ${newestEvents.length ? newestEvents.map((event) => `
                <article class="timeline-item">
                  <h4>Turn ${event.turn}: ${event.kind === 'random_event' ? 'World event — ' : ''}${escapeHtml(event.action_title)}</h4>
                  <p>${escapeHtml(event.message)}</p>
                  <div class="event-meta">${escapeHtml(eventChangeSummary(event))} · ${event.resources_after.days_remaining} days left</div>
                </article>`).join('') : '<p class="action-description">No official action has been taken yet.</p>'}
            </div>
          </div>
        </section>
      </aside>
    </div>`;

  document.querySelectorAll('.action-button[data-action-id]').forEach((button) => {
    button.addEventListener('click', () => takeAction(button.dataset.actionId));
  });
  document.querySelectorAll('.city-location[data-action-id]').forEach((button) => {
    button.addEventListener('click', () => takeAction(button.dataset.actionId));
  });
  document.getElementById('restartButton')?.addEventListener('click', resetSession);
  mountCityRenderer();
  pendingVisualEvent = null;
}

async function startSession(citizenId) {
  if (requestInFlight) return;
  requestInFlight = true;
  try {
    const created = await api('/api/v1/sessions', {
      method: 'POST',
      body: JSON.stringify({ scenario_id: scenario.id, profile_id: citizenId, citizen_id: citizenId, campaign_id: activeCampaignId }),
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
    pendingVisualEvent = result.visual_event || null;
    if (state.status !== 'active' && state.campaign_id) {
      campaigns = await api('/api/v1/campaigns');
    }
    dialogLabel.textContent = state.status === 'won' ? 'MISSION COMPLETE' : state.status === 'lost' ? 'MISSION ENDED' : 'STOCHASTIC OUTCOME';
    dialogTitle.textContent = state.status === 'won' ? 'Thresholds secured' : state.status === 'lost' ? 'Campaign ended' : 'Civic progress by factor';
    dialogMessage.textContent = result.message;
    const factorChanges = factorChangeLines(result.value_changes, state, true);
    dialogDelta.textContent = factorChanges.length
      ? factorChanges.join(' · ')
      : 'No visible civic factor changed in this outcome.';
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
    if (state.campaign_id) {
      activeCampaignId = state.campaign_id;
      storageSet(campaignStorageKey, activeCampaignId);
    }
    if (!scenario || scenario.id !== state.game_pack_id) {
      scenario = await api(`/api/v1/scenarios/${encodeURIComponent(state.game_pack_id)}`);
      applyScenarioTheme();
    }
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
    const [health, loadedScenarios, loadedGenerator, loadedCampaigns] = await Promise.all([
      api('/api/v1/health'),
      api('/api/v1/scenarios'),
      api('/api/v1/scenario-generator'),
      api('/api/v1/campaigns'),
    ]);
    scenarios = loadedScenarios;
    generatorCatalog = loadedGenerator;
    campaigns = loadedCampaigns;
    campaignsSupported = health.campaigns_supported !== false;
    const storedCampaign = storageGet(campaignStorageKey);
    activeCampaignId = campaigns.some((item) => item.id === storedCampaign) ? storedCampaign : null;
    setConnection(true, `${health.sessions} saved session${health.sessions === 1 ? '' : 's'}`);
    const restored = await restoreSession();
    if (!restored) renderScenarioSelection();
  } catch (error) {
    setConnection(false);
    renderError(error.message);
  }
}

newGameButton.addEventListener('click', () => {
  if (!confirm('Start a new game? The current session remains saved on the server.')) return;
  state = null;
  storageRemove(sessionStorageKey);
  renderScenarioSelection();
});

dialogClose.addEventListener('click', () => resultDialog.close());
window.addEventListener('online', () => setConnection(true));
window.addEventListener('offline', () => setConnection(false));

if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => navigator.serviceWorker.register('/sw.js').catch(() => {}));
}

boot();
