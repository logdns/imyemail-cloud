using Chck.Mail.Core;
using Chck.Mail.Data;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Chck.Mail.Tests;

public class ComposeTests
{
    [Theory]
    [InlineData("\r")]
    [InlineData("\r\n")]
    public async Task SuccessfulSendClosesDraftWhenTextBoxOnlyNormalizesLineEndings(string lineEnding)
    {
        var engine = new ControlledMailEngine();
        var completion = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Send = _ => completion.Task;
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        Assert.True(vm.BeginCompose());
        vm.ComposeTo = "recipient@example.test";
        vm.ComposeBody = "Reply\n\n> Original line\n> Next line";
        var sending = vm.SendAsync();
        vm.ComposeBody = vm.ComposeBody.Replace("\n", lineEnding, StringComparison.Ordinal);
        completion.SetResult("queued-id");
        await sending;
        Assert.Single(engine.Sent);
        Assert.False(vm.ShowCompose);
        Assert.Empty(vm.ComposeBody);
    }

    [Fact]
    public async Task LineEndingNormalizationDoesNotHideActualNewWhitespaceEdits()
    {
        var engine = new ControlledMailEngine();
        var completion = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Send = _ => completion.Task;
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        Assert.True(vm.BeginCompose());
        vm.ComposeTo = "recipient@example.test";
        vm.ComposeBody = "Reply\nQuoted line";
        var sending = vm.SendAsync();
        vm.ComposeBody = "Reply\r\nQuoted line ";
        completion.SetResult("queued-id");
        await sending;
        Assert.True(vm.ShowCompose);
        Assert.Equal("Reply\r\nQuoted line ", vm.ComposeBody);
    }

    [Fact]
    public async Task SendWithoutExplicitAccountSelectionDoesNotFallbackToFirstAccount()
    {
        var engine = new ControlledMailEngine();
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        Assert.True(vm.BeginCompose());
        vm.ComposeAccountId = "";
        vm.ComposeTo = "recipient@example.test";
        vm.ComposeBody = "Keep this draft";
        await vm.SendAsync();
        Assert.Empty(engine.Sent);
        Assert.True(vm.ShowCompose);
        Assert.Equal("Keep this draft", vm.ComposeBody);
        Assert.Contains("sending account", vm.Status);
    }

    [Fact]
    public async Task ReplyAllRoutesOriginalAccountAndFiltersOwnAndDuplicateAddresses()
    {
        var engine = new ControlledMailEngine();
        var row = new MessageRow("b:INBOX:10:1", "Meeting", "sender@example.test", "", false,
            ThreadId: "original@example.test", To: ["b@imyemail.test", "a@imyemail.test", "team@example.test"],
            Cc: ["TEAM@example.test", "cc@example.test", "b@imyemail.test"], ReplyTo: ["reply@example.test"]);
        engine.Rows["b:INBOX"] = [row];
        engine.Cached[row.Id] = new("Original body", "");
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenAsync(row);
        Assert.True(vm.BeginReply(replyAll: true));
        Assert.Equal("b", vm.ComposeAccountId);
        Assert.Equal("reply@example.test; team@example.test", vm.ComposeTo);
        Assert.Equal("cc@example.test", vm.ComposeCc);
        Assert.Equal("Re: Meeting", vm.ComposeSubject);
        Assert.Contains("> Original body", vm.ComposeBody);
        await vm.SendAsync();
        var sent = Assert.Single(engine.Sent);
        Assert.Equal("b", sent.AccountId);
        Assert.Equal("original@example.test", sent.InReplyTo);
        Assert.DoesNotContain(sent.To.Concat(sent.Cc), address => address.EndsWith("@imyemail.test", StringComparison.Ordinal));
        Assert.False(vm.ShowCompose);
    }

    [Fact]
    public async Task ReplyNeverQuotesPreviousOrLateSelectedBody()
    {
        var engine = new ControlledMailEngine();
        var old = ControlledMailEngine.Row("a:INBOX:1", unread: false);
        var next = ControlledMailEngine.Row("b:INBOX:2", unread: false);
        engine.Rows["a:INBOX"] = [old];
        engine.Rows["b:INBOX"] = [next];
        engine.Cached[old.Id] = new("OLD PRIVATE BODY", "");
        var release = new TaskCompletionSource<MessageBody>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Fetch = _ => release.Task;
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenAsync(old);
        var loading = vm.OpenAsync(next);
        Assert.False(vm.BeginReply());
        Assert.False(vm.ShowCompose);
        release.SetResult(new("New body", ""));
        await loading;
        Assert.True(vm.BeginReply());
        Assert.Contains("New body", vm.ComposeBody);
        Assert.DoesNotContain("OLD PRIVATE BODY", vm.ComposeBody);
        Assert.Equal("b", vm.ComposeAccountId);
    }

    [Fact]
    public async Task ForwardQuotesBodyAndDisclosesAttachmentBoundaryAndNewComposeClearsIt()
    {
        var engine = new ControlledMailEngine();
        var row = ControlledMailEngine.Row("b:INBOX:1", unread: false) with { HasAttachments = true };
        engine.Rows["b:INBOX"] = [row];
        engine.Cached[row.Id] = new("Forwarded body", "");
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        await vm.OpenAsync(row);
        Assert.True(vm.BeginForward());
        Assert.Equal("b", vm.ComposeAccountId);
        Assert.Empty(vm.ComposeTo);
        Assert.StartsWith("Fwd:", vm.ComposeSubject);
        Assert.Contains("Forwarded body", vm.ComposeBody);
        Assert.Contains("attachment is not attached", vm.ComposeBody);
        Assert.True(vm.BeginCompose());
        Assert.Equal("b", vm.ComposeAccountId);
        Assert.Empty(vm.ComposeTo);
        Assert.Empty(vm.ComposeCc);
        Assert.Empty(vm.ComposeSubject);
        Assert.Empty(vm.ComposeBody);
    }

    [Fact]
    public async Task SendingGuardsDuplicatesAndFailurePreservesDraftForRetry()
    {
        var engine = new ControlledMailEngine();
        var completion = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Send = _ => completion.Task;
        var vm = new MailboxViewModel(engine) { ComposeMarkdown = false };
        await vm.BootstrapAsync();
        Assert.True(vm.BeginCompose());
        vm.ComposeAccountId = "b";
        vm.ComposeTo = "recipient@example.test";
        vm.ComposeCc = "cc@example.test";
        vm.ComposeSubject = "Keep draft";
        vm.ComposeBody = "Keep content";
        var send = vm.SendAsync();
        Assert.True(vm.IsSending);
        await vm.SendAsync();
        Assert.Single(engine.Sent);
        Assert.False(vm.BeginCompose());
        completion.SetException(new IOException("disk full"));
        await send;
        Assert.False(vm.IsSending);
        Assert.True(vm.ShowCompose);
        Assert.Equal("b", vm.ComposeAccountId);
        Assert.Equal("Keep content", vm.ComposeBody);
        Assert.Equal("Keep draft", vm.ComposeSubject);
        Assert.Equal("cc@example.test", vm.ComposeCc);
        Assert.Contains("disk full", vm.Status);
    }

    [Fact]
    public async Task LateSendCompletionDoesNotDiscardNewEditsAndInvalidAccountDoesNotQueue()
    {
        var engine = new ControlledMailEngine();
        var completion = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.Send = _ => completion.Task;
        var vm = new MailboxViewModel(engine);
        await vm.BootstrapAsync();
        Assert.True(vm.BeginCompose());
        vm.ComposeTo = "recipient@example.test";
        vm.ComposeBody = "Original draft";
        var send = vm.SendAsync();
        vm.ComposeBody = "New edit";
        completion.SetResult("queued-id");
        await send;
        Assert.Equal("Original draft", Assert.Single(engine.Sent).BodyText);
        Assert.True(vm.ShowCompose);
        Assert.Equal("New edit", vm.ComposeBody);
        vm.ComposeAccountId = "removed-account";
        await vm.SendAsync();
        Assert.Single(engine.Sent);
    }

    [Fact]
    public void EnvelopeAddressesRoundTripWithoutLosingLegacyMetadataOnUpsert()
    {
        using var store = MailStore.OpenInMemory();
        store.InsertAccount(new("a", "a@example.test", "A", "custom"));
        store.UpsertFolder(new("a:INBOX", "INBOX", "inbox", 1), "a");
        store.UpsertMessage("a", "a:INBOX", 1, "Envelope", "[]", 0, "", new(), false, null, null,
            "[\"to@example.test\"]", "[\"cc@example.test\"]", "[\"reply@example.test\"]");
        store.UpsertMessage("a", "a:INBOX", 1, "Updated", "[]", 0, "", new(), false, null, null);
        var row = Assert.Single(store.ListMessages("a:INBOX"));
        Assert.Equal("to@example.test", Assert.Single(row.To!));
        Assert.Equal("cc@example.test", Assert.Single(row.Cc!));
        Assert.Equal("reply@example.test", Assert.Single(row.ReplyTo!));
        Assert.False(store.NeedsEnvelopeMetadata("a:INBOX"));
    }

    [Fact]
    public void VersionThreeMigrationPreservesExistingBodyAndRequestsEnvelopeBackfill()
    {
        var path = Path.Combine(Path.GetTempPath(), "chck-compose-migration-" + Guid.NewGuid().ToString("N") + ".db");
        using (var old = new MailStore(path))
        {
            old.InsertAccount(new("a", "a@example.test", "A", "custom"));
            old.UpsertFolder(new("a:INBOX", "INBOX", "inbox", 1), "a");
            old.UpsertMessage("a", "a:INBOX", 1, "Legacy", "[]", 0, "", new(), false, null, null);
            old.PutBody("a:INBOX:1", "<p>Kept</p>", "Kept");
        }
        using (var connection = new SqliteConnection($"Data Source={path}"))
        {
            connection.Open();
            using var command = connection.CreateCommand();
            command.CommandText = "ALTER TABLE messages DROP COLUMN reply_to_json; PRAGMA user_version = 3;";
            command.ExecuteNonQuery();
        }
        // Simulate restarting the older app; no pooled connection may retain the deliberately removed schema.
        SqliteConnection.ClearAllPools();
        using var migrated = new MailStore(path);
        Assert.Equal(4, migrated.UserVersion());
        Assert.Equal("Kept", migrated.GetBody("a:INBOX:1")?.Text);
        Assert.Empty(Assert.Single(migrated.ListMessages("a:INBOX")).ReplyTo!);
        Assert.True(migrated.NeedsEnvelopeMetadata("a:INBOX"));
    }
}
