#!/usr/bin/env bash

set -ueo pipefail

if [[ "${EPHEMERAL:-}" == "true" ]]; then
  echo deleting $1
fi
