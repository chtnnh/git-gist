# Deprecation policy

Until **1.0**, flags and reserved command names may change with CHANGELOG notes.

From **1.0**:

- Reserved builtin names are stable; removals require a major version
- Config `schema_version` migrations must be backward compatible or documented
- JSON field renames require a major version or dual-write period
- Announce deprecations for at least one minor release before removal when practical

Escape hatch `gg git -- …` remains stable for passthrough collisions.
