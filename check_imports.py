#!/usr/bin/env python3
"""
Housaky Import Checker
Scans src/housaky/ to verify all .rs files are reachable from the module tree.

Usage:
    python3 check_imports.py [--fix] [--verbose]

Reports:
    - Files on disk not declared in any mod.rs
    - mod.rs declarations with no matching file
    - Modules declared but never imported outside their own tree
"""

import os
import re
import sys
from pathlib import Path
from collections import defaultdict

SRC_DIR = Path(__file__).parent / "src"
HOUSAKY_DIR = SRC_DIR / "housaky"


def find_all_rs_files(root: Path) -> list[Path]:
    """Find all .rs files under root."""
    return sorted(root.rglob("*.rs"))


def parse_mod_declarations(mod_file: Path) -> list[str]:
    """Extract `pub mod X;` and `mod X;` declarations from a mod.rs file."""
    if not mod_file.exists():
        return []
    content = mod_file.read_text(errors="replace")
    # Match both `pub mod name;` and `mod name;` but not `pub mod name { ... }`
    pattern = re.compile(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;", re.MULTILINE)
    return pattern.findall(content)


def resolve_mod_target(parent_dir: Path, mod_name: str) -> Path | None:
    """Given a directory and a mod name, find the file it resolves to."""
    # Check parent_dir/mod_name.rs
    file_path = parent_dir / f"{mod_name}.rs"
    if file_path.exists():
        return file_path
    # Check parent_dir/mod_name/mod.rs
    dir_path = parent_dir / mod_name / "mod.rs"
    if dir_path.exists():
        return dir_path
    return None


def find_imports_of_module(src_dir: Path, module_path: str) -> list[tuple[Path, str]]:
    """Search all .rs files for imports of a given module path."""
    results = []
    pattern = re.compile(re.escape(module_path))
    for rs_file in src_dir.rglob("*.rs"):
        try:
            content = rs_file.read_text(errors="replace")
        except Exception:
            continue
        for i, line in enumerate(content.splitlines(), 1):
            if pattern.search(line):
                results.append((rs_file, f"line {i}: {line.strip()[:120]}"))
    return results


def build_module_tree(root_mod: Path) -> dict:
    """Recursively build module tree starting from a mod.rs."""
    tree = {}
    if not root_mod.exists():
        return tree

    parent_dir = root_mod.parent
    declared = parse_mod_declarations(root_mod)

    for mod_name in declared:
        target = resolve_mod_target(parent_dir, mod_name)
        tree[mod_name] = {
            "declared_in": str(root_mod),
            "resolved_to": str(target) if target else None,
            "exists": target is not None,
        }
        # If it resolves to a directory mod.rs, recurse
        if target and target.name == "mod.rs":
            subtree = build_module_tree(target)
            tree[mod_name]["children"] = subtree

    return tree


def collect_reachable_files(tree: dict, parent_dir: Path) -> set[Path]:
    """Collect all files reachable from the module tree."""
    reachable = set()
    for mod_name, info in tree.items():
        if info["resolved_to"]:
            resolved = Path(info["resolved_to"])
            reachable.add(resolved)
            # If it's a mod.rs, add all .rs files in its directory that are declared
            if resolved.name == "mod.rs":
                mod_dir = resolved.parent
                if "children" in info:
                    reachable |= collect_reachable_files(info["children"], mod_dir)
                # Also add the mod.rs itself
                reachable.add(resolved)
        # Also handle the file variant
        file_path = parent_dir / f"{mod_name}.rs"
        if file_path.exists():
            reachable.add(file_path)
        dir_mod = parent_dir / mod_name / "mod.rs"
        if dir_mod.exists():
            reachable.add(dir_mod)
    return reachable


def check_external_usage(src_dir: Path, housaky_dir: Path, module_name: str) -> list[tuple[Path, str]]:
    """Check if a housaky module is used outside its own directory."""
    patterns = [
        f"housaky::{module_name}",
        f"crate::housaky::{module_name}",
        f"super::{module_name}",
    ]

    results = []
    module_dir = housaky_dir / module_name
    module_file = housaky_dir / f"{module_name}.rs"

    for rs_file in src_dir.rglob("*.rs"):
        # Skip files inside the module's own directory
        try:
            if module_dir.exists() and rs_file.is_relative_to(module_dir):
                continue
            if rs_file == module_file:
                continue
        except ValueError:
            pass

        try:
            content = rs_file.read_text(errors="replace")
        except Exception:
            continue

        for pattern in patterns:
            if pattern in content:
                for i, line in enumerate(content.splitlines(), 1):
                    if pattern in line:
                        results.append((rs_file, f"line {i}: {line.strip()[:120]}"))
                break  # One match per file is enough

    return results


def main():
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    print("=" * 70)
    print("HOUSAKY IMPORT CHECKER")
    print("=" * 70)
    print()

    # 1. Build module tree from housaky/mod.rs
    housaky_mod = HOUSAKY_DIR / "mod.rs"
    if not housaky_mod.exists():
        print(f"ERROR: {housaky_mod} not found!")
        sys.exit(1)

    print("[1/5] Building module tree from housaky/mod.rs...")
    tree = build_module_tree(housaky_mod)
    declared_count = len(tree)
    print(f"  Declared modules in mod.rs: {declared_count}")

    # 2. Find all .rs files on disk
    print("\n[2/5] Scanning disk for .rs files...")
    all_rs_files = find_all_rs_files(HOUSAKY_DIR)
    # Filter out the mod.rs itself and non-Rust files
    all_rs_files = [f for f in all_rs_files if f.name.endswith(".rs")]
    print(f"  Total .rs files on disk: {len(all_rs_files)}")

    # 3. Check for files NOT in module tree
    print("\n[3/5] Checking for orphaned files (on disk but not in module tree)...")
    reachable = collect_reachable_files(tree, HOUSAKY_DIR)
    reachable.add(housaky_mod)  # mod.rs itself is always reachable

    orphaned = []
    for rs_file in all_rs_files:
        if rs_file not in reachable:
            # Check if it's declared in a local mod.rs
            parent_mod = rs_file.parent / "mod.rs"
            local_name = rs_file.stem
            if parent_mod.exists() and parent_mod != rs_file:
                local_decls = parse_mod_declarations(parent_mod)
                if local_name in local_decls:
                    continue  # It's declared in its parent's mod.rs
            orphaned.append(rs_file)

    if orphaned:
        print(f"  ORPHANED FILES ({len(orphaned)}):")
        for f in orphaned:
            rel = f.relative_to(HOUSAKY_DIR)
            print(f"    - {rel}")
    else:
        print("  No orphaned files found.")

    # 4. Check for phantom declarations (declared but file missing)
    print("\n[4/5] Checking for phantom declarations (declared but no file)...")
    phantoms = []
    for mod_name, info in tree.items():
        if not info["exists"]:
            phantoms.append((mod_name, info["declared_in"]))

    if phantoms:
        print(f"  PHANTOM DECLARATIONS ({len(phantoms)}):")
        for name, declared_in in phantoms:
            print(f"    - `{name}` declared in {declared_in} but no file found")
    else:
        print("  No phantom declarations found.")

    # 5. Check for dead modules (declared but never imported externally)
    print("\n[5/5] Checking for dead modules (declared but never used externally)...")
    dead_modules = []
    active_modules = []

    for mod_name in sorted(tree.keys()):
        usages = check_external_usage(SRC_DIR, HOUSAKY_DIR, mod_name)
        if not usages:
            dead_modules.append(mod_name)
        else:
            active_modules.append((mod_name, len(usages)))

    if dead_modules:
        print(f"  DEAD MODULES ({len(dead_modules)}):")
        for name in dead_modules:
            print(f"    - {name}")
    else:
        print("  All modules are used externally.")

    if verbose and active_modules:
        print(f"\n  ACTIVE MODULES ({len(active_modules)}):")
        for name, count in active_modules:
            print(f"    - {name} ({count} external references)")

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"  Total .rs files:          {len(all_rs_files)}")
    print(f"  Declared modules:         {declared_count}")
    print(f"  Orphaned files:           {len(orphaned)}")
    print(f"  Phantom declarations:     {len(phantoms)}")
    print(f"  Dead modules:             {len(dead_modules)}")
    print(f"  Active modules:           {len(active_modules)}")

    health = len(active_modules) / max(declared_count, 1) * 100
    print(f"  Module health:            {health:.1f}%")

    if orphaned or phantoms or dead_modules:
        print("\n  STATUS: ISSUES FOUND - see above for details")
        return 1
    else:
        print("\n  STATUS: ALL CLEAN")
        return 0


if __name__ == "__main__":
    sys.exit(main())
