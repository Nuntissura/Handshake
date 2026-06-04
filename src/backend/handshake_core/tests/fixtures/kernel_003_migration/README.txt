MT-055 KERNEL-003 sandbox migration fixtures.

These JSON fixtures snapshot representative KERNEL-003 validation jobs and
their adapter-neutral ProcessSpec translations. Tests run the same fixtures
through trait-object SandboxAdapter test doubles and compare Docker/Podman
argv generation without requiring live Docker or WSL2 backends by default.
