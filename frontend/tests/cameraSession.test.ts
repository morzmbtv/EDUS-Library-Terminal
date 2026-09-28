import test from 'node:test';
import assert from 'node:assert/strict';
import { createCameraSession } from '../src/shared/lib/cameraSession.ts';
function fixture() {
  let stopped = 0;
  const stream = {
    getTracks: () => [
      {
        stop: () => {
          stopped++;
        },
      },
    ],
  } as unknown as MediaStream;
  return {
    stream,
    get stopped() {
      return stopped;
    },
  };
}
test('camera stops on navigation and ignores late permission response', async () => {
  const value = fixture();
  let finish!: (value: MediaStream) => void;
  const camera = createCameraSession(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const request = camera.start();
  camera.stop();
  finish(value.stream);
  assert.equal(await request, null);
  assert.equal(value.stopped, 1);
});
test('camera switching releases previous tracks and stops on disposal', async () => {
  const first = fixture(),
    second = fixture();
  let calls = 0;
  const camera = createCameraSession(async () => (calls++ === 0 ? first.stream : second.stream));
  assert.equal(await camera.start(), first.stream);
  assert.equal(await camera.start(), second.stream);
  assert.equal(first.stopped, 1);
  camera.stop();
  assert.equal(second.stopped, 1);
});
