# Data Service - Data validation and transformation
# Demonstrates: register_function with Pydantic validation

import asyncio
import os
from iii import register_worker, InitOptions, Logger
from pydantic import BaseModel, ValidationError

class TransformInput(BaseModel):
    data: dict

iii = register_worker(
    os.environ.get("III_BRIDGE_URL", "ws://localhost:49134"),
    InitOptions(worker_name="data-service")
)
logger = Logger()

# Decorators are available
# @iii.register_function({"id": "data-service::transform"})
def transform_handler(payload: dict) -> dict:
    try:
        validated = TransformInput.model_validate(payload)
    except ValidationError as e:
        logger.error(f"Validation error: {e}")
        return {"error": "Invalid payload", "details": e.errors()}

    worker_version = iii.trigger({"function_id": "state::get", "payload": {"scope": "shared", "key": "WORKER_VERSION"}})

    logger.info("Processing data with data-service...")

    return {
        "transformed": validated.data,
        "keys": list(validated.data.keys()),
        "source": "data-service",
        "worker-version": f"worker version {worker_version}"
    }

iii.register_function({"id": "data-service::transform"}, transform_handler)

print("Data service started - listening for calls")

loop = asyncio.new_event_loop()
try:
    loop.run_forever()
except KeyboardInterrupt:
    pass
