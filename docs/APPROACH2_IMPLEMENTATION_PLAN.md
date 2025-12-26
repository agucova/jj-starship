# Implementation Plan: Approach 2 (Exact Prefix Computation)

## Overview

This document outlines the implementation plan for adding Approach 2 (`repo.shortest_unique_change_id_prefix_len()`) as an optional alternative to the current fixed 4-character prefix. This provides users with a choice between ultra-fast performance (fixed prefix) and exact jj-cli parity (computed prefix).

## Goals

1. **Backward Compatible**: Default behavior remains the same (fixed 4-char prefix)
2. **User Configurable**: Add option to enable exact prefix computation
3. **Well Documented**: Clear documentation on the trade-offs
4. **Minimal Impact**: Small code changes, no breaking changes

## Implementation Phases

### Phase 1: Add Configuration Option

#### 1.1 Update Config Structure

**File**: `src/config.rs`

Add a new field to the `Config` struct:

```rust
pub struct Config {
    // ... existing fields ...
    
    /// Use exact shortest unique prefix (slower but matches jj-cli)
    pub exact_change_id_prefix: bool,
}
```

Update `Default` impl:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            exact_change_id_prefix: false, // Default to fast fixed prefix
        }
    }
}
```

#### 1.2 Add CLI Flag

**File**: `src/main.rs`

Add command-line argument:

```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields ...
    
    /// Use exact shortest unique prefix for change_id coloring (slower but matches jj-cli)
    #[arg(long, global = true)]
    exact_change_id_prefix: bool,
}
```

Update `Config::new()` to accept the new parameter:

```rust
impl Config {
    pub fn new(
        // ... existing parameters ...
        exact_change_id_prefix: bool,
    ) -> Self {
        Self {
            // ... existing fields ...
            exact_change_id_prefix: exact_change_id_prefix
                || env::var("JJ_STARSHIP_EXACT_CHANGE_ID_PREFIX").is_ok(),
        }
    }
}
```

Update the call site in `main()`:

```rust
let config = Config::new(
    // ... existing arguments ...
    cli.exact_change_id_prefix,
);
```

#### 1.3 Add Environment Variable Support

**Environment Variable**: `JJ_STARSHIP_EXACT_CHANGE_ID_PREFIX`

If set (to any value), enables exact prefix computation.

### Phase 2: Update JJ Collection Logic

#### 2.1 Modify `collect()` Function Signature

**File**: `src/jj.rs`

Update function to accept config flag:

```rust
pub fn collect(
    repo_root: &Path, 
    id_length: usize, 
    ancestor_depth: usize,
    exact_prefix: bool,  // NEW PARAMETER
) -> Result<JjInfo>
```

#### 2.2 Implement Conditional Prefix Logic

**File**: `src/jj.rs`

Replace the fixed prefix computation with conditional logic:

```rust
// Compute change_id prefix length for coloring
let change_id_prefix_len = if exact_prefix {
    // Approach 2: Use exact shortest unique prefix
    repo.shortest_unique_change_id_prefix_len(commit.change_id())
        .unwrap_or(id_length)  // Fallback to full length on error
        .min(change_id.len())
} else {
    // Approach 1: Fixed 4-character prefix (default, fastest)
    // This is much faster while still providing visual distinction
    4.min(change_id.len())
};
```

#### 2.3 Update Call Site

**File**: `src/main.rs`

Update the `run_prompt()` function to pass the config flag:

```rust
fn run_prompt(cwd: &Path, config: &Config) -> Option<String> {
    let result = detect::detect(cwd);

    match result.repo_type {
        RepoType::Jj | RepoType::JjColocated => {
            let repo_root = result.repo_root?;
            let info = jj::collect(
                &repo_root,
                config.id_length,
                config.ancestor_bookmark_depth,
                config.exact_change_id_prefix,  // Pass the flag
            ).ok()?;
            Some(output::format_jj(&info, config))
        }
        // ... rest unchanged ...
    }
}
```

### Phase 3: Documentation Updates

#### 3.1 Update README.md

**File**: `README.md`

Add to the CLI Options table:

```markdown
| `--exact-change-id-prefix` | Use exact shortest unique prefix for coloring (slower but matches jj-cli) |
```

Add to Environment Variables section:

```markdown
- `JJ_STARSHIP_EXACT_CHANGE_ID_PREFIX` - Enable exact prefix computation
```

Add a new "Performance vs Accuracy" section:

```markdown
## Performance vs Accuracy

By default, jj-starship uses a fixed 4-character prefix for change_id coloring, providing
optimal prompt performance (~50ns). For exact jj-cli behavior, use `--exact-change-id-prefix`:

```toml
[custom.jj]
command = "jj-starship --exact-change-id-prefix"
when = "jj-starship detect"
```

**Performance comparison:**
- Fixed 4-char prefix (default): ~50ns - Optimal for prompts
- Exact computation: ~400-500ns - Matches jj-cli behavior

The 7-10x performance difference is rarely noticeable in practice, but the fixed prefix
provides more consistent prompt responsiveness.
```

#### 3.2 Update BENCHMARK.md

**File**: `BENCHMARK.md`

Add a section on the implementation:

```markdown
## Implementation Status

As of version X.X.X, both approaches are available:

- **Default**: Fixed 4-character prefix (Approach 1)
- **Optional**: Exact prefix via `--exact-change-id-prefix` flag (Approach 2)

This allows users to choose between optimal performance and exact jj-cli parity.
```

### Phase 4: Testing

#### 4.1 Unit Tests

**File**: `src/jj.rs`

Add tests to verify both modes work correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_prefix_mode() {
        // Test that fixed prefix mode returns 4
        // (Would need test repository setup)
    }
    
    #[test]
    fn test_exact_prefix_mode() {
        // Test that exact mode computes actual prefix
        // (Would need test repository setup)
    }
}
```

#### 4.2 Integration Tests

Test the full flow with both flags:

1. Test default behavior (no flag) → fixed prefix
2. Test `--exact-change-id-prefix` flag → exact computation
3. Test `JJ_STARSHIP_EXACT_CHANGE_ID_PREFIX` env var → exact computation
4. Test that output formatting works correctly with both modes

#### 4.3 Performance Validation

Run the benchmark binary to verify performance characteristics:

```bash
# Verify fixed prefix mode (default)
cargo run --release -- /path/to/jj/repo

# Verify exact mode performance
cargo run --release -- --exact-change-id-prefix /path/to/jj/repo

# Run benchmark to confirm no regression
cargo run --release --bin prefix_benchmark /path/to/jj/repo
```

### Phase 5: Release Preparation

#### 5.1 Update Changelog

Add entry describing the new feature:

```markdown
### Added
- Optional `--exact-change-id-prefix` flag for exact shortest unique prefix computation
- `JJ_STARSHIP_EXACT_CHANGE_ID_PREFIX` environment variable support
- Documentation comparing performance vs accuracy trade-offs
```

#### 5.2 Migration Guide

No migration needed - feature is opt-in and backward compatible.

#### 5.3 Version Bump

Follow semantic versioning:
- Minor version bump (e.g., 0.3.0 → 0.4.0) as this adds new functionality

## File Changes Summary

| File | Changes | Lines Changed (approx) |
|------|---------|------------------------|
| `src/config.rs` | Add `exact_change_id_prefix` field | +5 |
| `src/main.rs` | Add CLI flag, pass to collect() | +10 |
| `src/jj.rs` | Add parameter, conditional logic | +15 |
| `README.md` | Document new option and trade-offs | +30 |
| `BENCHMARK.md` | Document implementation status | +10 |

**Total estimated changes**: ~70 lines

## Risk Assessment

### Low Risk
- ✅ Backward compatible (default unchanged)
- ✅ Simple conditional logic
- ✅ Well-understood performance characteristics
- ✅ Opt-in feature

### Mitigation Strategies

1. **Performance regression**: Ensure default path unchanged, add benchmark tests
2. **User confusion**: Clear documentation on trade-offs
3. **Breaking changes**: None - purely additive

## Timeline

- **Phase 1**: 30 minutes (config changes)
- **Phase 2**: 30 minutes (logic implementation)
- **Phase 3**: 45 minutes (documentation)
- **Phase 4**: 45 minutes (testing)
- **Phase 5**: 15 minutes (release prep)

**Total estimated time**: ~3 hours

## Alternative Approaches Considered

### Option A: Always Use Exact (No Configuration)
**Rejected**: Would slow down all users by 7-10x, contrary to optimization goal

### Option B: Auto-Detect Based on Repository Size
**Rejected**: Adds complexity, unpredictable behavior

### Option C: Separate Binary (jj-starship-exact)
**Rejected**: Maintenance burden, confusing for users

## Open Questions

1. **Should this be per-repository configurable?**
   - Current plan: Global config only
   - Future: Could add per-repo .jj-starship.toml support

2. **Should we expose more granular control?**
   - Current plan: Boolean flag only
   - Future: Could add `--prefix-mode=fixed|exact|auto`

3. **Should we cache the exact computation?**
   - Current plan: No caching (rely on jj-lib's index cache)
   - Future: Could add application-level caching if needed

## Success Criteria

- [ ] Default behavior unchanged (fixed 4-char prefix)
- [ ] New flag works correctly
- [ ] Environment variable works correctly
- [ ] No performance regression on default path
- [ ] Comprehensive documentation
- [ ] All tests pass
- [ ] Benchmark confirms expected performance

## Future Enhancements

1. **Per-repository configuration**: Allow `.jj-starship.toml` in repo root
2. **Adaptive mode**: Auto-switch based on repository characteristics
3. **Caching**: Add application-level cache for exact computations
4. **Metrics**: Log which mode is being used for analytics
