#!/usr/bin/env node
/** Verify the seven-tab car detail page, official gallery and mobile navigation over CDP. */

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
    this.events = []
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data)
      if (message.method === 'Runtime.exceptionThrown') {
        this.events.push(message.params.exceptionDetails)
      }
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
await cdp.call('Network.enable')
await cdp.call('Network.setCacheDisabled', { cacheDisabled: true })
await cdp.call('Network.clearBrowserCache')
await cdp.call('Network.enable')
await cdp.call('Network.setCacheDisabled', { cacheDisabled: true })

async function evaluate(expression) {
  const result = await cdp.call('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  })
  if (result.exceptionDetails) {
    throw new Error(
      result.exceptionDetails.exception?.description
        || result.exceptionDetails.text,
    )
  }
  return result.result.value
}

async function navigate(url) {
  await cdp.call('Page.navigate', { url })
  const wantsCarDetail = new URL(url).searchParams.has('car')
  await sleep(700)
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const ready = await evaluate(`(() => ({
      complete: document.readyState === 'complete',
      loaded: ${wantsCarDetail}
        ? Boolean(document.querySelector('.dialog'))
        : Boolean(document.querySelector('.catalog-hero')),
      booting: Boolean(document.querySelector('.boot-screen')),
    }))()`)
    if (ready.complete && ready.loaded && !ready.booting) return
    await sleep(250)
  }
  await sleep(500)
}

async function screenshot(name) {
  const result = await cdp.call('Page.captureScreenshot', {
    format: 'png',
    fromSurface: true,
    captureBeyondViewport: false,
  })
  await fs.writeFile(path.join(outDir, name), Buffer.from(result.data, 'base64'))
}

async function clickSelector(selector) {
  const point = await evaluate(`(async () => {
    const node = document.querySelector(${JSON.stringify(selector)})
    if (!node) return null
    node.scrollIntoView({ block: 'center', inline: 'center' })
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
    const rect = node.getBoundingClientRect()
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
  })()`)
  if (!point) throw new Error(`Missing click target: ${selector}`)
  await cdp.call('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x: point.x,
    y: point.y,
    button: 'left',
    clickCount: 1,
  })
  await cdp.call('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x: point.x,
    y: point.y,
    button: 'left',
    clickCount: 1,
  })
}

async function dragSelector(selector, deltaX, deltaY) {
  const point = await evaluate(`(async () => {
    const node = document.querySelector(${JSON.stringify(selector)})
    if (!node) return null
    node.scrollIntoView({ block: 'center', inline: 'center' })
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
    const rect = node.getBoundingClientRect()
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
  })()`)
  if (!point) throw new Error(`Missing drag target: ${selector}`)
  await cdp.call('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x: point.x,
    y: point.y,
    button: 'left',
    buttons: 1,
    clickCount: 1,
  })
  await cdp.call('Input.dispatchMouseEvent', {
    type: 'mouseMoved',
    x: point.x + deltaX,
    y: point.y + deltaY,
    button: 'left',
    buttons: 1,
  })
  await cdp.call('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x: point.x + deltaX,
    y: point.y + deltaY,
    button: 'left',
    buttons: 0,
    clickCount: 1,
  })
}

await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 1440,
  height: 1200,
  deviceScaleFactor: 1,
  mobile: false,
})
await navigate(`${base}?lang=zh&l1=zh`)
await evaluate(`(() => {
  for (const key of [
    'gt7cafe.official.autoplay',
    'gt7cafe.official.speed',
    'gt7cafe.official.textSize',
    'gt7cafe.official.textX',
    'gt7cafe.official.textY',
    'gt7.carlist.slideshowSpeed',
    'gt7.carlist.lastSlideshowPosition',
    'gt7.carlist.fullscreenInfoTextScale',
    'gt7.carlist.fullscreenInfoOffset',
  ]) localStorage.removeItem(key)
})()`)
await evaluate(`document.querySelector('.catalog-slideshow-launch')?.click()`)
await sleep(700)
const scalePreferenceTest = await evaluate(`(async () => {
  const button = [...document.querySelectorAll('.mirror-info-tools button')]
    .find((node) => node.textContent?.trim() === 'A+')
  if (!button) return null
  button.click()
  await new Promise((resolve) => setTimeout(resolve, 100))
  button.click()
  await new Promise((resolve) => setTimeout(resolve, 100))
  return {
    scale: localStorage.getItem('gt7.carlist.fullscreenInfoTextScale'),
    font: document.querySelector('.mirror-info-copy')?.getAttribute('style') || '',
  }
})()`)
await evaluate(`document.querySelector('.mirror-slideshow-close')?.click()`)
await sleep(300)
await navigate(`${base}?lang=zh&l1=zh`)
const filterExpected = await evaluate(`(async () => {
  const [data, mirror] = await Promise.all([
    fetch('/data/cars.json').then((response) => response.json()),
    fetch('/data/mirror/cars.cn.json').then((response) => response.json()),
  ])
  const officialIds = new Set(mirror.cars.map((car) => car.id))
  const localOfficialIds = new Set(
    data.cars
      .map((car) => car.officialId || car.officialDetails?.officialId)
      .filter(Boolean),
  )
  const normalize = (value) => String(value || '').toLowerCase().replace(/[^a-z0-9]+/g, '')
  const localNames = data.cars.flatMap((car) => [normalize(car.nameEn), normalize(car.nameZh)])
  const officialOnly = mirror.cars.filter((record) => {
    if (localOfficialIds.has(record.id)) return false
    const fullName = normalize(record.name_long)
    const modelName = normalize(record.model)
    return !localNames.some((name) =>
      name === fullName || name === modelName || (modelName.length > 4 && name.endsWith(modelName))
    )
  }).length
  return {
    all: data.cars.length + officialOnly,
    brandCentral: data.cars.filter((car) => car.texts?.brandCentral).length,
    usedCar: data.cars.filter((car) => car.texts?.usedCar).length,
    vehicleCollection: data.cars.filter((car) => car.texts?.vehicleCollection).length,
    cafe: data.cars.filter((car) => car.conversation?.length || car.dialogues?.length).length,
    hidden: data.cars.filter((car) => !officialIds.has(car.officialId || car.officialDetails?.officialId)).length,
    officialPage: mirror.car_count,
    officialOnly,
  }
})()`)
const garagePayload = await evaluate(`(async () => {
  const response = await fetch('/data/garage_details.json')
  const data = await response.json()
  const peugeot = data.cars.find((car) => car.id === '0x00008200')
  return {
    ok: response.ok,
    counts: data.counts,
    peugeotTypes: peugeot?.garage?.vehicleTypes?.map((type) => type.nameEn) || [],
    peugeotCountry: peugeot?.garage?.country?.code || '',
  }
})()`)
const catalogCounts = await evaluate(`(() => ({
  hero: document.querySelector('.catalog-hero p')?.innerText || '',
  summary: document.querySelector('.summary')?.innerText || '',
  cacheStatus: document.querySelector('.image-cache-status')?.textContent?.trim() || '',
  serviceWorker: Boolean(navigator.serviceWorker?.controller),
  brandHeadings: [...document.querySelectorAll('.brandgroup h2')]
    .map((node) => node.textContent.replace(/\\s+/g, ' ').trim()),
}))()`)
const filterObserved = {}
for (const [index, id] of ['all', 'brandCentral', 'usedCar', 'vehicleCollection', 'cafe', 'hidden'].entries()) {
  await evaluate(`document.querySelectorAll('.seg button')[${index}]?.click()`)
  await sleep(250)
  filterObserved[id] = await evaluate(`document.querySelectorAll('.card').length`)
}
const filterLabels = await evaluate(`[...document.querySelectorAll('.seg button')].map((node) => node.textContent.trim())`)
await screenshot('main-source-filters.png')

await evaluate(`document.querySelector('.seg button:first-child')?.click()`)
await sleep(250)
const groupLabels = await evaluate(
  `[...document.querySelectorAll('.group-tabs button')].map((node) => node.textContent.trim())`,
)
const sortModes = []
for (let index = 0; index < 5; index += 1) {
  await evaluate(`document.querySelectorAll('.group-tabs button')[${index}]?.click()`)
  await sleep(250)
  sortModes.push(
    await evaluate(`(() => ({
      label: document.querySelectorAll('.group-tabs button')[${index}]?.textContent?.trim() || '',
      heading: document.querySelector('.brandgroup h2')?.innerText?.trim() || '',
      ids: [...document.querySelectorAll('[data-car-id]')]
        .slice(0, 6)
        .map((node) => node.getAttribute('data-car-id')),
      groups: document.querySelectorAll('.brandgroup').length,
    }))()`),
  )
}
await evaluate(`document.querySelector('.group-tabs button:first-child')?.click()`)
await sleep(250)
await evaluate(`(() => {
  const input = document.querySelector('.search')
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
  setter.call(input, 'Xiaomi Vision Gran Turismo')
  input.dispatchEvent(new Event('input', { bubbles: true }))
})()`)
await sleep(500)
const officialOnlySearch = await evaluate(`(() => ({
  count: document.querySelectorAll('[data-car-id]').length,
  names: [...document.querySelectorAll('.card .name')].map((node) => node.textContent.trim()),
  ids: [...document.querySelectorAll('[data-car-id]')].map((node) => node.dataset.carId),
}))()`)
await evaluate(`(() => {
  const input = document.querySelector('.search')
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
  setter.call(input, '')
  input.dispatchEvent(new Event('input', { bubbles: true }))
})()`)
await sleep(300)

await navigate(`${base}?lang=zh&l1=zh&car=0x00006F59&tab=cafe`)
const cafeDesktop = await evaluate(`(() => {
  const grid = document.querySelector('.cafe-dialogues')
  const turns = [...document.querySelectorAll('.cafe-scene .dialogue')]
  const normalize = (value) => String(value || '')
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[\\p{P}\\p{S}\\s]+/gu, '')
  const seen = new Set()
  let duplicates = 0
  for (const turn of turns) {
    const speaker = turn.querySelector('.who')?.textContent || ''
    const text = [...turn.querySelectorAll('.dialogue-text')]
      .map((node) => node.textContent).join('')
    const key = speaker + '\\0' + normalize(text)
    if (seen.has(key)) duplicates += 1
    else seen.add(key)
  }
  return {
    scenes: document.querySelectorAll('.cafe-scene').length,
    columns: grid ? getComputedStyle(grid).gridTemplateColumns : '',
    turns: turns.length,
    duplicates,
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('cafe-multi-scene-desktop.png')

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00006F48&tab=official`)
const officialDesktop = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  tab: document.querySelector('.car-tabs button.on')?.textContent?.trim() || '',
  slides: document.querySelector('.official-image-count')?.textContent?.trim() || '',
  thumbnails: document.querySelectorAll('.official-thumbnails').length,
  inlineControls: Boolean(document.querySelector('.official-visual-pane .official-inline-controls')),
  visualWidth: Math.round(document.querySelector('.official-visual-pane')?.getBoundingClientRect().width || 0),
  copyWidth: Math.round(document.querySelector('.official-copy')?.getBoundingClientRect().width || 0),
  tabCount: document.querySelectorAll('.car-tabs button').length,
  image: (() => {
    const node = document.querySelector('.official-stage-media img')
    return node ? {
      src: node.getAttribute('src'),
      width: node.naturalWidth,
      height: node.naturalHeight,
    } : null
  })(),
  intro: document.querySelector('.official-copy')?.innerText || '',
}))()`)
if (!officialDesktop.tab) {
  console.log(JSON.stringify(cdp.events.slice(-5), null, 2))
}
await evaluate(`(() => {
  const ranges = document.querySelectorAll('.official-inline-controls input[type=range]')
  if (ranges.length < 2) {
    throw new Error('official controls missing; tabs=' + document.querySelectorAll('.car-tabs button').length + ', text=' + document.body.innerText.slice(0, 300) + ', html=' + document.body.innerHTML.slice(0, 300))
  }
  const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
  setValue.call(ranges[0], '7')
  ranges[0].dispatchEvent(new Event('input', { bubbles: true }))
  ranges[0].dispatchEvent(new Event('change', { bubbles: true }))
  setValue.call(ranges[1], '29')
  ranges[1].dispatchEvent(new Event('input', { bubbles: true }))
  ranges[1].dispatchEvent(new Event('change', { bubbles: true }))
})()`)
await evaluate(`document.querySelector('.official-stage-media img')?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))`)
await sleep(900)
await dragSelector('.mirror-info-tools button:first-of-type', 120, -48)
await sleep(120)
await evaluate(`document.querySelector('.mirror-slideshow-controls > div button:nth-child(10)')?.click()`)
await sleep(250)
const officialFullscreen = await evaluate(`(() => {
  const overlay = document.querySelector('.mirror-slideshow')
  const image = document.querySelector('.mirror-slideshow-image')
  const info = document.querySelector('.mirror-slideshow-info > div')
  const copy = document.querySelector('.mirror-info-copy')
  return {
    active: Boolean(overlay),
    imageSrc: image?.getAttribute('src') || '',
    imageWidth: image?.naturalWidth || 0,
    imageHeight: image?.naturalHeight || 0,
    title: document.querySelector('.mirror-slideshow-info h2')?.textContent?.trim() || '',
    intro: copy?.innerText || '',
    slide: document.querySelector('.mirror-image-counter')?.textContent?.trim() || '',
    infoTransform: info?.getAttribute('style') || '',
    infoFontSize: copy?.getAttribute('style') || '',
    speed: localStorage.getItem('gt7.carlist.slideshowSpeed'),
    textScale: localStorage.getItem('gt7.carlist.fullscreenInfoTextScale'),
    offset: localStorage.getItem('gt7.carlist.fullscreenInfoOffset'),
    speedPressed: document.querySelector('.mirror-slideshow-controls button[aria-pressed="true"]')
      ?.textContent?.trim() || '',
    bodyOverflow: document.body.style.overflow,
  }
})()`)
const fullscreenCarBefore = officialFullscreen.title
await evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', code: 'ArrowDown', bubbles: true }))`)
await sleep(250)
const fullscreenCarAfter = await evaluate(
  `document.querySelector('.mirror-slideshow-info h2')?.textContent?.trim() || ''`,
)
await screenshot('official-tab-fullscreen.png')
await clickSelector('.mirror-slideshow-close')
await sleep(500)
await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00006F48&tab=official`)
await evaluate(`document.querySelector('.official-inline-fullscreen')?.click()`)
await sleep(700)
const officialPersisted = await evaluate(`(() => {
  const ranges = document.querySelectorAll('.official-inline-controls input[type=range]')
  const caption = document.querySelector('.official-caption')
  const info = document.querySelector('.mirror-slideshow-info > div')
  const copy = document.querySelector('.mirror-info-copy')
  return {
    speed: ranges[0]?.value || '',
    textSize: ranges[1]?.value || '',
    captionStyle: caption?.getAttribute('style') || '',
    autoplay: localStorage.getItem('gt7cafe.official.autoplay'),
    storedSpeed: localStorage.getItem('gt7.carlist.slideshowSpeed'),
    storedScale: localStorage.getItem('gt7.carlist.fullscreenInfoTextScale'),
    storedOffset: localStorage.getItem('gt7.carlist.fullscreenInfoOffset'),
    infoTransform: info?.getAttribute('style') || '',
    infoFontSize: copy?.getAttribute('style') || '',
    overlayActive: Boolean(document.querySelector('.mirror-slideshow')),
  }
})()`)
await clickSelector('.mirror-slideshow-close')
await sleep(300)
await screenshot('official-tab-desktop.png')

await navigate(`${base}?lang=both&l1=zh&l2=us`)
const slideshowLauncher = await evaluate(`(() => {
  const button = document.querySelector('.catalog-slideshow-launch')
  return {
    exists: Boolean(button),
    label: button?.textContent?.replace(/\\s+/g, ' ').trim() || '',
    disabled: button?.disabled || false,
  }
})()`)
await clickSelector('.catalog-slideshow-launch')
await sleep(700)
await clickSelector('.mirror-slideshow-controls > button:nth-child(2)')
const catalogSlideshowFirst = await evaluate(`(() => ({
  active: Boolean(document.querySelector('.mirror-slideshow')),
  title: document.querySelector('.mirror-slideshow-info h2')?.textContent?.trim() || '',
  counter: document.querySelector('.mirror-image-counter')?.textContent?.trim() || '',
  image: document.querySelector('.mirror-slideshow-image')?.getAttribute('src') || '',
}))()`)
await clickSelector('.mirror-slideshow-controls > button:nth-child(3)')
await clickSelector('.mirror-slideshow-controls > button:nth-child(3)')
const catalogSlideshowAdvanced = await evaluate(`(() => ({
  title: document.querySelector('.mirror-slideshow-info h2')?.textContent?.trim() || '',
  counter: document.querySelector('.mirror-image-counter')?.textContent?.trim() || '',
}))()`)
await clickSelector('.mirror-slideshow-controls > div button:nth-child(1)')
await clickSelector('.mirror-slideshow-close')
await sleep(400)
await evaluate(`document.querySelector('.dialog .close')?.click()`)
await sleep(200)
await clickSelector('.catalog-slideshow-launch')
await sleep(600)
await clickSelector('.mirror-slideshow-controls > button:nth-child(2)')
const catalogSlideshowResumed = await evaluate(`(() => ({
  title: document.querySelector('.mirror-slideshow-info h2')?.textContent?.trim() || '',
  counter: document.querySelector('.mirror-image-counter')?.textContent?.trim() || '',
}))()`)
await clickSelector('.mirror-slideshow-close')
await sleep(250)

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x0000D1BB&tab=vehicleCollection`)
const desktop = await evaluate(`(() => {
  const tabs = [...document.querySelectorAll('.car-tabs button')]
  const panel = document.querySelector('.text-panel')
  return {
    title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
    tabs: tabs.map((node) => node.textContent.trim()),
    panel: panel?.innerText || '',
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('vehicle-collection-desktop.png')

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x0000D1BB&tab=garageSpecs`)
const garageDesktop = await evaluate(`(() => {
  const tabs = [...document.querySelectorAll('.car-tabs button')]
  const panel = document.querySelector('.garage-specs-panel')
  return {
    title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
    tabs: tabs.map((node) => node.textContent.trim()),
    active: document.querySelector('.car-tabs button.on')?.textContent?.trim() || '',
    panel: panel?.innerText || '',
    cards: panel?.querySelectorAll('.spec-card').length || 0,
    headerSpecs: document.querySelector('.head-specs')?.innerText || '',
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('garage-specs-desktop.png')

async function pressDigit(digit) {
  await evaluate('document.activeElement?.blur()')
  const code = 48 + Number(digit)
  await cdp.call('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
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

await pressDigit('3')
const brand = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
await pressDigit('4')
const collection = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
await pressDigit('5')
const garage = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
await pressDigit('1')
const officialTab = await evaluate(
  `document.querySelector('.car-tabs button.on')?.textContent?.trim() || ''`,
)
const keyboard = { officialTab, brand, collection, garage }

await navigate(`${base}?lang=en&l1=us&l2=zh&car=0x0000D1BB&tab=vehicleCollection`)
const english = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
  brandHeadings: [...document.querySelectorAll('.brandgroup h2')]
    .map((node) => node.textContent.replace(/\\s+/g, ' ').trim()),
}))()`)

await navigate(`${base}?lang=en&l1=us&l2=zh&car=0x0000D1BB&tab=garageSpecs`)
const garageEnglish = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.garage-specs-panel')?.innerText || '',
  tabs: [...document.querySelectorAll('.car-tabs button')].map((node) => node.textContent.trim()),
}))()`)

await navigate(`${base}?lang=en&l1=us&l2=zh&car=0x00009D5B&tab=vehicleCollection`)
const missingCollection = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
  muted: [...document.querySelectorAll('.car-tabs button')]
    .find((node) => node.textContent.includes('Car Collection'))
    ?.classList.contains('muted') || false,
}))()`)

await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00008200&tab=vehicleCollection`)
const peugeot = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
}))()`)

await navigate(`${base}?lang=jp&l1=jp&car=0x00008200&tab=vehicleCollection`)
const japaneseCollection = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.collection-main-copy')?.innerText || '',
}))()`)
await screenshot('vehicle-collection-peugeot.png')

await navigate(`${base}?lang=zh&l1=zh&car=0x00008200&tab=garageSpecs`)
const garagePeugeot = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.garage-specs-panel')?.innerText || '',
  cards: document.querySelectorAll('.spec-card').length,
}))()`)

await navigate(`${base}?lang=zh&l1=zh&car=0x00009D62&tab=garageSpecs`)
await sleep(1000)
const garageVariant = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.garage-specs-panel')?.innerText || '',
  cards: document.querySelectorAll('.spec-card').length,
}))()`)

await navigate(`${base}?lang=zh&l1=zh&car=0x00007600&tab=garageSpecs`)
const garageMissing = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  panel: document.querySelector('.garage-specs-panel')?.innerText || '',
  muted: [...document.querySelectorAll('.car-tabs button')]
    .find((node) => node.dataset.tab === 'garageSpecs')
    ?.classList.contains('muted') || false,
}))()`)

await cdp.call('Emulation.setDeviceMetricsOverride', {
  width: 764,
  height: 900,
  deviceScaleFactor: 1,
  mobile: false,
})
await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00006F48&tab=official`)
const mediumOfficial = await evaluate(`(() => {
  const title = document.querySelector('.dialog-title')
  const header = document.querySelector('.dialog header')
  const layout = document.querySelector('.official-layout')
  return {
    title: title?.textContent?.trim() || '',
    titleWidth: title?.getBoundingClientRect().width || 0,
    titleHeight: title?.getBoundingClientRect().height || 0,
    headerDirection: header ? getComputedStyle(header).flexDirection : '',
    columns: layout ? getComputedStyle(layout).gridTemplateColumns.split(' ').length : 0,
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }
})()`)
await screenshot('official-tab-medium.png')

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
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  tabs: [...document.querySelectorAll('.car-tabs button')].map((node) => node.textContent.trim()),
  panel: document.querySelector('.text-panel')?.innerText || '',
  fixedNav: getComputedStyle(document.querySelector('.mobile-car-nav')).position,
  navBottom: getComputedStyle(document.querySelector('.mobile-car-nav')).bottom,
  bodyWidth: document.documentElement.scrollWidth,
  viewportWidth: document.documentElement.clientWidth,
}))()`)
await screenshot('vehicle-collection-mobile.png')
await navigate(`${base}?lang=zh&l1=zh&car=0x00006F59&tab=cafe`)
const cafeMobile = await evaluate(`(() => {
  const grid = document.querySelector('.cafe-dialogues')
  return {
    scenes: document.querySelectorAll('.cafe-scene').length,
    columns: grid ? getComputedStyle(grid).gridTemplateColumns : '',
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
    fixedNav: getComputedStyle(document.querySelector('.mobile-car-nav')).position,
  }
})()`)
await screenshot('cafe-multi-scene-mobile.png')
await navigate(`${base}?lang=zh&l1=zh&car=0x00006F48&tab=official`)
const officialMobile = await evaluate(`(() => {
  const image = document.querySelector('.official-stage-media img')
  return {
    tabs: document.querySelectorAll('.car-tabs button').length,
    imageCount: document.querySelector('.official-image-count')?.textContent?.trim() || '',
    loaded: image ? image.naturalWidth > 0 : false,
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
    fixedNav: getComputedStyle(document.querySelector('.mobile-car-nav')).position,
  }
})()`)
await screenshot('official-tab-mobile.png')
await navigate(`${base}?lang=zh&l1=zh&car=0x0000D1BB&tab=garageSpecs`)
const mobileGarage = await evaluate(`(() => {
  const panel = document.querySelector('.garage-specs-panel')
  const grid = panel?.querySelector('.specs-grid')
  return {
    title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
    tabs: [...document.querySelectorAll('.car-tabs button')].map((node) => node.textContent.trim()),
    panel: panel?.innerText || '',
    cards: panel?.querySelectorAll('.spec-card').length || 0,
    columns: grid ? getComputedStyle(grid).gridTemplateColumns : '',
    bodyWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
    fixedNav: getComputedStyle(document.querySelector('.mobile-car-nav')).position,
  }
})()`)
await screenshot('garage-specs-mobile.png')
await navigate(`${base}?lang=zh&l1=zh&car=0x0000D1BB&tab=vehicleCollection`)
await evaluate(`document.querySelector('.mobile-car-step:last-child')?.click()`)
await sleep(700)
const mobileAfter = await evaluate(`(() => ({
  title: document.querySelector('.dialog-title')?.textContent?.trim() || '',
  tab: document.querySelector('.car-tabs button.on')?.textContent?.trim() || '',
  panel: document.querySelector('.text-panel')?.innerText || '',
}))()`)
const persistedFullscreenOffset = JSON.parse(officialPersisted.storedOffset || '{}')

const checks = {
  sevenTabs: desktop.tabs.length === 7 &&
    desktop.tabs.some((text) => text.includes('官方车表')) &&
    desktop.tabs.some((text) => text.includes('车库数据')),
  officialGallery: officialDesktop.tab.includes('官方车表') &&
    officialDesktop.tabCount === 7 &&
    /^4K · [1-4] \/ 4$/.test(officialDesktop.slides) &&
    officialDesktop.thumbnails === 0 &&
    officialDesktop.inlineControls &&
    officialDesktop.visualWidth > officialDesktop.copyWidth &&
    officialDesktop.image?.src?.endsWith('.avif') &&
    officialDesktop.image.width === 3840 &&
    officialDesktop.image.height === 2160 &&
    officialDesktop.intro.includes('官方车表介绍') &&
    officialDesktop.intro.includes('GT7网站介绍') &&
    officialDesktop.intro.includes('1989年3月') &&
    officialDesktop.intro.includes('The 180SX arrived in March 1989') &&
    !officialDesktop.intro.includes('jpg'),
  officialFullscreen: officialFullscreen.active &&
    officialFullscreen.imageSrc.endsWith('.avif') &&
    officialFullscreen.imageWidth === 3840 &&
    officialFullscreen.imageHeight === 2160 &&
    officialFullscreen.title.includes('180SX') &&
    officialFullscreen.intro.includes('1989年3月') &&
    /^图片 [1-9][0-9]* \/ [1-9][0-9]*$/.test(officialFullscreen.slide) &&
    officialFullscreen.speed === '10' &&
    officialFullscreen.textScale === '1.2' &&
    officialFullscreen.speedPressed === '10×' &&
    officialFullscreen.bodyOverflow === 'hidden',
  fullscreenCarNavigation: fullscreenCarBefore.includes('180SX') &&
    fullscreenCarAfter.length > 0 &&
    fullscreenCarAfter !== fullscreenCarBefore,
  officialSettingsPersisted: officialPersisted.speed === '10' &&
    officialPersisted.textSize === '29' &&
    officialPersisted.captionStyle.includes('font-size: 29px') &&
    officialPersisted.autoplay === '1' &&
    scalePreferenceTest?.scale === '1.2' &&
    scalePreferenceTest.font.includes('19.2px') &&
    officialPersisted.storedSpeed === '10' &&
    officialPersisted.storedScale === '1.2' &&
    Math.abs(persistedFullscreenOffset.x) >= 50 &&
    Math.abs(persistedFullscreenOffset.y) >= 20 &&
    officialPersisted.infoTransform.includes('translate3d') &&
    officialPersisted.infoFontSize.includes('19.2px') &&
    officialPersisted.overlayActive,
  fiveSortModes: groupLabels.length === 5 &&
    sortModes.every((mode) => mode.label && mode.groups > 0 && mode.ids.length > 0) &&
    new Set(sortModes.map((mode) => mode.ids.join(','))).size >= 3,
  catalogSlideshowLauncher: slideshowLauncher.exists &&
    !slideshowLauncher.disabled &&
    slideshowLauncher.label.includes('全屏幻灯片') &&
    catalogSlideshowFirst.active &&
    catalogSlideshowFirst.title.length > 0 &&
    catalogSlideshowFirst.image.endsWith('.avif') &&
    catalogSlideshowAdvanced.counter !== catalogSlideshowFirst.counter &&
    catalogSlideshowResumed.title === catalogSlideshowAdvanced.title &&
    catalogSlideshowResumed.counter === catalogSlideshowAdvanced.counter,
  splitLabels: desktop.tabs.some((text) => text.includes('品牌购物')) &&
    desktop.tabs.some((text) => text.includes('车辆收藏')) &&
    desktop.tabs.some((text) => text.includes('车库数据')) &&
    !desktop.tabs.some((text) => text.includes('品牌购物 / 车库')),
  xiaomiBilingual: desktop.title.includes('SU7') &&
    desktop.panel.includes('小米已在全球智能设备领域') &&
    desktop.panel.includes('Xiaomi has achieved global success'),
  collectionPayload: desktop.panel.includes('小米已在全球智能设备领域') &&
    !desktop.panel.includes('Catch') &&
    !desktop.panel.includes('Long 0x') &&
    !desktop.panel.includes('PS5 RT2') &&
    !desktop.panel.includes('一句话简介') &&
    !desktop.panel.includes('DescriptionLong'),
  englishHeadings: !english.panel.includes('Catch copy') &&
    !english.panel.includes('Car Collection text (game RT2)') &&
    !english.panel.includes('Official car-list overview (short)') &&
    !english.panel.includes('一句话简介') &&
    !english.panel.includes('车辆收藏正文') &&
    english.panel.includes('three-motor all-wheel-drive system'),
  missingCollectionState: missingCollection.title.includes('2002 Turbo') &&
    missingCollection.muted &&
    missingCollection.panel.includes('No Car Collection entry is available'),
  cafeDesktopColumns: cafeDesktop.scenes >= 4 &&
    cafeDesktop.columns.split(' ').length === 2 &&
    cafeDesktop.duplicates === 0 &&
    cafeDesktop.bodyWidth <= cafeDesktop.viewportWidth,
  keyboardTabs: keyboard.officialTab.includes('官方车表') &&
    keyboard.brand.includes('品牌购物') &&
    keyboard.collection.includes('车辆收藏') &&
    keyboard.garage.includes('车库数据'),
  xiaomiGarageBilingual: garageDesktop.title.includes('SU7') &&
    garageDesktop.active.includes('车库数据') &&
    garageDesktop.panel.includes('PP 681.42') &&
    garageDesktop.panel.includes('1,548 PS') &&
    garageDesktop.panel.includes('2,360 公斤') &&
    garageDesktop.panel.includes('四驱') &&
    garageDesktop.panel.includes('1526 HP') &&
    garageDesktop.panel.includes('5203 lbs.') &&
    garageDesktop.panel.includes('4WD') &&
    garageDesktop.panel.includes('中国') &&
    garageDesktop.panel.includes('#公路车') &&
    garageDesktop.panel.includes('#电动车') &&
    garageDesktop.cards === 11 &&
    garageDesktop.headerSpecs.includes('681.42'),
  garageEnglishHeadings: garageEnglish.panel.includes('PERFORMANCE POINTS') &&
    garageEnglish.panel.includes('MAX POWER / RPM') &&
    garageEnglish.panel.includes('WEIGHT') &&
    garageEnglish.panel.includes('1526 HP') &&
    !garageEnglish.panel.includes('最大功率 / 转速') &&
    garageEnglish.tabs.length === 7,
  peugeotCollection: peugeot.title.includes('205 Turbo 16 Evolution 2') &&
    peugeot.panel.includes('entirely different animal') &&
    peugeot.panel.includes('B组别') &&
    !peugeot.panel.includes('0x00007F7E') &&
    !peugeot.panel.includes('PS5 RT2'),
  japaneseCollection: japaneseCollection.title.includes('205 Turbo 16 Evolution 2') &&
    japaneseCollection.panel.includes('世界ラリー選手権') &&
    japaneseCollection.panel.includes('グループBマシン') &&
    !japaneseCollection.panel.includes('The 205 Turbo 16, a Group B car'),
  mobileNoOverflow: mobileBefore.bodyWidth <= mobileBefore.viewportWidth,
  mobileFixedNav: mobileBefore.fixedNav === 'fixed',
  mobileSwitchedCar: mobileBefore.title !== mobileAfter.title,
  mobileCollection: mobileBefore.tabs.length === 7 &&
    mobileBefore.panel.includes('小米已在全球智能设备领域'),
  mobileGarage: mobileGarage.title.includes('SU7') &&
    mobileGarage.tabs.length === 7 &&
    mobileGarage.cards === 11 &&
    mobileGarage.panel.includes('PP 681.42') &&
    mobileGarage.panel.includes('1,548 PS') &&
    mobileGarage.fixedNav === 'fixed' &&
    mobileGarage.bodyWidth <= mobileGarage.viewportWidth,
  cafeMobileColumns: cafeMobile.scenes >= 4 &&
    cafeMobile.columns.split(' ').length === 1 &&
    cafeMobile.fixedNav === 'fixed' &&
    cafeMobile.bodyWidth <= cafeMobile.viewportWidth,
  officialMobileLayout: officialMobile.tabs === 7 &&
    /^4K · [1-4] \/ 4$/.test(officialMobile.imageCount) &&
    officialMobile.loaded &&
    officialMobile.fixedNav === 'fixed' &&
    officialMobile.bodyWidth <= officialMobile.viewportWidth,
  peugeotGarage: garagePeugeot.title.includes('205 Turbo 16 Evolution 2') &&
    garagePeugeot.panel.includes('PP 671.14') &&
    garagePeugeot.panel.includes('455 PS') &&
    garagePeugeot.panel.includes('910 公斤') &&
    garagePeugeot.panel.includes('四驱') &&
    garagePeugeot.panel.includes('法国') &&
    garagePeugeot.panel.includes('#赛车') &&
    garagePeugeot.panel.includes('#WRC世界拉力锦标赛') &&
    garagePeugeot.panel.includes('#中置引擎') &&
    garagePeugeot.cards === 11,
  garageJson: garagePayload.ok &&
    garagePayload.counts.withSpecifications === 580 &&
    garagePayload.counts.withVehicleTypes === 576 &&
    garagePayload.peugeotCountry === 'fr' &&
    garagePayload.peugeotTypes.join(',') === 'Racing Car,WRC,Midship',
  variantGarageAlias: garageVariant.title.includes('Mangusta') &&
    garageVariant.panel.includes('PP 461.13') &&
    garageVariant.panel.includes('305 PS') &&
    garageVariant.panel.includes('该特别版未单独列入官方车表') &&
    garageVariant.cards === 11,
  missingGarageState: garageMissing.title.includes('F1 W08') &&
    garageMissing.muted &&
    garageMissing.panel.includes('这辆车没有官方车库规格'),
  mediumOfficialLayout: mediumOfficial.title.includes('180SX') &&
    mediumOfficial.titleWidth >= 180 &&
    mediumOfficial.titleHeight < 90 &&
    mediumOfficial.headerDirection === 'column' &&
    mediumOfficial.columns === 1 &&
    mediumOfficial.bodyWidth <= mediumOfficial.viewportWidth,
  sixSourceFilters: filterLabels.length === 6 &&
    filterLabels.some((label) => label.includes('品牌店')) &&
    filterLabels.some((label) => label.includes('二手车')) &&
    filterLabels.some((label) => label.includes('车辆收藏')) &&
    filterLabels.some((label) => label.includes('咖啡屋对白')) &&
    filterLabels.some((label) => label.includes('隐藏车型') && label.includes('11')),
  sourceFilterCounts: ['all', 'brandCentral', 'usedCar', 'vehicleCollection', 'cafe', 'hidden']
    .every((id) => filterObserved[id] === filterExpected[id]),
  officialCounts: catalogCounts.hero.includes(String(filterExpected.all)) &&
    catalogCounts.hero.includes(`官方车表 ${filterExpected.officialPage}`) &&
    catalogCounts.hero.includes(`隐藏车型 ${filterExpected.hidden}`) &&
    catalogCounts.serviceWorker,
  localizedBrandIdentity: (() => {
    const zhXiaomi = catalogCounts.brandHeadings.filter((name) =>
      /Xiaomi|小米/.test(name),
    )
    const enXiaomi = english.brandHeadings.filter((name) =>
      /Xiaomi|小米/.test(name),
    )
    return zhXiaomi.length === 1 &&
      zhXiaomi[0].includes('小米') &&
      !zhXiaomi[0].includes('Xiaomi') &&
      enXiaomi.length === 1 &&
      enXiaomi[0].includes('Xiaomi') &&
      !enXiaomi[0].includes('小米') &&
      catalogCounts.brandHeadings.filter((name) => /Afeela|AFEELA/.test(name)).length === 1
  })(),
  officialOnlyCars: officialOnlySearch.count === 1 &&
    officialOnlySearch.names.some((name) => name.includes('Xiaomi Vision Gran Turismo')) &&
    officialOnlySearch.ids.includes('official:car3656'),
  mobileFilterStrip: mobileFilters.buttons === 6 &&
    mobileFilters.bodyWidth <= mobileFilters.viewportWidth &&
    mobileFilters.scrollWidth >= mobileFilters.clientWidth,
}
const report = {
  filterExpected,
  garagePayload,
  filterObserved,
  filterLabels,
  catalogCounts,
  officialOnlySearch,
  groupLabels,
  sortModes,
  scalePreferenceTest,
  mobileFilters,
  desktop,
  cafeDesktop,
  officialDesktop,
  officialFullscreen,
  fullscreenCarBefore,
  fullscreenCarAfter,
  officialPersisted,
  slideshowLauncher,
  catalogSlideshowFirst,
  catalogSlideshowAdvanced,
  catalogSlideshowResumed,
  garageDesktop,
  english,
  garageEnglish,
  missingCollection,
  keyboard,
  peugeot,
  japaneseCollection,
  garagePeugeot,
  garageVariant,
  garageMissing,
  mediumOfficial,
  mobileBefore,
  cafeMobile,
  officialMobile,
  mobileGarage,
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
