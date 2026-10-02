package email.imy.cloud.design

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

@Composable
fun UnreadDot(visible: Boolean, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .size(8.dp)
            .clip(CircleShape)
            .background(if (visible) MaterialTheme.colorScheme.primary else Color.Transparent),
    )
}

@Composable
fun AccountColorBar(color: Long, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .width(ChckSpacing.AccountBar)
            .background(Color(color)),
    )
}

@Composable
fun ChckEmptyState(
    title: String,
    body: String,
    action: String? = null,
    onAction: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        Spacer(Modifier.height(8.dp))
        Text(body, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (action != null && onAction != null) {
            Spacer(Modifier.height(12.dp))
            TextButton(onClick = onAction) { Text(action) }
        }
    }
}
