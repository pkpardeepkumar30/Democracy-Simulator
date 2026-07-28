import { CityRenderer } from './CityRenderer';
import { cityMapLibrary, cityPlansByAsset } from './cityPlans';
import type { CityMapLibrary, CityMapPlan } from './types';

declare global {
  interface Window {
    CivicCityRenderer?: typeof CityRenderer;
    CivicCityPlans?: Record<string, CityMapPlan>;
    CivicCityMapLibrary?: CityMapLibrary;
  }
}

window.CivicCityRenderer = CityRenderer;
window.CivicCityPlans = cityPlansByAsset;
window.CivicCityMapLibrary = cityMapLibrary;
