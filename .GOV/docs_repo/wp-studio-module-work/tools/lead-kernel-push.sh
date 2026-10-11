#!/usr/bin/env bash
# Lead helper: commit the given kernel paths (message file $1, paths after), then integrate and push gov_kernel.
# Never branches, never forces. If origin/gov_kernel moved: merge only when no path overlaps, under the Studio gameplan.
set -uo pipefail
K="D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"
MSG="$1"; shift
cd "$K" || exit 1
if [ "$#" -gt 0 ]; then
  git add -- "$@" 2>/dev/null
  git commit -q -F "$MSG" -- "$@" 2>&1 | grep -v "will be replaced" || true
fi
git fetch -q origin gov_kernel || { echo "PUSH: fetch failed"; exit 2; }
read -r BEHIND AHEAD < <(git rev-list --left-right --count origin/gov_kernel...HEAD)
if [ "$BEHIND" -gt 0 ]; then
  MB=$(git merge-base HEAD origin/gov_kernel)
  OVER=$(comm -12 <(git diff --name-only "$MB" origin/gov_kernel | sort) <(git diff --name-only "$MB" HEAD | sort) | wc -l)
  if [ "$OVER" -ne 0 ]; then echo "PUSH: STOP overlap=$OVER paths with origin/gov_kernel; resolve manually"; exit 3; fi
  echo "PUSH: integrating $BEHIND remote commits (overlap 0)"
  printf 'Merge origin/gov_kernel into gov_kernel (Studio push; overlap 0 vs merge-base %s)\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n' "${MB:0:8}" > "$MSG.merge"
  (cd .GOV/docs_repo/wp-studio-module-work && GAMEPLAN_ROLE=orchestrator git merge --no-ff -q -F "$MSG.merge" origin/gov_kernel) || { echo "PUSH: merge failed"; exit 4; }
fi
git push -q origin gov_kernel 2>&1 | tail -1
git fetch -q origin gov_kernel
echo "PUSH: $(git rev-list --left-right --count origin/gov_kernel...HEAD | tr '\t' '/') (behind/ahead) head $(git rev-parse --short HEAD)"
