import asyncio
import os
from iii import register_worker, InitOptions, Logger

iii = register_worker(
    os.environ.get("III_URL", "ws://localhost:49134"),
    InitOptions(worker_name="math-worker"),
)
logger = Logger()


def add_handler(payload: dict) -> dict:
    a = payload.get("a", 0)
    b = payload.get("b", 0)
    logger.info(f"math::add called in Python with a={a}, b={b}")
    return {"c": a + b}


iii.register_function("math::add", add_handler)

print("Math worker started - listening for calls")

loop = asyncio.new_event_loop()
try:
    loop.run_forever()
except KeyboardInterrupt:
    pass
