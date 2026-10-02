using System.Text.Json;

namespace Chck.Mail.Core;

public sealed class SecretStore : ISecretStore
{
    private readonly string? _path;
    private readonly Dictionary<string, string> _mem;
    private readonly object _gate = new();

    private SecretStore(string? path, Dictionary<string, string> mem)
    {
        _path = path;
        _mem = mem;
    }

    public static SecretStore Memory() => new(null, []);

    public static SecretStore OpenBeside(string dbPath)
    {
        var path = dbPath + ".secrets";
        Dictionary<string, string> mem = [];
        if (File.Exists(path))
        {
            try
            {
                mem = JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(path)) ?? [];
            }
            catch (JsonException)
            {
                mem = [];
            }
        }

        return new SecretStore(path, mem);
    }

    public string? Get(string id)
    {
        lock (_gate)
        {
            return _mem.TryGetValue(id, out var value) ? value : null;
        }
    }

    public void Set(string id, string secret)
    {
        lock (_gate)
        {
            _mem[id] = secret;
            Persist();
        }
    }

    public void Delete(string id)
    {
        lock (_gate)
        {
            _mem.Remove(id);
            Persist();
        }
    }

    public IReadOnlyDictionary<string, string> Snapshot()
    {
        lock (_gate)
        {
            return new Dictionary<string, string>(_mem);
        }
    }

    private void Persist()
    {
        if (_path is null)
        {
            return;
        }

        var dir = Path.GetDirectoryName(_path);
        if (!string.IsNullOrEmpty(dir))
        {
            Directory.CreateDirectory(dir);
        }

        var tmp = _path + ".tmp";
        File.WriteAllText(tmp, JsonSerializer.Serialize(_mem));
        File.Move(tmp, _path, overwrite: true);
    }
}
