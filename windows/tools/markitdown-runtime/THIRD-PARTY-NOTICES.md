# Local document conversion runtime

This directory bundles official **CPython 3.13.15 for Windows x64** and
**Microsoft MarkItDown 0.1.7**, including the docx, pdf, pptx and xlsx optional
dependencies. The same helper runs on Windows x64 and on Windows 11 ARM64 through
Windows' x64 application compatibility support. The main client remains native
to its chosen architecture.

Python: https://www.python.org/ — PSF license and third-party notices are in the
bundled `LICENSE.txt`. Microsoft MarkItDown: https://github.com/microsoft/markitdown
— MIT license in `Licenses/markitdown-0.1.7-LICENSE.txt`.
Every dependency retains its original `.dist-info` metadata and license files;
some packages also place license texts beside their native libraries.
`runtime-lock.json` lists exact versions, upstream URLs and SHA-256 hashes.

Some upstream wheels do not ship full license texts. `Licenses/` supplements
MarkItDown, Magika, FlatBuffers, openpyxl and et_xmlfile with their official release
source licenses. Cobble 0.1.4 does not include a separate license file in its
release source; its original README declares 2-Clause BSD and its original setup
metadata identifies author Michael Williamson. Both originals are retained here.
`Licenses/sources.json` records exact origins and hashes of these supplements.

CPython's archive digest was read from its official python.org Sigstore bundle,
retained as `python-embed.sigstore.json`. Each wheel digest was cross-checked with
that release's official PyPI JSON metadata. Packaging verifies all pinned hashes.
This is archive integrity verification, not a claim that this build pipeline
performs complete Sigstore identity/transparency-log verification.

No user Python installation or pip is required. The embedded interpreter uses
an isolated `python313._pth`; application execution does not resolve or download
dependencies. Conversion privacy and accepted input types are enforced by the
application's `convert.py` worker, not by MarkItDown's general-purpose CLI.
