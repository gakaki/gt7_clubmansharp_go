package com.gakaki.gt7offline

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

private val Gt7Colors = darkColorScheme(
    primary = Color(0xFFF4F4F5),
    onPrimary = Color(0xFF111111),
    secondary = Color(0xFFB8B8BE),
    background = Color(0xFF0B0B0D),
    onBackground = Color(0xFFF7F7F8),
    surface = Color(0xFF161619),
    onSurface = Color(0xFFF4F4F5),
    surfaceVariant = Color(0xFF242428),
    onSurfaceVariant = Color(0xFFB6B6BC),
    outline = Color(0xFF3D3D43),
    error = Color(0xFFFF6B78),
)

private val Gt7Typography = Typography(
    displaySmall = TextStyle(
        fontWeight = FontWeight.Black,
        fontSize = 38.sp,
        letterSpacing = (-1.2).sp,
    ),
    headlineMedium = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 27.sp,
        letterSpacing = (-0.6).sp,
    ),
    titleLarge = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 20.sp,
    ),
    titleMedium = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 16.sp,
    ),
    bodyLarge = TextStyle(fontSize = 15.sp, lineHeight = 24.sp),
    bodyMedium = TextStyle(fontSize = 13.sp, lineHeight = 20.sp),
    labelLarge = TextStyle(fontWeight = FontWeight.SemiBold, fontSize = 13.sp),
)

@Composable
fun Gt7Theme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = Gt7Colors,
        typography = Gt7Typography,
        content = content,
    )
}
