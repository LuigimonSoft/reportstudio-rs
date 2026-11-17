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
  const awbPath = path.join(directory, 'awb.pdf');
  const awbChangedPath = path.join(directory, 'awb-changed.pdf');
  for (const [file, shipper] of [[awbPath, 'NORTHSTAR EXPORTS LLC'], [awbChangedPath, 'ALTERNATIVE SHIPPER LLC']]) {
    execFileSync('cargo', ['run', '--quiet', '--locked', '--example', 'awb_fixture', '--', file, shipper], {cwd:root, env:process.env, stdio:'pipe'});
  }
  const numberPath = path.join(directory, 'numbers.pdf');
  execFileSync('cargo', ['run', '--quiet', '--locked', '--example', 'number_fixture', '--', numberPath, path.join(here, 'fixtures', 'number-report.json')], {cwd:root, env:process.env, stdio:'pipe'});
  const multipage = JSON.parse(await readFile(path.join(root, 'examples/simple-report.json'), 'utf8'));
  const field = text => ({id:'p',type:'text',x:0,y:0,width:4,height:0.2,text,style:{fontsize:10,color:'#17213a'}});
  multipage.report.pageHeader = {id:'ph',height:0.3,elements:[field('HEADER')]};
  multipage.report.pageFooter = {id:'pf',height:0.3,elements:[field('Page [PAGE_NUMBER] of [PAGE_COUNT]')]};
  multipage.report.footer = {id:'rf',height:0.3,elements:[field('REPORT END')]};
  const multiJson = path.join(directory, 'multipage.json');
  const multiPdf = path.join(directory, 'multipage.pdf');
  await writeFile(multiJson, JSON.stringify(multipage));
  execFileSync('cargo', ['run','--quiet','--locked','--example','pagination_fixture','--',multiPdf,multiJson], {cwd:root,env:process.env,stdio:'pipe'});
  const unicodeReport = JSON.parse(await readFile(path.join(root,'examples/simple-report.json'),'utf8'));
  unicodeReport.report.header.elements[0].text='México Montréal 東京';
  unicodeReport.report.header.elements[0].style.bold=false;
  const unicodeJson=path.join(directory,'unicode.json');await writeFile(unicodeJson,JSON.stringify(unicodeReport));
  const chartReport = JSON.parse(await readFile(path.join(root,'examples/simple-report.json'),'utf8'));
  chartReport.report.header.height=2.7;
  chartReport.report.header.elements.push({id:'chart',type:'chart',x:0,y:0.5,width:4,height:2,style:{color:'#000000'},chart:{kind:'column',dataset:'details',category:'description',value:'',aggregation:'count',sort:'source',limit:10,title:'Sales chart',color:'#2563eb',categoryColors:{'Implementation consulting':'#ff0000'},background:'#ffffff',showLegend:true,showLabels:true,showGrid:true}});
  const chartJson=path.join(directory,'chart.json');await writeFile(chartJson,JSON.stringify(chartReport));
  const groupReport = JSON.parse(await readFile(path.join(root,'examples/simple-report.json'),'utf8'));
  groupReport.report.details[0].groupBy='category';
  const groupField = text => ({id:'g',type:'text',x:0,y:0,width:4,height:0.2,text,style:{fontsize:10,color:'#17213a'}});
  groupReport.report.details[0].groupHeader={id:'gh',height:0.3,elements:[groupField('Group [category]')]};
  groupReport.report.details[0].groupFooter={id:'gf',height:0.3,elements:[groupField('Subtotal [SUM(amount)]')]};
  const groupJson=path.join(directory,'group.json'), groupPdf=path.join(directory,'group.pdf');
  await writeFile(groupJson,JSON.stringify(groupReport));
  execFileSync('cargo',['run','--quiet','--locked','--example','group_fixture','--',groupPdf,groupJson],{cwd:root,env:process.env,stdio:'pipe'});
  const imageReport = JSON.parse(await readFile(path.join(root, 'examples/simple-report.json'), 'utf8'));
  imageReport.report.header.elements.push({id:'image',type:'image',x:4.5,y:0.05,width:0.5,height:0.25,src:'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC',style:{color:'#000000'}});
  const imageJson=path.join(directory,'image.json');
  await writeFile(imageJson, JSON.stringify(imageReport));
  const formulaReport = JSON.parse(await readFile(path.join(here, 'fixtures', 'number-report.json'), 'utf8'));
  formulaReport.report.formulas = [{name:'Double',expression:'amount * 2'}];
  formulaReport.report.details[0].band.elements = [{id:'value',type:'text',x:0,y:0,width:3,height:0.2,text:'[Double]',style:{fontsize:10,color:'#17213a',format:'number'}}];
  formulaReport.report.details[0].groupFooter = {id:'total',height:0.3,elements:[{id:'t',type:'text',x:0,y:0,width:3,height:0.2,text:'Total [SUM(amount)]',style:{fontsize:10,color:'#17213a',format:'number'}}]};
  const formulaJson = path.join(directory,'formula.json'), formulaPdf = path.join(directory,'formula.pdf');
  await writeFile(formulaJson, JSON.stringify(formulaReport));
  execFileSync('cargo',['run','--quiet','--locked','--example','number_fixture','--',formulaPdf,formulaJson],{cwd:root,env:process.env,stdio:'pipe'});
  reports = {
    '/unicode.pdf': makeReport('unicode.pdf','Implementation consulting',unicodeJson),
    '/chart.pdf': makeReport('chart.pdf','Implementation consulting',chartJson),
    '/group.pdf': groupPdf,
    '/image.pdf': makeReport('image.pdf', 'Implementation consulting', imageJson),
    '/formula.pdf': formulaPdf,
    '/multipage.pdf': multiPdf,
    '/awb.pdf': awbPath,
    '/awb-changed.pdf': awbChangedPath,
    '/awb-reference.pdf': path.join(root, 'examples/awb/awb-reference.pdf'),
    '/footer.pdf': makeReport('footer.pdf', 'Implementation consulting', path.join(here, 'fixtures', 'footer-report.json')),
    '/footer-reference.pdf': path.join(here, 'fixtures', 'designer-footer-reference.pdf'),
    '/numbers.pdf': numberPath,
    '/numbers-reference.pdf': path.join(here, 'fixtures', 'designer-number-reference.pdf'),
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

test('formatted values match the designer PDF', async ({page}) => {
  const actual = await inspectPDF(page, '/numbers.pdf');
  const reference = await inspectPDF(page, '/numbers-reference.pdf');
  expect(actual.items.length).toBe(reference.items.length);
  for (const expected of reference.items) {
    const item = actual.items.find(candidate => candidate.text === expected.text);
    expect(item, `missing ${expected.text}`).toBeDefined();
    expect(item.x, expected.text).toBeCloseTo(expected.x, 2);
    expect(item.y, expected.text).toBeCloseTo(expected.y, 2);
    expect(item.width, expected.text).toBeCloseTo(expected.width, 2);
  }
  for (const value of ['1,500.25', '$1,500.25', '12.57%', '1/2/2026', '-2.68', '-$2.68', '-0.75%', '12/31/2025']) {
    expect(actual.items.some(item => item.text === value), `missing ${value}`).toBe(true);
  }
});

test('page and report footer positions match the designer', async ({page}) => {
  const actual = await inspectPDF(page, '/footer.pdf');
  const reference = await inspectPDF(page, '/footer-reference.pdf');
  for (const text of ['Page 1 of 1', 'END OF REPORT']) {
    const item=actual.items.find(candidate=>candidate.text===text);
    const expected=reference.items.find(candidate=>candidate.text===text);
    expect(item, `missing ${text}`).toBeDefined();expect(expected).toBeDefined();
    expect(item.x).toBeCloseTo(expected.x, 2);expect(item.y).toBeCloseTo(expected.y, 2);
  }
});

test('air waybill text and geometry match the designer reference', async ({page}) => {
  const actual = await inspectPDF(page, '/awb.pdf');
  const reference = await inspectPDF(page, '/awb-reference.pdf');
  expect(actual.pageCount).toBe(1);
  expect(actual.pageCount).toBe(reference.pageCount);
  expect(actual.width).toBeCloseTo(reference.width, 3);
  expect(actual.height).toBeCloseTo(reference.height, 3);
  expect(actual.items.length).toBe(reference.items.length);
  for (let index=0; index<reference.items.length; index++) {
    const got=actual.items[index], expected=reference.items[index];
    expect(got.text, `text ${index}`).toBe(expected.text);
    expect(got.x, expected.text).toBeCloseTo(expected.x, 2);
    expect(got.y, expected.text).toBeCloseTo(expected.y, 2);
    expect(got.width, expected.text).toBeCloseTo(expected.width, 2);
  }
  for (const text of ['NORTHSTAR EXPORTS LLC','DEMO IMPORTS SA','ELECTRONIC EQUIPMENT','MACHINE PARTS','PRINTED MATERIAL']) {
    expect(actual.items.some(item=>item.text.includes(text)), `missing ${text}`).toBe(true);
  }
});

test('air waybill uses the supplied shipper instead of the JSON sample value', async ({page}) => {
  const actual = await inspectPDF(page, '/awb-changed.pdf');
  expect(actual.items.some(item=>item.text.includes('ALTERNATIVE SHIPPER LLC'))).toBe(true);
  expect(actual.items.some(item=>item.text.includes('NORTHSTAR EXPORTS LLC'))).toBe(false);
});

test('air waybill raster stays close to the designer reference', async ({page}) => {
  await page.goto(baseURL);
  const diff=await page.evaluate(async () => {
    const pdfjs=await import('/pdf.mjs');pdfjs.GlobalWorkerOptions.workerSrc='/pdf.worker.mjs';
    const pixels=[];for(const url of ['/awb.pdf','/awb-reference.pdf']){
      const task=pdfjs.getDocument({url,standardFontDataUrl:`${location.origin}/fonts/`,useSystemFonts:false});
      try {const doc=await task.promise;const p=await doc.getPage(1);const viewport=p.getViewport({scale:1});const canvas=document.createElement('canvas');canvas.width=Math.ceil(viewport.width);canvas.height=Math.ceil(viewport.height);const context=canvas.getContext('2d');await p.render({canvas,canvasContext:context,viewport}).promise;pixels.push(context.getImageData(0,0,canvas.width,canvas.height).data);}finally{await task.destroy();}
    }
    let total=0,max=0,significant=0;for(let i=0;i<pixels[0].length;i+=4){let high=false;for(let j=0;j<3;j++){const delta=Math.abs(pixels[0][i+j]-pixels[1][i+j]);total+=delta;max=Math.max(max,delta);high ||= delta>20;}if(high)significant++;}return {pixels:pixels[0].length/4,total,max,significant};
  });
  // Allow raster rounding, while requiring the same overall lines and layout.
  expect(diff.significant / diff.pixels, JSON.stringify(diff)).toBeLessThan(0.006);
  expect(diff.total / (diff.pixels * 3), JSON.stringify(diff)).toBeLessThan(0.5);
  expect(diff.max, JSON.stringify(diff)).toBeLessThanOrEqual(96);
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


test('multi-page PDF preserves every record and page counters', async ({page}) => {
  await page.goto(baseURL);
  const pages = await page.evaluate(async () => {
    const pdfjs = await import('/pdf.mjs');
    pdfjs.GlobalWorkerOptions.workerSrc = '/pdf.worker.mjs';
    const task = pdfjs.getDocument({url:'/multipage.pdf',standardFontDataUrl:`${location.origin}/fonts/`,useSystemFonts:false});
    try {
      const pdf = await task.promise, pages=[];
      for (let n=1; n<=pdf.numPages; n++) {
        const content = await (await pdf.getPage(n)).getTextContent();
        pages.push(content.items.map(item => item.str).filter(Boolean));
      }
      return pages;
    } finally { await task.destroy(); }
  });
  expect(pages.length).toBeGreaterThanOrEqual(3);
  for (const [index, items] of pages.entries()) {
    expect(items).toContain('HEADER');
    expect(items).toContain(`Page ${index + 1} of ${pages.length}`);
    expect(items.includes('REPORT END')).toBe(index === pages.length - 1);
  }
  const all = pages.flat();
  for (let n=0; n<100; n++) expect(all.filter(s=>s===`Item${String(n).padStart(3,'0')}`)).toHaveLength(1);
});


test('PDF.js extracts calculated formulas and the aggregate', async ({page}) => {
  const report = await inspectPDF(page, '/formula.pdf');
  const texts = report.items.map(item=>item.text);
  expect(texts).toContain('3,000.5');
  expect(texts).toContain('-5.35');
  expect(texts).toContain('Total 1,497.58');
});


test('embedded PNG appears with its expected color and geometry', async ({page}) => {
  await page.goto(baseURL);
  const samples = await page.evaluate(async () => {
    const pdfjs=await import('/pdf.mjs'); pdfjs.GlobalWorkerOptions.workerSrc='/pdf.worker.mjs';
    const task=pdfjs.getDocument({url:'/image.pdf',standardFontDataUrl:`${location.origin}/fonts/`,useSystemFonts:false});
    try {
      const pdf=await task.promise, first=await pdf.getPage(1), viewport=first.getViewport({scale:1});
      const canvas=document.querySelector('#page');canvas.width=viewport.width;canvas.height=viewport.height;
      const context=canvas.getContext('2d');await first.render({canvas,canvasContext:context,viewport}).promise;
      return {inside:[...context.getImageData(370,46,1,1).data],outside:[...context.getImageData(400,46,1,1).data]};
    } finally {await task.destroy()}
  });
  expect(samples.inside.slice(0,3)).toEqual([255,0,0]);
  expect(samples.outside.slice(0,3)).toEqual([255,255,255]);
});


test('group headers and totals survive pagination', async ({page}) => {
  await page.goto(baseURL);
  const pages = await page.evaluate(async()=>{
    const pdfjs=await import('/pdf.mjs');pdfjs.GlobalWorkerOptions.workerSrc='/pdf.worker.mjs';
    const task=pdfjs.getDocument({url:'/group.pdf',standardFontDataUrl:`${location.origin}/fonts/`,useSystemFonts:false});
    try {const pdf=await task.promise, pages=[];for(let n=1;n<=pdf.numPages;n++){const content=await(await pdf.getPage(n)).getTextContent();pages.push(content.items.map(x=>x.str).filter(Boolean));}return pages;}finally{await task.destroy()}
  });
  expect(pages.length).toBeGreaterThanOrEqual(3);
  for(const items of pages) expect(items.some(x=>x==='Group North'||x==='Group South')).toBeTruthy();
  const all=pages.flat();expect(all.filter(x=>x==='Subtotal 50')).toHaveLength(2);
  for(let n=0;n<100;n++) expect(all.filter(x=>x===`Item${String(n).padStart(3,'0')}`)).toHaveLength(1);
  expect(all.indexOf('Group North')).toBeLessThan(all.indexOf('Group South'));
});


test('bound chart displays supplied categories and their assigned colors', async ({page}) => {
  await page.goto(baseURL);
  const result=await page.evaluate(async()=>{
    const pdfjs=await import('/pdf.mjs');pdfjs.GlobalWorkerOptions.workerSrc='/pdf.worker.mjs';
    const task=pdfjs.getDocument({url:'/chart.pdf',standardFontDataUrl:`${location.origin}/fonts/`,useSystemFonts:false});
    try {const pdf=await task.promise,first=await pdf.getPage(1),viewport=first.getViewport({scale:1});const canvas=document.querySelector('#page');canvas.width=viewport.width;canvas.height=viewport.height;const ctx=canvas.getContext('2d');await first.render({canvas,canvasContext:ctx,viewport}).promise;
      const text=(await first.getTextContent()).items.map(item=>item.str).filter(Boolean);const pixels=ctx.getImageData(36,72,288,144).data;let red=0,blue=0;
      for(let i=0;i<pixels.length;i+=4){if(pixels[i]>235&&pixels[i+1]<30&&pixels[i+2]<30)red++;if(pixels[i]<70&&pixels[i+1]>70&&pixels[i+1]<130&&pixels[i+2]>200)blue++;}return {text,red,blue};
    }finally{await task.destroy()}
  });
  expect(result.text).toContain('Sales chart');expect(result.text).toContain('Implementation consulting');expect(result.text).toContain('Report development');
  expect(result.red).toBeGreaterThan(50);expect(result.blue).toBeGreaterThan(50);
});


test('embedded Unicode font renders and extracts Japanese and accented text', async ({page}) => {
  const result=await inspectPDF(page,'/unicode.pdf');
  const item=result.items.find(x=>x.text==='México Montréal 東京');
  expect(item).toBeDefined();
  expect(item.ink).toBeGreaterThan(30);
  expect(item.width).toBeGreaterThan(50);
  const japaneseInk=await page.evaluate(({x,y,width,height})=>{
    const canvas=document.querySelector('#page'),context=canvas.getContext('2d');
    const left=Math.max(0,Math.floor(x+width-31)),top=Math.max(0,Math.floor(height-y-20));
    const pixels=context.getImageData(left,top,30,22).data;let ink=0;
    for(let i=0;i<pixels.length;i+=4) if(pixels[i+3]&&Math.min(pixels[i],pixels[i+1],pixels[i+2])<180) ink++;
    return ink;
  },{x:item.x,y:item.y,width:item.width,height:result.height});
  expect(japaneseInk).toBeGreaterThan(10);
});
