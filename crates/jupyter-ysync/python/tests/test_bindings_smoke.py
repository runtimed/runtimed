"""Exercise the installed extension without optional conformance dependencies."""

import json
import unittest

from jupyter_ysync import NotebookDoc


class TestBindingsSmoke(unittest.TestCase):
    def test_document_api_and_update_roundtrip(self):
        source = NotebookDoc()
        target = NotebookDoc()

        self.assertEqual(source.cell_count(), 0)
        self.assertIsInstance(source.client_id(), int)
        self.assertNotEqual(source.client_id(), target.client_id())
        self.assertEqual(json.loads(source.to_json()), {"cells": [], "metadata": {}})

        update = source.get_update()
        state_vector = source.get_state_vector()
        self.assertIsInstance(update, bytes)
        self.assertIsInstance(state_vector, bytes)
        self.assertGreater(len(update), 0)
        self.assertGreater(len(state_vector), 0)

        self.assertIsNone(target.apply_update(update))
        self.assertEqual(json.loads(target.to_json()), json.loads(source.to_json()))
        self.assertEqual(target.get_state_vector(), state_vector)

    def test_invalid_update_is_a_python_error(self):
        doc = NotebookDoc()
        before = doc.to_json()
        with self.assertRaises(ValueError):
            doc.apply_update(b"")
        self.assertEqual(doc.to_json(), before)

    def test_update_requires_bytes(self):
        doc = NotebookDoc()
        with self.assertRaises(TypeError):
            doc.apply_update("not bytes")


if __name__ == "__main__":
    unittest.main()
