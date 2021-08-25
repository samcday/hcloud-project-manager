#!/usr/bin/env bash
set -ueo pipefail

HCLOUD_USER_TOKEN=$(hcloud-project-manager login)
echo "::add-mask::$HCLOUD_USER_TOKEN"
export HCLOUD_USER_TOKEN

if [[ "$2" == "create" ]]; then
  project_id=$(hcloud-project-manager create "$1")
  token=$(hcloud-project-manager token "$1")
  echo "::add-mask::$token"
  echo "::set-output name=project_id::$project_id"
  echo "::set-output name=token::$token"
  echo "HCLOUD_PROJECT_ID=$project_id" >> "$GITHUB_ENV"

  if [[ "$SET_TOKEN" == "true" ]]; then
   echo "HCLOUD_TOKEN=$token" >> "$GITHUB_ENV"
  fi
elif [[ "$2" == "delete" ]]; then
  hcloud-project-manager delete "$1"
fi
