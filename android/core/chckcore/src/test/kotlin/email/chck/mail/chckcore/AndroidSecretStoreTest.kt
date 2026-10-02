package email.imy.cloud.chckcore

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class AndroidSecretStoreTest {
    @Test
    fun memoryBackendRoundtrip() {
        AndroidSecretStore.useMemory()
        assertNull(AndroidSecretStore.set("a1", "os-pass"))
        assertEquals("os-pass", AndroidSecretStore.get("a1"))
        assertNull(AndroidSecretStore.delete("a1"))
        assertNull(AndroidSecretStore.get("a1"))
    }
}
