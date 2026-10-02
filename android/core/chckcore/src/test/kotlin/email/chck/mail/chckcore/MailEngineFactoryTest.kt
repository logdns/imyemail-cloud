package email.imy.cloud.chckcore

import email.imy.cloud.data.JsonMailEngine
import email.imy.cloud.data.PreviewEngine
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlin.test.assertFailsWith
import kotlinx.coroutines.runBlocking

class MailEngineFactoryTest {
    @Test
    fun previewFlagUsesPreviewEngine() {
        val engine = MailEngineFactory.create(preview = true, nativeAvailable = false)
        assertTrue(engine is PreviewEngine)
    }

    @Test
    fun jsonInvokeIsPreferredOverPreview() = runBlocking<Unit> {
        val engine = MailEngineFactory.create(
            preview = false,
            nativeAvailable = false,
            invoke = { _, _ ->
                """[{"id":"a1","email":"dev@qq.com","display_name":"Dev","provider_id":"qq"}]"""
            },
        )
        assertTrue(engine is JsonMailEngine)
        assertTrue(engine.accounts().any { it.email == "dev@qq.com" })
    }

    @Test
    fun missingBinaryReportsFailureInsteadOfSampleMail() = runBlocking<Unit> {
        val engine = MailEngineFactory.create(
            preview = false,
            binary = null,
            nativeAvailable = false,
            invoke = null,
        )
        assertFailsWith<IllegalStateException> { engine.accounts() }
    }
}
