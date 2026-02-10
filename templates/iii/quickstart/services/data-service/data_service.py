# Data Service - Data validation and transformation
# Demonstrates: registerFunction with Pydantic validation

import asyncio
from iii import III, InitOptions, get_context
from pydantic import BaseModel

class TransformInput(BaseModel):
    data: dict

iii = III(
    "ws://localhost:49134",
    InitOptions(worker_name="data-service")
)

@iii.function("data-service.transform")
async def transform_handler(input: dict) -> dict:
    ctx = get_context()
    validated = TransformInput(**input)
    ctx.logger.info("Processing data with data-service...")
    await asyncio.sleep(0.5)
    
    return {
        "transformed": validated.data,
        "keys": list(validated.data.keys()),
        "source": "data-service"
    }

async def main():
    await iii.connect()
    print("Data service started - listening for calls")
    await asyncio.Future()

if __name__ == "__main__":
    asyncio.run(main())
