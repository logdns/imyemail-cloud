package email.imy.cloud.di

import android.content.Context
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import email.imy.cloud.chckcore.AndroidSecretStore
import email.imy.cloud.chckcore.MailEngineFactory
import email.imy.cloud.data.MailEngine
import email.imy.cloud.data.MailboxStore
import email.imy.cloud.design.AppPreferences
import java.io.File
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object EngineModule {
    @Provides
    @Singleton
    fun mailEngine(@ApplicationContext context: Context): MailEngine {
        return try {
            AndroidSecretStore.init(context)
            val db = File(context.filesDir, "mail.db").absolutePath
            MailEngineFactory.create(db, binary = null)
        } catch (_: Exception) {
            MailEngineFactory.unavailable()
        }
    }

    @Provides
    @Singleton
    fun mailboxStore(@ApplicationContext context: Context, engine: MailEngine): MailboxStore =
        MailboxStore(engine, AppPreferences.savedAllowRemoteImages(context)) { value ->
            AppPreferences.saveAllowRemoteImages(context, value)
        }
}
