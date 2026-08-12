# Local Git hooks for this repo.
#
# Enable once per clone:
#
#   git config core.hooksPath .githooks
#
# `pre-push` rejects `vX.Y.Z` tag pushes unless that version is frozen under
# `website/versions.json` (see `./scripts/docs-version.sh`). Git has no
# `pre-tag` hook; freeze + commit before tagging.
