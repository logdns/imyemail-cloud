package email.imy.cloud.design

import email.imy.cloud.data.L10n

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

object ChckColor {
    val Brand = Color(0xFF3D6BFE)
    val BrandDark = Color(0xFF6B8AFF)
    val Success = Color(0xFF0F9D58)
}

object ChckBrand {
    const val Product = "imyemail-cloud-native"
    const val Domain = "imy.email"
    const val ApplicationId = "email.imy.cloud"
    val Tagline get() = L10n.t("一次收件，处处原生。")
}

object ChckSpacing {
    val NavMin = 240.dp
    val NavMax = 320.dp
    val ListMin = 360.dp
    val CompactBreakpoint = 840.dp
    val ExpandedBreakpoint = 1200.dp
    val AccountBar = 3.dp
}
