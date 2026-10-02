# Isolated mail smoke harness

Creates 2 fictional accounts, 6 folders, 64 cached messages (48 inbox), long HTML messages and local text attachments. Calls the real MailKitEngine cache and queue APIs without configuring any IMAP/SMTP host or credentials. Never calls send/sync APIs or uses the OS credential vault. Existing mail.db, WAL or SHM files cause refusal; each run needs a new directory.

```powershell
dotnet build tools/Chck.Mail.Smoke/Chck.Mail.Smoke.csproj -c Release
dotnet run --project tools/Chck.Mail.Smoke/Chck.Mail.Smoke.csproj -c Release --no-build -- --db-dir "$env:TEMP\chck-smoke-unique" --iterations 200
```

Omit `--db-dir` to create a unique temporary directory. The final stdout line is JSON, exit code 0 means all consistency assertions passed. The directory is kept for UI inspection; no user database is replaced. Cache timings are local API timings on this machine, not network mail loading or WebView rendering latency. The first read is measured separately and is not guaranteed to be an OS disk-cache cold read.

Queue verification removes one failed operation, converts one failed send into a draft without changing its JSON, and checks queued/sending/sent/draft entries remain untouched. The queued fixture is scheduled for 2100. No real recipient or delivery is involved.

For UI inspection, point an explicitly isolated application data directory at this database; do not copy over the normal user mail.db. The current app's data-directory selection must be handled separately by the caller. Bootstrap/periodic synchronization can report missing IMAP configuration: that is intentional and occurs before any network connection. Inspect inbox/folder selection, long-message scrolling, attachment labels, `Release` search, rapid mail switching and reading while this expected sync error is displayed. This fixture does not verify actual server connectivity, delivery or OS notifications.

## Optional local protocol verification

`--protocol` uses a deliberately fixed isolated GreenMail endpoint `192.168.31.8`, IMAPS 3993 / SMTPS 3465, and fictional accounts `dev@imyemail.test` / `devpass`, `alice@imyemail.test` / `alicepass`. Start that test server and seed dev's inbox first. This mode performs real IMAP header/body download, checks the cache, queues a local SMTP message for the undo window, flushes it, verifies `sent`, then reads Alice's mailbox to verify identical delivered content. It never addresses a public recipient. The self-signed-certificate exception is confined to these fixture accounts. Fictional credentials are stored in sidecars within the isolated directory so the UI can reuse this fixture.

```powershell
dotnet run --project tools/Chck.Mail.Smoke/Chck.Mail.Smoke.csproj -c Release -- --protocol --db-dir "$env:TEMP\chck-protocol-unique"
```

`coldBodyMs` includes the new IMAPS connection, authentication and body download against this local server. It does not measure a public provider or WebView rendering. OS notifications still need actual UI activation and a new test message arriving after its initial inbox snapshot.
