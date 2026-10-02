using Chck.Mail.Core;

namespace Chck.Mail.Sync;

public sealed class IdleSupervisor
{
    private readonly IMailEngine _engine;
    private CancellationTokenSource? _cts;
    private Task? _loop;

    public IdleSupervisor(IMailEngine engine)
    {
        _engine = engine;
    }

    public void Start()
    {
        _cts = new CancellationTokenSource();
        _loop = Task.Run(() => RunAsync(_cts.Token));
    }

    public async Task StopAsync()
    {
        if (_cts is null)
        {
            return;
        }

        await _cts.CancelAsync();
        if (_loop is not null)
        {
            try
            {
                await _loop;
            }
            catch (OperationCanceledException)
            {
            }
        }
    }

    private async Task RunAsync(CancellationToken ct)
    {
        var delay = TimeSpan.FromSeconds(30);
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await _engine.TickAsync(idle: true, ct: ct);
                delay = TimeSpan.FromMinutes(2);
            }
            catch (OperationCanceledException)
            {
                throw;
            }
            catch (Exception)
            {
                delay = TimeSpan.FromSeconds(Math.Min(delay.TotalSeconds * 2, 1800));
            }

            await Task.Delay(delay, ct);
        }
    }
}
