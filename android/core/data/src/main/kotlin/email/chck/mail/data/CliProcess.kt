package email.imy.cloud.data

import java.io.File
import java.util.concurrent.TimeUnit

object CliProcess {
    fun run(binary: String, dbPath: String, cmd: List<String>, timeoutSec: Long = 30): String {
        val proc = ProcessBuilder(listOf(binary, "--db", dbPath) + cmd).start()
        val out = proc.inputStream.bufferedReader().readText()
        val err = proc.errorStream.bufferedReader().readText()
        if (!proc.waitFor(timeoutSec, TimeUnit.SECONDS)) {
            proc.destroyForcibly()
            error("imyemail-cloud timed out")
        }
        if (proc.exitValue() != 0) {
            error(err.ifBlank { out }.trim())
        }
        return out.trim()
    }

    fun resolveBinary(extra: List<String> = emptyList()): String? {
        System.getenv("IMYEMAIL_CLOUD_MAIL")?.takeIf { File(it).canExecute() }?.let { return it }
        val candidates = extra + listOf(
            "/Users/ideadev/codedev/core/target/debug/imyemail-cloud",
            "/Users/ideadev/codedev/core/target/release/imyemail-cloud",
            "/usr/local/bin/imyemail-cloud",
            "/opt/homebrew/bin/imyemail-cloud",
        )
        return candidates.firstOrNull { File(it).canExecute() }
    }
}
