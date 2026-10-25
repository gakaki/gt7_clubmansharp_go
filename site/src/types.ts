export interface DialogueTurn {
  id: string
  speaker: string
  speakerZh?: string
  roleEn?: string
  roleZh?: string
  en?: string
  zh?: string
  avatar?: string
  sceneId?: string
  sceneKey?: string
  [key: string]: unknown
}

export interface CarText {
  titleEn?: string
  titleZh?: string
  en?: string
  zh?: string
  catch?: {
    en?: string
    zh?: string
    textId?: string
    sourceEn?: string
    sourceZh?: string
  }
  long?: {
    en?: string
    zh?: string
    textId?: string
    sourceEn?: string
    sourceZh?: string
  }
  sourceBreakdown?: { en?: string; zh?: string }
  textId?: string
  textIds?: { catch?: string; long?: string }
  officialId?: string
  carKey?: string
  variants?: Array<Record<string, unknown>>
  [key: string]: unknown
}

export interface Car {
  id: string
  brand: string
  nameEn: string
  nameZh: string
  brandLogo?: string
  image?: string
  officialId?: string
  conversation?: DialogueTurn[]
  dialogues?: DialogueTurn[]
  texts?: Record<string, CarText | undefined>
  dealer?: Record<string, Record<string, unknown> | undefined>
  specs?: {
    en?: Record<string, string | number>
    zh?: Record<string, string | number>
    sourceUrlEn?: string
    sourceUrlZh?: string
    officialId?: string
    source?: string
    match?: string
    [key: string]: unknown
  }
  officialDetails?: {
    officialId?: string
    heroEn?: string
    heroZh?: string
    descriptionEn?: string
    descriptionZh?: string
    sourceUrlEn?: string
    sourceUrlZh?: string
    images?: Array<{
      name?: string
      jxl: string
      width?: number
      height?: number
    }>
    [key: string]: unknown
  }
  garageDetails?: {
    country?: { code?: string; nameEn?: string; nameZh?: string }
    vehicleTypes?: Array<{ id?: string; tagEn?: string; tagZh?: string }>
    [key: string]: unknown
  }
  [key: string]: unknown
}

export interface CarsPayload {
  cars: Car[]
  stats?: Record<string, number>
  carTextStats?: Record<string, number>
  unmatched?: DialogueTurn[]
  speakers?: Array<Record<string, any>>
  cast?: Array<Record<string, any>>
  dialogueTranslationDir?: string
  [key: string]: unknown
}

export interface VehicleCollectionLanguageTable {
  code: string
  sourceFile: string
  DescriptionCatch: Record<string, string>
  DescriptionLong: Record<string, string>
}

export interface MirrorCar {
  id: string
  brand: string
  model: string
  name_long: string
  logo: string
  thumbnail: string
  country: string
  drive_train: string
  power: string
  power_value: number | null
  weight_value: number | null
  description: string
  hero: string
  car_class: string
  performance_point: string
  specs: Array<{ label: string; value: string }>
  images: string[]
}

export interface MirrorCatalog {
  locale: string
  car_count: number
  image_count: number
  cars: MirrorCar[]
}
