#!/usr/bin/env python3
"""Check that packaging the same binary twice gives the same archive."""
import hashlib
import tempfile
import unittest
from pathlib import Path

import package_release


class PackageReleaseTest(unittest.TestCase):
    def test_archive_digest_is_reproducible(self):
        with tempfile.TemporaryDirectory() as scratch:
            binary = Path(scratch) / "ejm-cli"
            binary.write_bytes(b"binary")
            digests = []
            for run in ("first", "second"):
                _, _, archive = package_release.package("linux-x86_64", binary, Path(scratch) / run)
                digests.append(hashlib.sha256(archive.read_bytes()).hexdigest())
            self.assertEqual(digests[0], digests[1])


if __name__ == "__main__":
    unittest.main()
