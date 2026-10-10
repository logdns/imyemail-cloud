import base64
import hashlib
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
FINGERPRINT = "476ea6e573714e9c46a43471b5938d791e1c3851beb7b3eef8269cf084f48548"


class ReleaseSigningTests(unittest.TestCase):
    def command(self, *arguments, **kwargs):
        return subprocess.run(arguments, check=True, capture_output=True, timeout=30, **kwargs)

    def test_hosted_cleanup_pins_certificate_and_rejects_self_hosted(self):
        with tempfile.TemporaryDirectory(prefix="cleanup-fixture-") as temporary:
            root = pathlib.Path(temporary)
            tools = root / "tools"
            tools.mkdir()
            (root / "imyemail-signing").mkdir()
            log = root / "calls"
            security = tools / "security"
            security.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> "$FIXTURE_LOG"
if [ "$1" = "delete-certificate" ] && [ "${FIXTURE_FAIL:-0}" = 1 ]; then
  exit 1
fi
''')
            security.chmod(0o755)
            sudo = tools / "sudo"
            sudo.write_text('#!/bin/sh\nshift\nexec "$@"\n')
            sudo.chmod(0o755)
            environment = {
                "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                "HOME": temporary, "RUNNER_TEMP": temporary,
                "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/main",
                "RUNNER_ENVIRONMENT": "github-hosted", "FIXTURE_LOG": str(log),
            }
            script = ROOT / "scripts/cleanup-macos-signing.sh"
            self.command("bash", str(script), env=environment, cwd=ROOT)
            self.assertIn("delete-certificate -Z " + FINGERPRINT.upper(), log.read_text())
            self.assertIn("delete-keychain", log.read_text())
            log.write_text("")
            environment["FIXTURE_FAIL"] = "1"
            result = subprocess.run(["bash", str(script)], env=environment, cwd=ROOT, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("delete-keychain", log.read_text())
            environment["RUNNER_ENVIRONMENT"] = "self-hosted"
            log.write_text("")
            result = subprocess.run(["bash", str(script)], env=environment, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(log.read_text(), "")

    def test_public_certificate_and_key_match(self):
        certificate = ROOT / "signing/release-cert.pem"
        der = self.command("openssl", "x509", "-in", str(certificate), "-outform", "DER").stdout
        self.assertEqual(hashlib.sha256(der).hexdigest(), FINGERPRINT)
        self.assertEqual(der, (ROOT / "signing/release-cert.der").read_bytes())
        public = self.command("openssl", "x509", "-in", str(certificate), "-pubkey", "-noout").stdout
        self.assertEqual(public, (ROOT / "signing/release-public.pem").read_bytes())
        text = self.command("openssl", "x509", "-in", str(certificate), "-text", "-noout").stdout.decode()
        self.assertIn("CA:FALSE", text)
        self.assertIn("Code Signing", text)

    def test_signed_manifest_rejects_tampering(self):
        with tempfile.TemporaryDirectory(prefix="signing-test-") as temporary:
            root = pathlib.Path(temporary)
            (root / "signing").mkdir()
            assets = root / "assets"
            assets.mkdir()
            package = assets / "fixture.zip"
            package.write_bytes(b"fixture package, no executable")
            key = root / "fixture.key"
            self.command("openssl", "genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(key))
            self.command("openssl", "req", "-new", "-x509", "-key", str(key), "-subj", "/CN=Local Fixture", "-days", "1", "-out", str(root / "signing/release-cert.pem"))
            self.command("openssl", "x509", "-in", str(root / "signing/release-cert.pem"), "-outform", "DER", "-out", str(root / "signing/release-cert.der"))
            public = self.command("openssl", "pkey", "-in", str(key), "-pubout").stdout
            (root / "signing/release-public.pem").write_bytes(public)
            script = root / "sign-manifest.sh"
            shutil.copyfile(ROOT / "scripts/sign-release-manifest.sh", script)
            environment = {
                "PATH": os.environ["PATH"], "HOME": temporary, "RUNNER_TEMP": temporary,
                "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/main",
                "GITHUB_SHA": "fixture", "GITHUB_REPOSITORY": "fixture/local", "GITHUB_RUN_ID": "1",
                "RELEASE_KEY_BASE64": base64.b64encode(key.read_bytes()).decode(),
            }
            self.command("bash", str(script), str(assets), cwd=root, env=environment)
            manifest = assets / "SHA256SUMS"
            self.assertIn(hashlib.sha256(package.read_bytes()).hexdigest(), manifest.read_text())
            verification = ["openssl", "dgst", "-sha256", "-verify", str(root / "signing/release-public.pem"), "-signature", str(assets / "SHA256SUMS.sig"), str(manifest)]
            self.command(*verification)
            manifest.write_text(manifest.read_text() + "altered\n")
            result = subprocess.run(verification, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(list(root.glob("release-key.*")), [])
            environment["GITHUB_EVENT_NAME"] = "pull_request"
            result = subprocess.run(["bash", str(script), str(assets)], cwd=root, env=environment, capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
