package email.imy.cloud.design

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import email.imy.cloud.data.L10n

object AppPreferences {
    var theme by mutableStateOf("system")
        private set
    var allowRemoteImages by mutableStateOf(false)
        private set
    fun initialize(context: Context) {
        val preferences = context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE)
        L10n.language = L10n.normalized(preferences.getString("language", "en"))
        theme = preferences.getString("theme", "system").orEmpty().takeIf { it in listOf("system", "light", "dark") } ?: "system"
        allowRemoteImages = preferences.getBoolean("allow_remote_images", false)
    }
    fun saveTheme(context: Context, value: String) {
        theme = value
        context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE).edit().putString("theme", value).apply()
    }
    fun savedLanguage(context: Context) = L10n.normalized(context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE).getString("language", "en"))
    fun saveLanguage(context: Context, value: String) {
        context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE).edit().putString("language", L10n.normalized(value)).apply()
    }
    fun savedAllowRemoteImages(context: Context): Boolean = context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE)
        .getBoolean("allow_remote_images", false)
    fun saveAllowRemoteImages(context: Context, value: Boolean) {
        allowRemoteImages = value
        context.getSharedPreferences("chck.appearance", Context.MODE_PRIVATE).edit().putBoolean("allow_remote_images", value).apply()
    }
}
