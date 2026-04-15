#!/usr/bin/env bash
# Bootstrap (or reuse) a throwaway GitHub Projects V2 board for recording the
# gh-board demo GIF. Idempotent: safe to run repeatedly.
#
# Usage:
#   ./.vhs/setup.sh                    # uses @me as the owner
#   GH_BOARD_DEMO_OWNER=my-org ./.vhs/setup.sh
#
# Prints the project number to stdout and the human-readable url to stderr so
# you can pipe into an env var:
#   export GH_BOARD_DEMO_PROJECT=$(./.vhs/setup.sh)

set -euo pipefail

OWNER="${GH_BOARD_DEMO_OWNER:-@me}"
TITLE="${GH_BOARD_DEMO_TITLE:-gh-board demo}"

log() { printf '%s\n' "$*" >&2; }

require() {
  command -v "$1" >/dev/null 2>&1 || { log "missing required command: $1"; exit 1; }
}

require gh
require jq

# --- find or create the project ----------------------------------------------
log "looking up project '$TITLE' under $OWNER"
number=$(gh project list --owner "$OWNER" --format json \
  | jq -r --arg t "$TITLE" '.projects[] | select(.title == $t) | .number' \
  | head -n1)

if [[ -z "${number}" ]]; then
  log "creating new project '$TITLE'"
  number=$(gh project create --owner "$OWNER" --title "$TITLE" --format json | jq -r '.number')
else
  log "reusing existing project #$number"
fi

project_id=$(gh project view "$number" --owner "$OWNER" --format json | jq -r '.id')

# --- fetch Status field metadata --------------------------------------------
fields_json=$(gh project field-list "$number" --owner "$OWNER" --format json)
status_field_id=$(jq -r '.fields[] | select(.name == "Status") | .id' <<<"$fields_json")
todo_opt=$(jq -r '.fields[] | select(.name == "Status") | .options[] | select(.name == "Todo") | .id' <<<"$fields_json")
in_progress_opt=$(jq -r '.fields[] | select(.name == "Status") | .options[] | select(.name == "In Progress") | .id' <<<"$fields_json")
done_opt=$(jq -r '.fields[] | select(.name == "Status") | .options[] | select(.name == "Done") | .id' <<<"$fields_json")

# --- seed draft issues (only if the board is empty) --------------------------
item_count=$(gh project item-list "$number" --owner "$OWNER" --format json | jq '.items | length')

if [[ "$item_count" -eq 0 ]]; then
  log "seeding draft items"

  add_item() {
    local title="$1" body="$2" status_opt="$3"
    local item_id
    item_id=$(gh project item-create "$number" --owner "$OWNER" \
      --title "$title" --body "$body" --format json | jq -r '.id')
    gh project item-edit --id "$item_id" \
      --project-id "$project_id" \
      --field-id "$status_field_id" \
      --single-select-option-id "$status_opt" >/dev/null
  }

  add_item "Setup CI pipeline"         "Configure GitHub Actions for build and test."          "$todo_opt"
  add_item "Write user guide"          "Document installation and common workflows."            "$todo_opt"
  add_item "Investigate flaky tests"   "Integration tests intermittently fail on macOS."        "$todo_opt"
  add_item "Implement OAuth login"     "Add GitHub OAuth login flow with PKCE."                 "$in_progress_opt"
  add_item "Refactor error handling"   "Introduce a unified error type with anyhow + thiserror." "$in_progress_opt"
  add_item "Ship v0.1.0 release"       "Publish first tagged release with changelog."            "$done_opt"
  add_item "Draft project README"      "Added installation, usage, and key bindings."            "$done_opt"
else
  log "board already has $item_count items, skipping seed"
fi

printf '%s\n' "$number"
