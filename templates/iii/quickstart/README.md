# Welcome to iii

This quickstart demonstrates how iii transforms all backend functionality
into workers that can operate, scale, and error independently of each other.

The `workers/` folder contains a TypeScript **client** and **payment-worker**,
a Rust **compute-worker**, and a Python **data-worker**. Each runs in an
isolated microVM sandbox via `iii worker dev`.

Check `workers/client/src/worker.ts` to see how the orchestration works.

## Prerequisites

- **iii** installed (https://iii.dev/docs)
- **macOS Apple Silicon** or **Linux with KVM** (required for `iii worker dev`)

> **Windows users:** Run inside WSL 2 with KVM support enabled.

## Quick Start

### 1. Start the engine

```bash
iii
```

### 2. Start each worker in a separate terminal

```bash
iii worker dev ./workers/client
```

```bash
iii worker dev ./workers/payment-worker
```

```bash
iii worker dev ./workers/data-worker
```

```bash
iii worker dev ./workers/compute-worker
```

Only the client worker is required for this demo. The application still works
with whichever other workers are running and reports errors for any that are missing.

### 3. Try it out

```bash
curl -X POST http://localhost:3111/orchestrate \
  -H "Content-Type: application/json" \
  -d '{"data":{"message":"hello from client"},"n":42}' | jq
```

With all workers running the output looks like:

```json
{
  "client": "ok",
  "computeWorker": { "input": 42, "result": 84, "source": "compute-worker" },
  "dataWorker": {
    "keys": ["message"],
    "source": "data-worker",
    "transformed": { "message": "hello from client" }
  },
  "errors": [],
  "externalWorker": {
    "body": { "message": "Payment recorded" },
    "source": "payment-worker",
    "status": 200
  }
}
```

### 4. Try the iii Console

```bash
iii console
```

Open http://localhost:3113/ to see logs, traces, and runtime state.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      iii Engine                             │
│           (port 49134 (engine), 3111 (http))                │
└──────────┬──────────┬──────────┬──────────┬─────────────────┘
           │          │          │          │
    ┌──────┴───┐ ┌────┴────┐ ┌───┴───┐ ┌────┴─────┐
    │  Client  │ │ Compute │ │ Data  │ │ Payment  │
    │   (TS)   │ │  (Rust) │ │  (Py) │ │   (TS)   │
    └──────────┘ └─────────┘ └───────┘ └──────────┘
```

Workers communicate via the engine regardless of language. Functions can be
triggered across processes, languages, and application boundaries.

## Next Steps

- Explore `workers/client/src/worker.ts` to understand the orchestration
- Edit `config.yaml` to customize engine workers
- Visit https://iii.dev/docs/concepts to learn more about Workers, Triggers, and Functions
