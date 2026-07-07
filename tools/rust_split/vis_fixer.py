#!/usr/bin/env python3
"""Iteratively fix E0616 (private field) / E0624 (private method) by adding pub(crate)."""
import re
import subprocess
import sys
import glob

PKG = sys.argv[1] if len(sys.argv) > 1 else 'ugaris-server'
SRC_GLOB = sys.argv[2] if len(sys.argv) > 2 else 'crates/ugaris-server/src/**/*.rs'


def build():
    r = subprocess.run(['cargo', 'test', '-p', PKG, '--no-run'], capture_output=True, text=True)
    return r.stderr


def fix_field(struct_name, field_name):
    for path in glob.glob(SRC_GLOB, recursive=True):
        src = open(path).read()
        m = re.search(r'^(?:pub(?:\(crate\))?\s+)?struct\s+' + re.escape(struct_name) + r'\b[^;{]*\{', src, re.M)
        if not m:
            continue
        # find field line inside struct block
        start = m.end()
        depth = 1
        i = start
        while i < len(src) and depth > 0:
            if src[i] == '{':
                depth += 1
            elif src[i] == '}':
                depth -= 1
            i += 1
        block = src[start:i]
        new_block, n = re.subn(r'^(\s+)(' + re.escape(field_name) + r'\s*:)', r'\1pub(crate) \2', block, count=1, flags=re.M)
        if n:
            open(path, 'w').write(src[:start] + new_block + src[i:])
            return path
    return None


def fix_method(method_name):
    for path in glob.glob(SRC_GLOB, recursive=True):
        src = open(path).read()
        new_src, n = re.subn(r'^(\s+)fn\s+(' + re.escape(method_name) + r')\b', r'\1pub(crate) fn \2', src, count=1, flags=re.M)
        if n:
            open(path, 'w').write(new_src)
            return path
    return None


for iteration in range(30):
    err = build()
    fields = set(re.findall(r"error\[E0616\]: field `(\w+)` of struct `(?:[\w:]*?)(\w+)` is private", err))
    methods = set(re.findall(r"error\[E0624\]: (?:method|associated function) `(\w+)` is private", err))
    if not fields and not methods:
        remaining = err.count('error[')
        print(f'done after {iteration} iterations; other errors: {remaining}')
        errs = re.findall(r'error\[\w+\][^\n]*', err)
        for e in list(dict.fromkeys(errs))[:15]:
            print(' ', e)
        break
    print(f'iter {iteration}: {len(fields)} field fixes, {len(methods)} method fixes')
    for field, struct in fields:
        p = fix_field(struct, field)
        print(f'  field {struct}.{field} -> {p}')
    for m in methods:
        p = fix_method(m)
        print(f'  method {m} -> {p}')
