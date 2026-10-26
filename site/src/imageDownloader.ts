import {
  IMAGE_CACHE_NAME,
  loadImageManifest,
  type ImageAsset,
  type ImageManifest,
} from './imageCache'
import type { CarsPayload, MirrorCatalog } from './types'

export type ImageDownloaderPhase =
  | 'idle'
  | 'preparing'
  | 'scanning'
  | 'downloading'
  | 'paused'
  | 'complete'
  | 'complete-with-errors'
  | 'error'

export type ImageDownloaderStats = {
  total: number
  processed: number
  cached: number
  downloaded: number
  failed: number
  downloadedBytes: number
  sourceBytes: number
  currentPath: string
}

export type ImageDownloaderSnapshot = {
  phase: ImageDownloaderPhase
  manifest: ImageManifest | null
  stats: ImageDownloaderStats
  error: string | null
  unsupported: boolean
  persisted: boolean
  quota: number
  usage: number
}

export const DOWNLOAD_CONCURRENCY = 24
const DOWNLOAD_ATTEMPTS = 3
const DOWNLOAD_TIMEOUT_MS = 20_000

const EMPTY_STATS: ImageDownloaderStats = {
  total: 0,
  processed: 0,
  cached: 0,
  downloaded: 0,
  failed: 0,
  downloadedBytes: 0,
  sourceBytes: 0,
  currentPath: '',
}

const listeners = new Set<() => void>()
let activeController: AbortController | null = null
let activeRun: Promise<void> | null = null
let lastCatalog: CarsPayload | null = null
let lastMirrors: Record<string, MirrorCatalog> = {}
let snapshot: ImageDownloaderSnapshot = {
  phase: 'idle',
  manifest: null,
  stats: EMPTY_STATS,
  error: null,
  unsupported: false,
  persisted: false,
  quota: 0,
  usage: 0,
}

function publish(patch: Partial<ImageDownloaderSnapshot>) {
  snapshot = { ...snapshot, ...patch }
  for (const listener of listeners) listener()
}

function updateStats(
  updater: (current: ImageDownloaderStats) => ImageDownloaderStats,
) {
  publish({ stats: updater(snapshot.stats) })
}

function isSupported() {
  return (
    typeof window !== 'undefined' &&
    'caches' in window &&
    'serviceWorker' in navigator
  )
}

function registerImageServiceWorker() {
  if (!('serviceWorker' in navigator)) return
  void navigator.serviceWorker
    .register('/sw.js', {
      scope: '/',
      updateViaCache: 'none',
    })
    .then(() => navigator.serviceWorker.ready)
    .catch((reason: unknown) => {
      console.warn('Unable to activate GT7 offline image worker', reason)
    })
}

function delay(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds))
}

async function cacheAsset(
  cache: Cache,
  asset: ImageAsset,
  parentSignal: AbortSignal,
) {
  let lastError: unknown = null
  for (let attempt = 0; attempt < DOWNLOAD_ATTEMPTS; attempt += 1) {
    const requestController = new AbortController()
    const abortRequest = () => requestController.abort()
    parentSignal.addEventListener('abort', abortRequest, { once: true })
    const timeout = window.setTimeout(
      () =>
        requestController.abort(
          new DOMException('Image request timed out', 'TimeoutError'),
        ),
      DOWNLOAD_TIMEOUT_MS,
    )
    try {
      const response = await fetch(asset.path, {
        cache: 'no-store',
        headers: { 'X-GT7-Direct-Download': '1' },
        signal: requestController.signal,
      })
      if (!response.ok) throw new Error(`${response.status} ${asset.path}`)
      const contentLength = Number(response.headers.get('content-length'))
      await cache.put(asset.path, response)
      return (
        Number.isFinite(contentLength) && contentLength > 0
          ? contentLength
          : asset.bytes
      )
    } catch (reason) {
      if (parentSignal.aborted) throw reason
      lastError = reason
      if (attempt + 1 < DOWNLOAD_ATTEMPTS) {
        await delay(400 * 2 ** attempt)
      }
    } finally {
      window.clearTimeout(timeout)
      parentSignal.removeEventListener('abort', abortRequest)
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error(`Unable to cache ${asset.path}`)
}

async function refreshStorageEstimate() {
  if (typeof navigator.storage === 'undefined') return
  const persistRequest = navigator.storage.persist?.()
  const persisted = persistRequest ? await persistRequest.catch(() => false) : false
  const estimate = await navigator.storage.estimate?.().catch(() => null)
  publish({
    persisted: Boolean(persisted),
    quota: estimate?.quota || 0,
    usage: estimate?.usage || 0,
  })
}

function beginRun(task: (controller: AbortController) => Promise<void>) {
  const controller = new AbortController()
  activeController = controller
  const run = task(controller)
    .catch((reason: unknown) => {
      if (
        controller.signal.aborted ||
        (reason instanceof DOMException && reason.name === 'AbortError')
      ) {
        return
      }
      publish({
        phase: 'error',
        error: reason instanceof Error ? reason.message : String(reason),
      })
    })
    .finally(() => {
      if (activeController === controller) {
        activeController = null
        activeRun = null
      }
      void refreshStorageEstimate()
    })
  activeRun = run
  return run
}

async function downloadManifest(
  manifest: ImageManifest,
  controller: AbortController,
) {
  const queue = manifest.assets
  let nextIndex = 0
  let failedThisRun = 0
  let networkStarted = false

  publish({
    phase: 'scanning',
    manifest,
    stats: {
      ...EMPTY_STATS,
      total: queue.length,
      sourceBytes: manifest.total_bytes,
    },
    error: null,
    unsupported: false,
  })

  const cache = await caches.open(IMAGE_CACHE_NAME)
  const consume = async (asset: ImageAsset) => {
    if (controller.signal.aborted) return
    let outcome: 'cached' | 'downloaded' | 'failed' = 'failed'
    let receivedBytes = 0
    updateStats((current) => ({ ...current, currentPath: asset.path }))

    try {
      const cached = await cache.match(asset.path, { ignoreVary: true })
      if (cached?.ok) {
        outcome = 'cached'
        return
      }

      if (!networkStarted) {
        networkStarted = true
        publish({ phase: 'downloading' })
      }
      receivedBytes = await cacheAsset(cache, asset, controller.signal)
      outcome = 'downloaded'
    } catch {
      if (controller.signal.aborted) return
      failedThisRun += 1
    } finally {
      if (!controller.signal.aborted) {
        updateStats((current) => ({
          ...current,
          processed: current.processed + 1,
          cached: current.cached + (outcome === 'cached' ? 1 : 0),
          downloaded:
            current.downloaded + (outcome === 'downloaded' ? 1 : 0),
          failed: current.failed + (outcome === 'failed' ? 1 : 0),
          downloadedBytes: current.downloadedBytes + receivedBytes,
        }))
      }
    }
  }

  const worker = async () => {
    while (!controller.signal.aborted) {
      const asset = queue[nextIndex]
      nextIndex += 1
      if (!asset) return
      await consume(asset)
    }
  }

  await Promise.all(
    Array.from(
      { length: Math.min(DOWNLOAD_CONCURRENCY, queue.length) },
      () => worker(),
    ),
  )
  if (controller.signal.aborted) return
  updateStats((current) => ({ ...current, currentPath: '' }))
  publish({
    phase: failedThisRun ? 'complete-with-errors' : 'complete',
  })
}

export function getImageDownloaderSnapshot() {
  return snapshot
}

export function subscribeImageDownloader(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function startImageDownload(
  catalog: CarsPayload,
  mirrors: Record<string, MirrorCatalog> = {},
) {
  if (activeRun) return activeRun
  lastCatalog = catalog
  lastMirrors = mirrors
  publish({ phase: 'preparing', error: null, unsupported: false })

  return beginRun(async (controller) => {
    if (!isSupported()) {
      publish({
        phase: 'error',
        unsupported: true,
        error: 'This browser does not support Cache Storage and Service Workers',
      })
      return
    }
    registerImageServiceWorker()
    void refreshStorageEstimate()
    const manifest = await loadImageManifest(catalog, mirrors, controller.signal)
    await downloadManifest(manifest, controller)
  })
}

export function ensureAutoImageDownload(
  catalog: CarsPayload,
  mirrors: Record<string, MirrorCatalog> = {},
) {
  if (!isSupported() || activeRun) return activeRun
  if (snapshot.phase === 'complete') return Promise.resolve()
  return startImageDownload(catalog, mirrors)
}

export function pauseImageDownload() {
  const controller = activeController
  if (!controller) return
  activeController = null
  activeRun = null
  controller.abort()
  publish({ phase: 'paused', stats: { ...snapshot.stats, currentPath: '' } })
}

export function resumeImageDownload() {
  if (activeRun || !snapshot.manifest) return
  const manifest = snapshot.manifest
  beginRun((controller) => downloadManifest(manifest, controller))
}

export function retryImageDownload() {
  if (activeRun) return
  if (snapshot.manifest) {
    resumeImageDownload()
  } else if (lastCatalog) {
    void startImageDownload(lastCatalog, lastMirrors)
  }
}
