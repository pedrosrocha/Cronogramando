"""PDF extraction with two deterministic engines.

The sidecar keeps the heavy PDF libraries out of import time so the pure
block/markdown helpers can be unit-tested without PyMuPDF installed.
"""

from __future__ import annotations

import base64
import os
from statistics import median
from typing import Any, Iterable

ACADEMIC_CONNECTORS = {"arxiv", "doaj", "openalex"}
RASTER_TYPES = {"image", "table_image", "formula_image"}

# Characters that are a strong signal for a rasterized formula block.
MATH_CHARS = set("=+−-×÷∑∫√≤≥≠≈∞πθαβγλμσΔ∂^_{}|<>±·∘")
MATH_CORE_CHARS = set("=∫∑√≤≥≠≈^")

MIN_RASTER_PIXELS = 6.0
RASTER_ZOOM = 2.0
MAX_PDF_BYTES = int(os.environ.get("PDF_MAX_BYTES", 64 * 1024 * 1024))

# `find_tables()` is accurate but costs ~1s/page on large textbooks. Small
# documents keep it; large books switch to a fast line-grid detector so the
# first extraction stays in the same order of magnitude as the download.
TABLE_DETECTION = os.environ.get("PDF_TABLE_DETECTION", "auto").strip().lower()
TABLE_FINDER_MAX_PAGES = int(os.environ.get("PDF_TABLE_FINDER_MAX_PAGES", "20"))


def select_engine(connector: str) -> str:
    """Pick the extraction engine deterministically from the connector name."""

    name = (connector or "").strip().lower()
    return "academic" if name in ACADEMIC_CONNECTORS else "normal"


def blocks_to_markdown(blocks: Iterable[dict[str, Any]]) -> str:
    """Render the unified block schema as Markdown."""

    parts: list[str] = []
    for block in blocks:
        kind = block.get("type")
        if kind == "heading":
            text = _normalize(block.get("text") or "")
            if not text:
                continue
            level = _heading_level(block.get("level"))
            parts.append(f"{'#' * level} {text}")
        elif kind == "paragraph":
            text = _normalize(block.get("text") or "")
            if text:
                parts.append(text)
        elif kind in RASTER_TYPES:
            image = block.get("image") or ""
            if image:
                label = kind.removesuffix("_image")
                parts.append(f"![{label}]({image})")
    return "\n\n".join(parts).strip()


def extract_pdf(data: bytes, connector: str) -> dict[str, Any]:
    """Extract a PDF into the unified block schema plus Markdown."""

    if len(data) > MAX_PDF_BYTES:
        raise ValueError(f"PDF exceeds the {MAX_PDF_BYTES} byte limit")
    if not data.startswith(b"%PDF"):
        raise ValueError("uploaded data is not a PDF")

    fitz = _import_fitz()
    engine = select_engine(connector)

    document = fitz.open(stream=data, filetype="pdf")
    try:
        if document.page_count == 0:
            raise ValueError("PDF has no pages")

        table_mode = _table_mode(document.page_count)
        blocks: list[dict[str, Any]] = []
        for page_index in range(document.page_count):
            page = document.load_page(page_index)
            blocks.extend(
                _extract_page(fitz, page, page_index + 1, engine, table_mode)
            )

        return {
            "connector": connector,
            "engine": engine,
            "page_count": document.page_count,
            "blocks": blocks,
            "markdown": blocks_to_markdown(blocks),
        }
    finally:
        document.close()


def _import_fitz():
    try:
        import fitz  # type: ignore
    except ImportError as exc:  # pragma: no cover - depends on deployment
        raise RuntimeError(
            "PyMuPDF is required for PDF extraction (pip install pymupdf)"
        ) from exc
    return fitz


def _table_mode(page_count: int) -> str:
    if TABLE_DETECTION in {"lines", "pymupdf"}:
        return TABLE_DETECTION
    return "pymupdf" if page_count <= TABLE_FINDER_MAX_PAGES else "lines"


def _extract_page(
    fitz, page, page_number: int, engine: str, table_mode: str
) -> list[dict[str, Any]]:
    data = page.get_text("dict")
    page_width = float(data.get("width") or page.rect.width or 1.0)
    page_height = float(data.get("height") or page.rect.height or 1.0)

    table_rects = _find_table_rects(page, table_mode, page_width, page_height)
    items: list[dict[str, Any]] = []

    text_blocks: list[tuple[tuple[float, float, float, float], str, float]] = []
    for block in data.get("blocks", []):
        bbox = _bbox(block.get("bbox"))
        if bbox is None:
            continue
        if any(_overlap_ratio(bbox, table) >= 0.5 for table in table_rects):
            continue
        if block.get("type") == 1:
            raster = _raster_item(
                fitz, page, "image", bbox, page_number, page_width, page_height
            )
            if raster is not None:
                items.append(raster)
            continue

        text, max_size = _block_text(block)
        if text:
            text_blocks.append((bbox, text, max_size))

    body_size = _body_font_size([size for _, _, size in text_blocks])
    for bbox, text, max_size in text_blocks:
        heading_level = _heading_level_for(text, max_size, body_size)
        if heading_level is not None:
            items.append(
                {
                    "type": "heading",
                    "level": heading_level,
                    "text": text,
                    "page": page_number,
                    "bbox": list(bbox),
                }
            )
            continue

        if _looks_like_formula(text):
            raster = _raster_item(
                fitz,
                page,
                "formula_image",
                bbox,
                page_number,
                page_width,
                page_height,
            )
            if raster is not None:
                items.append(raster)
            continue

        items.append(
            {
                "type": "paragraph",
                "text": text,
                "page": page_number,
                "bbox": list(bbox),
            }
        )

    for table_rect in table_rects:
        raster = _raster_item(
            fitz,
            page,
            "table_image",
            table_rect,
            page_number,
            page_width,
            page_height,
        )
        if raster is not None:
            items.append(raster)

    return _order_items(items, page_width, engine)


def _block_text(block: dict[str, Any]) -> tuple[str, float]:
    lines: list[str] = []
    max_size = 0.0
    for line in block.get("lines", []):
        spans = line.get("spans", [])
        for span in spans:
            max_size = max(max_size, float(span.get("size") or 0.0))
        text = "".join(span.get("text") or "" for span in spans)
        text = _normalize(text)
        if text:
            lines.append(text)
    return _normalize(" ".join(lines)), max_size


def _body_font_size(sizes: list[float]) -> float:
    usable = [size for size in sizes if size > 0]
    if not usable:
        return 11.0
    return float(median(usable))


def _heading_level_for(text: str, size: float, body_size: float) -> int | None:
    if len(text) > 180:
        return None
    if size >= body_size * 1.45:
        return 1
    if size >= body_size * 1.15:
        return 2
    return None


def _heading_level(value: Any) -> int:
    try:
        level = int(value)
    except (TypeError, ValueError):
        level = 2
    return max(1, min(6, level))


def _looks_like_formula(text: str) -> bool:
    """Heuristic: symbol-dense, short text is rasterized as a formula image."""

    if not text or len(text) > 240:
        return False
    chars = [char for char in text if not char.isspace()]
    if not chars:
        return False
    math = sum(1 for char in chars if char in MATH_CHARS)
    ratio = math / len(chars)
    has_core = any(char in MATH_CORE_CHARS for char in text)
    return has_core and (ratio >= 0.18 or (ratio >= 0.12 and len(text) < 80))


def _find_table_rects(
    page, mode: str, page_width: float, page_height: float
) -> list[tuple[float, float, float, float]]:
    if mode == "pymupdf":
        try:
            finder = page.find_tables()
        except Exception:  # pragma: no cover - PyMuPDF version differences
            return []
        rects: list[tuple[float, float, float, float]] = []
        for table in getattr(finder, "tables", []):
            rect = _bbox(getattr(table, "bbox", None))
            if rect is not None:
                rects.append(rect)
        return rects
    return _line_table_rects(page, page_width, page_height)


def _line_table_rects(
    page, page_width: float, page_height: float
) -> list[tuple[float, float, float, float]]:
    """Fast table detection from the page's vector lines.

    PyMuPDF's table finder is accurate but expensive on long textbooks. This
    detector clusters intersecting horizontal/vertical rules, which covers
    ruled tables (the common case in OpenStax) without a per-page text scan.
    """

    try:
        drawings = page.get_drawings()
    except Exception:  # pragma: no cover - defensive
        return []

    horizontal, vertical = _drawing_lines(drawings)
    boxes = _line_clusters(horizontal, vertical)
    page_area = page_width * page_height
    rects: list[tuple[float, float, float, float]] = []
    for x0, y0, x1, y1, _, _ in boxes:
        width = x1 - x0
        height = y1 - y0
        if width * height >= page_area * 0.5:
            continue
        if width >= page_width * 0.95 and height >= page_height * 0.35:
            continue
        rects.append((x0, y0, x1, y1))
    return rects


def _drawing_lines(
    drawings: list[dict[str, Any]], tolerance: float = 1.0
) -> tuple[list[tuple[float, float, float]], list[tuple[float, float, float]]]:
    horizontal: list[tuple[float, float, float]] = []
    vertical: list[tuple[float, float, float]] = []

    for drawing in drawings:
        for item in drawing.get("items", []):
            if item[0] == "l":
                start, end = item[1], item[2]
                if abs(start.y - end.y) <= tolerance and abs(start.x - end.x) > 2:
                    horizontal.append(
                        (min(start.x, end.x), max(start.x, end.x), start.y)
                    )
                elif abs(start.x - end.x) <= tolerance and abs(start.y - end.y) > 2:
                    vertical.append(
                        (min(start.y, end.y), max(start.y, end.y), start.x)
                    )
            elif item[0] == "re":
                rect = item[1]
                horizontal.append((rect.x0, rect.x1, rect.y0))
                horizontal.append((rect.x0, rect.x1, rect.y1))
                vertical.append((rect.y0, rect.y1, rect.x0))
                vertical.append((rect.y0, rect.y1, rect.x1))

    return horizontal, vertical


def _merge_lines(
    lines: list[tuple[float, float, float]], tolerance: float = 2.0
) -> list[tuple[float, float, float]]:
    merged: list[tuple[float, float, float]] = []
    for start, end, position in sorted(lines, key=lambda line: (line[2], line[0])):
        if (
            merged
            and abs(merged[-1][2] - position) <= tolerance
            and start <= merged[-1][1] + tolerance
        ):
            previous_start, previous_end, previous_position = merged[-1]
            merged[-1] = (
                previous_start,
                max(previous_end, end),
                previous_position,
            )
        else:
            merged.append((start, end, position))
    return merged


def _line_clusters(
    horizontal: list[tuple[float, float, float]],
    vertical: list[tuple[float, float, float]],
) -> list[tuple[float, float, float, float, int, int]]:
    horizontal = _merge_lines(horizontal)
    vertical = _merge_lines(vertical)
    total = len(horizontal) + len(vertical)
    if total == 0:
        return []

    parent = list(range(total))

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    def union(left: int, right: int) -> None:
        left_root = find(left)
        right_root = find(right)
        if left_root != right_root:
            parent[right_root] = left_root

    for h_index, (x0, x1, y) in enumerate(horizontal):
        for v_index, (v_y0, v_y1, x) in enumerate(vertical):
            if x0 - 1 <= x <= x1 + 1 and v_y0 - 1 <= y <= v_y1 + 1:
                union(h_index, len(horizontal) + v_index)

    groups: dict[int, list[int]] = {}
    for index in range(total):
        groups.setdefault(find(index), []).append(index)

    boxes: list[tuple[float, float, float, float, int, int]] = []
    for indices in groups.values():
        group_h = [horizontal[i] for i in indices if i < len(horizontal)]
        group_v = [vertical[i - len(horizontal)] for i in indices if i >= len(horizontal)]
        if len(group_h) < 2 or len(group_v) < 2:
            continue
        x0 = min([line[0] for line in group_h] + [line[2] for line in group_v])
        x1 = max([line[1] for line in group_h] + [line[2] for line in group_v])
        y0 = min([line[2] for line in group_h] + [line[0] for line in group_v])
        y1 = max([line[2] for line in group_h] + [line[1] for line in group_v])
        if x1 - x0 < 40 or y1 - y0 < 20:
            continue
        boxes.append((x0, y0, x1, y1, len(group_h), len(group_v)))

    boxes.sort(key=lambda box: -((box[2] - box[0]) * (box[3] - box[1])))
    kept: list[tuple[float, float, float, float, int, int]] = []
    for box in boxes:
        if any(
            box[0] >= other[0] - 2
            and box[1] >= other[1] - 2
            and box[2] <= other[2] + 2
            and box[3] <= other[3] + 2
            for other in kept
        ):
            continue
        kept.append(box)
    return kept


def _raster_item(
    fitz,
    page,
    kind: str,
    bbox: tuple[float, float, float, float],
    page_number: int,
    page_width: float,
    page_height: float,
) -> dict[str, Any] | None:
    x0, y0, x1, y1 = bbox
    x0 = max(0.0, min(x0, page_width))
    x1 = max(0.0, min(x1, page_width))
    y0 = max(0.0, min(y0, page_height))
    y1 = max(0.0, min(y1, page_height))
    if x1 - x0 < MIN_RASTER_PIXELS or y1 - y0 < MIN_RASTER_PIXELS:
        return None

    try:
        clip = fitz.Rect(x0, y0, x1, y1)
        pixmap = page.get_pixmap(
            matrix=fitz.Matrix(RASTER_ZOOM, RASTER_ZOOM), clip=clip, alpha=False
        )
        if kind == "image":
            payload = pixmap.tobytes("jpeg", jpg_quality=75)
            mime = "jpeg"
        else:
            payload = pixmap.tobytes("png")
            mime = "png"
    except Exception:  # pragma: no cover - rasterization best effort
        return None

    encoded = base64.b64encode(payload).decode("ascii")
    return {
        "type": kind,
        "page": page_number,
        "bbox": [x0, y0, x1, y1],
        "image": f"data:image/{mime};base64,{encoded}",
    }


def _order_items(
    items: list[dict[str, Any]], page_width: float, engine: str
) -> list[dict[str, Any]]:
    if engine != "academic":
        return sorted(items, key=lambda item: (item["bbox"][1], item["bbox"][0]))

    middle = page_width / 2.0
    full_width = [
        item for item in items if item["bbox"][2] - item["bbox"][0] >= page_width * 0.55
    ]
    narrow = [item for item in items if item not in full_width]

    ordered: list[dict[str, Any]] = []
    remaining = list(narrow)
    for separator in sorted(full_width, key=lambda item: item["bbox"][1]):
        band = [item for item in remaining if item["bbox"][1] < separator["bbox"][1]]
        remaining = [item for item in remaining if item["bbox"][1] >= separator["bbox"][1]]
        ordered.extend(_column_order(band, middle))
        ordered.append(separator)
    ordered.extend(_column_order(remaining, middle))
    return ordered


def _column_order(
    items: list[dict[str, Any]], middle: float
) -> list[dict[str, Any]]:
    left = [
        item
        for item in items
        if (item["bbox"][0] + item["bbox"][2]) / 2.0 < middle
    ]
    right = [
        item
        for item in items
        if (item["bbox"][0] + item["bbox"][2]) / 2.0 >= middle
    ]
    key = lambda item: (item["bbox"][1], item["bbox"][0])  # noqa: E731
    return sorted(left, key=key) + sorted(right, key=key)


def _bbox(value: Any) -> tuple[float, float, float, float] | None:
    if value is None or len(value) != 4:
        return None
    try:
        x0, y0, x1, y1 = (float(part) for part in value)
    except (TypeError, ValueError):
        return None
    if x1 <= x0 or y1 <= y0:
        return None
    return (x0, y0, x1, y1)


def _overlap_ratio(
    inner: tuple[float, float, float, float],
    outer: tuple[float, float, float, float],
) -> float:
    x0 = max(inner[0], outer[0])
    y0 = max(inner[1], outer[1])
    x1 = min(inner[2], outer[2])
    y1 = min(inner[3], outer[3])
    if x1 <= x0 or y1 <= y0:
        return 0.0
    overlap = (x1 - x0) * (y1 - y0)
    area = (inner[2] - inner[0]) * (inner[3] - inner[1])
    return overlap / area if area > 0 else 0.0


def _normalize(text: str) -> str:
    return " ".join(text.split())


def probe_pdf(data: bytes, max_pages: int = 2) -> dict[str, Any]:
    """Return page count and a short text-layer probe without full extraction."""

    if len(data) > MAX_PDF_BYTES:
        raise ValueError(f"PDF exceeds the {MAX_PDF_BYTES} byte limit")
    if not data.startswith(b"%PDF"):
        raise ValueError("uploaded data is not a PDF")

    fitz = _import_fitz()
    document = fitz.open(stream=data, filetype="pdf")
    try:
        if document.page_count == 0:
            raise ValueError("PDF has no pages")
        pages = max(1, min(int(max_pages), document.page_count))
        parts: list[str] = []
        for page_index in range(pages):
            page = document.load_page(page_index)
            parts.append(page.get_text("text") or "")
        text = " ".join(" ".join(parts).split())
        return {
            "page_count": document.page_count,
            "text_chars": len(text),
            "has_text_layer": len(text) >= 100,
            "excerpt": text[:1500],
        }
    finally:
        document.close()
