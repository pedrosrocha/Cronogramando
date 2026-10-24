import unittest

from app.extraction import blocks_to_markdown, select_engine


class EngineSelectionTests(unittest.TestCase):
    def test_academic_connectors_use_academic_engine(self):
        for connector in ("arxiv", "DOAJ", "OpenAlex"):
            self.assertEqual(select_engine(connector), "academic")

    def test_other_connectors_use_normal_engine(self):
        for connector in ("openstax", "gutenberg", "internet_archive", ""):
            self.assertEqual(select_engine(connector), "normal")


class MarkdownTests(unittest.TestCase):
    def test_renders_every_block_type(self):
        blocks = [
            {"type": "heading", "level": 1, "text": "Calculus"},
            {"type": "paragraph", "text": "Limits and continuity."},
            {"type": "image", "image": "data:image/png;base64,aaa"},
            {"type": "table_image", "image": "data:image/png;base64,bbb"},
            {"type": "formula_image", "image": "data:image/png;base64,ccc"},
        ]

        self.assertEqual(
            blocks_to_markdown(blocks),
            "# Calculus\n\n"
            "Limits and continuity.\n\n"
            "![image](data:image/png;base64,aaa)\n\n"
            "![table](data:image/png;base64,bbb)\n\n"
            "![formula](data:image/png;base64,ccc)",
        )

    def test_heading_level_is_clamped(self):
        self.assertEqual(
            blocks_to_markdown([{"type": "heading", "level": 99, "text": "Deep"}]),
            "###### Deep",
        )

    def test_empty_text_is_skipped(self):
        self.assertEqual(
            blocks_to_markdown(
                [
                    {"type": "heading", "level": 1, "text": " "},
                    {"type": "paragraph", "text": ""},
                ]
            ),
            "",
        )


class TableDetectionTests(unittest.TestCase):
    def test_auto_mode_uses_accurate_finder_for_short_documents(self):
        from app.extraction import _table_mode

        self.assertEqual(_table_mode(10), "pymupdf")

    def test_auto_mode_uses_fast_lines_for_long_documents(self):
        from app.extraction import _table_mode

        self.assertEqual(_table_mode(500), "lines")

    def test_merges_collinear_segments(self):
        from app.extraction import _merge_lines

        merged = _merge_lines([(0, 10, 5), (10, 20, 5), (0, 10, 40)])
        self.assertEqual(merged, [(0, 20, 5), (0, 10, 40)])

    def test_clusters_a_ruled_grid(self):
        from app.extraction import _line_clusters

        horizontal = [(0, 100, 0), (0, 100, 50)]
        vertical = [(0, 50, 0), (0, 50, 100)]
        boxes = _line_clusters(horizontal, vertical)

        self.assertEqual(len(boxes), 1)
        x0, y0, x1, y1, h_count, v_count = boxes[0]
        self.assertEqual((x0, y0, x1, y1), (0, 0, 100, 50))
        self.assertEqual((h_count, v_count), (2, 2))

    def test_ignores_lines_without_a_grid(self):
        from app.extraction import _line_clusters

        self.assertEqual(_line_clusters([(0, 100, 0)], [(0, 50, 0)]), [])


class ReadingOrderTests(unittest.TestCase):
    def test_academic_engine_reads_left_column_before_right(self):
        from app.extraction import _order_items

        items = [
            {"type": "paragraph", "text": "right", "bbox": [300, 10, 500, 20]},
            {"type": "paragraph", "text": "left", "bbox": [50, 10, 250, 20]},
        ]

        ordered = _order_items(items, page_width=550, engine="academic")
        self.assertEqual([item["text"] for item in ordered], ["left", "right"])

    def test_normal_engine_reads_top_to_bottom(self):
        from app.extraction import _order_items

        items = [
            {"type": "paragraph", "text": "bottom", "bbox": [50, 100, 250, 120]},
            {"type": "paragraph", "text": "top", "bbox": [300, 10, 500, 20]},
        ]

        ordered = _order_items(items, page_width=550, engine="normal")
        self.assertEqual([item["text"] for item in ordered], ["top", "bottom"])

    def test_academic_engine_keeps_full_width_separator_in_order(self):
        from app.extraction import _order_items

        items = [
            {"type": "paragraph", "text": "right", "bbox": [300, 10, 500, 20]},
            {"type": "paragraph", "text": "left", "bbox": [50, 10, 250, 20]},
            {"type": "heading", "text": "section", "bbox": [20, 200, 530, 230]},
            {"type": "paragraph", "text": "after", "bbox": [50, 300, 250, 320]},
        ]

        ordered = _order_items(items, page_width=550, engine="academic")
        self.assertEqual(
            [item["text"] for item in ordered], ["left", "right", "section", "after"]
        )


if __name__ == "__main__":
    unittest.main()
