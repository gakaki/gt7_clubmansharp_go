package com.gakaki.gt7offline

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import android.graphics.BitmapFactory

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        AndroidAssets.initialize(applicationContext)
        enableEdgeToEdge()
        setContent {
            Gt7OfflineApp()
        }
    }
}

internal object AndroidAssets {
    private lateinit var assets: android.content.res.AssetManager

    fun initialize(context: android.content.Context) {
        assets = context.assets
    }

    fun readBytes(path: String): ByteArray =
        assets.open(path.removePrefix("/")).use { it.readBytes() }

    fun exists(path: String): Boolean =
        runCatching { assets.open(path.removePrefix("/")).use { true } }.getOrDefault(false)
}

actual fun createOfflineResourceReader(): OfflineResourceReader = object : OfflineResourceReader {
    override suspend fun readBytes(path: String): ByteArray = withContext(Dispatchers.IO) {
        AndroidAssets.readBytes(path)
    }

    override suspend fun exists(path: String): Boolean = withContext(Dispatchers.IO) {
        AndroidAssets.exists(path)
    }
}

actual suspend fun decodeOfflineImage(bytes: ByteArray): ImageBitmap? = withContext(Dispatchers.Default) {
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()
}
