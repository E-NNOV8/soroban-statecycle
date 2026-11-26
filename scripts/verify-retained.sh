#!/usr/bin/env bash
set -euo pipefail

echo "Verifying retained Core/Blend archival regression proof..."
PYTHONPATH=src python3 -c "
from pathlib import Path
from archival_regression.campaign import verify_retained_evidence_bundle
res = verify_retained_evidence_bundle(Path('docs/experiments/stage1/host-resume-20261015'))
print(res)
assert res['status'] == 'PASS'
"
PYTHONPATH=src python3 -m unittest discover tests
