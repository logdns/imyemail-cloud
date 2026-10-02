namespace Chck.Mail.Core;

/// <summary>Tracks unified inbox identities, including an explicitly empty initial baseline.</summary>
public sealed class MailArrivalTracker
{
    private readonly HashSet<string> _seen = new(StringComparer.Ordinal);
    private bool _initialized;

    public IReadOnlyList<MessageRow> Observe(IReadOnlyList<MessageRow> inbox)
    {
        var arrivals = new List<MessageRow>();
        foreach (var row in inbox)
        {
            if (_seen.Add(row.Id) && _initialized && row.Unread) arrivals.Add(row);
        }
        _initialized = true;
        return arrivals.OrderByDescending(row => row.DateUnix).ToArray();
    }

    public void Reset()
    {
        _seen.Clear();
        _initialized = false;
    }
}
