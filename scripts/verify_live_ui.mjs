#!/usr/bin/env node
/**
 * Verify the deployed GT7 Cafe UI through the Chrome DevTools Protocol.
 *
 * Usage:
 *   node verify_live_ui.mjs --base https://gt7.gakaki.com/
 *     [--cdp http://127.0.0.1:9333] [--out-dir ../site]
 */

import fs from 'node:fs/promises'
import path from 'node:path'

function arg(name, fallback) {
  const index = process.argv.indexOf(name)
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback
}

const base = arg('--base', 'https://gt7.gakaki.com/')
const cdpRoot = arg('--cdp', 'http://127.0.0.1:9333')
const outDir = path.resolve(arg('--out-dir', '../site'))

class Cdp {
  constructor(ws) {
    this.ws = ws
    this.nextId = 1
    this.pending = new Map()
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data)
      if (!message.id) return
      const pending = this.pending.get(message.id)
      if (!pending) return
      this.pending.delete(message.id)
      if (message.error) pending.reject(new Error(message.error.message))
      else pending.resolve(message.result)
    })
  }

  call(method, params = {}) {
    const id = this.nextId++
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject })
      this.ws.send(JSON.stringify({ id, method, params }))
    })
  }
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

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
  await cdp.call('Emulation.setDeviceMetricsOverride', {
    width: 1440,
    height: 1200,
    deviceScaleFactor: 1,
    mobile: false,
  })

  async function navigate(url, waitMs = 6000) {
    const loaded = new Promise((resolve) => {
      const listener = (event) => {
        const message = JSON.parse(event.data)
        if (message.method === 'Page.loadEventFired') {
          ws.removeEventListener('message', listener)
          resolve()
        }
      }
      ws.addEventListener('message', listener)
    })
    await cdp.call('Page.navigate', { url })
    await Promise.race([loaded, sleep(10000)])
    await sleep(waitMs)
  }

  async function evaluate(expression) {
    const result = await cdp.call('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    })
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.text)
    }
    return result.result.value
  }

  async function screenshot(name) {
    const result = await cdp.call('Page.captureScreenshot', {
      format: 'png',
      fromSurface: true,
      captureBeyondViewport: false,
    })
    await fs.mkdir(outDir, { recursive: true })
    await fs.writeFile(path.join(outDir, name), Buffer.from(result.data, 'base64'))
  }

  await navigate(`${base}?lang=both`)
  const overview = await evaluate(`(() => ({
    title: document.title,
    cards: document.querySelectorAll('.card').length,
    brands: document.querySelectorAll('.brandgroup').length,
    cast: document.querySelectorAll('.cast-person').length,
    text: document.body.innerText.slice(0, 1200)
  }))()`)
  await screenshot('live_main_verified.png')

  await navigate(`${base}?lang=both&car=0x0000D1BB`)
  const car = await evaluate(`(() => {
    const modal = document.querySelector('.dialog')
    const tabs = [...document.querySelectorAll('.car-tabs button')]
    const summary = modal?.innerText.slice(0, 1800) || ''
    return {
      modal: !!modal,
      title: document.querySelector('.dialog h3')?.textContent || '',
      tabs: tabs.map((node) => ({ text: node.textContent.trim(), muted: node.classList.contains('muted') })),
      hasXiaomi: summary.includes('Xiaomi') || summary.includes('小米'),
      hasTextId: summary.includes('0x0000CF2F'),
      width: document.documentElement.scrollWidth,
    }
  })()`)
  await evaluate(`(() => {
    const button = [...document.querySelectorAll('.car-tabs button')]
      .find((node) => node.textContent.includes('品牌'))
    button?.click()
    return !!button
  })()`)
  await sleep(800)
  const brandPanel = await evaluate(`(() => ({
    text: document.querySelector('.text-panel')?.innerText.slice(0, 2200) || '',
    images: [...document.querySelectorAll('.text-panel img')].map((node) => ({
      src: node.getAttribute('src'),
      naturalWidth: node.naturalWidth,
    })),
  }))()`)
  await screenshot('live_su7_verified.png')

  await navigate(`${base}?lang=both&review=unmatched`)
  const review = await evaluate(`(() => ({
    modal: !!document.querySelector('.review-dialog, .unmatched-modal, .dialog'),
    entries: document.querySelectorAll('.review-entry, .unmatched-entry, .entry').length,
    text: document.body.innerText.slice(0, 2000),
  }))()`)
  await screenshot('live_review_verified.png')

  const checks = {
    overview: overview.cards > 500 && overview.brands > 10 && overview.cast >= 15,
    xiaomi: car.modal && car.title.includes('SU7') &&
      car.tabs.some((tab) => /品牌|Brand/.test(tab.text)) &&
      car.hasXiaomi && brandPanel.text.includes('0x0000CF2F') &&
      brandPanel.text.includes('Nürburgring'),
    review: review.entries > 0 && review.text.includes('253'),
  }
  const report = { base, overview, car, brandPanel, review, checks }
  await fs.writeFile(
    path.join(outDir, 'live_ui_verified.json'),
    JSON.stringify(report, null, 2),
    'utf8',
  )
  console.log(JSON.stringify({ checks, overview, car, review }, null, 2))
  if (Object.values(checks).some((value) => !value)) process.exitCode = 1
  ws.close()
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
