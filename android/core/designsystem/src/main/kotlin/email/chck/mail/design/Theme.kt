package email.imy.cloud.design

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.Color

private val LightColors = lightColorScheme(
    primary = ChckColor.Brand,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFE8EEFF),
    onPrimaryContainer = Color(0xFF2143A7),
    secondary = Color(0xFF55627D),
    secondaryContainer = Color(0xFFE8EEFF),
    onSecondaryContainer = Color(0xFF2143A7),
    tertiary = ChckColor.Success,
    background = Color(0xFFF5F6FA),
    onBackground = Color(0xFF202637),
    surface = Color.White,
    onSurface = Color(0xFF202637),
    surfaceVariant = Color(0xFFF0F2F8),
    onSurfaceVariant = Color(0xFF687084),
    surfaceContainer = Color(0xFFF0F2F8),
    surfaceContainerLow = Color(0xFFF7F8FC),
    surfaceContainerHigh = Color(0xFFEAEDF5),
    outline = Color(0xFF929BAE),
    outlineVariant = Color(0xFFE2E6EF),
)

private val DarkColors = darkColorScheme(
    primary = ChckColor.BrandDark,
    onPrimary = Color(0xFF102A70),
    primaryContainer = Color(0xFF26395F),
    onPrimaryContainer = Color(0xFFDDE5FF),
    secondary = Color(0xFFB6C2DE),
    secondaryContainer = Color(0xFF26395F),
    onSecondaryContainer = Color(0xFFDDE5FF),
    tertiary = ChckColor.Success,
    background = Color(0xFF13161E),
    onBackground = Color(0xFFE7EAF3),
    surface = Color(0xFF1B1F2A),
    onSurface = Color(0xFFE7EAF3),
    surfaceVariant = Color(0xFF232938),
    onSurfaceVariant = Color(0xFFAAB3C6),
    surfaceContainer = Color(0xFF1A1E29),
    surfaceContainerLow = Color(0xFF171B25),
    surfaceContainerHigh = Color(0xFF252B3A),
    outline = Color(0xFF7A849A),
    outlineVariant = Color(0xFF343C4D),
)

@Composable
fun ChckTheme(
    darkTheme: Boolean = when (AppPreferences.theme) { "light" -> false; "dark" -> true; else -> isSystemInDarkTheme() },
    dynamicColor: Boolean = false,
    lockBrandColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val colorScheme = when {
        dynamicColor && !lockBrandColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColors
        else -> LightColors
    }
    MaterialTheme(colorScheme = colorScheme, content = content)
}
