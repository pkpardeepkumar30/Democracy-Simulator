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
  map_asset?: string | null;
  locations?: VisualLocation[];
};

export type CityMapFeature = {
  kind: 'road' | 'rail' | 'water';
  class: string;
  name?: string;
  points: [number, number][];
};

export type CityMapPlan = {
  id: string;
  city: string;
  country: string;
  label: string;
  bbox: [number, number, number, number];
  source_url: string;
  extract_url: string;
  features: CityMapFeature[];
};

export type CityMapLibrary = {
  schema_version: number;
  generated_at: string;
  attribution: string;
  extract_provider: string;
  license: string;
  license_url: string;
  plans: CityMapPlan[];
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
  mapPlan: CityMapPlan | null;
  status: PublicState['status'];
  visualEvent: VisualEvent | null;
};
