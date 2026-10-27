import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package_locks", Path(__file__).with_name("check-package-locks.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ArchiveLockTests(unittest.TestCase):
    def check(self, contents, reviewed):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "example-1.0.0.crate"
            with tarfile.open(path, "w:gz") as archive:
                data = contents.encode()
                member = tarfile.TarInfo("example-1.0.0/Cargo.lock")
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
            return module.check_archive(path, reviewed)

    def test_preserves_reviewed_registry_version_and_checksum(self):
        package = {"name": "h2", "version": "0.4.20", "source": "registry+example", "checksum": "approved"}
        lock = '\n'.join(['[[package]]'] + [f'{key} = "{value}"' for key, value in package.items()])
        self.assertEqual(self.check(lock, [package]), [])
        self.assertTrue(self.check(lock.replace('0.4.20', '0.4.13'), [package]))
        self.assertTrue(self.check(lock.replace('approved', 'different'), [package]))
        self.assertTrue(self.check(lock.replace('registry+example', 'git+example'), [package]))
        self.assertTrue(self.check(lock, []))

    def test_workspace_dependency_can_acquire_registry_identity(self):
        lock = '[[package]]\nname = "jupyter-protocol"\nversion = "2.0.3"\nsource = "registry+example"\nchecksum = "newly-published"'
        self.assertEqual(self.check(lock, [{"name": "jupyter-protocol", "version": "2.0.3"}]), [])
        self.assertTrue(self.check(lock, [{"name": "jupyter-protocol", "version": "2.0.2"}]))

    def test_missing_lockfile_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "example-1.0.0.crate"
            with tarfile.open(path, "w:gz"):
                pass
            with self.assertRaises(KeyError):
                module.check_archive(path, [])


if __name__ == "__main__":
    unittest.main()
