import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (relativePath) => readFile(path.join(repositoryRoot, relativePath), 'utf8');
const scriptSafe = (source) => source.replaceAll('</script>', '<\\/script>');

const [shell, styles, packText, offlineApi, application] = await Promise.all([
  read('web/index.html'),
  read('web/styles.css'),
  read('game-packs/drainage/game.json'),
  read('web/offline-api.js'),
  read('web/app.js'),
]);

const pack = JSON.stringify(JSON.parse(packText), null, 2);
const generated = shell
  .replace('  <link rel="manifest" href="/manifest.webmanifest" />\n', '')
  .replace('  <link rel="stylesheet" href="/styles.css" />', `<style>\n${styles}</style>`)
  .replace('Game state persists on the server', 'Game state persists in this browser')
  .replace(
    '  <script src="/city.bundle.js" defer></script>\n  <script src="/app.js" defer></script>',
    `  <script type="application/json" id="embeddedGamePack">${scriptSafe(pack)}</script>\n` +
      `  <script>${scriptSafe(offlineApi)}</script>\n` +
      `  <script>${scriptSafe(application)}</script>`,
  );

if (generated === shell || generated.includes('src="/app.js"')) {
  throw new Error('standalone shell markers did not match web/index.html');
}

await writeFile(path.join(repositoryRoot, 'PLAY_WITHOUT_DOCKER.html'), generated, 'utf8');
console.log('Generated PLAY_WITHOUT_DOCKER.html from the current web shell, pack and offline engine.');
