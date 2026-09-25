#!/usr/bin/env node
// Rasterize saved SVGs at their intrinsic viewBox size. Mermaid CLI's PNG
// export resizes its viewport after measuring a responsive SVG, which can
// shrink the drawing inside the screenshot because of the page's margins.
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import puppeteer from 'puppeteer';

const inputs = process.argv.slice(2);
if (inputs.length === 0 || inputs.some(input => path.extname(input) !== '.svg')) {
  throw new Error('Usage: node rasterize-svg.mjs input.svg [input.svg ...]');
}

const browser = await puppeteer.launch({ headless: true });
try {
  const page = await browser.newPage();
  for (const input of inputs) {
    await page.goto(pathToFileURL(path.resolve(input)).href);
    const size = await page.evaluate(async () => {
      await document.fonts.ready;
      const svg = document.documentElement;
      const viewBox = svg.viewBox.baseVal;
      const width = viewBox.width || svg.width.baseVal.value;
      const height = viewBox.height || svg.height.baseVal.value;
      if (!(width > 0 && height > 0)) throw new Error('SVG has no positive dimensions');
      svg.style.width = `${width}px`;
      svg.style.height = `${height}px`;
      svg.style.maxWidth = 'none';
      svg.style.display = 'block';
      return { width: Math.ceil(width), height: Math.ceil(height) };
    });
    await page.setViewport({ ...size, deviceScaleFactor: 1 });
    await page.screenshot({
      path: input.slice(0, -4) + '.png',
      clip: { x: 0, y: 0, ...size },
    });
  }
} finally {
  await browser.close();
}
