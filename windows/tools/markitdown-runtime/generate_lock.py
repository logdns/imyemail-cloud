"""Refresh distributable Windows wheel metadata from uv's target-platform pylock.

Run only when deliberately updating dependencies; packaging consumes runtime-lock.json.
No package is installed into the host interpreter.
"""
import argparse
import base64
import concurrent.futures
import hashlib
import json
import pathlib
import tomllib
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent
PYTHON_VERSION = "3.13.15"


def fetch(url):
    with urllib.request.urlopen(url, timeout=120) as response:
        return response.read()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--download-cache", type=pathlib.Path)
    args = parser.parse_args()
    compiled = tomllib.loads((ROOT / "pylock.toml").read_text())
    artifacts = []
    for package in compiled["packages"]:
        # uv already filtered tags for CPython 3.13 / Windows x64. Prefer the
        # smaller pure-Python variant where upstream also ships accelerators.
        candidate = min(package["wheels"], key=lambda wheel: wheel["size"])
        filename = urllib.parse.unquote(candidate["url"].rsplit("/", 1)[1])
        published = json.loads(fetch(f'https://pypi.org/pypi/{package["name"]}/{package["version"]}/json'))
        authoritative = next(item for item in published["urls"] if item["filename"] == filename)
        assert authoritative["url"] == candidate["url"]
        assert authoritative["digests"]["sha256"] == candidate["hashes"]["sha256"]
        artifacts.append({"name": package["name"], "version": package["version"],
                          "filename": filename, "url": candidate["url"],
                          "sha256": candidate["hashes"]["sha256"], "size": candidate["size"]})

    filename = f"python-{PYTHON_VERSION}-embed-amd64.zip"
    url = f"https://www.python.org/ftp/python/{PYTHON_VERSION}/{filename}"
    bundle_data = fetch(url + ".sigstore")
    (ROOT / "python-embed.sigstore.json").write_bytes(bundle_data)
    digest = json.loads(bundle_data)["messageSignature"]["messageDigest"]
    assert digest["algorithm"] == "SHA2_256"
    python = {"version": PYTHON_VERSION, "architecture": "x64", "filename": filename,
              "url": url, "sha256": base64.b64decode(digest["digest"]).hex(),
              "sha256Source": url + ".sigstore"}
    lock = {"schemaVersion": 1, "platform": "win_amd64", "python": python,
            "markitdown": "0.1.7", "extras": ["docx", "pdf", "pptx", "xlsx"], "wheels": artifacts}
    (ROOT / "runtime-lock.json").write_text(json.dumps(lock, indent=2) + "\n")
    print(f"Locked {len(artifacts)} wheels ({sum(a['size'] for a in artifacts) / 1024 / 1024:.1f} MiB compressed).")
    if args.download_cache:
        args.download_cache.mkdir(parents=True, exist_ok=True)

        def download(artifact):
            path = args.download_cache / artifact["filename"]
            if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]:
                return
            payload = fetch(artifact["url"])
            assert hashlib.sha256(payload).hexdigest() == artifact["sha256"], artifact["filename"]
            path.write_bytes(payload)

        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
            list(executor.map(download, [python] + artifacts))
        print(f"Verified runtime and wheel cache: {args.download_cache}")


if __name__ == "__main__":
    main()
