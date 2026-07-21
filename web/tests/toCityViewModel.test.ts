import { describe, expect, it } from 'vitest';
import { toCityViewModel } from '../city-src/toCityViewModel';

describe('toCityViewModel', () => {
  it('maps only server-enabled actions to interactive locations', () => {
    const view = toCityViewModel(
      {
        visual_theme: {
          primary_color: '#112233',
          locations: [
            { id: 'office', label: 'Office', x: 120, y: -10 },
            { id: 'court', label: 'Court', x: 50, y: 50 },
          ],
        },
        citizens: [{ id: 'citizen', visual: { start_location_id: 'court' } }],
      },
      {
        citizen_id: 'citizen',
        status: 'active',
        metrics: { progress: 35 },
        indicators: [{ id: 'documentation', label: 'Evidence strength', value: 35, min: 0, max: 100, group: 'metric' }],
        available_actions: [
          { id: 'locked', title: 'Locked', location_id: 'office', enabled: false },
          { id: 'open', title: 'Open', location_id: 'court', enabled: true },
        ],
      },
    );
    expect(view.locations[0]).toMatchObject({ x: 100, y: 0, actionId: null, enabled: false });
    expect(view.locations[1]).toMatchObject({ actionId: 'open', enabled: true });
    expect(view.playerLocationId).toBe('court');
    expect(view.factors).toEqual([{ id: 'documentation', label: 'Evidence strength', value: 35, min: 0, max: 100, group: 'metric' }]);
  });

  it('disables every hotspot after a terminal result and preserves the visual cue', () => {
    const cue = { focus_location_id: 'office', animation: 'repair-complete', effect: 'success' };
    const view = toCityViewModel(
      { visual_theme: { locations: [{ id: 'office', label: 'Office', x: 10, y: 20 }] } },
      {
        citizen_id: 'citizen',
        status: 'won',
        metrics: { progress: 100 },
        available_actions: [{ id: 'action', title: 'Action', location_id: 'office', enabled: true }],
      },
      cue,
    );
    expect(view.locations[0].enabled).toBe(false);
    expect(view.visualEvent).toEqual(cue);
    expect(view.factors).toEqual([]);
  });
});
