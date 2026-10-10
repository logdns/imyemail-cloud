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
	app.messages = []message{
		{ID: "mail-1", Subject: "A calmer place for your mail", From: []address{{Name: "imyemail team", Email: "hello@imyemail.test"}}, Snippet: "Meet your new native inbox."},
		{ID: "mail-2", Subject: "A few ideas for the week", From: []address{{Name: "Studio Notes", Email: "studio@imyemail.test"}}, Snippet: "Less noise. A little more focus."},
		{ID: "mail-3", Subject: "Coffee on Friday?", From: []address{{Name: "Alex Chen", Email: "alex@imyemail.test"}}, Snippet: "Let's catch up after a busy week."},
	}
	app.folders = []folder{{ID: "inbox", Name: "Inbox"}, {ID: "archive", Name: "Archive"}, {ID: "sent", Name: "Sent"}}
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
	tester.SetDark(true)
	tester.Frame()
	darkOutput, err := os.Create(path + ".dark.png")
	if err != nil {
		t.Fatal(err)
	}
	if err := png.Encode(darkOutput, tester.Image()); err != nil {
		darkOutput.Close()
		t.Fatal(err)
	}
	if err := darkOutput.Close(); err != nil {
		t.Fatal(err)
	}
}

func TestPulseInspiredLayoutKeepsActionsAndFilterStates(t *testing.T) {
	for _, dark := range []bool{false, true} {
		for _, width := range []int{880, 1120} {
			app := newMailApp(&fakeEngine{body: "fixture body"})
			app.loadInbox()
			tester := ui.NewTester(app.view, width, 640)
			tester.SetDark(dark)
			tester.Frame()
			for _, label := range []string{"Inbox", "Compose", "Add account", "Read mail-1"} {
				bounds, found := tester.Find(label)
				if !found || bounds.X < 0 || bounds.Y < 0 || bounds.X+bounds.W > float32(width) || bounds.Y+bounds.H > 640 {
					t.Fatalf("action clipped in %dpx/dark=%v: %s %+v", width, dark, label, bounds)
				}
			}
			app.query = "does not match"
			tester.Frame()
			if !tester.HasText("No matching cached messages.") {
				t.Fatal("missing filtered empty state")
			}
			app.query = ""
			tester.Frame()
			if err := tester.Click("Outbox"); err != nil || app.page != "outbox" {
				t.Fatalf("outbox action failed: %v", err)
			}
			if err := tester.Click("Add account"); err != nil || !tester.HasText("Save account") {
				t.Fatalf("account controls missing: %v", err)
			}
		}
	}
}

func TestSummaryReportsLoadedCacheNotServerTotals(t *testing.T) {
	app := newMailApp(&fakeEngine{})
	app.loadInbox()
	tester := ui.NewTester(app.view, 1120, 760)
	for _, label := range []string{"Cached messages", "Loaded folders", "1 cached"} {
		if !tester.HasText(label) {
			t.Fatal("loaded-cache scope missing: " + label)
		}
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
