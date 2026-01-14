<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import draggable from 'vuedraggable'

// --- 接口定义 ---
interface Sentence {
  id: string
  line_ids: string[]
  type: 'text' | 'title' | 'note'
  normalized: string
  en: string
  zh: string
}

interface Glyph {
  id: string
  content: string
}

interface Line {
  id: string
  glyphs: Glyph[]
}

const props = defineProps<{
  sentences: Sentence[]
  lines: Line[]
}>()

const emit = defineEmits<{
  (e: 'hover-line', id: string): void
  (e: 'leave-line'): void
  (e: 'update', sentences: Sentence[]): void
}>()

// --- 本地状态 ---
const localSentences = ref<Sentence[]>([])
const localUnassignedIds = ref<string[]>([])
const ghostSlots = ref<string[][]>([[]])
// --- 拖拽配置 ---
const dragOptions = {
  group: { name: 'lines', pull: true, put: true },
  animation: 150,
  forceFallback: true,
  fallbackClass: 'drag-fallback',
  fallbackTolerance: 3,
  delay: 0,
  scroll: true,
  scrollSensitivity: 30
}

// 幽灵槽位的配置：只接收，不保留
const ghostSlotOptions = {
  group: { name: 'lines', pull: false, put: true },
  animation: 150,
  forceFallback: true,
  fallbackClass: 'drag-fallback',
}

// --- 辅助函数 ---
function getItemKey(key: string) {
  return key
}

function getLineIndex(lineId: string): number {
  return props.lines.findIndex(l => l.id === lineId) + 1
}

function getLineContent(lineId: string): string {
  const line = props.lines.find(l => l.id === lineId)
  if (!line) return ''
  const text = line.glyphs.map(g => g.content).join('')
  return text.length > 5 ? text.slice(0, 5) + '...' : text
}

// --- 数据同步 ---
watch(() => [props.sentences, props.lines], () => {
  localSentences.value = JSON.parse(JSON.stringify(props.sentences))
  const assignedIds = new Set<string>()
  props.sentences.forEach(s => s.line_ids.forEach(id => assignedIds.add(id)))
  localUnassignedIds.value = props.lines
    .filter(l => !assignedIds.has(l.id))
    .map(l => l.id)
}, { immediate: true, deep: true })

// 单独监听句子数量变化，更新幽灵槽位数组
watch(() => localSentences.value.length, (len) => {
  ghostSlots.value = Array(len + 1).fill(null).map(() => [])
}, { immediate: true })

// --- 拖拽逻辑 ---
function emitUpdate() {
  emit('update', JSON.parse(JSON.stringify(localSentences.value)))
}

function onDragEnd() {
  checkEmptySlots()
  emitUpdate()
}

function checkEmptySlots() {
  let hasChange = false
  for (let i = localSentences.value.length - 1; i >= 0; i--) {
    const s = localSentences.value[i]
    if (s.line_ids.length === 0) {
      if (!s.normalized && !s.en && !s.zh) {
        localSentences.value.splice(i, 1)
        hasChange = true
      }
    }
  }
  if (hasChange) emitUpdate()
}


function onGhostSlotChange(evt: any, insertIndex: number) {
  if (evt.added) {
    const lineId = evt.added.element

    // 创建新句子
    const newSentence: Sentence = {
      id: crypto.randomUUID(),
      line_ids: [lineId],
      type: 'text',
      normalized: '',
      en: '',
      zh: ''
    }

    // 插入到指定位置
    localSentences.value.splice(insertIndex, 0, newSentence)

    // 清空幽灵槽位
    nextTick(() => {
      // 清空所有幽灵槽位
      ghostSlots.value = ghostSlots.value.map(() => [])
      emitUpdate()
    })
  }
}

// 样式映射
const typeColors = {
  text: 'border-blue-500/50 bg-blue-500/5',
  title: 'border-yellow-500/50 bg-yellow-500/5',
  note: 'border-green-500/50 bg-green-500/5'
}

// --- 全局防干扰 ---
const preventDefault = (e: Event) => e.preventDefault()
onMounted(() => {
  window.addEventListener('dragover', preventDefault)
  window.addEventListener('drop', preventDefault)
})
onUnmounted(() => {
  window.removeEventListener('dragover', preventDefault)
  window.removeEventListener('drop', preventDefault)
})
</script>

<template>
  <div class="h-full bg-zinc-950 flex flex-col border-l border-zinc-800">
    
    <div class="h-12 flex items-center px-4 border-b border-zinc-800 bg-zinc-900 shrink-0 justify-between z-10">
      <span class="text-zinc-100 font-medium text-base">拓扑结构</span>
      <span class="text-zinc-500 text-sm bg-zinc-800 px-1.5 py-0.5 rounded">
        {{ localSentences.length }} 句
      </span>
    </div>

    <div class="p-4 border-b border-zinc-800 bg-zinc-950 shrink-0 z-10 shadow-sm">
      <div class="text-sm text-zinc-500 mb-2 font-medium flex justify-between">
        <span>待分配 (Inbox)</span>
        <span class="text-[10px]">{{ localUnassignedIds.length }}</span>
      </div>

      <draggable 
        v-model="localUnassignedIds" 
        v-bind="dragOptions" 
        :item-key="getItemKey" 
        tag="div"
        handle=".drag-handle"
        class="flex flex-wrap gap-2 p-3 bg-zinc-900/50 rounded-lg border border-dashed border-zinc-700 min-h-15 max-h-40 overflow-y-auto"
        @end="onDragEnd"
      >
        <template #item="{ element: lineId }">
          <div 
            class="drag-item h-8 px-2 bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded text-sm flex items-center gap-1 shadow-sm select-none border border-zinc-600"
            :title="getLineContent(lineId)" 
            @mouseenter="emit('hover-line', lineId)" 
            @mouseleave="emit('leave-line')"
          >
            <span class="drag-handle cursor-grab active:cursor-grabbing text-zinc-500 hover:text-white px-1 font-bold text-sm">⋮⋮</span>
            <span>L{{ getLineIndex(lineId) }}</span>
          </div>
        </template>
      </draggable>
    </div>

    <div class="flex-1 overflow-y-auto p-4 bg-zinc-950">
      
      <div class="text-sm text-zinc-500 mb-2 font-medium sticky top-0 bg-zinc-950 py-1 z-0">
        句子流 (Sentence Flow)
      </div>

      <div class="ghost-slot h-8 border-2 border-dashed border-zinc-700 hover:border-yellow-500/50 hover:bg-yellow-500/10 rounded mb-2 flex items-center justify-center text-xs text-zinc-600 hover:text-yellow-500 transition-colors">
        <draggable 
          v-if="ghostSlots[0]" 
          v-model="ghostSlots[0]" 
          v-bind="ghostSlotOptions" 
          :item-key="getItemKey"
          tag="div" 
          class="w-full h-full flex items-center justify-center"
          @change="(evt: any) => onGhostSlotChange(evt, 0)"
        >
          <template #item="{ element }">
            <div class="hidden">{{ element }}</div>
          </template>
          <template #header>
            <span class="pointer-events-none">拖入新建首句</span>
          </template>
        </draggable>
      </div>

      <div v-for="(sentence, index) in localSentences" :key="sentence.id">
        <div :class="['rounded border-l-4 p-3 transition-colors mb-2', typeColors[sentence.type] || typeColors.text]">
          <div class="flex justify-between items-center mb-2 select-none">
            <span class="text-sm text-zinc-500 font-mono">#{{ index + 1 }}</span>
            <span class="text-[10px] text-zinc-500 px-1.5 py-0.5 bg-black/20 rounded">
              {{ sentence.type }}
            </span>
          </div>

          <draggable 
            v-model="sentence.line_ids" 
            v-bind="dragOptions" 
            :item-key="getItemKey" 
            tag="div"
            handle=".drag-handle" 
            class="flex flex-wrap items-center gap-2 min-h-8" 
            @end="onDragEnd"
          >
            <template #item="{ element: lineId }">
              <div 
                class="drag-item h-8 px-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm flex items-center gap-1 shadow-sm select-none border border-zinc-700 ring-1 ring-inset ring-transparent hover:ring-blue-500/50"
                :title="getLineContent(lineId)" 
                @mouseenter="emit('hover-line', lineId)"
                @mouseleave="emit('leave-line')"
              >
                <span class="drag-handle cursor-grab active:cursor-grabbing text-zinc-500 hover:text-white px-1 font-bold text-sm">⋮⋮</span>
                <span>L{{ getLineIndex(lineId) }}</span>
              </div>
            </template>
          </draggable>
        </div>

        <div class="ghost-slot h-8 border-2 border-dashed border-zinc-700 hover:border-yellow-500/50 hover:bg-yellow-500/10 rounded mb-2 flex items-center justify-center text-xs text-zinc-600 hover:text-yellow-500 transition-colors">
          <draggable 
            v-if="ghostSlots[index + 1]" 
            v-model="ghostSlots[index + 1]" 
            v-bind="ghostSlotOptions" 
            :item-key="getItemKey" 
            tag="div"
            class="w-full h-full flex items-center justify-center"
            @change="(evt: any) => onGhostSlotChange(evt, index + 1)"
          >
            <template #item="{ element }">
              <div class="hidden">{{ element }}</div>
            </template>
            <template #header>
              <span class="pointer-events-none">拖入新建句子</span>
            </template>
          </draggable>
        </div>
      </div>
      
    </div>
  </div>
</template>

<style scoped>
.drag-fallback {
  opacity: 1 !important;
  background: #27272a;
  border: 1px solid #60a5fa;
  border-radius: 4px;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5);
  transform: rotate(3deg);
  pointer-events: none;
  z-index: 9999;
}

.sortable-ghost {
  opacity: 0.2;
  background: #4f46e5;
}

.sortable-drag {
  opacity: 0;
}

.ghost-slot {
  min-height: 32px;
}
</style>