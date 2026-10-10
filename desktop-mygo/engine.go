package main

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

const maxRequest = 4 * 1024 * 1024
const maxResponse = 16 * 1024 * 1024

var allowedMethods = map[string]bool{
	"list_accounts": true, "list_providers": true, "add_account": true,
	"test_account": true, "remove_account": true, "list_folders": true,
	"sync_folder": true, "unified_inbox": true, "list_messages": true,
	"body": true, "cached_body": true, "save_draft": true, "send": true,
	"flush_due_sends": true, "outbox": true, "undo_send": true, "set_flags": true,
	"search": true,
}

type mailEngine interface {
	Call(context.Context, string, any, any) error
}

type engineClient struct {
	mu      sync.Mutex
	process *exec.Cmd
	input   io.WriteCloser
	output  *bufio.Scanner
	cancel  context.CancelFunc
	done    chan struct{}
	nextID  uint64
}

func privateDatabase(directory string) (string, error) {
	if !filepath.IsAbs(directory) {
		return "", errors.New("data directory must be absolute")
	}
	if err := os.MkdirAll(directory, 0700); err != nil {
		return "", errors.New("cannot create private data directory")
	}
	info, err := os.Lstat(directory)
	if err != nil || !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return "", errors.New("unsafe data directory")
	}
	if err := os.Chmod(directory, 0700); err != nil {
		return "", errors.New("cannot protect data directory")
	}
	database := filepath.Join(directory, "mail.db")
	if _, err := os.Lstat(database + ".secrets"); err == nil || !os.IsNotExist(err) {
		return "", errors.New("legacy plaintext sidecar is not accepted; use a new data directory")
	}
	if info, err = os.Lstat(database); err == nil {
		if !info.Mode().IsRegular() || info.Mode()&os.ModeSymlink != 0 {
			return "", errors.New("unsafe database path")
		}
	} else if !os.IsNotExist(err) {
		return "", errors.New("cannot inspect database path")
	}
	file, err := os.OpenFile(database, os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return "", errors.New("cannot open private database")
	}
	if err = file.Chmod(0600); err != nil {
		file.Close()
		return "", errors.New("cannot protect database")
	}
	return database, file.Close()
}

func engineEnvironment() []string {
	var environment []string
	for _, name := range []string{
		"HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "SystemRoot", "SYSTEMROOT",
		"DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR", "LANG", "TMPDIR", "TEMP", "TMP",
	} {
		if value, exists := os.LookupEnv(name); exists {
			environment = append(environment, name+"="+value)
		}
	}
	return environment
}

func startEngine(binary, directory string) (*engineClient, error) {
	if !filepath.IsAbs(binary) {
		return nil, errors.New("engine path must be absolute")
	}
	database, err := privateDatabase(directory)
	if err != nil {
		return nil, err
	}
	lifetime, cancel := context.WithCancel(context.Background())
	process := exec.CommandContext(lifetime, binary, "desktop-stdio", database)
	process.Env = engineEnvironment()
	process.Stderr = io.Discard
	input, err := process.StdinPipe()
	if err != nil {
		cancel()
		return nil, errors.New("engine IPC unavailable")
	}
	output, err := process.StdoutPipe()
	if err != nil {
		input.Close()
		cancel()
		return nil, errors.New("engine IPC unavailable")
	}
	if err := process.Start(); err != nil {
		input.Close()
		output.Close()
		cancel()
		return nil, errors.New("bundled mail engine could not start")
	}
	scanner := bufio.NewScanner(output)
	scanner.Buffer(make([]byte, 4096), maxResponse)
	client := &engineClient{process: process, input: input, output: scanner, cancel: cancel, done: make(chan struct{})}
	go func() {
		process.Wait()
		close(client.done)
	}()
	return client, nil
}

func (client *engineClient) Close() {
	client.cancel()
	<-client.done
}

func (client *engineClient) Call(ctx context.Context, method string, arguments, result any) error {
	if !allowedMethods[method] {
		return errors.New("operation is not allowed")
	}
	client.mu.Lock()
	defer client.mu.Unlock()
	client.nextID++
	request, err := json.Marshal(map[string]any{"id": client.nextID, "method": method, "args": arguments})
	if err != nil || len(request)+1 > maxRequest {
		return errors.New("request is invalid or too large")
	}
	request = append(request, '\n')
	completed := make(chan error, 1)
	go func() {
		if _, err := client.input.Write(request); err != nil {
			completed <- errors.New("mail engine disconnected")
			return
		}
		if !client.output.Scan() {
			completed <- errors.New("mail engine disconnected or response too large")
			return
		}
		var response struct {
			ID     uint64          `json:"id"`
			Result json.RawMessage `json:"result"`
			Error  string          `json:"error"`
		}
		if err := json.Unmarshal(client.output.Bytes(), &response); err != nil || response.ID != client.nextID {
			completed <- errors.New("mail engine protocol mismatch")
			return
		}
		if response.Error != "" {
			completed <- errors.New("operation failed; check account settings, network and OS credential vault")
			return
		}
		if result != nil {
			if err := json.Unmarshal(response.Result, result); err != nil {
				completed <- errors.New("mail engine returned an invalid result")
				return
			}
		}
		completed <- nil
	}()
	select {
	case err := <-completed:
		return err
	case <-ctx.Done():
		client.cancel()
		<-completed
		return errors.New("operation timed out or was cancelled; restart before retrying")
	}
}

func callWithTimeout(engine mailEngine, method string, arguments, result any) error {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()
	return engine.Call(ctx, method, arguments, result)
}

type account struct {
	ID    string `json:"id"`
	Email string `json:"email"`
}

type folder struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

type address struct {
	Email string `json:"email"`
	Name  string `json:"name,omitempty"`
}

type message struct {
	ID      string    `json:"id"`
	Subject string    `json:"subject"`
	Snippet string    `json:"snippet"`
	From    []address `json:"from"`
}

type queueItem struct {
	ID        string `json:"draft_id"`
	AccountID string `json:"account_id"`
	State     string `json:"state"`
}

func (item message) sender() string {
	if len(item.From) == 0 {
		return "Unknown sender"
	}
	if item.From[0].Name != "" {
		return item.From[0].Name
	}
	return item.From[0].Email
}

func nestedRequest(value any) map[string]any {
	encoded, _ := json.Marshal(value)
	return map[string]any{"json": string(encoded)}
}

func validateAddress(value string) error {
	if strings.ContainsAny(value, "\r\n\x00") || !strings.Contains(value, "@") {
		return fmt.Errorf("enter a valid email address")
	}
	return nil
}
