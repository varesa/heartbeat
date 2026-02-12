#!/usr/bin/env bash
set -euo pipefail

cargo lambda build \
  --release \
  --arm64 \
  --output-format zip \
  --package heartbeat-checker

echo "Build complete: target/lambda/heartbeat-checker/bootstrap.zip"
