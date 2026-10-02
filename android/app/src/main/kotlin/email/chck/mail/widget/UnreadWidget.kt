package email.imy.cloud.widget

import email.imy.cloud.data.L10n

import android.content.Context
import androidx.compose.ui.unit.dp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.provideContent
import androidx.glance.layout.Alignment
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.text.Text
import email.imy.cloud.design.ChckBrand

class UnreadWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        provideContent {
            Column(
                modifier = GlanceModifier.fillMaxSize().padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalAlignment = Alignment.Start,
            ) {
                Text(ChckBrand.Product)
                Text(L10n.t("统一收件箱"))
                Text(L10n.t("未读 2"))
            }
        }
    }
}

class UnreadWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = UnreadWidget()
}
