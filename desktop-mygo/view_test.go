package main

import (
	"encoding/json"
	"image/png"
	"os"
	"strings"
	"testing"

	"github.com/egoist/mygo/ui"
)

func TestNativeInboxAndReadFlow(t *testing.T) {
	app := newMailApp(&fakeEngine{body: "Plain-text fixture"})
	app.loadInbox()
	tester := ui.NewTester(app.view, 1120, 760)
	if !tester.HasText("imyemail-cloud-mygo") || !tester.HasText("Inbox") {
		t.Fatal(tester.Texts())
	}
	if err := tester.Click("Read mail-1"); err != nil {
		t.Fatal(err)
	}
	if app.selected.ID != "mail-1" || app.body != "Plain-text fixture" {
		t.Fatal("message not read")
	}
	if err := tester.Click("Compose"); err != nil {
		t.Fatal(err)
	}
	if app.page != "compose" || !tester.HasText("New message") {
		t.Fatal("composer did not open")
	}
}

func TestReadmeScreenshot(t *testing.T) {
	path := os.Getenv("IMYEMAIL_CLOUD_CAPTURE_PATH")
	if path == "" {
		t.Skip("screenshot output not requested")
	}
	app := newMailApp(&fakeEngine{body: "Welcome to imyemail-cloud-mygo.\n\nThis is synthetic test mail rendered by the real native UI.\nNo private mailbox or credentials are used."})
	app.loadInbox()
	tester := ui.NewTester(app.view, 1120, 760)
	if err := tester.Click("Read mail-1"); err != nil {
		t.Fatal(err)
	}
	output, err := os.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := png.Encode(output, tester.Image()); err != nil {
		output.Close()
		t.Fatal(err)
	}
	if err := output.Close(); err != nil {
		t.Fatal(err)
	}
}

func TestComposerKeepsOriginalAccount(t *testing.T) {
	fake := &fakeEngine{sendState: "queued"}
	app := newMailApp(fake)
	app.loadInbox()
	tester := ui.NewTester(app.view, 1120, 760)
	if err := tester.Click("Compose"); err != nil {
		t.Fatal(err)
	}
	app.accountID = "account-2"
	app.to, app.subject, app.draft = "dev@imyemail.test", "fixture", "body"
	app.submit(true)
	for index, method := range fake.methods {
		if method == "send" {
			var payload map[string]any
			request := fake.args[index].(map[string]any)
			if err := json.Unmarshal([]byte(request["json"].(string)), &payload); err != nil {
				t.Fatal(err)
			}
			if payload["account_id"] != "account-1" {
				t.Fatal("sidebar changed the composing account")
			}
			return
		}
	}
	t.Fatal("send was not called")
}

func TestUnknownSendOutcomeDoesNotResubmit(t *testing.T) {
	fake := &fakeEngine{failure: "send"}
	app := newMailApp(fake)
	app.accountID, app.composeAccountID = "account-1", "account-1"
	app.to, app.subject, app.draft = "dev@imyemail.test", "fixture", "body"
	app.submit(true)
	if !strings.Contains(app.status, "unknown") || !strings.Contains(app.status, "outbox") {
		t.Fatal(app.status)
	}
	count := 0
	for _, method := range fake.methods {
		if method == "send" {
			count++
		}
	}
	if count != 1 || app.draft != "body" {
		t.Fatal("unknown outcome retried or discarded the draft")
	}
}

func TestNativeLoadingErrorAndNarrowLayout(t *testing.T) {
	app := newMailApp(&fakeEngine{failure: "list_accounts"})
	app.loadInbox()
	tester := ui.NewTester(app.view, 880, 640)
	if app.busy || !tester.HasText("fixture operation failed") {
		t.Fatal(tester.Texts())
	}
	if err := tester.Click("Add account"); err != nil {
		t.Fatal(err)
	}
	if !tester.HasText("Add an IMAP account") {
		t.Fatal(tester.Texts())
	}
	tester.SetDark(true)
	tester.Frame()
	if !tester.HasText("Save account") {
		t.Fatal("dark layout lost controls")
	}
}
