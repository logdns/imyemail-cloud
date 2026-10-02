package email.imy.cloud.chckcore

object NativeBridge {
    @Volatile
    private var loaded: Boolean? = null

    fun available(): Boolean {
        loaded?.let { return it }
        return synchronized(this) {
            loaded ?: runCatching {
                System.loadLibrary("chck_mail")
                true
            }.getOrDefault(false).also { loaded = it }
        }
    }

    external fun nativeOpen(path: String): Long
    external fun nativeCall(handle: Long, method: String, args: String): String
    external fun nativeClose(handle: Long)
}
