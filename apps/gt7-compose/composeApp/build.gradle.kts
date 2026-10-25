import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("org.jetbrains.kotlin.multiplatform")
    id("com.android.application")
    id("org.jetbrains.compose")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

val siteResources = rootProject.layout.projectDirectory.dir("../../site/public")
val androidSupplementalAssets = layout.buildDirectory.dir("android-supplemental-assets")

fun androidLogoAlias(name: String): String? {
    if (name.all { it.code < 128 }) return null
    val extension = name.substringAfterLast('.', "")
    val stem = name.substringBeforeLast('.').map { character ->
        "u" + character.code.toString(16).padStart(4, '0')
    }.joinToString("")
    return if (extension.isBlank()) "logo-$stem" else "logo-$stem.$extension"
}

val prepareAndroidSupplementalAssets by tasks.registering {
    outputs.dir(androidSupplementalAssets)
    doLast {
        val target = androidSupplementalAssets.get().asFile
        project.delete(target)
        target.mkdirs()

        // AAPT ignores underscore-prefixed directories, so preserve that content under a safe name.
        project.copy {
            from(siteResources.dir("_offline"))
            into(target.resolve("offline_web"))
        }

        val logoDir = siteResources.dir("logos").asFile
        logoDir.listFiles()?.filter { it.isFile }?.forEach { logo ->
            androidLogoAlias(logo.name)?.let { alias ->
                project.copy {
                    from(logo)
                    into(target.resolve("logos"))
                    rename { alias }
                }
            }
        }
    }
}

kotlin {
    androidTarget {
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }
    }
    jvm("desktop") {
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_17)
        }
    }

    sourceSets {
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.materialIconsExtended)
            implementation(compose.ui)
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
            implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.8.1")
        }
        androidMain.dependencies {
            implementation("androidx.activity:activity-compose:1.10.1")
        }
        val desktopMain by getting {
            dependencies {
                implementation(compose.desktop.currentOs)
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-swing:1.10.2")
            }
        }
    }
}

android {
    namespace = "com.gakaki.gt7offline"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.gakaki.gt7offline"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
    }

    sourceSets["main"].assets.srcDir(siteResources)
    sourceSets["main"].assets.srcDir(androidSupplementalAssets)

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    packaging {
        resources.excludes += setOf(
            "META-INF/AL2.0",
            "META-INF/LGPL2.1",
            "META-INF/LICENSE*",
            "META-INF/NOTICE*",
        )
    }
}

tasks.matching { it.name.contains("Assets", ignoreCase = true) }.configureEach {
    if (name.contains("merge", ignoreCase = true) || name.contains("package", ignoreCase = true)) {
        dependsOn(prepareAndroidSupplementalAssets)
    }
}

val desktopAppResources = layout.buildDirectory.dir("desktop-app-resources")
val prepareDesktopAppResources by tasks.registering(Sync::class) {
    from(siteResources)
    // Compose Desktop packages the "common" subtree for every native target.
    into(desktopAppResources.map { it.dir("common") })
}

compose.desktop {
    application {
        mainClass = "com.gakaki.gt7offline.MainKt"

        jvmArgs += listOf(
            "-Dfile.encoding=UTF-8",
            "-Dgt7.offlineResources=${siteResources.asFile.absolutePath}",
            "-Xmx4g",
        )

        nativeDistributions {
            targetFormats(TargetFormat.Msi, TargetFormat.Dmg)
            packageName = "GT7OfflineArchive"
            packageVersion = "1.0.0"
            description = "Gran Turismo 7 offline vehicle and cafe archive"
            copyright = "Game data belongs to Sony Interactive Entertainment and Polyphony Digital."
            vendor = "Gakaki"
            appResourcesRootDir.set(desktopAppResources)

            windows {
                menuGroup = "GT7 Offline Archive"
                shortcut = true
                perUserInstall = true
            }
            macOS {
                bundleID = "com.gakaki.gt7offline"
                dockName = "GT7 Offline Archive"
            }
        }
    }
}

tasks.matching {
    it.name == "prepareAppResources" ||
    it.name == "createDistributable" ||
        it.name == "packageMsi" ||
        it.name == "packageDmg"
}.configureEach {
    dependsOn(prepareDesktopAppResources)
}

tasks.register("verifyOfflineSources") {
    doLast {
        val required = listOf(
            "data/cars.json",
            "data/mirror/cars.cn.json",
            "data/mirror/cars.us.json",
            "data/menu_books.json",
            "cars/car24.png",
            "avatars/extracted/sarah.png",
            "common/dist/gt7/carlist/assets/car24_1_01-CKbT4nkf.avif",
            "official/car24/car24_1_01.jxl",
        )
        val missing = required.filterNot { siteResources.file(it).asFile.isFile }
        check(missing.isEmpty()) { "Missing offline resources: ${missing.joinToString()}" }
        logger.lifecycle("Offline source check passed: ${required.size} representative resources.")
    }
}

val packageWindowsPortableZip by tasks.registering(Zip::class) {
    dependsOn("createDistributable")
    from(layout.buildDirectory.dir("compose/binaries/main/app/GT7OfflineArchive"))
    into("GT7OfflineArchive")
    archiveFileName.set("GT7OfflineArchive-1.0.0-windows-x64.zip")
    destinationDirectory.set(layout.buildDirectory.dir("artifacts"))
    isPreserveFileTimestamps = false
    isReproducibleFileOrder = true
}
