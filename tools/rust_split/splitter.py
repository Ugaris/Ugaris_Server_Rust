#!/usr/bin/env python3
"""General-purpose Rust file splitter.

Parses a large Rust source file into top-level items (string/comment aware),
optionally explodes named impl blocks and inline `mod tests` into second-level
items, then redistributes items into a module directory according to a spec.
"""

import re
import sys
import os
import importlib.util
from collections import OrderedDict, defaultdict


def scan_states(text):
    """Return bytearray: state[i]=1 if char i is inside string/comment."""
    n = len(text)
    states = bytearray(n)
    i = 0
    mode = 0  # 0 normal, 1 line comment, 2 block comment, 3 string, 4 raw string
    block_depth = 0
    raw_hashes = 0
    while i < n:
        c = text[i]
        if mode == 0:
            if c == '/' and i + 1 < n and text[i + 1] == '/':
                mode = 1
                states[i] = 1
            elif c == '/' and i + 1 < n and text[i + 1] == '*':
                mode = 2
                block_depth = 1
                states[i] = 1
            elif c == '"':
                mode = 3
                states[i] = 1
            elif c == 'r' and i + 1 < n and (text[i + 1] == '"' or text[i + 1] == '#'):
                j = i + 1
                h = 0
                while j < n and text[j] == '#':
                    h += 1
                    j += 1
                if j < n and text[j] == '"':
                    raw_hashes = h
                    mode = 4
                    for k in range(i, j + 1):
                        states[k] = 1
                    i = j + 1
                    continue
            elif c == "'":
                if i + 1 < n and text[i + 1] == '\\':
                    j = i + 2
                    while j < n and text[j] != "'":
                        j += 1
                    for k in range(i, min(j + 1, n)):
                        states[k] = 1
                    i = j + 1
                    continue
                elif i + 2 < n and text[i + 2] == "'":
                    states[i] = states[i + 1] = states[i + 2] = 1
                    i += 3
                    continue
        elif mode == 1:
            states[i] = 1
            if c == '\n':
                mode = 0
                states[i] = 0
        elif mode == 2:
            states[i] = 1
            if c == '/' and i + 1 < n and text[i + 1] == '*':
                block_depth += 1
                states[i + 1] = 1
                i += 2
                continue
            if c == '*' and i + 1 < n and text[i + 1] == '/':
                block_depth -= 1
                states[i + 1] = 1
                i += 2
                if block_depth == 0:
                    mode = 0
                continue
        elif mode == 3:
            states[i] = 1
            if c == '\\':
                if i + 1 < n:
                    states[i + 1] = 1
                i += 2
                continue
            if c == '"':
                mode = 0
        elif mode == 4:
            states[i] = 1
            if c == '"':
                j = i + 1
                h = 0
                while j < n and text[j] == '#' and h < raw_hashes:
                    h += 1
                    j += 1
                if h == raw_hashes:
                    for k in range(i, j):
                        states[k] = 1
                    i = j
                    mode = 0
                    continue
        i += 1
    return states


ITEM_RE = re.compile(
    r'^(?:pub(?:\((?:crate|super|self|in [^)]*)\))?\s+)?'
    r'(?:default\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?(?:const\s+)?'
    r'(fn|struct|enum|impl|mod|trait|type|use|const|static|macro_rules!)\b'
)
NAME_RE = re.compile(r'\b(?:fn|struct|enum|mod|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)')
IMPL_RE = re.compile(r'\bimpl(?:\s*<[^>]*>)?\s+(.*?)\s*(?:\{|$)')


class Item:
    def __init__(self, kind, name, start, end):
        self.kind = kind
        self.name = name
        self.start = start
        self.end = end
        self.children = None


class Ctx:
    def __init__(self, path):
        self.text = open(path).read()
        self.lines = self.text.splitlines(keepends=True)
        states = scan_states(self.text)
        self.rows = []
        pos = 0
        for line in self.lines:
            self.rows.append(states[pos:pos + len(line)])
            pos += len(line)

    def in_string_at_line_start(self, li):
        row = self.rows[li]
        return len(row) > 0 and row[0] == 1 and not self.lines[li].lstrip().startswith('//')

    def clean_line(self, li):
        """line with string/comment chars blanked"""
        out = []
        row = self.rows[li]
        for ci, ch in enumerate(self.lines[li]):
            out.append(' ' if (ci < len(row) and row[ci] == 1) else ch)
        return ''.join(out)


def find_item_end(ctx, i, limit):
    """Scan from line i for end of item (brace close or ; at depth 0)."""
    depth = 0
    j = i
    while j < limit:
        row = ctx.rows[j]
        line = ctx.lines[j]
        ended = False
        for ci, ch in enumerate(line):
            if ci < len(row) and row[ci] == 1:
                continue
            if ch in '{([':
                depth += 1
            elif ch in '})]':
                depth -= 1
                if depth == 0 and ch == '}':
                    ended = True
            elif ch == ';' and depth == 0:
                ended = True
        if ended:
            return j + 1
        j += 1
    return limit


def parse_region(ctx, start, limit, indent):
    """Parse items at the given indent level within [start, limit)."""
    prefix = ' ' * indent
    items = []
    i = start
    pending = None
    while i < limit:
        line = ctx.lines[i]
        stripped = line.strip()
        if stripped == '':
            i += 1
            continue
        # line must start exactly at `indent`
        starts_here = line.startswith(prefix) and (len(line) > indent and line[indent] not in ' \t')
        if indent == 0:
            starts_here = line[0] not in ' \t'
        if not starts_here or ctx.in_string_at_line_start(i):
            i += 1
            continue
        rest = line[indent:]
        if stripped.startswith('//'):
            if pending is None:
                pending = i
            i += 1
            continue
        if stripped.startswith('#['):
            if pending is None:
                pending = i
            depth = ctx.clean_line(i).count('[') - ctx.clean_line(i).count(']')
            while depth > 0 and i + 1 < limit:
                i += 1
                depth += ctx.clean_line(i).count('[') - ctx.clean_line(i).count(']')
            i += 1
            continue
        m = ITEM_RE.match(rest)
        if not m:
            pending = None
            i += 1
            continue
        kind = m.group(1)
        if kind == 'impl':
            im = IMPL_RE.search(rest)
            name = 'impl ' + (im.group(1).strip() if im else '?')
        elif kind == 'use':
            name = 'use'
        elif kind == 'macro_rules!':
            nm = re.search(r'macro_rules!\s+([A-Za-z_][A-Za-z0-9_]*)', rest)
            name = nm.group(1) if nm else '?'
        else:
            nm = NAME_RE.search(rest)
            name = nm.group(1) if nm else '?'
        s = pending if pending is not None else i
        e = find_item_end(ctx, i, limit)
        items.append(Item(kind, name, s, e))
        pending = None
        i = e
    return items


def parse_children(ctx, item, indent=4):
    i = item.start
    while i < item.end and '{' not in ctx.clean_line(i):
        i += 1
    body_start = i + 1
    body_end = item.end - 1
    return parse_region(ctx, body_start, body_end, indent)


def extract(ctx, start, end, dedent=0, indent=0):
    """Extract lines [start,end) shifting indentation, string-aware."""
    out = []
    for li in range(start, end):
        line = ctx.lines[li]
        if ctx.in_string_at_line_start(li):
            out.append(line)
            continue
        if line.strip() == '':
            out.append('\n' if line.endswith('\n') else line)
            continue
        if dedent and line.startswith(' ' * dedent):
            line = line[dedent:]
        if indent:
            line = ' ' * indent + line
        out.append(line)
    return ''.join(out)


def make_pub_crate(chunk, kind):
    if kind in ('impl', 'use', 'macro_rules!', 'mod'):
        return chunk
    lines = chunk.splitlines(keepends=True)
    attr_depth = 0
    for idx, line in enumerate(lines):
        s = line.strip()
        if attr_depth > 0:
            attr_depth += s.count('[') - s.count(']')
            continue
        if s.startswith('#['):
            attr_depth = s.count('[') - s.count(']')
            continue
        if s.startswith('//') or s == '':
            continue
        core = line.lstrip()
        ind = line[:len(line) - len(core)]
        if re.match(r'^pub\b', core):
            return chunk
        if ITEM_RE.match(core):
            lines[idx] = ind + 'pub(crate) ' + core
            return ''.join(lines)
        return chunk
    return chunk


def match_rule(rule, name):
    if isinstance(rule, str):
        return rule == name
    k, v = rule
    if k == 'prefix':
        return name.startswith(v)
    if k == 'contains':
        return v in name
    if k == 're':
        return re.search(v, name) is not None
    raise ValueError(rule)


def assign_target(name, assign, default):
    for rule, target in assign:
        if match_rule(rule, name):
            return target
    return default


def main():
    spec_path = sys.argv[1]
    sys.path.insert(0, os.path.dirname(os.path.abspath(spec_path)))
    spec = importlib.import_module(os.path.splitext(os.path.basename(spec_path))[0])

    ctx = Ctx(spec.SOURCE)
    items = parse_region(ctx, 0, len(ctx.lines), 0)

    # coverage check
    covered = set()
    for it in items:
        for li in range(it.start, it.end):
            if li in covered:
                print(f'OVERLAP line {li+1} ({it.kind} {it.name})', file=sys.stderr)
                sys.exit(1)
            covered.add(li)
    bad = [(i + 1, ctx.lines[i]) for i in range(len(ctx.lines))
           if i not in covered and ctx.lines[i].strip() != '']
    if bad:
        print('UNCOVERED non-blank lines:', file=sys.stderr)
        for ln, l in bad[:30]:
            print(f'  {ln}: {l.rstrip()}', file=sys.stderr)
        sys.exit(1)

    explode_impls = getattr(spec, 'EXPLODE_IMPLS', [])
    explode_tests = getattr(spec, 'EXPLODE_TESTS', None)
    assign = spec.ASSIGN
    default_file = getattr(spec, 'DEFAULT_FILE', '')
    test_default = getattr(spec, 'TEST_DEFAULT_FILE', 'tests/misc.rs')
    test_assign = getattr(spec, 'TEST_ASSIGN', [])
    make_crate_vis = getattr(spec, 'PUB_CRATE_REWRITE', True)

    outputs = defaultdict(list)
    mod_items = []
    mod_impl_items = []
    test_mod_items = []
    stats = defaultdict(int)
    unmatched = []

    for it in items:
        if it.kind == 'use':
            mod_items.append(extract(ctx, it.start, it.end))
            continue
        if it.kind == 'mod' and explode_tests and it.name == explode_tests:
            for ch in parse_children(ctx, it):
                chunk = extract(ctx, ch.start, ch.end, dedent=4)
                raw = ''.join(ctx.lines[ch.start:ch.end])
                if ch.kind == 'fn' and '#[test]' in raw:
                    t = assign_target('tests::' + ch.name, test_assign, test_default)
                    outputs[t].append(chunk)
                    stats[t] += 1
                    if t == test_default:
                        unmatched.append('tests::' + ch.name)
                else:
                    test_mod_items.append(chunk)
            continue
        if it.kind == 'impl' and any(it.name == 'impl ' + t for t in explode_impls):
            impl_target = it.name[len('impl '):]
            for ch in parse_children(ctx, it):
                chunk = extract(ctx, ch.start, ch.end, dedent=4)
                if make_crate_vis:
                    chunk = make_pub_crate(chunk, ch.kind)
                qual = impl_target + '::' + ch.name
                t = assign_target(qual, assign, default_file)
                if t == default_file:
                    unmatched.append(qual)
                if t == '':
                    mod_impl_items.append((impl_target, chunk))
                else:
                    outputs[t].append(('IMPL', impl_target, chunk))
                stats[t] += 1
            continue
        t = assign_target(it.name, assign, default_file)
        chunk = extract(ctx, it.start, it.end)
        if t == '':
            mod_items.append(chunk)
        else:
            if make_crate_vis:
                chunk = make_pub_crate(chunk, it.kind)
            outputs[t].append(chunk)
        stats[t] += 1

    dest = spec.DEST_DIR
    os.makedirs(dest, exist_ok=True)
    file_headers = getattr(spec, 'FILE_HEADERS', {})
    submods = OrderedDict()
    test_submods = OrderedDict()

    for target, chunks in outputs.items():
        path = os.path.join(dest, target)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        modname = os.path.splitext(os.path.basename(target))[0]
        if target.startswith('tests/'):
            test_submods[modname] = True
        else:
            submods[modname] = True
        parts = []
        hdr = file_headers.get(target, '')
        if hdr:
            parts.append(hdr.rstrip() + '\n\n')
        parts.append('use super::*;\n\n')
        cur_impl = None
        for c in chunks:
            if isinstance(c, tuple):
                _, impl_target, chunk = c
                if cur_impl != impl_target:
                    if cur_impl is not None:
                        parts.append('}\n\n')
                    parts.append(f'impl {impl_target} {{\n')
                    cur_impl = impl_target
                parts.append(indent_chunk(chunk))
                parts.append('\n')
            else:
                if cur_impl is not None:
                    parts.append('}\n\n')
                    cur_impl = None
                parts.append(c)
                if not c.endswith('\n\n'):
                    parts.append('\n')
        if cur_impl is not None:
            parts.append('}\n')
        with open(path, 'w') as f:
            f.write(''.join(parts))

    if explode_tests and (test_mod_items or test_submods):
        tpath = os.path.join(dest, 'tests')
        os.makedirs(tpath, exist_ok=True)
        with open(os.path.join(tpath, 'mod.rs'), 'w') as f:
            f.write('use super::*;\n\n')
            for name in sorted(test_submods):
                f.write(f'mod {name};\n')
            f.write('\n')
            for c in test_mod_items:
                f.write(c)
                if not c.endswith('\n\n'):
                    f.write('\n')

    header = getattr(spec, 'MOD_HEADER', '')
    with open(os.path.join(dest, 'mod.rs'), 'w') as f:
        if header:
            f.write(header.rstrip() + '\n\n')
        for name in sorted(submods):
            f.write(f'mod {name};\n')
        if submods:
            f.write('\n')
        for name in sorted(submods):
            f.write(f'pub use {name}::*;\n')
        if submods:
            f.write('\n')
        if explode_tests and (test_mod_items or test_submods):
            f.write('#[cfg(test)]\nmod tests;\n\n')
        for c in mod_items:
            f.write(c)
            if not c.endswith('\n\n'):
                f.write('\n')
        cur_impl = None
        for impl_target, chunk in mod_impl_items:
            if cur_impl != impl_target:
                if cur_impl is not None:
                    f.write('}\n\n')
                f.write(f'impl {impl_target} {{\n')
                cur_impl = impl_target
            f.write(indent_chunk(chunk))
            f.write('\n')
        if cur_impl is not None:
            f.write('}\n')

    print('=== split stats (items per file) ===')
    for t in sorted(stats):
        print(f'{t or "mod.rs"}: {stats[t]}')
    if unmatched:
        print(f'=== unmatched -> default ({len(unmatched)}) ===')
        for u in unmatched[:60]:
            print(' ', u)


def indent_chunk(chunk):
    # indent by 4, but not lines inside multiline strings; chunk was extracted
    # string-aware already, so re-scan it
    states = scan_states(chunk)
    out = []
    pos = 0
    for line in chunk.splitlines(keepends=True):
        starts_in_string = states[pos] == 1 and not line.lstrip().startswith('//') if len(line) else False
        if line.strip() and not starts_in_string:
            out.append('    ' + line)
        else:
            out.append(line)
        pos += len(line)
    return ''.join(out)


if __name__ == '__main__':
    main()
