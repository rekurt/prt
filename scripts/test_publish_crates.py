import importlib.util
import json
import unittest
import urllib.error
from unittest.mock import MagicMock, patch

spec = importlib.util.spec_from_file_location("publish", "scripts/publish-crates.py")
publish = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publish)


class PublishTests(unittest.TestCase):
    def test_index_access_error_is_not_absence(self):
        error = urllib.error.HTTPError("url", 403, "Forbidden", {}, None)
        with patch.object(publish.urllib.request, "urlopen", side_effect=error):
            with self.assertRaises(urllib.error.HTTPError):
                publish.indexed("prt", "0.6.0")

    def test_yanked_version_is_not_available(self):
        response = MagicMock()
        response.__enter__.return_value.read.return_value = b'{"vers":"0.6.0","yanked":true}\n'
        with patch.object(publish.urllib.request, "urlopen", return_value=response):
            self.assertFalse(publish.indexed("prt", "0.6.0"))

    def test_publish_failure_stops_before_dependent_crate(self):
        metadata = json.dumps({"packages": [{"name": name, "version": "0.6.0"} for name in ("prt", "prt-core")]})
        with patch.object(publish.subprocess, "check_output", return_value=metadata), \
             patch.object(publish, "indexed", return_value=False), \
             patch.object(publish.subprocess, "run", side_effect=publish.subprocess.CalledProcessError(1, "cargo")) as run:
            with self.assertRaises(publish.subprocess.CalledProcessError):
                publish.main()
            run.assert_called_once_with(["cargo", "publish", "--locked", "-p", "prt-core"], check=True)

    def test_retry_skips_core_and_publishes_binary_only_after_indexing(self):
        metadata = json.dumps({"packages": [{"name": name, "version": "0.6.0"} for name in ("prt", "prt-core")]})
        with patch.object(publish.subprocess, "check_output", return_value=metadata), \
             patch.object(publish, "indexed", side_effect=[True, True, False, False, True]), \
             patch.object(publish.time, "sleep"), \
             patch.object(publish.subprocess, "run") as run:
            publish.main()
            run.assert_called_once_with(["cargo", "publish", "--locked", "-p", "prt"], check=True)


if __name__ == "__main__":
    unittest.main()
