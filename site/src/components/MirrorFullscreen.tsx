import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from 'react'

export interface MirrorSlideCar {
  id: string
  brand: string
  model: string
  logo: string
  images: string[]
  hero: string
  description: string
  carClass: string
  performancePoint: string
  specs: Array<{ label: string; value: string }>
}

export interface SlideshowPosition {
  carId: string
  imageIndex: number
}

const SPEEDS = Array.from({ length: 10 }, (_, index) => index + 1)
const TEXT_SCALE_KEY = 'gt7.carlist.fullscreenInfoTextScale'
const OFFSET_KEY = 'gt7.carlist.fullscreenInfoOffset'

function storedScale() {
  const stored = localStorage.getItem(TEXT_SCALE_KEY)
  if (stored === null || stored === '') return 1
  const value = Number(stored)
  return Number.isFinite(value) ? Math.min(1.6, Math.max(0.8, value)) : 1
}

function storedOffset() {
  try {
    const value = JSON.parse(localStorage.getItem(OFFSET_KEY) || 'null')
    return {
      x: Number.isFinite(value?.x) ? Number(value.x) : 0,
      y: Number.isFinite(value?.y) ? Number(value.y) : 0,
    }
  } catch {
    return { x: 0, y: 0 }
  }
}

export function MirrorFullscreen({
  cars,
  initialCarId,
  initialImageIndex,
  speed,
  chinese,
  onSpeedChange,
  onClose,
}: {
  cars: MirrorSlideCar[]
  initialCarId: string
  initialImageIndex: number
  speed: number
  chinese: boolean
  onSpeedChange: (speed: number) => void
  onClose: (position: SlideshowPosition) => void
}) {
  const slides = useMemo(
    () => cars.flatMap((car) => car.images.map((src, imageIndex) => ({ car, src, imageIndex }))),
    [cars],
  )
  const initialIndex = useMemo(() => {
    const carIndex = cars.findIndex((car) => car.id === initialCarId)
    if (carIndex < 0) return 0
    return (
      cars.slice(0, carIndex).reduce((total, car) => total + car.images.length, 0) +
      Math.min(initialImageIndex, Math.max(0, cars[carIndex].images.length - 1))
    )
  }, [cars, initialCarId, initialImageIndex])
  const [index, setIndex] = useState(initialIndex)
  const [playing, setPlaying] = useState(true)
  const [showInfo, setShowInfo] = useState(true)
  const [textScale, setTextScale] = useState(storedScale)
  const [offset, setOffset] = useState(storedOffset)
  const infoRef = useRef<HTMLDivElement>(null)
  const closingRef = useRef(false)
  const dragRef = useRef<{
    pointerId: number
    startX: number
    startY: number
    offset: { x: number; y: number }
    rect: DOMRect
    lastOffset: { x: number; y: number }
  } | null>(null)
  const slide = slides[index]
  const slideRef = useRef<SlideshowPosition>({ carId: initialCarId, imageIndex: initialImageIndex })
  slideRef.current = {
    carId: slide?.car.id || initialCarId,
    imageIndex: slide?.imageIndex ?? 0,
  }
  const close = useCallback(() => {
    if (closingRef.current) return
    closingRef.current = true
    onClose(slideRef.current)
  }, [onClose])

  useEffect(() => {
    document.documentElement.requestFullscreen?.().catch(() => {})
    const onFullscreen = () => {
      if (!document.fullscreenElement) close()
    }
    document.addEventListener('fullscreenchange', onFullscreen)
    return () => document.removeEventListener('fullscreenchange', onFullscreen)
  }, [close])

  useEffect(() => {
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = previousOverflow
    }
  }, [])

  useEffect(() => {
    if (!playing || slides.length < 2) return
    const timer = window.setTimeout(
      () => setIndex((current) => (current + 1) % slides.length),
      Math.round(5000 / speed),
    )
    return () => window.clearTimeout(timer)
  }, [index, playing, slides.length, speed])

  useEffect(() => {
    if (slides.length < 2) return
    for (const delta of [-1, 1]) {
      const adjacent = slides[(index + delta + slides.length) % slides.length]
      if (!adjacent) continue
      const image = new Image()
      image.src = adjacent.src
    }
  }, [index, slides])

  const moveCar = (delta: number) => {
    if (!slide || cars.length < 2) return
    const carIndex = cars.findIndex((car) => car.id === slide.car.id)
    if (carIndex < 0) return
    const nextCarIndex = (carIndex + delta + cars.length) % cars.length
    const nextCar = cars[nextCarIndex]
    if (!nextCar.images.length) return
    const imageIndex = Math.min(slide.imageIndex, nextCar.images.length - 1)
    const nextIndex =
      cars.slice(0, nextCarIndex).reduce((total, car) => total + car.images.length, 0) +
      imageIndex
    setIndex(nextIndex)
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target
      if (
        target instanceof HTMLElement &&
        (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
      ) {
        return
      }
      const digit = /^(?:Digit|Numpad)([0-9])$/.exec(event.code)
      if (digit) {
        event.preventDefault()
        onSpeedChange(Number(digit[1]) || 10)
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault()
        setIndex((current) => (current - 1 + slides.length) % slides.length)
      } else if (event.key === 'ArrowRight') {
        event.preventDefault()
        setIndex((current) => (current + 1) % slides.length)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        moveCar(-1)
      } else if (event.key === 'ArrowDown') {
        event.preventDefault()
        moveCar(1)
      } else if (event.key === ' ') {
        event.preventDefault()
        setPlaying((current) => !current)
      } else if (event.key === 'Escape') {
        event.preventDefault()
        close()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [cars, close, onSpeedChange, slide, slides.length])

  const persistScale = (value: number) => {
    const next = Math.round(Math.min(1.6, Math.max(0.8, value)) * 10) / 10
    setTextScale(next)
    localStorage.setItem(TEXT_SCALE_KEY, String(next))
  }
  const persistOffset = (value: { x: number; y: number }) => {
    setOffset(value)
    localStorage.setItem(OFFSET_KEY, JSON.stringify(value))
  }
  const onDragStart = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (!infoRef.current) return
    event.currentTarget.setPointerCapture(event.pointerId)
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      offset,
      rect: infoRef.current.getBoundingClientRect(),
      lastOffset: offset,
    }
  }
  const onDragMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const drag = dragRef.current
    if (!drag || drag.pointerId !== event.pointerId) return
    const minimumVisible = 96
    const desiredLeft = drag.rect.left + event.clientX - drag.startX
    const desiredTop = drag.rect.top + event.clientY - drag.startY
    const left = Math.min(
      window.innerWidth - minimumVisible,
      Math.max(minimumVisible - drag.rect.width, desiredLeft),
    )
    const top = Math.min(window.innerHeight - 220, Math.max(48, desiredTop))
    const next = {
      x: Math.round(drag.offset.x + left - drag.rect.left),
      y: Math.round(drag.offset.y + top - drag.rect.top),
    }
    drag.lastOffset = next
    setOffset(next)
  }
  const onDragEnd = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const drag = dragRef.current
    if (!drag || drag.pointerId !== event.pointerId) return
    dragRef.current = null
    persistOffset(drag.lastOffset)
  }

  if (!slide) return null
  const text = (zh: string, en: string) => (chinese ? zh : en)
  const sizes = [0.8, 0.9, 1, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6]

  return (
    <section className="mirror-slideshow" aria-label={text('GT7 全屏车图', 'GT7 fullscreen gallery')}>
      <img
        className="mirror-slideshow-image"
        src={slide.src}
        alt={`${slide.car.brand} ${slide.car.model}`}
        onDoubleClick={close}
      />
      <aside className="mirror-slideshow-info">
        <div ref={infoRef} style={{ transform: `translate3d(${offset.x}px, ${offset.y}px, 0)` }}>
          <header>
            <img src={slide.car.logo} alt="" />
            <div>
              <small>{slide.car.brand}</small>
              <h2>{slide.car.model}</h2>
            </div>
            <button type="button" onClick={() => setShowInfo((value) => !value)}>
              {showInfo ? text('隐藏资料', 'Hide details') : text('显示资料', 'Show details')}
            </button>
          </header>
          <div className="mirror-info-tools">
            <button
              type="button"
              onPointerDown={onDragStart}
              onPointerMove={onDragMove}
              onPointerUp={onDragEnd}
              onPointerCancel={onDragEnd}
            >
              {text('拖动资料', 'Drag details')}
            </button>
            <button type="button" onClick={() => persistScale(textScale - 0.1)}>A-</button>
            <span>{Math.round(textScale * 100)}%</span>
            <button type="button" onClick={() => persistScale(textScale + 0.1)}>A+</button>
            <button type="button" onClick={() => persistOffset({ x: 0, y: 0 })}>
              {text('复位', 'Reset')}
            </button>
          </div>
          {showInfo && (
            <div className="mirror-info-copy" style={{ fontSize: `${16 * textScale}px` }}>
              {slide.car.hero && <h3>{slide.car.hero}</h3>}
              {slide.car.description && <p>{slide.car.description}</p>}
              <div className="mirror-info-meta">
                {slide.car.carClass && <b>{slide.car.carClass}</b>}
                <strong>{slide.car.performancePoint}</strong>
              </div>
              <dl>
                {slide.car.specs.map((spec) => (
                  <div key={spec.label}>
                    <dt>{spec.label}</dt>
                    <dd>{spec.value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          )}
        </div>
      </aside>
      <button type="button" className="mirror-slideshow-close" onClick={close} aria-label="关闭">
        ×
      </button>
      <div className="mirror-slideshow-controls">
        <button type="button" onClick={() => setIndex((index - 1 + slides.length) % slides.length)}>←</button>
        <button type="button" onClick={() => setPlaying((value) => !value)}>
          {playing ? text('暂停', 'Pause') : text('播放', 'Play')}
        </button>
        <button type="button" onClick={() => setIndex((index + 1) % slides.length)}>→</button>
        <span className="mirror-key-hint">{text('← → 图片 · ↑ ↓ 车辆', '← → image · ↑ ↓ car')}</span>
        <span className="mirror-image-counter">{text('图片', 'Image')} {index + 1} / {slides.length}</span>
        <span className="mirror-speed-label">{text('速度', 'Speed')} {speed}×</span>
        <div>
          {SPEEDS.map((value) => (
            <button
              key={value}
              type="button"
              className={value === speed ? 'on' : ''}
              onClick={() => onSpeedChange(value)}
              aria-label={`${text('速度', 'Speed')} ${value}×`}
              aria-pressed={value === speed}
            >
              {value}×
            </button>
          ))}
        </div>
      </div>
      <span className="mirror-info-scale-hint">{sizes.map((value) => Math.round(value * 100)).join(' / ')}</span>
    </section>
  )
}
