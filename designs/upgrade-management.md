# Toolshed Install, Uninstall, Upgrade, and Downgrade

## First time install

```
curl -f https://kernelle.io/install.sh
```

- Does nothing if kernelle is already installed. Does not re-install

## Manual uninstall

```
~/.kernelle/uninstall.sh
```

- Does not destroy settings/source files
- Does not destroy custom rules
- Removes CLIs

## Upgrade/Downgrade

```
kernelle upgrade [version] [--latest]
```

- if up to date, does nothing
- if version is specified, uses the given version (to allow manual downgrade). This is a semantic version tag
- if --latest is specified, uses the latest commit from kernelle's main branch
- if --latest AND a version is specified, warns the user that the two are mutually exclusive and does nothing.
- otherwise, upgrades to the latest semver tagged version.

Considerations:
- Does not destroy settings/source files
- Does not destroy custom rules
- Does not touch keeper keys
- Pulls and rebuilds kernelle's tools/MCPs from scratch
