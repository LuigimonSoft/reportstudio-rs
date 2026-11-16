import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const pdfjsDir = path.join(here, 'node_modules', 'pdfjs-dist');
let directory, server, baseURL, reports;

test.beforeAll(async () => {
  directory = await mkdtemp(path.join(os.tmpdir(), 'reportstudio-pdf-browser-'));
  const makeReport = (filename, first, reportPath) => {
    const output = path.join(directory, filename);
    execFileSync('cargo', ['run', '--quiet', '--locked', '--example', 'browser_fixture', '--', output, first, ...(reportPath ? [reportPath] : [])], {
      cwd: root,
      env: process.env,
      stdio: 'pipe',
    });
    return output;
  };
  const graphicReport = JSON.parse(await readFile(path.join(root, 'examples/simple-report.json'), 'utf8'));
  graphicReport.report.header.elements.push(
    {id:'box',type:'box',x:4.8,y:0.08,width:0.8,height:0.25,style:{color:'#ff0000'}},
    {id:'line',type:'line',x:6,y:0.10,width:1,height:0.025,style:{color:'#0000ff'}},
  );
  const graphicPath = path.join(directory, 'graphics.json');
  await writeFile(graphicPath, JSON.stringify(graphicReport));
  reports = {
    '/graphics.pdf': makeReport('graphics.pdf', 'Implementation consulting', graphicPath),
    '/designer-reference.pdf': path.join(here, 'fixtures', 'designer-simple-reference.pdf'),
    '/typography.pdf': makeReport('typography.pdf', 'Implementation consulting', path.join(here, 'fixtures', 'typography-report.json')),
    '/typography-reference.pdf': path.join(here, 'fixtures', 'designer-typography-reference.pdf'),
    '/table.pdf': makeReport('table.pdf', 'Fragile equipment for international shipment', path.join(here, 'fixtures', 'table-report.json')),
    '/table-reference.pdf': path.join(here, 'fixtures', 'designer-table-reference.pdf'),
    '/original.pdf': makeReport('original.pdf', 'Implementation consulting'),
    '/changed.pdf': makeReport('changed.pdf', 'Changed shipment'),
  };
  server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      let file;
      if (reports[url.pathname]) file = reports[url.pathname];
      else if (url.pathname === '/pdf.mjs' || url.pathname === '/pdf.worker.mjs') file = path.join(pdfjsDir, 'build', url.pathname.slice(1));
      else if (/^\/fonts\/[A-Za-z0-9_.-]+\.(ttf|pfb)$/.test(url.pathname)) file = path.join(pdfjsDir, 'standard_fonts', path.basename(url.pathname));
      else if (url.pathname === '/') { response.writeHead(200, {'content-type':'text/html; charset=utf-8'}); response.end('<!doctype html><title>Report Studio PDF test</title><canvas id="page"></canvas>'); return; }
      else { response.writeHead(404); response.end(); return; }
      const bytes = await readFile(file);
      response.writeHead(200, {'content-type': file.endsWith('.mjs') ? 'text/javascript' : file.endsWith('.pdf') ? 'application/pdf' : 'application/octet-stream'});
      response.end(bytes);
    } catch { response.writeHead(500); response.end(); }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  baseURL = `http://127.0.0.1:${server.address().port}`;
});
test.afterAll(async () => {
  if (server?.listening) await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  if (directory) await rm(directory, {recursive:true, force:true});
});

async function inspectPDF(page, filename) {
  await page.goto(baseURL);
  return page.evaluate(async (file) => {
    const pdfjs = await import('/pdf.mjs');
    pdfjs.GlobalWorkerOptions.workerSrc = '/pdf.worker.mjs';
    const task = pdfjs.getDocument({url: file, standardFontDataUrl: `${location.origin}/fonts/`, useSystemFonts:false});
    try {
      const document = await task.promise;
      const first = await document.getPage(1);
      const viewport = first.getViewport({scale:1});
      const canvas = globalThis.document.querySelector('#page');
      canvas.width = Math.ceil(viewport.width); canvas.height = Math.ceil(viewport.height);
      const context = canvas.getContext('2d');
      await first.render({canvas, canvasContext:context, viewport}).promise;
      const text = await first.getTextContent();
      const items = text.items.filter(item => typeof item.str === 'string' && item.str.trim()).map(item => {
        const [x,y] = viewport.convertToViewportPoint(item.transform[4], item.transform[5]);
        const fontSize = Math.hypot(item.transform[2], item.transform[3]);
        const left = Math.max(0, Math.floor(x));
        const top = Math.max(0, Math.floor(y - fontSize - 3));
        const right = Math.min(canvas.width, Math.ceil(x + item.width + 3));
        const bottom = Math.min(canvas.height, Math.ceil(y + 3));
        const pixels = context.getImageData(left, top, Math.max(1,right-left), Math.max(1,bottom-top)).data;
        let ink = 0;
        for (let i=0; i<pixels.length; i+=4) if (pixels[i+3] && Math.min(pixels[i],pixels[i+1],pixels[i+2]) < 220) ink++;
        return {text:item.str, x:item.transform[4], y:item.transform[5], width:item.width, ink};
      });
      const pixel = (x, y) => Array.from(context.getImageData(x, y, 1, 1).data);
      return {pageCount:document.numPages, width:viewport.width, height:viewport.height, items,
        samples: {box:pixel(400,50), line:pixel(480,44), blank:pixel(400,80)},
        tableEdge:Array.from(context.getImageData(36,77,2,58).data)};
    } finally { await task.destroy(); }
  }, filename);
}

test('the generated PDF opens, has correct page geometry and renders every text item', async ({page}) => {
  const report = await inspectPDF(page, '/original.pdf');
  expect(report.pageCount).toBe(1);
  expect(report.width).toBeCloseTo(612, 4);
  expect(report.height).toBeCloseTo(792, 4);
  const expected = [
    {text:'Simple report', x:36, y:744.512},
    {text:'Implementation consulting', x:36, y:712.82},
    {text:'Report development', x:36, y:694.82},
  ];
  for (const wanted of expected) {
    const item = report.items.find(entry => entry.text === wanted.text);
    expect(item, `missing ${wanted.text}`).toBeDefined();
    expect(item.x).toBeCloseTo(wanted.x, 1);
    expect(item.y).toBeCloseTo(wanted.y, 1);
    expect(item.width).toBeGreaterThan(0);
    expect(item.ink, `${wanted.text} should be visible on the rendered canvas`).toBeGreaterThan(10);
  }
  await expect(page.locator('#page')).toHaveAttribute('width', '612');
  await expect(page.locator('#page')).toHaveAttribute('height', '792');
});

test('text positions match the designer reference PDF', async ({page}) => {
  const actual = await inspectPDF(page, '/original.pdf');
  const reference = await inspectPDF(page, '/designer-reference.pdf');
  expect(actual.pageCount).toBe(reference.pageCount);
  expect(actual.width).toBeCloseTo(reference.width, 3);
  expect(actual.height).toBeCloseTo(reference.height, 3);
  for (const expected of reference.items) {
    const item = actual.items.find(candidate => candidate.text === expected.text);
    expect(item, `missing ${expected.text}`).toBeDefined();
    expect(item.x).toBeCloseTo(expected.x, 2);
    expect(item.y).toBeCloseTo(expected.y, 2);
    expect(item.width).toBeCloseTo(expected.width, 2);
  }
});

test('aligned, bold italic and multiline text match the designer reference', async ({page}) => {
  const actual = await inspectPDF(page, '/typography.pdf');
  const reference = await inspectPDF(page, '/typography-reference.pdf');
  expect(actual.items.length).toBe(reference.items.length);
  for (const expected of reference.items) {
    const item = actual.items.find(candidate => candidate.text === expected.text);
    expect(item, `missing ${expected.text}`).toBeDefined();
    expect(item.x, expected.text).toBeCloseTo(expected.x, 2);
    expect(item.y, expected.text).toBeCloseTo(expected.y, 2);
    expect(item.width, expected.text).toBeCloseTo(expected.width, 2);
    expect(item.ink, expected.text).toBeGreaterThan(0);
  }
});

test('repeated table rows and wrapped text match the designer reference', async ({page}) => {
  const actual = await inspectPDF(page, '/table.pdf');
  const reference = await inspectPDF(page, '/table-reference.pdf');
  expect(actual.pageCount).toBe(reference.pageCount);
  expect(actual.items.length).toBe(reference.items.length);
  expect(reference.tableEdge.some((channel, index) => index % 4 !== 3 && channel < 240)).toBe(true);
  const edgeDifference = Math.max(...actual.tableEdge.map((channel, index) => Math.abs(channel - reference.tableEdge[index])));
  // A tolerance of three levels allows raster rounding, but still detects a missing border or gap.
  expect(edgeDifference).toBeLessThanOrEqual(3);
  for (const expected of reference.items) {
    const item = actual.items.find(candidate => candidate.text === expected.text);
    expect(item, `missing ${expected.text}`).toBeDefined();
    expect(item.x, expected.text).toBeCloseTo(expected.x, 2);
    expect(item.y, expected.text).toBeCloseTo(expected.y, 2);
    expect(item.width, expected.text).toBeCloseTo(expected.width, 2);
    expect(item.ink, expected.text).toBeGreaterThan(0);
  }
});

test('line and box are rendered with the expected colors and positions', async ({page}) => {
  const report = await inspectPDF(page, '/graphics.pdf');
  const [red, green, blue] = report.samples.box;
  expect(red).toBeGreaterThan(240); expect(green).toBeLessThan(20); expect(blue).toBeLessThan(20);
  const [lineRed, lineGreen, lineBlue] = report.samples.line;
  expect(lineRed).toBeLessThan(40); expect(lineGreen).toBeLessThan(40); expect(lineBlue).toBeGreaterThan(220);
  expect(report.samples.blank.slice(0,3)).toEqual([255,255,255]);
});

test('changing the supplied data changes the generated document', async ({page}) => {
  const report = await inspectPDF(page, '/changed.pdf');
  const texts = report.items.map(item => item.text);
  expect(texts).toContain('Changed shipment');
  expect(texts).toContain('Report development');
  expect(texts).not.toContain('Implementation consulting');
  expect(report.items.find(item => item.text === 'Changed shipment').ink).toBeGreaterThan(10);
});

test('the PDF parser rejects malformed documents', async ({page}) => {
  await page.goto(baseURL);
  const errorName = await page.evaluate(async () => {
    const pdfjs = await import('/pdf.mjs');
    pdfjs.GlobalWorkerOptions.workerSrc = '/pdf.worker.mjs';
    const task = pdfjs.getDocument({data:new TextEncoder().encode('not a PDF')});
    try { await task.promise; return null; } catch (error) { return error.name; } finally { await task.destroy(); }
  });
  expect(errorName).toBe('InvalidPDFException');
});
