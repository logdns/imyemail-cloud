package main

import (
	"fmt"
	"strings"

	"github.com/egoist/mygo/ui"
)

var inboxIcon = designIcon(`<path d="M4 5h16v14H4zM4 13h5l2 3h2l2-3h5"/>`)
var sendIcon = designIcon(`<path d="m3 4 18 8-18 8 4-8-4-8Zm4 8h14"/>`)
var accountIcon = designIcon(`<circle cx="12" cy="8" r="4"/><path d="M4 21v-2a8 8 0 0 1 16 0v2"/>`)
var folderIcon = designIcon(`<path d="M3 6h7l2 3h9v11H3z"/>`)
var lockIcon = designIcon(`<rect x="5" y="10" width="14" height="11" rx="3"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>`)

func designIcon(paths string) *ui.SVG {
	return ui.MustParseSVG([]byte(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">` + paths + `</svg>`))
}

func mailTheme(dark bool) ui.Theme {
	theme := *ui.LightTheme()
	if dark {
		theme = *ui.DarkTheme()
		theme.Background = ui.Hex("#171719")
		theme.Surface = ui.Hex("#232326")
		theme.Border = ui.Hex("#36363a")
	} else {
		theme.Background = ui.Hex("#ffffff")
		theme.Surface = ui.Hex("#f4f4f6")
		theme.Border = ui.Hex("#e5e5e9")
	}
	theme.Accent = ui.Hex("#2862ef")
	theme.AccentHover = ui.Hex("#1e52d6")
	theme.AccentPressed = ui.Hex("#1744b8")
	theme.Radius = 9
	theme.FontSize = 13
	return theme
}

func (app *mailApp) sidebar(c *ui.Context, width float32) {
	theme := c.Theme()
	ui.Column(c).Width(width).Shrink(0).FillHeight().Background(theme.Surface).Padding(22, 14).Gap(10).Children(func() {
		ui.Column(c).Padding(8, 10, 20).Gap(6).Children(func() {
			ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
				ui.Column(c).Width(30).Height(30).Radius(9).Background(theme.Accent).Center().Children(func() {
					ui.Icon(c, inboxIcon).Width(18).Height(18).TextColor(theme.AccentText)
				})
				ui.Text(c, "imyemail").FontSize(20).Bold()
			})
			ui.Text(c, "imyemail-cloud-mygo").FontSize(12).TextColor(theme.TextMuted)
		})
		app.navigation(c, "Inbox", inboxIcon, app.page == "inbox", app.loadInbox)
		app.navigation(c, "Outbox", sendIcon, app.page == "outbox", app.loadOutbox)
		ui.Text(c, "ACCOUNTS").FontSize(10).Bold().TextColor(theme.TextMuted).Padding(18, 10, 2)
		ui.Scroll(c).Grow(1).FillWidth().Children(func() {
			ui.Column(c).Gap(6).FillWidth().Children(func() {
				for _, item := range app.accounts {
					app.navigation(c.Key("account:"+item.ID), item.Email, accountIcon, app.accountID == item.ID && app.folderID != "", func() { app.loadFolders(item.ID) })
				}
				if len(app.accounts) == 0 {
					ui.Text(c, "Connect your first mailbox.").Padding(8, 10).TextColor(theme.TextMuted).FontSize(12)
				}
				if len(app.folders) > 0 {
					ui.Text(c, "FOLDERS").FontSize(10).Bold().TextColor(theme.TextMuted).Padding(18, 10, 4)
				}
				for _, item := range app.folders {
					app.navigation(c.Key("folder:"+item.ID), item.Name, folderIcon, app.folderID == item.ID, func() { app.loadFolder(item.ID, false) })
				}
			})
		})
		app.navigation(c, "Add account", accountIcon, app.page == "account", func() { app.page = "account" })
		ui.Divider(c)
		ui.Row(c).Gap(6).Padding(6, 10).Children(func() {
			ui.Icon(c, lockIcon).Width(13).Height(13).TextColor(theme.TextMuted)
			ui.Text(c, "OS credential vault").FontSize(11).TextColor(theme.TextMuted)
		})
		ui.Link(c, "About / Help", "https://imy.email").FontSize(12).Padding(0, 10)
		ui.Text(c, "MYGO / "+version).FontSize(10).TextColor(theme.TextMuted).Padding(0, 10)
	})
}

func (app *mailApp) navigation(c *ui.Context, label string, icon *ui.SVG, active bool, action func()) {
	theme := c.Theme()
	background, foreground := theme.Surface, theme.Text
	if active {
		background, foreground = theme.Accent, theme.AccentText
	}
	ui.Button(c, "").Label(label).FillWidth().Padding(10).Background(background).Border(0, ui.Transparent).Radius(8).Disabled(app.busy).OnClick(action).Children(func() {
		ui.Row(c).Gap(9).AlignItems(ui.Center).Children(func() {
			ui.Icon(c, icon).Width(16).Height(16).TextColor(foreground)
			ui.Text(c, label).FontSize(13).TextColor(foreground).Grow(1)
		})
	})
}

func (app *mailApp) summary(c *ui.Context) {
	theme := c.Theme()
	ui.Row(c).Gap(12).FillWidth().Children(func() {
		for _, item := range []struct {
			label string
			value int
			color string
			icon  *ui.SVG
		}{
			{"Accounts", len(app.accounts), "#3283e8", accountIcon},
			{"Cached messages", len(app.messages), "#26a269", inboxIcon},
			{"Loaded folders", len(app.folders), "#cf850b", folderIcon},
		} {
			ui.Column(c.Key(item.label)).Grow(1).Background(theme.Surface).Radius(15).Padding(16).Gap(9).Children(func() {
				ui.Row(c).Gap(7).AlignItems(ui.Center).Children(func() {
					ui.Icon(c, item.icon).Width(15).Height(15).TextColor(ui.Hex(item.color))
					ui.Text(c, item.label).FontSize(12).FontWeight(600).TextColor(ui.Hex(item.color))
				})
				ui.Text(c, fmt.Sprint(item.value)).FontSize(27).Bold()
			})
		}
	})
}

func (app *mailApp) filteredMessages() []message {
	var filtered []message
	query := strings.ToLower(strings.TrimSpace(app.query))
	for _, item := range app.messages {
		if strings.Contains(strings.ToLower(item.Subject+" "+item.sender()), query) {
			filtered = append(filtered, item)
		}
	}
	return filtered
}
