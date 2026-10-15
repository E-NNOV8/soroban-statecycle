"""Infrastructure classification tests, not protocol/archival simulation proof."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock
from urllib.error import HTTPError
from urllib.request import Request

from archival_regression.preflight import (
    PASS, INCONCLUSIVE, NOT_TESTED, run_probe, usable_backend,
    local_rpc_health, summarize, NoRedirect,
)


class PreflightBehavior(unittest.TestCase):
    def test_real_successful_process_is_not_campaign_pass(self):
        result = run_probe("example", [sys.executable, "-c", "print('available')"], cwd=".")
        self.assertEqual(result["result"], PASS)
        self.assertEqual(summarize([result])["stage1_result"], INCONCLUSIVE)

    def test_real_nonzero_process_is_inconclusive_not_contract_fail(self):
        result = run_probe("docker", [sys.executable, "-c", "print('success'); exit(1)"], cwd=".")
        self.assertEqual(result["result"], INCONCLUSIVE)
        self.assertFalse(usable_backend(result))

    def test_missing_program_preserves_error(self):
        result = run_probe("native-core", ["/nonexistent-archival-regression-core"], cwd=".")
        self.assertEqual(result["result"], INCONCLUSIVE)
        self.assertIsNone(result["exit_code"])
        self.assertIn("FileNotFoundError", result["error"])

    def test_timeout_never_counts_as_expected_negative(self):
        with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(["docker"], 15)):
            result = run_probe("docker", ["docker"], cwd=".")
        self.assertEqual(result["result"], INCONCLUSIVE)
        self.assertEqual(summarize([result])["campaign"]["negative"], NOT_TESTED)

    def test_successful_empty_or_malformed_backend_output_is_rejected(self):
        for text in ("", "null", '""', "{}", "not json", "123"):
            with self.subTest(text=text):
                self.assertFalse(usable_backend({"id": "docker", "result": PASS,
                                                "exit_code": 0, "stdout": text}))

    def test_missing_and_wrong_type_backend_fields_are_rejected(self):
        for probe in ({}, {"id": "docker", "result": PASS, "exit_code": 0},
                      {"id": "native-core", "result": PASS, "exit_code": 0, "stdout": None},
                      {"id": "podman", "result": PASS, "exit_code": 0, "stdout": '{"host":null}'}):
            self.assertFalse(usable_backend(probe))

    def test_available_backend_still_never_certifies_campaign(self):
        probes = [{"id": "docker", "result": PASS, "exit_code": 0, "stdout": '"29.0"'},
                  {"id": "stellar", "result": PASS, "exit_code": 0, "stdout": "stellar 28.1"}]
        report = summarize(probes)
        self.assertEqual(report["stage1_result"], NOT_TESTED)
        self.assertEqual(report["fresh_state_campaign_runs"], 0)
        self.assertTrue(all(value == NOT_TESTED for value in report["campaign"].values()))

    def test_successful_retry_of_setup_is_not_recovery_proof(self):
        for code, result in ((1, INCONCLUSIVE), (0, PASS)):
            report = summarize([{"id": "native-core", "result": result,
                                 "exit_code": code, "stdout": "v29"}])
            self.assertEqual(report["campaign"]["retry"], NOT_TESTED)
            self.assertNotEqual(report["stage1_result"], PASS)

    def test_empty_probe_set_fails_closed(self):
        self.assertEqual(summarize([])["stage1_result"], INCONCLUSIVE)

    def test_rpc_health_does_not_bypass_unavailable_runtime(self):
        report = summarize([{"id": "local-rpc", "result": PASS},
                            {"id": "stellar", "result": PASS, "exit_code": 0, "stdout": "stellar"}])
        self.assertEqual(report["stage1_result"], INCONCLUSIVE)

    def test_public_and_credential_bearing_urls_are_rejected_before_network(self):
        for url in ("https://soroban-testnet.stellar.org", "http://user:pass@localhost:8000",
                    "http://127.0.0.1:8000/?token=secret", "file:///tmp/not-rpc"):
            with patch("archival_regression.preflight.build_opener") as opener:
                with self.assertRaises(ValueError):
                    local_rpc_health(url)
                opener.assert_not_called()

    def test_local_rpc_redirect_is_refused(self):
        with self.assertRaises(HTTPError) as caught:
            NoRedirect().redirect_request(Request("http://127.0.0.1:8000"),
                                          None, 302, "Found", {},
                                          "https://public-network.example")
        caught.exception.close()

    def test_rpc_malformed_missing_and_error_responses_fail_closed(self):
        for body in (b"not json", b"null", b"{}", b'{"result":{"status":"healthy"}}',
                     b'{"jsonrpc":"2.0","id":1,"error":{},"result":{"status":"healthy"}}'):
            response = MagicMock()
            response.__enter__.return_value = response
            response.read.return_value = body
            response.status = 200
            with patch("archival_regression.preflight.build_opener") as opener:
                opener.return_value.open.return_value = response
                self.assertEqual(local_rpc_health("http://127.0.0.1:8000")["result"], INCONCLUSIVE)

    def test_health_response_is_only_health_not_archival(self):
        response = MagicMock()
        response.__enter__.return_value = response
        response.status = 200
        response.read.return_value = b'{"jsonrpc":"2.0","id":1,"result":{"status":"healthy"}}'
        with patch("archival_regression.preflight.build_opener") as opener:
            opener.return_value.open.return_value = response
            result = local_rpc_health("http://127.0.0.1:8000")
        self.assertEqual(result["result"], PASS)
        self.assertEqual(summarize([result])["campaign"]["archival"], NOT_TESTED)

    def test_cli_retains_nonzero_exit_and_refuses_overwriting_evidence(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "evidence"
            with patch("archival_regression.preflight.collect", return_value=[]):
                from archival_regression.preflight import main
                self.assertEqual(main(["--output", str(output)]), 2)
                self.assertEqual(json.loads((output / "report.json").read_text())["stage1_result"], INCONCLUSIVE)
                with self.assertRaises(FileExistsError):
                    main(["--output", str(output)])


if __name__ == "__main__":
    unittest.main()
