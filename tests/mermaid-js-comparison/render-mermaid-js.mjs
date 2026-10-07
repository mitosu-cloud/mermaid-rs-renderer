#!/usr/bin/env node
// Use Mermaid CLI's renderer with one browser for the complete fixture corpus.
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import puppeteer from 'puppeteer';
import { renderMermaid } from '@mermaid-js/mermaid-cli';

const directory = path.dirname(fileURLToPath(import.meta.url));
const reference = path.join(directory, 'reference');
const output = path.join(directory, 'comparison-output');
const requested = new Set(process.argv.slice(2).map(value => path.basename(value, '.mmd')));
const files = (await fs.readdir(reference)).filter(name => name.endsWith('.mmd') &&
  (!requested.size || requested.has(path.basename(name, '.mmd')))).sort();
if (!files.length) throw new Error('No matching comparison fixtures');
for (const stem of requested) {
  if (!files.includes(`${stem}.mmd`)) throw new Error(`Comparison fixture not found: ${stem}`);
}
await fs.mkdir(output, { recursive: true });
const versions = {};
for (const name of ['mermaid', '@mermaid-js/mermaid-cli']) {
  const metadata = JSON.parse(await fs.readFile(path.join(directory, 'node_modules', name, 'package.json'), 'utf8'));
  versions[name] = metadata.version;
}
const report = { generatedAt: new Date().toISOString(), versions, total: files.length,
  svg: 0, png: 0, failures: [] };
let next = 0;
let completed = 0;
const browser = await puppeteer.launch({ headless: true });
try {
  await Promise.all(Array.from({ length: Math.min(3, files.length) }, async () => {
    const page = await browser.newPage();
    try {
      while (next < files.length) {
        const file = files[next++];
        const stem = path.basename(file, '.mmd');
        const svgPath = path.join(output, `${stem}-js.svg`);
        const pngPath = path.join(output, `${stem}-js.png`);
        await Promise.all([svgPath, pngPath].map(value => fs.rm(value, { force: true })));
        let format = 'svg';
        try {
          const definition = await fs.readFile(path.join(reference, file), 'utf8');
          const { data } = await renderMermaid(browser, definition, 'svg', {
            svgId: 'my-svg', viewport: { width: 800, height: 600, deviceScaleFactor: 1 },
          });
          const svg = new TextDecoder().decode(data);
          if (svg.includes('Syntax error in text')) throw new Error('Mermaid returned an error diagram');
          await fs.writeFile(svgPath, data);
          report.svg++;
          format = 'png';
          await page.goto(pathToFileURL(svgPath).href);
          const size = await page.evaluate(async () => {
            await document.fonts.ready;
            const element = document.documentElement;
            const bounds = element.viewBox.baseVal;
            if (!(bounds.width > 0 && bounds.height > 0)) throw new Error('SVG has no positive dimensions');
            element.style.width = `${bounds.width}px`;
            element.style.height = `${bounds.height}px`;
            element.style.maxWidth = 'none';
            element.style.display = 'block';
            return { width: Math.ceil(bounds.width), height: Math.ceil(bounds.height) };
          });
          await page.setViewport({ ...size, deviceScaleFactor: 1 });
          await page.screenshot({ path: pngPath, clip: { x: 0, y: 0, ...size } });
          report.png++;
        } catch (error) {
          report.failures.push({ fixture: stem, format, error: String(error) });
          console.error(`[JS ${format} FAIL] ${stem}: ${String(error).split('\n')[0]}`);
        }
        completed++;
        if (completed % 25 === 0) console.log(`Mermaid JS: ${completed}/${files.length} rendered`);
      }
    } finally {
      await page.close();
    }
  }));
} finally {
  await browser.close();
}
await fs.writeFile(path.join(output, 'mermaid-js-run.json'), JSON.stringify(report, null, 2) + '\n');
console.log(`Mermaid ${versions.mermaid}, CLI ${versions['@mermaid-js/mermaid-cli']}: ${report.svg}/${files.length} SVGs, ${report.png}/${files.length} PNGs; ${report.failures.length} failures`);
if (report.failures.length) process.exitCode = 1;
