"""tools/parity.py manifest_lines: a '#' inside a quoted TOML value is not a comment."""
import unittest

from tools import parity


class ManifestLinesTests(unittest.TestCase):
    def test_hash_inside_quoted_value_is_kept(self):
        text = '[dependencies]\nfoo = { git = "https://example.com/repo#branch", version = "1" }\n'
        self.assertEqual(
            parity.manifest_lines(text),
            {("[dependencies]", 'foo = { git = "https://example.com/repo#branch", version = "1" }')},
        )

    def test_comment_after_quoted_hash_is_dropped(self):
        text = '[dependencies]\nfoo = "a#b" # trailing comment\n# whole line\n'
        self.assertEqual(parity.manifest_lines(text), {("[dependencies]", 'foo = "a#b"')})

    def test_hash_inside_single_quoted_value_is_kept(self):
        text = "[dependencies]\nfoo = 'a#b'\n"
        self.assertEqual(parity.manifest_lines(text), {("[dependencies]", "foo = 'a#b'")})

    def test_escaped_quote_does_not_end_the_value(self):
        text = '[dependencies]\nfoo = "a\\"#b" # note\n'
        self.assertEqual(parity.manifest_lines(text), {("[dependencies]", 'foo = "a\\"#b"')})


if __name__ == "__main__":
    unittest.main()
