#!/usr/bin/env node
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer';

function readArg(name, short) {
  const index = process.argv.findIndex((arg) => arg === name || arg === short);
  if (index === -1 || index + 1 >= process.argv.length) {
    return undefined;
  }
  return process.argv[index + 1];
}

const input = readArg('--input', '-i');
const output = readArg('--output', '-o');
const puppeteerConfigFile = readArg('--puppeteerConfigFile', '-p');

if (!input || !output) {
  console.error('Usage: node render-upstream-mermaid.mjs -i input.mmd -o output.svg [-p puppeteer-config.json]');
  process.exit(1);
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const upstreamMermaid = process.env.MERMAID_UPSTREAM_DIST
  ? path.resolve(process.env.MERMAID_UPSTREAM_DIST)
  : path.resolve(scriptDir, '../../../mermaid/packages/mermaid/dist/mermaid.min.js');

let puppeteerConfig = {
  args: ['--no-sandbox', '--disable-setuid-sandbox'],
};

if (puppeteerConfigFile) {
  const loaded = JSON.parse(await fs.readFile(puppeteerConfigFile, 'utf8'));
  puppeteerConfig = { ...puppeteerConfig, ...loaded };
}

const source = await fs.readFile(input, 'utf8');

try {
  await fs.access(upstreamMermaid);
} catch {
  console.error(`Missing upstream Mermaid bundle: ${upstreamMermaid}`);
  process.exit(1);
}

const browser = await puppeteer.launch(puppeteerConfig);

try {
  const page = await browser.newPage();
  await page.setContent('<!doctype html><html><body><div id="container"></div></body></html>', {
    waitUntil: 'load',
  });
  await page.addScriptTag({ path: upstreamMermaid });

  const svg = await page.evaluate(async (diagram) => {
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'loose',
    });
    const result = await mermaid.render('my-svg', diagram);
    return result.svg;
  }, source);

  await fs.writeFile(output, `${svg}\n`, 'utf8');
} finally {
  await browser.close();
}
