# Welcome to iii

This is the iii quickstart project, it's intended to demonstrate how iii works, teach the basics of using iii, and show the power of having a central coordinator.

One of the first the first things you might notice is that the services/ folder contains `client` and `payment-service` TypeScript projects, a Rust `compute-service`, and a Python `data-service`. For demonstration these services are all in the same project. The languages for each service, and project structure are chosen only for the convenience of demonstration.

These services can easily be located in their own projects, written in other languages, or already running on servers which you only have API access to.

## Prerequisites

### Required

- **iii engine** installed (see https://iii.dev/docs for details)
- **Node.js** (for client, and payment-service)

### Optional

- **Python 3** (for data-service)
- **Rust/Cargo** (for compute-service)

## Quick Start

### 1. Start the iii engine

```bash
iii -c iii-config.yaml
```

### 2. Start the services

At a minimum you will need to start the Client and at least one of the other Services to see a result.

Run each in a separate terminal:

```bash
# Client (TypeScript orchestrator)
cd services/client
npm install
npm run dev

# Compute Service (Rust)
cd services/compute-service
cargo run

# Data Service (Python)
cd services/data-service
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python data_service.py

# Payment Service (TypeScript)
cd services/payment-service
npm install
npm run dev
```

### 3. Try it out

```bash
curl http://localhost:3111/orchestrate
```

If all services are running the output will look like the below:

```json
{
  "client": "ok",
  "computeService": { "input": 42, "result": 84, "source": "compute-service" },
  "dataService": {
    "keys": ["message"],
    "source": "data-service",
    "transformed": { "message": "hello from client" }
  },
  "externalService": {
    "body": { "message": "Payment recorded" },
    "source": "payment-service",
    "status": 200
  },
  "errors": []
}
```

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

Services communicate via the iii engine regardless of language and with iii performing the central orchestration it is possible to call functions across processes, languages, services, domains, and application boundaries.
