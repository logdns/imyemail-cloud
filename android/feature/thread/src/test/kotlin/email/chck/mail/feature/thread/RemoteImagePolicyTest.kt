package email.imy.cloud.feature.thread

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class RemoteImagePolicyTest {
    @Test
    fun remoteImagesAreBlockedByDefaultAndHttpsOnlyWhenEnabled() {
        val blocked = remoteImageDocument("<img src=\"https://images.test/a.png\">", "", false)
        assertTrue("img-src data: cid:" in blocked)
        assertFalse("img-src https:" in blocked)

        val allowed = remoteImageDocument("<img src=\"https://images.test/a.png\">", "", true)
        assertTrue("img-src https: data: cid:" in allowed)
        assertFalse("img-src http:" in allowed)
        assertTrue("default-src 'none'" in allowed)
    }
}
