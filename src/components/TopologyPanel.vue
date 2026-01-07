<script setup lang="ts">
interface Sentence {
  id: string
  line_ids: string[]
  type: 'text' | 'title' | 'note'
  normalized: string
  en: string
  zh: string
}

defineProps<{
  sentences: Sentence[]
}>()
</script>

<template>
  <div class="h-full bg-zinc-900 p-4 overflow-y-auto">
    <div class="text-zinc-400 text-sm mb-4">拓扑关系图</div>
    <div class="text-zinc-500 text-sm">
      共 {{ sentences.length }} 个槽位（待实现拖拽功能）
    </div>
  </div>
</template>