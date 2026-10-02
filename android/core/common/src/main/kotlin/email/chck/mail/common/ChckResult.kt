package email.imy.cloud.common

sealed class ChckError {
    data object AuthFailed : ChckError()
    data object AuthExpired : ChckError()
    data class Network(val kind: String) : ChckError()
    data class Server(val kind: String) : ChckError()
    data class Unknown(val debugInfo: String) : ChckError()
}

sealed class ChckResult<out T> {
    data class Ok<T>(val value: T) : ChckResult<T>()
    data class Err(val error: ChckError) : ChckResult<Nothing>()
}

fun Throwable.toChckError(): ChckError {
    val msg = message.orEmpty().lowercase()
    return when {
        "auth" in msg -> ChckError.AuthFailed
        "tls" in msg || "timeout" in msg || "offline" in msg -> ChckError.Network(msg)
        else -> ChckError.Unknown(message ?: "unknown")
    }
}
