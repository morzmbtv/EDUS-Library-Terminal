// Node does not run Vite's compile-time `define` replacement.  The test
// runner opts into the same demonstration adapter that Vite serves in dev.
globalThis.__EDUS_DEMO_RUNTIME__ = true
globalThis.__EDUS_TERMINAL_TEST_RUNTIME__ = false
