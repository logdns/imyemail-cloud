# Local MarkItDown import worker

`convert.py <snapshot-path>` invokes Microsoft's unmodified `markitdown==0.1.7`
format converters. It converts DOCX, PDF, PPTX, XLSX, HTML/HTM, TXT, Markdown and
CSV to editable Markdown. Images are not OCR'd, audio is not transcribed, and
scanned/image-only PDFs may return `empty_document`. Formatting extraction is
best effort; preview before sending.

One UTF-8 JSON object is written to stdout:

```json
{"markdown":"converted text","version":"0.1.7"}
```

Failure exits with code 1 and a stable `{"error":"code"}` object. Diagnostics
from dependencies are discarded rather than logging document contents/paths.
The worker does not modify the selected document. The Windows client supplies
a private local snapshot, applies a timeout/process memory limit, and removes
the snapshot after success, failure, or cancellation.

## Boundaries

- Input: 25 MiB; output: 2,000,000 Unicode characters (the Windows client also
  checks its UTF-16 string limit).
- Office archives: at most 2,048 entries, 100 MiB expanded, 50 MiB per entry,
  and 200:1 compression ratio. Duplicate/traversal/encrypted/symlink entries
  are refused; ZIPs are never extracted to disk.
- All Office XML is parsed with `defusedxml` first. DTDs and entities are
  refused. External resource relationships are refused. HTTP(S)/mailto
  hyperlink relationships may remain as inert Markdown links.
- Only the explicit official format converter is called. No general URL
  dispatcher, plugin registration, cloud client, LLM, OCR or audio converter
  is used. MarkItDown's package imports its optional converter definitions but
  they are never selected; its required Magika dependency is installed but no
  detection model is instantiated.
- During conversion an irreversible Python audit hook rejects socket use,
  child process execution, file writes, and reads outside the Python runtime
  roots. All document bytes are already in memory. This is defense in depth,
  not a claim that Python audit hooks constitute an OS security sandbox.

## Dependencies and verification

`requirements.txt` is the exact dependency set used for macOS Python 3.14
development tests. Production Windows bundles an isolated CPython 3.13 x64
runtime (also executable on Windows 11 ARM64) using the Windows-specific,
hashed wheel lock at `../markitdown-runtime/`. That lock also includes Windows
conditional packages. Installation/import never requires a user Python setup
or a network download at conversion time.

```sh
python3 -m venv /tmp/chck-markitdown-tests
/tmp/chck-markitdown-tests/bin/pip install -r tools/markitdown/requirements.txt
/tmp/chck-markitdown-tests/bin/python -m unittest discover -s tools/markitdown -p test_convert.py -v
```

Tests generate real DOCX/PPTX/XLSX/PDF inputs, verify actual upstream conversion
and Unicode/table/formatting output, and reject entities, external Office
resources, malformed inputs, ZIP bombs and limits. The HTML network regression
uses a local listener to prove document resource URLs are not fetched.

MarkItDown source: https://github.com/microsoft/markitdown/tree/v0.1.7

MarkItDown uses the MIT license, included as `LICENSE-MarkItDown.txt`. Retain
each dependency's wheel `.dist-info` license files in the bundled runtime;
dependency licenses differ and are not covered by MarkItDown's MIT license.
