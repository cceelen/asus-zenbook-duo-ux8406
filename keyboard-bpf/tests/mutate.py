#!/usr/bin/env python3
"""Mutation check of the HID-BPF program against the host harness.

There is no cargo-mutants for C, so this applies mutants to a temporary copy of
the source, builds the harness with each one and runs it. A mutant is
  caught     the harness failed, crashed or timed out
  unviable   the mutant does not build (a _Static_assert or the compiler
             refuses it); the build catches it
  survived   the harness passed: a test is missing, or the mutant is
             equivalent (the program behaves the same)
Mutants, each applied alone, outside comments, strings and preprocessor lines:
  - each comparison (== != < > <= >=) turned to its opposite, and < > to <= >=
  - && and || turned into each other
  - each ! removed
  - the two arms of each ternary swapped
  - each return statement dropped
  - each statement that is a call or an assignment dropped
  - each if condition replaced by 0 and by 1
  - each integer and character literal changed by +1 and -1
  - true and false swapped

usage: mutate.py PROGRAM.bpf.c [WORKDIR] [--list-survivors]
Exit status 1 if a mutant survived and is not in EQUIVALENT below.
"""

import concurrent.futures
import importlib.util
import os
import pathlib
import re
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("run_host", HERE / "run_host.py")
run_host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(run_host)

# Mutants that survive and cannot change what the program does: the source
# line (regex), the mutant's description (regex) and why.
EQUIVALENT = [
    (
        r"kCodeNone = 0x00",
        r"literal",
        (
            "kCodeNone is not used: the code 0 needs no name, as every code that "
            "is not listed is no key"
        ),
    ),
    (
        r"int err = 0;",
        r"literal",
        "a dead store: hid_bpf_hw_request's result replaces it before it is read",
    ),
    (
        r"return err < 0 \? err : 0;",
        r"(< -> <=|literal 0 \+1)",
        (
            "for err == 0 both arms give 0, so `err <= 0` and `err < 1` return "
            "what `err < 0` returns"
        ),
    ),
]

OPS = (
    "<<=",
    ">>=",
    "...",
    "->",
    "++",
    "--",
    "<<",
    ">>",
    "<=",
    ">=",
    "==",
    "!=",
    "&&",
    "||",
    "+=",
    "-=",
    "*=",
    "/=",
    "|=",
    "&=",
    "^=",
)
OPPOSITE = {"==": "!=", "!=": "==", "<": ">=", ">": "<=", "<=": ">", ">=": "<"}
CONTROL = {"if", "else", "for", "while", "do", "switch"}
TYPES = {
    "static",
    "const",
    "struct",
    "enum",
    "int",
    "char",
    "bool",
    "void",
    "__u8",
    "__u32",
    "unsigned",
    "_Static_assert",
    "typedef",
}


class Token:
    def __init__(self, kind, text, start, end):
        self.kind, self.text, self.start, self.end = kind, text, start, end


def lex(src):
    """Tokens outside comments, strings and preprocessor lines."""
    tokens, i, n, line_start = [], 0, len(src), True
    while i < n:
        c = src[i]
        if c == "\n":
            line_start, i = True, i + 1
        elif c in " \t\r":
            i += 1
        elif src.startswith("/*", i):
            i = src.index("*/", i) + 2
        elif src.startswith("//", i):
            i = src.index("\n", i)
        elif c == "#" and line_start:
            while True:  # to the end of the line, with continuations
                j = src.index("\n", i)
                cont = src[i:j].endswith("\\")
                i = j + 1
                if not cont:
                    break
        else:
            line_start = False
            m = (
                re.compile(r"0[xX][0-9a-fA-F]+[uUlL]*|\d+[uUlL]*").match(src, i)
                if c.isdigit()
                else None
            )
            if m:
                tokens.append(Token("num", m.group(), i, m.end()))
                i = m.end()
            elif c.isalpha() or c == "_":
                m = re.compile(r"\w+").match(src, i)
                tokens.append(Token("id", m.group(), i, m.end()))
                i = m.end()
            elif c == '"' or c == "'":
                j = i + 1
                while src[j] != c:
                    j += 2 if src[j] == "\\" else 1
                kind = "char" if c == "'" else "str"
                tokens.append(Token(kind, src[i : j + 1], i, j + 1))
                i = j + 1
            else:
                op = next((o for o in OPS if src.startswith(o, i)), c)
                tokens.append(Token("op", op, i, i + len(op)))
                i += len(op)
    return tokens


def line_of(src, pos):
    return src.count("\n", 0, pos) + 1


def statement_end(tokens, k):
    """Index of the ';' that ends the statement starting at token k."""
    depth = 0
    for j in range(k, len(tokens)):
        t = tokens[j].text
        if t in "({[" and tokens[j].kind == "op":
            depth += 1
        elif t in ")}]" and tokens[j].kind == "op":
            depth -= 1
        elif t == ";" and depth == 0:
            return j
    return None


def mutants(src):
    """Yield (description, start, end, replacement)."""
    tokens = lex(src)
    depth = 0
    at_statement_start = True
    for k, t in enumerate(tokens):
        where = f"line {line_of(src, t.start)}"
        if t.kind == "op":
            if t.text in OPPOSITE:
                yield (
                    f"{where}: {t.text} -> {OPPOSITE[t.text]}",
                    t.start,
                    t.end,
                    OPPOSITE[t.text],
                )
            if t.text in ("<", ">"):
                yield (f"{where}: {t.text} -> {t.text}=", t.start, t.end, t.text + "=")
            if t.text == "&&":
                yield (f"{where}: && -> ||", t.start, t.end, "||")
            if t.text == "||":
                yield (f"{where}: || -> &&", t.start, t.end, "&&")
            if t.text == "!":
                yield (f"{where}: remove !", t.start, t.end, "")
            if t.text == "?":
                yield from ternary(src, tokens, k, where)
        if t.kind == "num":
            value = int(t.text.rstrip("uUlL"), 0)
            for delta in (1, -1):
                yield (
                    f"{where}: literal {t.text} {delta:+d}",
                    t.start,
                    t.end,
                    f"({value + delta})",
                )
        if t.kind == "char":
            ch = t.text[1]
            for delta in (1, -1):
                yield (
                    f"{where}: char {t.text} {delta:+d}",
                    t.start,
                    t.end,
                    str(ord(ch) + delta),
                )
        if t.kind == "id" and t.text in ("true", "false"):
            other = "false" if t.text == "true" else "true"
            yield (f"{where}: {t.text} -> {other}", t.start, t.end, other)
        # Statements inside function bodies.
        if t.kind == "op" and t.text == "{":
            depth += 1
            at_statement_start = True
            continue
        if t.kind == "op" and t.text == "}":
            depth -= 1
            at_statement_start = True
            continue
        if t.kind == "op" and t.text == ";":
            at_statement_start = True
            continue
        if t.kind == "id" and t.text == "if" and tokens[k + 1].text == "(":
            close = matching(tokens, k + 1)
            for value in ("0", "1"):
                yield (
                    f"{where}: if condition -> {value}",
                    tokens[k + 1].end,
                    tokens[close].start,
                    value,
                )
        if (
            at_statement_start
            and depth >= 1
            and t.text not in TYPES
            and t.kind != "op"
            and t.text not in CONTROL
        ):
            end = statement_end(tokens, k)
            if end is not None and not (t.text == "case" or t.text == "default"):
                what = "return" if t.text == "return" else "statement"
                yield (
                    f"{where}: drop {what} `{src[t.start : tokens[end].end][:50]}`",
                    t.start,
                    tokens[end].end,
                    ";",
                )
        at_statement_start = False


def matching(tokens, k):
    """Index of the ')' that closes the '(' at token k."""
    depth = 0
    for j in range(k, len(tokens)):
        if tokens[j].kind == "op" and tokens[j].text == "(":
            depth += 1
        elif tokens[j].kind == "op" and tokens[j].text == ")":
            depth -= 1
            if depth == 0:
                return j
    raise ValueError("unbalanced")


def ternary(src, tokens, k, where):
    """Swap the arms of the ternary whose '?' is token k."""
    depth, colon, end = 0, None, None
    for j in range(k + 1, len(tokens)):
        t = tokens[j]
        if t.kind != "op":
            continue
        if t.text in "([{":
            depth += 1
        elif t.text in ")]}":
            if depth == 0:
                end = j
                break
            depth -= 1
        elif depth == 0 and t.text == ":" and colon is None:
            colon = j
        elif depth == 0 and t.text in (";", ","):
            end = j
            break
    first = src[tokens[k].end : tokens[colon].start]
    second = src[tokens[colon].end : tokens[end].start]
    yield (
        f"{where}: swap ternary arms",
        tokens[k].end,
        tokens[end].start,
        f"{second.strip()} : {first.strip()} ",
    )


def build_and_run(index, desc, mutated, work, compiler, flags, test_objects):
    src = work / f"m{index}" / "src"
    src.mkdir(parents=True)
    path = src / "mutant.bpf.c"
    path.write_text(mutated)
    obj = work / f"m{index}" / "mutant.o"
    built = subprocess.run(
        [compiler, *flags, "-Werror=return-type", "-c", str(path), "-o", str(obj)],
        capture_output=True,
        check=False,
    )
    if built.returncode != 0:
        return desc, "unviable"
    binary = work / f"m{index}" / "harness"
    linked = subprocess.run(
        [compiler, *flags, str(obj), *test_objects, "-o", str(binary)],
        capture_output=True,
        check=False,
    )
    if linked.returncode != 0:
        return desc, "unviable"
    try:
        ran = subprocess.run(
            [str(binary)], capture_output=True, timeout=20, check=False
        )
    except subprocess.TimeoutExpired:
        return desc, "caught"
    if ran.returncode != 0:
        return desc, "caught"
    # The program's udev property variables are an interface of the loader.
    try:
        run_host.check_symbol_sizes(obj)
    except SystemExit:
        return desc, "caught"
    return desc, "survived"


def argued(src, desc):
    """The reason why a surviving mutant is equivalent, or None."""
    text = src.splitlines()[int(re.match(r"line (\d+)", desc).group(1)) - 1]
    for line_re, desc_re, reason in EQUIVALENT:
        if re.search(line_re, text) and re.search(desc_re, desc):
            return reason
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        sys.exit(__doc__)
    program = pathlib.Path(args[0]).resolve()
    work = pathlib.Path(
        args[1]
        if len(args) > 1
        else os.environ.get("TMPDIR", "/tmp") + "/ux8406-bpf-mutants"
    )
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    compiler = shutil.which("clang") or os.environ.get("CC") or "cc"
    flags = run_host.harness_flags([])
    test_objects = [
        run_host.compile_one(compiler, flags, source, work / (source.stem + ".o"))
        for source in run_host.TESTS
    ]
    src = program.read_text()
    jobs = []
    for index, (desc, start, end, repl) in enumerate(mutants(src)):
        jobs.append((index, desc, src[:start] + repl + src[end:]))
    with concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count()) as pool:
        results = list(
            pool.map(
                lambda j: build_and_run(
                    j[0], j[1], j[2], work, compiler, flags, test_objects
                ),
                jobs,
            )
        )
    counts = {"caught": 0, "unviable": 0, "survived": 0}
    bad = []
    for desc, outcome in results:
        counts[outcome] += 1
        if outcome == "survived":
            reason = argued(src, desc)
            print(f"SURVIVED {desc}" + (f"  [equivalent: {reason}]" if reason else ""))
            if reason is None:
                bad.append(desc)
    print(
        f"{len(results)} mutants: {counts['caught']} caught, "
        f"{counts['unviable']} unviable, {counts['survived']} survived "
        f"({len(bad)} not argued as equivalent)"
    )
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
