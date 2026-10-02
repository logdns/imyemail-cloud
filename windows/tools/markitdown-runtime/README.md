# Building the offline Windows MarkItDown helper

```powershell
.\scripts\prepare-markitdown.ps1 -PublishDirectory .\artifacts\publish\win-arm64
# Fully offline packaging after filling the cache:
.\scripts\prepare-markitdown.ps1 -PublishDirectory .\artifacts\publish\win-arm64 -WheelCache C:\build-cache\markitdown -Offline
```

The output is `PublishDirectory/MarkItDown/`. The application invokes
`MarkItDown/python.exe -I MarkItDown/convert.py <input-path>`. `-WorkerPath` can
override the source worker, defaulting to `tools/markitdown/convert.py`.
Use the same x64 helper for x64 and Windows 11 ARM64 client packages. The script
never installs into the host interpreter, changes PATH, invokes pip or uploads
documents. Only build-time downloads from python.org/files.pythonhosted.org occur.

`runtime-lock.json` is the package-time lock: 42 exact wheels plus the official
CPython embeddable x64 archive. All hashes are mandatory. A populated `WheelCache`
contains those original archive filenames, without unpacking. The default cache
is `%LOCALAPPDATA%/imy.email-build-cache/markitdown-win-x64`.

`pylock.toml` is generated with uv for Windows x64/Python 3.13, including Windows
environment-marker dependencies such as pyreadline3 and tzdata. It intentionally
differs from the macOS worker-test requirements. To intentionally refresh the lock:

```sh
uv pip compile tools/markitdown-runtime/requirements.in --python-version 3.13.15 --python-platform x86_64-pc-windows-msvc --only-binary :all: --no-python-downloads --format pylock.toml -o tools/markitdown-runtime/pylock.toml
python3 tools/markitdown-runtime/generate_lock.py --download-cache /tmp/chck-markitdown-wheelcache
```

The generator selects compatible wheels from uv's resolution, cross-checks their
hashes against official PyPI release metadata, and pins Python's digest from its
official Sigstore bundle. It does not change the global Python environment.
Review updated versions before shipping. Windows packaging extracts wheels
without executing setup.py, preserves licenses/metadata/native DLLs, and checks
imports for all requested document converters plus an offline HTML conversion.
The application worker's document-format regression suite is a separate gate.
