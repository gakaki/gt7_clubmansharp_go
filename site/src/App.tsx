import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import { Dialog } from '@base-ui/react/dialog'
import { Tabs } from '@base-ui/react/tabs'
import { Maximize2, Minimize2, Move, Pause, Play, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  MirrorFullscreen,
  type MirrorSlideCar,
  type SlideshowPosition,
} from '@/components/MirrorFullscreen'
import {
  ImageCacheStatus,
  ImageDownloader,
} from '@/components/ImageDownloader'
import { ensureAutoImageDownload } from '@/imageDownloader'
import type {
  Car,
  CarText,
  CarsPayload,
  DialogueTurn,
  MirrorCatalog,
  MirrorCar,
  VehicleCollectionLanguageTable,
} from '@/types'

const base = import.meta.env.BASE_URL
const avatarVersion = '20261024-portrait-circle-safe'

function assetUrl(path: string) {
  return path.startsWith('/') ? path : `${base}${path}`
}

function readSlideshowPosition(): SlideshowPosition | null {
  try {
    const value = JSON.parse(localStorage.getItem('gt7.carlist.lastSlideshowPosition') || 'null')
    if (typeof value?.carId === 'string' && Number.isInteger(value?.imageIndex)) {
      return value as SlideshowPosition
    }
  } catch {
    // Ignore a stale position and start from the selected car.
  }
  return null
}

const languageOptions = [
  { id: 'ar', native: 'العربية', english: 'Arabic', zh: '阿拉伯语', rtl: true },
  { id: 'cn', native: '简体中文', english: 'Chinese (Simplified)', zh: '简体中文' },
  { id: 'cz', native: 'Čeština', english: 'Czech', zh: '捷克语' },
  { id: 'de', native: 'Deutsch', english: 'German', zh: '德语' },
  { id: 'es', native: 'Español', english: 'Spanish', zh: '西班牙语' },
  { id: 'ms', native: 'Español latinoamericano', english: 'Spanish (Latin America)', zh: '拉美西班牙语' },
  { id: 'fr', native: 'Français', english: 'French', zh: '法语' },
  { id: 'gb', native: 'English (UK)', english: 'English (UK)', zh: '英语（英国）' },
  { id: 'it', native: 'Italiano', english: 'Italian', zh: '意大利语' },
  { id: 'jp', native: '日本語', english: 'Japanese', zh: '日语' },
  { id: 'kr', native: '한국어', english: 'Korean', zh: '韩语' },
  { id: 'nl', native: 'Nederlands', english: 'Dutch', zh: '荷兰语' },
  { id: 'pl', native: 'Polski', english: 'Polish', zh: '波兰语' },
  { id: 'pt', native: 'Português', english: 'Portuguese', zh: '葡萄牙语' },
  { id: 'bp', native: 'Português brasileiro', english: 'Portuguese (Brazil)', zh: '巴西葡萄牙语' },
  { id: 'ru', native: 'Русский', english: 'Russian', zh: '俄语' },
  { id: 'th', native: 'ไทย', english: 'Thai', zh: '泰语' },
  { id: 'tr', native: 'Türkçe', english: 'Turkish', zh: '土耳其语' },
  { id: 'el', native: 'Ελληνικά', english: 'Greek', zh: '希腊语' },
  { id: 'tw', native: '繁體中文', english: 'Chinese (Traditional)', zh: '繁体中文' },
  { id: 'us', native: 'English (US)', english: 'English (US)', zh: '英语（美国）' },
  { id: 'zh', native: '中文（简体）', english: 'Chinese', zh: '中文' },
] as const

const languageIds = new Set(languageOptions.map((language) => language.id))

const carTextTabs = [
  { id: 'official', labelZh: '官方车表', labelEn: 'Official car list' },
  { id: 'cafe', labelZh: '咖啡屋对话', labelEn: 'Cafe dialogue' },
  { id: 'brandCentral', labelZh: '品牌购物', labelEn: 'Brand Central' },
  { id: 'vehicleCollection', labelZh: '车辆收藏', labelEn: 'Car Collection' },
  { id: 'garageSpecs', labelZh: '车库数据', labelEn: 'Garage Specs' },
  { id: 'usedCar', labelZh: '二手车', labelEn: 'Used Cars' },
  { id: 'collection', labelZh: '收藏馆', labelEn: 'Hagerty Collection' },
] as const

const garageSpecFields = [
  { key: 'performancePoint', labelZh: '性能点', labelEn: 'Performance points' },
  { key: 'maxPower', labelZh: '最大功率 / 转速', labelEn: 'Max power / rpm' },
  { key: 'maxTorque', labelZh: '最大扭矩 / 转速', labelEn: 'Max torque / rpm' },
  { key: 'weight', labelZh: '车重', labelEn: 'Weight' },
  { key: 'driveTrain', labelZh: '驱动形式', labelEn: 'Drivetrain' },
  { key: 'aspirationLong', labelZh: '进气形式', labelEn: 'Aspiration' },
  { key: 'displacement', labelZh: '排量', labelEn: 'Displacement' },
  { key: 'carClass', labelZh: '车辆级别', labelEn: 'Car class' },
  { key: 'length', labelZh: '长度', labelEn: 'Length' },
  { key: 'width', labelZh: '宽度', labelEn: 'Width' },
  { key: 'height', labelZh: '高度', labelEn: 'Height' },
]

const headerSpecFields = [
  garageSpecFields[0],
  garageSpecFields[1],
  garageSpecFields[3],
  garageSpecFields[4],
]

const sourceFilters = [
  { id: 'all', labelZh: '全部', labelEn: 'All' },
  { id: 'brandCentral', labelZh: '品牌店', labelEn: 'Brand Central' },
  { id: 'usedCar', labelZh: '二手车', labelEn: 'Used Cars' },
  { id: 'vehicleCollection', labelZh: '车辆收藏', labelEn: 'Car Collection' },
  { id: 'cafe', labelZh: '咖啡屋对白', labelEn: 'Cafe dialogue' },
  { id: 'hidden', labelZh: '隐藏车型', labelEn: 'Hidden cars' },
]

type LocaleCode = (typeof languageOptions)[number]['id']
type TabId = (typeof carTextTabs)[number]['id']
type GroupMode = 'brand' | 'country' | 'drive' | 'power' | 'weight'

const POWER_THRESHOLDS = [1600, 1300, 1100, 1000, 900, 800, 700, 600, 500, 400, 300, 200, 100, 0]
const WEIGHT_THRESHOLDS = [2300, 2000, 1800, 1700, 1600, 1500, 1400, 1300, 1200, 1000, 900, 800, 700, 600, 500, 0]

function numericSpec(value: unknown): number | null {
  const parsed = Number(String(value ?? '').replace(/[^\d.-]/g, ''))
  return Number.isFinite(parsed) ? parsed : null
}

function groupMetric(car: Car, mode: GroupMode): number | null {
  if (mode === 'power') {
    return numericSpec(car.specs?.zh?.power_v ?? car.specs?.en?.power_v ?? car.specs?.zh?.power ?? car.specs?.en?.power)
  }
  return numericSpec(car.specs?.zh?.weight_v ?? car.specs?.en?.weight_v ?? car.specs?.zh?.weight ?? car.specs?.en?.weight)
}

function thresholdIndex(value: number | null, thresholds: number[]): number {
  if (value === null) return thresholds.length
  const index = thresholds.findIndex((threshold) => value >= threshold)
  return index < 0 ? thresholds.length : index
}

function storedNumber(key: string, fallback: number, min: number, max: number) {
  const stored = localStorage.getItem(key)
  if (stored === null || stored === '') return fallback
  const value = Number(stored)
  return Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback
}

function sanitizeGlyphs<T>(value: T): T {
  if (typeof value === 'string') {
    const withoutReplacement = value.replace(/\uFFFD+/g, '')
    const withoutControls = [...withoutReplacement]
      .filter((character) => {
        const code = character.codePointAt(0) || 0
        return code === 9 || code === 10 || code === 13 || (code >= 32 && code !== 127)
      })
      .join('')
    return withoutControls
      .replace(/[ \t]{2,}/g, ' ')
      .replace(/ +\n/g, '\n') as T
  }
  if (Array.isArray(value)) return value.map(sanitizeGlyphs) as T
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, sanitizeGlyphs(item)]),
    ) as T
  }
  return value
}

function isChineseLocale(code: string) {
  return ['zh', 'cn', 'tw'].includes(code)
}

const mirrorSpecKeys: Record<string, string> = {
  displacement: 'displacement',
  排气量: 'displacement',
  'drive train': 'driveTrain',
  drivetrain: 'driveTrain',
  '传动系统': 'driveTrain',
  'max power': 'maxPower',
  '最大马力': 'maxPower',
  'max torque': 'maxTorque',
  '最大扭矩': 'maxTorque',
  weight: 'weight',
  车重: 'weight',
  aspiration: 'aspirationLong',
  进气形式: 'aspirationLong',
  length: 'length',
  全长: 'length',
  width: 'width',
  全宽: 'width',
  height: 'height',
  全高: 'height',
}

function officialIdFor(car: Car) {
  return car.officialId || car.officialDetails?.officialId || null
}

function manufacturerIdFor(car: Car, officialCatalog?: MirrorCatalog) {
  const localId =
    car.specs?.en?.manufacturerId ||
    car.specs?.zh?.manufacturerId ||
    car.specs?.manufacturerId
  if (localId) return String(localId)
  const officialId = officialIdFor(car)
  return officialCatalog?.cars.find((record) => record.id === officialId)
    ?.manufacturer_id
}

function carFromOfficialMirror(record: MirrorCar, code: string): Car {
  const specValues: Record<string, string | number> = {
    id: record.id,
    manufacturerId: record.manufacturer_id || '',
    carClass: record.car_class,
    performancePoint: record.performance_point,
    power: record.power,
    power_v: record.power_value ?? '',
    driveTrain: record.drive_train,
    weight_v: record.weight_value ?? '',
    nameLong: record.name_long,
    nameShort: record.model,
  }
  for (const spec of record.specs || []) {
    const normalizedLabel = spec.label.trim().toLowerCase()
    const key =
      mirrorSpecKeys[normalizedLabel] ||
      mirrorSpecKeys[spec.label.trim()]
    if (key && !specValues[key]) specValues[key] = spec.value
  }
  const countryName = record.country || ''
  const chinese = isChineseLocale(code)
  return {
    id: `official:${record.id}`,
    officialId: record.id,
    brand: record.brand || 'Gran Turismo',
    brandLogo: undefined,
    image: record.thumbnail?.replace(/^\//, ''),
    nameEn: record.name_long || record.model,
    nameZh: record.name_long || record.model,
    specs: {
      en: { ...specValues },
      zh: { ...specValues },
      officialId: record.id,
      source: 'official-car-list-mirror',
      match: 'official-only',
    },
    officialDetails: {
      officialId: record.id,
      heroEn: chinese ? record.hero : record.hero,
      heroZh: record.hero,
      descriptionEn: record.description,
      descriptionZh: record.description,
      sourceUrlEn: `https://www.gran-turismo.com/us/gt7/carlist/id/${record.id}`,
      sourceUrlZh: `https://www.gran-turismo.com/cn/gt7/carlist/id/${record.id}`,
      images: record.images.map((image) => ({ jxl: image })),
    },
    garageDetails: {
      country: {
        code: record.country_id,
        nameEn: chinese ? countryName : countryName,
        nameZh: countryName,
      },
      vehicleTypes: record.car_class
        ? [{ tagEn: `#${record.car_class}`, tagZh: `#${record.car_class}` }]
        : [],
    },
    source: 'official-only',
  } as Car
}

function mergeOfficialCars(
  localCars: Car[],
  officialCatalog: MirrorCatalog | undefined,
  code: string,
) {
  if (!officialCatalog?.cars?.length) return { cars: localCars, officialOnly: 0 }
  const normalizeName = (value: string) =>
    value.toLowerCase().replace(/[^a-z0-9]+/g, '')
  const officialIds = new Set(
    localCars.map(officialIdFor).filter((id): id is string => Boolean(id)),
  )
  const localNames = localCars.flatMap((car) => [
    normalizeName(car.nameEn),
    normalizeName(car.nameZh),
  ])
  const additions = officialCatalog.cars
    .filter((record) => {
      if (officialIds.has(record.id)) return false
      const fullName = normalizeName(record.name_long)
      const modelName = normalizeName(record.model)
      return !localNames.some(
        (localName) =>
          localName === fullName ||
          localName === modelName ||
          (modelName.length > 4 && localName.endsWith(modelName)),
      )
    })
    .map((record) => carFromOfficialMirror(record, code))
  return {
    cars: additions.length ? [...localCars, ...additions] : localCars,
    officialOnly: additions.length,
  }
}

function languageMeta(code: string): {
  id: string
  native: string
  english: string
  zh: string
  rtl?: boolean
} {
  return languageOptions.find((language) => language.id === code) || languageOptions[0]
}

function dialogueLocaleClass(code: string) {
  if (isChineseLocale(code) || ['jp', 'kr'].includes(code)) return 'cjk'
  if (code === 'ar') return 'rtl'
  return 'latin'
}

function initials(name: string) {
  return name
    .split(/\s+/)
    .map((word) => word[0])
    .join('')
    .slice(0, 2)
    .toUpperCase()
}

function avatarColor(name: string) {
  let hash = 0
  for (const character of name) hash = (hash * 31 + character.charCodeAt(0)) >>> 0
  return `hsl(${hash % 360} 42% 46%)`
}

function avatarUrl(path?: string) {
  return path ? `${base}${path}?v=${avatarVersion}` : ''
}

function dialogueDedupeKey(turn: DialogueTurn) {
  const normalize = (value: unknown) =>
    String(value || '')
      .normalize('NFKC')
      .toLowerCase()
      .replace(/[\p{P}\p{S}\s]+/gu, '')
  const text = normalize(turn.zh) || normalize(turn.en)
  return text ? `${turn.speaker || ''}\u0000${text}` : `id:${turn.id}`
}

function yearOf(name: string) {
  const match = name.match(/'(\d\d)$/)
  return match ? `'${match[1]}` : ''
}

function LanguagePicker({
  primary,
  secondary,
  bilingual,
  onPrimary,
  onSecondary,
  onBilingual,
  compact = false,
}: {
  primary: LocaleCode
  secondary: LocaleCode
  bilingual: boolean
  onPrimary: (value: LocaleCode) => void
  onSecondary: (value: LocaleCode) => void
  onBilingual: () => void
  compact?: boolean
}) {
  return (
    <div
      className={`language-picker ${compact ? 'detail-lang' : ''}`}
      role="group"
      aria-label="对话语言"
    >
      <select
        value={primary}
        aria-label="第一语言"
        onChange={(event) => onPrimary(event.target.value as LocaleCode)}
      >
        {languageOptions.map((language) => (
          <option key={language.id} value={language.id}>
            {language.native}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={`bilingual-toggle ${bilingual ? 'on' : ''}`}
        aria-pressed={bilingual}
        onClick={onBilingual}
      >
        双语 · Bilingual
      </button>
      {bilingual && (
        <select
          value={secondary}
          aria-label="第二语言"
          onChange={(event) => onSecondary(event.target.value as LocaleCode)}
        >
          {languageOptions
            .filter((language) => language.id !== primary)
            .map((language) => (
              <option key={language.id} value={language.id}>
                {language.native}
              </option>
            ))}
        </select>
      )}
    </div>
  )
}

export default function App() {
  const initial = useMemo(() => {
    const params = new URLSearchParams(location.search)
    const explicitLang = params.get('lang')
    const legacyLang = explicitLang || localStorage.getItem('gt7cafe.lang')
    const legacyPrimary =
      legacyLang === 'both'
        ? 'zh'
        : legacyLang === 'en'
          ? 'us'
          : legacyLang
    const primary =
      params.get('l1') ||
      (explicitLang && explicitLang !== 'both' ? legacyPrimary : null) ||
      localStorage.getItem('gt7cafe.primaryLang') ||
      legacyPrimary ||
      'zh'
    const secondary =
      params.get('l2') ||
      localStorage.getItem('gt7cafe.secondaryLang') ||
      (primary === 'us' ? 'zh' : 'us')
    return {
      params,
      primary: (languageIds.has(primary as LocaleCode) ? primary : 'zh') as LocaleCode,
      secondary: (languageIds.has(secondary as LocaleCode) ? secondary : 'us') as LocaleCode,
      bilingual:
        explicitLang === 'both' ||
        (!explicitLang && localStorage.getItem('gt7cafe.bilingual') === '1'),
    }
  }, [])

  const [data, setData] = useState<CarsPayload | null>(null)
  const [primaryLang, setPrimaryLangState] = useState<LocaleCode>(initial.primary)
  const [secondaryLang, setSecondaryLangState] = useState<LocaleCode>(
    initial.secondary === initial.primary
      ? initial.primary === 'us'
        ? 'zh'
        : 'us'
      : initial.secondary,
  )
  const [bilingual, setBilingual] = useState(initial.bilingual)
  const [dialogueTranslations, setDialogueTranslations] = useState<
    Record<string, Record<string, string>>
  >({})
  const [vehicleCollectionI18n, setVehicleCollectionI18n] = useState<
    Record<string, VehicleCollectionLanguageTable>
  >({})
  const [mirrorCatalogs, setMirrorCatalogs] = useState<
    Record<string, MirrorCatalog>
  >({})
  const [loadingLanguages, setLoadingLanguages] = useState<string[]>([])
  const loadingLanguagesRef = useRef(new Set<string>())
  const [languageErrors, setLanguageErrors] = useState<string[]>([])
  const [searchInput, setSearchInput] = useState('')
  const [query, setQuery] = useState('')
  const [brandFilter, setBrandFilter] = useState('ALL')
  const [groupMode, setGroupMode] = useState<GroupMode>('brand')
  const [speakerFilter, setSpeakerFilter] = useState('ALL')
  const [sourceFilter, setSourceFilter] = useState('all')
  const [activeCarId, setActiveCarId] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<TabId>('official')
  const [showUnmatched, setShowUnmatched] = useState(false)
  const [showCast, setShowCast] = useState(false)
  const [showImageDownloader, setShowImageDownloader] = useState(false)
  const [copyState, setCopyState] = useState('复制链接')
  const [officialSlideIndex, setOfficialSlideIndex] = useState(0)
  const [officialAutoplay, setOfficialAutoplayState] = useState(
    localStorage.getItem('gt7cafe.official.autoplay') === '1',
  )
  const [officialSpeed, setOfficialSpeedState] = useState(
    storedNumber(
      'gt7.carlist.slideshowSpeed',
      storedNumber('gt7cafe.official.speed', 6, 1, 10),
      1,
      10,
    ),
  )
  const [officialTextSize, setOfficialTextSizeState] = useState(
    storedNumber('gt7cafe.official.textSize', 22, 12, 42),
  )
  const [officialTextX, setOfficialTextX] = useState(
    storedNumber('gt7cafe.official.textX', 28, 8, 92),
  )
  const [officialTextY, setOfficialTextY] = useState(
    storedNumber('gt7cafe.official.textY', 76, 8, 92),
  )
  const [officialFullscreen, setOfficialFullscreen] = useState(false)
  const [mirrorFullscreenOpen, setMirrorFullscreenOpen] = useState(false)
  const [slideshowPosition, setSlideshowPosition] = useState<SlideshowPosition | null>(
    readSlideshowPosition,
  )
  const [mirrorStart, setMirrorStart] = useState<SlideshowPosition>(() => {
    const stored = readSlideshowPosition()
    return stored || { carId: '', imageIndex: 0 }
  })
  const slideshowOrderRef = useRef<string[]>([])
  const [officialCaptionDragging, setOfficialCaptionDragging] = useState(false)
  const [officialImageError, setOfficialImageError] = useState(false)
  const officialStageRef = useRef<HTMLElement | null>(null)
  const tabListRef = useRef<HTMLDivElement | null>(null)

  const sourceCars = useMemo(() => data?.cars || [], [data])
  const primaryMirrorCatalog =
    mirrorCatalogs[primaryLang] ||
    mirrorCatalogs[primaryLang === 'zh' ? 'cn' : primaryLang]
  const brandNamesByManufacturer = useMemo(() => {
    const names: Record<string, Record<string, string>> = {}
    for (const [code, catalog] of Object.entries(mirrorCatalogs)) {
      for (const record of catalog?.cars || []) {
        if (!record.manufacturer_id || !record.brand) continue
        names[record.manufacturer_id] ||= {}
        names[record.manufacturer_id][code] = record.brand
      }
    }
    return names
  }, [mirrorCatalogs])
  const mergedCatalog = useMemo(
    () => mergeOfficialCars(sourceCars, primaryMirrorCatalog, primaryLang),
    [primaryLang, primaryMirrorCatalog, sourceCars],
  )
  const cars = mergedCatalog.cars
  const hiddenCars = useMemo(
    () => cars.filter((car) => !officialIdFor(car)),
    [cars],
  )
  const officialPageCount = primaryMirrorCatalog?.car_count || 580
  const activeLocales = bilingual ? [primaryLang, secondaryLang] : [primaryLang]
  const showZhContent = activeLocales.some(isChineseLocale)
  const showEnContent = activeLocales.some((code) => !isChineseLocale(code))
  const uiLang = showZhContent ? (bilingual ? 'both' : 'zh') : 'en'
  const activeCar = cars.find((car) => car.id === activeCarId) || null
  const manufacturerId = (car: Car) =>
    manufacturerIdFor(car, primaryMirrorCatalog)
  const brandKeyFor = (car: Car) =>
    manufacturerId(car) || car.brand.trim().toLowerCase()
  const localizedBrandFor = (car: Car, code: string) => {
    const id = manufacturerId(car)
    if (id) {
      const names = brandNamesByManufacturer[id]
      if (names?.[code]) return names[code]
      if (isChineseLocale(code)) {
        const chineseName = names?.zh || names?.cn || names?.tw
        if (chineseName) return chineseName
      }
      const first = names ? Object.values(names)[0] : ''
      if (first) return first
    }
    return car.brand
  }
  const displayBrand = (car: Car) =>
    localizedBrandFor(
      car,
      showZhContent
        ? ['zh', 'cn', 'tw'].find((code) => activeLocales.includes(code as LocaleCode)) ||
            primaryLang
        : primaryLang,
    )
  const detailCopy = useCallback(
    (zh: string, en: string) => {
      if (uiLang === 'zh') return zh
      if (uiLang === 'en') return en
      return `${zh} · ${en}`
    },
    [uiLang],
  )

  useEffect(() => {
    fetch(`${base}data/cars.json`)
      .then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.json()
      })
      .then((payload) => setData(sanitizeGlyphs(payload) as CarsPayload))
      .catch((error) => console.error('Unable to load GT7 archive', error))
  }, [])

  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(searchInput), 160)
    return () => window.clearTimeout(timer)
  }, [searchInput])

  useEffect(() => {
    if (!data) return
    const params = new URLSearchParams(location.search)
    const wantedFilter = params.get('filter')
    if (sourceFilters.some((filter) => filter.id === wantedFilter)) {
      setSourceFilter(wantedFilter!)
    }
    const wantedCar = params.get('car')
    if (wantedCar) {
      const car = data.cars.find(
        (item) => item.id.toLowerCase() === wantedCar.toLowerCase(),
      )
      if (car) {
        setActiveCarId(car.id)
        const wantedTab = params.get('tab') as TabId | null
        setActiveTab(
          wantedTab && carTextTabs.some((tab) => tab.id === wantedTab)
            ? wantedTab
            : firstAvailableTab(car),
        )
      }
    }
    setShowUnmatched(params.get('review') === 'unmatched')
    setShowCast(params.get('cast') === 'all')
  }, [data])

  const ensureLanguage = useCallback(
    async (code: string) => {
      if (
        !code ||
        (dialogueTranslations[code] &&
          vehicleCollectionI18n[code] &&
          mirrorCatalogs[code]) ||
        loadingLanguagesRef.current.has(code)
      ) {
        return
      }
      loadingLanguagesRef.current.add(code)
      setLoadingLanguages([...loadingLanguagesRef.current])
      try {
        const dir = data?.dialogueTranslationDir || 'data/dialogue_translations'
        const mirrorCode = code === 'zh' ? 'cn' : code
        const [response, collectionResponse, mirrorResponse] = await Promise.all([
          fetch(`${base}${dir}/${code}.json`, { cache: 'no-cache' }),
          fetch(`${base}data/vehicle_collection_i18n/${code}.json`, {
            cache: 'no-cache',
          }),
          fetch(`${base}data/mirror/cars.${mirrorCode}.json`, {
            cache: 'no-cache',
          }),
        ])
        if (response.ok) {
          const translation = (await response.json()) as Record<string, string>
          setDialogueTranslations((current) => ({ ...current, [code]: translation }))
        } else {
          setDialogueTranslations((current) => ({
            ...current,
            [code]: current[code] || {},
          }))
        }
        if (collectionResponse.ok) {
          const collection = (await collectionResponse.json()) as VehicleCollectionLanguageTable
          setVehicleCollectionI18n((current) => ({ ...current, [code]: collection }))
        } else {
          setVehicleCollectionI18n((current) => ({
            ...current,
            [code]: current[code] || {
              code,
              sourceFile: '',
              DescriptionCatch: {},
              DescriptionLong: {},
            },
          }))
        }
        if (mirrorResponse.ok) {
          const mirror = (await mirrorResponse.json()) as MirrorCatalog
          setMirrorCatalogs((current) => ({ ...current, [code]: mirror }))
        }
      } catch (error) {
        console.error(`Unable to load dialogue language ${code}`, error)
        setLanguageErrors((current) =>
          current.includes(code) ? current : [...current, code],
        )
      } finally {
        loadingLanguagesRef.current.delete(code)
        setLoadingLanguages([...loadingLanguagesRef.current])
      }
    },
    [
      data?.dialogueTranslationDir,
      dialogueTranslations,
      mirrorCatalogs,
      vehicleCollectionI18n,
    ],
  )

  useEffect(() => {
    for (const code of activeLocales) void ensureLanguage(code)
  }, [activeLocales, ensureLanguage])

  useEffect(() => {
    if (!data || !primaryMirrorCatalog) return
    void ensureAutoImageDownload(data, mirrorCatalogs)
  }, [data, mirrorCatalogs, primaryMirrorCatalog])

  const syncLanguageSettings = useCallback(
    (nextPrimary = primaryLang, nextSecondary = secondaryLang, nextBilingual = bilingual) => {
      localStorage.setItem('gt7cafe.primaryLang', nextPrimary)
      localStorage.setItem('gt7cafe.secondaryLang', nextSecondary)
      localStorage.setItem('gt7cafe.bilingual', nextBilingual ? '1' : '0')
      localStorage.setItem('gt7cafe.lang', nextBilingual ? 'both' : nextPrimary)
      const url = new URL(location.href)
      url.searchParams.set('lang', nextBilingual ? 'both' : nextPrimary)
      url.searchParams.set('l1', nextPrimary)
      if (nextBilingual) url.searchParams.set('l2', nextSecondary)
      else url.searchParams.delete('l2')
      history.replaceState(null, '', url)
    },
    [primaryLang, secondaryLang, bilingual],
  )

  function setPrimaryLanguage(code: LocaleCode) {
    const next = languageIds.has(code) ? code : 'zh'
    const nextSecondary = secondaryLang === next ? (next === 'us' ? 'zh' : 'us') : secondaryLang
    setPrimaryLangState(next)
    setSecondaryLangState(nextSecondary)
    syncLanguageSettings(next, nextSecondary)
  }

  function setSecondaryLanguage(code: LocaleCode) {
    if (!languageIds.has(code) || code === primaryLang) return
    setSecondaryLangState(code)
    syncLanguageSettings(primaryLang, code)
  }

  function toggleBilingual() {
    setBilingual(!bilingual)
    syncLanguageSettings(primaryLang, secondaryLang, !bilingual)
  }

  function dialogueText(turn: DialogueTurn, code: string) {
    const translated = dialogueTranslations[code]?.[turn.id]
    if (translated) return translated
    if (isChineseLocale(code)) return turn.zh || turn.en || ''
    if (code === 'us' || code === 'gb') return turn.en || turn.zh || ''
    if (loadingLanguages.includes(code)) return '…'
    if (languageErrors.includes(code)) {
      return detailCopy('该语言文本加载失败', 'This language failed to load')
    }
    return '…'
  }

  function languageName(code: string) {
    const language = languageMeta(code)
    return uiLang === 'zh' ? language.zh : language.native
  }

  function speakerName(turn: DialogueTurn) {
    if (uiLang === 'zh') return turn.speakerZh || turn.speaker
    if (uiLang === 'both') return `${turn.speakerZh || turn.speaker} · ${turn.speaker}`
    return turn.speaker
  }

  function speakerRole(turn: DialogueTurn) {
    return uiLang === 'zh' ? turn.roleZh : turn.roleEn
  }

  function visibleConversation(car: Car) {
    const turns = car.conversation?.length ? car.conversation : car.dialogues || []
    return speakerFilter === 'ALL'
      ? turns
      : turns.filter((turn) => turn.speaker === speakerFilter)
  }

  function allTextEntries(car: Car) {
    return Object.values(car.texts || {}).flatMap((text) => [
      text,
      ...((text?.variants || []) as CarText[]),
    ])
  }

  const brands = useMemo(() => {
    const counts = new Map<string, { key: string; name: string; count: number }>()
    for (const car of cars) {
      const key = brandKeyFor(car)
      const current = counts.get(key) || {
        key,
        name: displayBrand(car),
        count: 0,
      }
      current.count += 1
      counts.set(key, current)
    }
    return [...counts.values()].sort((a, b) =>
      a.name.localeCompare(b.name, 'en'),
    )
  }, [bilingual, cars, mirrorCatalogs, primaryLang, secondaryLang])

  const brandLogos = useMemo(() => {
    const logos: Record<string, string> = {}
    for (const car of cars) {
      const key = brandKeyFor(car)
      if (car.brandLogo && !logos[key]) logos[key] = car.brandLogo
    }
    return logos
  }, [bilingual, cars, mirrorCatalogs, primaryLang, secondaryLang])

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return cars.filter((car) => {
      if (brandFilter !== 'ALL' && brandKeyFor(car) !== brandFilter) return false
      const conversation = visibleConversation(car)
      if (sourceFilter === 'cafe' && !conversation.length) return false
      if (sourceFilter === 'hidden' && officialIdFor(car)) return false
      if (
        sourceFilter !== 'all' &&
        sourceFilter !== 'cafe' &&
        sourceFilter !== 'hidden' &&
        !car.texts?.[sourceFilter]
      ) {
        return false
      }
      if (speakerFilter !== 'ALL' && !conversation.length) return false
      if (!needle) return true
      if (`${car.nameEn} ${car.nameZh} ${car.brand} ${displayBrand(car)}`.toLowerCase().includes(needle)) {
        return true
      }
      if (
        [...(car.dialogues || []), ...(car.conversation || [])].some((turn) =>
          `${turn.en} ${turn.zh} ${turn.speaker} ${turn.speakerZh}`
            .toLowerCase()
            .includes(needle),
        )
      ) {
        return true
      }
      return allTextEntries(car).some((text) =>
        `${text?.en} ${text?.zh} ${text?.titleEn} ${text?.titleZh}`
          .toLowerCase()
          .includes(needle),
      )
    })
  }, [
    bilingual,
    brandFilter,
    cars,
    mirrorCatalogs,
    primaryLang,
    query,
    secondaryLang,
    sourceFilter,
    speakerFilter,
  ])

  const grouped = useMemo(() => {
    const groups = new Map<string, { key: string; label: string; cars: Car[] }>()
    const collator = new Intl.Collator('en', { sensitivity: 'base' })
    for (const car of filtered) {
      let key = ''
      let label = ''
      if (groupMode === 'brand') {
        key = brandKeyFor(car)
        label = displayBrand(car)
      } else if (groupMode === 'country') {
        key = car.garageDetails?.country?.code || 'other'
        label =
          (uiLang === 'zh'
            ? car.garageDetails?.country?.nameZh
            : car.garageDetails?.country?.nameEn) || detailCopy('其他', 'Other')
      } else if (groupMode === 'drive') {
        const drive =
          (uiLang === 'zh' ? car.specs?.zh?.driveTrain : car.specs?.en?.driveTrain) ||
          car.specs?.en?.driveTrain ||
          detailCopy('其他', 'Other')
        key = String(drive)
        label = String(drive)
      } else {
        const thresholds = groupMode === 'power' ? POWER_THRESHOLDS : WEIGHT_THRESHOLDS
        const index = thresholdIndex(groupMetric(car, groupMode), thresholds)
        const lower = thresholds[index]
        const upper = index > 0 ? thresholds[index - 1] : null
        key = String(index)
        const unit = groupMode === 'power' ? 'PS' : detailCopy('公斤', 'kg')
        label =
          lower === undefined
            ? detailCopy('其他', 'Other')
            : `${lower.toLocaleString('en-US')} - ${upper === null ? '' : (upper - 1).toLocaleString('en-US')}${unit}`
      }
      const group = groups.get(key) || { key, label, cars: [] }
      group.cars.push(car)
      groups.set(key, group)
    }
    const values = [...groups.values()]
    if (groupMode === 'brand' || groupMode === 'country') {
      values.sort((left, right) => collator.compare(left.label, right.label))
    } else if (groupMode === 'drive') {
      const order = ['FF', 'FR', '4WD', 'MR', 'RR']
      values.sort((left, right) => {
        const li = order.indexOf(left.key)
        const ri = order.indexOf(right.key)
        return (li < 0 ? 99 : li) - (ri < 0 ? 99 : ri) || collator.compare(left.label, right.label)
      })
    } else {
      values.sort((left, right) => Number(left.key) - Number(right.key))
    }
    for (const group of values) {
      group.cars.sort((left, right) =>
        collator.compare(
          uiLang === 'zh' ? left.nameZh : left.nameEn,
          uiLang === 'zh' ? right.nameZh : right.nameEn,
        ),
      )
    }
    return values
  }, [detailCopy, filtered, groupMode, uiLang])

  const sortedFiltered = useMemo(
    () => grouped.flatMap((group) => group.cars),
    [grouped],
  )

  const activeIndex = activeCar ? sortedFiltered.findIndex((car) => car.id === activeCar.id) : -1
  const navCar = activeCar || sortedFiltered[Math.max(0, activeIndex)] || null
  const navPosition = activeCar ? activeIndex + 1 : 0
  const shortcutStateRef = useRef({
    activeCar,
    activeTab,
    filtered: sortedFiltered,
    navCar,
    showCast,
    showUnmatched,
    overlayOpen: mirrorFullscreenOpen,
  })
  shortcutStateRef.current = {
    activeCar,
    activeTab,
    filtered: sortedFiltered,
    navCar,
    showCast,
    showUnmatched,
    overlayOpen: mirrorFullscreenOpen,
  }
  const officialImages = activeCar?.officialDetails?.images || []
  const officialMirrorCopies = activeCar ? mirrorCopiesFor(activeCar) : []
  const mirrorImagePaths = officialMirrorCopies.flatMap(
    (copy) => copy.record?.images || [],
  )
  const officialImagePaths = (
    mirrorImagePaths.length
      ? mirrorImagePaths
      : officialImages.map((image) => image.jxl)
  ).filter((value, index, values) => values.indexOf(value) === index)
  const currentOfficialImage = officialImagePaths[officialSlideIndex] || null
  const mirrorSlideCars = useMemo<MirrorSlideCar[]>(
    () =>
      sortedFiltered.map((car) => {
        const record = mirrorCarFor(car, primaryLang)
        const images = (
          record?.images?.length
            ? record.images
            : (car.officialDetails?.images || []).map((image) => image.jxl)
        ).map(assetUrl)
        const logo = record?.logo
          ? assetUrl(record.logo)
          : car.brandLogo
            ? `${base}logos/${car.brandLogo}`
            : ''
        return {
          id: car.id,
          brand: record?.brand || displayBrand(car),
          model: record?.model || displayName(car),
          logo,
          images,
          hero: record?.hero || car.officialDetails?.heroZh || car.officialDetails?.heroEn || '',
          description:
            record?.description ||
            car.officialDetails?.descriptionZh ||
            car.officialDetails?.descriptionEn ||
            '',
          carClass: record?.car_class || String(car.specs?.zh?.carClass || car.specs?.en?.carClass || ''),
          performancePoint:
            record?.performance_point ||
            String(car.specs?.zh?.performancePoint || car.specs?.en?.performancePoint || ''),
          specs: record?.specs || [],
        }
      }),
    [mirrorCatalogs, primaryLang, sortedFiltered, uiLang],
  )
  const officialCaptionStyle: CSSProperties = {
    left: `${officialTextX}%`,
    top: `${officialTextY}%`,
    fontSize: `${officialTextSize}px`,
  }

  function tabHasContent(car: Car | null | undefined, tabId: TabId) {
    if (tabId === 'official') {
      return Boolean(car?.officialDetails)
    }
    if (tabId === 'cafe') {
      return Boolean(car?.conversation?.length || car?.dialogues?.length)
    }
    if (tabId === 'garageSpecs') {
      return Boolean(
        car?.specs?.en?.performancePoint ||
          car?.specs?.zh?.performancePoint ||
          car?.specs?.en?.power ||
          car?.specs?.zh?.power,
      )
    }
    return Boolean(car?.texts?.[tabId]?.en || car?.texts?.[tabId]?.zh)
  }

  function firstAvailableTab(car: Car): TabId {
    return carTextTabs.find((tab) => tabHasContent(car, tab.id))?.id || 'cafe'
  }

  function tabLabel(tab: (typeof carTextTabs)[number]) {
    const label =
      uiLang === 'zh'
        ? tab.labelZh
        : uiLang === 'both'
          ? `${tab.labelZh} · ${tab.labelEn}`
          : tab.labelEn
    if (!activeCar) return label
    if (tab.id === 'cafe') {
      const count = activeCar.conversation?.length || 0
      return count
        ? `${label} · ${count}`
        : `${label} · ${detailCopy('无', 'None')}`
    }
    return tabHasContent(activeCar, tab.id)
      ? label
      : `${label} · ${detailCopy('无', 'None')}`
  }

  function setTab(tabId: TabId) {
    setActiveTab(tabId)
    const url = new URL(location.href)
    url.searchParams.set('tab', tabId)
    history.replaceState(null, '', url)
  }

  function setCarParam(id: string | null) {
    const url = new URL(location.href)
    if (id) url.searchParams.set('car', id)
    else {
      url.searchParams.delete('car')
      url.searchParams.delete('tab')
    }
    history.replaceState(null, '', url)
  }

  function openCar(car: Car, preferredTab?: TabId) {
    setActiveCarId(car.id)
    setTab(
      preferredTab && tabHasContent(car, preferredTab)
        ? preferredTab
        : firstAvailableTab(car),
    )
    setCarParam(car.id)
    const url = new URL(location.href)
    url.searchParams.delete('review')
    url.searchParams.delete('cast')
    history.replaceState(null, '', url)
  }

  function closeCar() {
    if (document.fullscreenElement === officialStageRef.current) {
      void document.exitFullscreen().catch(() => {})
    }
    setActiveCarId(null)
    setCarParam(null)
  }

  function move(delta: number, openIfClosed = false) {
    const { activeCar: currentCar, filtered: currentCars } =
      shortcutStateRef.current
    if (!currentCars.length) return
    const currentIndex = currentCar
      ? currentCars.findIndex((car) => car.id === currentCar.id)
      : -1
    const nextIndex =
      currentIndex < 0
        ? delta > 0
          ? 0
          : currentCars.length - 1
        : (currentIndex + delta + currentCars.length) % currentCars.length
    const nextCar = currentCars[nextIndex]
    if (currentCar) {
      openCar(
        nextCar,
        tabHasContent(nextCar, shortcutStateRef.current.activeTab)
          ? shortcutStateRef.current.activeTab
          : firstAvailableTab(nextCar),
      )
    } else if (openIfClosed) {
      openCar(nextCar)
    } else {
      requestAnimationFrame(() =>
        document.querySelector('.card.active')?.scrollIntoView({
          block: 'nearest',
          behavior: 'smooth',
        }),
      )
    }
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const shortcut = shortcutStateRef.current
      const editing = ['INPUT', 'TEXTAREA', 'SELECT'].includes(
        (event.target as HTMLElement)?.tagName,
      )
      if (editing) {
        if (event.key === 'Escape') {
          ;(event.target as HTMLElement).blur()
          setShowCast(false)
          closeCar()
        }
        return
      }
      if (shortcut.overlayOpen) return
      if (event.key === 'Escape') {
        if (shortcut.showUnmatched) setShowUnmatched(false)
        else if (shortcut.showCast) setShowCast(false)
        else closeCar()
        return
      }
      if (
        officialStageRef.current &&
        document.fullscreenElement === officialStageRef.current
      ) {
        if (event.key === 'ArrowRight') moveOfficialSlide(1)
        if (event.key === 'ArrowLeft') moveOfficialSlide(-1)
        return
      }
      if (
        shortcut.activeCar &&
        shortcut.activeTab === 'official' &&
        ['ArrowLeft', 'ArrowRight'].includes(event.key)
      ) {
        event.preventDefault()
        moveOfficialSlide(event.key === 'ArrowRight' ? 1 : -1)
      } else if (event.key === 'ArrowDown') {
        event.preventDefault()
        move(1)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        move(-1)
      } else if (shortcut.activeCar && event.key === 'Tab') {
        event.preventDefault()
        const index = carTextTabs.findIndex(
          (tab) => tab.id === shortcut.activeTab,
        )
        const next =
          (index + (event.shiftKey ? -1 : 1) + carTextTabs.length) % carTextTabs.length
        setTab(carTextTabs[next].id)
      } else if (
        shortcut.activeCar &&
        ['1', '2', '3', '4', '5', '6', '7'].includes(event.key)
      ) {
        event.preventDefault()
        setTab(carTextTabs[Number(event.key) - 1].id)
      } else if (event.key === 'Enter') {
        event.preventDefault()
        if (shortcut.navCar) openCar(shortcut.navCar)
      } else if (event.key === '/') {
        event.preventDefault()
        document.querySelector<HTMLInputElement>('.search')?.focus()
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [])

  useEffect(() => {
    setOfficialSlideIndex(0)
    setOfficialImageError(false)
  }, [activeCarId])

  useEffect(() => {
    setOfficialImageError(false)
  }, [currentOfficialImage])

  useEffect(() => {
    if (
      !officialAutoplay ||
      activeTab !== 'official' ||
      officialImagePaths.length < 2
    ) {
      return
    }
    const timer = window.setTimeout(() => {
      setOfficialSlideIndex((index) => (index + 1) % officialImagePaths.length)
    }, Math.round(5200 / officialSpeed))
    return () => window.clearTimeout(timer)
  }, [
    activeTab,
    officialAutoplay,
    officialImagePaths.length,
    officialSlideIndex,
    officialSpeed,
  ])

  useEffect(() => {
    const onFullscreen = () => {
      setOfficialFullscreen(document.fullscreenElement === officialStageRef.current)
    }
    document.addEventListener('fullscreenchange', onFullscreen)
    return () => document.removeEventListener('fullscreenchange', onFullscreen)
  }, [])

  function setOfficialAutoplay(value: boolean) {
    setOfficialAutoplayState(value)
    localStorage.setItem('gt7cafe.official.autoplay', value ? '1' : '0')
  }

  function setOfficialSpeed(value: number) {
    const next = Math.min(10, Math.max(1, Math.trunc(Number(value) || 6)))
    setOfficialSpeedState(next)
    localStorage.setItem('gt7cafe.official.speed', String(next))
    localStorage.setItem('gt7.carlist.slideshowSpeed', String(next))
  }

  function setOfficialTextSize(value: number) {
    const next = Math.min(42, Math.max(12, Number(value) || 22))
    setOfficialTextSizeState(next)
    localStorage.setItem('gt7cafe.official.textSize', String(next))
  }

  function saveOfficialTextPosition() {
    localStorage.setItem('gt7cafe.official.textX', String(officialTextX))
    localStorage.setItem('gt7cafe.official.textY', String(officialTextY))
  }

  function setOfficialCaptionPreset(x: number, y: number) {
    setOfficialTextX(x)
    setOfficialTextY(y)
    localStorage.setItem('gt7cafe.official.textX', String(x))
    localStorage.setItem('gt7cafe.official.textY', String(y))
  }

  function moveOfficialSlide(delta: number) {
    if (!officialImagePaths.length) return
    setOfficialSlideIndex(
      (index) => (index + delta + officialImagePaths.length) % officialImagePaths.length,
    )
  }

  function updateOfficialCaptionFromPointer(event: ReactPointerEvent<HTMLDivElement>) {
    if (!officialCaptionDragging || !officialStageRef.current) return
    const rect = officialStageRef.current.getBoundingClientRect()
    setOfficialTextX(
      Math.min(92, Math.max(8, ((event.clientX - rect.left) / rect.width) * 100)),
    )
    setOfficialTextY(
      Math.min(92, Math.max(8, ((event.clientY - rect.top) / rect.height) * 100)),
    )
  }

  async function toggleOfficialFullscreen() {
    if (!activeCar) return
    const imageIndex =
      slideshowPosition?.carId === activeCar.id
        ? slideshowPosition.imageIndex
        : officialSlideIndex
    setOfficialSlideIndex(imageIndex)
    setMirrorStart({ carId: activeCar.id, imageIndex })
    setOfficialFullscreen(false)
    setMirrorFullscreenOpen(true)
    if (!officialAutoplay) setOfficialAutoplay(true)
  }

  function openCatalogSlideshow() {
    const orderedCarIds = mirrorSlideCars.map((car) => car.id)
    if (!orderedCarIds.length) return
    const previousOrder = slideshowOrderRef.current
    const orderChanged =
      orderedCarIds.length !== previousOrder.length ||
      orderedCarIds.some((carId, index) => carId !== previousOrder[index])
    slideshowOrderRef.current = orderedCarIds
    const resumeId =
      !orderChanged &&
      slideshowPosition?.carId &&
      orderedCarIds.includes(slideshowPosition.carId)
        ? slideshowPosition.carId
        : orderedCarIds[0]
    const imageIndex =
      !orderChanged && resumeId === slideshowPosition?.carId
        ? slideshowPosition.imageIndex
        : 0
    setMirrorStart({ carId: resumeId, imageIndex })
    setMirrorFullscreenOpen(true)
    if (!officialAutoplay) setOfficialAutoplay(true)
  }

  function closeMirrorFullscreen(position: SlideshowPosition) {
    setMirrorFullscreenOpen(false)
    setSlideshowPosition(position)
    setMirrorStart(position)
    localStorage.setItem('gt7.carlist.lastSlideshowPosition', JSON.stringify(position))
    const car = cars.find((item) => item.id === position.carId)
    if (car) {
      setOfficialSlideIndex(position.imageIndex)
      setActiveCarId(car.id)
      setCarParam(car.id)
      setTab(activeTab)
    }
    if (document.fullscreenElement) {
      document.exitFullscreen().catch(() => {})
    }
  }

  function specValue(locale: 'zh' | 'en', key: string) {
    const value = activeCar?.specs?.[locale]?.[key]
    return value === undefined || value === null ? '' : String(value)
  }

  function compactSpecValue(key: string) {
    const zh = specValue('zh', key)
    const en = specValue('en', key)
    if (showZhContent && showEnContent) {
      return zh && en && zh !== en ? `${zh} · ${en}` : zh || en
    }
    return showZhContent ? zh : en
  }

  const activeCafeScenes = useMemo(() => {
    if (!activeCar) return []
    const groups: Array<{
      key: string
      titleZh: string
      titleEn: string
      turns: DialogueTurn[]
    }> = []
    const byKey = new Map<string, (typeof groups)[number]>()
    const seen = new Set<string>()
    for (const turn of visibleConversation(activeCar)) {
      const dedupeKey = dialogueDedupeKey(turn)
      if (seen.has(dedupeKey)) continue
      seen.add(dedupeKey)
      const key = turn.sceneKey || turn.sceneId || 'scene'
      let group = byKey.get(key)
      if (!group) {
        group = { key, titleZh: '', titleEn: '', turns: [] }
        byKey.set(key, group)
        groups.push(group)
      }
      group.turns.push(turn)
    }
    for (const group of groups) {
      const sceneSpeakers = [...new Set(group.turns.map((turn) => turn.speaker))]
      const guest =
        sceneSpeakers.find((name) => !['Sarah', 'Luca'].includes(name)) ||
        sceneSpeakers[0] ||
        'GT Cafe'
      const guestZh =
        group.turns.find((turn) => turn.speaker === guest)?.speakerZh || guest
      const sarahZh =
        group.turns.find((turn) => turn.speaker === 'Sarah')?.speakerZh || 'Sarah'
      group.titleEn =
        sceneSpeakers.includes('Sarah') && guest !== 'Sarah'
          ? `${guest} and Sarah`
          : guest
      group.titleZh =
        sceneSpeakers.includes('Sarah') && guest !== 'Sarah'
          ? `${guestZh} 与 ${sarahZh}`
          : guestZh
    }
    return groups
  }, [activeCar, speakerFilter])

  function displayName(car: Car) {
    return showZhContent ? car.nameZh : car.nameEn
  }

  function carNameLocales() {
    const groups = new Set<string>()
    return activeLocales.filter((code) => {
      const group = isChineseLocale(code) ? 'zh' : 'en'
      if (groups.has(group)) return false
      groups.add(group)
      return true
    })
  }

  const letters = useMemo(
    () => [...new Set(brands.map((brand) => brand.name[0].toUpperCase()))].sort(),
    [brands],
  )

  const unmatched = data?.unmatched || []
  const filteredUnmatched = unmatched.filter((dialogue) => {
    if (speakerFilter !== 'ALL' && dialogue.speaker !== speakerFilter) return false
    if (!query) return true
    return `${dialogue.en} ${dialogue.zh} ${dialogue.speaker} ${dialogue.speakerZh}`
      .toLowerCase()
      .includes(query.trim().toLowerCase())
  })

  if (!data) {
    return (
      <div className="boot-screen">
        <span>GT</span>
        <b>{detailCopy('正在载入车辆、对话与车库资料…', 'Loading cars, dialogue and garage data…')}</b>
      </div>
    )
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="gt">
            <img
              src="/common/dist/gt7/carlist/assets/logo_gtcom-kY3Al7KV.svg"
              alt="Gran Turismo"
            />
          </span>
          <div className="titles">
            <h1>{detailCopy('咖啡屋对话库', 'Café dialogue archive')}</h1>
            <p>Gran Turismo 7 · GT Café Dialogues</p>
          </div>
        </div>
        <ImageCacheStatus onOpen={() => setShowImageDownloader(true)} />
        <a className="menu-archive-link" href="/menus_cafe">
          {detailCopy('菜单书档案', 'Menu books')}
        </a>
        <LanguagePicker
          primary={primaryLang}
          secondary={secondaryLang}
          bilingual={bilingual}
          onPrimary={setPrimaryLanguage}
          onSecondary={setSecondaryLanguage}
          onBilingual={toggleBilingual}
        />
      </header>

      <section className="summary">
        <div className="stat">
          <b>{cars.length}</b>
          <span>{detailCopy('车辆总库', 'All cars')}</span>
        </div>
        <div className="stat">
          <b>{officialPageCount} / {hiddenCars.length}</b>
          <span>{detailCopy('官方 / 隐藏', 'Official / hidden')}</span>
        </div>
        <div className="stat">
          <b>{data.stats?.withDialogue || 0}</b>
          <span>{detailCopy('有咖啡屋对白', 'Cars with café dialogue')}</span>
        </div>
        <div className="stat">
          <b>{data.stats?.dialogues || 0}</b>
          <span>{detailCopy('CarDetail 对白', 'CarDetail turns')}</span>
        </div>
        <div className="stat">
          <b>{data.carTextStats?.mappedEntries || 0}</b>
          <span>{detailCopy('车辆说明', 'Vehicle descriptions')}</span>
        </div>
        <div className="stat">
          <b>{brands.length}</b>
          <span>{detailCopy('品牌', 'Manufacturers')}</span>
        </div>
        <div className="stat">
          <b>{data.cast?.length || 0}</b>
          <span>{detailCopy('咖啡屋角色', 'Café characters')}</span>
        </div>
        <button className="stat review-stat" type="button" onClick={() => setShowUnmatched(true)}>
          <b>{unmatched.length}</b>
          <span>{detailCopy('上一轮歧义复核', 'Prior ambiguity review')}</span>
        </button>
      </section>

      <p className="summary-note">
        {detailCopy(
          `${data.stats?.mappedDialogues || 0} 条 CarDetail 已定位 · 重建完整场景 ${data.stats?.conversationLines || 0} 行对白 · ${data.carTextStats?.carsWithAnyText || 0} 辆车具品牌购物、车辆收藏、二手车或收藏馆说明`,
          `${data.stats?.mappedDialogues || 0} CarDetail entries mapped · ${data.stats?.conversationLines || 0} reconstructed scene lines · ${data.carTextStats?.carsWithAnyText || 0} cars with Brand Central, Car Collection, Used Cars or Hagerty text`,
        )}
      </p>

      <section className="castbar" aria-label={detailCopy('咖啡屋主要角色', 'Main café characters')}>
        {(data.cast || []).slice(0, 8).map((person: any) => (
          <div key={person.id} className="cast-person">
            <img src={avatarUrl(person.avatar)} alt={person.en} loading="eager" />
            <span>
              <b>
                {uiLang === 'zh'
                  ? person.zh
                  : uiLang === 'both'
                    ? `${person.zh} · ${person.en}`
                    : person.en}
              </b>
              <small>
                {uiLang === 'zh'
                  ? person.roleZh
                  : uiLang === 'both'
                    ? `${person.roleZh} · ${person.roleEn}`
                    : person.roleEn}
              </small>
            </span>
          </div>
        ))}
        <button className="cast-all" type="button" onClick={() => setShowCast(true)}>
          {detailCopy(
            `查看全部 ${data.cast?.length || 0} 位角色`,
            `View all ${data.cast?.length || 0} characters`,
          )}
        </button>
      </section>

      <div className="toolbar">
        <input
          className="search"
          type="search"
          value={searchInput}
          onChange={(event) => setSearchInput(event.target.value)}
          placeholder={detailCopy(
            '搜索车型 / 对话 / 车辆说明…（按 / 聚焦）',
            'Search cars / dialogue / descriptions… (press /)',
          )}
        />
        <div
          className="seg"
          role="group"
          aria-label={detailCopy('车辆资料筛选', 'Vehicle data filters')}
        >
          {sourceFilters.map((filter) => (
            <button
              key={filter.id}
              className={sourceFilter === filter.id ? 'on' : ''}
              aria-pressed={sourceFilter === filter.id}
              onClick={() => {
                setSourceFilter(filter.id)
                const url = new URL(location.href)
                if (filter.id === 'all') url.searchParams.delete('filter')
                else url.searchParams.set('filter', filter.id)
                history.replaceState(null, '', url)
              }}
            >
              {uiLang === 'zh'
                ? filter.labelZh
                : uiLang === 'both'
                  ? `${filter.labelZh} · ${filter.labelEn}`
                  : filter.labelEn}
              {filter.id === 'hidden' ? ` · ${hiddenCars.length}` : ''}
            </button>
          ))}
        </div>
      </div>

      <section className="catalog-hero" aria-label="Gran Turismo 7">
        <button
          type="button"
          className="catalog-slideshow-launch"
          onClick={openCatalogSlideshow}
          disabled={!mirrorSlideCars.length}
        >
          <span aria-hidden="true">▶</span>
          {detailCopy('全屏幻灯片', 'Fullscreen slideshow')}
          <small>
            {mirrorSlideCars.reduce((total, car) => total + car.images.length, 0)}{' '}
            {detailCopy('张', 'images')}
          </small>
        </button>
        <img
          src="/common/dist/gt7/carlist/assets/logo_gt7-BQSHSLc9.png"
          alt="Gran Turismo 7 The Real Driving Simulator"
        />
        <p>
          <b>{cars.length}</b>
          <span>{detailCopy(`官方车表 ${officialPageCount}`, `Official ${officialPageCount}`)}</span>
          <span>{detailCopy(`隐藏车型 ${hiddenCars.length}`, `Hidden ${hiddenCars.length}`)}</span>
          <span>{detailCopy('截至 2026 年 10 月', 'as of October 2026')}</span>
        </p>
      </section>

      <nav className="brandbar">
        <button
          className={`chip ${brandFilter === 'ALL' ? 'on' : ''}`}
          onClick={() => setBrandFilter('ALL')}
        >
          {detailCopy('全部品牌', 'All manufacturers')}
        </button>
        {letters.map((letter) => (
          <button
            key={letter}
            className="letter"
            onClick={() => {
              const first = brands.find((brand) => brand.name[0].toUpperCase() === letter)
              if (first) {
                document
                  .getElementById(`brand-${first.name}`)
                  ?.scrollIntoView({ behavior: 'smooth', block: 'start' })
              }
            }}
          >
            {letter}
          </button>
        ))}
      </nav>

      <nav className="group-tabs" aria-label={detailCopy('车辆排序与分组', 'Vehicle grouping')}>
        {([
          ['brand', detailCopy('按厂商', 'By Manufacturer')],
          ['country', detailCopy('按国家/地区', 'By Country/Region')],
          ['drive', detailCopy('按驱动形式', 'By Drivetrain')],
          ['power', detailCopy('按马力', 'By Power')],
          ['weight', detailCopy('按车重', 'By Weight')],
        ] as Array<[GroupMode, string]>).map(([mode, label]) => (
          <button
            key={mode}
            type="button"
            className={groupMode === mode ? 'on' : ''}
            aria-pressed={groupMode === mode}
            onClick={() => setGroupMode(mode)}
          >
            {label}
          </button>
        ))}
      </nav>

      <nav className="characterbar" aria-label="咖啡屋角色筛选">
        <button
          className={`chip character ${speakerFilter === 'ALL' ? 'on' : ''}`}
          onClick={() => setSpeakerFilter('ALL')}
        >
          {detailCopy('全部角色', 'All characters')}
        </button>
        {(data.speakers || []).map((speaker: any) => (
          <button
            key={speaker.id}
            className={`chip character ${speakerFilter === speaker.id ? 'on' : ''}`}
            onClick={() => setSpeakerFilter(speaker.id)}
          >
            <img src={avatarUrl(speaker.avatar)} alt={speaker.en} loading="lazy" />
            {uiLang === 'zh'
              ? speaker.zh
              : uiLang === 'both'
                ? `${speaker.zh} · ${speaker.en}`
                : speaker.en}{' '}
            <small>{speaker.count}</small>
          </button>
        ))}
      </nav>

      <main className="content">
        {grouped.map((group) => (
          <section key={group.key} id={`group-${group.key}`} className="brandgroup">
            <h2>
              {groupMode === 'brand' && brandLogos[group.key] && (
                <img
                  className="blogo"
                  src={`${base}logos/${brandLogos[group.key]}`}
                  alt={group.label}
                  loading="lazy"
                />
              )}
              {group.label} <span>{group.cars.length}</span>
            </h2>
            <div className="grid">
              {group.cars.map((car) => (
                <article
                  key={car.id}
                  data-car-id={car.id}
                  className={`card ${visibleConversation(car).length ? '' : 'empty'} ${
                    activeCar?.id === car.id ? 'active' : ''
                  }`}
                  onClick={() => openCar(car)}
                >
                  <div className="car-media">
                    {car.image ? (
                      <img
                        className="thumb"
                        src={`${base}${car.image}`}
                        alt={car.nameEn}
                        loading="lazy"
                        decoding="async"
                        width="360"
                        height="180"
                      />
                    ) : (
                      <div className="thumb-placeholder" aria-hidden="true">
                        <span>GT7</span>
                        <b>{displayBrand(car)}</b>
                      </div>
                    )}
                  </div>
                  <div className="name">
                    {carNameLocales().map((code, index) => (
                      <span key={code}>
                        {index > 0 && <br />}
                        {isChineseLocale(code) ? car.nameZh : car.nameEn}
                      </span>
                    ))}
                  </div>
                  <div className="meta">
                    <span className="year">{yearOf(car.nameEn)}</span>
                    <span className="badges">
                      {visibleConversation(car).length ? (
                        <span className="badge has">
                          {detailCopy('对话', 'Dialogue')} · {visibleConversation(car).length}
                        </span>
                      ) : (
                        <span className="badge none">
                          {detailCopy('无对话', 'No dialogue')}
                        </span>
                      )}
                      {carTextTabs.filter(
                        (tab) =>
                          tab.id !== 'cafe' &&
                          tab.id !== 'official' &&
                          tabHasContent(car, tab.id),
                      ).length > 0 && (
                        <span className="badge text">
                          {detailCopy('资料', 'Data')} ·{' '}
                          {
                            carTextTabs.filter(
                              (tab) =>
                                tab.id !== 'cafe' &&
                                tab.id !== 'official' &&
                                tabHasContent(car, tab.id),
                            ).length
                          }
                        </span>
                      )}
                    </span>
                  </div>
                </article>
              ))}
            </div>
          </section>
        ))}
        {!grouped.length && (
          <p className="nonefound">
            {detailCopy('没有匹配的车辆。', 'No matching cars.')}
          </p>
        )}
      </main>

      <nav
        className="mobile-car-nav"
        aria-label={detailCopy('上一辆车和下一辆车', 'Previous and next car')}
      >
        <button
          type="button"
          className="mobile-car-step"
          aria-label={detailCopy('上一辆车', 'Previous car')}
          disabled={!filtered.length}
          onClick={() => move(-1, true)}
        >
          <strong>‹</strong><span>{detailCopy('上一辆', 'Previous')}</span>
        </button>
        <button
          type="button"
          className="mobile-car-current"
          onClick={() => (activeCar ? closeCar() : navCar && openCar(navCar))}
        >
          <small>{navCar ? displayBrand(navCar) : detailCopy('选择车辆', 'Select car')}</small>
          <b>
            {navCar
              ? displayName(navCar)
              : detailCopy('浏览全部车型', 'Browse all cars')}
          </b>
          <span aria-live="polite">{navPosition} / {filtered.length}</span>
        </button>
        <button
          type="button"
          className="mobile-car-step"
          aria-label={detailCopy('下一辆车', 'Next car')}
          disabled={!filtered.length}
          onClick={() => move(1, true)}
        >
          <strong>›</strong><span>{detailCopy('下一辆', 'Next')}</span>
        </button>
      </nav>

      <Dialog.Root open={showCast} onOpenChange={setShowCast}>
        <Dialog.Portal>
          <Dialog.Backdrop className="car-dialog-backdrop" />
          <Dialog.Popup className="modal">
            <div className="dialog cast-dialog">
              <header>
                <div>
                  <Dialog.Title className="dialog-title">
                    {detailCopy('GT 咖啡屋角色图鉴', 'GT Café character archive')}
                  </Dialog.Title>
                  <p>
                    {detailCopy(
                      `全部真实咖啡屋角色与设计师头像，共 ${data.cast?.length || 0} 位`,
                      `All real café characters and designer portraits, ${data.cast?.length || 0} total`,
                    )}
                  </p>
                </div>
                <Dialog.Close
                  className="close"
                  aria-label={detailCopy('关闭', 'Close')}
                >
                  <X size={18} />
                </Dialog.Close>
              </header>
              <div className="cast-grid">
                {(data.cast || []).map((person: any) => (
                  <button
                    key={person.id}
                    className="cast-card"
                    onClick={() => {
                      setSpeakerFilter(person.id)
                      setShowCast(false)
                    }}
                  >
                    <img className="avatar-portrait" src={avatarUrl(person.avatar)} alt={person.en} />
                    <span className="cast-card-name">
                      {uiLang === 'zh'
                        ? person.zh
                        : uiLang === 'both'
                          ? `${person.zh} · ${person.en}`
                          : person.en}
                    </span>
                    <span className="cast-card-role">
                      {uiLang === 'zh'
                        ? person.roleZh
                        : uiLang === 'both'
                          ? `${person.roleZh} · ${person.roleEn}`
                          : person.roleEn}
                    </span>
                    <span className="cast-card-count">
                      {person.count
                        ? detailCopy(
                            `${person.count} 条车辆对白`,
                            `${person.count} vehicle dialogue turns`,
                          )
                        : detailCopy('核心剧情角色', 'Core story character')}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>

      <Dialog.Root open={showUnmatched} onOpenChange={setShowUnmatched}>
        <Dialog.Portal>
          <Dialog.Backdrop className="car-dialog-backdrop" />
          <Dialog.Popup className="modal">
            <div className="dialog review-dialog">
              <header>
                <div>
                  <Dialog.Title className="dialog-title">
                    {detailCopy('上一轮歧义对白人工复核', 'Prior ambiguous dialogue review')}
                  </Dialog.Title>
                  <p>
                    {detailCopy(
                      '保留精确 RT2 标签、候选车与最终归属，供人工复核',
                      'Exact RT2 labels, candidate cars and final assignments retained for manual review',
                    )}
                  </p>
                </div>
                <Dialog.Close className="close" aria-label={detailCopy('关闭', 'Close')}>
                  <X size={18} />
                </Dialog.Close>
              </header>
              <div className="list review-list">
                {filteredUnmatched.map((dialogue) => (
                  <div key={dialogue.id} className="entry">
                    <div className="speaker">
                      {dialogue.avatar ? (
                        <img className="avatar photo" src={avatarUrl(dialogue.avatar)} alt={dialogue.speaker} />
                      ) : (
                        <span className="avatar" style={{ background: avatarColor(dialogue.speaker) }}>
                          {initials(dialogue.speaker)}
                        </span>
                      )}
                      <span className="speaker-copy">
                        <span className="who">{speakerName(dialogue)}</span>
                        <span className="role">{speakerRole(dialogue)}</span>
                      </span>
                      <span className="id">{dialogue.id}</span>
                    </div>
                    {activeLocales.map((code) => (
                      <div
                        key={code}
                        className={`dialogue-text ${dialogueLocaleClass(code)}`}
                        dir={languageMeta(code).rtl ? 'rtl' : 'ltr'}
                        lang={code}
                      >
                        {bilingual && <span className="translation-language">{languageName(code)}</span>}
                        {dialogueText(dialogue, code)}
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            </div>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>

      <Dialog.Root open={showImageDownloader} onOpenChange={setShowImageDownloader}>
        <Dialog.Portal>
          <Dialog.Backdrop className="car-dialog-backdrop" />
          <Dialog.Popup className="modal">
            <div className="dialog downloader-dialog">
              <ImageDownloader
                catalog={data}
                mirrors={mirrorCatalogs}
                onBack={() => setShowImageDownloader(false)}
              />
            </div>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>

      <Dialog.Root
        open={Boolean(activeCar)}
        onOpenChange={(open) => {
          if (!open) closeCar()
        }}
      >
        <Dialog.Portal>
          <Dialog.Backdrop className="car-dialog-backdrop" />
          <Dialog.Popup
            className="modal"
            initialFocus={() => officialStageRef.current}
          >
            {activeCar && (
              <div className="dialog">
                <header>
                  <div className={`headmain ${activeCar.image ? '' : 'no-car-image'}`}>
                    {activeCar.image && (
                      <img className="modalcar" src={`${base}${activeCar.image}`} alt={activeCar.nameEn} />
                    )}
                    <div className="headcopy">
                      <Dialog.Title className="dialog-title">
                        {displayName(activeCar)}
                      </Dialog.Title>
                      <p>
                        {showZhContent
                          ? `${activeCar.nameEn} · ${displayBrand(activeCar)}`
                          : displayBrand(activeCar)}
                      </p>
                      {activeCar.specs && (
                        <div className="head-specs" aria-label="车辆核心规格">
                          {headerSpecFields.map((field) => (
                            <span key={field.key}>
                              <small>{detailCopy(field.labelZh, field.labelEn)}</small>
                              <b>{compactSpecValue(field.key) || '—'}</b>
                            </span>
                          ))}
                        </div>
                      )}
                      <p className="note">
                        {detailCopy('资料可用：', 'Available data: ')}
                        {carTextTabs
                          .filter((tab) => tabHasContent(activeCar, tab.id))
                          .map((tab) =>
                            uiLang === 'zh'
                              ? tab.labelZh
                              : uiLang === 'both'
                                ? `${tab.labelZh} · ${tab.labelEn}`
                                : tab.labelEn,
                          )
                          .join(' · ')}
                      </p>
                    </div>
                  </div>
                  <div className="detail-controls">
                    <LanguagePicker
                      compact
                      primary={primaryLang}
                      secondary={secondaryLang}
                      bilingual={bilingual}
                      onPrimary={setPrimaryLanguage}
                      onSecondary={setSecondaryLanguage}
                      onBilingual={toggleBilingual}
                    />
                    <div className="actions">
                      <Button
                        variant="outline"
                        className="copy"
                        onClick={async () => {
                          try {
                            await navigator.clipboard.writeText(location.href)
                            setCopyState('已复制')
                            window.setTimeout(() => setCopyState('复制链接'), 1500)
                          } catch {
                            setCopyState('复制失败')
                          }
                        }}
                      >
                        {copyState === '已复制'
                          ? detailCopy('已复制', 'Copied')
                          : copyState === '复制失败'
                            ? detailCopy('复制失败', 'Copy failed')
                            : detailCopy('复制链接', 'Copy link')}
                      </Button>
                      <Dialog.Close className="close" aria-label="关闭">
                        <X size={18} />
                      </Dialog.Close>
                    </div>
                  </div>
                </header>

                <Tabs.Root
                  value={activeTab}
                  onValueChange={(value) => setTab(value as TabId)}
                >
                  <Tabs.List
                    ref={tabListRef}
                    className="car-tabs"
                    aria-label="车辆资料"
                    activateOnFocus
                  >
                    {carTextTabs.map((tab) => (
                      <Tabs.Tab
                        key={tab.id}
                        value={tab.id}
                        data-tab={tab.id}
                        className={`${activeTab === tab.id ? 'on' : ''} ${
                          tabHasContent(activeCar, tab.id) ? '' : 'muted'
                        }`}
                      >
                        {tabLabel(tab)}
                      </Tabs.Tab>
                    ))}
                  </Tabs.List>

                  <Tabs.Panel value="official" className="official-panel">
                    {activeCar.officialDetails ? (
                      <div className="official-layout">
                        <div className="official-visual-pane">
                          <section
                            ref={(node) => {
                              officialStageRef.current = node
                            }}
                            className={`official-stage ${officialFullscreen ? 'is-fullscreen' : ''}`}
                            tabIndex={0}
                            onPointerDown={() => {
                              officialStageRef.current?.focus({ preventScroll: true })
                            }}
                            onDoubleClick={(event) => {
                              if ((event.target as HTMLElement).closest('button')) return
                              void toggleOfficialFullscreen()
                            }}
                          >
                          <div className="official-stage-media">
                            {currentOfficialImage && !officialImageError ? (
                              <img
                                src={assetUrl(currentOfficialImage)}
                                alt={`${displayName(activeCar)} ${officialSlideIndex + 1}`}
                                loading="eager"
                                decoding="async"
                                onError={() => setOfficialImageError(true)}
                              />
                            ) : (
                              <div className="official-no-image">
                                {officialImageError
                                  ? detailCopy(
                                      '当前浏览器无法解码这张 JPEG XL 图片',
                                      'This browser cannot decode this JPEG XL image',
                                    )
                                  : detailCopy('没有官方详情图片', 'No official gallery image')}
                              </div>
                            )}
                          </div>
                          {officialImagePaths.length > 0 && (
                            <div className="official-image-count">
                              4K · {officialSlideIndex + 1} / {officialImagePaths.length}
                            </div>
                          )}
                          <button
                            type="button"
                            className="official-fullscreen-button"
                            onClick={toggleOfficialFullscreen}
                            aria-label={officialFullscreen ? '退出全屏' : '全屏播放'}
                          >
                            {officialFullscreen ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
                          </button>
                          {officialImagePaths.length > 1 && (
                            <>
                              <button
                                type="button"
                                className="official-stage-arrow prev"
                                onClick={() => moveOfficialSlide(-1)}
                                aria-label="上一张"
                              >
                                ‹
                              </button>
                              <button
                                type="button"
                                className="official-stage-arrow next"
                                onClick={() => moveOfficialSlide(1)}
                                aria-label="下一张"
                              >
                                ›
                              </button>
                            </>
                          )}
                          <div className="official-caption" style={officialCaptionStyle}>
                            <span>
                              {displayBrand(activeCar)} · {activeCar.officialDetails.officialId}
                            </span>
                            <h3>{displayName(activeCar)}</h3>
                            {showZhContent && <p>{activeCar.officialDetails.heroZh}</p>}
                            {showEnContent && (
                              <p className="en">{activeCar.officialDetails.heroEn}</p>
                            )}
                            {showZhContent && (
                              <p className="official-caption-body">
                                {activeCar.officialDetails.descriptionZh}
                              </p>
                            )}
                            {showEnContent && (
                              <p className="official-caption-body en">
                                {activeCar.officialDetails.descriptionEn}
                              </p>
                            )}
                            <div
                              className="official-caption-drag"
                              onPointerDown={(event) => {
                                setOfficialCaptionDragging(true)
                                event.currentTarget.setPointerCapture?.(event.pointerId)
                              }}
                              onPointerMove={updateOfficialCaptionFromPointer}
                              onPointerUp={(event) => {
                                setOfficialCaptionDragging(false)
                                event.currentTarget.releasePointerCapture?.(event.pointerId)
                                saveOfficialTextPosition()
                              }}
                              onPointerCancel={() => {
                                setOfficialCaptionDragging(false)
                                saveOfficialTextPosition()
                              }}
                            >
                              <Move size={14} /> {detailCopy('拖动可调整文字位置', 'Drag to reposition text')}
                            </div>
                          </div>
                          <div className="official-fullscreen-controls">
                            <button
                              type="button"
                              onClick={() => setOfficialAutoplay(!officialAutoplay)}
                              aria-label={officialAutoplay ? '暂停' : '播放'}
                            >
                              {officialAutoplay ? <Pause size={16} /> : <Play size={16} />}
                            </button>
                            <label>
                              <span>{officialSpeed}×</span>
                              <input
                                type="range"
                                min="1"
                                max="10"
                                step="1"
                                value={officialSpeed}
                                onChange={(event) => setOfficialSpeed(Number(event.target.value))}
                              />
                            </label>
                            <label>
                              <span>{detailCopy('字号', 'Text')} {officialTextSize}</span>
                              <input
                                type="range"
                                min="12"
                                max="42"
                                step="1"
                                value={officialTextSize}
                                onChange={(event) => setOfficialTextSize(Number(event.target.value))}
                              />
                            </label>
                            <div className="official-position-buttons">
                              <button type="button" onClick={() => setOfficialCaptionPreset(24, 24)}>↖</button>
                              <button type="button" onClick={() => setOfficialCaptionPreset(76, 24)}>↗</button>
                              <button type="button" onClick={() => setOfficialCaptionPreset(24, 78)}>↙</button>
                              <button type="button" onClick={() => setOfficialCaptionPreset(76, 78)}>↘</button>
                            </div>
                            <button type="button" onClick={toggleOfficialFullscreen}>
                              {officialFullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
                            </button>
                          </div>
                          </section>
                          <div className="official-inline-controls" aria-label={detailCopy('幻灯片控制', 'Slideshow controls')}>
                            <button
                              type="button"
                              className={`official-play-toggle ${officialAutoplay ? 'on' : ''}`}
                              onClick={() => setOfficialAutoplay(!officialAutoplay)}
                            >
                              {officialAutoplay
                                ? detailCopy('暂停幻灯片', 'Pause slideshow')
                                : detailCopy('播放幻灯片', 'Play slideshow')}
                            </button>
                            <label>
                              <span>{detailCopy('播放速度', 'Speed')}</span>
                              <input
                                type="range"
                                min="1"
                                max="10"
                                step="1"
                                value={officialSpeed}
                                onChange={(event) => setOfficialSpeed(Number(event.target.value))}
                              />
                              <b>{officialSpeed}×</b>
                            </label>
                            <label>
                              <span>{detailCopy('文字大小', 'Text size')}</span>
                              <input
                                type="range"
                                min="12"
                                max="42"
                                step="1"
                                value={officialTextSize}
                                onChange={(event) => setOfficialTextSize(Number(event.target.value))}
                              />
                              <b>{officialTextSize}px</b>
                            </label>
                            <button
                              type="button"
                              className="official-inline-fullscreen"
                              onClick={toggleOfficialFullscreen}
                            >
                              <Maximize2 size={15} />
                              {detailCopy('全屏播放', 'Fullscreen')}
                            </button>
                          </div>
                        </div>
                        <aside className="official-copy">
                          <div className="official-copy-head">
                            <span className="official-kicker">
                              {detailCopy('官方车表介绍', 'Official car-list introduction')}
                            </span>
                            <h4>{displayName(activeCar)}</h4>
                            {showZhContent && (
                              <p className="official-hero">{activeCar.officialDetails.heroZh}</p>
                            )}
                            {showEnContent && (
                              <p className="official-hero">{activeCar.officialDetails.heroEn}</p>
                            )}
                          </div>
                          <div className="official-description">
                            {showZhContent && <p>{activeCar.officialDetails.descriptionZh}</p>}
                            {showEnContent && (
                              <p className="en">{activeCar.officialDetails.descriptionEn}</p>
                            )}
                          </div>
                          {officialMirrorCopies.some(
                            (copy) => copy.record?.hero || copy.record?.description,
                          ) && (
                            <section className="gt7-website-intro">
                              <h4>{detailCopy('GT7网站介绍', 'GT7 website introduction')}</h4>
                              {officialMirrorCopies.map(
                                (copy) =>
                                  copy.record && (
                                    <article key={`mirror-${copy.code}`}>
                                      {bilingual && <h5>{copy.label}</h5>}
                                      {copy.record.hero && (
                                        <p className="official-hero">{copy.record.hero}</p>
                                      )}
                                      {copy.record.description && (
                                        <p>{copy.record.description}</p>
                                      )}
                                    </article>
                                  ),
                              )}
                            </section>
                          )}
                          <a
                            className="official-source-link"
                            href={
                              showZhContent
                                ? activeCar.officialDetails.sourceUrlZh || activeCar.officialDetails.sourceUrlEn
                                : activeCar.officialDetails.sourceUrlEn || activeCar.officialDetails.sourceUrlZh
                            }
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {detailCopy('查看官方原始车表页面', 'Open official car-list page')} ↗
                          </a>
                        </aside>
                      </div>
                    ) : (
                      <div className="no-dialogue text-empty">
                        <strong>
                          {detailCopy(
                            '这辆车没有官方车表介绍',
                            'No official car-list introduction is available',
                          )}
                        </strong>
                        <span>
                          {detailCopy(
                            '仅收录当前官方车表中存在的车型。',
                            'Only cars present in the current official car list are included here.',
                          )}
                        </span>
                      </div>
                    )}
                  </Tabs.Panel>

                  <Tabs.Panel value="cafe" className="dialogues cafe-dialogues">
                    {activeCafeScenes.length ? (
                      activeCafeScenes.map((scene) => (
                        <section key={scene.key} className="cafe-scene">
                          <header className="scene-head">
                            <div>
                              <h4>{uiLang === 'zh' ? scene.titleZh : scene.titleEn}</h4>
                              <small>{scene.key}</small>
                            </div>
                          </header>
                          {scene.turns.map((turn) => (
                            <article key={`${scene.key}-${turn.id}`} className="dialogue">
                              <div className="speaker">
                                {turn.avatar ? (
                                  <img
                                    className="avatar photo"
                                    src={avatarUrl(turn.avatar)}
                                    alt={turn.speaker}
                                    loading="lazy"
                                  />
                                ) : (
                                  <span
                                    className="avatar"
                                    style={{ background: avatarColor(turn.speaker) }}
                                  >
                                    {initials(turn.speaker)}
                                  </span>
                                )}
                                <span className="speaker-copy">
                                  <span className="who">{speakerName(turn)}</span>
                                  <span className="role">{speakerRole(turn)}</span>
                                </span>
                                <span className="id">{turn.id}</span>
                              </div>
                              {activeLocales.map((code) => (
                                <div
                                  key={code}
                                  className={`dialogue-text ${dialogueLocaleClass(code)}`}
                                  dir={languageMeta(code).rtl ? 'rtl' : 'ltr'}
                                  lang={code}
                                >
                                  {bilingual && (
                                    <span className="translation-language">{languageName(code)}</span>
                                  )}
                                  {dialogueText(turn, code)}
                                </div>
                              ))}
                            </article>
                          ))}
                        </section>
                      ))
                    ) : (
                      <div className="no-dialogue">
                        <strong>
                          {detailCopy(
                            '没有找到这辆车的咖啡屋场景',
                            'No café scene was found for this car',
                          )}
                        </strong>
                        <span>
                          {detailCopy(
                            '已按 RT2 消息键和 DMS 场景表核对，未借用相邻车辆对白。',
                            'RT2 message keys and DMS scene tables were checked; no neighbouring-car dialogue was substituted.',
                          )}
                        </span>
                      </div>
                    )}
                  </Tabs.Panel>

                  <Tabs.Panel value="garageSpecs" className="garage-specs-panel">
                    {activeCar.specs ? (
                      <>
                        <div className="specs-heading">
                          <div>
                            <span className="specs-eyebrow">{detailCopy('官方车库数据', 'Official garage data')}</span>
                            <h4>{detailCopy('车辆规格', 'Vehicle specifications')}</h4>
                            <div className="class-pp">
                              <strong>{specValue('zh', 'carClass') || specValue('en', 'carClass') || '—'}</strong>
                              <span>{specValue('zh', 'performancePoint') || specValue('en', 'performancePoint')}</span>
                            </div>
                            <p>
                              {detailCopy(
                                '动力、尺寸与性能点来自 GT7 官方车表；中英文原始单位分别保留。',
                                'Power, dimensions and performance points come from the official GT7 car list; source units are preserved.',
                              )}
                            </p>
                          </div>
                          <a
                            className="spec-source-link"
                            href={
                              showZhContent
                                ? activeCar.specs.sourceUrlZh || activeCar.specs.sourceUrlEn
                                : activeCar.specs.sourceUrlEn || activeCar.specs.sourceUrlZh
                            }
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {detailCopy('查看官方车表', 'Official car entry')} ↗
                          </a>
                        </div>
                        <div className="garage-taxonomy">
                          {activeCar.garageDetails?.country && (
                            <div className="garage-country">
                              <small>{detailCopy('品牌国家', 'Brand country')}</small>
                              <b>
                                {uiLang === 'zh'
                                  ? activeCar.garageDetails.country.nameZh
                                  : uiLang === 'both'
                                    ? `${activeCar.garageDetails.country.nameZh} · ${activeCar.garageDetails.country.nameEn}`
                                    : activeCar.garageDetails.country.nameEn}
                              </b>
                              <span>{activeCar.garageDetails.country.code?.toUpperCase()}</span>
                            </div>
                          )}
                          {activeCar.garageDetails?.vehicleTypes?.length ? (
                            <div className="vehicle-types">
                              <small>{detailCopy('车辆类型', 'Vehicle type')}</small>
                              <div>
                                {activeCar.garageDetails.vehicleTypes.map((type) => (
                                  <span key={type.id} className="vehicle-type-tag">
                                    {uiLang === 'zh'
                                      ? type.tagZh
                                      : uiLang === 'both'
                                        ? `${type.tagZh} · ${type.tagEn}`
                                        : type.tagEn}
                                  </span>
                                ))}
                              </div>
                            </div>
                          ) : null}
                        </div>
                        <section className="specs-grid" aria-label="车辆详细规格">
                          {garageSpecFields.map((field) => (
                            <article
                              key={field.key}
                              className={`spec-card spec-${field.key}`}
                            >
                              <small>{detailCopy(field.labelZh, field.labelEn)}</small>
                              {showZhContent && (
                                <div className="spec-value">
                            {bilingual && (
                              <span className="spec-locale">
                                {languageName(activeLocales.find(isChineseLocale) || 'zh')}
                              </span>
                            )}
                                  <b>{specValue('zh', field.key) || '—'}</b>
                                </div>
                              )}
                              {showEnContent && (
                                <div className="spec-value en">
                                  {bilingual && <span className="spec-locale">EN</span>}
                                  <b>{specValue('en', field.key) || '—'}</b>
                                </div>
                              )}
                            </article>
                          ))}
                        </section>
                        {activeCar.specs.match === 'variant-base-alias' && (
                          <p className="spec-note">
                            {detailCopy(
                              '该特别版未单独列入官方车表，此处显示其明确对应基础车型的机械规格。',
                              'This special variant is not listed separately in the official car list; these are the specifications of its explicitly mapped base car.',
                            )}
                          </p>
                        )}
                        {!activeCar.garageDetails?.vehicleTypes?.length && (
                          <p className="spec-note">
                            {detailCopy(
                              '这辆车存在于当前官方车表，但本机游戏版本的 CarType.dmsb 中没有对应的车辆类型行。',
                              'This car exists in the current official list, but the local game build has no matching CarType.dmsb row.',
                            )}
                          </p>
                        )}
                      </>
                    ) : (
                      <div className="no-dialogue text-empty">
                        <strong>
                          {detailCopy(
                            '这辆车没有官方车库规格',
                            'No official garage specifications are available',
                          )}
                        </strong>
                        <span>
                          {detailCopy(
                            '已按官方车辆 ID、完整车型名称和明确变体别名核对，没有借用其他车辆数据。',
                            'Official vehicle IDs, full model names and explicit variant aliases were checked; no data from another car was substituted.',
                          )}
                        </span>
                      </div>
                    )}
                  </Tabs.Panel>

                  {(['brandCentral', 'vehicleCollection', 'usedCar', 'collection'] as const).map(
                    (tabId) => (
                      <Tabs.Panel key={tabId} value={tabId} className="text-panel">
                        {activeTextFor(tabId) ? (
                          <TextPanel
                            tabId={tabId}
                            text={activeTextFor(tabId)!}
                            showZh={showZhContent}
                            showEn={showEnContent}
                            detailCopy={detailCopy}
                            offer={activeOfferFor(tabId)}
                            collectionCopies={
                              tabId === 'vehicleCollection'
                                ? collectionCopiesFor(activeTextFor(tabId)!)
                                : []
                            }
                          />
                        ) : (
                          <div className="no-dialogue text-empty">
                            <strong>
                              {tabId === 'vehicleCollection'
                                ? detailCopy(
                                    '这辆车没有车辆收藏说明',
                                    'No Car Collection entry is available for this car',
                                  )
                                : detailCopy(
                                    '这辆车没有该商店的官方说明文本',
                                    'No official description is available for this source',
                                  )}
                            </strong>
                            <span>
                              {detailCopy(
                                '已检查 PS5 文本表，没有借用其他车辆内容。',
                                'The PS5 text tables were checked; no text from another car was substituted.',
                              )}
                            </span>
                          </div>
                        )}
                      </Tabs.Panel>
                    ),
                  )}
                </Tabs.Root>
                <div className="keyhint">
                  {detailCopy(
                    '← → 切换图片 · ↑ ↓ 切换车辆 · 1–7 或 Tab 切换资料页 · Esc 关闭',
                    '← → Change image · ↑ ↓ Change car · 1–7 or Tab switch tab · Esc close',
                  )}
                </div>
              </div>
            )}
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>

      {mirrorFullscreenOpen && mirrorSlideCars.length > 0 && (
        <MirrorFullscreen
          cars={mirrorSlideCars}
          initialCarId={mirrorStart.carId}
          initialImageIndex={mirrorStart.imageIndex}
          speed={officialSpeed}
          chinese={showZhContent}
          onSpeedChange={setOfficialSpeed}
          onClose={closeMirrorFullscreen}
        />
      )}

      <footer className="foot">
        <a href={`${base}data/cafe_dialogues.csv`} download>
          {detailCopy('下载全部对白 CSV', 'Download all dialogue CSV')}
        </a>
        {' · '}
        <a href={`${base}data/cafe_scene_dialogues.csv`} download>
          {detailCopy('下载完整场景 CSV', 'Download scene dialogue CSV')}
        </a>
        {' · '}
        <a href={`${base}data/vehicle_collection_texts.csv`} download>
          {detailCopy('车辆收藏中英文本 CSV', 'Car Collection text CSV')}
        </a>
        {' · '}
        <a href={`${base}data/garage_specs.csv`} download>
          {detailCopy('车库规格 CSV', 'Garage specifications CSV')}
        </a>
        {' · '}
        <a href={`${base}data/garage_details.json`} download>
          {detailCopy('车库与收藏管理 JSON', 'Garage and collection JSON')}
        </a>
        {' · '}
        <a href={`${base}data/menu_books.json`} download>
          {detailCopy('菜单书 JSON', 'Menu books JSON')}
        </a>
        <div className="credits">
          {detailCopy(
            '数据从本机 PS5 GT7 资源解包；头像来自 IDP0/TXS5 面部图集，未使用占位头像。',
            'Data was extracted from the local PS5 GT7 resources; portraits come from IDP0/TXS5 face atlases without placeholder avatars.',
          )}
        </div>
      </footer>
    </div>
  )

  function collectionCopiesFor(text: CarText) {
    return activeLocales.map((code) => {
      const table = vehicleCollectionI18n[code]
      return {
        code,
        label: languageName(code),
        catch: text.textIds?.catch
          ? table?.DescriptionCatch?.[text.textIds.catch]
          : undefined,
        long: text.textIds?.long
          ? table?.DescriptionLong?.[text.textIds.long]
          : undefined,
      }
    })
  }

  function mirrorCarFor(car: Car, code: string): MirrorCar | null {
    const catalog = mirrorCatalogs[code]
    if (!catalog) return null
    const officialId = car.officialId || car.officialDetails?.officialId
    if (officialId) {
      const exact = catalog.cars.find((item) => item.id === officialId)
      if (exact) return exact
    }
    const normalize = (value: string) =>
      value.toLowerCase().replace(/[^a-z0-9]+/g, '')
    const name = normalize(car.nameEn)
    return (
      catalog.cars.find(
        (item) =>
          normalize(item.name_long) === name || normalize(item.model) === name,
      ) || null
    )
  }

  function mirrorCopiesFor(car: Car) {
    return activeLocales.map((code) => ({
      code,
      label: languageName(code),
      record: mirrorCarFor(car, code),
    }))
  }

  function activeTextFor(tabId: TabId): CarText | null {
    if (!activeCar) return null
    return (activeCar.texts?.[tabId] as CarText | undefined) || null
  }

  function activeOfferFor(tabId: TabId) {
    return (activeCar?.dealer?.[tabId] as Record<string, unknown> | undefined) || null
  }
}

function TextPanel({
  tabId,
  text,
  showZh,
  showEn,
  detailCopy,
  offer,
  collectionCopies,
}: {
  tabId: TabId
  text: CarText
  showZh: boolean
  showEn: boolean
  detailCopy: (zh: string, en: string) => string
  offer: Record<string, unknown> | null
  collectionCopies: Array<{
    code: string
    label: string
    catch?: string
    long?: string
  }>
}) {
  return (
    <>
      {offer && (
        <div className="dealer-offer">
          <strong>Cr. {Number(offer.price || 0).toLocaleString('zh-CN')}</strong>
          {Number(offer.discountPrice || 0) > 0 && (
            <span>
              {detailCopy('优惠', 'Discount')} Cr. {Number(offer.discountPrice).toLocaleString('zh-CN')}
            </span>
          )}
          <span>{detailCopy('稀有度', 'Rarity')} {String(offer.rarity || '')}</span>
        </div>
      )}
      {tabId === 'vehicleCollection' ? (
        <section className="collection-part collection-main-copy">
          {collectionCopies.some((copy) => copy.long)
            ? collectionCopies
                .filter((copy) => copy.long)
                .map((copy) => (
                  <div className="text-copy" key={`${copy.code}-long`}>
                    <p>{copy.long}</p>
                  </div>
                ))
            : (
                <>
                  {showZh && (
                    <div className="text-copy zh-copy">
                      <p>{text.long?.zh || text.zh}</p>
                    </div>
                  )}
                  {showEn && (
                    <div className="text-copy en-copy">
                      <p>{text.long?.en || text.en}</p>
                    </div>
                  )}
                </>
              )}
        </section>
      ) : (
        <>
          {showZh && (
            <div className="text-copy zh-copy">
              <p>{text.zh}</p>
            </div>
          )}
          {showEn && (
            <div className="text-copy en-copy">
              <p>{text.en}</p>
            </div>
          )}
        </>
      )}
      {Boolean(text.variants?.length) && (
        <section className="text-variants">
          <h4>{detailCopy('补充介绍', 'Additional description')}</h4>
          {(text.variants as CarText[]).map((variant) => (
            <article key={String(variant.textId || variant.carKey || variant.sourceName)} className="text-variant">
              {showZh && (
                <div className="text-copy zh-copy">
                  <p>{variant.zh}</p>
                </div>
              )}
              {showEn && (
                <div className="text-copy en-copy">
                  <p>{variant.en}</p>
                </div>
              )}
            </article>
          ))}
        </section>
      )}
    </>
  )
}
