import { useEffect, useMemo, useRef, useState } from 'react'
import { Dialog } from '@base-ui/react/dialog'
import { Switch } from '@base-ui/react/switch'
import { Tabs } from '@base-ui/react/tabs'
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  ChevronLeft,
  ChevronRight,
  CirclePause,
  CirclePlay,
  Database,
  ExternalLink,
  Search,
  Sparkles,
  Trophy,
  Users,
  X,
} from 'lucide-react'
import { cn } from './lib/utils'
import './menus.css'

const BASE = import.meta.env.BASE_URL
const CATEGORY_FILTERS = [
  ['all', '全部', 'All'],
  ['main', '主线 1–39', 'Main'],
  ['bonus', '奖励 40–51', 'Bonus'],
  ['extra', '追加收藏', 'Extra'],
]

function normalize(value = '') {
  return value
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
}

function sanitizeDisplayText(value) {
  if (typeof value === 'string') {
    return value.replace(/[《》<>]/g, '').replace(/\uFFFD+/g, '')
  }
  if (Array.isArray(value)) return value.map(sanitizeDisplayText)
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, sanitizeDisplayText(item)]),
    )
  }
  return value
}

function readLang() {
  const value = new URLSearchParams(location.search).get('lang')
  if (['zh', 'en', 'both'].includes(value)) return value
  const stored = localStorage.getItem('gt7cafe.lang')
  return ['zh', 'en', 'both'].includes(stored) ? stored : 'both'
}

function findRequestedRecord(records) {
  const value = new URLSearchParams(location.search).get('book')
  if (!value) return null
  const exact = records.find((record) => record.id === value)
  if (exact) return exact
  const numeric = Number(value.replace(/^em/i, ''))
  if (Number.isFinite(numeric)) {
    return (
      records.find(
        (record) => record.category === 'extra' && record.number === numeric,
      ) ||
      records.find((record) => record.number === numeric) ||
      null
    )
  }
  return null
}

function DisplayText({ lang, zh, en, className, zhClassName, enClassName }) {
  if (lang === 'both') {
    return (
      <span className={className}>
        <span className={cn('block', zhClassName)}>{zh || en}</span>
        <span className={cn('mt-1 block text-[0.82em] opacity-60', enClassName)}>
          {en || zh}
        </span>
      </span>
    )
  }
  return <span className={className}>{lang === 'zh' ? zh || en : en || zh}</span>
}

function CategoryBadge({ record, lang }) {
  const tone =
    record.category === 'main'
      ? 'border-cyan-300/25 bg-cyan-300/10 text-cyan-100'
      : record.category === 'bonus'
        ? 'border-amber-300/25 bg-amber-300/10 text-amber-100'
        : 'border-fuchsia-300/25 bg-fuchsia-300/10 text-fuchsia-100'
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border px-2.5 py-1 text-[10px] font-semibold tracking-[0.16em] uppercase',
        tone,
      )}
    >
      {lang === 'zh' ? record.categoryZh : record.categoryEn}
    </span>
  )
}

function CoverArtwork({ record, lang, eager = false }) {
  const cars = record.cars?.slice(0, 3) || []
  if (record.cover) {
    return (
      <div className="menu-cover relative isolate aspect-video overflow-hidden rounded-[22px] border border-white/10 bg-[#090d14]">
        <img
          src={`${BASE}${record.cover}`}
          alt={record.collectionNameEn}
          className="absolute inset-0 h-full w-full object-cover"
          loading={eager ? 'eager' : 'lazy'}
          fetchPriority={eager ? 'high' : 'auto'}
          decoding="async"
        />
        <div className="absolute inset-0 bg-gradient-to-t from-black/15 to-transparent" />
        <div className="absolute top-4 right-4 left-4 flex items-center justify-between gap-2">
          <span className="rounded-full border border-white/20 bg-black/60 px-2.5 py-1 text-[9px] font-semibold tracking-[0.12em] text-white/75 uppercase backdrop-blur">
            {lang === 'zh' ? '\u6e38\u620f\u5185\u5c01\u9762' : 'In-game cover'}
          </span>
          {String(record.runtime || '').startsWith('online') && (
            <span className="rounded-full border border-amber-200/25 bg-amber-300/15 px-2.5 py-1 text-[9px] font-semibold tracking-[0.1em] text-amber-50 uppercase backdrop-blur">
              {lang === 'zh' ? '1.72 \u65b0\u589e' : '1.72 new'}
            </span>
          )}
        </div>
      </div>
    )
  }

  return (
    <div
      className={cn(
        'menu-cover relative isolate overflow-hidden rounded-[22px] border border-white/10 bg-[#090d14]',
        'aspect-video',
      )}
    >
      <div className="menu-cover-rays" aria-hidden="true" />
      {cars.length ? (
        <div className={cn('absolute inset-0 grid', vehiclesGridClass(cars.length))}>
          {cars.map((car, index) => (
            <div
              key={`${record.id}-${car.id || index}`}
              className="menu-cover-car relative flex min-h-0 items-center justify-center overflow-hidden"
            >
              {car.image ? (
                <img
                  src={`${BASE}${car.image}`}
                  alt={car.nameEn}
                  className="h-full w-full object-contain p-[4%] drop-shadow-[0_10px_18px_rgba(0,0,0,0.8)]"
                  loading={eager ? 'eager' : 'lazy'}
                  decoding="async"
                />
              ) : (
                <span className="text-3xl font-black text-white/15">{index + 1}</span>
              )}
            </div>
          ))}
        </div>
      ) : (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="text-center">
            <span className="block text-[clamp(3rem,14vw,8rem)] font-black leading-none tracking-[-0.08em] text-white/[0.07]">
              {record.category === 'extra' ? `EM${record.number}` : record.number}
            </span>
            <span className="mt-2 block text-[10px] font-bold tracking-[0.3em] text-white/30 uppercase">
              GT Cafe
            </span>
          </div>
        </div>
      )}
      <div className="absolute inset-0 bg-gradient-to-t from-[#05070b] via-[#05070b]/10 to-transparent" />
      <div className="absolute inset-y-0 left-0 w-[9px] bg-gradient-to-r from-black/80 to-transparent" />
      <div className="absolute inset-y-3 left-3 w-px bg-white/15" />
      <div className="absolute top-4 right-4 flex items-center gap-2">
        <span className="rounded-full border border-white/15 bg-black/55 px-2.5 py-1 text-[9px] font-semibold tracking-[0.12em] text-white/65 uppercase backdrop-blur">
          {record.coverStatus === 'container-only'
            ? lang === 'zh'
              ? '封面源已找到'
              : 'Cover source found'
            : lang === 'zh'
              ? '无独立封面'
              : 'No cover asset'}
        </span>
      </div>
      <div className="absolute right-5 bottom-4 left-7">
        <div className="mb-2 flex items-center gap-2 text-[10px] font-bold tracking-[0.18em] text-white/55 uppercase">
          <span>{record.displayNumber}</span>
          <span className="h-px w-8 bg-white/20" />
          <span>{record.internalCode}</span>
        </div>
        <div className="line-clamp-2 text-[clamp(1rem,2.4vw,1.7rem)] font-black leading-tight text-white">
          {lang === 'en' ? record.collectionNameEn : record.collectionNameZh}
          {lang === 'both' && (
            <span className="mt-1 block text-[0.56em] font-semibold tracking-[-0.01em] text-white/48">
              {record.collectionNameEn}
            </span>
          )}
        </div>
      </div>
    </div>
  )
}

function vehiclesGridClass(count) {
  if (count === 1) return 'grid-cols-1 p-[12%]'
  if (count === 2) return 'grid-cols-2 -rotate-3 scale-110 p-[6%]'
  return 'grid-cols-3 -rotate-2 scale-110 p-[3%]'
}

function BrandPill({ brand, lang }) {
  return (
    <span className="rounded-full border border-white/10 bg-white/[0.055] px-2.5 py-1 text-[11px] font-medium text-white/65">
      {brand || (lang === 'zh' ? '未知厂商' : 'Unknown')}
    </span>
  )
}

function CarTile({ car, lang }) {
  return (
    <article className="group/car overflow-hidden rounded-2xl border border-white/10 bg-white/[0.035]">
      <div className="aspect-[16/9] bg-gradient-to-br from-white/[0.06] to-transparent p-2">
        {car.image ? (
          <img
            src={`${BASE}${car.image}`}
            alt={car.nameEn}
            className="h-full w-full object-contain transition duration-300 group-hover/car:scale-[1.035]"
            loading="lazy"
            decoding="async"
          />
        ) : (
          <div className="flex h-full items-center justify-center text-xs text-white/30">
            {lang === 'zh' ? '车辆图片未找到' : 'Car image unavailable'}
          </div>
        )}
      </div>
      <div className="border-t border-white/[0.07] px-3 py-2.5">
        <div className="text-[10px] font-semibold tracking-[0.13em] text-cyan-200/70 uppercase">
          {car.brand}
        </div>
        <div className="mt-0.5 line-clamp-2 text-[13px] font-semibold leading-snug text-white/90">
          {lang === 'en' ? car.nameEn : car.nameZh || car.nameEn}
          {lang === 'both' && car.nameZh && car.nameZh !== car.nameEn && (
            <span className="mt-0.5 block text-[10px] font-medium text-white/35">
              {car.nameEn}
            </span>
          )}
        </div>
      </div>
    </article>
  )
}

function MenuCoverSkeleton() {
  return <div className="menu-cover-skeleton aspect-[4/3] animate-pulse rounded-[22px] bg-white/[0.05]" />
}

function MenuBookDialog({
  record,
  records,
  lang,
  open,
  onOpenChange,
  onStep,
  playing,
  onPlayChange,
  onLangChange,
}) {
  const [tab, setTab] = useState('overview')
  const index = records.findIndex((item) => item.id === record?.id)

  useEffect(() => {
    if (open) setTab('overview')
  }, [record?.id, open])

  if (!record) return null

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Backdrop className="menu-dialog-backdrop" />
        <Dialog.Popup className="menu-dialog-popup" aria-describedby={undefined}>
          <div className="menu-dialog-shell">
            <header className="menu-dialog-header">
              <div className="flex min-w-0 items-center gap-3">
                <span className="grid size-10 shrink-0 place-items-center rounded-xl border border-white/10 bg-white/[0.06] text-sm font-black text-cyan-200">
                  {record.category === 'extra' ? `EM${record.number}` : record.number}
                </span>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <CategoryBadge record={record} lang={lang} />
                    <span className="text-[10px] font-semibold tracking-[0.16em] text-white/35">
                      {record.internalCode}
                    </span>
                  </div>
                  <Dialog.Title className="mt-1 truncate text-sm font-semibold text-white/85">
                    {lang === 'both'
                      ? `${record.titleZh} · ${record.titleEn}`
                      : lang === 'zh'
                        ? record.titleZh
                        : record.titleEn}
                  </Dialog.Title>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                <div className="menu-language-switch" role="group" aria-label="Language">
                  {[
                    ['zh', '\u4e2d'],
                    ['en', 'EN'],
                    ['both', '\u53cc'],
                  ].map(([value, label]) => (
                    <button
                      key={value}
                      type="button"
                      data-active={lang === value}
                      onClick={() => onLangChange(value)}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                <button
                  type="button"
                  className="menu-icon-button"
                  onClick={() => onPlayChange(!playing)}
                  aria-label={playing ? '暂停自动播放' : '开始自动播放'}
                >
                  {playing ? <CirclePause size={18} /> : <CirclePlay size={18} />}
                </button>
                <Dialog.Close className="menu-icon-button" aria-label="关闭">
                  <X size={20} />
                </Dialog.Close>
              </div>
            </header>

            <div className="menu-dialog-scroll">
              <section className="grid gap-6 lg:grid-cols-[minmax(270px,0.8fr)_minmax(0,1.2fr)]">
                <CoverArtwork record={record} lang={lang} large eager />
                <div className="flex min-w-0 flex-col">
                  <div className="mb-3 flex flex-wrap items-center gap-2">
                    <CategoryBadge record={record} lang={lang} />
                    <span className="text-[11px] font-semibold tracking-[0.15em] text-white/35">
                      {record.displayNumber} / {record.internalCode}
                    </span>
                  </div>
                  <h2 className="text-[clamp(1.6rem,5vw,3.2rem)] font-black leading-[0.98] tracking-[-0.045em] text-white">
                    {lang === 'en'
                      ? record.collectionNameEn
                      : record.collectionNameZh}
                  </h2>
                  {lang === 'both' && (
                    <p className="mt-2 text-lg font-semibold tracking-[-0.02em] text-white/38">
                      {record.collectionNameEn}
                    </p>
                  )}
                  <DisplayText
                    lang={lang}
                    zh={record.titleZh}
                    en={record.titleEn}
                    className="mt-3 block max-w-2xl text-sm leading-6 text-white/55"
                  />
                  <div className="mt-5 flex flex-wrap gap-2">
                    {record.manufacturers.length ? (
                      record.manufacturers.map((brand) => (
                        <BrandPill key={brand} brand={brand} lang={lang} />
                      ))
                    ) : (
                      <BrandPill lang={lang} />
                    )}
                  </div>
                  <div className="mt-auto grid gap-2 pt-6 sm:grid-cols-3">
                    <div className="menu-stat-box">
                      <Users size={15} />
                      <span>{record.cars.length || '—'}</span>
                      <small>{lang === 'zh' ? '车辆' : 'Cars'}</small>
                    </div>
                    <div className="menu-stat-box">
                      <Trophy size={15} />
                      <span>{record.collectorLevel || '—'}</span>
                      <small>{lang === 'zh' ? '收藏等级' : 'Collector'}</small>
                    </div>
                    <div className="menu-stat-box">
                      <Sparkles size={15} />
                      <span>{record.category === 'extra' ? 'EM' : 'MB'}</span>
                      <small>{lang === 'zh' ? '分类' : 'Type'}</small>
                    </div>
                  </div>
                </div>
              </section>

              <Tabs.Root
                value={tab}
                onValueChange={setTab}
                className="mt-7"
              >
                <Tabs.List className="menu-tabs-list" aria-label="菜单书详情">
                  <Tabs.Tab value="overview" className="menu-tab">
                    {lang === 'zh' ? '概览' : 'Overview'}
                  </Tabs.Tab>
                  <Tabs.Tab value="cars" className="menu-tab">
                    {lang === 'zh' ? '车辆与厂商' : 'Cars & makes'}
                  </Tabs.Tab>
                  <Tabs.Tab value="text" className="menu-tab">
                    {lang === 'zh' ? '游戏文字' : 'Game text'}
                  </Tabs.Tab>
                  <Tabs.Tab value="source" className="menu-tab">
                    {lang === 'zh' ? '数据来源' : 'Sources'}
                  </Tabs.Tab>
                </Tabs.List>

                <Tabs.Panel value="overview" className="menu-tab-panel">
                  <div className="grid gap-4 md:grid-cols-2">
                    <article className="menu-copy-card md:col-span-2">
                      <div className="menu-section-kicker">
                        <BookOpen size={14} />
                        {lang === 'zh' ? '菜单书说明' : 'Menu Book description'}
                      </div>
                      <DisplayText
                        lang={lang}
                        zh={record.descriptionZh}
                        en={record.descriptionEn}
                        className="mt-3 block text-sm leading-7 text-white/72"
                      />
                    </article>
                    <article className="menu-copy-card menu-story-card md:col-span-2">
                      <div className="menu-story-heading">
                        <div className="menu-section-kicker">
                          <BookOpen size={14} />
                          {lang === 'zh' ? '菜单书故事' : 'Menu Book story'}
                        </div>
                        <span>
                          {record.storyTextIdZh || record.storyTextIdEn ||
                            (lang === 'zh' ? '更新摘要' : 'Update summary')}
                        </span>
                      </div>
                      <div className="menu-story-copy">
                        {lang !== 'en' && (
                          <p className="whitespace-pre-line">{record.storyZh || record.descriptionZh}</p>
                        )}
                        {lang !== 'zh' && (
                          <p className="menu-story-en whitespace-pre-line">
                            {record.storyEn || record.descriptionEn}
                          </p>
                        )}
                      </div>
                    </article>
                    <article className="menu-copy-card">
                      <div className="menu-section-kicker">
                        <Trophy size={14} />
                        {lang === 'zh' ? '完成奖励' : 'Reward'}
                      </div>
                      <p className="mt-3 text-base font-semibold text-amber-100/90">
                        <DisplayText
                          lang={lang}
                          zh={record.rewardZh}
                          en={record.rewardEn}
                        />
                      </p>
                    </article>
                    <article className="menu-copy-card">
                      <div className="menu-section-kicker">
                        <Users size={14} />
                        {lang === 'zh' ? '收藏条件' : 'Collection requirement'}
                      </div>
                      <p className="mt-3 text-sm leading-6 text-white/70">
                        <DisplayText
                          lang={lang}
                          zh={record.requirementsZh}
                          en={record.requirementsEn}
                        />
                      </p>
                    </article>
                  </div>
                </Tabs.Panel>

                <Tabs.Panel value="cars" className="menu-tab-panel">
                  {record.cars.length ? (
                    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                      {record.cars.map((car, carIndex) => (
                        <CarTile
                          key={`${record.id}-${car.id}-${carIndex}`}
                          car={car}
                          lang={lang}
                        />
                      ))}
                    </div>
                  ) : (
                    <div className="menu-empty-card">
                      <Users size={24} />
                      <strong>
                        {lang === 'zh'
                          ? '这是赛事或功能型菜单书'
                          : 'This is an event or activity Menu Book'}
                      </strong>
                      <span>
                        {lang === 'zh'
                          ? '没有固定收藏车型，查看“游戏文字”中的完成条件。'
                          : 'It has no fixed collection cars. See Game text for its completion condition.'}
                      </span>
                    </div>
                  )}
                </Tabs.Panel>

                <Tabs.Panel value="text" className="menu-tab-panel">
                  <div className="grid gap-4 lg:grid-cols-2">
                    {lang !== 'en' && (
                      <article className="menu-copy-card">
                        <div className="menu-section-kicker">
                          <Database size={14} />
                          中文游戏文本
                        </div>
                        <p className="mt-3 whitespace-pre-line text-sm leading-7 text-white/72">
                          {record.storyZh || record.descriptionZh}
                        </p>
                      </article>
                    )}
                    {lang !== 'zh' && (
                      <article className="menu-copy-card">
                        <div className="menu-section-kicker">
                          <Database size={14} />
                          English game text
                        </div>
                        <p className="mt-3 whitespace-pre-line text-sm leading-7 text-white/72">
                          {record.storyEn || record.descriptionEn}
                        </p>
                      </article>
                    )}
                  </div>
                </Tabs.Panel>

                <Tabs.Panel value="source" className="menu-tab-panel">
                  <div className="menu-copy-card">
                    <div className="menu-section-kicker">
                      <Database size={14} />
                      {lang === 'zh' ? '本地 PS5 数据定位' : 'Local PS5 source IDs'}
                    </div>
                    <dl className="mt-4 grid gap-3 text-xs sm:grid-cols-2">
                      {Object.entries(record.sources || {}).map(([key, value]) => (
                        <div key={key} className="rounded-xl border border-white/[0.07] bg-black/20 p-3">
                          <dt className="font-semibold tracking-[0.08em] text-white/35 uppercase">
                            {key}
                          </dt>
                          <dd className="mt-1 break-all font-mono text-[11px] leading-5 text-cyan-100/75">
                            {Array.isArray(value) ? value.join(', ') : String(value || '—')}
                          </dd>
                        </div>
                      ))}
                      <div className="rounded-xl border border-white/[0.07] bg-black/20 p-3 sm:col-span-2">
                        <dt className="font-semibold tracking-[0.08em] text-white/35 uppercase">
                          coverAsset
                        </dt>
                        <dd className="mt-1 break-all font-mono text-[11px] leading-5 text-fuchsia-100/70">
                          {record.coverAsset || '—'}
                        </dd>
                      </div>
                    </dl>
                    <p className="mt-4 text-xs leading-5 text-white/38">
                      {lang === 'zh'
                        ? '封面 JXL 容器已从 PS5 内容库精确定位；当前公共网页以游戏内车辆图生成同构图封面。若后续兼容游戏专用 JXL 流，可直接替换为原始封面。'
                        : 'The original JXL cover container is resolved in the PS5 store. This public build reconstructs the cover layout from game car thumbnails until the proprietary JXL stream is decodable.'}
                    </p>
                  </div>
                </Tabs.Panel>
              </Tabs.Root>
            </div>

            <div className="menu-dialog-bottom">
              <button
                type="button"
                className="menu-step-button"
                onClick={() => onStep(-1)}
              >
                <ChevronLeft size={18} />
                <span className="hidden sm:inline">
                  {lang === 'zh' ? '上一菜单' : 'Previous'}
                </span>
              </button>
              <div className="min-w-0 flex-1 text-center">
                <div className="truncate text-xs font-semibold text-white/80">
                  {lang === 'zh'
                    ? record.collectionNameZh
                    : record.collectionNameEn}
                </div>
                <div className="mt-0.5 text-[10px] tracking-[0.12em] text-white/35">
                  {index + 1} / {records.length}
                </div>
              </div>
              <button
                type="button"
                className="menu-step-button"
                onClick={() => onStep(1)}
              >
                <span className="hidden sm:inline">
                  {lang === 'zh' ? '下一菜单' : 'Next'}
                </span>
                <ChevronRight size={18} />
              </button>
            </div>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

export default function MenuBooksApp() {
  const [data, setData] = useState(null)
  const [error, setError] = useState('')
  const [lang, setLangState] = useState(readLang)
  const [category, setCategory] = useState('all')
  const [brandLetter, setBrandLetter] = useState('ALL')
  const [query, setQuery] = useState('')
  const [activeId, setActiveId] = useState(null)
  const [lastViewedId, setLastViewedId] = useState(null)
  const [playing, setPlaying] = useState(false)
  const searchRef = useRef(null)

  useEffect(() => {
    fetch(`${BASE}data/menu_books.json`)
      .then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.json()
      })
      .then((payload) => {
        const sanitized = sanitizeDisplayText(payload)
        setData(sanitized)
        const requested = findRequestedRecord(sanitized.records)
        if (requested) {
          setActiveId(requested.id)
          setLastViewedId(requested.id)
        }
      })
      .catch((reason) => setError(reason.message || String(reason)))
  }, [])

  const records = data?.records || []
  const active = records.find((record) => record.id === activeId) || null
  const activeIndex = Math.max(
    0,
    records.findIndex((record) => record.id === activeId),
  )

  const brandLetters = useMemo(() => {
    const letters = new Set()
    for (const record of records) {
      for (const brand of record.manufacturers || []) {
        const letter = brand?.trim()?.[0]?.toUpperCase()
        if (letter) letters.add(letter)
      }
    }
    return [...letters].sort()
  }, [records])

  const filtered = useMemo(() => {
    const needle = normalize(query.trim())
    return records.filter((record) => {
      if (category !== 'all' && record.category !== category) return false
      if (
        brandLetter !== 'ALL' &&
        !record.manufacturers.some((brand) =>
          brand.toUpperCase().startsWith(brandLetter),
        )
      ) {
        return false
      }
      if (!needle) return true
      return normalize(
        [
          record.titleEn,
          record.titleZh,
          record.collectionNameEn,
          record.collectionNameZh,
          record.descriptionEn,
          record.descriptionZh,
          record.storyEn,
          record.storyZh,
          record.rewardEn,
          record.rewardZh,
          record.internalCode,
          ...record.manufacturers,
          ...record.cars.flatMap((car) => [car.nameEn, car.nameZh, car.brand]),
        ].join(' '),
      ).includes(needle)
    })
  }, [records, category, brandLetter, query])

  function setLang(value) {
    setLangState(value)
    localStorage.setItem('gt7cafe.lang', value)
    const url = new URL(location.href)
    url.searchParams.set('lang', value)
    history.replaceState(null, '', url)
  }

  function rememberBook(record) {
    const url = new URL(location.href)
    if (record) url.searchParams.set('book', record.id)
    else url.searchParams.delete('book')
    history.replaceState(null, '', url)
  }

  function openRecord(record) {
    setActiveId(record.id)
    setLastViewedId(record.id)
    rememberBook(record)
  }

  function closeRecord() {
    setActiveId(null)
    rememberBook(null)
  }

  function step(delta) {
    if (!records.length) return
    const anchor = active || records.find((record) => record.id === lastViewedId)
    const index = anchor
      ? records.findIndex((record) => record.id === anchor.id)
      : -1
    const nextIndex =
      index < 0
        ? delta > 0
          ? 0
          : records.length - 1
        : (index + delta + records.length) % records.length
    openRecord(records[nextIndex])
  }

  function changePlaying(value) {
    if (value && !active && records.length) {
      openRecord(records[0])
    }
    setPlaying(value)
  }

  useEffect(() => {
    if (!playing || !active) return undefined
    let timer
    const syncTimer = () => {
      window.clearInterval(timer)
      timer = undefined
      if (document.visibilityState === 'visible') {
        timer = window.setInterval(() => step(1), 5200)
      }
    }
    syncTimer()
    document.addEventListener('visibilitychange', syncTimer)
    return () => {
      document.removeEventListener('visibilitychange', syncTimer)
      window.clearInterval(timer)
    }
  }, [playing, activeId, records.length])

  useEffect(() => {
    function onKeyDown(event) {
      const editing = ['INPUT', 'TEXTAREA', 'SELECT'].includes(
        event.target?.tagName,
      )
      if (editing && event.key !== 'Escape') return
      if (event.key === 'Escape' && active) {
        closeRecord()
        return
      }
      if (event.key === '/' && !editing) {
        event.preventDefault()
        searchRef.current?.focus()
        return
      }
      if (
        event.key === ' ' &&
        !editing &&
        !['BUTTON', 'A'].includes(event.target?.tagName)
      ) {
        event.preventDefault()
        changePlaying(!playing)
        return
      }
      if (['ArrowRight', 'ArrowDown', 'PageDown'].includes(event.key)) {
        event.preventDefault()
        step(1)
      } else if (['ArrowLeft', 'ArrowUp', 'PageUp'].includes(event.key)) {
        event.preventDefault()
        step(-1)
      } else if (!active && event.key === 'Enter') {
        event.preventDefault()
        if (records[0]) openRecord(records[0])
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [active, records, playing])

  const counts = {
    ...data?.counts,
    withCoverContainer:
      data?.counts?.withAnyCover ??
      data?.counts?.withDisplayCover ??
      data?.counts?.withCoverContainer,
  }
  const current =
    active ||
    records.find((record) => record.id === lastViewedId) ||
    records[activeIndex]

  return (
    <div className="menu-page min-h-screen bg-[#f5f5f7] text-[#17171a] selection:bg-cyan-300/25">
      <div className="pointer-events-none fixed inset-0 menu-page-atmosphere" />
      <div className="relative mx-auto min-h-screen max-w-[1500px] px-4 pb-32 sm:px-6 lg:px-10">
        <header className="sticky top-0 z-40 -mx-4 border-b border-black/[0.07] bg-white/90 px-4 py-3 backdrop-blur-xl sm:-mx-6 sm:px-6 lg:-mx-10 lg:px-10">
          <div className="mx-auto flex max-w-[1420px] items-center gap-3">
            <a
              href="/"
              className="group flex min-w-0 items-center gap-3"
              aria-label="返回 GT7 研究站"
            >
              <span className="grid size-11 place-items-center rounded-2xl bg-white text-sm font-black tracking-[-0.08em] text-black shadow-[0_0_30px_rgba(255,255,255,0.14)]">
                GT7
              </span>
              <span className="hidden min-w-0 sm:block">
                <span className="block truncate text-sm font-bold tracking-[-0.02em] text-white">
                  Café Menu Archive
                </span>
                <span className="mt-0.5 block text-[10px] tracking-[0.18em] text-white/35 uppercase">
                  PS5 1.710.000 · DMSB + RT2
                </span>
              </span>
            </a>
            <div className="ml-auto flex items-center gap-2">
              <label className="hidden items-center gap-2 rounded-xl border border-white/10 bg-white/[0.045] px-3 py-2 md:flex">
                <Search size={14} className="text-white/35" />
                <input
                  ref={searchRef}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  className="w-44 bg-transparent text-xs text-white outline-none placeholder:text-white/28 lg:w-56"
                  placeholder={lang === 'zh' ? '搜索菜单、车辆、厂商' : 'Search menus, cars, makes'}
                />
                <kbd className="rounded border border-white/10 px-1.5 py-0.5 text-[9px] text-white/30">
                  /
                </kbd>
              </label>
              <div className="menu-language-switch" role="group" aria-label="语言">
                {[
                  ['zh', '中'],
                  ['en', 'EN'],
                  ['both', '双语'],
                ].map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    data-active={lang === value}
                    onClick={() => setLang(value)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </header>

        <main className="pt-8 sm:pt-12">
          <section className="relative overflow-hidden rounded-[28px] border border-white/[0.08] bg-white/[0.025] px-5 py-7 sm:px-8 sm:py-10 lg:px-12">
            <div className="absolute -top-32 right-[-8%] size-80 rounded-full bg-cyan-400/[0.08] blur-3xl" />
            <div className="relative grid gap-8 lg:grid-cols-[1fr_auto] lg:items-end">
              <div>
                <div className="flex items-center gap-2 text-[11px] font-semibold tracking-[0.22em] text-cyan-200/70 uppercase">
                  <BookOpen size={15} />
                  GT Café Menu Books
                </div>
                <h1 className="mt-4 max-w-4xl text-[clamp(2.5rem,7vw,6.4rem)] font-black leading-[0.86] tracking-[-0.065em]">
                  {lang === 'zh' ? (
                    <>
                      菜单书
                      <span className="text-white/24">档案</span>
                    </>
                  ) : lang === 'en' ? (
                    <>
                      Menu Book
                      <span className="text-white/24"> archive</span>
                    </>
                  ) : (
                    <>
                      菜单书
                      <span className="text-white/24"> / Menu Book</span>
                    </>
                  )}
                </h1>
                <p className="mt-5 max-w-2xl text-sm leading-6 text-white/48 sm:text-base">
                  {lang === 'zh'
                    ? '主线、奖励赛与 Extra Menu 收藏，统一展示车辆、厂商、奖励、收藏等级与游戏内中英文原文。'
                    : lang === 'en'
                      ? 'Main, Bonus and Extra Menu collections with cars, manufacturers, rewards, collector levels and both game-language texts.'
                      : '主线、奖励赛与 Extra Menu 收藏 / Main, Bonus and Extra collections — cars, makes, rewards and bilingual game text.'}
                </p>
              </div>
              <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-2">
                {[
                  [counts.total || '—', '菜单书', 'Menus'],
                  [counts.withCars || '—', '含车辆', 'With cars'],
                  [counts.withCoverContainer || '—', '封面源', 'Covers found'],
                  [counts.resolvedCars || '—', '车辆映射', 'Cars mapped'],
                ].map(([value, zh, en]) => (
                  <div key={en} className="menu-hero-stat">
                    <b>{value}</b>
                    <span>{lang === 'zh' ? zh : en}</span>
                  </div>
                ))}
              </div>
            </div>
          </section>

          <section className="mt-6 grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto]">
            <div className="flex items-center gap-2 overflow-x-auto pb-1">
              {CATEGORY_FILTERS.map(([value, zh, en]) => (
                <button
                  key={value}
                  type="button"
                  className="menu-filter-chip shrink-0"
                  data-active={category === value}
                  onClick={() => setCategory(value)}
                >
                  {lang === 'zh' ? zh : en}
                </button>
              ))}
              <span className="mx-1 h-6 w-px shrink-0 bg-white/10" />
              <button
                type="button"
                className="menu-letter-chip shrink-0"
                data-active={brandLetter === 'ALL'}
                onClick={() => setBrandLetter('ALL')}
              >
                {lang === 'zh' ? '全厂商' : 'All makes'}
              </button>
              {brandLetters.map((letter) => (
                <button
                  key={letter}
                  type="button"
                  className="menu-letter-chip shrink-0"
                  data-active={brandLetter === letter}
                  onClick={() => setBrandLetter(letter)}
                >
                  {letter}
                </button>
              ))}
            </div>
            <div className="flex items-center gap-2 lg:justify-end">
              <Switch.Root
                checked={playing}
                onCheckedChange={changePlaying}
                className="menu-switch"
                aria-label={lang === 'zh' ? '自动播放' : 'Autoplay'}
              >
                <Switch.Thumb className="menu-switch-thumb" />
              </Switch.Root>
              <span className="text-xs font-medium text-white/55">
                {lang === 'zh' ? '自动播放' : 'Autoplay'}
              </span>
              <span className="text-[10px] text-white/25">5.2s</span>
            </div>
          </section>

          <div className="mt-5 flex items-center justify-between gap-3">
            <div className="text-xs text-white/35">
              {lang === 'zh'
                ? `显示 ${filtered.length} / ${records.length} 本`
                : `${filtered.length} of ${records.length} menu books`}
            </div>
            <div className="hidden items-center gap-2 text-[10px] text-white/25 sm:flex">
              <span>← →</span>
              {lang === 'zh' ? '切换' : 'Navigate'}
              <span>·</span>
              <span>Space</span>
              {lang === 'zh' ? '自动播放' : 'Autoplay'}
            </div>
          </div>

          {error ? (
            <div className="menu-empty-card mt-6">
              <Database size={26} />
              <strong>无法载入菜单书数据</strong>
              <span>{error}</span>
            </div>
          ) : !data ? (
            <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              {Array.from({ length: 8 }, (_, index) => (
                <MenuCoverSkeleton key={index} />
              ))}
            </div>
          ) : (
            <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              {filtered.map((record, index) => (
                <button
                  key={record.id}
                  type="button"
                  className="menu-book-card group text-left"
                  onClick={() => openRecord(record)}
                >
                  <CoverArtwork
                    record={record}
                    lang={lang}
                    eager={index < 4}
                  />
                  <div className="p-4">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-[10px] font-bold tracking-[0.16em] text-white/32 uppercase">
                            {record.displayNumber}
                          </span>
                          <span className="text-[9px] font-mono text-white/18">
                            {record.internalCode}
                          </span>
                        </div>
                        <h2 className="mt-2 text-lg font-bold leading-tight tracking-[-0.025em] text-white">
                          {lang === 'en'
                            ? record.collectionNameEn
                            : record.collectionNameZh}
                          {lang === 'both' && (
                            <span className="mt-1 block text-[11px] font-medium text-white/38">
                              {record.collectionNameEn}
                            </span>
                          )}
                        </h2>
                      </div>
                      <ArrowRight
                        size={16}
                        className="mt-1 shrink-0 text-white/20 transition group-hover:translate-x-1 group-hover:text-cyan-200"
                      />
                    </div>
                    <div className="mt-3 flex flex-wrap gap-1.5">
                      {(record.manufacturers.length
                        ? record.manufacturers.slice(0, 3)
                        : [lang === 'zh' ? '赛事 / 功能' : 'Event / Activity']
                      ).map((brand) => (
                        <BrandPill key={brand} brand={brand} lang={lang} />
                      ))}
                    </div>
                    <div className="mt-4 flex items-center justify-between border-t border-white/[0.07] pt-3 text-[10px] font-medium text-white/30">
                      <span>
                        {record.cars.length
                          ? lang === 'zh'
                            ? `${record.cars.length} 辆车`
                            : `${record.cars.length} cars`
                          : lang === 'zh'
                            ? '无数值收藏'
                            : 'No car set'}
                      </span>
                      <span>
                        {record.collectorLevel
                          ? `CL ${record.collectorLevel}`
                          : lang === 'zh'
                            ? '主线'
                            : 'Story'}
                      </span>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}

          {data && !filtered.length && (
            <div className="menu-empty-card mt-6">
              <Search size={24} />
              <strong>{lang === 'zh' ? '没有匹配的菜单书' : 'No matching menus'}</strong>
              <span>
                {lang === 'zh'
                  ? '试试车辆名、厂商或内部编号。'
                  : 'Try a car name, manufacturer or internal code.'}
              </span>
            </div>
          )}

          <footer className="mt-10 flex flex-col gap-3 border-t border-white/[0.07] py-6 text-[11px] text-white/28 sm:flex-row sm:items-center sm:justify-between">
            <span>
              {lang === 'zh'
                ? '数据：GT7 PS5 1.710.000 DMSB / RT2；封面源：本地内容库'
                : 'Data: GT7 PS5 1.710.000 DMSB / RT2 · Covers: local content store'}
            </span>
            <a className="inline-flex items-center gap-1.5 hover:text-white/60" href="/">
              {lang === 'zh' ? '返回车辆研究站' : 'Back to car research'}
              <ExternalLink size={12} />
            </a>
          </footer>
        </main>
      </div>

      {active && (
        <MenuBookDialog
          record={active}
          records={records}
          lang={lang}
          open={Boolean(active)}
          onOpenChange={(open) => {
            if (!open) {
              setPlaying(false)
              closeRecord()
            }
          }}
          onStep={step}
          playing={playing}
          onPlayChange={changePlaying}
          onLangChange={setLang}
        />
      )}

      <nav className="menu-mobile-nav lg:hidden" aria-label="上一菜单和下一菜单">
        <button type="button" onClick={() => step(-1)} disabled={!records.length}>
          <ArrowLeft size={18} />
          <span>{lang === 'zh' ? '上一菜单' : 'Previous'}</span>
        </button>
        <button
          type="button"
          className="min-w-0 flex-1 text-center"
          onClick={() => (current ? openRecord(current) : setPlaying(!playing))}
        >
          <small>
            {current
              ? current.displayNumber
              : lang === 'zh'
                ? '浏览菜单书'
                : 'Browse menus'}
          </small>
          <b className="mt-0.5 block truncate">
            {current
              ? lang === 'zh'
                ? current.collectionNameZh
                : current.collectionNameEn
              : lang === 'zh'
                ? '选择一本菜单书'
                : 'Choose a Menu Book'}
          </b>
        </button>
        <button type="button" onClick={() => step(1)} disabled={!records.length}>
          <span>{lang === 'zh' ? '下一菜单' : 'Next'}</span>
          <ArrowRight size={18} />
        </button>
      </nav>
    </div>
  )
}
