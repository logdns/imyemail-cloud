package main

import (
	"errors"
	"fmt"
	"net/mail"
	"strconv"
	"strings"

	"github.com/egoist/mygo/ui"
)

type mailApp struct {
	engine           mailEngine
	dispatch         func(func())
	busy             bool
	status           string
	page             string
	accounts         []account
	folders          []folder
	messages         []message
	outbox           []queueItem
	accountID        string
	composeAccountID string
	folderID         string
	selected         message
	body             string
	query            string
	email            string
	password         string
	imapHost         string
	smtpHost         string
	imapPort         string
	smtpPort         string
	startTLS         bool
	to               string
	subject          string
	draft            string
	lastDraftID      string
}

func newMailApp(engine mailEngine) *mailApp {
	return &mailApp{engine: engine, page: "inbox", status: "Ready", imapPort: "993", smtpPort: "465"}
}

func (app *mailApp) task(work func() (func(), error)) {
	if app.busy {
		return
	}
	app.busy = true
	app.status = "Working..."
	run := func() {
		apply, err := work()
		finish := func() {
			app.busy = false
			if err != nil {
				app.status = err.Error()
			} else {
				app.status = "Ready"
				apply()
			}
		}
		if app.dispatch == nil {
			finish()
		} else {
			app.dispatch(finish)
		}
	}
	if app.dispatch == nil {
		run()
	} else {
		go run()
	}
}

func (app *mailApp) loadInbox() {
	app.task(func() (func(), error) {
		var accounts []account
		var messages []message
		if err := callWithTimeout(app.engine, "list_accounts", map[string]any{}, &accounts); err != nil {
			return nil, err
		}
		if err := callWithTimeout(app.engine, "unified_inbox", map[string]any{"limit": 100}, &messages); err != nil {
			return nil, err
		}
		return func() {
			app.accounts, app.messages = accounts, messages
			app.page, app.folderID = "inbox", ""
			app.selected, app.body = message{}, ""
			if len(accounts) > 0 && app.accountID == "" {
				app.accountID = accounts[0].ID
			}
		}, nil
	})
}

func (app *mailApp) loadFolders(id string) {
	app.task(func() (func(), error) {
		var folders []folder
		if err := callWithTimeout(app.engine, "list_folders", map[string]any{"account_id": id}, &folders); err != nil {
			return nil, err
		}
		return func() { app.accountID, app.folders = id, folders }, nil
	})
}

func (app *mailApp) loadOutbox() {
	app.task(func() (func(), error) {
		var items []queueItem
		if err := callWithTimeout(app.engine, "outbox", map[string]any{}, &items); err != nil {
			return nil, err
		}
		return func() { app.outbox, app.page = items, "outbox" }, nil
	})
}

func (app *mailApp) loadFolder(id string, synchronize bool) {
	app.task(func() (func(), error) {
		if synchronize {
			if err := callWithTimeout(app.engine, "sync_folder", map[string]any{"folder_id": id}, nil); err != nil {
				return nil, err
			}
		}
		var messages []message
		if err := callWithTimeout(app.engine, "list_messages", map[string]any{"folder_id": id, "limit": 100}, &messages); err != nil {
			return nil, err
		}
		return func() {
			app.messages, app.folderID, app.page = messages, id, "inbox"
			app.selected, app.body = message{}, ""
		}, nil
	})
}

func (app *mailApp) readMessage(item message) {
	app.task(func() (func(), error) {
		var body struct {
			Text string `json:"text"`
		}
		if err := callWithTimeout(app.engine, "body", map[string]any{"message_id": item.ID}, &body); err != nil {
			return nil, err
		}
		if body.Text == "" {
			body.Text = "No plain-text body. HTML and remote resources are not rendered in this edition."
		}
		return func() { app.selected, app.body = item, body.Text }, nil
	})
}

func (app *mailApp) addAccount() {
	if err := validateAddress(app.email); err != nil {
		app.status = err.Error()
		return
	}
	imapPort, err := strconv.Atoi(app.imapPort)
	if err != nil || imapPort < 1 || imapPort > 65535 {
		app.status = "IMAP port must be between 1 and 65535"
		return
	}
	smtpPort, err := strconv.Atoi(app.smtpPort)
	if err != nil || smtpPort < 1 || smtpPort > 65535 {
		app.status = "SMTP port must be between 1 and 65535"
		return
	}
	if app.password == "" || strings.TrimSpace(app.imapHost) == "" || strings.TrimSpace(app.smtpHost) == "" {
		app.status = "Email, app password and both server hosts are required"
		return
	}
	request := nestedRequest(map[string]any{
		"email": app.email, "password": app.password, "imap_host": app.imapHost,
		"imap_port": imapPort, "smtp_host": app.smtpHost, "smtp_port": smtpPort,
		"smtp_starttls": app.startTLS, "accept_invalid_certs": false,
	})
	app.password = ""
	app.task(func() (func(), error) {
		var created account
		if err := callWithTimeout(app.engine, "add_account", request, &created); err != nil {
			return nil, err
		}
		return func() {
			app.accounts = append(app.accounts, created)
			app.accountID, app.page = created.ID, "inbox"
			app.status = "Account added. Select it, choose a folder and click Sync folder."
		}, nil
	})
}

func composeRequest(id, recipients, subject, text string) (map[string]any, error) {
	if id == "" || strings.ContainsAny(subject, "\r\n\x00") || len(text) > 1024*1024 {
		return nil, errors.New("select an account; subject must be one line and body at most 1 MiB")
	}
	parsed, err := mail.ParseAddressList(recipients)
	if err != nil || len(parsed) == 0 {
		return nil, errors.New("enter valid recipient email addresses")
	}
	var to []string
	for _, recipient := range parsed {
		if err := validateAddress(recipient.Address); err != nil {
			return nil, err
		}
		to = append(to, recipient.Address)
	}
	return nestedRequest(map[string]any{
		"account_id": id, "to": to, "cc": []string{}, "bcc": []string{},
		"subject": subject, "body_text": text, "undo_window_secs": 0,
	}), nil
}

func (app *mailApp) submit(send bool) {
	request, err := composeRequest(app.composeAccountID, app.to, app.subject, app.draft)
	if err != nil {
		app.status = err.Error()
		return
	}
	app.task(func() (func(), error) {
		method := "save_draft"
		if send {
			method = "send"
		}
		var result struct {
			ID string `json:"id"`
		}
		if err := callWithTimeout(app.engine, method, request, &result); err != nil {
			if send {
				return nil, errors.New("Send outcome unknown. Restart and inspect the outbox before resubmitting.")
			}
			return nil, err
		}
		var sendState string
		var queue []queueItem
		if send {
			if err := callWithTimeout(app.engine, "outbox", map[string]any{}, &queue); err == nil {
				for _, item := range queue {
					if item.ID == result.ID {
						sendState = item.State
					}
				}
			}
		}
		return func() {
			app.lastDraftID = result.ID
			app.status = "Draft saved"
			if send {
				app.page, app.outbox = "outbox", queue
				app.to, app.subject, app.draft = "", "", ""
				app.composeAccountID = ""
				app.status = "Delivery not confirmed. Draft retained in outbox; do not resubmit."
				if sendState == "sent" {
					app.status = "SMTP submission succeeded. Recipient delivery is not guaranteed."
				}
			}
		}, nil
	})
}

func (app *mailApp) view(c *ui.Context) {
	theme := mailTheme(c.Theme().Dark)
	c.SetTheme(&theme)
	width, _ := c.Size()
	sidebarWidth := float32(220)
	if width < 1000 {
		sidebarWidth = 200
	}
	ui.Row(c).Fill().Children(func() {
		app.sidebar(c, sidebarWidth)
		ui.Divider(c)
		ui.Column(c).Grow(1).FillHeight().Padding(24).Gap(20).Children(func() {
			ui.Row(c).Gap(12).AlignItems(ui.Center).FillWidth().Children(func() {
				ui.Column(c).Grow(1).Gap(4).Children(func() {
					ui.Text(c, "Your mail, at a glance.").FontSize(23).Bold()
					ui.Text(c, "Private by design. Native by choice.").FontSize(12).TextColor(theme.TextMuted)
				})
				ui.PrimaryButton(c, "Compose").Padding(10, 16).Disabled(app.busy || len(app.accounts) == 0).OnClick(func() {
					if app.composeAccountID == "" {
						app.composeAccountID = app.accountID
					}
					app.page = "compose"
				})
			})
			if width >= 1000 && app.page == "inbox" {
				app.summary(c)
			}
			ui.Column(c).Grow(1).FillWidth().Gap(14).Children(func() {
				switch app.page {
				case "account":
					app.accountView(c)
				case "compose":
					app.composeView(c)
				case "outbox":
					ui.Text(c, "Send queue").FontSize(24).Bold()
					ui.Text(c, "Sent means SMTP submission, not guaranteed recipient delivery. Failed/unknown sends must not be blindly resubmitted.")
					ui.Scroll(c).Grow(1).Children(func() {
						for _, item := range app.outbox {
							ui.Text(c.Key(item.ID), item.State+" / "+item.ID).Padding(8)
						}
					})
				default:
					app.inboxView(c)
				}
			})
			ui.Row(c).Gap(8).AlignItems(ui.Center).FillWidth().Children(func() {
				color := theme.Success
				if app.busy {
					color = theme.Warning
				}
				ui.Column(c).Width(6).Height(6).Radius(3).Background(color)
				ui.Text(c, app.status).FontSize(11).TextColor(theme.TextMuted).Grow(1)
			})
		})
	})
}

func (app *mailApp) inboxView(c *ui.Context) {
	theme := c.Theme()
	filtered := app.filteredMessages()
	width, _ := c.Size()
	listWidth := float32(285)
	if width < 1000 {
		listWidth = 235
	}
	ui.Row(c).Gap(12).Children(func() {
		ui.Text(c, "Inbox").Bold().FontSize(20).Grow(1)
		ui.Text(c, fmt.Sprintf("%d cached", len(app.messages))).FontSize(12).TextColor(theme.TextMuted)
		ui.Button(c, "Sync folder").Disabled(app.busy || app.folderID == "").OnClick(func() { app.loadFolder(app.folderID, true) })
	})
	ui.TextInput(c, &app.query).Label("Filter cached messages").Placeholder("Filter by subject or sender")
	ui.Row(c).Grow(1).FillWidth().Gap(16).Children(func() {
		ui.Scroll(c).Width(listWidth).Shrink(0).FillHeight().Background(theme.Surface).Radius(14).Padding(8).Children(func() {
			ui.Column(c).FillWidth().Gap(6).Children(func() {
				if len(app.messages) == 0 {
					ui.Text(c, "No cached messages. Add an account or synchronize a folder.").Padding(12).FontSize(13).TextColor(theme.TextMuted)
				} else if len(filtered) == 0 {
					ui.Text(c, "No matching cached messages.").Padding(12).TextColor(theme.TextMuted)
				}
				for _, item := range filtered {
					background := theme.Surface
					if app.selected.ID == item.ID {
						background = theme.SurfacePressed
					}
					ui.Button(c.Key(item.ID), "").Label("Read "+item.ID).FillWidth().Background(background).Border(0, ui.Transparent).Padding(14).Radius(10).Disabled(app.busy).OnClick(func() { app.readMessage(item) }).Children(func() {
						ui.Column(c).FillWidth().Gap(6).Children(func() {
							ui.Text(c, item.sender()).FontSize(12).FontWeight(600).TextColor(theme.Accent)
							ui.Text(c, item.Subject).FontSize(14).Bold()
							if item.Snippet != "" {
								ui.Text(c, item.Snippet).FontSize(12).TextColor(theme.TextMuted)
							}
						})
					})
				}
			})
		})
		ui.Column(c).Grow(1).FillHeight().Background(theme.Surface).Radius(14).Padding(22).Gap(16).Children(func() {
			if app.selected.ID == "" {
				ui.Column(c).Grow(1).FillWidth().Center().Gap(14).Children(func() {
					ui.Icon(c, inboxIcon).Width(38).Height(38).TextColor(theme.TextMuted)
					ui.Text(c, "A little room to focus.").FontSize(20).Bold()
					ui.Text(c, "Choose a message to start reading.").FontSize(13).TextColor(theme.TextMuted)
				})
			} else {
				ui.Text(c, app.selected.Subject).FontSize(22).Bold()
				ui.Row(c).Gap(9).AlignItems(ui.Center).Children(func() {
					ui.Column(c).Width(32).Height(32).Radius(10).Background(theme.Background).Center().Children(func() {
						ui.Icon(c, accountIcon).Width(17).Height(17).TextColor(theme.Accent)
					})
					ui.Text(c, app.selected.sender()).FontSize(12).TextColor(theme.TextMuted).Grow(1)
				})
				ui.Divider(c)
				ui.TextArea(c, &app.body).Label("Message body").ReadOnly(true).Grow(1).FillWidth().Background(theme.Surface).Border(0, ui.Transparent).FontSize(14)
			}
		})
	})
}

func (app *mailApp) accountView(c *ui.Context) {
	ui.Text(c, "Add an IMAP account").FontSize(28).Bold()
	ui.Text(c, "Use an app password. TLS certificate verification is always enabled.")
	ui.TextInput(c, &app.email).Label("Email").Placeholder("Email address").Disabled(app.busy)
	ui.TextInput(c, &app.password).Label("App password").Placeholder("App password").Password().Disabled(app.busy)
	ui.Row(c).Gap(12).Children(func() {
		ui.TextInput(c, &app.imapHost).Label("IMAP host").Placeholder("imap.example.com").Grow(1).Disabled(app.busy)
		ui.TextInput(c, &app.imapPort).Label("IMAP port").Width(100).Disabled(app.busy)
	})
	ui.Row(c).Gap(12).Children(func() {
		ui.TextInput(c, &app.smtpHost).Label("SMTP host").Placeholder("smtp.example.com").Grow(1).Disabled(app.busy)
		ui.TextInput(c, &app.smtpPort).Label("SMTP port").Width(100).Disabled(app.busy)
	})
	ui.Checkbox(c, &app.startTLS, "SMTP STARTTLS (usually port 587)").Disabled(app.busy)
	ui.PrimaryButton(c, "Save account").Disabled(app.busy).OnClick(app.addAccount)
	ui.Text(c, "Secrets use the OS credential vault. No plaintext storage fallback.")
}

func (app *mailApp) composeView(c *ui.Context) {
	ui.Text(c, "New message").FontSize(28).Bold()
	sender := app.composeAccountID
	for _, item := range app.accounts {
		if item.ID == app.composeAccountID {
			sender = item.Email
		}
	}
	ui.Text(c, "Sending account: "+sender).FontSize(12)
	ui.TextInput(c, &app.to).Label("To").Placeholder("Recipient addresses").Disabled(app.busy)
	ui.TextInput(c, &app.subject).Label("Subject").Placeholder("Subject").Disabled(app.busy)
	ui.TextArea(c, &app.draft).Label("Draft body").Placeholder("Write a plain-text message").Grow(1).FillWidth().Disabled(app.busy)
	ui.Row(c).Gap(12).Children(func() {
		ui.Button(c, "Save draft").Disabled(app.busy).OnClick(func() { app.submit(false) })
		ui.PrimaryButton(c, "Send message").Disabled(app.busy).OnClick(func() { app.submit(true) })
		if app.lastDraftID != "" {
			ui.Text(c, fmt.Sprintf("Last draft: %s", app.lastDraftID)).FontSize(12)
		}
	})
}
