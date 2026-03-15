# Shared Hooks Bundle

Shared git-hook plumbing lives here.

## Current Hook

- `pre-commit`
  - local governance hygiene hook
  - install with:
    - `git config core.hooksPath .GOV/roles_shared/scripts/hooks`

## Scope Rule

- hooks are shared repo workflow tooling, not role-owned logic
- hook code should remain lightweight and delegate substantial checks to governed `just` commands or shared/role checks
