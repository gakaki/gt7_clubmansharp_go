package com.gakaki.gt7offline

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.jetbrains.skia.Image
import java.nio.file.Files
import java.nio.file.Path

fun main() = application {
    val state = rememberWindowState(size = DpSize(1360.dp, 900.dp))
    Window(
        onCloseRequest = ::exitApplication,
        state = state,
        title = "GT7 Offline Archive",
    ) {
        Gt7OfflineApp()
    }
}

private object DesktopAssets {
    private val root: Path = sequenceOf(
        System.getProperty("compose.application.resources.dir"),
        System.getProperty("gt7.offlineResources"),
        System.getenv("GT7_OFFLINE_RESOURCES"),
        "../../site/public",
        "site/public",
    ).filterNotNull()
        .map { Path.of(it).toAbsolutePath().normalize() }
        .firstOrNull { Files.isDirectory(it) }
        ?: error("Offline resources not found. Set GT7_OFFLINE_RESOURCES.")

    fun path(relative: String): Path {
        val clean = relative.removePrefix("/")
        val target = root.resolve(clean).normalize()
        require(target.startsWith(root)) { "Invalid offline resource path: $relative" }
        return target
    }
}

actual fun createOfflineResourceReader(): OfflineResourceReader = object : OfflineResourceReader {
    override suspend fun readBytes(path: String): ByteArray = withContext(Dispatchers.IO) {
        Files.readAllBytes(DesktopAssets.path(path))
    }

    override suspend fun exists(path: String): Boolean = withContext(Dispatchers.IO) {
        Files.isRegularFile(DesktopAssets.path(path))
    }
}

actual suspend fun decodeOfflineImage(bytes: ByteArray): ImageBitmap? = withContext(Dispatchers.Default) {
    runCatching { Image.makeFromEncoded(bytes).toComposeImageBitmap() }.getOrNull()
}
