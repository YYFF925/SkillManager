// Disposable screenshot renderer. Uses an isolated headless browser and only local prototype files.
const { chromium } = require('/Users/fei/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs = require('fs');
const path = require('path');
const { pathToFileURL } = require('url');
const frames = [
  ['01-code-library', 'view=code'],
  ['02-lobster-library', 'view=lobster'],
  ['03-shared-agent', 'view=shared'],
  ['04-codex-agent', 'view=codex'],
  ['05-hermes-profiles', 'view=hermes'],
  ['06-hermes-profile-picker', 'view=hermes&picker=1'],
  ['07-upstream-changes', 'view=code&modal=updates'],
  ['08-preset-overview', 'view=presets'],
  ['09-preset-detail', 'view=preset-detail&preset=matt'],
  ['10-preset-members-filter', 'view=preset-detail&preset=matt&filter=member'],
  ['11-preset-deployment', 'view=preset-detail&preset=matt&modal=preset-deploy'],
  ['12-preset-create', 'view=presets&modal=preset-create'],
  ['13-global-preset', 'view=preset-detail&preset=global'],
  ['14-global-instructions', 'view=code&modal=instructions'],
  ['15-cursor-instructions', 'view=code&modal=instructions&instruction=cursor'],
  ['16-preset-sidebar-actions', 'view=preset-detail&domain=lobster&preset=home'],
  ['17-skill-drag-handle', 'view=preset-detail&domain=lobster&preset=home'],
  ['18-preset-icon-picker', 'view=preset-detail&domain=lobster&preset=auto&modal=preset-rename'],
  ['19-settings', 'view=settings'],
  ['20-settings-edit-path', 'view=settings'],
  ['21-hermes-skill-detail', 'view=hermes'],
  ['22-skill-file-tree', 'view=hermes'],
  ['23-skill-card-menu', 'view=code'],
  ['24-skill-add-tags', 'view=code'],
  ['25-single-skill-update', 'view=code'],
  ['26-deployment-indicators', 'view=code'],
  ['27-windows-unsupported', 'view=code'],
  ['28-existing-deployment-warning', 'view=code'],
  ['29-agent-upload-question', 'view=code'],
  ['30-hermes-readonly-manager', 'view=hermes'],
  ['31-manager-compact-layout', 'view=code'],
  ['32-intake-single-page', 'view=code'],
  ['33-intake-inline-preview', 'view=code'],
  ['34-repository-update-review', 'view=code'],
  ['35-upstream-additions-list', 'view=code'],
  ['36-adopt-metadata', 'view=codex'],
  ['37-batch-adopt-metadata', 'view=codex'],
  ['38-maintenance-task', 'view=code'],
  ['39-skip-upstream-version', 'view=code'],
  ['40-skipped-versions', 'view=code'],
  ['41-tool-center', 'view=tools'],
  ['42-tool-overview', 'view=tools&tool=browser-use'],
  ['43-tool-maintenance-guide', 'view=tools&tool=browser-use'],
  ['44-tool-update-task', 'view=tools&tool=browser-use'],
  ['45-tool-skill-intake', 'view=tools&tool=browser-use'],
  ['46-tool-registration', 'view=tools']
];
const requestedFrames = process.argv.slice(2);
const selectedFrames = requestedFrames.length ? frames.filter(([name]) => requestedFrames.includes(name)) : frames;
(async () => {
  const output = path.join(__dirname, 'renders');
  fs.mkdirSync(output, { recursive: true });
  const browser = await chromium.launch({
    executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    headless: true
  });
  try {
    const context = await browser.newContext({ viewport: { width: 1720, height: 1040 }, deviceScaleFactor: 1 });
    await context.route('**/*', route => {
      const url = route.request().url();
      return url.startsWith('file:') || url.startsWith('data:') ? route.continue() : route.abort();
    });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    for (const [name, query] of selectedFrames) {
      await page.goto(pathToFileURL(path.join(__dirname, 'index.html')).href + '?' + query + '&capture=1');
      await page.evaluate(() => document.fonts.ready);
      if (name === '21-hermes-skill-detail' || name === '22-skill-file-tree') {
        await page.locator('[data-action="profile-detail"][data-name="cloud-vps-management"]').first().click();
        if (name === '22-skill-file-tree') await page.locator('[data-action="skill-tab"][data-value="files"]').click();
      }
      if (name === '23-skill-card-menu' || name === '24-skill-add-tags') {
        const card = page.locator('[data-skill="code:agent-skill-deployment"]');
        await card.hover();
        await card.locator('[data-action="card-menu"]').click();
        if (name === '24-skill-add-tags') await page.locator('#card-menu [data-action="skill-add-tags"]').click();
      }
      if (name === '25-single-skill-update') await page.locator('[data-skill="code:matt-code-review"] [data-action="skill-update"]').click();
      if (name === '27-windows-unsupported') await page.evaluate(() => { Model.updateSkill('code:matt-ask-matt', {manager: {systems: ['macos']}}); Model.setSystem('windows'); render(); });
      if (name === '28-existing-deployment-warning') await page.evaluate(() => { Model.updateSkill('code:matt-ask-matt', {manager: {agents: ['codex']}}); render(); });
      if (name === '26-deployment-indicators' || name === '28-existing-deployment-warning') {
        await page.locator('[data-skill="code:matt-ask-matt"] [data-action="skill"]').first().click();
        await page.locator('[data-action="skill-tab"][data-value="files"]').click();
      }
      if (name === '29-agent-upload-question') {
        await page.evaluate(() => agentEditDialog());
        await page.locator('#agent-body').fill('# Agent maintenance example');
        await page.locator('[data-action="agent-cli"]').click();
      }
      if (name === '30-hermes-readonly-manager') {
        await page.locator('[data-action="profile-detail"][data-name="cloud-vps-management"]').first().click();
        await page.locator('[data-action="skill-tab"][data-value="manager"]').click();
      }
      if (name === '31-manager-compact-layout') {
        await page.locator('[data-skill="code:matt-ask-matt"] [data-action="skill"]').first().click();
        await page.locator('[data-action="skill-tab"][data-value="manager"]').click();
      }
      if (name === '32-intake-single-page' || name === '33-intake-inline-preview') {
        await page.locator('[data-action="intake"]').click();
        if (name === '33-intake-inline-preview') await page.locator('[data-action="intake-preview"]').click();
      }
      if (name === '34-repository-update-review') {
        await page.locator('[data-action="updates"]').click();
        await page.locator('[data-action="repo-update"][data-repo="matt-skills"]').click();
      }
      if (name === '35-upstream-additions-list') await page.locator('[data-action="new-updates"]').click();
      if (name === '36-adopt-metadata') await page.locator('[data-action="adopt"][data-name="local-notes"]').click();
      if (name === '37-batch-adopt-metadata') {
        await page.locator('[data-action="multi"]').click();
        await page.locator('[data-select="codex|local-notes"]').check();
        await page.locator('[data-select="codex|quick-shell"]').check();
        await page.locator('[data-action="batch-adopt"]').click();
      }
      if (name === '38-maintenance-task') {
        const card = page.locator('[data-skill="code:matt-code-review"]');
        await card.hover();
        await card.locator('[data-action="card-menu"]').click();
        await page.locator('#card-menu [data-action="maintenance-task"]').click();
        await page.locator('#maintenance-request').fill('补充所选技能的中文正文与中文注释，整理代码评审相关标签，保留 Codex 专用覆盖内容。');
      }
      if (name === '39-skip-upstream-version' || name === '40-skipped-versions') {
        await page.locator('[data-skill="code:matt-code-review"] [data-action="skill-update"]').click();
        if (name === '40-skipped-versions') {
          await page.locator('[data-action="skill-skip-version"]').click();
          await page.locator('[data-action="updates"]').first().click();
          await page.locator('[data-action="skipped-updates"]').click();
        }
      }
      if (name === '43-tool-maintenance-guide') await page.locator('[data-action="tool-tab"][data-value="guide"]').click();
      if (name === '44-tool-update-task') await page.locator('[data-action="tool-task-update"]').first().click();
      if (name === '45-tool-skill-intake') {
        await page.locator('[data-action="tool-tab"][data-value="skill"]').first().click();
        await page.locator('[data-action="tool-skill-intake"]').click();
      }
      if (name === '46-tool-registration') await page.locator('[data-action="tool-register"]').click();
      const state = await page.evaluate(() => ({
        cards: document.querySelectorAll('.card,.preset-box').length,
        brokenImages: [...document.images].filter(i => !i.complete || i.naturalWidth === 0).map(i => i.src),
        horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
        title: document.querySelector('.headline h1')?.textContent
      }));
      if (errors.length || state.brokenImages.length || state.horizontalOverflow) {
        throw new Error(JSON.stringify({ name, errors, state }));
      }
      if (name === '20-settings-edit-path') {
        await page.locator('.settings-agent-card[data-agent="codex"]').hover();
        await page.locator('[data-action="settings-path-edit"][data-target="codex"][data-kind="skills"]').click();
      }
      if (name === '16-preset-sidebar-actions') await page.locator('.preset-row.selected').hover();
      if (name === '17-skill-drag-handle') await page.locator('#skill-grid .card').first().hover();
      await page.screenshot({ path: path.join(output, name + '.png') });
      console.log(JSON.stringify({ name, ...state }));
    }
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exit(1); });
