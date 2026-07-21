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
    const cost = action.cost;
    if (session.resources.money < cost.money || session.resources.energy < cost.energy || session.resources.influence < cost.influence || session.resources.days_remaining <= cost.days) {
      return `insufficient resources: requires ₹${cost.money}, ${cost.energy} energy, ${cost.influence} influence and ${cost.days} days`;
    }
    for (const requirement of action.requirements || []) {
      const value = metric(session, requirement.metric);
      if ((requirement.min != null && value < requirement.min) || (requirement.max != null && value > requirement.max)) return `action unavailable: ${requirement.message}`;
    }
    return null;
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
  }

  function clamp(session) {
    for (const key of ['progress', 'documentation', 'community_support', 'public_attention', 'integrity']) {
      session.metrics[key] = Math.max(0, Math.min(100, session.metrics[key]));
    }
    session.resources.energy = Math.max(0, Math.min(100, session.resources.energy));
    session.resources.influence = Math.max(0, Math.min(100, session.resources.influence));
    session.resources.days_remaining = Math.max(0, session.resources.days_remaining);
  }

  function resolve(session) {
    if (session.metrics.progress >= pack.mission.win_progress) {
      session.status = 'won';
      session.current_status = 'The drainage repair has been completed and verified by residents.';
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
    return {
      id: session.id,
      citizen_id: session.citizen_id,
      citizen_name: session.citizen_name,
      citizen_context: session.citizen_context,
      mission_title: session.mission_title,
      objective: session.objective,
      current_status: session.current_status,
      resources: structuredClone(session.resources),
      metrics: structuredClone(session.metrics),
      status: session.status,
      turn: session.turn,
      events: structuredClone(session.events),
      available_actions: pack.actions.map((action) => {
        const reason = session.status === 'active' ? checkAction(session, action) : 'session is already finished';
        return {
          id: action.id,
          title: action.title,
          description: action.description,
          cost: action.cost,
          enabled: !reason && session.status === 'active',
          disabled_reason: reason,
        };
      }),
    };
  }

  function create(citizenId, existingId = null) {
    const citizen = pack.citizens.find((item) => item.id === citizenId);
    if (!citizen) throw new Error('citizen profile not found');
    const seed = Math.floor(Math.random() * 0xFFFFFFFF);
    const rng = mulberry32(seed);
    return {
      id: existingId || randomId(),
      game_pack_id: pack.id,
      citizen_id: citizen.id,
      citizen_name: citizen.name,
      citizen_context: citizen.context,
      mission_title: pack.mission.title,
      objective: pack.mission.objective,
      current_status: pack.mission.starting_status,
      resources: structuredClone(citizen.starting_resources),
      metrics: structuredClone(citizen.starting_metrics),
      hidden: {
        departmental_backlog: integer(rng, 30, 90),
        officer_integrity: integer(rng, 25, 95),
        election_pressure: integer(rng, 10, 90),
        corruption_pressure: integer(rng, 10, 85),
      },
      status: 'active',
      turn: 0,
      seed,
      events: [],
      action_results: {},
    };
  }

  function act(session, request) {
    if (session.action_results[request.client_action_id]) return session.action_results[request.client_action_id];
    if (session.status !== 'active') throw new Error('session is already finished');
    const action = pack.actions.find((item) => item.id === request.action_id);
    if (!action) throw new Error('action not found');
    const unavailable = checkAction(session, action);
    if (unavailable) throw new Error(unavailable);

    session.resources.money -= action.cost.money;
    session.resources.energy -= action.cost.energy;
    session.resources.influence -= action.cost.influence;
    session.resources.days_remaining -= action.cost.days;
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
    session.events.push({
      turn: session.turn,
      action_id: action.id,
      action_title: action.title,
      outcome_id: selected.id,
      message: selected.message,
      progress_change: progressChange,
      resources_after: structuredClone(session.resources),
    });
    const response = {
      outcome_id: selected.id,
      message: selected.message,
      progress_change: progressChange,
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
        return jsonResponse({ status: 'ok', game_pack: pack.id, version: 'offline', sessions: Object.keys(sessions).length });
      }
      if (path === '/api/v1/scenario' && method === 'GET') {
        return jsonResponse({
          id: pack.id,
          title: pack.title,
          description: pack.description,
          version: pack.version,
          mission: pack.mission,
          citizens: pack.citizens,
        });
      }
      if (path === '/api/v1/sessions' && method === 'POST') {
        const body = JSON.parse(options.body || '{}');
        const session = create(body.citizen_id);
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
