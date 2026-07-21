(() => {
  const pack = JSON.parse(document.getElementById('embeddedGamePack').textContent);
  const storeKey = 'civic-sim-offline-sessions';
  let memoryStore = {};

  function loadStore() {
    try { return JSON.parse(localStorage.getItem(storeKey) || '{}'); }
    catch { return structuredClone(memoryStore); }
  }

  function saveStore(store) {
    memoryStore = structuredClone(store);
    try { localStorage.setItem(storeKey, JSON.stringify(store)); }
    catch { /* opaque origins use the in-memory fallback */ }
  }

  function randomId() {
    return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }

  function mulberry32(seed) {
    return function next() {
      let t = seed += 0x6D2B79F5;
      t = Math.imul(t ^ t >>> 15, t | 1);
      t ^= t + Math.imul(t ^ t >>> 7, t | 61);
      return ((t ^ t >>> 14) >>> 0) / 4294967296;
    };
  }

  function integer(rng, min, max) {
    return Math.floor(rng() * (max - min + 1)) + min;
  }

  function metric(session, name) {
    if (Object.hasOwn(session.values || {}, name)) return Number(session.values[name]);
    if (Object.hasOwn(session.hidden_values || {}, name)) return Number(session.hidden_values[name]);
    if (Object.hasOwn(session.persistent_consequences || {}, name)) return Number(session.persistent_consequences[name]);
    const lookup = {
      progress: session.metrics.progress,
      documentation: session.metrics.documentation,
      community_support: session.metrics.community_support,
      public_attention: session.metrics.public_attention,
      integrity: session.metrics.integrity,
      money: session.resources.money,
      energy: session.resources.energy,
      influence: session.resources.influence,
      days_remaining: session.resources.days_remaining,
      departmental_backlog: session.hidden.departmental_backlog,
      officer_integrity: session.hidden.officer_integrity,
      election_pressure: session.hidden.election_pressure,
      corruption_pressure: session.hidden.corruption_pressure,
    };
    return Number(lookup[name] ?? 0);
  }

  function checkAction(session, action) {
    const useLimit = action.max_uses ?? 1;
    const useCount = session.events.filter((event) => event.kind === 'action' && event.action_id === action.id).length;
    if (useCount >= useLimit) return 'action unavailable: this opportunity has already been used';
    const cost = action.cost;
    if (session.resources.money < cost.money || session.resources.energy < cost.energy || session.resources.influence < cost.influence || session.resources.days_remaining <= cost.days) {
      return `insufficient resources: requires ₹${cost.money}, ${cost.energy} energy, ${cost.influence} influence and ${cost.days} days`;
    }
    for (const [id, required] of Object.entries(cost.values || {})) {
      if (metric(session, id) < required) return `insufficient resources: requires ${required} ${id}`;
    }
    for (const requirement of action.requirements || []) {
      const value = metric(session, requirement.metric);
      if ((requirement.min != null && value < requirement.min) || (requirement.max != null && value > requirement.max)) return `action unavailable: ${requirement.message}`;
    }
    return null;
  }

  function publicValueSnapshot(session) {
    return {
      ...(session.values || {}),
      money: session.resources.money,
      energy: session.resources.energy,
      influence: session.resources.influence,
      days_remaining: session.resources.days_remaining,
      progress: session.metrics.progress,
      documentation: session.metrics.documentation,
      community_support: session.metrics.community_support,
      public_attention: session.metrics.public_attention,
      integrity: session.metrics.integrity,
    };
  }

  function valueChanges(before, after) {
    return Object.fromEntries(
      [...new Set([...Object.keys(before), ...Object.keys(after)])]
        .map((id) => [id, (after[id] || 0) - (before[id] || 0)])
        .filter(([, change]) => change !== 0),
    );
  }

  function applyDelta(session, delta = {}) {
    const resources = delta.resources || {};
    session.resources.money += resources.money || 0;
    session.resources.energy += resources.energy || 0;
    session.resources.influence += resources.influence || 0;
    session.resources.days_remaining += resources.days_remaining || 0;
    session.metrics.progress += delta.progress || 0;
    session.metrics.documentation += delta.documentation || 0;
    session.metrics.community_support += delta.community_support || 0;
    session.metrics.public_attention += delta.public_attention || 0;
    session.metrics.integrity += delta.integrity || 0;
    const legacy = {
      money: ['resources', 'money'], energy: ['resources', 'energy'], influence: ['resources', 'influence'], days_remaining: ['resources', 'days_remaining'],
      progress: ['metrics', 'progress'], documentation: ['metrics', 'documentation'], community_support: ['metrics', 'community_support'], public_attention: ['metrics', 'public_attention'], integrity: ['metrics', 'integrity'],
    };
    for (const [id, change] of Object.entries(delta.values || {})) {
      session.values[id] = (session.values[id] || 0) + change;
      const target = legacy[id];
      if (target) session[target[0]][target[1]] = session.values[id];
    }
    for (const [id, change] of Object.entries(delta.consequences || {})) {
      session.persistent_consequences[id] = (session.persistent_consequences[id] || 0) + change;
    }
    syncLegacyValues(session);
  }

  function syncLegacyValues(session) {
    Object.assign(session.values, {
      money: session.resources.money,
      energy: session.resources.energy,
      influence: session.resources.influence,
      days_remaining: session.resources.days_remaining,
      progress: session.metrics.progress,
      documentation: session.metrics.documentation,
      community_support: session.metrics.community_support,
      public_attention: session.metrics.public_attention,
      integrity: session.metrics.integrity,
    });
  }

  function clamp(session) {
    for (const key of ['progress', 'documentation', 'community_support', 'public_attention', 'integrity']) {
      session.metrics[key] = Math.max(0, Math.min(100, session.metrics[key]));
    }
    session.resources.energy = Math.max(0, Math.min(100, session.resources.energy));
    session.resources.influence = Math.max(0, Math.min(100, session.resources.influence));
    session.resources.days_remaining = Math.max(0, session.resources.days_remaining);
    for (const definition of pack.value_definitions || []) {
      if (Object.hasOwn(session.values, definition.id)) {
        session.values[definition.id] = Math.max(definition.min ?? 0, Math.min(definition.max ?? 100, session.values[definition.id]));
      }
    }
    syncLegacyValues(session);
  }

  function resolve(session) {
    const ending = (pack.endings || []).find((candidate) => (candidate.conditions || []).every((condition) => {
      const value = metric(session, condition.metric);
      return (condition.min == null || value >= condition.min) && (condition.max == null || value <= condition.max);
    }));
    if (ending) {
      session.status = ending.status;
      session.ending_id = ending.id;
      session.current_status = ending.message;
    } else if (!(pack.endings || []).length && session.metrics.progress >= pack.mission.win_progress) {
      session.status = 'won';
      session.current_status = 'The mission objectives have been completed.';
    } else if (session.resources.days_remaining <= 0) {
      session.status = 'lost';
      session.current_status = 'The deadline expired before the work could be completed.';
    } else if (session.resources.energy <= 0) {
      session.status = 'lost';
      session.current_status = 'The citizen exhausted their capacity to continue pursuing the case.';
    } else if (session.resources.money < 0) {
      session.status = 'lost';
      session.current_status = 'The citizen can no longer finance the campaign.';
    }
  }

  function publicState(session) {
    const values = {
      ...(session.values || {}),
      money: session.resources.money,
      energy: session.resources.energy,
      influence: session.resources.influence,
      days_remaining: session.resources.days_remaining,
      progress: session.metrics.progress,
      documentation: session.metrics.documentation,
      community_support: session.metrics.community_support,
      public_attention: session.metrics.public_attention,
      integrity: session.metrics.integrity,
    };
    const indicators = (pack.value_definitions || []).filter((definition) => !definition.hidden_from_hud).map((definition) => ({
      id: definition.id,
      label: definition.label,
      description: definition.description || '',
      group: definition.group || 'metric',
      value: Number(values[definition.id] || 0),
      min: definition.min ?? 0,
      max: definition.max ?? 100,
      format: definition.format || 'number',
    }));
    return {
      id: session.id,
      game_pack_id: session.game_pack_id,
      game_pack_version: session.game_pack_version || pack.version,
      citizen_id: session.citizen_id,
      citizen_name: session.citizen_name,
      citizen_context: session.citizen_context,
      mission_title: session.mission_title,
      objective: session.objective,
      current_status: session.current_status,
      resources: structuredClone(session.resources),
      metrics: structuredClone(session.metrics),
      values,
      indicators,
      persistent_consequences: structuredClone(session.persistent_consequences || {}),
      status: session.status,
      ending_id: session.ending_id || null,
      turn: session.turn,
      events: structuredClone(session.events),
      available_actions: session.status !== 'active' ? [] : pack.actions
        .filter((action) => !checkAction(session, action))
        .map((action) => ({
          id: action.id,
          title: action.title,
          description: action.description,
          action_type: action.action_type || '',
          location_id: action.location_id || null,
          cost: action.cost,
          enabled: true,
          disabled_reason: null,
        })),
    };
  }

  function create(citizenId, existingId = null) {
    const citizen = pack.citizens.find((item) => item.id === citizenId);
    if (!citizen) throw new Error('citizen profile not found');
    const seed = Math.floor(Math.random() * 0xFFFFFFFF);
    const rng = mulberry32(seed);
    const values = {
      ...(citizen.starting_values || {}),
      money: citizen.starting_resources.money,
      energy: citizen.starting_resources.energy,
      influence: citizen.starting_resources.influence,
      days_remaining: citizen.starting_resources.days_remaining,
      progress: citizen.starting_metrics.progress,
      documentation: citizen.starting_metrics.documentation,
      community_support: citizen.starting_metrics.community_support,
      public_attention: citizen.starting_metrics.public_attention,
      integrity: citizen.starting_metrics.integrity,
    };
    return {
      id: existingId || randomId(),
      game_pack_id: pack.id,
      game_pack_version: pack.version,
      citizen_id: citizen.id,
      citizen_name: citizen.name,
      citizen_context: citizen.context,
      mission_title: pack.mission.title,
      objective: pack.mission.objective,
      current_status: pack.mission.starting_status,
      resources: structuredClone(citizen.starting_resources),
      metrics: structuredClone(citizen.starting_metrics),
      values,
      hidden: {
        departmental_backlog: integer(rng, 30, 90),
        officer_integrity: integer(rng, 25, 95),
        election_pressure: integer(rng, 10, 90),
        corruption_pressure: integer(rng, 10, 85),
      },
      hidden_values: {},
      persistent_consequences: {},
      triggered_random_events: [],
      status: 'active',
      turn: 0,
      seed,
      events: [],
      action_results: {},
      ending_id: null,
    };
  }

  function act(session, request) {
    if (session.action_results[request.client_action_id]) return session.action_results[request.client_action_id];
    if (session.status !== 'active') throw new Error('session is already finished');
    const action = pack.actions.find((item) => item.id === request.action_id);
    if (!action) throw new Error('action not found');
    const unavailable = checkAction(session, action);
    if (unavailable) throw new Error(unavailable);
    const valuesBefore = publicValueSnapshot(session);

    session.resources.money -= action.cost.money;
    session.resources.energy -= action.cost.energy;
    session.resources.influence -= action.cost.influence;
    session.resources.days_remaining -= action.cost.days;
    for (const [id, value] of Object.entries(action.cost.values || {})) {
      session.values[id] = (session.values[id] || 0) - value;
    }
    applyDelta(session, action.guaranteed_effect);

    const rng = mulberry32((session.seed ^ Math.imul(session.turn + 1, 0x9E3779B9)) >>> 0);
    const weighted = action.outcomes.map((outcome) => {
      let weight = outcome.base_weight;
      for (const condition of outcome.conditions || []) {
        const value = metric(session, condition.metric);
        const matches = (condition.min == null || value >= condition.min) && (condition.max == null || value <= condition.max);
        if (matches) weight *= condition.multiplier;
      }
      return { outcome, weight: Math.max(0.001, weight) };
    });
    const total = weighted.reduce((sum, item) => sum + item.weight, 0);
    let roll = rng() * total;
    let selected = weighted[weighted.length - 1].outcome;
    for (const item of weighted) {
      if (roll < item.weight) { selected = item.outcome; break; }
      roll -= item.weight;
    }

    const before = session.metrics.progress;
    applyDelta(session, selected.effect);
    session.metrics.progress += selected.progress_max > selected.progress_min
      ? integer(rng, selected.progress_min, selected.progress_max)
      : selected.progress_min;
    clamp(session);
    session.turn += 1;
    session.current_status = selected.message;
    resolve(session);
    const progressChange = session.metrics.progress - before;
    const changes = valueChanges(valuesBefore, publicValueSnapshot(session));
    session.events.push({
      turn: session.turn,
      action_id: action.id,
      action_title: action.title,
      outcome_id: selected.id,
      message: selected.message,
      progress_change: progressChange,
      resources_after: structuredClone(session.resources),
      kind: 'action',
      value_changes: changes,
      visual_event: selected.visual_event || null,
    });
    const response = {
      outcome_id: selected.id,
      message: selected.message,
      progress_change: progressChange,
      value_changes: changes,
      visual_event: selected.visual_event || null,
      state: publicState(session),
    };
    session.action_results[request.client_action_id] = response;
    return response;
  }

  function jsonResponse(body, status = 200) {
    return Promise.resolve(new Response(JSON.stringify(body), {
      status,
      headers: { 'Content-Type': 'application/json' },
    }));
  }

  globalThis.fetch = async (input, options = {}) => {
    try {
      const url = new URL(String(input), 'http://offline.local');
      const path = url.pathname;
      const method = (options.method || 'GET').toUpperCase();
      const sessions = loadStore();

      if (path === '/api/v1/health' && method === 'GET') {
        return jsonResponse({ status: 'ok', game_pack: pack.id, version: 'offline', sessions: Object.keys(sessions).length, campaigns_supported: false });
      }
      if (path === '/api/v1/scenario' && method === 'GET') {
        return jsonResponse({
          schema_version: pack.schema_version || 1,
          id: pack.id,
          title: pack.title,
          description: pack.description,
          version: pack.version,
          environment: pack.environment || {},
          mission: pack.mission,
          value_definitions: pack.value_definitions || [],
          institutions: pack.institutions || [],
          stakeholders: pack.stakeholders || [],
          barriers: pack.barriers || [],
          visual_theme: pack.visual_theme || {},
          citizens: pack.citizens,
        });
      }
      if (path === '/api/v1/scenarios' && method === 'GET') {
        return jsonResponse([{
          id: pack.id,
          title: pack.title,
          description: pack.description,
          version: pack.version,
          objective_type: pack.mission.objective_type || '',
          world_region: pack.environment?.world_region || '',
          role_count: pack.citizens.length,
          visual_theme: pack.visual_theme || {},
        }]);
      }
      if (path === '/api/v1/scenario-generator' && method === 'GET') {
        return jsonResponse({ schema_version: 1, categories: {}, difficulties: [], modifiers: [], templates: [] });
      }
      if (path === '/api/v1/campaigns' && method === 'GET') {
        return jsonResponse([]);
      }
      if (path === `/api/v1/scenarios/${pack.id}` && method === 'GET') {
        return jsonResponse({
          schema_version: pack.schema_version || 1,
          id: pack.id,
          title: pack.title,
          description: pack.description,
          version: pack.version,
          environment: pack.environment || {},
          mission: pack.mission,
          value_definitions: pack.value_definitions || [],
          institutions: pack.institutions || [],
          stakeholders: pack.stakeholders || [],
          barriers: pack.barriers || [],
          visual_theme: pack.visual_theme || {},
          citizens: pack.citizens,
        });
      }
      if (path === '/api/v1/sessions' && method === 'POST') {
        const body = JSON.parse(options.body || '{}');
        if (body.scenario_id && body.scenario_id !== pack.id) return jsonResponse({ error: 'scenario not found in standalone mode' }, 404);
        const session = create(body.profile_id || body.citizen_id);
        sessions[session.id] = session;
        saveStore(sessions);
        return jsonResponse(publicState(session), 201);
      }
      const match = path.match(/^\/api\/v1\/sessions\/([^/]+)(?:\/(actions|reset))?$/);
      if (match) {
        const id = decodeURIComponent(match[1]);
        const operation = match[2];
        let session = sessions[id];
        if (!session) return jsonResponse({ error: 'session not found' }, 404);
        if (!operation && method === 'GET') return jsonResponse(publicState(session));
        if (operation === 'actions' && method === 'POST') {
          const result = act(session, JSON.parse(options.body || '{}'));
          sessions[id] = session;
          saveStore(sessions);
          return jsonResponse(result);
        }
        if (operation === 'reset' && method === 'POST') {
          session = create(session.citizen_id, id);
          sessions[id] = session;
          saveStore(sessions);
          return jsonResponse(publicState(session));
        }
      }
      return jsonResponse({ error: 'offline route not found' }, 404);
    } catch (error) {
      return jsonResponse({ error: error instanceof Error ? error.message : String(error) }, 400);
    }
  };
})();
