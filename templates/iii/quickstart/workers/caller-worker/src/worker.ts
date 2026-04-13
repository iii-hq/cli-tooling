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
        "You've connected two workers and they're interoperating seamlessly, now let's add a few more workers to expand this project's functionality.",
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
