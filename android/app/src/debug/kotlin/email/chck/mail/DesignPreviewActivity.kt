package email.imy.cloud

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import email.imy.cloud.data.MailboxStore
import email.imy.cloud.data.PreviewEngine

/** Explicit debug-only UI fixture; never opens a database or accesses real accounts. */
class DesignPreviewActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val model = ViewModelProvider(this, object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T = MailboxViewModel(MailboxStore(PreviewEngine())) as T
        })[MailboxViewModel::class.java]
        setContent { ChckMailApp(model) }
    }
}
