package email.imy.cloud.chckcore

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKeys

object AndroidSecretStore {
    const val PREFS = "email.imy.cloud.secrets"

    @Volatile
    private var backend: Backend = Uninitialized

    @JvmStatic
    fun init(context: Context) {
        synchronized(this) {
            if (backend !is Uninitialized) return
            val alias = MasterKeys.getOrCreate(MasterKeys.AES256_GCM_SPEC)
            val prefs = EncryptedSharedPreferences.create(
                PREFS,
                alias,
                context.applicationContext,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )
            backend = PrefsBackend(prefs)
        }
    }

    @JvmStatic
    fun useMemory() {
        synchronized(this) { backend = MemoryBackend() }
    }

    @JvmStatic
    fun get(id: String): String? = backend.get(id)

    @JvmStatic
    fun set(id: String, secret: String): String? = backend.set(id, secret)

    @JvmStatic
    fun delete(id: String): String? = backend.delete(id)

    private interface Backend {
        fun get(id: String): String?
        fun set(id: String, secret: String): String?
        fun delete(id: String): String?
    }

    private object Uninitialized : Backend {
        override fun get(id: String): String? = null
        override fun set(id: String, secret: String): String? = "uninitialized"
        override fun delete(id: String): String? = "uninitialized"
    }

    private class PrefsBackend(private val prefs: SharedPreferences) : Backend {
        override fun get(id: String): String? = prefs.getString(id, null)
        override fun set(id: String, secret: String): String? = runCatching {
            check(prefs.edit().putString(id, secret).commit())
            null
        }.exceptionOrNull()?.message
        override fun delete(id: String): String? = runCatching {
            check(prefs.edit().remove(id).commit())
            null
        }.exceptionOrNull()?.message
    }

    private class MemoryBackend : Backend {
        private val mem = LinkedHashMap<String, String>()
        override fun get(id: String): String? = synchronized(mem) { mem[id] }
        override fun set(id: String, secret: String): String? {
            synchronized(mem) { mem[id] = secret }
            return null
        }
        override fun delete(id: String): String? {
            synchronized(mem) { mem.remove(id) }
            return null
        }
    }
}
