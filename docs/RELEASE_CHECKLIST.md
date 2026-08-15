# Release Checklist

This document ensures consistent documentation updates for every release.

## Pre-Release Tasks

- [ ] All tests passing (`cargo test`)
- [ ] Clippy clean (`cargo clippy --message-format=short`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Release notes drafted in GitHub

## Release Tasks (Automated + Manual)

### GitHub Release
- [ ] Create tag (`git tag -a vX.Y.Z -m "Version X.Y.Z"`)
- [ ] Push tag (`git push origin vX.Y.Z`)
- [ ] Create release on GitHub with detailed notes

### Documentation Updates (Automated via Workflow)
The `.github/workflows/release.yml` will automatically:
- [ ] Update version badge in README
- [ ] Update test count badge in README
- [ ] Create PR with documentation changes

### Manual Documentation Updates Required

#### 1. CHANGELOG.md
- [ ] Add new version section at the top
- [ ] List all features added
- [ ] List all bug fixes
- [ ] List breaking changes
- [ ] List new dependencies

#### 2. .learnings/history.md
- [ ] Add entry for the release
- [ ] Document what was built
- [ ] Document key decisions
- [ ] List files created/modified
- [ ] Note test count
- [ ] Add link to GitHub release

#### 3. .learnings/learnings.md
- [ ] Add any new patterns discovered
- [ ] Add any bugs found and fixed
- [ ] Add any gotchas encountered
- [ ] Add any architectural decisions

#### 4. README.md (if needed)
- [ ] Update Features list if new features added
- [ ] Update Architecture diagram if modules changed
- [ ] Update Tech Stack if dependencies changed
- [ ] Update Testing section if test categories changed
- [ ] Add new CLI examples if commands added
- [ ] Add new configuration options

#### 5. AGENTS.md (if workflow changes)
- [ ] Update workflow instructions if process changed
- [ ] Add new patterns to follow
- [ ] Update subagent recommendations

## Post-Release Verification

- [ ] Verify release is on GitHub with correct tag
- [ ] Verify release notes are complete
- [ ] Verify documentation PR was created (if workflow enabled)
- [ ] Verify README badges are updated
- [ ] Verify CHANGELOG.md is updated
- [ ] Verify learnings files are updated
- [ ] Verify cargo install works (`cargo install joshify`)
- [ ] Verify release assets are attached (if applicable)

## Rollback Plan (if needed)

If a release has critical issues:

1. Document the issue in `.learnings/learnings.md`
2. Create a fix PR
3. Release vX.Y.Z+1 with the fix
4. Update GitHub release notes to mention the issue

---

## Template for GitHub Release Notes

```markdown
## What's New

### Features
- Feature 1 description
- Feature 2 description

### Bug Fixes
- Fix 1 description
- Fix 2 description

### Breaking Changes
- Breaking change 1 (if any)

### Dependencies
- Added `crate = "version"`
- Updated `crate` to "version"

## Installation

```bash
cargo install joshify
```

Or download the binary from the release assets.

## Test Count: XXX tests passing
```

---

**Last Updated**: This checklist should be reviewed and updated as part of each release.
