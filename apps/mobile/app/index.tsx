import React, { useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  Pressable,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';

const API_URL = process.env.EXPO_PUBLIC_API_URL ?? 'http://localhost:8080';

type Resources = {
  money: number;
  energy: number;
  influence: number;
  days_remaining: number;
};

type Metrics = {
  progress: number;
  documentation: number;
  community_support: number;
  public_attention: number;
  integrity: number;
};

type Citizen = {
  id: string;
  name: string;
  occupation: string;
  context: string;
  starting_resources: Resources;
};

type Scenario = {
  id: string;
  title: string;
  description: string;
  environment: { world_region: string; political_system: string; administrative_capacity: string };
  mission: { title: string; objective: string };
  citizens: Citizen[];
};

type ScenarioSummary = {
  id: string;
  title: string;
  description: string;
  objective_type: string;
  world_region: string;
  role_count: number;
  visual_theme?: { primary_color?: string; accent_color?: string };
};

type GeneratorCatalog = {
  difficulties: { id: string; label: string; description: string }[];
};

type Action = {
  id: string;
  title: string;
  description: string;
  cost: { money: number; energy: number; influence: number; days: number };
  enabled: boolean;
  disabled_reason?: string | null;
};

type GameEvent = {
  turn: number;
  action_title: string;
  message: string;
  progress_change: number;
  value_changes?: Record<string, number>;
};

type GameState = {
  id: string;
  game_pack_id: string;
  citizen_name: string;
  citizen_context: string;
  mission_title: string;
  objective: string;
  current_status: string;
  resources: Resources;
  metrics: Metrics;
  indicators?: Indicator[];
  status: 'active' | 'won' | 'lost';
  turn: number;
  events: GameEvent[];
  available_actions: Action[];
};

type Indicator = {
  id: string;
  label: string;
  group: 'resource' | 'metric' | 'relationship' | 'skill';
  value: number;
  min: number;
  max: number;
  format: 'number' | 'percent' | 'money' | 'days';
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(body.error ?? 'Request failed');
  return body as T;
}

function money(value: number) {
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency: 'INR',
    maximumFractionDigits: 0,
  }).format(value);
}

function factorChangeText(changes: Record<string, number> | undefined, state: GameState) {
  const indicators = new Map(
    (state.indicators ?? [])
      .filter((indicator) => indicator.group !== 'resource' && indicator.id !== 'progress')
      .map((indicator) => [indicator.id, indicator]),
  );
  const evidenceStages = ['Unverified', 'Independently verified', 'Corroborated', 'Chain of custody confirmed', 'Legally admissible'];
  const lines = Object.entries(changes ?? {})
    .filter(([id, value]) => value !== 0 && indicators.has(id))
    .map(([id, value]) => {
      const indicator = indicators.get(id)!;
      if (id === 'evidence_stage') return `${indicator.label} → ${evidenceStages[indicator.value] ?? `Stage ${indicator.value}`}`;
      const signed = `${value > 0 ? '+' : ''}${value}${indicator.format === 'percent' ? '%' : ''}`;
      const current = indicator.format === 'percent' ? `${indicator.value}%` : `${indicator.value}/${indicator.max}`;
      return `${indicator.label} ${signed} → ${current}`;
    });
  return lines.length ? lines.join('\n') : 'No visible civic factor changed in this outcome.';
}

function Meter({ id, label, value, min = 0, max = 100, format = 'percent' }: { id: string; label: string; value: number; min?: number; max?: number; format?: Indicator['format'] }) {
  const range = Math.max(1, max - min);
  const percent = Math.max(0, Math.min(100, ((value - min) / range) * 100));
  const evidenceStages = ['Unverified', 'Independently verified', 'Corroborated', 'Chain of custody confirmed', 'Legally admissible'];
  const display = id === 'evidence_stage'
    ? evidenceStages[value] ?? `Stage ${value}`
    : format === 'percent' ? `${value}%` : `${value}/${max}`;
  return (
    <View style={styles.meterRow}>
      <View style={styles.rowBetween}>
        <Text style={styles.meterLabel}>{label}</Text>
        <Text style={styles.meterValue}>{display}</Text>
      </View>
      <View style={styles.track}>
        <View style={[styles.fill, { width: `${percent}%` }]} />
      </View>
    </View>
  );
}

export default function Index() {
  const [catalog, setCatalog] = useState<ScenarioSummary[] | null>(null);
  const [generatorCatalog, setGeneratorCatalog] = useState<GeneratorCatalog | null>(null);
  const [difficulty, setDifficulty] = useState('standard');
  const [scenario, setScenario] = useState<Scenario | null>(null);
  const [game, setGame] = useState<GameState | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      request<ScenarioSummary[]>('/api/v1/scenarios'),
      request<GeneratorCatalog>('/api/v1/scenario-generator'),
    ])
      .then(([scenarios, generator]) => {
        setCatalog(scenarios);
        setGeneratorCatalog(generator);
      })
      .catch((reason: Error) => setError(reason.message));
  }, []);

  const recentEvents = useMemo(() => [...(game?.events ?? [])].reverse(), [game?.events]);

  async function chooseScenario(scenarioId: string) {
    setBusy(true);
    try {
      setScenario(await request<Scenario>(`/api/v1/scenarios/${scenarioId}`));
    } catch (reason) {
      Alert.alert('Could not load environment', reason instanceof Error ? reason.message : 'Unknown error');
    } finally {
      setBusy(false);
    }
  }

  async function generateRandomWorld() {
    setBusy(true);
    try {
      setScenario(await request<Scenario>('/api/v1/scenarios/generate', {
        method: 'POST',
        body: JSON.stringify({ selections: {}, difficulty, modifiers: [], randomize_unspecified: true }),
      }));
    } catch (reason) {
      Alert.alert('Could not generate world', reason instanceof Error ? reason.message : 'Unknown error');
    } finally {
      setBusy(false);
    }
  }

  async function start(citizenId: string) {
    setBusy(true);
    try {
      setGame(await request<GameState>('/api/v1/sessions', {
        method: 'POST',
        body: JSON.stringify({ scenario_id: scenario?.id, profile_id: citizenId, citizen_id: citizenId }),
      }));
    } catch (reason) {
      Alert.alert('Could not start', reason instanceof Error ? reason.message : 'Unknown error');
    } finally {
      setBusy(false);
    }
  }

  async function act(actionId: string) {
    if (!game || busy) return;
    setBusy(true);
    try {
      const result = await request<{ message: string; progress_change: number; value_changes?: Record<string, number>; state: GameState }>(
        `/api/v1/sessions/${game.id}/actions`,
        {
          method: 'POST',
          body: JSON.stringify({
            action_id: actionId,
            client_action_id: `${Date.now()}-${Math.random()}`,
          }),
        },
      );
      setGame(result.state);
      Alert.alert(
        result.state.status === 'won' ? 'Thresholds secured' : result.state.status === 'lost' ? 'Campaign ended' : 'Civic progress by factor',
        `${result.message}\n\n${factorChangeText(result.value_changes, result.state)}`,
      );
    } catch (reason) {
      Alert.alert('Action failed', reason instanceof Error ? reason.message : 'Unknown error');
    } finally {
      setBusy(false);
    }
  }

  if (error) {
    return (
      <SafeAreaView style={styles.safe}>
        <View style={styles.center}>
          <Text style={styles.eyebrow}>CONNECTION ERROR</Text>
          <Text style={styles.title}>Server unavailable</Text>
          <Text style={styles.body}>{error}</Text>
          <Text style={styles.body}>Configured API: {API_URL}</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (!catalog) {
    return (
      <SafeAreaView style={styles.safe}>
        <View style={styles.center}><ActivityIndicator size="large" /></View>
      </SafeAreaView>
    );
  }

  if (!scenario) {
    return (
      <SafeAreaView style={styles.safe}>
        <ScrollView contentContainerStyle={styles.content}>
          <Text style={styles.eyebrow}>WORLD-SCALE CIVIC SIMULATION</Text>
          <Text style={styles.hero}>Choose an environment</Text>
          <Text style={styles.body}>Political systems, institutions and civic cultures change how the same tools work.</Text>
          {generatorCatalog ? (
            <View style={styles.card}>
              <Text style={styles.occupation}>SCENARIO COMPOSER</Text>
              <Text style={styles.cardTitle}>Generate a random world</Text>
              <Text style={styles.body}>The server composes a seeded region, political system, administration, role, objective, modifiers and visual palette.</Text>
              <View style={styles.difficultyRow}>
                {generatorCatalog.difficulties.map((item) => (
                  <Pressable key={item.id} onPress={() => setDifficulty(item.id)} style={[styles.difficultyChip, difficulty === item.id && styles.difficultyChipActive]}>
                    <Text style={[styles.difficultyText, difficulty === item.id && styles.difficultyTextActive]}>{item.label}</Text>
                  </Pressable>
                ))}
              </View>
              <Pressable disabled={busy} onPress={generateRandomWorld} style={({ pressed }) => [styles.primaryButton, pressed && styles.pressed]}>
                <Text style={styles.primaryButtonText}>Generate world</Text>
              </Pressable>
            </View>
          ) : null}
          <View style={styles.stack}>
            {catalog.map((item) => (
              <View key={item.id} style={styles.card}>
                <Text style={styles.occupation}>{item.world_region}</Text>
                <Text style={styles.cardTitle}>{item.title}</Text>
                <Text style={styles.body}>{item.description}</Text>
                <Text style={styles.costLine}>{item.objective_type} · {item.role_count} player roles</Text>
                <Pressable
                  accessibilityRole="button"
                  disabled={busy}
                  onPress={() => chooseScenario(item.id)}
                  style={({ pressed }) => [styles.primaryButton, pressed && styles.pressed]}
                >
                  <Text style={styles.primaryButtonText}>Explore environment</Text>
                </Pressable>
              </View>
            ))}
          </View>
        </ScrollView>
      </SafeAreaView>
    );
  }

  if (!game) {
    return (
      <SafeAreaView style={styles.safe}>
        <ScrollView contentContainerStyle={styles.content}>
          <Text style={styles.eyebrow}>CIVIC SIMULATION</Text>
          <Text style={styles.hero}>{scenario.mission.title}</Text>
          <Text style={styles.body}>{scenario.description}</Text>
          <Text style={styles.costLine}>{scenario.environment.world_region} · {scenario.environment.political_system}</Text>
          <View style={styles.stack}>
            {scenario.citizens.map((citizen) => (
              <View key={citizen.id} style={styles.card}>
                <Text style={styles.occupation}>{citizen.occupation}</Text>
                <Text style={styles.cardTitle}>{citizen.name}</Text>
                <Text style={styles.body}>{citizen.context}</Text>
                <Text style={styles.costLine}>
                  {money(citizen.starting_resources.money)} · {citizen.starting_resources.energy} energy · {citizen.starting_resources.days_remaining} days
                </Text>
                <Pressable
                  accessibilityRole="button"
                  disabled={busy}
                  onPress={() => start(citizen.id)}
                  style={({ pressed }) => [styles.primaryButton, pressed && styles.pressed]}
                >
                  <Text style={styles.primaryButtonText}>Play as {citizen.name.split(' ')[0]}</Text>
                </Pressable>
              </View>
            ))}
          </View>
          <Pressable style={styles.secondaryButton} onPress={() => setScenario(null)}>
            <Text style={styles.secondaryButtonText}>Choose another environment</Text>
          </Pressable>
        </ScrollView>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.safe}>
      <ScrollView contentContainerStyle={styles.content}>
        <View style={styles.missionCard}>
          <Text style={styles.eyebrowLight}>CURRENT MISSION</Text>
          <Text style={styles.missionTitle}>{game.mission_title}</Text>
          <Text style={styles.bodyLight}>{game.objective}</Text>
          <View style={styles.rowBetween}>
            <Text style={styles.citizen}>{game.citizen_name}</Text>
            <Text style={styles.turn}>Turn {game.turn}</Text>
          </View>
        </View>

        <View style={styles.resourceGrid}>
          {(game.indicators?.filter((indicator) => indicator.group === 'resource') ?? [
            { id: 'money', label: 'Money', group: 'resource' as const, value: game.resources.money, min: 0, max: 100, format: 'money' as const },
            { id: 'energy', label: 'Energy', group: 'resource' as const, value: game.resources.energy, min: 0, max: 100, format: 'number' as const },
            { id: 'influence', label: 'Influence', group: 'resource' as const, value: game.resources.influence, min: 0, max: 100, format: 'number' as const },
            { id: 'days_remaining', label: 'Days', group: 'resource' as const, value: game.resources.days_remaining, min: 0, max: 180, format: 'days' as const },
          ]).map((indicator) => (
            <View key={indicator.id} style={styles.resourceCard}>
              <Text style={styles.resourceLabel}>{indicator.label}</Text>
              <Text style={styles.resourceValue}>{indicator.format === 'money' ? money(indicator.value) : indicator.format === 'percent' ? `${indicator.value}%` : indicator.value}</Text>
            </View>
          ))}
        </View>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Current situation</Text>
          <Text style={styles.status}>{game.current_status}</Text>
        </View>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Civic capacity</Text>
          {(game.indicators?.filter((indicator) => indicator.group !== 'resource' && indicator.id !== 'progress') ?? [
            { id: 'documentation', label: 'Evidence strength', value: game.metrics.documentation, min: 0, max: 100 },
            { id: 'community_support', label: 'Public support', value: game.metrics.community_support, min: 0, max: 100 },
            { id: 'public_attention', label: 'Media pressure', value: game.metrics.public_attention, min: 0, max: 100 },
            { id: 'integrity', label: 'Integrity', value: game.metrics.integrity, min: 0, max: 100 },
          ]).map((indicator) => <Meter key={indicator.id} id={indicator.id} label={indicator.label} value={indicator.value} min={indicator.min} max={indicator.max} format={'format' in indicator ? indicator.format : 'percent'} />)}
        </View>

        {game.status === 'active' ? (
          <View style={styles.stack}>
            <Text style={styles.sectionTitle}>Choose the next action</Text>
            {game.available_actions.filter((action) => action.enabled).map((action) => (
              <Pressable
                key={action.id}
                accessibilityRole="button"
                disabled={busy}
                onPress={() => act(action.id)}
                style={({ pressed }) => [
                  styles.actionCard,
                  busy && styles.disabled,
                  pressed && styles.pressed,
                ]}
              >
                <Text style={styles.actionTitle}>{action.title}</Text>
                <Text style={styles.body}>{action.description}</Text>
                <Text style={styles.costLine}>
                  {action.cost.money ? money(action.cost.money) : ''} {action.cost.energy ? `· ${action.cost.energy} energy` : ''} {action.cost.influence ? `· ${action.cost.influence} influence` : ''} · {action.cost.days} days
                </Text>
              </Pressable>
            ))}
          </View>
        ) : (
          <View style={[styles.card, game.status === 'won' ? styles.won : styles.lost]}>
            <Text style={styles.hero}>{game.status === 'won' ? 'Mission completed' : 'Mission failed'}</Text>
            <Text style={styles.body}>{game.current_status}</Text>
            <Pressable style={styles.primaryButton} onPress={() => setGame(null)}>
              <Text style={styles.primaryButtonText}>Start another game</Text>
            </Pressable>
          </View>
        )}

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Case history</Text>
          {recentEvents.length === 0 ? <Text style={styles.body}>No official action yet.</Text> : recentEvents.map((event) => (
            <View key={`${event.turn}-${event.action_title}`} style={styles.event}>
              <Text style={styles.eventTitle}>Turn {event.turn}: {event.action_title}</Text>
              <Text style={styles.body}>{event.message}</Text>
              <Text style={styles.costLine}>Civic factors updated</Text>
            </View>
          ))}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: '#f4f1e9' },
  content: { padding: 16, gap: 14, paddingBottom: 36 },
  center: { flex: 1, alignItems: 'center', justifyContent: 'center', padding: 24, gap: 10 },
  stack: { gap: 12 },
  rowBetween: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', gap: 10 },
  eyebrow: { color: '#bb6b22', fontSize: 12, fontWeight: '800', letterSpacing: 1.5 },
  eyebrowLight: { color: '#f1bd82', fontSize: 12, fontWeight: '800', letterSpacing: 1.5 },
  hero: { color: '#17212b', fontSize: 34, fontWeight: '800', letterSpacing: -1.2 },
  title: { color: '#17212b', fontSize: 28, fontWeight: '800' },
  sectionTitle: { color: '#17212b', fontSize: 19, fontWeight: '800', marginTop: 4 },
  body: { color: '#5f6c77', fontSize: 15, lineHeight: 22 },
  bodyLight: { color: '#dce5eb', fontSize: 15, lineHeight: 22 },
  card: { borderWidth: 1, borderColor: '#d9d4c8', backgroundColor: '#fffdf8', borderRadius: 18, padding: 18, gap: 10 },
  cardTitle: { color: '#17212b', fontSize: 18, fontWeight: '800' },
  occupation: { color: '#bb6b22', fontSize: 13, fontWeight: '800' },
  difficultyRow: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  difficultyChip: { borderWidth: 1, borderColor: '#d9d4c8', borderRadius: 999, paddingHorizontal: 12, paddingVertical: 8 },
  difficultyChipActive: { borderColor: '#172c3f', backgroundColor: '#172c3f' },
  difficultyText: { color: '#5f6c77', fontSize: 13, fontWeight: '700' },
  difficultyTextActive: { color: '#fff' },
  primaryButton: { minHeight: 46, borderRadius: 12, backgroundColor: '#172c3f', alignItems: 'center', justifyContent: 'center', paddingHorizontal: 16, marginTop: 6 },
  primaryButtonText: { color: '#fff', fontWeight: '800' },
  secondaryButton: { minHeight: 44, borderRadius: 12, borderWidth: 1, borderColor: '#d9d4c8', alignItems: 'center', justifyContent: 'center', paddingHorizontal: 16, marginTop: 6 },
  secondaryButtonText: { color: '#172c3f', fontWeight: '800' },
  pressed: { opacity: 0.75 },
  disabled: { opacity: 0.48 },
  costLine: { color: '#6e5b46', fontSize: 13, fontWeight: '700', lineHeight: 19 },
  missionCard: { backgroundColor: '#172c3f', borderRadius: 20, padding: 20, gap: 9 },
  missionTitle: { color: '#fff', fontSize: 30, fontWeight: '800', letterSpacing: -0.8 },
  citizen: { color: '#fff', fontWeight: '800', marginTop: 10 },
  turn: { color: '#c3d1da', marginTop: 10 },
  resourceGrid: { flexDirection: 'row', flexWrap: 'wrap', gap: 10 },
  resourceCard: { width: '48%', flexGrow: 1, borderWidth: 1, borderColor: '#d9d4c8', backgroundColor: '#fff', borderRadius: 14, padding: 14 },
  resourceLabel: { color: '#5f6c77', fontSize: 11, fontWeight: '800', textTransform: 'uppercase' },
  resourceValue: { color: '#17212b', fontSize: 20, fontWeight: '800', marginTop: 3 },
  progressTrack: { height: 12, backgroundColor: '#e5e0d6', borderRadius: 999, overflow: 'hidden' },
  progressFill: { height: '100%', backgroundColor: '#bb6b22' },
  status: { color: '#5b462e', backgroundColor: '#f6e9da', borderRadius: 10, padding: 12, lineHeight: 20 },
  meterRow: { gap: 6, marginTop: 8 },
  meterLabel: { color: '#5f6c77', fontSize: 13 },
  meterValue: { color: '#17212b', fontSize: 13, fontWeight: '800' },
  track: { height: 8, backgroundColor: '#e6e1d7', borderRadius: 999, overflow: 'hidden' },
  fill: { height: '100%', backgroundColor: '#172c3f' },
  actionCard: { borderWidth: 1, borderColor: '#d9d4c8', backgroundColor: '#fffdf8', borderRadius: 16, padding: 16, gap: 7 },
  actionTitle: { color: '#17212b', fontSize: 17, fontWeight: '800' },
  warning: { color: '#9b3c35', fontSize: 13, fontWeight: '700' },
  event: { borderTopWidth: 1, borderTopColor: '#e4dfd5', paddingTop: 12, gap: 4 },
  eventTitle: { color: '#17212b', fontSize: 14, fontWeight: '800' },
  won: { backgroundColor: '#e1eee7', borderColor: '#aac8b8' },
  lost: { backgroundColor: '#f4e2df', borderColor: '#d7aaa6' },
});
