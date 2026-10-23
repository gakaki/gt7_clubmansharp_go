#!/usr/bin/env node
/** Validate the Cafe menu archive through Chrome DevTools Protocol. */

import fs from 'node:fs/promises'
import path from 'node:path'

const base = process.argv[2] || 'http://127.0.0.1:4173'
const cdpRoot = process.argv[3] || 'http://127.0.0.1:9333'
const output = path.resolve('reports')
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

class Cdp {
  constructor(ws) {
    this.ws = ws
    this.id = 0
    this.pending = new Map()
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data)
      if (!message.id || !this.pending.has(message.id)) return
      const pending = this.pending.get(message.id)
      this.pending.delete(message.id)
      if (message.error) pending.reject(new Error(message.error.message))
      else pending.resolve(message.result)
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

async function main() {
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

  async function metrics(width, height, mobile) {
    await cdp.call('Emulation.setDeviceMetricsOverride', {
      width,
      height,
      deviceScaleFactor: 1,
      mobile,
    })
  }

  async function navigate(url) {
    await cdp.call('Page.navigate', { url })
    for (let attempt = 0; attempt < 60; attempt++) {
      await sleep(150)
      try {
        if (await evaluate(`document.querySelectorAll('.menu-book-card').length > 0`)) {
          await evaluate('scrollTo(0, 0)')
          for (let imageAttempt = 0; imageAttempt < 30; imageAttempt++) {
            await sleep(150)
            try {
              const loaded = await evaluate(`(() => {
                const image = document.querySelector('.menu-book-card .menu-cover > img')
                return !image || image.naturalWidth > 0
              })()`)
              if (loaded) break
            } catch {
              // The renderer may still be committing the new document.
            }
          }
          return
        }
      } catch {
        // Runtime contexts are briefly unavailable while navigation commits.
      }
    }
    throw new Error(`page did not render: ${url}`)
  }

  async function screenshot(name) {
    const image = await cdp.call('Page.captureScreenshot', {
      format: 'png',
      fromSurface: true,
      captureBeyondViewport: false,
    })
    await fs.mkdir(output, { recursive: true })
    await fs.writeFile(path.join(output, name), Buffer.from(image.data, 'base64'))
  }

  await metrics(1440, 1100, false)
  await navigate(`${base}/menus_cafe?lang=both`)
  const desktop = await evaluate(`(() => ({
    cards: document.querySelectorAll('.menu-book-card').length,
    covers: document.querySelectorAll(
      '.menu-book-card .menu-cover > img'
    ).length,
    missingCoverCards: [...document.querySelectorAll('.menu-book-card')]
      .filter((node) => !node.querySelector('.menu-cover > img'))
      .map((node) => node.innerText.split('\\n')[0])
      .slice(0, 50),
    online57Cover: !!document.querySelectorAll('.menu-book-card')[107]
      ?.querySelector('.menu-cover > img'),
    loadedVisibleCovers: [...document.querySelectorAll(
      '.menu-book-card .menu-cover > img'
    )].filter((node) => node.naturalWidth > 0).length,
    navDisplay: getComputedStyle(document.querySelector('.menu-mobile-nav')).display,
    hasReplacementGlyph: document.body.innerText.includes('\\uFFFD'),
    online57: [...document.querySelectorAll('.menu-book-card')]
      .some((node) => node.innerText.includes('EM 57')),
    width: document.documentElement.scrollWidth,
    viewport: innerWidth
  }))()`)
  await screenshot('menus_cafe_desktop.png')

  await navigate(`${base}/menus_cafe?book=extra-052&lang=both`)
  const dialog = await evaluate(`(() => ({
    open: !!document.querySelector('.menu-dialog-shell'),
    heading: document.querySelector('.menu-dialog-shell')?.innerText.slice(0, 700) || '',
    languageButtons: document.querySelectorAll(
      '.menu-dialog-header .menu-language-switch button'
    ).length,
    coverLoaded: [...document.querySelectorAll('.menu-dialog-shell img')]
      .some((node) => node.naturalWidth > 0)
  }))()`)
  await evaluate(`window.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })
  )`)
  await sleep(400)
  const stepped = await evaluate(`(() => ({
    url: location.search,
    heading: document.querySelector('.menu-dialog-shell')?.innerText.slice(0, 500) || ''
  }))()`)

  await navigate(`${base}/menus_cafe?book=extra-056&lang=zh`)
  await evaluate(`document.querySelector('.menu-switch')?.click()`)
  await sleep(5550)
  const autoplay = await evaluate(`(() => ({
    url: location.search,
    heading: document.querySelector('.menu-dialog-shell')?.innerText.slice(0, 220) || ''
  }))()`)
  await evaluate(`document.querySelector('.menu-switch')?.click()`)

  await metrics(390, 844, true)
  await navigate(`${base}/menus_cafe?book=extra-052&lang=zh`)
  await screenshot('menus_cafe_mobile_detail.png')
  await evaluate(`document.querySelector('.menu-dialog-header .menu-icon-button:last-child')?.click()`)
  await sleep(250)
  await evaluate('scrollTo(0, 0)')
  await screenshot('menus_cafe_mobile_top.png')
  const fixedBefore = await evaluate(`(() => {
    const node = document.querySelector('.menu-mobile-nav')
    const rect = node.getBoundingClientRect()
    return {
      top: rect.top,
      bottom: rect.bottom,
      display: getComputedStyle(node).display,
      text: node.innerText
    }
  })()`)
  await evaluate('scrollTo(0, document.body.scrollHeight)')
  await sleep(250)
  const fixedAfter = await evaluate(`(() => {
    const rect = document.querySelector('.menu-mobile-nav').getBoundingClientRect()
    return {
      top: rect.top,
      bottom: rect.bottom,
      width: innerWidth,
      scrollWidth: document.documentElement.scrollWidth,
      hasReplacementGlyph: document.body.innerText.includes('\\uFFFD')
    }
  })()`)
  await screenshot('menus_cafe_mobile.png')

  const missingHref = await evaluate(`(() => [...document.images]
    .filter((node) => node.complete && node.naturalWidth === 0)
    .map((node) => node.src)
    .filter((src) => src.includes('/menu_covers/')))()`)
  const checks = {
    desktopCount: desktop.cards === 108,
    desktopCovers: desktop.covers === 74 && desktop.loadedVisibleCovers > 0,
    desktopNoMobileNav: desktop.navDisplay === 'none',
    desktopNoMojibakeBox: !desktop.hasReplacementGlyph,
    online57Present: desktop.online57,
    dialogOpened: dialog.open,
    dialogHyundai: /Hyundai|现代/.test(dialog.heading),
    dialogLanguageControls: dialog.languageButtons === 3,
    dialogCoverLoaded: dialog.coverLoaded,
    keyboardTargetStepped:
      stepped.url.includes('book=extra-053') && /Muscle|肌肉/.test(stepped.heading),
    autoplayAdvanced:
      autoplay.url.includes('book=extra-057') && /Le Mans|勒芒/.test(autoplay.heading),
    mobileNavVisible: fixedBefore.display !== 'none',
    mobileNavKeepsLastViewed: /EM\\s*52|Hyundai/.test(fixedBefore.text),
    mobileNavFixed:
      Math.abs(fixedBefore.top - fixedAfter.top) < 1 &&
      Math.abs(fixedBefore.bottom - fixedAfter.bottom) < 1,
    mobileNoOverflow: fixedAfter.scrollWidth === fixedAfter.width,
    mobileNoMojibakeBox: !fixedAfter.hasReplacementGlyph,
    brokenCoverImages: missingHref.length === 0,
  }
  const report = {
    desktop,
    dialog,
    stepped,
    autoplay,
    fixedBefore,
    fixedAfter,
    missingHref,
    checks,
  }
  await fs.writeFile(
    path.join(output, 'menus_cafe_ui_verified.json'),
    JSON.stringify(report, null, 2),
    'utf8',
  )
  console.log(JSON.stringify(report, null, 2))
  if (Object.values(checks).some((value) => !value)) process.exitCode = 1
  ws.close()
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
