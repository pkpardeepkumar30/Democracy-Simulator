import cityPlanData from '../city-data/city-plans.json';
import type { CityMapLibrary, CityMapPlan } from './types';

export const cityMapLibrary = cityPlanData as CityMapLibrary;

export const cityPlansByAsset = Object.fromEntries(
  cityMapLibrary.plans.map((plan) => [`osm:${plan.id}`, plan]),
) as Record<string, CityMapPlan>;

export function cityPlanForAsset(asset: string | null | undefined): CityMapPlan | null {
  return asset ? cityPlansByAsset[asset] ?? null : null;
}
