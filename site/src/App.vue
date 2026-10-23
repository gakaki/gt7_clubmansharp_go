<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'

const data = ref(null)
const base = import.meta.env.BASE_URL
const languageOptions = [
  { id: 'ar', native: 'العربية', english: 'Arabic', zh: '阿拉伯语', rtl: true },
  { id: 'cn', native: '简体中文', english: 'Chinese (Simplified)', zh: '简体中文' },
  { id: 'cz', native: 'Čeština', english: 'Czech', zh: '捷克语' },
  { id: 'de', native: 'Deutsch', english: 'German', zh: '德语' },
  { id: 'es', native: 'Español', english: 'Spanish', zh: '西班牙语' },
  { id: 'fr', native: 'Français', english: 'French', zh: '法语' },
  { id: 'gb', native: 'English (UK)', english: 'English (UK)', zh: '英语（英国）' },
  { id: 'it', native: 'Italiano', english: 'Italian', zh: '意大利语' },
  { id: 'jp', native: '日本語', english: 'Japanese', zh: '日语' },
  { id: 'kr', native: '한국어', english: 'Korean', zh: '韩语' },
  { id: 'nl', native: 'Nederlands', english: 'Dutch', zh: '荷兰语' },
  { id: 'pl', native: 'Polski', english: 'Polish', zh: '波兰语' },
  { id: 'pt', native: 'Português', english: 'Portuguese', zh: '葡萄牙语' },
  { id: 'ru', native: 'Русский', english: 'Russian', zh: '俄语' },
  { id: 'th', native: 'ไทย', english: 'Thai', zh: '泰语' },
  { id: 'tr', native: 'Türkçe', english: 'Turkish', zh: '土耳其语' },
  { id: 'tw', native: '繁體中文', english: 'Chinese (Traditional)', zh: '繁体中文' },
  { id: 'us', native: 'English (US)', english: 'English (US)', zh: '英语（美国）' },
  { id: 'zh', native: '中文（简体）', english: 'Chinese', zh: '中文' },
]
const languageIds = new Set(languageOptions.map((language) => language.id))
const initialParams = new URLSearchParams(location.search)
const explicitLang = initialParams.get('lang')
const legacyLang = explicitLang || localStorage.getItem('gt7cafe.lang')
const legacyPrimary = legacyLang === 'both'
  ? 'zh'
  : legacyLang === 'en'
    ? 'us'
    : legacyLang
const initialPrimary = initialParams.get('l1')
  || (explicitLang && explicitLang !== 'both' ? legacyPrimary : null)
  || localStorage.getItem('gt7cafe.primaryLang')
  || legacyPrimary
  || 'zh'
const primaryLang = ref(languageIds.has(initialPrimary) ? initialPrimary : 'zh')
const initialSecondary = initialParams.get('l2')
  || localStorage.getItem('gt7cafe.secondaryLang')
  || 'en'
const secondaryLang = ref(
  languageIds.has(initialSecondary) && initialSecondary !== primaryLang.value
    ? initialSecondary
    : (primaryLang.value === 'en' ? 'zh' : 'en'),
)
const bilingual = ref(
  explicitLang
    ? explicitLang === 'both'
    : localStorage.getItem('gt7cafe.bilingual') === '1',
)
const lang = computed(() =>
  bilingual.value ? 'both' : (isChineseLocale(primaryLang.value) ? 'zh' : 'en'),
)
const showZhContent = computed(() => activeLocales().some(isChineseLocale))
const showEnContent = computed(() =>
  activeLocales().some((code) => !isChineseLocale(code)) || !showZhContent.value,
)
const dialogueTranslations = ref({})
const loadingLanguages = ref(new Set())
const query = ref('')
const searchInput = ref('')
const brandFilter = ref('ALL')
const speakerFilter = ref('ALL')
const sourceFilter = ref('all')
const activeCar = ref(null)
const activeIndex = ref(-1)
const showUnmatched = ref(false)
const showCast = ref(false)
const activeTab = ref('cafe')

// A few RT2 source strings contain undecodable glyph slots. Never expose the
// U+FFFD replacement box in the research UI; keep the surrounding sentence
// intact while omitting only the unknown code point.
function sanitizeGlyphs(value) {
  if (typeof value === 'string') {
    return value
      .replace(/\uFFFD+/g, '')
      .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
      .replace(/[ \t]{2,}/g, ' ')
      .replace(/ +\n/g, '\n')
  }
  if (Array.isArray(value)) return value.map(sanitizeGlyphs)
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, sanitizeGlyphs(item)]),
    )
  }
  return value
}

const carTextTabs = [
  { id: 'cafe', labelZh: '咖啡屋对话', labelEn: 'Cafe dialogue' },
  { id: 'brandCentral', labelZh: '品牌购物', labelEn: 'Brand Central' },
  { id: 'vehicleCollection', labelZh: '车辆收藏', labelEn: 'Car Collection' },
  { id: 'garageSpecs', labelZh: '车库数据', labelEn: 'Garage Specs' },
  { id: 'usedCar', labelZh: '二手车', labelEn: 'Used Cars' },
  { id: 'collection', labelZh: '收藏馆', labelEn: 'Hagerty Collection' },
]
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
]
const textMerchants = {
  brandCentral: {
    nameZh: '品牌中心顾问',
    nameEn: 'Brand Central Advisor',
    roleZh: '品牌中心车辆顾问',
    roleEn: 'Brand Central vehicle advisor',
    avatar: 'avatars/extracted/brand-central-advisor.png',
  },
  vehicleCollection: {
    nameZh: '车辆收藏',
    nameEn: 'Car Collection',
    roleZh: 'PS5 RT2 × 官方车表说明',
    roleEn: 'PS5 RT2 × official car-list entry',
    avatar: '',
  },
  usedCar: {
    nameZh: '二手车商',
    nameEn: 'Used Car Dealer',
    roleZh: '二手车商',
    roleEn: 'Used car dealer',
    avatar: 'avatars/extracted/used-car-owner.png',
  },
  collection: {
    nameZh: '麦基尔·哈格蒂',
    nameEn: 'McKeel Hagerty',
    roleZh: 'Hagerty 收藏馆',
    roleEn: 'Hagerty Collection',
    avatar: 'avatars/extracted/hagerty.png',
  },
}

let searchTimer
watch(searchInput, (v) => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => { query.value = v }, 160)
})

onMounted(async () => {
  const res = await fetch(`${import.meta.env.BASE_URL}data/cars.json`)
  data.value = sanitizeGlyphs(await res.json())
  await ensureSelectedLanguages()
  window.__gt7LanguageState = () => ({
    primary: primaryLang.value,
    secondary: secondaryLang.value,
    bilingual: bilingual.value,
    loaded: Object.keys(dialogueTranslations.value),
    loading: [...loadingLanguages.value],
    sample: dialogueTranslations.value[primaryLang.value]
      ? Object.keys(dialogueTranslations.value[primaryLang.value]).slice(0, 3)
      : [],
  })
  const params = new URLSearchParams(location.search)
  const wantedSource = params.get('filter')
  if (sourceFilters.some((filter) => filter.id === wantedSource)) {
    sourceFilter.value = wantedSource
  }
  const wanted = params.get('car')
  if (wanted) {
    const car = data.value.cars.find((c) => c.id.toLowerCase() === wanted.toLowerCase())
    if (car) {
      activeCar.value = car
      activeIndex.value = navigable.value.findIndex((item) => item.id === car.id)
      const wantedTab = params.get('tab')
      activeTab.value = carTextTabs.some((tab) => tab.id === wantedTab)
        ? wantedTab
        : firstAvailableTab(car)
    }
  }
  if (params.get('review') === 'unmatched') openUnmatched()
  if (params.get('cast') === 'all') showCast.value = true
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

function onKey(e) {
  const editing = ['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target?.tagName)
  if (editing) {
    if (e.key === 'Escape') {
      e.target.blur()
      if (showCast.value) showCast.value = false
      else if (activeCar.value) closeCar()
    }
    return
  }
  if (e.key === 'Escape') {
    if (showUnmatched.value) {
      toggleUnmatched()
      return
    }
    if (showCast.value) {
      showCast.value = false
      return
    }
    closeCar()
    return
  }
  if (['ArrowDown', 'ArrowRight'].includes(e.key)) {
    e.preventDefault()
    move(1)
  } else if (['ArrowUp', 'ArrowLeft'].includes(e.key)) {
    e.preventDefault()
    move(-1)
  } else if (activeCar.value && e.key === 'Tab') {
    e.preventDefault()
    cycleTab(e.shiftKey ? -1 : 1)
  } else if (activeCar.value && ['1', '2', '3', '4', '5', '6'].includes(e.key)) {
    e.preventDefault()
    setActiveTab(carTextTabs[Number(e.key) - 1].id)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    openActive()
  } else if (e.key === '/') {
    e.preventDefault()
    document.querySelector('.search')?.focus()
  }
}
function isChineseLocale(code) {
  return ['zh', 'cn', 'tw'].includes(code)
}

function languageMeta(code) {
  return languageOptions.find((language) => language.id === code) || languageOptions[0]
}

function languageName(code) {
  const language = languageMeta(code)
  return lang.value === 'zh' ? language.zh : language.native
}

function activeLocales() {
  return bilingual.value
    ? [primaryLang.value, secondaryLang.value]
    : [primaryLang.value]
}

async function ensureLanguage(code) {
  if (!code || dialogueTranslations.value[code] || loadingLanguages.value.has(code)) return
  loadingLanguages.value = new Set([...loadingLanguages.value, code])
  try {
    const dir = data.value?.dialogueTranslationDir || 'data/dialogue_translations'
    const response = await fetch(`${base}${dir}/${code}.json`)
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    dialogueTranslations.value = {
      ...dialogueTranslations.value,
      [code]: await response.json(),
    }
  } catch (error) {
    console.warn(`Unable to load dialogue language ${code}`, error)
  } finally {
    const next = new Set(loadingLanguages.value)
    next.delete(code)
    loadingLanguages.value = next
  }
}

async function ensureSelectedLanguages() {
  for (const code of activeLocales()) {
    await ensureLanguage(code)
  }
}

function syncLanguageSettings() {
  localStorage.setItem('gt7cafe.primaryLang', primaryLang.value)
  localStorage.setItem('gt7cafe.secondaryLang', secondaryLang.value)
  localStorage.setItem('gt7cafe.bilingual', bilingual.value ? '1' : '0')
  localStorage.setItem('gt7cafe.lang', bilingual.value ? 'both' : primaryLang.value)
  const url = new URL(location.href)
  url.searchParams.set('lang', bilingual.value ? 'both' : primaryLang.value)
  url.searchParams.set('l1', primaryLang.value)
  if (bilingual.value) url.searchParams.set('l2', secondaryLang.value)
  else url.searchParams.delete('l2')
  history.replaceState(null, '', url)
  ensureSelectedLanguages()
}

function setPrimaryLanguage(code) {
  primaryLang.value = languageIds.has(code) ? code : 'zh'
  if (secondaryLang.value === primaryLang.value) {
    secondaryLang.value = primaryLang.value === 'en' ? 'zh' : 'en'
  }
  syncLanguageSettings()
}

function setSecondaryLanguage(code) {
  if (!languageIds.has(code) || code === primaryLang.value) return
  secondaryLang.value = code
  syncLanguageSettings()
}

function toggleBilingual() {
  bilingual.value = !bilingual.value
  syncLanguageSettings()
}

const cars = computed(() => data.value?.cars ?? [])
const stats = computed(() => data.value?.stats ?? { cars: 0, withDialogue: 0, dialogues: 0, unmatched: 0 })
const carTextStats = computed(() => data.value?.carTextStats ?? {
  entries: 0,
  mappedEntries: 0,
  carsWithAnyText: 0,
})
const unmatched = computed(() => data.value?.unmatched ?? [])
const legacyReviewStats = computed(() => ({
  total: unmatched.value.length,
  resolved: data.value?.stats?.legacyReviewResolved ?? 0,
}))
const speakers = computed(() => data.value?.speakers ?? [])
const cast = computed(() => data.value?.cast ?? [])

const brands = computed(() => {
  const map = new Map()
  for (const c of cars.value) map.set(c.brand, (map.get(c.brand) || 0) + 1)
  return [...map.entries()]
    .sort((a, b) => a[0].localeCompare(b[0], 'en'))
    .map(([name, count]) => ({ name, count }))
})

const brandLogos = computed(() => {
  const map = {}
  for (const c of cars.value) {
    if (c.brandLogo && !map[c.brand]) map[c.brand] = c.brandLogo
  }
  return map
})

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  return cars.value.filter((c) => {
    if (brandFilter.value !== 'ALL' && c.brand !== brandFilter.value) return false
    const carDialogues = visibleDialogues(c)
    const carConversation = visibleConversation(c)
    if (sourceFilter.value === 'cafe' && !carConversation.length) return false
    if (
      sourceFilter.value !== 'all' &&
      sourceFilter.value !== 'cafe' &&
      !c.texts?.[sourceFilter.value]
    ) {
      return false
    }
    if (speakerFilter.value !== 'ALL' && !carConversation.length) return false
    if (!q) return true
    const base = `${c.nameEn} ${c.nameZh} ${c.brand}`.toLowerCase()
    if (base.includes(q)) return true
    const dialogueHit = carDialogues.some((d) =>
      `${d.en} ${d.zh} ${d.speaker} ${d.speakerZh}`.toLowerCase().includes(q),
    ) || carConversation.some((d) =>
      `${d.en} ${d.zh} ${d.speaker} ${d.speakerZh}`.toLowerCase().includes(q),
    )
    if (dialogueHit) return true
    return allTextEntries(c).some((text) =>
      `${text.en} ${text.zh} ${text.titleEn} ${text.titleZh}`.toLowerCase().includes(q),
    )
  })
})

function setSourceFilter(id) {
  sourceFilter.value = id
  const url = new URL(location.href)
  if (id === 'all') url.searchParams.delete('filter')
  else url.searchParams.set('filter', id)
  history.replaceState(null, '', url)
}

function sourceFilterLabel(filter) {
  return lang.value === 'zh'
    ? filter.labelZh
    : lang.value === 'both'
      ? `${filter.labelZh} · ${filter.labelEn}`
      : filter.labelEn
}

function visibleDialogues(c) {
  if (speakerFilter.value === 'ALL') return c.dialogues
  return c.dialogues.filter((d) => d.speaker === speakerFilter.value)
}
function visibleConversation(c) {
  const turns = c.conversation?.length ? c.conversation : (c.dialogues || [])
  if (speakerFilter.value === 'ALL') return turns
  return turns.filter((d) => d.speaker === speakerFilter.value)
}
function allTextEntries(car) {
  return Object.values(car?.texts || {}).flatMap((text) => [
    text,
    ...(text.variants || []),
  ])
}

const grouped = computed(() => {
  const g = new Map()
  for (const c of filtered.value) {
    if (!g.has(c.brand)) g.set(c.brand, [])
    g.get(c.brand).push(c)
  }
  return [...g.entries()]
})

const navigable = computed(() => filtered.value)
const navCar = computed(() =>
  activeCar.value || navigable.value[activeIndex.value] || null,
)
const navPosition = computed(() => {
  if (!navCar.value) return 0
  const index = navigable.value.findIndex((car) => car.id === navCar.value.id)
  return index < 0 ? 0 : index + 1
})
const filteredUnmatched = computed(() => {
  const q = query.value.trim().toLowerCase()
  return unmatched.value.filter((d) => {
    if (speakerFilter.value !== 'ALL' && d.speaker !== speakerFilter.value) return false
    if (!q) return true
    return `${d.en} ${d.zh} ${d.speaker} ${d.speakerZh} ${d.car || ''} ${d.resolvedCarNameEn || ''} ${d.resolvedCarNameZh || ''}`
      .toLowerCase()
      .includes(q)
  })
})

watch(navigable, (list) => {
  if (!activeCar.value) return
  activeIndex.value = list.findIndex((car) => car.id === activeCar.value.id)
})
watch(activeTab, async (tab) => {
  await nextTick()
  document
    .querySelector(`.car-tabs button[data-tab="${tab}"]`)
    ?.scrollIntoView({ block: 'nearest', inline: 'center' })
})

function move(delta, openIfClosed = false) {
  const list = navigable.value
  if (!list.length) return
  activeIndex.value = activeIndex.value < 0
    ? (delta > 0 ? 0 : list.length - 1)
    : (activeIndex.value + delta + list.length) % list.length
  if (activeCar.value) {
    const c = list[activeIndex.value]
    activeCar.value = {
      ...c,
      dialogues: visibleDialogues(c),
      conversation: visibleConversation(c),
    }
    const nextTab = carTextTabs.find(
      (tab) => tab.id === activeTab.value && tabHasContent(activeCar.value, tab.id),
    )?.id || firstAvailableTab(activeCar.value)
    setActiveTab(nextTab)
    setCarParam(c.id)
  } else if (openIfClosed) {
    openCar(list[activeIndex.value])
  }
  if (!activeCar.value) requestAnimationFrame(() => {
    document.querySelector('.card.active')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  })
}
function stepMobile(delta) {
  move(delta, true)
}
function toggleNavCar() {
  if (activeCar.value) closeCar()
  else if (navCar.value) openCar(navCar.value)
}
function openActive() {
  const c = navigable.value[activeIndex.value]
  if (c) openCar(c)
}
function openCar(c) {
  const index = navigable.value.findIndex((x) => x.id === c.id)
  activeIndex.value = index
  activeCar.value = {
    ...c,
    dialogues: visibleDialogues(c),
    conversation: visibleConversation(c),
  }
  activeTab.value = firstAvailableTab(activeCar.value)
  setCarParam(c.id)
  const url = new URL(location.href)
  url.searchParams.delete('review')
  history.replaceState(null, '', url)
}
function openCarById(id) {
  const car = cars.value.find((item) => item.id === id)
  if (car) openCar(car)
}
function openUnmatched() {
  showUnmatched.value = true
  showCast.value = false
  if (activeCar.value) closeCar()
  const url = new URL(location.href)
  url.searchParams.set('review', 'unmatched')
  history.replaceState(null, '', url)
}
function toggleUnmatched() {
  if (!showUnmatched.value) {
    openUnmatched()
    return
  }
  showUnmatched.value = false
  const url = new URL(location.href)
  url.searchParams.delete('review')
  history.replaceState(null, '', url)
}
function closeCar() {
  activeCar.value = null
  setCarParam(null)
}
function setCarParam(id) {
  const url = new URL(location.href)
  if (id) url.searchParams.set('car', id)
  else url.searchParams.delete('car')
  history.replaceState(null, '', url)
}
async function copyLink() {
  try {
    await navigator.clipboard.writeText(location.href)
    copyState.value = '已复制'
    setTimeout(() => { copyState.value = '复制链接' }, 1500)
  } catch {
    copyState.value = '复制失败'
  }
}
const copyState = ref('复制链接')

function tabHasContent(car, tabId) {
  if (tabId === 'cafe') return Boolean(car?.conversation?.length || car?.dialogues?.length)
  if (tabId === 'garageSpecs') {
    return Boolean(
      car?.specs?.en?.performancePoint
      || car?.specs?.zh?.performancePoint
      || car?.specs?.en?.power
      || car?.specs?.zh?.power,
    )
  }
  return Boolean(car?.texts?.[tabId]?.en || car?.texts?.[tabId]?.zh)
}

function firstAvailableTab(car) {
  return carTextTabs.find((tab) => tabHasContent(car, tab.id))?.id || 'cafe'
}

function setActiveTab(tabId) {
  activeTab.value = tabId
  const url = new URL(location.href)
  url.searchParams.set('tab', tabId)
  history.replaceState(null, '', url)
}
function cycleTab(delta) {
  const current = carTextTabs.findIndex((tab) => tab.id === activeTab.value)
  const next = (current + delta + carTextTabs.length) % carTextTabs.length
  setActiveTab(carTextTabs[next].id)
}

function tabLabel(tab) {
  const label = lang.value === 'zh'
    ? tab.labelZh
    : lang.value === 'both'
      ? `${tab.labelZh} · ${tab.labelEn}`
      : tab.labelEn
  if (!activeCar.value) return label
  if (tab.id === 'cafe') {
    return `${label} · ${activeCar.value.conversation?.length || 0}`
  }
  return `${label} · ${tabHasContent(activeCar.value, tab.id) ? '有' : '无'}`
}

function detailCopy(zh, en) {
  if (lang.value === 'zh') return zh
  if (lang.value === 'en') return en
  return `${zh} · ${en}`
}

function textCategoryCount(car) {
  return carTextTabs.filter(
    (tab) => tab.id !== 'cafe' && tabHasContent(car, tab.id),
  ).length
}

function cafeSceneGroups(car) {
  const groups = []
  const byKey = new Map()
  for (const turn of car?.conversation || []) {
    const key = turn.sceneKey || turn.sceneId || 'scene'
    if (!byKey.has(key)) {
      const group = { key, titleZh: '', titleEn: '', turns: [] }
      byKey.set(key, group)
      groups.push(group)
    }
    byKey.get(key).turns.push(turn)
  }
  for (const group of groups) {
    const speakers = [...new Set(group.turns.map((turn) => turn.speaker))]
    const guest = speakers.find((name) => !['Sarah', 'Luca'].includes(name)) || speakers[0] || 'GT Cafe'
    group.titleEn = speakers.includes('Sarah') && guest !== 'Sarah'
      ? `${guest} and Sarah`
      : guest
    const guestZh = group.turns.find((turn) => turn.speaker === guest)?.speakerZh
    const sarahZh = group.turns.find((turn) => turn.speaker === 'Sarah')?.speakerZh
    group.titleZh = speakers.includes('Sarah') && guest !== 'Sarah'
      ? `${guestZh || guest} 与 ${sarahZh || 'Sarah'}`
      : guestZh || guest
    group.visibleTurns = speakerFilter.value === 'ALL'
      ? group.turns
      : group.turns.filter((turn) => turn.speaker === speakerFilter.value)
  }
  return groups.filter((group) => group.visibleTurns.length)
}

const activeCafeScenes = computed(() => cafeSceneGroups(activeCar.value))

function sceneTitle(scene) {
  return lang.value === 'zh' ? scene.titleZh : scene.titleEn
}

function activeText() {
  if (!activeCar.value || activeTab.value === 'cafe') return null
  return activeCar.value.texts?.[activeTab.value] || null
}
function specValue(locale, key) {
  return activeCar.value?.specs?.[locale]?.[key] || ''
}
function compactSpecValue(key) {
  const zh = specValue('zh', key)
  const en = specValue('en', key)
  if (showZhContent.value && showEnContent.value) {
    return zh && en && zh !== en ? `${zh} · ${en}` : (zh || en)
  }
  return showZhContent.value ? zh : en
}
function specLabel(field) {
  return detailCopy(field.labelZh, field.labelEn)
}
function activeSpecsSourceUrl() {
  const specs = activeCar.value?.specs
  if (!specs) return ''
  return showZhContent.value
    ? (specs.sourceUrlZh || specs.sourceUrlEn || '')
    : (specs.sourceUrlEn || specs.sourceUrlZh || '')
}
function activeOffer() {
  if (!activeCar.value || activeTab.value === 'cafe') return null
  return activeCar.value.dealer?.[activeTab.value] || null
}
function activeMerchant() {
  return textMerchants[activeTab.value] || null
}
function formatPrice(value) {
  return Number(value || 0).toLocaleString('zh-CN')
}
function initials(name) {
  return name.split(/\s+/).map((w) => w[0]).join('').slice(0, 2).toUpperCase()
}
function avatarColor(name) {
  let h = 0
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0
  return `hsl(${h % 360} 42% 46%)`
}
function avatarUrl(path) {
  return path ? `${base}${path}?v=20261023c` : ''
}

const letters = computed(() => {
  const set = new Set(brands.value.map((b) => b.name[0].toUpperCase()))
  return [...set].sort()
})

function displayName(c) {
  return isChineseLocale(primaryLang.value) ? c.nameZh : c.nameEn
}
function carNameLocales() {
  const groups = new Set()
  return activeLocales().filter((code) => {
    const group = isChineseLocale(code) ? 'zh' : 'en'
    if (groups.has(group)) return false
    groups.add(group)
    return true
  })
}
function dialogueText(turn, code) {
  const translated = dialogueTranslations.value[code]?.[turn.id]
  if (translated) return translated
  if (isChineseLocale(code)) return turn.zh || turn.en || ''
  if (code === 'us' || code === 'gb') return turn.en || turn.zh || ''
  return turn.en || turn.zh || ''
}
function dialogueLocaleClass(code) {
  if (isChineseLocale(code) || ['jp', 'kr'].includes(code)) return 'cjk'
  if (code === 'ar') return 'rtl'
  return 'latin'
}
function dialogueDirection(code) {
  return languageMeta(code).rtl ? 'rtl' : 'ltr'
}
function speakerName(d) {
  if (lang.value === 'zh') return d.speakerZh || d.speaker
  if (lang.value === 'both') return `${d.speakerZh || d.speaker} · ${d.speaker}`
  return d.speaker
}
function speakerRole(d) {
  return lang.value === 'zh' ? d.roleZh : d.roleEn
}
function speakerLabel(s) {
  if (lang.value === 'zh') return s.zh
  if (lang.value === 'both') return `${s.zh} · ${s.en}`
  return s.en
}
function filterBySpeaker(s) {
  speakerFilter.value = s.id
  showCast.value = false
  nextTick(() => document.querySelector('.content')?.scrollIntoView({ behavior: 'smooth' }))
}
function jumpTo(letter) {
  const first = brands.value.find((b) => b.name[0].toUpperCase() === letter)
  if (!first) return
  document.getElementById(`brand-${first.name}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}
function yearOf(name) {
  const m = name.match(/'(\d\d)$/)
  return m ? `'${m[1]}` : ''
}
</script>

<template>
  <div class="app">
    <header class="topbar">
      <div class="brand">
        <span class="gt">GT</span>
        <div class="titles">
          <h1>咖啡屋对话库</h1>
          <p>Gran Turismo 7 · GT Café Dialogues</p>
        </div>
      </div>
      <a class="menu-archive-link" href="/menus_cafe">菜单书档案</a>
      <div class="language-picker" role="group" aria-label="对白语言">
        <select
          :value="primaryLang"
          aria-label="第一语言"
          @change="setPrimaryLanguage($event.target.value)"
        >
          <option v-for="language in languageOptions" :key="language.id" :value="language.id">
            {{ language.native }}
          </option>
        </select>
        <button
          type="button"
          class="bilingual-toggle"
          :class="{ on: bilingual }"
          :aria-pressed="bilingual"
          @click="toggleBilingual"
        >
          双语
        </button>
        <select
          v-if="bilingual"
          :value="secondaryLang"
          aria-label="第二语言"
          @change="setSecondaryLanguage($event.target.value)"
        >
          <option
            v-for="language in languageOptions.filter((item) => item.id !== primaryLang)"
            :key="language.id"
            :value="language.id"
          >
            {{ language.native }}
          </option>
        </select>
      </div>
    </header>

    <section class="summary">
      <div class="stat"><b>{{ stats.cars }}</b><span>收录车辆</span></div>
      <div class="stat"><b>{{ stats.withDialogue }}</b><span>有咖啡屋对白</span></div>
      <div class="stat"><b>{{ stats.dialogues }}</b><span>CarDetail 对白</span></div>
      <div class="stat"><b>{{ carTextStats.mappedEntries }}</b><span>车辆说明</span></div>
      <div class="stat"><b>{{ brands.length }}</b><span>品牌</span></div>
      <div class="stat"><b>{{ cast.length }}</b><span>咖啡屋角色</span></div>
      <button class="stat review-stat" type="button" @click="openUnmatched">
        <b>{{ unmatched.length }}</b>
        <span>上一轮歧义复核</span>
      </button>
    </section>
    <p class="summary-note">
      {{ stats.mappedDialogues }} 条 CarDetail 已定位到车辆 · 重建完整场景 {{ stats.conversationLines }} 行对白
      · {{ carTextStats.carsWithAnyText }} 辆车已有品牌购物/车辆收藏/二手车/收藏馆说明
    </p>
    <section class="castbar" aria-label="咖啡屋主要角色">
      <div v-for="person in cast" :key="person.id" class="cast-person">
        <img :src="avatarUrl(person.avatar)" :alt="person.en" loading="eager" />
        <span>
          <b>{{ speakerLabel(person) }}</b>
          <small>{{ speakerRole(person) }}</small>
        </span>
      </div>
      <button class="cast-all" @click="showCast = true">
        查看全部 {{ cast.length }} 位角色
      </button>
    </section>

    <div class="toolbar">
      <input v-model="searchInput" class="search" type="search" placeholder="搜索车型 / 对白 / 车辆说明…（按 / 聚焦）" />
      <div class="seg" role="group" aria-label="车辆资料筛选">
        <button
          v-for="filter in sourceFilters"
          :key="filter.id"
          :class="{ on: sourceFilter === filter.id }"
          :aria-pressed="sourceFilter === filter.id"
          @click="setSourceFilter(filter.id)"
        >
          {{ sourceFilterLabel(filter) }}
        </button>
      </div>
    </div>

    <nav class="brandbar">
      <button class="chip" :class="{ on: brandFilter === 'ALL' }" @click="brandFilter = 'ALL'">全部品牌</button>
      <button
        v-for="l in letters"
        :key="l"
        class="letter"
        @click="jumpTo(l)"
      >{{ l }}</button>
    </nav>
    <nav class="characterbar" aria-label="咖啡屋角色筛选">
      <button class="chip character" :class="{ on: speakerFilter === 'ALL' }" @click="speakerFilter = 'ALL'">
        全部角色
      </button>
      <button
        v-for="s in speakers"
        :key="s.id"
        class="chip character"
        :class="{ on: speakerFilter === s.id }"
        @click="speakerFilter = s.id"
      >
        <img :src="avatarUrl(s.avatar)" :alt="s.en" loading="lazy" />
        {{ speakerLabel(s) }} <small>{{ s.count }}</small>
      </button>
    </nav>

    <main class="content">
      <section v-for="[brand, list] in grouped" :key="brand" :id="`brand-${brand}`" class="brandgroup">
        <h2>
          <img v-if="brandLogos[brand]" class="blogo" :src="base + 'logos/' + brandLogos[brand]" :alt="brand" loading="lazy" />
          {{ brand }} <span>{{ list.length }}</span>
        </h2>
        <div class="grid">
          <article
            v-for="c in list"
            :key="c.id"
            class="card"
            :class="{ empty: !visibleConversation(c).length, active: navigable[activeIndex]?.id === c.id }"
            @click="openCar(c)"
          >
            <div class="car-media">
              <img
                v-if="c.image"
                class="thumb"
                :src="base + c.image"
                :alt="c.nameEn"
                loading="lazy"
                decoding="async"
                width="360"
                height="180"
              />
              <div v-else class="thumb-placeholder" aria-hidden="true">
                <span>GT7</span>
                <b>{{ c.brand }}</b>
              </div>
            </div>
            <div class="name">
              <template v-for="(code, index) in carNameLocales()" :key="code">
                <br v-if="index" />
                {{ isChineseLocale(code) ? c.nameZh : c.nameEn }}
              </template>
            </div>
            <div class="meta">
              <span class="year">{{ yearOf(c.nameEn) }}</span>
              <span class="badges">
                <span v-if="visibleConversation(c).length" class="badge has">对白 · {{ visibleConversation(c).length }}</span>
                <span v-else class="badge none">无对白</span>
                <span v-if="textCategoryCount(c)" class="badge text">资料 · {{ textCategoryCount(c) }}</span>
              </span>
            </div>
          </article>
        </div>
      </section>
      <p v-if="!grouped.length" class="nonefound">没有匹配的车辆。</p>

      <section v-if="unmatched.length" class="unmatched">
        <button class="toggle" @click="toggleUnmatched">
          上一轮 253 条歧义 CarDetail（已按精确消息表解析，供人工复核）· 当前 {{ filteredUnmatched.length }} / {{ unmatched.length }}{{ showUnmatched ? ' ▲' : ' ▼' }}
        </button>
      </section>
    </main>

    <nav
      v-if="navigable.length && !showUnmatched && !showCast"
      class="mobile-car-nav"
      aria-label="上一辆车和下一辆车"
    >
      <button
        type="button"
        class="mobile-car-step"
        :aria-label="lang === 'en' ? 'Previous car' : '上一辆车'"
        :disabled="!navigable.length"
        @click="stepMobile(-1)"
      >
        <strong>‹</strong>
        <span>{{ lang === 'en' ? 'Previous' : '上一辆' }}</span>
      </button>
      <button type="button" class="mobile-car-current" @click="toggleNavCar">
        <small>{{ navCar ? navCar.brand : (lang === 'en' ? 'Choose a car' : '选择车辆') }}</small>
        <b>{{ navCar ? displayName(navCar) : (lang === 'en' ? 'Browse all cars' : '浏览全部车型') }}</b>
        <span aria-live="polite">{{ navPosition }} / {{ navigable.length }}</span>
      </button>
      <button
        type="button"
        class="mobile-car-step"
        :aria-label="lang === 'en' ? 'Next car' : '下一辆车'"
        :disabled="!navigable.length"
        @click="stepMobile(1)"
      >
        <strong>›</strong>
        <span>{{ lang === 'en' ? 'Next' : '下一辆' }}</span>
      </button>
    </nav>

    <div v-if="showUnmatched" class="modal review-modal" @click.self="toggleUnmatched">
      <div class="dialog review-dialog">
        <header>
          <div>
            <h3>上一轮 253 条对白人工复核</h3>
            <p>按 RT2 精确标签重新归属 · {{ legacyReviewStats.resolved }} / {{ legacyReviewStats.total }} 条已定位车辆</p>
          </div>
          <button class="close" @click="toggleUnmatched">✕</button>
        </header>
        <div class="list review-list">
          <div class="review-intro">
            <strong>逐条核对车辆归属与场景</strong>
            <span>{{ legacyReviewStats.resolved }} / {{ legacyReviewStats.total }} 条已通过 RT2 精确消息标签定位到车辆。</span>
            <span>全部记录保留在下方，可逐条检查原候选车、精确归属和场景 ID。</span>
          </div>
          <div v-for="d in filteredUnmatched" :key="d.id" class="entry">
            <div class="speaker">
              <img v-if="d.avatar" class="avatar photo" :src="avatarUrl(d.avatar)" :alt="d.speaker" loading="lazy" />
              <span
                v-else
                class="avatar"
                :style="{ background: avatarColor(d.speaker) }"
              >{{ initials(d.speaker) }}</span>
              <span class="speaker-copy">
                <span class="who">{{ speakerName(d) }}</span>
                <span class="role">{{ speakerRole(d) }}</span>
              </span>
              <span class="id">{{ d.id }}</span>
            </div>
            <div class="entry-meta">
              <span>旧候选车：<b>{{ d.car || '未指定' }}</b></span>
              <span>精确归属：<b>{{ d.resolvedCarNameEn || '未解析' }}</b> · {{ d.resolvedCarNameZh || '' }}</span>
              <span>场景：<code>{{ d.sceneId || 'unknown' }}</code></span>
            </div>
            <div
              v-for="code in activeLocales()"
              :key="code"
              class="dialogue-text"
              :class="dialogueLocaleClass(code)"
              :dir="dialogueDirection(code)"
              :lang="code"
            >
              <span v-if="bilingual" class="translation-language">{{ languageName(code) }}</span>
              {{ dialogueText(d, code) }}
            </div>
          </div>
          <div class="review-actions">
            <a :href="base + 'data/unmatched_dialogues.csv'" download>下载全部 {{ unmatched.length }} 条 CSV</a>
            <a :href="base + 'data/unmatched_dialogues.md'" download>下载分组人工判定清单 MD</a>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showCast" class="modal cast-modal" @click.self="showCast = false">
      <div class="dialog cast-dialog">
        <header>
          <div>
            <h3>GT 咖啡屋角色图鉴</h3>
            <p>5 位核心/常驻角色 + 10 位真实汽车或时尚设计师，共 {{ cast.length }} 位</p>
          </div>
          <button class="close" @click="showCast = false">✕</button>
        </header>
        <div class="cast-grid">
          <button
            v-for="person in cast"
            :key="person.id"
            class="cast-card"
            @click="filterBySpeaker(person)"
          >
            <img :src="avatarUrl(person.avatar)" :alt="person.en" />
            <span class="cast-card-name">{{ speakerLabel(person) }}</span>
            <span class="cast-card-role">{{ speakerRole(person) }}</span>
            <span class="cast-card-count">
              {{ person.count ? `${person.count} 条车辆对白` : '核心剧情角色' }}
            </span>
          </button>
        </div>
      </div>
    </div>

    <div v-if="activeCar" class="modal" @click.self="closeCar">
      <div class="dialog">
        <header>
          <div class="headmain" :class="{ 'no-car-image': !activeCar.image }">
            <img v-if="activeCar.image" class="modalcar" :src="base + activeCar.image" :alt="activeCar.nameEn" />
            <div class="headcopy">
              <h3>{{ displayName(activeCar) }}</h3>
              <p>{{ isChineseLocale(primaryLang) ? `${activeCar.nameEn} · ${activeCar.brand}` : activeCar.brand }}</p>
              <div v-if="activeCar.specs" class="head-specs" aria-label="车辆核心规格">
                <span v-for="field in headerSpecFields" :key="field.key">
                  <small>{{ specLabel(field) }}</small>
                  <b>{{ compactSpecValue(field.key) || '—' }}</b>
                </span>
              </div>
              <p class="note">
                咖啡屋 {{ tabHasContent(activeCar, 'cafe') ? '有' : '无' }}对白
                · 品牌购物 {{ tabHasContent(activeCar, 'brandCentral') ? '有' : '无' }}
                · 车辆收藏 {{ tabHasContent(activeCar, 'vehicleCollection') ? '有' : '无' }}
                · 车库规格 {{ tabHasContent(activeCar, 'garageSpecs') ? '有' : '无' }}
                · 二手车 {{ tabHasContent(activeCar, 'usedCar') ? '有' : '无' }}
                · 收藏馆 {{ tabHasContent(activeCar, 'collection') ? '有' : '无' }}
              </p>
            </div>
          </div>
          <div class="detail-controls">
            <div class="language-picker detail-lang" role="group" aria-label="对白语言">
              <select
                :value="primaryLang"
                aria-label="第一语言"
                @change="setPrimaryLanguage($event.target.value)"
              >
                <option v-for="language in languageOptions" :key="language.id" :value="language.id">
                  {{ language.native }}
                </option>
              </select>
              <button
                type="button"
                class="bilingual-toggle"
                :class="{ on: bilingual }"
                :aria-pressed="bilingual"
                @click="toggleBilingual"
              >
                双语
              </button>
              <select
                v-if="bilingual"
                :value="secondaryLang"
                aria-label="第二语言"
                @change="setSecondaryLanguage($event.target.value)"
              >
                <option
                  v-for="language in languageOptions.filter((item) => item.id !== primaryLang)"
                  :key="language.id"
                  :value="language.id"
                >
                  {{ language.native }}
                </option>
              </select>
            </div>
            <div class="actions">
              <button class="copy" @click="copyLink">{{ copyState }}</button>
              <button class="close" @click="closeCar">✕</button>
            </div>
          </div>
        </header>
        <nav class="car-tabs" role="tablist" aria-label="车辆资料">
          <button
            v-for="tab in carTextTabs"
            :key="tab.id"
            :data-tab="tab.id"
            role="tab"
            :aria-selected="activeTab === tab.id"
            :class="{ on: activeTab === tab.id, muted: !tabHasContent(activeCar, tab.id) }"
            @click="setActiveTab(tab.id)"
          >
            {{ tabLabel(tab) }}
          </button>
        </nav>
        <div v-if="activeTab === 'cafe'" class="dialogues cafe-dialogues" role="tabpanel">
          <template v-if="activeCafeScenes.length">
            <section
              v-for="scene in activeCafeScenes"
              :key="scene.key"
              class="cafe-scene"
            >
              <header class="scene-head">
                <div>
                  <h4>{{ sceneTitle(scene) }}</h4>
                  <small>{{ scene.key }}</small>
                </div>
              </header>
              <article
                v-for="d in scene.visibleTurns"
                :key="`${scene.key}-${d.id}`"
                class="dialogue"
              >
                <div class="speaker">
                  <img v-if="d.avatar" class="avatar photo" :src="avatarUrl(d.avatar)" :alt="d.speaker" loading="lazy" />
                  <span v-else class="avatar" :style="{ background: avatarColor(d.speaker) }">
                    {{ initials(d.speaker) }}
                  </span>
                  <span class="speaker-copy">
                    <span class="who">{{ speakerName(d) }}</span>
                    <span class="role">{{ speakerRole(d) }}</span>
                  </span>
                  <span class="id">{{ d.id }}</span>
                </div>
                <div
                  v-for="code in activeLocales()"
                  :key="code"
                  class="dialogue-text"
                  :class="dialogueLocaleClass(code)"
                  :dir="dialogueDirection(code)"
                  :lang="code"
                >
                  <span v-if="bilingual" class="translation-language">{{ languageName(code) }}</span>
                  {{ dialogueText(d, code) }}
                </div>
              </article>
            </section>
          </template>
          <template v-else>
            <article v-for="d in activeCar.conversation" :key="d.id" class="dialogue">
              <div class="speaker">
                <img v-if="d.avatar" class="avatar photo" :src="avatarUrl(d.avatar)" :alt="d.speaker" loading="lazy" />
                <span v-else class="avatar" :style="{ background: avatarColor(d.speaker) }">
                  {{ initials(d.speaker) }}
                </span>
                <span class="speaker-copy">
                  <span class="who">{{ speakerName(d) }}</span>
                  <span class="role">{{ speakerRole(d) }}</span>
                </span>
                <span class="id">{{ d.id }}</span>
              </div>
              <div
                v-for="code in activeLocales()"
                :key="code"
                class="dialogue-text"
                :class="dialogueLocaleClass(code)"
                :dir="dialogueDirection(code)"
                :lang="code"
              >
                <span v-if="bilingual" class="translation-language">{{ languageName(code) }}</span>
                {{ dialogueText(d, code) }}
              </div>
            </article>
          </template>
          <div v-if="!activeCar.conversation.length" class="no-dialogue">
            <strong>没有找到这辆车的咖啡屋场景</strong>
            <span>已按 RT2 消息键和 DMS 场景表核对，未借用相邻车辆对白。</span>
          </div>
        </div>
        <div
          v-else-if="activeTab === 'garageSpecs'"
          class="garage-specs-panel"
          role="tabpanel"
        >
          <template v-if="activeCar.specs">
            <div class="specs-heading">
              <div>
                <span class="specs-eyebrow">{{ detailCopy('官方车库数据', 'Official garage data') }}</span>
                <h4>{{ detailCopy('车辆规格', 'Vehicle specifications') }}</h4>
                <p>
                  {{ detailCopy(
                    '动力、底盘尺寸与性能点来自 Gran Turismo 7 官方车表，中英文单位分别保留。',
                    'Power, dimensions and performance points come from the official Gran Turismo 7 car list; source units are preserved.',
                  ) }}
                </p>
              </div>
              <a
                v-if="activeSpecsSourceUrl()"
                class="spec-source-link"
                :href="activeSpecsSourceUrl()"
                target="_blank"
                rel="noopener noreferrer"
              >
                {{ detailCopy('查看官方车表', 'Official car entry') }} ↗
              </a>
            </div>
            <section class="specs-grid" aria-label="车辆详细规格">
              <article
                v-for="field in garageSpecFields"
                :key="field.key"
                class="spec-card"
                :class="`spec-${field.key}`"
              >
                <small>{{ specLabel(field) }}</small>
                <div v-if="showZhContent" class="spec-value">
                  <span v-if="bilingual" class="spec-locale">中文</span>
                  <b>{{ specValue('zh', field.key) || '—' }}</b>
                </div>
                <div v-if="showEnContent" class="spec-value en">
                  <span v-if="bilingual" class="spec-locale">EN</span>
                  <b>{{ specValue('en', field.key) || '—' }}</b>
                </div>
              </article>
            </section>
            <p v-if="activeCar.specs.match === 'variant-base-alias'" class="spec-note">
              {{ detailCopy(
                '该特别版未单独列入官方车表，此处显示其明确对应基础车型的机械规格。',
                'This special variant is not listed separately in the official car list; the mechanical specifications shown here belong to its explicitly mapped base car.',
              ) }}
            </p>
            <div class="specs-meta">
              <span>{{ activeCar.specs.officialId }}</span>
              <span>{{ activeCar.specs.source }}</span>
            </div>
          </template>
          <div v-else class="no-dialogue text-empty">
            <strong>{{ detailCopy('这辆车没有官方车库规格', 'No official garage specifications are available for this car') }}</strong>
            <span>
              {{ detailCopy(
                '已按官方车辆 ID、完整车型名称和明确变体别名核对；没有借用其他车辆的数据。',
                'The official car ID, full model name and explicit variant aliases were checked; no other vehicle’s data was substituted.',
              ) }}
            </span>
          </div>
        </div>
        <div v-else class="text-panel" role="tabpanel">
          <template v-if="activeText()">
            <div v-if="activeMerchant()" class="text-merchant">
              <img
                v-if="activeMerchant().avatar"
                :src="avatarUrl(activeMerchant().avatar)"
                :alt="activeMerchant().nameEn"
                loading="eager"
              />
              <span>
                <b>
                  {{ lang === 'zh' ? activeMerchant().nameZh
                    : lang === 'both'
                      ? `${activeMerchant().nameZh} · ${activeMerchant().nameEn}`
                      : activeMerchant().nameEn }}
                </b>
                <small>
                  {{ lang === 'zh' ? activeMerchant().roleZh
                    : lang === 'both'
                      ? `${activeMerchant().roleZh} · ${activeMerchant().roleEn}`
                      : activeMerchant().roleEn }}
                </small>
              </span>
            </div>
            <div class="text-source">
              <span>
                {{ lang === 'zh' ? activeText().titleZh
                  : lang === 'both'
                    ? `${activeText().titleZh} · ${activeText().titleEn}`
                    : activeText().titleEn }}
              </span>
              <span v-if="activeText().carKey">{{ activeText().carKey }}</span>
              <span v-if="activeTab !== 'vehicleCollection'">{{ activeText().textId }}</span>
              <template v-else>
                <span v-if="activeText().textIds?.catch">Catch {{ activeText().textIds.catch }}</span>
                <span v-if="activeText().textIds?.long">Long {{ activeText().textIds.long }}</span>
                <span v-if="activeText().officialId">
                  {{ detailCopy('官方车表', 'Official car list') }} {{ activeText().officialId }}
                </span>
              </template>
            </div>
            <div v-if="activeOffer()" class="dealer-offer">
              <strong>Cr. {{ formatPrice(activeOffer().price) }}</strong>
              <span v-if="activeOffer().discountPrice > 0">优惠 Cr. {{ formatPrice(activeOffer().discountPrice) }}</span>
              <span>稀有度 {{ activeOffer().rarity }}</span>
              <span v-if="activeOffer().isHot">{{ lang === 'zh' ? '热门' : 'Hot' }}</span>
              <span v-if="activeOffer().isPick">{{ lang === 'zh' ? '精选' : 'Pick' }}</span>
              <span v-if="activeOffer().isNew">{{ lang === 'zh' ? '新车' : 'New' }}</span>
            </div>
            <p v-if="activeTab === 'brandCentral'" class="text-source-note">
              <template v-if="lang === 'zh'">
                这是品牌购物页的车型介绍；车辆收藏的独立短简介与长正文位于“车辆收藏”页签。
              </template>
              <template v-else-if="lang === 'en'">
                This is the Brand Central description. The separate short and full Car Collection entries are under the “Car Collection” tab.
              </template>
              <template v-else>
                这是品牌购物页的车型介绍；车辆收藏的独立短简介与长正文位于“车辆收藏”页签。
                This is the Brand Central description. The separate short and full Car Collection entries are under the “Car Collection” tab.
              </template>
            </p>
            <template v-if="activeTab === 'vehicleCollection'">
              <section class="collection-part">
                <h4>{{ detailCopy('一句话简介', 'Catch copy') }}</h4>
                <div v-if="showZhContent" class="text-copy zh-copy">
                  <p>{{ activeText().catch?.zh }}</p>
                </div>
                <div v-if="showEnContent" class="text-copy en-copy">
                  <p>{{ activeText().catch?.en }}</p>
                </div>
              </section>
              <section class="collection-part">
                <h4>{{ detailCopy('车辆收藏正文', 'Full description') }}</h4>
                <div v-if="showZhContent" class="text-copy zh-copy">
                  <p>{{ activeText().long?.zh }}</p>
                </div>
                <div v-if="showEnContent" class="text-copy en-copy">
                  <p>{{ activeText().long?.en }}</p>
                </div>
              </section>
            </template>
            <template v-else>
              <div v-if="showZhContent" class="text-copy zh-copy">
                <h4>{{ activeText().titleZh }}</h4>
                <p>{{ activeText().zh }}</p>
              </div>
              <div v-if="showEnContent" class="text-copy en-copy">
                <h4>{{ activeText().titleEn }}</h4>
                <p>{{ activeText().en }}</p>
              </div>
            </template>
            <section v-if="activeText().variants?.length" class="text-variants">
              <h4>{{ detailCopy('补充介绍', 'Additional description') }}</h4>
              <article
                v-for="variant in activeText().variants"
                :key="variant.textId"
                class="text-variant"
              >
                <div class="text-source">
                  <span>{{ variant.textId }}</span>
                  <span>{{ variant.carKey || variant.sourceName }}</span>
                </div>
                <div v-if="showZhContent" class="text-copy zh-copy">
                  <p>{{ variant.zh }}</p>
                </div>
                <div v-if="showEnContent" class="text-copy en-copy">
                  <p>{{ variant.en }}</p>
                </div>
              </article>
            </section>
          </template>
          <div v-else class="no-dialogue text-empty">
            <strong>
              {{ activeTab === 'vehicleCollection'
                ? detailCopy('这辆车没有车辆收藏说明', 'No Car Collection entry is available for this car')
                : detailCopy('这辆车没有该商店的官方说明文本', 'No official description is available for this source') }}
            </strong>
            <span>
              {{ activeTab === 'vehicleCollection'
                ? detailCopy(
                    '已检查 DescriptionCatch / DescriptionLong，没有借用其他车型的收藏正文。',
                    'DescriptionCatch and DescriptionLong were checked; no other car’s entry was substituted.',
                  )
                : detailCopy(
                    '已检查 PS5 文本表，没有强行关联其他车辆的内容。',
                    'The PS5 text tables were checked; no text from another car was substituted.',
                  ) }}
            </span>
          </div>
        </div>
        <div class="keyhint">← → / ↑ ↓ 切换车辆 · 1–6 或 Tab 切换资料页签 · Esc 关闭</div>
      </div>
    </div>

    <footer class="foot">
      <a :href="base + 'data/cafe_dialogues.csv'" download>下载全部对白 CSV</a>
      · <a :href="base + 'data/cafe_scene_dialogues.csv'" download>下载完整场景 CSV</a>
      · <a :href="base + 'data/cafe_variants.csv'" download>下载全部初次/重复/获胜分支 CSV</a>
      · <a :href="base + 'data/unmatched_dialogues.csv'" download>下载 253 条歧义复核 CSV</a>
      · <a :href="base + 'data/unmatched_dialogues.md'" download>人工判定清单 MD</a>
      · <a :href="base + 'data/car_texts.csv'" download>品牌购物/二手车/收藏馆说明 CSV</a>
      · <a :href="base + 'data/vehicle_collection_texts.csv'" download>车辆收藏中英文 CSV</a>
      · <a :href="base + 'data/garage_specs.csv'" download>车库规格中英文 CSV</a>
      · <a :href="base + 'data/dealer_offers.csv'" download>当前车商报价 CSV</a>
      · <a :href="base + 'data/car_text_orphans.csv'" download>未归属说明文本 CSV</a>
      · 数据来源：Gran Turismo 7 (PS5, PPSA01317) 本地化文本 · 仅供研究使用
      <div class="credits">19 张角色头像均由本机 PS5 IDP0/TXS5 面部图集解码；站点未使用共享占位头像。</div>
      <div class="kbdhint">键盘：↑/↓/←/→ 浏览 · Enter 打开 · Esc 关闭 · / 搜索</div>
    </footer>
  </div>
</template>
