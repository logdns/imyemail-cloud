package email.imy.cloud.sync

import android.content.Context
import androidx.hilt.work.HiltWorker
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import dagger.assisted.Assisted
import dagger.assisted.AssistedInject
import dagger.hilt.android.qualifiers.ApplicationContext
import email.imy.cloud.data.MailEngine
import java.util.concurrent.TimeUnit
import javax.inject.Inject
import javax.inject.Singleton

@HiltWorker
class SyncWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted params: WorkerParameters,
    private val engine: MailEngine,
) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        return try {
            engine.flushDueSends()
            engine.tick(accountId = null, idle = false)
            Result.success()
        } catch (cancelled: kotlinx.coroutines.CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            Result.retry()
        }
    }
}

@Singleton
class SyncScheduler @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    fun ensurePeriodic() {
        val request = PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES)
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .build()
        WorkManager.getInstance(context)
            .enqueueUniquePeriodicWork("chck-sync", ExistingPeriodicWorkPolicy.KEEP, request)
    }
}
