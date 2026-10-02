package email.imy.cloud

import android.app.Application
import androidx.hilt.work.HiltWorkerFactory
import androidx.work.Configuration
import dagger.hilt.android.HiltAndroidApp
import email.imy.cloud.chckcore.AndroidSecretStore
import email.imy.cloud.sync.SyncScheduler
import javax.inject.Inject

@HiltAndroidApp
class ChckApplication : Application(), Configuration.Provider {
    @Inject lateinit var workerFactory: HiltWorkerFactory
    @Inject lateinit var syncScheduler: SyncScheduler

    override val workManagerConfiguration: Configuration
        get() = Configuration.Builder().setWorkerFactory(workerFactory).build()

    override fun onCreate() {
        email.imy.cloud.design.AppPreferences.initialize(this)
        AndroidSecretStore.init(this)
        super.onCreate()
        syncScheduler.ensurePeriodic()
    }
}
