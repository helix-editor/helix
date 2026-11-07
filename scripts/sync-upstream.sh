#!/usr/bin/env bash
# Sync this fork with upstream helix-editor/helix.
#
# Branch model
#   master  pure mirror of upstream/master; never commit here
#   local   your custom commits, rebased onto master; your daily driver
#
# Usage: scripts/sync-upstream.sh [--no-build]
#
# Exit status
#   0  synced (or already up to date)
#   1  rebase conflict; local left untouched
#   2  check failed; local left untouched
#   3  unsafe state (dirty tree, diverged mirror, missing remote)
set -euo pipefail

MIRROR=master
FORK=local
REMOTE=upstream
CHECK_CMD="HELIX_DISABLE_AUTO_GRAMMAR_BUILD=1 cargo check -p helix-term --quiet"

log() { printf '[sync-upstream] %s\n' "$*"; }
die() {
    printf '[sync-upstream] error: %s\n' "$1" >&2
    exit "${2:-3}"
}

build=true
case "${1:-}" in
    --no-build) build=false ;;
    "") ;;
    *) die "unknown argument: $1" ;;
esac

cd "$(git rev-parse --show-toplevel)"

git rev-parse --verify -q "refs/heads/$MIRROR" >/dev/null || die "missing branch $MIRROR"
git rev-parse --verify -q "refs/heads/$FORK" >/dev/null || die "missing branch $FORK"
git rev-parse --verify -q "refs/remotes/$REMOTE/$MIRROR" >/dev/null ||
    die "no '$REMOTE/$MIRROR'; run: git remote add $REMOTE https://github.com/helix-editor/helix.git && git fetch $REMOTE"
git diff-index --quiet HEAD -- || die "working tree is dirty; commit or stash first"

git fetch --quiet --prune --no-tags "$REMOTE"
git fetch --quiet --prune origin

# --- mirror ---------------------------------------------------------------
git switch --quiet "$MIRROR"
if ! git merge-base --is-ancestor "$MIRROR" "$REMOTE/$MIRROR"; then
    die "$MIRROR has commits that are not in $REMOTE/$MIRROR; reset it with: git branch -f $MIRROR $REMOTE/$MIRROR"
fi
git merge --ff-only --quiet "$REMOTE/$MIRROR"
if [ "$(git rev-parse "$MIRROR")" != "$(git rev-parse "origin/$MIRROR" 2>/dev/null || true)" ]; then
    git push --quiet origin "$MIRROR"
    log "updated $MIRROR -> $(git rev-parse --short "$MIRROR")"
else
    log "$MIRROR already current"
fi

# --- patch branch ---------------------------------------------------------
git switch --quiet "$FORK"
before=$(git rev-parse HEAD)

if ! git rebase --quiet "$MIRROR"; then
    git rebase --abort || true
    die "rebase conflict. Resolve with: git switch $FORK && git rebase $MIRROR" 1
fi

after=$(git rev-parse HEAD)
if [ "$before" = "$after" ]; then
    log "$FORK already up to date"
    exit 0
fi

if $build; then
    log "running: $CHECK_CMD"
    if ! eval "$CHECK_CMD"; then
        git reset --hard --quiet "$before"
        die "check failed; reverted $FORK to $(git rev-parse --short "$before")" 2
    fi
fi

git push --quiet --force-with-lease origin "$FORK"
log "rebased $FORK onto $MIRROR; pushed $(git rev-parse --short "$after")"
