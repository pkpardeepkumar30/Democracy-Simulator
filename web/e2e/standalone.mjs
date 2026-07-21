import { chromium } from 'playwright-core';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const executablePath = process.env.BROWSER_PATH;
if (!executablePath) throw new Error('BROWSER_PATH must point to a Chromium or Edge executable');

const standaloneUrl = pathToFileURL(path.resolve(process.cwd(), '..', 'PLAY_WITHOUT_DOCKER.html')).href;
const browser = await chromium.launch({ executablePath, headless: true });
const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
const errors = [];
page.on('pageerror', (error) => errors.push(error.message));
page.on('console', (message) => {
  if (message.type() === 'error') errors.push(message.text());
});

await page.goto(standaloneUrl, { waitUntil: 'load' });
await page.evaluate(() => localStorage.clear());
await page.reload({ waitUntil: 'load' });
await page.locator('.select-scenario').click();
await page.locator('.select-profile').first().click();

const initialIds = await page.locator('.action-button').evaluateAll((buttons) =>
  buttons.map((button) => button.dataset.actionId),
);
if (initialIds.join(',') !== 'file_complaint,collect_signatures') {
  throw new Error(`standalone leaked unavailable initial actions: ${initialIds.join(',')}`);
}
if (await page.locator('.action-button:disabled').count()) {
  throw new Error('standalone displayed unavailable actions as disabled buttons');
}

await page.locator('[data-action-id="file_complaint"]').first().click();
await page.locator('#resultDialog[open]').waitFor({ state: 'visible' });
const factorFeedback = await page.locator('#dialogDelta').textContent();
if (!factorFeedback?.includes('Evidence strength +') || !factorFeedback.includes('% → ')) {
  throw new Error(`standalone did not report percentage factor movement: ${factorFeedback}`);
}
await page.locator('#dialogClose').click();
if (await page.locator('[data-action-id="file_complaint"]').count()) {
  throw new Error('standalone retained the consumed complaint action');
}
if (!(await page.locator('[data-action-id="visit_office"]').count())) {
  throw new Error('standalone did not expose the newly unlocked office action');
}

await browser.close();
if (errors.length) throw new Error(`standalone browser errors: ${errors.join(' | ')}`);
console.log(JSON.stringify({ initial_actions: initialIds, factor_feedback: factorFeedback, consumed_action_hidden: true, unlocked_action_visible: true }));
