<template>
  <div class="action-bar"><div class="action-left"><slot /></div><div class="action-right"><slot name="right" /></div></div>
</template>

<style scoped>
.action-bar { display: flex; justify-content: space-between; align-items: center; gap: 24px; padding: 24px 48px; border-top: 1px solid var(--line, #DCE6F0); background: var(--surface, #fff); flex: 0 0 auto; }
.action-left, .action-right { display: flex; align-items: center; gap: 16px; min-width: 0; }.action-right { margin-left: auto; justify-content: flex-end; }
@media (max-width: 1400px) { .action-bar { padding: 20px 32px; } }
@media (max-width: 700px) { .action-bar { padding: 16px; gap: 16px; flex-wrap: wrap; }.action-left, .action-right { flex-wrap: wrap; }.action-right { flex: 1; } }
@media (min-width: 901px) and (max-width: 1300px) and (max-height: 820px) { .action-bar { min-height: 88px; padding: 12px 24px; gap: 16px; } }
</style>
