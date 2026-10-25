package com.gakaki.gt7offline

import androidx.compose.ui.graphics.ImageBitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json

interface OfflineResourceReader {
    suspend fun readBytes(path: String): ByteArray
    suspend fun exists(path: String): Boolean
}

expect fun createOfflineResourceReader(): OfflineResourceReader

expect suspend fun decodeOfflineImage(bytes: ByteArray): ImageBitmap?

class OfflineRepository(
    private val resources: OfflineResourceReader = createOfflineResourceReader(),
) {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        explicitNulls = false
    }
    private val mirrorCache = mutableMapOf<String, MirrorCatalog>()
    private val dialogueCache = mutableMapOf<String, Map<String, String>>()
    private val collectionCache = mutableMapOf<String, VehicleCollectionLanguage>()
    private val imageCache = ImageMemoryCache(96)
    private val mirrorMutex = Mutex()
    private val dialogueMutex = Mutex()
    private val collectionMutex = Mutex()

    suspend fun loadCars(): List<Car> = withContext(Dispatchers.Default) {
        json.decodeFromString<CarsPayload>(readText("data/cars.json")).cars
    }

    suspend fun loadMenuBooks(): List<MenuBook> = withContext(Dispatchers.Default) {
        json.decodeFromString<MenuBooksPayload>(readText("data/menu_books.json")).records
            .sortedWith(compareBy<MenuBook> { it.category }.thenBy { it.number })
    }

    suspend fun loadMirror(locale: String): MirrorCatalog = mirrorMutex.withLock {
        mirrorCache[locale]?.let { return it }
        val mirrorCode = if (locale == "zh") "cn" else locale
        val value = withContext(Dispatchers.Default) {
            json.decodeFromString<MirrorCatalog>(readText("data/mirror/cars.$mirrorCode.json"))
        }
        mirrorCache[locale] = value
        value
    }

    suspend fun loadDialogueTranslations(locale: String): Map<String, String> =
        dialogueMutex.withLock {
            dialogueCache[locale]?.let { return it }
            val mirrorCode = if (locale == "zh") "cn" else locale
            val value = withContext(Dispatchers.Default) {
                runCatching {
                    json.decodeFromString<Map<String, String>>(
                        readText("data/dialogue_translations/$mirrorCode.json"),
                    )
                }.getOrDefault(emptyMap())
            }
            dialogueCache[locale] = value
            value
        }

    suspend fun loadCollectionTranslations(locale: String): VehicleCollectionLanguage =
        collectionMutex.withLock {
            collectionCache[locale]?.let { return it }
            val mirrorCode = if (locale == "zh") "cn" else locale
            val value = withContext(Dispatchers.Default) {
                runCatching {
                    json.decodeFromString<VehicleCollectionLanguage>(
                        readText("data/vehicle_collection_i18n/$mirrorCode.json"),
                    )
                }.getOrDefault(VehicleCollectionLanguage())
            }
            collectionCache[locale] = value
            value
        }

    suspend fun loadImage(path: String?): ImageBitmap? {
        if (path.isNullOrBlank()) return null
        val clean = path.removePrefix("/")
        imageCache.get(clean)?.let { return it }
        val bytes = runCatching { resources.readBytes(clean) }.getOrNull()
            ?: androidBrandLogoAlias(clean)?.let { alias ->
                runCatching { resources.readBytes(alias) }.getOrNull()
            }
            ?: return null
        val bitmap = decodeOfflineImage(bytes) ?: return null
        imageCache.put(clean, bitmap)
        return bitmap
    }

    private suspend fun readText(path: String): String =
        resources.readBytes(path).toString(Charsets.UTF_8).removePrefix("\uFEFF")
}

private fun androidBrandLogoAlias(path: String): String? {
    if (!path.startsWith("logos/")) return null
    val name = path.substringAfter('/')
    if (name.all { it.code < 128 }) return null
    val extension = name.substringAfterLast('.', "")
    val stem = name.substringBeforeLast('.').map { character ->
        "u" + character.code.toString(16).padStart(4, '0')
    }.joinToString("")
    return if (extension.isBlank()) "logos/logo-$stem" else "logos/logo-$stem.$extension"
}

private class ImageMemoryCache(private val limit: Int) {
    private val values = object : LinkedHashMap<String, ImageBitmap>(limit, 0.75f, true) {
        override fun removeEldestEntry(
            eldest: MutableMap.MutableEntry<String, ImageBitmap>?,
        ): Boolean = size > limit
    }

    @Synchronized
    fun get(key: String): ImageBitmap? = values[key]

    @Synchronized
    fun put(key: String, value: ImageBitmap) {
        values[key] = value
    }
}
