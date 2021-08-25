#!/usr/bin/env bash

set -ueo pipefail

if [[ "${EPHEMERAL:-}" == "true" ]]; then
  export HCLOUD_USER_TOKEN=$(hcloud-project-manager login)
  hcloud-project-manager delete $1
fi
