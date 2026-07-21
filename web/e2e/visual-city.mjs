import { chromium } from 'playwright-core';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:8080';
const executablePath = process.env.BROWSER_PATH;
const outputDirectory = process.env.SCREENSHOT_DIR ?? path.join(process.cwd(), 'test-results');
if (!executablePath) throw new Error('BROWSER_PATH must point to a Chromium or Edge executable');
await mkdir(outputDirectory, { recursive: true });

async function json(url, init) {
  const response = await fetch(`${baseUrl}${url}`, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(body.error ?? `HTTP ${response.status}`);
  return body;
}

const scenario = await json('/api/v1/scenarios/civic-drainage-v1');
const session = await json('/api/v1/sessions', {
  method: 'POST',
  body: JSON.stringify({ scenario_id: scenario.id, profile_id: scenario.citizens[0].id }),
});
const browser = await chromium.launch({ executablePath, headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 1 });
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
page.on('console', (message) => {
  if (message.type() === 'error') errors.push(message.text());
});

await page.goto(baseUrl, { waitUntil: 'networkidle' });
await page.evaluate((sessionId) => localStorage.setItem('civic-sim-session-id', sessionId), session.id);
await page.reload({ waitUntil: 'networkidle' });
const canvas = page.locator('#cityCanvasHost canvas');
await canvas.waitFor({ state: 'visible', timeout: 15_000 });
if (await page.locator('#cityDomFallback').isVisible()) throw new Error('DOM fallback remained visible with Phaser active');
if (await page.locator('.action-button').count() < 1) throw new Error('accessible DOM actions are missing');
await page.screenshot({ path: path.join(outputDirectory, 'visual-city-desktop.png'), fullPage: true });

const enabled = session.available_actions.find((action) => action.enabled && action.location_id);
if (!enabled) throw new Error('fixture session has no enabled location action');
const location = scenario.visual_theme.locations.find((item) => item.id === enabled.location_id);
if (!location) throw new Error('enabled action location is absent from the scenario');
const box = await canvas.boundingBox();
if (!box) throw new Error('city canvas has no layout box');
const logicalX = 55 + location.x * 8.9;
const logicalY = 72 + location.y * 4.65;
await page.mouse.click(box.x + (logicalX / 1000) * box.width, box.y + (logicalY / 600) * box.height);
await page.waitForFunction(() => document.querySelector('.turn-number strong')?.textContent !== '0', null, { timeout: 15_000 });
await page.locator('#resultDialog[open]').waitFor({ state: 'visible', timeout: 15_000 });
const factorFeedback = await page.locator('#dialogDelta').textContent();
if (!factorFeedback?.includes('Evidence strength +') || !factorFeedback.includes('% → ')) {
  throw new Error(`outcome dialog did not report percentage factor movement: ${factorFeedback}`);
}
await page.screenshot({ path: path.join(outputDirectory, 'visual-city-outcome.png'), fullPage: true });

await page.setViewportSize({ width: 390, height: 844 });
await page.reload({ waitUntil: 'networkidle' });
await page.locator('#cityCanvasHost canvas').waitFor({ state: 'visible', timeout: 15_000 });
const mobileBox = await page.locator('#cityCanvasHost canvas').boundingBox();
if (!mobileBox || mobileBox.width > 390 || mobileBox.height < 150) throw new Error('responsive city canvas has invalid dimensions');
await page.screenshot({ path: path.join(outputDirectory, 'visual-city-mobile.png'), fullPage: true });

const renderedScenarios = [scenario.id];
await page.setViewportSize({ width: 1100, height: 800 });
for (const scenarioId of ['examination-scandal-v1', 'factory-ground-v1']) {
  const nextScenario = await json(`/api/v1/scenarios/${scenarioId}`);
  const nextSession = await json('/api/v1/sessions', {
    method: 'POST',
    body: JSON.stringify({ scenario_id: scenarioId, profile_id: nextScenario.citizens[0].id }),
  });
  await page.evaluate((sessionId) => localStorage.setItem('civic-sim-session-id', sessionId), nextSession.id);
  await page.reload({ waitUntil: 'networkidle' });
  await page.locator('#cityCanvasHost canvas').waitFor({ state: 'visible', timeout: 15_000 });
  if (await page.locator('#cityDomFallback').isVisible()) throw new Error(`${scenarioId} fell back from Phaser`);
  renderedScenarios.push(scenarioId);
}

const generatedScenario = await json('/api/v1/scenarios/generate', {
  method: 'POST',
  body: JSON.stringify({
    seed: 20260721,
    selections: { objective_type: 'business_land', world_region: 'east_asian_industrial_city' },
    difficulty: 'standard',
    randomize_unspecified: true,
  }),
});
const generatedSession = await json('/api/v1/sessions', {
  method: 'POST',
  body: JSON.stringify({ scenario_id: generatedScenario.id, profile_id: generatedScenario.citizens[0].id }),
});
await page.evaluate((sessionId) => localStorage.setItem('civic-sim-session-id', sessionId), generatedSession.id);
await page.reload({ waitUntil: 'networkidle' });
await page.locator('#cityCanvasHost canvas').waitFor({ state: 'visible', timeout: 15_000 });
if (await page.locator('#cityDomFallback').isVisible()) throw new Error('generated scenario fell back from Phaser');
renderedScenarios.push(generatedScenario.id);

await browser.close();
if (errors.length) throw new Error(`browser errors: ${errors.join(' | ')}`);
console.log(JSON.stringify({
  session_id: session.id,
  action_id: enabled.id,
  factor_feedback: factorFeedback,
  desktop_canvas: { width: Math.round(box.width), height: Math.round(box.height) },
  mobile_canvas: { width: Math.round(mobileBox.width), height: Math.round(mobileBox.height) },
  rendered_scenarios: renderedScenarios,
  screenshots: outputDirectory,
}, null, 2));
