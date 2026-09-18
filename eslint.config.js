import js from '@eslint/js'
import ts from 'typescript-eslint'
import vue from 'eslint-plugin-vue'
export default [
  { ignores: ['dist/**','node_modules/**','.npm-cache/**'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...vue.configs['flat/essential'],
  { files: ['**/*.vue'], languageOptions: { parserOptions: { parser: ts.parser } } },
  { languageOptions: { globals: { Event:'readonly', FocusEvent:'readonly', location:'readonly', window:'readonly', document:'readonly', localStorage:'readonly', navigator:'readonly', console:'readonly', setTimeout:'readonly', clearTimeout:'readonly', setInterval:'readonly', clearInterval:'readonly', crypto:'readonly', BroadcastChannel:'readonly', Storage:'readonly', HTMLElement:'readonly', HTMLInputElement:'readonly', HTMLTextAreaElement:'readonly', KeyboardEvent:'readonly', URLSearchParams:'readonly' } }, rules: { 'vue/multi-word-component-names':'off', '@typescript-eslint/no-explicit-any':'error' } }
]
