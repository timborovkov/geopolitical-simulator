# Contributing

This repository currently contains documents only. There is no application toolchain, test suite or CI workflow yet.

Keep requirements in `docs/implementation-order.md` synchronized with `docs/implementation-register.csv`. Preserve stable IDs and update the requirement count when adding or removing requirements. Keep source citations and evidence limitations explicit.

Before committing, check local Markdown links, CSV structure, requirement IDs/counts, `git diff --check`, and `git status --short`. Review staged changes for unintended files or credentials. Application tests and builds become relevant when executable code is introduced.

Use descriptive commits. For ongoing work, use focused branches and pull requests when requested or appropriate; do not rewrite shared history without authorization. After an authorized push, verify the remote commit and inspect any configured CI checks. There is no deployment or release process yet.
