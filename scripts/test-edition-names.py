import json
import os
import pathlib
import platform
import plistlib
import shutil
import subprocess
import tempfile
import unittest
import xml.etree.ElementTree as xml


ROOT = pathlib.Path(__file__).resolve().parents[1]
NATIVE = "imyemail-cloud-native"
MYGO = "imyemail-cloud-mygo"


class EditionNamesTests(unittest.TestCase):
    def text(self, path):
        return (ROOT / path).read_text(encoding="utf-8-sig")

    def test_apple_names_preserve_bundle_identity(self):
        for path in ["apple/packaging/Info.plist", "apple/ios/Info.plist", "apple/packaging/ios/Info.plist"]:
            with self.subTest(path=path):
                data = plistlib.loads((ROOT / path).read_bytes())
                self.assertEqual(data["CFBundleDisplayName"], NATIVE)
                self.assertEqual(data["CFBundleName"], NATIVE)
                self.assertIn(data["CFBundleIdentifier"], {"email.imy.cloud", "$(PRODUCT_BUNDLE_IDENTIFIER)"})
        package = self.text("apple/scripts/package-app.sh")
        self.assertIn("dist/imyemail-cloud-native.app", package)
        self.assertIn("Contents/MacOS/imyemail-cloud", package)

    def test_android_names_preserve_application_id(self):
        for path in (ROOT / "android/app/src/main/res").glob("values*/strings.xml"):
            with self.subTest(path=path):
                self.assertEqual(xml.parse(path).find(".//string[@name='app_name']").text, NATIVE)
        self.assertIn('applicationId = "email.imy.cloud"', self.text("android/app/build.gradle.kts"))

    def test_windows_names_preserve_upgrade_and_executable(self):
        installer = self.text("windows/installer/imyemail-cloud.iss")
        self.assertIn("AppName=" + NATIVE, installer)
        self.assertIn("AppId={{39D3BD46-178F-4D6E-9D44-D42A1EDE76E7}", installer)
        self.assertIn(r"DefaultDirName={localappdata}\Programs\imyemail-cloud", installer)
        self.assertIn(r"{app}\imyemail-cloud.exe", installer)
        self.assertIn("imyemail-cloud-native-windows-", self.text("windows/scripts/build-installer.ps1"))
        manifest = self.text("windows/Chck.Mail.Package/Package.appxmanifest")
        self.assertIn('Identity Name="imyemail-cloud"', manifest)
        self.assertIn("<DisplayName>" + NATIVE + "</DisplayName>", manifest)

    def test_mygo_keeps_independent_config_and_vault(self):
        data = json.loads(self.text("desktop-mygo/mygo.json"))
        self.assertEqual(data["name"], MYGO)
        self.assertEqual(data["identifier"], "email.imy.cloud.mygo")
        main = self.text("desktop-mygo/main.go")
        self.assertIn('filepath.Join(root, "imyemail-cloud-mygo")', main)
        self.assertIn('Title: "imyemail-cloud-mygo"', main)
        self.assertIn('"email.imy.cloud.mygo"', self.text("core/crates/chck-cli/src/desktop.rs"))

    def test_release_tracks_and_current_documentation(self):
        workflow = self.text(".github/workflows/release-build.yml")
        self.assertIn("default: native-v0.2.3", workflow)
        for suffix in ["macos-arm64", "ios-arm64", "android", "linux"]:
            self.assertIn(NATIVE + "-" + suffix, workflow)
        self.assertIn('tag="mygo-v$version"', self.text(".github/workflows/mygo-desktop.yml"))
        for path in ["README.md", "README.zh-CN.md", "docs/INSTALL.md", "docs/INSTALL.zh-CN.md", "docs/VERSIONS.md"]:
            with self.subTest(path=path):
                content = self.text(path)
                self.assertIn(NATIVE, content)
                self.assertIn(MYGO, content)
                self.assertNotIn("/tag/v0.2.3", content)
                self.assertNotIn("/download/v0.2.3/", content)

    @unittest.skipUnless(platform.system() == "Linux" and shutil.which("dpkg-deb"), "Linux packaging fixture")
    def test_native_deb_replaces_legacy_only_and_preserves_alias(self):
        with tempfile.TemporaryDirectory(prefix="native-package-fixture-") as temporary:
            environment = {
                "PATH": os.environ["PATH"], "HOME": temporary, "TMPDIR": temporary,
                "IMYEMAIL_CLOUD_LINUX_BINARY": shutil.which("true"),
                "IMYEMAIL_CLOUD_LINUX_OUTPUT_DIR": temporary,
                "IMYEMAIL_CLOUD_LINUX_BUILD_DATE": "20261011",
                "IMYEMAIL_CLOUD_LINUX_MACHINE": platform.machine(),
            }
            subprocess.run(["sh", str(ROOT / "linux/scripts/package-deb.sh")], env=environment, check=True, capture_output=True, timeout=30)
            package = next(pathlib.Path(temporary).glob(NATIVE + "-*.deb"))
            subprocess.run(["sh", str(ROOT / "linux/scripts/verify-deb.sh"), str(package)], env=environment, check=True, capture_output=True, timeout=30)
            fields = subprocess.run(["dpkg-deb", "-f", str(package)], env=environment, check=True, capture_output=True, text=True, timeout=10).stdout
            self.assertIn("Package: " + NATIVE, fields)
            self.assertIn("Breaks: imyemail-cloud (<< 0.2.3)", fields)
            self.assertNotIn("Breaks: " + MYGO, fields)
            self.assertNotIn("Replaces: " + MYGO, fields)


if __name__ == "__main__":
    unittest.main()
