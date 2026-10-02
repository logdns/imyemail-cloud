namespace Chck.Mail.Core;

public interface ISecretStore
{
    string? Get(string id);
    void Set(string id, string secret);
    void Delete(string id);
    IReadOnlyDictionary<string, string> Snapshot();
}
