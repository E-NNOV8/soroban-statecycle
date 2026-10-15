#!/usr/bin/env bash
set -euo pipefail
export PYTHONPATH=src
python3 -W error::ResourceWarning -m unittest discover -s tests -v
python3 -m compileall -q src tests scripts
