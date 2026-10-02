"""Local, format-scoped Microsoft MarkItDown adapter. No URLs, plugins or AI."""
import io
import json
import zipfile
from chck_markitdown._stream_info import StreamInfo

MAX_BYTES = 8 * 1024 * 1024
MAX_TEXT = 200_000


def convert(data, extension):
    try:
        raw = bytes(data)
        if len(raw) > MAX_BYTES:
            raise ValueError("文件超过 8 MB，请选择较小的文档")
        stream = io.BytesIO(raw)
        if extension == ".docx":
            if not zipfile.is_zipfile(stream):
                raise ValueError("这不是有效的 Word .docx 文档")
            stream.seek(0)
            with zipfile.ZipFile(stream) as archive:
                entries = archive.infolist()
                if len(entries) > 2048 or sum(e.file_size for e in entries) > 32 * 1024 * 1024:
                    raise ValueError("文档解压后过大，请拆分后导入")
                if "word/document.xml" not in archive.namelist():
                    raise ValueError("这不是有效的 Word .docx 文档")
                for entry in entries:
                    if entry.filename.endswith((".xml", ".rels")):
                        xml = archive.read(entry).upper()
                        if b"<!DOCTYPE" in xml or b"<!ENTITY" in xml:
                            raise ValueError("文档含不支持的 XML 声明")
            stream.seek(0)
            from chck_markitdown.converters._docx_converter import DocxConverter
            result = DocxConverter().convert(stream, StreamInfo(extension=extension))
        elif extension == ".pdf":
            if not raw.startswith(b"%PDF-"):
                raise ValueError("这不是有效的 PDF 文档")
            from pdfminer.pdfpage import PDFPage
            try:
                for index, _ in enumerate(PDFPage.get_pages(stream, check_extractable=True)):
                    if index >= 100:
                        raise ValueError("PDF 超过 100 页，请拆分后导入")
            except ValueError:
                raise
            except Exception:
                raise ValueError("PDF 无法读取，可能已加密、限制复制或损坏") from None
            stream.seek(0)
            from chck_markitdown.converters._pdf_converter import PdfConverter
            result = PdfConverter().convert(stream, StreamInfo(extension=extension))
        else:
            raise ValueError("仅支持 Word .docx 和 PDF，旧版 .doc 请先另存为 .docx")
        markdown = result.markdown.strip()
        if not markdown:
            raise ValueError("文档没有可提取文字；扫描 PDF 暂不支持 OCR")
        if len(markdown) > MAX_TEXT:
            raise ValueError("转换结果超过 20 万字符，请拆分文档")
        return json.dumps({"markdown": markdown}, ensure_ascii=False)
    except ValueError as error:
        return json.dumps({"error": str(error)}, ensure_ascii=False)
    except Exception:
        return json.dumps({"error": "文档转换失败，可能已加密或损坏；原正文已保留"}, ensure_ascii=False)
