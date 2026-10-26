const IMAGE_CACHE_NAME = 'gt7-catalog-images-v1'
const MEDIA_PREFIXES = [
  '/common/',
  '/cars/',
  '/logos/',
  '/avatars/',
  '/menu_covers/',
  '/official/',
]
const MEDIA_EXTENSION =
  /\.(?:avif|jxl|webp|jpe?g|png|gif|svg|ico)(?:[?#].*)?$/i

function isMediaRequest(request) {
  const url = new URL(request.url)
  return (
    url.origin === self.location.origin &&
    MEDIA_PREFIXES.some((prefix) => url.pathname.startsWith(prefix)) &&
    MEDIA_EXTENSION.test(url.pathname)
  )
}

self.addEventListener('install', () => {
  self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys()
      await Promise.all(
        names
          .filter((name) => name.startsWith('gt7-catalog-images-') && name !== IMAGE_CACHE_NAME)
          .map((name) => caches.delete(name)),
      )
      await self.clients.claim()
    })(),
  )
})

self.addEventListener('message', (event) => {
  if (event.data === 'SKIP_WAITING') self.skipWaiting()
})

self.addEventListener('fetch', (event) => {
  const { request } = event
  if (
    request.method !== 'GET' ||
    !isMediaRequest(request) ||
    request.headers.has('range')
  ) {
    return
  }

  if (request.headers.get('X-GT7-Direct-Download') === '1') {
    event.respondWith(fetch(request))
    return
  }

  event.respondWith(
    (async () => {
      const cache = await caches.open(IMAGE_CACHE_NAME)
      const cached = await cache.match(request, { ignoreVary: true })
      if (cached) return cached
      const response = await fetch(request)
      if (response.ok && response.status === 200) {
        await cache.put(request, response.clone())
      }
      return response
    })(),
  )
})
