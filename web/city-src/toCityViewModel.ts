import type { CityViewModel, PublicScenario, PublicState, VisualEvent } from './types';

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, Number.isFinite(value) ? value : min));
}

export function toCityViewModel(
  scenario: PublicScenario,
  state: PublicState,
  visualEvent: VisualEvent | null = null,
): CityViewModel {
  const theme = scenario.visual_theme ?? {};
  const locations = (theme.locations ?? []).map((location) => {
    const actions = state.available_actions.filter((action) => action.enabled && action.location_id === location.id);
    const selected = actions[0] ?? null;
    return {
      ...location,
      x: clamp(location.x, 0, 100),
      y: clamp(location.y, 0, 100),
      actionId: selected?.id ?? null,
      actionTitle: selected?.title ?? null,
      enabled: Boolean(selected?.enabled && state.status === 'active'),
      actionCount: actions.length,
    };
  });
  const profile = scenario.citizens?.find((citizen) => citizen.id === state.citizen_id);
  const declaredStart = profile?.visual?.start_location_id ?? null;
  return {
    palette: {
      primary: theme.primary_color ?? '#172c3f',
      accent: theme.accent_color ?? '#bb6b22',
      background: theme.background_color ?? '#f4f1e9',
    },
    locations,
    playerLocationId: locations.some((location) => location.id === declaredStart)
      ? declaredStart
      : locations[0]?.id ?? null,
    factors: (state.indicators ?? [])
      .filter((indicator) => indicator.group !== 'resource' && indicator.id !== 'progress')
      .slice(0, 6)
      .map((indicator) => ({
        ...indicator,
        value: clamp(indicator.value, indicator.min, indicator.max),
      })),
    status: state.status,
    visualEvent,
  };
}
