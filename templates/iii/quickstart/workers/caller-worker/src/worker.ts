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

console.log('Caller worker started - listening for calls');
