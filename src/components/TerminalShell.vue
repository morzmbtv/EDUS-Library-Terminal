<template>
  <div class="terminal-shell">
    <header class="shell-header"><slot name="header" /></header>
    <div class="shell-main" id="main-content"><slot /></div>
    <footer class="shell-footer"><slot name="footer" /></footer>
  </div>
</template>

<style scoped>
.terminal-shell { min-height: 100dvh; height: 100dvh; display: flex; flex-direction: column; background: var(--background, #F8FBFE); color: var(--edus-navy, #0B1A4D); }
.shell-header, .shell-footer { flex: 0 0 auto; }
.shell-main { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.shell-footer { padding-bottom: env(safe-area-inset-bottom, 0); }
@media(max-width:760px){.terminal-shell{height:auto;min-height:100dvh}.shell-main{flex:none;min-height:auto}}
</style>
