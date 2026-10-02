"""One local document -> one JSON response, using Microsoft MarkItDown 0.1.7.

Run in a dedicated, time/memory limited process; this is a conservative import
adapter, not a general purpose OS sandbox. Never pass a URL or user credentials.
"""
from __future__ import annotations

import contextlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
from urllib.parse import urlsplit
import zipfile

VERSION = "0.1.7"
MAX_INPUT = 25 * 1024 * 1024
MAX_OUTPUT = 2_000_000
MAX_ENTRIES = 2048
MAX_EXPANDED = 100 * 1024 * 1024
MAX_ENTRY = 50 * 1024 * 1024
MAX_RATIO = 200
EXTENSIONS = {".docx", ".pdf", ".pptx", ".xlsx", ".html", ".htm", ".txt", ".md", ".csv"}


class ImportFailure(Exception):
    def __init__(self, code: str):
        self.code = code


def read_snapshot(path: str) -> tuple[bytes, str]:
    # Reject UNC, device, URL and symlink inputs. The caller supplies a private
    # local snapshot; never follow a document-supplied filename here.
    if path.startswith(("\\\\", "//")) or "://" in path:
        raise ImportFailure("invalid_path")
    p = Path(path)
    extension = p.suffix.lower()
    if extension not in EXTENSIONS:
        raise ImportFailure("unsupported_format")
    if p.is_symlink():
        raise ImportFailure("invalid_path")
    with p.open("rb") as file:
        info = os.fstat(file.fileno())
        if not stat.S_ISREG(info.st_mode):
            raise ImportFailure("invalid_path")
        if info.st_size > MAX_INPUT:
            raise ImportFailure("input_too_large")
        data = file.read(MAX_INPUT + 1)
    if len(data) > MAX_INPUT:
        raise ImportFailure("input_too_large")
    if not data:
        raise ImportFailure("empty_document")
    return data, extension


def validate_office(data: bytes) -> None:
    from defusedxml import ElementTree
    from defusedxml.common import DefusedXmlException

    try:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            entries = archive.infolist()
            if len(entries) > MAX_ENTRIES:
                raise ImportFailure("archive_limits")
            names: set[str] = set()
            total = 0
            for entry in entries:
                name = entry.filename
                parts = PurePosixPath(name).parts
                if (name in names or name.startswith(("/", "\\")) or "\\" in name
                        or ".." in parts or ":" in name or entry.flag_bits & 1
                        or stat.S_ISLNK(entry.external_attr >> 16)):
                    raise ImportFailure("unsafe_document")
                names.add(name)
                total += entry.file_size
                if (entry.file_size > MAX_ENTRY or total > MAX_EXPANDED
                        or entry.file_size > max(1, entry.compress_size) * MAX_RATIO):
                    raise ImportFailure("archive_limits")
                if name.lower().endswith((".xml", ".rels")):
                    # Parse all XML before any upstream converter sees it. DTDs,
                    # even harmless ones, and all entities are refused.
                    xml = archive.read(entry)
                    root = ElementTree.fromstring(xml, forbid_dtd=True,
                                                  forbid_entities=True, forbid_external=True)
                    for node in root.iter():
                        if node.tag.rsplit("}", 1)[-1] != "Relationship":
                            continue
                        if node.get("TargetMode", "").lower() == "external":
                            target = node.get("Target", "")
                            kind = node.get("Type", "")
                            if (not kind.endswith("/hyperlink")
                                    or urlsplit(target).scheme.lower() not in {"http", "https", "mailto"}):
                                raise ImportFailure("external_resource")
    except DefusedXmlException:
        raise ImportFailure("unsafe_document") from None
    except (zipfile.BadZipFile, ElementTree.ParseError, RuntimeError, ValueError):
        raise ImportFailure("invalid_document") from None


def deny_external_actions(event: str, args: tuple) -> None:
    # platform.uname()/onnxruntime read the local computer name on Windows.
    # gethostname does not resolve an address or transmit a network request.
    if (event.startswith("socket.") and event != "socket.gethostname"
            or event in {"subprocess.Popen", "os.system",
            "os.posix_spawn", "os.spawn", "os.exec", "pty.spawn"}):
        raise ImportFailure("external_resource")


def restrict_io() -> None:
    # The process is disposable, so an irreversible audit hook is appropriate.
    # Dynamic imports and pdfminer font maps may read the bundled Python tree.
    # Document data is already in memory. No other filesystem read is needed.
    roots = {Path(sys.prefix).resolve(), Path(sys.base_prefix).resolve()}

    def audit(event: str, args: tuple) -> None:
        deny_external_actions(event, args)
        if event == "open":
            path, mode, flags = args
            if isinstance(path, int):
                return
            if (flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND)
                    or isinstance(mode, str) and any(c in mode for c in "wax+")):
                raise ImportFailure("external_resource")
            resolved = Path(os.fsdecode(path)).resolve()
            if not any(resolved.is_relative_to(root) for root in roots):
                raise ImportFailure("external_resource")

    sys.dont_write_bytecode = True
    sys.addaudithook(audit)


def convert(data: bytes, extension: str) -> str:
    import markitdown
    from markitdown import StreamInfo
    from markitdown.converters import (
        CsvConverter, DocxConverter, HtmlConverter, PdfConverter,
        PlainTextConverter, PptxConverter, XlsxConverter,
    )

    if markitdown.__version__ != VERSION:
        raise ImportFailure("runtime_version")
    if extension in {".docx", ".pptx", ".xlsx"}:
        validate_office(data)
    converters = {
        ".docx": DocxConverter, ".pdf": PdfConverter, ".pptx": PptxConverter,
        ".xlsx": XlsxConverter, ".html": HtmlConverter, ".htm": HtmlConverter,
        ".csv": CsvConverter, ".txt": PlainTextConverter, ".md": PlainTextConverter,
    }
    selected = converters[extension]()
    restrict_io()
    # No MarkItDown instance, automatic URL routing, plugins, model clients,
    # OCR, speech, browser, or cloud service is configured or invoked.
    result = selected.convert(io.BytesIO(data), StreamInfo(extension=extension),
                              keep_data_uris=False)
    markdown = result.markdown
    if len(markdown) > MAX_OUTPUT:
        raise ImportFailure("output_too_large")
    if not markdown.strip():
        raise ImportFailure("empty_document")
    return markdown


def main() -> int:
    sys.dont_write_bytecode = True
    output = sys.stdout
    if hasattr(output, "reconfigure"):
        output.reconfigure(encoding="utf-8", errors="strict", newline="\n")
    code = 0
    # Converter/library diagnostics may contain private document text or paths.
    # Silence both streams; only the bounded machine-readable result leaves.
    with open(os.devnull, "w", encoding="utf-8") as sink:
        with contextlib.redirect_stdout(sink), contextlib.redirect_stderr(sink):
            try:
                # Apply network/process restrictions before dependency imports,
                # too, while allowing their normal module/metadata file reads.
                sys.addaudithook(deny_external_actions)
                if len(sys.argv) != 2:
                    raise ImportFailure("invalid_arguments")
                data, extension = read_snapshot(sys.argv[1])
                response = {"markdown": convert(data, extension), "version": VERSION}
            except ImportFailure as error:
                response, code = {"error": error.code}, 1
            except ImportError:
                response, code = {"error": "runtime_missing"}, 1
            except (FileNotFoundError, PermissionError, OSError):
                response, code = {"error": "file_unavailable"}, 1
            except Exception:
                response, code = {"error": "conversion_failed"}, 1
    output.write(json.dumps(response, ensure_ascii=False, separators=(",", ":")) + "\n")
    output.flush()
    return code


if __name__ == "__main__":
    raise SystemExit(main())
