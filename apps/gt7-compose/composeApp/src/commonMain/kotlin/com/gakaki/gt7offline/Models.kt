package com.gakaki.gt7offline

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

@Serializable
data class CarsPayload(
    val cars: List<Car> = emptyList(),
)

@Serializable
data class Car(
    val id: String,
    val brand: String = "",
    val brandLogo: String? = null,
    val image: String? = null,
    val nameEn: String = "",
    val nameZh: String = "",
    val officialId: String? = null,
    val conversation: List<DialogueTurn> = emptyList(),
    val dialogues: List<DialogueTurn> = emptyList(),
    val texts: Map<String, CarText> = emptyMap(),
    val specs: JsonObject? = null,
    val officialDetails: OfficialDetails? = null,
    val garageDetails: GarageDetails? = null,
) {
    fun displayName(locale: String): String =
        if (locale == "cn" || locale == "tw" || locale == "zh") {
            nameZh.ifBlank { nameEn }
        } else {
            nameEn.ifBlank { nameZh }
        }

    fun spec(locale: String, key: String): String? {
        val requested = if (locale == "cn" || locale == "tw" || locale == "zh") "zh" else "en"
        return specs
            ?.get(requested)
            ?.jsonObject
            ?.get(key)
            ?.jsonPrimitive
            ?.contentOrNull
    }
}

@Serializable
data class DialogueTurn(
    val id: String = "",
    val speaker: String = "",
    val speakerZh: String? = null,
    val roleEn: String? = null,
    val roleZh: String? = null,
    val en: String? = null,
    val zh: String? = null,
    val avatar: String? = null,
    val sceneId: String? = null,
    val sceneKey: String? = null,
    val sceneOrder: Int? = null,
    val isCarDetail: Boolean? = null,
)

@Serializable
data class CarText(
    val titleEn: String? = null,
    val titleZh: String? = null,
    val en: String? = null,
    val zh: String? = null,
    @SerialName("catch")
    val catchCopy: TextPart? = null,
    val long: TextPart? = null,
    val sourceBreakdown: TextPart? = null,
    val textId: String? = null,
    val textIds: TextIds? = null,
    val officialId: String? = null,
)

@Serializable
data class TextPart(
    val en: String? = null,
    val zh: String? = null,
    val textId: String? = null,
)

@Serializable
data class TextIds(
    val catch: String? = null,
    val long: String? = null,
)

@Serializable
data class OfficialDetails(
    val officialId: String? = null,
    val heroEn: String? = null,
    val heroZh: String? = null,
    val descriptionEn: String? = null,
    val descriptionZh: String? = null,
    val images: List<OfficialImage> = emptyList(),
)

@Serializable
data class OfficialImage(
    val name: String? = null,
    val jxl: String,
    val width: Int? = null,
    val height: Int? = null,
)

@Serializable
data class GarageDetails(
    val country: GarageCountry? = null,
    val vehicleTypes: List<GarageType> = emptyList(),
)

@Serializable
data class GarageCountry(
    val code: String? = null,
    val nameEn: String? = null,
    val nameZh: String? = null,
)

@Serializable
data class GarageType(
    val id: String? = null,
    val tagEn: String? = null,
    val tagZh: String? = null,
    val nameEn: String? = null,
    val nameZh: String? = null,
)

@Serializable
data class MirrorCatalog(
    val locale: String = "",
    @SerialName("car_count")
    val carCount: Int = 0,
    @SerialName("image_count")
    val imageCount: Int = 0,
    val cars: List<MirrorCar> = emptyList(),
) {
    private val byOfficialId: Map<String, MirrorCar> by lazy {
        cars.associateBy { it.id }
    }

    fun forCar(car: Car): MirrorCar? {
        val id = car.officialId ?: car.officialDetails?.officialId
        if (id != null) byOfficialId[id]?.let { return it }
        val name = car.nameEn.lowercase()
        return cars.firstOrNull { it.nameLong.lowercase() == name || it.model.lowercase() == name }
    }
}

@Serializable
data class MirrorCar(
    val id: String,
    val brand: String = "",
    val model: String = "",
    @SerialName("name_long")
    val nameLong: String = "",
    val logo: String = "",
    val thumbnail: String = "",
    val country: String = "",
    @SerialName("drive_train")
    val driveTrain: String = "",
    val power: String = "",
    val description: String = "",
    val hero: String = "",
    @SerialName("car_class")
    val carClass: String = "",
    @SerialName("performance_point")
    val performancePoint: String = "",
    val specs: List<MirrorSpec> = emptyList(),
    val images: List<String> = emptyList(),
)

@Serializable
data class MirrorSpec(
    val label: String = "",
    val value: String = "",
)

@Serializable
data class VehicleCollectionLanguage(
    val code: String = "",
    val DescriptionCatch: Map<String, String> = emptyMap(),
    val DescriptionLong: Map<String, String> = emptyMap(),
)

@Serializable
data class MenuBooksPayload(
    val records: List<MenuBook> = emptyList(),
)

@Serializable
data class MenuBook(
    val id: String,
    val number: Int = 0,
    val displayNumber: String = "",
    val category: String = "",
    val categoryEn: String = "",
    val categoryZh: String = "",
    val collectionNameEn: String = "",
    val collectionNameZh: String = "",
    val titleEn: String = "",
    val titleZh: String = "",
    val descriptionEn: String = "",
    val descriptionZh: String = "",
    val requirementsEn: String = "",
    val requirementsZh: String = "",
    val rewardEn: String = "",
    val rewardZh: String = "",
    val storyEn: String = "",
    val storyZh: String = "",
    val cover: String = "",
    val manufacturers: List<String> = emptyList(),
    val cars: List<MenuCar> = emptyList(),
) {
    fun title(locale: String): String =
        if (locale == "cn" || locale == "tw" || locale == "zh") titleZh.ifBlank { titleEn } else titleEn
}

@Serializable
data class MenuCar(
    val id: String = "",
    val brand: String = "",
    val nameEn: String = "",
    val nameZh: String = "",
    val image: String = "",
)

data class LanguageOption(
    val code: String,
    val label: String,
)

val offlineLanguages = listOf(
    LanguageOption("cn", "简体中文"),
    LanguageOption("tw", "繁體中文"),
    LanguageOption("us", "English (US)"),
    LanguageOption("gb", "English (UK)"),
    LanguageOption("jp", "日本語"),
    LanguageOption("kr", "한국어"),
    LanguageOption("de", "Deutsch"),
    LanguageOption("fr", "Français"),
    LanguageOption("it", "Italiano"),
    LanguageOption("es", "Español"),
    LanguageOption("ms", "Español latinoamericano"),
    LanguageOption("pt", "Português"),
    LanguageOption("bp", "Português brasileiro"),
    LanguageOption("nl", "Nederlands"),
    LanguageOption("pl", "Polski"),
    LanguageOption("ru", "Русский"),
    LanguageOption("cz", "Čeština"),
    LanguageOption("tr", "Türkçe"),
    LanguageOption("el", "Ελληνικά"),
    LanguageOption("ar", "العربية"),
    LanguageOption("th", "ไทย"),
)

fun isChinese(locale: String) = locale == "cn" || locale == "tw" || locale == "zh"

fun localizedOfficialHero(car: Car, locale: String): String =
    if (isChinese(locale)) {
        car.officialDetails?.heroZh.orEmpty().ifBlank { car.officialDetails?.heroEn.orEmpty() }
    } else {
        car.officialDetails?.heroEn.orEmpty()
    }

fun localizedOfficialDescription(car: Car, locale: String): String =
    if (isChinese(locale)) {
        car.officialDetails?.descriptionZh.orEmpty().ifBlank {
            car.officialDetails?.descriptionEn.orEmpty()
        }
    } else {
        car.officialDetails?.descriptionEn.orEmpty()
    }

fun localizedText(text: CarText?, locale: String): String? =
    if (isChinese(locale)) text?.zh?.ifBlank { null } ?: text?.en else text?.en?.ifBlank { null } ?: text?.zh

fun localizedText(text: TextPart?, locale: String): String? =
    if (isChinese(locale)) text?.zh?.ifBlank { null } ?: text?.en else text?.en?.ifBlank { null } ?: text?.zh
