# Microsoft MarkItDown Android subset

Vendored official markitdown 0.1.7 wheel: https://files.pythonhosted.org/packages/fc/16/51d269a754d690ec31d3faa0686c8c14ac955dbc0580c358f256ba3391ec/markitdown-0.1.7-py3-none-any.whl
SHA-256: 4eca912c87c6aa6897284a7f4bf6769a23bccf8544530f5d8b175fbe3797c916
MIT license: assets/licenses/markitdown-MIT.txt.

Only DOCX, HTML and PDF converters and shared types/utilities are bundled.
Package renamed chck_markitdown; empty package initializers avoid automatic file detection (Magika/ONNX), URL converters and cloud providers.
The sole converter adaptation makes pdfplumber optional; PDF uses the upstream pdfminer fallback on Android. No PDF table inference or OCR is promised.
Selected file type is checked before invoking DocxConverter/PdfConverter directly on a local byte stream.
