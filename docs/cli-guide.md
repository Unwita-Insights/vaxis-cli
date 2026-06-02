# CLI (Command Line Interface) — Complete Guide

## Table of Contents

1. [What is a CLI?](#what-is-a-cli)
2. [Why is CLI needed?](#why-is-cli-needed)
3. [Core Concepts](#core-concepts)
4. [Building a Basic CLI](#building-a-basic-cli)
5. [Testing a CLI](#testing-a-cli)
6. [Project Structure](#project-structure)
7. [Next Steps](#next-steps)

---

## What is a CLI?

A **CLI (Command Line Interface)** is a text-based interface where users interact with a program by typing commands in a terminal instead of clicking buttons in a graphical interface (GUI).

### Real-world examples you already use

```bash
git commit -m "message"
npm install react
python script.py --verbose
ls -la
```

---

## Why is CLI needed?

| Reason | Explanation |
|--------|-------------|
| **Automation** | Commands can be chained in scripts; GUIs cannot |
| **Speed** | Faster for developers than navigating menus |
| **Remote access** | Works over SSH — no desktop environment needed |
| **Composability** | Pipe output between commands (`grep`, `awk`, `sort`) |
| **Reproducibility** | Commands are text — easy to document, share, version |
| **Resource efficiency** | No rendering overhead like a GUI app |

---

## Core Concepts

### Anatomy of a CLI command

```
program  subcommand  argument     --option   value    --flag
  |          |           |            |         |        |
git        commit    file.txt    --message  "fix bug"  --all
```

| Part | Description | Example |
|------|-------------|---------|
| **Program** | The CLI tool itself | `git`, `npm`, `python` |
| **Subcommand** | Action to perform | `commit`, `install`, `run` |
| **Argument** | Positional input (order matters) | `Alice` in `greet Alice` |
| **Option** | Named input with a value | `--times 3` |
| **Flag** | Boolean switch, no value | `--shout`, `--verbose` |

### Exit codes

```bash
program exits with 0   → success
program exits with 1+  → error
```

Scripts rely on exit codes to detect failures:

```bash
python greet.py Alice && echo "success" || echo "failed"
```

---

## Building a Basic CLI

### The simplest CLI (Python + argparse)

**File: `greet.py`**

```python
import argparse

def main():
    parser = argparse.ArgumentParser(description="A greeting tool")

    # Positional argument (required)
    parser.add_argument("name", help="Name to greet")

    # Flag (boolean, no value)
    parser.add_argument("--shout", action="store_true", help="Print in uppercase")

    # Option (takes a value)
    parser.add_argument("--times", type=int, default=1, help="How many times to greet")

    args = parser.parse_args()

    message = f"Hello, {args.name}!"
    if args.shout:
        message = message.upper()

    for _ in range(args.times):
        print(message)

if __name__ == "__main__":
    main()
```

### Running it

```bash
python greet.py Alice
# Hello, Alice!

python greet.py Alice --shout
# HELLO, ALICE!

python greet.py Alice --times 3
# Hello, Alice!
# Hello, Alice!
# Hello, Alice!

python greet.py Alice --shout --times 3
# HELLO, ALICE!
# HELLO, ALICE!
# HELLO, ALICE!

python greet.py --help
# Shows auto-generated usage information
```

---

## Testing a CLI

### Step 1: Manual testing

Run the program directly and verify output by eye.

```bash
python greet.py Alice          # check happy path
python greet.py Alice --shout  # check flag behavior
python greet.py                # check missing arg error
python greet.py Alice --times abc  # check wrong type error
```

### Step 2: Write automated tests

Use `subprocess` to test the CLI exactly as a user would — through the terminal interface.

**File: `test_greet.py`**

```python
import subprocess

def run(args):
    """Run the CLI with given args, return (stdout, exit_code)."""
    result = subprocess.run(
        ["python", "greet.py"] + args,
        capture_output=True,
        text=True
    )
    return result.stdout.strip(), result.returncode


def test_basic_greeting():
    out, code = run(["Alice"])
    assert out == "Hello, Alice!"
    assert code == 0


def test_shout_flag():
    out, code = run(["Alice", "--shout"])
    assert out == "HELLO, ALICE!"


def test_times_option():
    out, code = run(["Alice", "--times", "3"])
    lines = out.split("\n")
    assert len(lines) == 3
    assert all(line == "Hello, Alice!" for line in lines)


def test_combined_flags():
    out, code = run(["Alice", "--shout", "--times", "2"])
    assert out == "HELLO, ALICE!\nHELLO, ALICE!"


def test_missing_required_arg():
    out, code = run([])
    assert code != 0  # must fail


def test_wrong_type_for_option():
    out, code = run(["Alice", "--times", "abc"])
    assert code != 0  # must fail


if __name__ == "__main__":
    test_basic_greeting()
    test_shout_flag()
    test_times_option()
    test_combined_flags()
    test_missing_required_arg()
    test_wrong_type_for_option()
    print("All tests passed!")
```

### Step 3: Run with pytest

```bash
pip install pytest
pytest test_greet.py -v
```

Expected output:

```
test_greet.py::test_basic_greeting     PASSED
test_greet.py::test_shout_flag         PASSED
test_greet.py::test_times_option       PASSED
test_greet.py::test_combined_flags     PASSED
test_greet.py::test_missing_required_arg PASSED
test_greet.py::test_wrong_type_for_option PASSED

6 passed in 0.42s
```

### Testing strategy

| Test type | What to test |
|-----------|-------------|
| **Happy path** | Normal inputs, expected outputs |
| **Flags & options** | Each flag and option in isolation |
| **Combinations** | Multiple flags together |
| **Error cases** | Missing args, wrong types, unknown flags |
| **Exit codes** | 0 on success, non-zero on failure |
| **Help text** | `--help` flag exits with 0 and prints usage |

---

## Project Structure

For a real CLI with multiple subcommands:

```
my_cli/
├── cli.py              # entry point — argument parsing only
├── commands/
│   ├── __init__.py
│   ├── build.py        # one file per subcommand
│   └── deploy.py
├── utils.py            # shared helpers
├── tests/
│   ├── test_build.py
│   └── test_deploy.py
└── requirements.txt
```

### Subcommand pattern

```python
# cli.py
import argparse
from commands import build, deploy

def main():
    parser = argparse.ArgumentParser(description="My deployment tool")
    subparsers = parser.add_subparsers(dest="command")

    build.register(subparsers)    # each command registers itself
    deploy.register(subparsers)

    args = parser.parse_args()

    if args.command == "build":
        build.run(args)
    elif args.command == "deploy":
        deploy.run(args)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
```

```bash
python cli.py build --target prod
python cli.py deploy --env staging
```

---

## Next Steps

| Topic | Library to explore |
|-------|-------------------|
| Better CLI ergonomics | [`click`](https://click.palletsprojects.com/) |
| Type-safe CLIs | [`typer`](https://typer.tiangolo.com/) (built on click) |
| Node.js CLIs | [`commander`](https://github.com/tj/commander.js) |
| Go CLIs | [`cobra`](https://cobra.dev/) (used by `kubectl`, `gh`) |
| Rich terminal output | [`rich`](https://github.com/Textualize/rich) |
| Progress bars | [`tqdm`](https://tqdm.github.io/) |

### Key takeaway

```
Parse text input → Run logic → Print to stdout → Exit with correct code
```

That is all a CLI is. Everything else is just structure around this loop.
