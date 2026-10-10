package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
)

const version = "0.3.3"

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run() error {
	showVersion := flag.Bool("version", false, "print version without opening a window")
	selfTest := flag.Bool("self-test", false, "check the bundled engine and UI without a window")
	enginePath := flag.String("engine", "", "absolute path to the core for development")
	dataPath := flag.String("data-dir", "", "absolute private data directory")
	flag.Parse()
	if *showVersion {
		fmt.Printf("imyemail-cloud-mygo %s (%s/%s)\n", version, runtime.GOOS, runtime.GOARCH)
		return nil
	}
	mygo.App.SetName("imyemail-cloud-mygo")
	mygo.App.SetVersion(version)
	if *enginePath == "" {
		resources, err := mygo.App.Path(mygo.PathResources)
		if err != nil {
			return fmt.Errorf("bundled resources unavailable")
		}
		name := "imyemail-cloud-core"
		if runtime.GOOS == "windows" {
			name += ".exe"
		}
		*enginePath = filepath.Join(resources, name)
	}
	if *dataPath == "" {
		root, err := os.UserConfigDir()
		if err != nil {
			return fmt.Errorf("private application directory unavailable")
		}
		*dataPath = filepath.Join(root, "imyemail-cloud-mygo")
	}
	if !*selfTest && !mygo.App.RequestSingleInstanceLock() {
		return nil
	}
	engine, err := startEngine(*enginePath, *dataPath)
	if err != nil {
		return err
	}
	defer engine.Close()
	if *selfTest {
		var accounts []account
		if err := callWithTimeout(engine, "list_accounts", map[string]any{}, &accounts); err != nil {
			return err
		}
		state := newMailApp(engine)
		tester := ui.NewTester(state.view, 1120, 760)
		if !tester.HasText("imyemail-cloud-mygo") || !tester.HasText("Inbox") {
			return fmt.Errorf("native view smoke test failed")
		}
		fmt.Printf("self-test OK: native MyGo UI and bundled Rust engine, version %s\n", version)
		return nil
	}
	state := newMailApp(engine)
	mygo.App.WhenReady(func() {
		window := mygo.NewWindow(mygo.WindowOptions{
			Title: "imyemail-cloud-mygo", Width: 1120, Height: 760,
			MinWidth: 880, MinHeight: 640, Content: ui.View(state.view),
		})
		state.dispatch = window.Update
		state.loadInbox()
		mygo.App.SetMenu(mygo.NewMenu([]*mygo.MenuItem{
			{Role: mygo.RoleAppMenu}, {Role: mygo.RoleEditMenu}, {Role: mygo.RoleWindowMenu},
		}))
	})
	mygo.App.OnWillQuit(func(_ *mygo.QuitEvent) { engine.Close() })
	if err := mygo.App.Run(); err != nil {
		return fmt.Errorf("native application could not start")
	}
	return nil
}
