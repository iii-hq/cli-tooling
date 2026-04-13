import { registerWorker, Logger } from 'iii-sdk';

const iii = registerWorker(process.env.III_URL ?? 'ws://localhost:49134');
const logger = new Logger();

iii.registerFunction(
  'math::add_two_numbers',
  async (payload: { a: number; b: number }) => {
    logger.info('math::add_two_numbers called in TypeScript', payload);

    const result = await iii.trigger({
      function_id: 'math::add',
      payload,
    });

    return {
      ...result,
      success:
        'Success! Open workers/caller-worker/src/worker.ts and workers/caller-worker/iii.worker.yaml to learn how this worked, or visit https://iii.dev/docs/concepts',
    };
  },
);

// --- Uncomment after: iii worker add iii-http ---
// iii.registerTrigger({
//   type: 'http',
//   function_id: 'math::add',
//   config: { api_path: '/math/add', http_method: 'POST' },
// });
// iii.registerTrigger({
//   type: 'http',
//   function_id: 'math::add_two_numbers',
//   config: { api_path: '/math/add-two-numbers', http_method: 'POST' },
// });

console.log('Caller worker started - listening for calls');
