import { useEffect, useSyncExternalStore } from 'react'
import {
  ensureAutoImageDownload,
  getImageDownloaderSnapshot,
  pauseImageDownload,
  resumeImageDownload,
  retryImageDownload,
  startImageDownload,
  subscribeImageDownloader,
} from '../imageDownloader'
import type { CarsPayload, MirrorCatalog } from '../types'
import '../downloader.css'

function formatBytes(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '—'
  const units = ['B', 'KB', 'MB', 'GB']
  let amount = value
  let unit = 0
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024
    unit += 1
  }
  return `${amount.toFixed(unit ? 1 : 0)} ${units[unit]}`
}

function phaseText(phase: string) {
  switch (phase) {
    case 'preparing':
      return '正在准备缓存清单'
    case 'scanning':
      return '正在扫描浏览器缓存'
    case 'downloading':
      return '正在下载图片'
    case 'paused':
      return '下载已暂停'
    case 'complete':
      return '全部图片已离线缓存'
    case 'complete-with-errors':
      return '部分图片下载失败'
    case 'error':
      return '离线缓存启动失败'
    default:
      return '等待开始'
  }
}

export function useImageDownloader() {
  return useSyncExternalStore(
    subscribeImageDownloader,
    getImageDownloaderSnapshot,
    getImageDownloaderSnapshot,
  )
}

export function ImageDownloader({
  catalog,
  mirrors = {},
  autoStart = true,
  onBack,
}: {
  catalog: CarsPayload
  mirrors?: Record<string, MirrorCatalog>
  autoStart?: boolean
  onBack?: () => void
}) {
  const downloader = useImageDownloader()
  const { phase, manifest, stats, error, unsupported, persisted, quota } =
    downloader
  const percent =
    stats.total > 0 ? Math.min(100, (stats.processed / stats.total) * 100) : 0

  useEffect(() => {
    if (autoStart) void ensureAutoImageDownload(catalog, mirrors)
  }, [autoStart, catalog, mirrors])

  return (
    <section className="image-downloader-panel" aria-label="图片离线缓存">
      <div className="image-downloader-heading">
        <div>
          <span className="download-eyebrow">OFFLINE IMAGE CACHE</span>
          <h2>下载全部图片</h2>
          <p>
            自动使用最多 {24} 路并发下载。已经缓存的图片会跳过，中断后再次运行只补缺失项。
          </p>
        </div>
        {onBack && (
          <button type="button" onClick={onBack}>
            返回车辆
          </button>
        )}
      </div>

      <div className="image-downloader-status">
        <strong>{phaseText(phase)}</strong>
        <span>{percent.toFixed(1)}%</span>
      </div>
      <div className="image-downloader-progress" aria-label={phaseText(phase)}>
        <span style={{ width: `${percent}%` }} />
      </div>

      <div className="image-downloader-stats">
        <span><b>{stats.total.toLocaleString()}</b>总图片</span>
        <span><b>{stats.cached.toLocaleString()}</b>缓存命中</span>
        <span><b>{stats.downloaded.toLocaleString()}</b>本次下载</span>
        <span><b>{stats.failed.toLocaleString()}</b>失败</span>
        <span><b>{Math.max(0, stats.total - stats.processed).toLocaleString()}</b>剩余</span>
        <span><b>{formatBytes(stats.downloadedBytes)}</b>本次流量</span>
        <span><b>{formatBytes(manifest?.total_bytes || 0)}</b>清单体积</span>
      </div>

      {stats.currentPath && (
        <p className="image-downloader-current">
          <span>当前</span>
          <code>{stats.currentPath}</code>
        </p>
      )}

      <p className="image-downloader-storage">
        {persisted ? '持久缓存已启用' : '浏览器使用尽力缓存'}
        {quota > 0 ? ` · 可用配额约 ${formatBytes(quota)}` : ''}
      </p>

      {error && (
        <p className="image-downloader-error">
          {unsupported ? '当前浏览器不支持 Cache Storage 或 Service Worker。' : error}
        </p>
      )}

      <div className="image-downloader-actions">
        {phase === 'downloading' || phase === 'scanning' || phase === 'preparing' ? (
          <button type="button" onClick={pauseImageDownload}>暂停下载</button>
        ) : null}
        {phase === 'paused' ? (
          <button type="button" onClick={resumeImageDownload}>继续下载</button>
        ) : null}
        {phase === 'complete-with-errors' || phase === 'error' ? (
          <button type="button" onClick={retryImageDownload}>重试缺失项</button>
        ) : null}
        {phase === 'idle' ? (
          <button type="button" onClick={() => void startImageDownload(catalog, mirrors)}>
            开始下载
          </button>
        ) : null}
      </div>
    </section>
  )
}

export function ImageCacheStatus({ onOpen }: { onOpen: () => void }) {
  const downloader = useImageDownloader()
  const active =
    downloader.phase === 'preparing' ||
    downloader.phase === 'scanning' ||
    downloader.phase === 'downloading' ||
    downloader.phase === 'paused' ||
    downloader.phase === 'complete-with-errors'
  const text =
    downloader.phase === 'complete'
      ? '图片已离线'
      : active
        ? `${downloader.stats.processed.toLocaleString()} / ${downloader.stats.total.toLocaleString()}`
        : '离线图片'
  return (
    <button type="button" className="image-cache-status" onClick={onOpen}>
      <span aria-hidden="true">{downloader.phase === 'complete' ? '✓' : '↓'}</span>
      {text}
    </button>
  )
}
