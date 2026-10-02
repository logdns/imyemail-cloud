namespace Chck.Mail.Core;

public sealed class OverlaySecretStore : ISecretStore
{
    private readonly ISecretStore _primary;
    private readonly SecretStore _fallback;

    public OverlaySecretStore(ISecretStore primary, SecretStore fallback)
    {
        _primary = primary;
        _fallback = fallback;
        Migrate();
    }

    public string? Get(string id) => _primary.Get(id) ?? _fallback.Get(id);

    public void Set(string id, string secret)
    {
        // The sidecar is a legacy migration source, never a plaintext fallback
        // for newly written credentials when the operating-system vault fails.
        _primary.Set(id, secret);
        _fallback.Delete(id);
    }

    public void Delete(string id)
    {
        // Implementations treat a missing entry as success. Other failures must
        // reach the caller so removing an account cannot silently retain secrets.
        _primary.Delete(id);
        _fallback.Delete(id);
    }

    public IReadOnlyDictionary<string, string> Snapshot()
    {
        var map = new Dictionary<string, string>(_fallback.Snapshot());
        foreach (var (id, secret) in _primary.Snapshot())
        {
            map[id] = secret;
        }

        return map;
    }

    private void Migrate()
    {
        foreach (var (id, secret) in _fallback.Snapshot())
        {
            if (_primary.Get(id) is not null)
            {
                _fallback.Delete(id);
                continue;
            }

            try
            {
                _primary.Set(id, secret);
                _fallback.Delete(id);
            }
            catch
            {
                // keep sidecar until OS store accepts the secret
            }
        }
    }
}
