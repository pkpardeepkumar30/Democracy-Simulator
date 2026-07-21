export type VisualLocation = {
  id: string;
  label: string;
  x: number;
  y: number;
  icon?: string | null;
};

export type VisualTheme = {
  id?: string;
  layout?: string;
  primary_color?: string;
  accent_color?: string;
  background_color?: string;
  locations?: VisualLocation[];
};

export type VisualEvent = {
  focus_location_id?: string | null;
  animation?: string;
  effect?: string;
};

export type PublicAction = {
  id: string;
  title: string;
  location_id?: string | null;
  enabled: boolean;
  disabled_reason?: string | null;
};

export type PublicState = {
  citizen_id: string;
  status: 'active' | 'won' | 'lost';
  metrics: { progress: number };
  indicators?: { id: string; label: string; value: number; min: number; max: number; group: string }[];
  available_actions: PublicAction[];
};

export type PublicScenario = {
  visual_theme?: VisualTheme;
  citizens?: {
    id: string;
    visual?: { start_location_id?: string | null } | null;
  }[];
};

export type CityLocationView = VisualLocation & {
  x: number;
  y: number;
  actionId: string | null;
  actionTitle: string | null;
  enabled: boolean;
  actionCount: number;
};

export type CityViewModel = {
  palette: { primary: string; accent: string; background: string };
  locations: CityLocationView[];
  playerLocationId: string | null;
  factors: { id: string; label: string; value: number; min: number; max: number }[];
  status: PublicState['status'];
  visualEvent: VisualEvent | null;
};
