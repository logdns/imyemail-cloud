package email.imy.cloud.data

object BuiltinProviders {
    val all: List<Provider> = listOf(
        p("gmail", "Gmail", "Gmail", "imap.gmail.com", "oauth2", listOf("gmail.com", "googlemail.com"), ProviderGroup.International, "https://imy.email", "smtp.gmail.com"),
        p("outlook", "Outlook / Hotmail", "Outlook / Hotmail", "outlook.office365.com", "oauth2", listOf("outlook.com", "hotmail.com", "live.com", "msn.com"), ProviderGroup.International, "https://imy.email", "smtp.office365.com", 587),
        p("yahoo", "Yahoo", "Yahoo", "imap.mail.yahoo.com", "app-password", listOf("yahoo.com", "yahoo.co.jp"), ProviderGroup.International, "https://imy.email", "smtp.mail.yahoo.com"),
        p("icloud", "iCloud", "iCloud", "imap.mail.me.com", "app-password", listOf("icloud.com", "me.com", "mac.com"), ProviderGroup.International, "https://imy.email", "smtp.mail.me.com", 587),
        p("aol", "AOL", "AOL", "imap.aol.com", "app-password", listOf("aol.com"), ProviderGroup.International, "https://imy.email", "smtp.aol.com"),
        p("yandex", "Yandex", "Yandex", "imap.yandex.com", "password", listOf("yandex.com", "yandex.ru", "ya.ru"), ProviderGroup.International, null, "smtp.yandex.com"),
        p("mailru", "Mail.ru", "Mail.ru", "imap.mail.ru", "password", listOf("mail.ru", "inbox.ru", "list.ru", "bk.ru"), ProviderGroup.International, null, "smtp.mail.ru"),
        p("qq", L10n.t("QQ 邮箱 / Foxmail"), "QQ Mail / Foxmail", "imap.qq.com", "authcode", listOf("qq.com", "foxmail.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.qq.com"),
        p("exmail", L10n.t("腾讯企业邮"), "Tencent Exmail", "imap.exmail.qq.com", "authcode", listOf("exmail.qq.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.exmail.qq.com"),
        p("netease163", L10n.t("网易 163"), "NetEase 163", "imap.163.com", "authcode", listOf("163.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.163.com"),
        p("netease126", L10n.t("网易 126"), "NetEase 126", "imap.126.com", "authcode", listOf("126.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.126.com"),
        p("yeah", L10n.t("网易 yeah.net"), "NetEase yeah.net", "imap.yeah.net", "authcode", listOf("yeah.net"), ProviderGroup.Domestic, "https://imy.email", "smtp.yeah.net"),
        p("aliyun", L10n.t("阿里邮箱"), "Aliyun Mail", "imap.aliyun.com", "password", listOf("aliyun.com"), ProviderGroup.Domestic, null, "smtp.aliyun.com"),
        p("aliyun-qiye", L10n.t("阿里企业邮箱"), "Aliyun Enterprise Mail", "imap.qiye.aliyun.com", "password", listOf("qiye.aliyun.com"), ProviderGroup.Domestic, null, "smtp.qiye.aliyun.com"),
        p("189", L10n.t("189 邮箱"), "189 Mail", "imap.189.cn", "password", listOf("189.cn"), ProviderGroup.Domestic, null, "smtp.189.cn"),
        p("sohu", L10n.t("搜狐邮箱"), "Sohu Mail", "imap.sohu.com", "password", listOf("sohu.com"), ProviderGroup.Domestic, null, "smtp.sohu.com"),
        p("sina", L10n.t("新浪邮箱"), "Sina Mail", "imap.sina.com", "password", listOf("sina.com", "sina.cn"), ProviderGroup.Domestic, null, "smtp.sina.com"),
        p("139", L10n.t("139 邮箱"), "139 Mail", "imap.139.com", "authcode", listOf("139.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.139.com"),
        p("21cn", L10n.t("21CN 邮箱"), "21CN Mail", "imap.21cn.com", "authcode", listOf("21cn.com"), ProviderGroup.Domestic, "https://imy.email", "smtp.21cn.com"),
        p("88", L10n.t("完美邮箱"), "88.com Mail", "imap.88.com", "password", listOf("88.com"), ProviderGroup.Domestic, null, "smtp.88.com"),
    )

    fun match(email: String): Provider? {
        val domain = email.substringAfter("@", missingDelimiterValue = "").lowercase()
        if (domain.isEmpty()) return null
        return all.firstOrNull { provider ->
            provider.domains.any { d -> domain == d || domain.endsWith(".$d") }
        }
    }

    private fun p(
        id: String,
        zh: String,
        en: String,
        imap: String,
        auth: String,
        domains: List<String>,
        group: ProviderGroup,
        help: String?,
        smtp: String,
        smtpPort: Int = 465,
    ) = Provider(
        id = id,
        displayName = zh,
        displayNameEn = en,
        imapHost = imap,
        authKind = auth,
        domains = domains,
        group = group,
        helpUrl = help,
        smtpHost = smtp,
        smtpPort = smtpPort,
    )
}
