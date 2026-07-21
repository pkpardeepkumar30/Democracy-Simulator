import { CityRenderer } from './CityRenderer';

declare global {
  interface Window {
    CivicCityRenderer?: typeof CityRenderer;
  }
}

window.CivicCityRenderer = CityRenderer;
