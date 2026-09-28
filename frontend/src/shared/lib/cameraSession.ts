/** Camera frames stay in the MediaStream preview. No capture, upload or persistence. */
export function createCameraSession(acquire: () => Promise<MediaStream>) {
  let generation = 0;
  let active: MediaStream | null = null;
  function stop() {
    generation++;
    active?.getTracks().forEach((track) => track.stop());
    active = null;
  }
  async function start(): Promise<MediaStream | null> {
    stop();
    const attempt = generation;
    const stream = await acquire();
    if (attempt !== generation) {
      stream.getTracks().forEach((track) => track.stop());
      return null;
    }
    active = stream;
    return stream;
  }
  return { start, stop };
}
