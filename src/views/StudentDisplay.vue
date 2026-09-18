<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { BookOpen, Check, ShieldCheck } from '@lucide/vue'
type DisplayState={type:'state';state:'idle'|'active'|'success';operation:'issue'|'accept';quantity:number;reader:string;books:{name:string;quantity:number}[];expiresAt:number}
const idle=():DisplayState=>({type:'state',state:'idle',operation:'issue',quantity:0,reader:'',books:[],expiresAt:0})
const state=ref<DisplayState>(idle())
const channel=typeof BroadcastChannel==='undefined'?null:new BroadcastChannel('edus-library-display')
let expiryTimer:ReturnType<typeof setInterval>
onMounted(()=>{
  channel?.addEventListener('message',e=>{const d=e.data;if(d?.type==='state'&&['idle','active','success'].includes(d.state)&&typeof d.expiresAt==='number'&&Array.isArray(d.books)){state.value=d.expiresAt<Date.now()?idle():d}})
  channel?.postMessage({type:'request'})
  expiryTimer=setInterval(()=>{if(state.value.expiresAt&&Date.now()>state.value.expiresAt)state.value=idle()},1000)
})
onUnmounted(()=>{channel?.close();clearInterval(expiryTimer)})
</script>
<template><main class="student-display"><header><img src="/assets/edus-logo.png" alt="EDUS" /><span>Библиотека</span></header><section v-if="state.state==='idle'" class="display-wait"><BookOpen :size="80" :stroke-width="1.3" /><h1>Добро пожаловать<br />в библиотеку</h1><p>Библиотекарь поможет получить<br />или вернуть книги.</p></section><section v-else class="display-operation"><div v-if="state.state==='success'" class="success-mark"><Check /></div><h1>{{state.state==='success'?(state.operation==='issue'?'Книги выданы':'Книги приняты'):(state.operation==='issue'?'Выдаём книги':'Принимаем книги')}}</h1><p v-if="state.reader">{{state.reader}}</p><div class="display-books"><div v-for="(book,i) in state.books" :key="i"><BookOpen :size="28" /><span>{{book.name}}</span><strong>{{book.quantity}} шт.</strong></div></div><p>{{state.quantity}} {{state.quantity===1?'книга':state.quantity<5?'книги':'книг'}}</p><span v-if="state.state==='success'" class="muted">{{state.operation==='issue'?'Приятного чтения!':'Спасибо, что вернули книги.'}}</span></section><footer><span><ShieldCheck :size="22" />Экран читателя</span><span>Демонстрационная версия</span></footer></main></template>
<style scoped>
.student-display{min-height:100dvh;display:flex;flex-direction:column;padding:40px 64px}header{display:flex;align-items:center;gap:32px;font-size:26px}header img{width:176px;height:auto}header span{border-left:1px solid var(--line);padding-left:32px}.display-wait,.display-operation{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:24px;text-align:center;padding:40px 0}.display-wait>svg{color:var(--edus-blue);margin-bottom:16px}h1{font-size:48px;line-height:1.3}p{font-size:28px;color:var(--muted)}footer{display:flex;justify-content:space-between;color:var(--muted);font-size:16px}footer span{display:flex;align-items:center;gap:10px}.display-books{width:min(900px,100%);max-height:38vh;overflow:auto;border-top:1px solid var(--line)}.display-books>div{display:flex;align-items:center;gap:24px;border-bottom:1px solid var(--line);padding:20px;text-align:left;font-size:24px}.display-books span{flex:1}.display-books svg{color:var(--edus-blue);flex-shrink:0}.display-books strong{white-space:nowrap;font-size:20px}@media(max-width:700px){.student-display{padding:24px}h1{font-size:34px}p{font-size:22px}header img{width:120px}header{gap:20px;font-size:20px}header span{padding-left:20px}footer{flex-wrap:wrap;gap:12px}.display-books>div{font-size:20px;gap:12px;padding:16px 0}}
</style>
