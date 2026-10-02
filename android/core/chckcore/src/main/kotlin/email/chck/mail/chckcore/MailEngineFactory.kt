package email.imy.cloud.chckcore

import email.imy.cloud.data.L10n

import email.imy.cloud.data.CliProcess
import email.imy.cloud.data.JsonMailEngine
import email.imy.cloud.data.MailEngine
import email.imy.cloud.data.PreviewEngine

object MailEngineFactory {
    fun create(
        dbPath: String? = null,
        preview: Boolean = envPreview(),
        binary: String? = CliProcess.resolveBinary(),
        nativeAvailable: Boolean = NativeBridge.available(),
        invoke: (suspend (String, String) -> String)? = null,
    ): MailEngine {
        if (preview) return PreviewEngine()
        if (invoke != null) return JsonMailEngine(invoke)
        if (nativeAvailable && !dbPath.isNullOrBlank()) {
            val handle = runCatching { NativeBridge.nativeOpen(dbPath) }.getOrDefault(0L)
            if (handle != 0L) {
                return JsonMailEngine { method, args ->
                    NativeBridge.nativeCall(handle, method, args.ifBlank { "{}" })
                }
            }
        }
        val bin = binary ?: return unavailable()
        val db = dbPath ?: "mail.db"
        return JsonMailEngine { method, args ->
            CliProcess.run(bin, db, listOf("ffi", "--method", method, "--args", args.ifBlank { "{}" }))
        }
    }

    fun unavailable(): MailEngine = JsonMailEngine { _, _ ->
        error(L10n.t("邮件服务暂时无法启动，请重新打开应用；若仍失败，请重新安装开发包"))
    }

    private fun envPreview(): Boolean =
        System.getenv("IMYEMAIL_CLOUD_PREVIEW") == "1" || System.getProperty("chck.preview") == "1"
}
