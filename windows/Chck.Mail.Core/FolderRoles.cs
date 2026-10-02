namespace Chck.Mail.Core;

public static class FolderRoles
{
    public static string FromFlagsAndName(IEnumerable<string> flags, string name)
    {
        foreach (var flag in flags)
        {
            var hit = flag.Trim('\\', ' ').ToLowerInvariant() switch
            {
                "inbox" => "inbox",
                "sent" => "sent",
                "drafts" => "drafts",
                "trash" => "trash",
                "junk" => "junk",
                "archive" => "archive",
                "flagged" or "starred" => "flagged",
                "all" or "allmail" => "all",
                _ => null,
            };
            if (hit is not null)
            {
                return hit;
            }
        }

        var n = name.Trim().Trim('"').Replace('\\', '/').ToLowerInvariant();
        return n switch
        {
            "inbox" => "inbox",
            "sent" or "sent messages" or "sent items" or "已发送" => "sent",
            "drafts" or "draft" or "草稿" => "drafts",
            "trash" or "deleted" or "deleted messages" or "已删除" or "废纸篓" => "trash",
            "junk" or "spam" or "bulk mail" or "垃圾邮件" => "junk",
            "archive" or "archived" or "归档" => "archive",
            "flagged" or "starred" or "星标" => "flagged",
            "[gmail]/all mail" or "all mail" => "all",
            _ => "custom",
        };
    }
}
