#!/usr/bin/env node
/** Verify multilingual dialogue switching and persisted single/dual choices. */

import fs from 'node:fs/promises'
import path from 'node:path'

function arg(name, fallback) {
  const index = process.argv.indexOf(name)
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback
}

const base = arg('--base', 'http://127.0.0.1:4176/')
const cdpRoot = arg('--cdp', 'http://127.0.0.1:9333')
const outRoot = path.resolve(arg('--out-dir', '../reports'))
const outDir = path.join(
  outRoot,
  `languages-${new Date().toISOString().replace(/[-:.TZ]/g, '')}`,
)
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
await fs.mkdir(outDir, { recursive: true })

class Cdp {
  constructor(ws) {
    this.ws = ws
    this.id = 0
    this.pending = new Map()
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data)
      const request = this.pending.get(message.id)
      if (!request) return
      this.pending.delete(message.id)
      if (message.error) request.reject(new Error(message.error.message))
      else request.resolve(message.result)
    })
  }

  call(method, params = {}) {
    const id = ++this.id
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject })
      this.ws.send(JSON.stringify({ id, method, params }))
    })
  }
}

const targets = await (await fetch(`${cdpRoot}/json`)).json()
const target = targets.find((item) => item.type === 'page')
if (!target) throw new Error('No Chrome page target')
const ws = new WebSocket(target.webSocketDebuggerUrl)
await new Promise((resolve, reject) => {
  ws.addEventListener('open', resolve, { once: true })
  ws.addEventListener('error', reject, { once: true })
})
const cdp = new Cdp(ws)
await cdp.call('Page.enable')
await cdp.call('Runtime.enable')
await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 1440,
  height: 1200,
  deviceScaleFactor: 1,
  mobile: false,
})

async function evaluate(expression) {
  const result = await cdp.call('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  })
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text)
  return result.result.value
}

async function navigate(url, wait = 3200) {
  await cdp.call('Page.navigate', { url })
  await sleep(wait)
}

async function screenshot(name) {
  const result = await cdp.call('Page.captureScreenshot', {
    format: 'png',
    fromSurface: true,
    captureBeyondViewport: false,
  })
  await fs.writeFile(path.join(outDir, name), Buffer.from(result.data, 'base64'))
}

const carId = '0x00004283'
await navigate(`${base}?lang=both&l1=jp&l2=de&car=${carId}&tab=cafe`)
const expected = await evaluate(`(async () => {
  const car = (await (await fetch('/data/cars.json')).json()).cars
    .find((item) => item.id === '${carId}')
  const jp = await (await fetch('/data/dialogue_translations/jp.json')).json()
  const de = await (await fetch('/data/dialogue_translations/de.json')).json()
  const turn = car.conversation.find((item) => jp[item.id] || de[item.id])
  return {
    id: turn.id,
    jp: (jp[turn.id] || '').slice(0, 35),
    de: (de[turn.id] || '').slice(0, 35),
  }
})()`)
await sleep(1200)
const initial = await evaluate(`(() => ({
  selectors: document.querySelectorAll('.detail-lang select').length,
  primary: document.querySelector('.detail-lang select')?.value || '',
  secondary: document.querySelectorAll('.detail-lang select')[1]?.value || '',
  bilingual: document.querySelector('.detail-lang .bilingual-toggle')?.classList.contains('on') || false,
  panel: document.querySelector('.cafe-dialogues')?.innerText || '',
  languageLabels: [...document.querySelectorAll('.translation-language')].slice(0, 4).map((node) => node.textContent.trim()),
  bodyWidth: document.documentElement.scrollWidth,
  viewportWidth: document.documentElement.clientWidth,
}))()`)
await screenshot('multilingual-jp-de.png')

await evaluate(`(() => {
  const pickers = document.querySelector('.detail-lang')
  const selects = pickers.querySelectorAll('select')
  selects[0].value = 'fr'
  selects[0].dispatchEvent(new Event('change', { bubbles: true }))
  selects[1].value = 'it'
  selects[1].dispatchEvent(new Event('change', { bubbles: true }))
})()`)
await sleep(1400)
const stored = await evaluate(`({
  primary: localStorage.getItem('gt7cafe.primaryLang'),
  secondary: localStorage.getItem('gt7cafe.secondaryLang'),
  bilingual: localStorage.getItem('gt7cafe.bilingual'),
})`)

await navigate(`${base}?car=${carId}&tab=cafe`)
const restored = await evaluate(`(() => ({
  primary: document.querySelector('.detail-lang select')?.value || '',
  secondary: document.querySelectorAll('.detail-lang select')[1]?.value || '',
  bilingual: document.querySelector('.detail-lang .bilingual-toggle')?.classList.contains('on') || false,
  panel: document.querySelector('.cafe-dialogues')?.innerText || '',
  languageLabels: [...document.querySelectorAll('.translation-language')].slice(0, 4).map((node) => node.textContent.trim()),
}))()`)

await evaluate(`document.querySelectorAll('.detail-lang .bilingual-toggle')[0]?.click()`)
await sleep(500)
const singleMode = await evaluate(`(() => ({
  selectors: document.querySelectorAll('.detail-lang select').length,
  primary: document.querySelector('.detail-lang select')?.value || '',
  bilingual: document.querySelector('.detail-lang .bilingual-toggle')?.classList.contains('on') || false,
}))()`)
await navigate(`${base}?car=${carId}&tab=cafe`)
const singleRestored = await evaluate(`(() => ({
  selectors: document.querySelectorAll('.detail-lang select').length,
  primary: document.querySelector('.detail-lang select')?.value || '',
  bilingual: document.querySelector('.detail-lang .bilingual-toggle')?.classList.contains('on') || false,
}))()`)
await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 390,
  height: 844,
  deviceScaleFactor: 2,
  mobile: true,
})
await navigate(`${base}?lang=both&l1=jp&l2=de&car=${carId}&tab=cafe`)
const mobile = await evaluate(`(() => {
  const picker = document.querySelector('.detail-lang')
  return {
    selectors: picker?.querySelectorAll('select').length || 0,
    pickerWidth: picker?.getBoundingClientRect().width || 0,
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
    panel: document.querySelector('.cafe-dialogues')?.innerText || '',
  }
})()`)
await screenshot('multilingual-mobile-jp-de.png')

const checks = {
  nineteenOptions: initial.selectors === 2 &&
    (await evaluate(`document.querySelectorAll('.detail-lang select')[0].options.length`)) === 19,
  japaneseAndGerman: initial.primary === 'jp' &&
    initial.secondary === 'de' &&
    initial.bilingual &&
    initial.panel.includes(expected.jp) &&
    initial.panel.includes(expected.de),
  bilingualLabels: initial.languageLabels.includes('日本語') &&
    initial.languageLabels.includes('Deutsch'),
  persistedSelection: stored.primary === 'fr' &&
    stored.secondary === 'it' &&
    stored.bilingual === '1',
  restoredSelection: restored.primary === 'fr' &&
    restored.secondary === 'it' &&
    restored.bilingual,
  singleModeToggle: singleMode.selectors === 1 &&
    singleMode.primary === 'fr' &&
    !singleMode.bilingual,
  singleModePersisted: singleRestored.selectors === 1 &&
    singleRestored.primary === 'fr' &&
    !singleRestored.bilingual,
  noHorizontalOverflow: initial.bodyWidth <= initial.viewportWidth,
  mobileLanguagePicker: mobile.selectors === 2 &&
    mobile.bodyWidth <= mobile.viewportWidth &&
    mobile.pickerWidth <= mobile.viewportWidth,
}

const report = {
  base,
  expected,
  initial,
  stored,
  restored,
  singleMode,
  singleRestored,
  mobile,
  checks,
}
await fs.writeFile(
  path.join(outDir, 'languages_ui_verified.json'),
  JSON.stringify(report, null, 2),
  'utf8',
)
console.log(JSON.stringify({ checks, initial: {
  primary: initial.primary,
  secondary: initial.secondary,
  languageLabels: initial.languageLabels,
}, restored: {
  primary: restored.primary,
  secondary: restored.secondary,
  bilingual: restored.bilingual,
} }, null, 2))
ws.close()
if (Object.values(checks).some((value) => !value)) process.exitCode = 1
