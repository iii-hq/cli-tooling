# Cross-Language Math

Two workers — one Python, one TypeScript — demonstrating cross-language function calls via the iii engine.

For a detailed walkthrough follow the [Quickstart tutorial](https://iii.dev/docs/quickstart).

## What's Inside

| Worker          | Language   | Function                | Does                                     |
| --------------- | ---------- | ----------------------- | ---------------------------------------- |
| `math-worker`   | Python     | `math::add`             | Returns `{ c: a + b }`                   |
| `caller-worker` | TypeScript | `math::add_two_numbers` | Calls `math::add` and returns the result |

## Quick Start

### 1. Start the engine

```bash
iii
```

### 2. Start the workers (separate terminals)

```bash
iii worker dev ./workers/math-worker
```

```bash
iii worker dev ./workers/caller-worker
```

### 3. Call functions from the CLI

Call the Python worker directly:

```bash
iii trigger --function-id='math::add' --payload='{"a": 2, "b": 3}'
```

```json
{ "c": 5 }
```

Call the TypeScript worker (which calls Python under the hood):

```bash
iii trigger --function-id='math::add_two_numbers' --payload='{"a": 10, "b": 20}'
```

```json
{ "c": 30 }
```

## Architecture

```text
┌──────────────┐          ┌──────────────────┐
│  iii trigger  │◀────────▶│    iii engine     │
│  (CLI)        │   WS     │   :49134          │
└──────────────┘          └──────┬───────┬────┘
                                 │       │
                          WS     │       │  WS
                                 ▼       ▼
                        ┌────────────┐  ┌──────────────┐
                        │math-worker │  │caller-worker │
                        │(Python)    │  │(TypeScript)  │
                        │math::add   │  │math::add_two │
                        │            │  │  _numbers    │
                        └────────────┘  └──────────────┘
```

## Next Steps

- Open `workers/math-worker/math_worker.py` and `workers/caller-worker/src/worker.ts` to see how functions are registered and called across languages.
- Read the [iii docs](https://iii.dev/docs) to learn how to use triggers, queues, state, and more.
