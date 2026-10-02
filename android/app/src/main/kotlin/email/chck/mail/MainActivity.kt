package email.imy.cloud

import android.content.Intent
import android.net.MailTo
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    private val viewModel: MailboxViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        email.imy.cloud.design.AppPreferences.initialize(this)
        enableEdgeToEdge()
        handleIntent(intent)
        setContent { ChckMailApp(viewModel) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        when (intent?.action) {
            Intent.ACTION_SEND -> {
                val text = intent.getStringExtra(Intent.EXTRA_TEXT).orEmpty()
                val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT).orEmpty()
                viewModel.prefillCompose(subject = subject, body = text)
            }
            Intent.ACTION_SENDTO, Intent.ACTION_VIEW -> {
                val data = intent.data
                if (data?.scheme == "mailto") {
                    val mail = runCatching { MailTo.parse(data.toString()) }.getOrNull() ?: return
                    viewModel.prefillCompose(to = mail.to.orEmpty(), subject = mail.subject.orEmpty(), body = mail.body.orEmpty())
                }
            }
            "email.imy.cloud.COMPOSE" -> viewModel.setShowCompose(true)
        }
    }
}
