using System.Runtime.InteropServices;
using System.Text;

namespace Chck.Mail.Core;

public sealed class WindowsCredentialStore : ISecretStore
{
    public const string Resource = "email.imy.cloud";

    private const uint CredTypeGeneric = 1;
    private const uint CredPersistLocalMachine = 2;
    private const int ErrorNotFound = 1168;

    public string? Get(string id)
    {
        if (!OperatingSystem.IsWindows())
        {
            return null;
        }

        if (!CredRead(Target(id), CredTypeGeneric, 0, out var ptr))
        {
            var code = Marshal.GetLastWin32Error();
            if (code == ErrorNotFound) return null;
            throw EngineException.Storage($"credential read {code}");
        }

        try
        {
            var cred = Marshal.PtrToStructure<Credential>(ptr);
            if (cred.CredentialBlob == IntPtr.Zero || cred.CredentialBlobSize == 0)
            {
                return null;
            }

            var bytes = new byte[cred.CredentialBlobSize];
            Marshal.Copy(cred.CredentialBlob, bytes, 0, bytes.Length);
            return Encoding.Unicode.GetString(bytes).TrimEnd('\0');
        }
        finally
        {
            CredFree(ptr);
        }
    }

    public void Set(string id, string secret)
    {
        if (!OperatingSystem.IsWindows())
        {
            throw EngineException.Storage("credential locker requires Windows");
        }

        // CredWrite replaces an existing target atomically; deleting first loses
        // a working credential if the replacement fails.
        var blob = Encoding.Unicode.GetBytes(secret);
        var handle = GCHandle.Alloc(blob, GCHandleType.Pinned);
        try
        {
            var cred = new Credential
            {
                Type = CredTypeGeneric,
                TargetName = Target(id),
                Comment = "imyemail-cloud",
                CredentialBlobSize = (uint)blob.Length,
                CredentialBlob = handle.AddrOfPinnedObject(),
                Persist = CredPersistLocalMachine,
                UserName = id,
            };
            if (!CredWrite(ref cred, 0))
            {
                throw EngineException.Storage($"credential write {Marshal.GetLastWin32Error()}");
            }
        }
        finally
        {
            handle.Free();
        }
    }

    public void Delete(string id)
    {
        if (!OperatingSystem.IsWindows())
        {
            return;
        }

        if (!CredDelete(Target(id), CredTypeGeneric, 0))
        {
            var code = Marshal.GetLastWin32Error();
            if (code is not 0 and not ErrorNotFound)
            {
                throw EngineException.Storage($"credential delete {code}");
            }
        }
    }

    public IReadOnlyDictionary<string, string> Snapshot() => new Dictionary<string, string>();

    private static string Target(string id) => $"{Resource}/{id}";

    [DllImport("advapi32", EntryPoint = "CredWriteW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CredWrite(ref Credential credential, uint flags);

    [DllImport("advapi32", EntryPoint = "CredReadW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CredRead(string targetName, uint type, uint flags, out IntPtr credential);

    [DllImport("advapi32", EntryPoint = "CredDeleteW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CredDelete(string targetName, uint type, uint flags);

    [DllImport("advapi32", EntryPoint = "CredFree")]
    private static extern void CredFree(IntPtr buffer);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct Credential
    {
        public uint Flags;
        public uint Type;
        public string TargetName;
        public string Comment;
        public long LastWritten;
        public uint CredentialBlobSize;
        public IntPtr CredentialBlob;
        public uint Persist;
        public uint AttributeCount;
        public IntPtr Attributes;
        public string TargetAlias;
        public string UserName;
    }
}
