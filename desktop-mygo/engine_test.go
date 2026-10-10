package main

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestMain(testingMain *testing.M) {
	if len(os.Args) > 1 && os.Args[1] == "desktop-stdio" {
		scanner := bufio.NewScanner(os.Stdin)
		for scanner.Scan() {
			var request struct {
				ID   uint64         `json:"id"`
				Args map[string]any `json:"args"`
			}
			if err := json.Unmarshal(scanner.Bytes(), &request); err != nil {
				os.Exit(2)
			}
			if request.Args["fixture"] == "timeout" {
				time.Sleep(10 * time.Second)
			}
			if request.Args["fixture"] == "wrong-id" {
				request.ID++
			}
			fmt.Printf("{\"id\":%d,\"result\":[]}\n", request.ID)
		}
		os.Exit(0)
	}
	os.Exit(testingMain.Run())
}

type fakeEngine struct {
	methods   []string
	args      []any
	failure   string
	body      string
	sendState string
}

func (fake *fakeEngine) Call(_ context.Context, method string, args, result any) error {
	fake.methods = append(fake.methods, method)
	fake.args = append(fake.args, args)
	if fake.failure == method {
		return errors.New("fixture operation failed")
	}
	var data string
	switch method {
	case "list_accounts":
		data = `[{"id":"account-1","email":"dev@imyemail.test"}]`
	case "unified_inbox", "list_messages":
		data = `[{"id":"mail-1","subject":"Fixture","from":[{"email":"sender@imyemail.test"}]}]`
	case "list_folders":
		data = `[{"id":"folder-1","name":"INBOX"}]`
	case "body":
		encoded, _ := json.Marshal(map[string]any{"text": fake.body, "html_sanitized": "<script>fixture</script>"})
		data = string(encoded)
	case "add_account":
		data = `{"id":"account-2","email":"new@imyemail.test"}`
	case "save_draft", "send":
		data = `{"id":"draft-1"}`
	case "outbox":
		data = fmt.Sprintf(`[{"draft_id":"draft-1","state":%q}]`, fake.sendState)
	default:
		data = `{}`
	}
	if result == nil {
		return nil
	}
	return json.Unmarshal([]byte(data), result)
}

func TestRequestContractAndInjectionRejection(t *testing.T) {
	request, err := composeRequest("account-1", "Ada <ada@imyemail.test>", "subject", "body")
	if err != nil {
		t.Fatal(err)
	}
	var payload map[string]any
	if err := json.Unmarshal([]byte(request["json"].(string)), &payload); err != nil {
		t.Fatal(err)
	}
	if payload["body_text"] != "body" || payload["to"].([]any)[0] != "ada@imyemail.test" {
		t.Fatal(payload)
	}
	for _, subject := range []string{"hello\r\nBcc: fixture", "a\x00b"} {
		if _, err := composeRequest("account-1", "ada@imyemail.test", subject, "body"); err == nil {
			t.Fatal("accepted injection")
		}
	}
	if _, err := composeRequest("account-1", "bad address", "subject", "body"); err == nil {
		t.Fatal("accepted invalid recipient")
	}
}

func TestPrivateDatabaseAndSidecarRejection(t *testing.T) {
	directory := filepath.Join(t.TempDir(), "profile")
	database, err := privateDatabase(directory)
	if err != nil {
		t.Fatal(err)
	}
	if info, _ := os.Stat(directory); info.Mode().Perm() != 0700 && os.PathSeparator != '\\' {
		t.Fatal("directory permissions")
	}
	if info, _ := os.Stat(database); info.Mode().Perm() != 0600 && os.PathSeparator != '\\' {
		t.Fatal("file permissions")
	}
	if err := os.WriteFile(database+".secrets", []byte("fixture"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := privateDatabase(directory); err == nil {
		t.Fatal("accepted legacy plaintext secrets")
	}
}

func TestEngineEnvironmentExcludesOverrides(t *testing.T) {
	t.Setenv("IMYEMAIL_CLOUD_SECRETS", "file")
	t.Setenv("DYLD_INSERT_LIBRARIES", "fixture")
	t.Setenv("LD_PRELOAD", "fixture")
	for _, entry := range engineEnvironment() {
		if strings.HasPrefix(entry, "IMYEMAIL_CLOUD_") || strings.HasPrefix(entry, "DYLD_") || strings.HasPrefix(entry, "LD_") {
			t.Fatal(entry)
		}
	}
}

func TestAccountValidationClearsPassword(t *testing.T) {
	fake := &fakeEngine{}
	app := newMailApp(fake)
	app.email, app.password, app.imapHost, app.smtpHost = "new@imyemail.test", "fixture-secret", "imap.imyemail.test", "smtp.imyemail.test"
	app.addAccount()
	if app.password != "" || app.accountID != "account-2" {
		t.Fatal("password retained or account not created")
	}
	var payload map[string]any
	json.Unmarshal([]byte(fake.args[0].(map[string]any)["json"].(string)), &payload)
	if payload["accept_invalid_certs"] != false {
		t.Fatal("TLS validation disabled")
	}
}

func TestReadIgnoresHTMLAndSendShowsFailure(t *testing.T) {
	fake := &fakeEngine{sendState: "failed"}
	app := newMailApp(fake)
	app.readMessage(message{ID: "mail-1"})
	if strings.Contains(app.body, "<script>") || !strings.Contains(app.body, "not rendered") {
		t.Fatal(app.body)
	}
	app.accountID, app.to, app.subject, app.draft = "account-1", "dev@imyemail.test", "fixture", "body"
	app.composeAccountID = "account-1"
	app.submit(true)
	if !strings.Contains(app.status, "not confirmed") || app.draft != "" {
		t.Fatal(app.status)
	}
	if strings.Contains(strings.Join(fake.methods, ","), "flush_due_sends") {
		t.Fatal("send unexpectedly flushes other queued messages")
	}
}

func TestEngineCallRejectsMethodsBeforeIO(t *testing.T) {
	client := &engineClient{}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := client.Call(ctx, "restore", map[string]any{}, nil); err == nil {
		t.Fatal("restore accepted")
	}
}

func TestProcessPipesCorrelationAndCancellation(t *testing.T) {
	binary, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	for _, fixture := range []string{"ok", "wrong-id", "timeout"} {
		t.Run(fixture, func(t *testing.T) {
			client, err := startEngine(binary, t.TempDir())
			if err != nil {
				t.Fatal(err)
			}
			defer client.Close()
			deadline := 5 * time.Second
			if fixture == "timeout" {
				deadline = 200 * time.Millisecond
			}
			ctx, cancel := context.WithTimeout(context.Background(), deadline)
			defer cancel()
			var result []account
			err = client.Call(ctx, "list_accounts", map[string]any{"fixture": fixture}, &result)
			if fixture == "ok" && err != nil {
				t.Fatal(err)
			}
			if fixture != "ok" && err == nil {
				t.Fatal("missing protocol or timeout error")
			}
		})
	}
}

func TestVersionMatchesPackageMetadata(t *testing.T) {
	contents, err := os.ReadFile("mygo.json")
	if err != nil {
		t.Fatal(err)
	}
	var config struct {
		Version string `json:"version"`
	}
	if err := json.Unmarshal(contents, &config); err != nil || config.Version != version {
		t.Fatal("version metadata mismatch")
	}
}
