import type { CarsPayload, MirrorCatalog } from './types'

export const IMAGE_CACHE_NAME = 'gt7-catalog-images-v2'
export const IMAGE_MANIFEST_URL = '/data/image-manifest.json'

export type ImageAsset = {
  path: string
  bytes: number
}

export type ImageManifest = {
  schema_version: 1
  count: number
  total_bytes: number
  assets: ImageAsset[]
}

const IMAGE_EXTENSION =
  /\.(?:avif|jxl|webp|jpe?g|png|gif|svg|ico)(?:[?#].*)?$/i

export function isLocalImagePath(value: unknown): value is string {
  return (
    typeof value === 'string' &&
    value.startsWith('/') &&
    !value.startsWith('//') &&
    IMAGE_EXTENSION.test(value)
  )
}

function absoluteImagePath(value: unknown, prefix = ''): string | null {
  if (typeof value !== 'string' || !value.trim()) return null
  const candidate = value.startsWith('/')
    ? value
    : `/${prefix ? `${prefix.replace(/^\/|\/$/g, '')}/` : ''}${value}`
  return isLocalImagePath(candidate) ? candidate : null
}

function collectCarImages(catalog: CarsPayload) {
  const paths = new Set<string>()
  const add = (value: unknown, prefix = '') => {
    const path = absoluteImagePath(value, prefix)
    if (path) paths.add(path)
  }

  for (const car of catalog.cars || []) {
    add(car.image, '')
    add(car.brandLogo, 'logos')
    add(car.officialDetails?.heroZh)
    add(car.officialDetails?.heroEn)
    for (const image of car.officialDetails?.images || []) add(image.jxl, '')
    for (const turn of [...(car.conversation || []), ...(car.dialogues || [])]) {
      add(turn.avatar, '')
    }
    for (const text of Object.values(car.texts || {})) {
      for (const variant of text?.variants || []) {
        add((variant as Record<string, unknown>).avatar, '')
      }
    }
  }
  for (const speaker of [...(catalog.speakers || []), ...(catalog.cast || [])]) {
    add(speaker?.avatar, '')
  }
  return paths
}

function collectMirrorImages(mirrors: Record<string, MirrorCatalog>) {
  const paths = new Set<string>()
  for (const catalog of Object.values(mirrors)) {
    for (const car of catalog?.cars || []) {
      for (const image of [
        car.hero,
        car.logo,
        car.thumbnail,
        car.country_flag,
        ...(car.images || []),
      ]) {
        if (isLocalImagePath(image)) paths.add(image)
      }
    }
    for (const image of Object.values(
      (catalog as MirrorCatalog & { country_flags?: Record<string, string> })
        .country_flags || {},
    )) {
      if (isLocalImagePath(image)) paths.add(image)
    }
  }
  return paths
}

function downloadPriority(path: string) {
  if (path.startsWith('/official/')) return 0
  if (path.startsWith('/common/dist/gt7/carlist/assets/')) return 1
  if (path.startsWith('/cars/')) return 2
  if (path.startsWith('/common/')) return 3
  if (path.startsWith('/logos/')) return 4
  if (path.startsWith('/avatars/')) return 5
  return 6
}

async function collectMenuBookCovers(signal: AbortSignal) {
  const paths = new Set<string>()
  try {
    const response = await fetch('/data/menu_books.json', {
      cache: 'no-cache',
      signal,
    })
    if (!response.ok) return paths
    const payload = (await response.json()) as {
      records?: Array<{ cover?: string; thumbnail?: string }>
    }
    for (const record of payload.records || []) {
      const cover = absoluteImagePath(record.cover)
      const thumbnail = absoluteImagePath(record.thumbnail)
      if (cover) paths.add(cover)
      if (thumbnail) paths.add(thumbnail)
    }
  } catch {
    // The cars and mirror catalogs remain a complete fallback manifest.
  }
  return paths
}

export async function imageManifestFromCatalogs(
  catalog: CarsPayload,
  mirrors: Record<string, MirrorCatalog>,
  signal: AbortSignal,
): Promise<ImageManifest> {
  const paths = collectCarImages(catalog)
  for (const path of collectMirrorImages(mirrors)) paths.add(path)
  for (const path of await collectMenuBookCovers(signal)) paths.add(path)
  const assets = [...paths]
    .sort(
      (left, right) =>
        downloadPriority(left) - downloadPriority(right) ||
        left.localeCompare(right),
    )
    .map((path) => ({ path, bytes: 0 }))
  if (!assets.length) throw new Error('No local images were found for offline caching')
  return {
    schema_version: 1,
    count: assets.length,
    total_bytes: 0,
    assets,
  }
}

export async function loadImageManifest(
  catalog: CarsPayload,
  mirrors: Record<string, MirrorCatalog>,
  signal: AbortSignal,
): Promise<ImageManifest> {
  try {
    const response = await fetch(IMAGE_MANIFEST_URL, {
      cache: 'no-cache',
      signal,
    })
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    const raw = (await response.json()) as Partial<ImageManifest>
    if (raw.schema_version !== 1 || !Array.isArray(raw.assets)) {
      throw new Error('Invalid image manifest')
    }
    const assets = raw.assets
      .filter(
        (asset): asset is ImageAsset =>
          isLocalImagePath(asset?.path) &&
          typeof asset.bytes === 'number' &&
          Number.isFinite(asset.bytes) &&
          asset.bytes >= 0,
      )
      .map((asset) => ({ path: asset.path, bytes: asset.bytes }))
    if (!assets.length) throw new Error('Image manifest is empty')
    return {
      schema_version: 1,
      count: assets.length,
      total_bytes: assets.reduce((sum, asset) => sum + asset.bytes, 0),
      assets,
    }
  } catch {
    if (signal.aborted) throw new DOMException('Aborted', 'AbortError')
    return imageManifestFromCatalogs(catalog, mirrors, signal)
  }
}
