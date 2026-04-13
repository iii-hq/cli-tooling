# Cross-Language Math

Two workers — one Python, one TypeScript — demonstrating cross-language function calls via the iii engine.

For a detailed walkthrough follow the [Quickstart tutorial](https://iii.dev/docs/quickstart).

## What's Inside

| Worker          | Language   | Function                   | Does                                                |
| --------------- | ---------- | -------------------------- | --------------------------------------------------- |
| `math-worker`   | Python     | `math::add`                | Returns `{ c: a + b }` (+ running total with state) |
| `caller-worker` | TypeScript | `math::add_two_numbers`    | Calls `math::add` and returns the result            |
| `iii-state`     | Built-in   | `state::get`, `state::set` | Persistent key-value store                          |
| `iii-http`      | Built-in   | —                          | Exposes functions as REST endpoints on `:3111`      |

## Quick Start

### 1. Start the engine

```bash
iii
```

### 2. Start the workers

```bash
iii worker add ./workers/math-worker
iii worker add ./workers/caller-worker
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

### 4. Add state (uncomment code in `math_worker.py`)

```bash
iii worker add iii-state
```

Uncomment the state block in `workers/math-worker/math_worker.py` and restart the worker:

```bash
iii worker stop math-worker
iii worker start math-worker
```

Then call:

```bash
iii trigger --function-id='math::add' --payload='{"a": 2, "b": 3}'
```

```json
{ "c": 5, "running_total": 5 }
```

```bash
iii trigger --function-id='math::add' --payload='{"a": 10, "b": 20}'
```

```json
{ "c": 30, "running_total": 35 }
```

### 5. Add HTTP endpoints (uncomment code in `worker.ts`)

```bash
iii worker add iii-http
```

Uncomment the trigger registrations in `workers/caller-worker/src/worker.ts` and restart the worker:

```bash
iii worker stop caller-worker
iii worker start caller-worker
```

Then:

```bash
curl -X POST http://localhost:3111/math/add -H 'Content-Type: application/json' -d '{"a": 4, "b": 6}'
```

```json
{ "c": 10, "running_total": 45 }
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
