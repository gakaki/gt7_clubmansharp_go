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

async function clickSelector(selector) {
  const point = await evaluate(`(() => {
    const node = document.querySelector(${JSON.stringify(selector)})
    const rect = node?.getBoundingClientRect()
    return rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : null
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
const filterObserved = {}
for (const [index, id] of ['all', 'brandCentral', 'usedCar', 'vehicleCollection', 'cafe'].entries()) {
  await evaluate(`document.querySelectorAll('.seg button')[${index}]?.click()`)
  await sleep(250)
  filterObserved[id] = await evaluate(`document.querySelectorAll('.card').length`)
}
const filterLabels = await evaluate(`[...document.querySelectorAll('.seg button')].map((node) => node.textContent.trim())`)
await screenshot('main-source-filters.png')

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
  slides: document.querySelectorAll('.official-thumbnails button').length,
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
  const ranges = document.querySelectorAll('.official-controls input[type=range]')
  if (ranges.length < 2) {
    throw new Error('official controls missing; tabs=' + document.querySelectorAll('.car-tabs button').length + ', text=' + document.body.innerText.slice(0, 300) + ', html=' + document.body.innerHTML.slice(0, 300))
  }
  ranges[0].value = '7'
  ranges[0].dispatchEvent(new Event('input', { bubbles: true }))
  ranges[1].value = '29'
  ranges[1].dispatchEvent(new Event('input', { bubbles: true }))
})()`)
await clickSelector('.official-fullscreen-button')
await sleep(700)
await evaluate(`document.querySelector('.official-position-buttons button:nth-child(4)')?.click()`)
await evaluate(`(() => {
  const ranges = document.querySelectorAll('.official-fullscreen-controls input[type=range]')
  ranges[0].value = '10'
  ranges[0].dispatchEvent(new Event('input', { bubbles: true }))
})()`)
await sleep(700)
const officialFullscreen = await evaluate(`(() => {
  const caption = document.querySelector('.official-caption')
  return {
    active: document.fullscreenElement === document.querySelector('.official-stage'),
    captionDisplay: getComputedStyle(caption).display,
    controlsDisplay: getComputedStyle(document.querySelector('.official-fullscreen-controls')).display,
    slide: document.querySelector('.official-image-count')?.textContent?.trim() || '',
    position: caption?.getAttribute('style') || '',
    autoplay: localStorage.getItem('gt7cafe.official.autoplay'),
  }
})()`)
await screenshot('official-tab-fullscreen.png')
await evaluate(`document.exitFullscreen?.()`)
await sleep(250)
await navigate(`${base}?lang=both&l1=zh&l2=us&car=0x00006F48&tab=official`)
const officialPersisted = await evaluate(`(() => {
  const ranges = document.querySelectorAll('.official-controls input[type=range]')
  const caption = document.querySelector('.official-caption')
  return {
    speed: ranges[0]?.value || '',
    textSize: ranges[1]?.value || '',
    captionStyle: caption?.getAttribute('style') || '',
    autoplay: localStorage.getItem('gt7cafe.official.autoplay'),
  }
})()`)
await screenshot('official-tab-desktop.png')

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
    slides: document.querySelectorAll('.official-thumbnails button').length,
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

const checks = {
  sevenTabs: desktop.tabs.length === 7 &&
    desktop.tabs.some((text) => text.includes('官方车表')) &&
    desktop.tabs.some((text) => text.includes('车库数据')),
  officialGallery: officialDesktop.tab.includes('官方车表') &&
    officialDesktop.tabCount === 7 &&
    officialDesktop.slides === 4 &&
    officialDesktop.image?.src?.endsWith('.jxl') &&
    officialDesktop.image.width === 3840 &&
    officialDesktop.image.height === 2160 &&
    officialDesktop.intro.includes('1989年3月') &&
    officialDesktop.intro.includes('The 180SX arrived in March 1989') &&
    !officialDesktop.intro.includes('jpg'),
  officialFullscreen: officialFullscreen.active &&
    officialFullscreen.captionDisplay === 'block' &&
    officialFullscreen.controlsDisplay === 'flex' &&
    /^4K · [1-4] \/ 4$/.test(officialFullscreen.slide) &&
    officialFullscreen.position.includes('left: 76%') &&
    officialFullscreen.position.includes('top: 78%') &&
    officialFullscreen.autoplay === '1',
  officialSettingsPersisted: officialPersisted.speed === '10' &&
    officialPersisted.textSize === '29' &&
    officialPersisted.captionStyle.includes('left: 76%') &&
    officialPersisted.captionStyle.includes('top: 78%') &&
    officialPersisted.captionStyle.includes('font-size: 29px') &&
    officialPersisted.autoplay === '1',
  splitLabels: desktop.tabs.some((text) => text.includes('品牌购物')) &&
    desktop.tabs.some((text) => text.includes('车辆收藏')) &&
    desktop.tabs.some((text) => text.includes('车库数据')) &&
    !desktop.tabs.some((text) => text.includes('品牌购物 / 车库')),
  xiaomiBilingual: desktop.title.includes('SU7') &&
    desktop.panel.includes('智能手机巨头') &&
    desktop.panel.includes('smartphone giant'),
  collectionPayload: desktop.panel.includes('Catch') &&
    desktop.panel.includes('Long') &&
    desktop.panel.includes('0x0000CF1E') &&
    desktop.panel.includes('0x0000CF1F'),
  englishHeadings: english.panel.includes('Catch copy') &&
    english.panel.includes('Car Collection text (game RT2)') &&
    english.panel.includes('Official car-list overview (short)') &&
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
    peugeot.panel.includes('Group B racing world by storm') &&
    peugeot.panel.includes('entirely different animal') &&
    peugeot.panel.includes('官方车表简介（短版）') &&
    peugeot.panel.includes('B组别') &&
    peugeot.panel.includes('0x00007F7E') &&
    peugeot.panel.includes('0x00007F7F'),
  japaneseCollection: japaneseCollection.title.includes('205 Turbo 16 Evolution 2') &&
    japaneseCollection.panel.includes('世界ラリー選手権') &&
    japaneseCollection.panel.includes('グループBマシン') &&
    !japaneseCollection.panel.includes('The 205 Turbo 16, a Group B car'),
  mobileNoOverflow: mobileBefore.bodyWidth <= mobileBefore.viewportWidth,
  mobileFixedNav: mobileBefore.fixedNav === 'fixed',
  mobileSwitchedCar: mobileBefore.title !== mobileAfter.title,
  mobileCollection: mobileBefore.tabs.length === 7 &&
    mobileBefore.panel.includes('智能手机巨头'),
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
    officialMobile.slides === 4 &&
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
  garagePayload,
  filterObserved,
  filterLabels,
  mobileFilters,
  desktop,
  cafeDesktop,
  officialDesktop,
  officialFullscreen,
  officialPersisted,
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
