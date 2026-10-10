package main

import (
	"testing"

	"github.com/egoist/mygo/ui"
)

func TestNativeInboxAndReadFlow(t *testing.T) {
	app := newMailApp(&fakeEngine{body: "Plain-text fixture"})
	app.loadInbox()
	tester := ui.NewTester(app.view, 1120, 760)
	if !tester.HasText("imyemail-cloud") || !tester.HasText("Inbox") {
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
