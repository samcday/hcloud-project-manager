#!/usr/bin/env bash
set -ueo pipefail

if [[ "${EPHEMERAL:-}" == "true" ]]; then
  HCLOUD_USER_TOKEN=$(hcloud-project-manager login)
  export HCLOUD_USER_TOKEN
  hcloud-project-manager delete "$1"
  echo deleted project "$1"
fi
