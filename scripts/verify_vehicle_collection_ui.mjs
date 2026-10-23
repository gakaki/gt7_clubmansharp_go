#!/usr/bin/env node
/** Verify the five-tab car detail page and mobile navigation over CDP. */

import fs from 'node:fs/promises'
import path from 'node:path'

function arg(name, fallback) {
  const index = process.argv.indexOf(name)
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback
}

const base = arg('--base', 'http://127.0.0.1:4174/')
const cdpRoot = arg('--cdp', 'http://127.0.0.1:9333')
const outRoot = path.resolve(arg('--out-dir', '../site'))
const outDir = path.join(
  outRoot,
  `run-${new Date().toISOString().replace(/[-:.TZ]/g, '')}`,
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

async function evaluate(expression) {
  const result = await cdp.call('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  })
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text)
  return result.result.value
}

async function navigate(url) {
  await cdp.call('Page.navigate', { url })
  await sleep(2800)
}

async function screenshot(name) {
  const result = await cdp.call('Page.captureScreenshot', {
    format: 'png',
    fromSurface: true,
    captureBeyondViewport: false,
  })
  await fs.writeFile(path.join(outDir, name), Buffer.from(result.data, 'base64'))
}

await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 1440,
  height: 1200,
  deviceScaleFactor: 1,
  mobile: false,
})
await navigate(`${base}?lang=zh&l1=zh`)
const filterExpected = await evaluate(`(async () => {
  const data = await (await fetch('/data/cars.json')).json()
  return {
    all: data.cars.length,
    brandCentral: data.cars.filter((car) => car.texts?.brandCentral).length,
    usedCar: data.cars.filter((car) => car.texts?.usedCar).length,
    vehicleCollection: data.cars.filter((car) => car.texts?.vehicleCollection).length,
    cafe: data.cars.filter((car) => car.conversation?.length || car.dialogues?.length).length,
  }
})()`)
const filterObserved = {}
for (const [index, id] of ['all', 'brandCentral', 'usedCar', 'vehicleCollection', 'cafe'].entries()) {
  await evaluate(`document.querySelectorAll('.seg button')[${index}]?.click()`)
  await sleep(250)
  filterObserved[id] = await evaluate(`document.querySelectorAll('.card').length`)
}
const filterLabels = await evaluate(`[...document.querySelectorAll('.seg button')].map((node) => node.textContent.trim())`)
await screenshot('main-source-filters.png')

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x0000D1BB&tab=vehicleCollection`)
const desktop = await evaluate(`(() => {
  const tabs = [...document.querySelectorAll('.car-tabs button')]
  const panel = document.querySelector('.text-panel')
  return {
    title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
    tabs: tabs.map((node) => node.textContent.trim()),
    panel: panel?.innerText || '',
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('vehicle-collection-desktop.png')

async function pressDigit(digit) {
  const code = 48 + Number(digit)
  await cdp.call('Input.dispatchKeyEvent', {
    type: 'keyDown',
    key: digit,
    code: `Digit${digit}`,
    windowsVirtualKeyCode: code,
    nativeVirtualKeyCode: code,
  })
  await cdp.call('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: digit,
    code: `Digit${digit}`,
    windowsVirtualKeyCode: code,
    nativeVirtualKeyCode: code,
  })
  await sleep(250)
}

await pressDigit('2')
const brand = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
await pressDigit('3')
const collection = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
const keyboard = { brand, collection }

await navigate(`${base}?lang=en&l1=us&l2=zh&car=0x0000D1BB&tab=vehicleCollection`)
const english = await evaluate(`(() => ({
  title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
}))()`)

await navigate(`${base}?lang=en&l1=us&l2=zh&car=0x00009D5B&tab=vehicleCollection`)
const missingCollection = await evaluate(`(() => ({
  title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
  muted: [...document.querySelectorAll('.car-tabs button')]
    .find((node) => node.textContent.includes('Car Collection'))
    ?.classList.contains('muted') || false,
}))()`)

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00008200&tab=vehicleCollection`)
const peugeot = await evaluate(`(() => ({
  title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
}))()`)
await screenshot('vehicle-collection-peugeot.png')

await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 390,
  height: 844,
  deviceScaleFactor: 2,
  mobile: true,
})
await navigate(`${base}?lang=zh&l1=zh`)
const mobileFilters = await evaluate(`(() => {
  const bar = document.querySelector('.seg')
  return {
    buttons: bar?.querySelectorAll('button').length || 0,
    clientWidth: bar?.clientWidth || 0,
    scrollWidth: bar?.scrollWidth || 0,
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('main-source-filters-mobile.png')
await navigate(`${base}?lang=zh&l1=zh&car=0x0000D1BB&tab=vehicleCollection`)
const mobileBefore = await evaluate(`(() => ({
  title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
  tabs: [...document.querySelectorAll('.car-tabs button')].map((node) => node.textContent.trim()),
  panel: document.querySelector('.text-panel')?.innerText || '',
  fixedNav: getComputedStyle(document.querySelector('.mobile-car-nav')).position,
  navBottom: getComputedStyle(document.querySelector('.mobile-car-nav')).bottom,
  bodyWidth: document.documentElement.scrollWidth,
  viewportWidth: document.documentElement.clientWidth,
}))()`)
await screenshot('vehicle-collection-mobile.png')
await evaluate(`document.querySelector('.mobile-car-step:last-child')?.click()`)
await sleep(700)
const mobileAfter = await evaluate(`(() => ({
  title: document.querySelector('.dialog h3')?.textContent?.trim() || '',
  tab: document.querySelector('.car-tabs button.on')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
}))()`)

const checks = {
  fiveTabs: desktop.tabs.length === 5,
  splitLabels: desktop.tabs.some((text) => text.includes('品牌购物')) &&
    desktop.tabs.some((text) => text.includes('车辆收藏')) &&
    !desktop.tabs.some((text) => text.includes('品牌购物 / 车库')),
  xiaomiBilingual: desktop.title.includes('SU7') &&
    desktop.panel.includes('智能手机巨头') &&
    desktop.panel.includes('smartphone giant'),
  collectionPayload: desktop.panel.includes('Catch') &&
    desktop.panel.includes('Long') &&
    desktop.panel.includes('0x0000CF1E') &&
    desktop.panel.includes('0x0000CF1F'),
  englishHeadings: english.panel.includes('Catch copy') &&
    english.panel.includes('Full description') &&
    !english.panel.includes('一句话简介') &&
    !english.panel.includes('车辆收藏正文'),
  missingCollectionState: missingCollection.title.includes('2002 Turbo') &&
    missingCollection.muted &&
    missingCollection.panel.includes('No Car Collection entry is available'),
  keyboardTabs: keyboard.brand.includes('品牌购物') &&
    keyboard.collection.includes('车辆收藏'),
  peugeotCollection: peugeot.title.includes('205 Turbo 16 Evolution 2') &&
    peugeot.panel.includes('Group B racing world by storm') &&
    peugeot.panel.includes('B组别') &&
    peugeot.panel.includes('0x00007F7E') &&
    peugeot.panel.includes('0x00007F7F'),
  mobileNoOverflow: mobileBefore.bodyWidth <= mobileBefore.viewportWidth,
  mobileFixedNav: mobileBefore.fixedNav === 'fixed',
  mobileSwitchedCar: mobileBefore.title !== mobileAfter.title,
  mobileCollection: mobileBefore.tabs.length === 5 &&
    mobileBefore.panel.includes('智能手机巨头'),
  fiveSourceFilters: filterLabels.length === 5 &&
    filterLabels.some((label) => label.includes('品牌店')) &&
    filterLabels.some((label) => label.includes('二手车')) &&
    filterLabels.some((label) => label.includes('车辆收藏')) &&
    filterLabels.some((label) => label.includes('咖啡屋对白')),
  sourceFilterCounts: Object.entries(filterExpected)
    .every(([id, count]) => filterObserved[id] === count),
  mobileFilterStrip: mobileFilters.buttons === 5 &&
    mobileFilters.bodyWidth <= mobileFilters.viewportWidth &&
    mobileFilters.scrollWidth >= mobileFilters.clientWidth,
}
const report = {
  filterExpected,
  filterObserved,
  filterLabels,
  mobileFilters,
  desktop,
  english,
  missingCollection,
  keyboard,
  peugeot,
  mobileBefore,
  mobileAfter,
  checks,
}
await fs.writeFile(
  path.join(outDir, 'vehicle-collection-ui.json'),
  JSON.stringify(report, null, 2),
  'utf8',
)
console.log(JSON.stringify({ checks, title: desktop.title, mobileBefore: {
  title: mobileBefore.title,
  tabs: mobileBefore.tabs,
  fixedNav: mobileBefore.fixedNav,
  bodyWidth: mobileBefore.bodyWidth,
  viewportWidth: mobileBefore.viewportWidth,
}, mobileAfter }, null, 2))
ws.close()
if (Object.values(checks).some((value) => !value)) process.exitCode = 1
