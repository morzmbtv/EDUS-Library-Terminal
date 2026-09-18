import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

/**
 * `terminal-test` is an explicit release target. It is deliberately separate
 * from both local development and the production bundle: production never
 * falls back to the local test adapter.
 */
export default defineConfig(({ mode }) => {
  const terminalTest = mode === 'terminal-test'
  return {
    define: {
      __EDUS_DEMO_RUNTIME__: JSON.stringify(mode === 'development'),
      __EDUS_TERMINAL_TEST_RUNTIME__: JSON.stringify(terminalTest),
    },
    plugins: [vue()],
    server: { port: 5173, strictPort: true },
    build: { target: 'es2022', outDir: terminalTest ? 'dist-terminal-test' : 'dist', emptyOutDir: true },
  }
})
