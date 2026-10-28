#!/usr/bin/env python3
"""Check that packaging the same binary at two different times gives the same archive."""
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import package_release


class PackageReleaseTest(unittest.TestCase):
    def test_archive_digest_is_reproducible(self):
        with tempfile.TemporaryDirectory() as scratch:
            binary = Path(scratch) / "ejm-cli"
            binary.write_bytes(b"binary")
            digests = []
            for run, now in (("first", 1_000_000_000), ("second", 2_000_000_000)):
                with mock.patch("time.time", return_value=now):
                    _, _, archive = package_release.package("linux-x86_64", binary, Path(scratch) / run)
                digests.append(hashlib.sha256(archive.read_bytes()).hexdigest())
            self.assertEqual(digests[0], digests[1])


if __name__ == "__main__":
    unittest.main()
