<script setup lang="ts">
import { ref, watch } from 'vue'

interface Sentence {
  id: string
  line_ids: string[]
  type: 'text' | 'title' | 'note'
  normalized: string
  en: string
  zh: string
}

const props = defineProps<{
  sentence: Sentence | null
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', data: { normalized: string; en: string; zh: string; type: 'text' | 'title' | 'note' }): void
}>()

const form = ref({
  normalized: '',
  en: '',
  zh: '',
  type: 'text' as 'text' | 'title' | 'note'
})

// 监听 sentence 变化，更新表单
watch(() => props.sentence, (newVal) => {
  if (newVal) {
    form.value = {
      normalized: newVal.normalized || '',
      en: newVal.en || '',
      zh: newVal.zh || '',
      type: newVal.type || 'text'
    }
  }
}, { immediate: true })

function handleSave() {
  emit('save', { ...form.value })
}

function handleClose() {
  emit('close')
}
</script>

<template>
  <div 
    v-if="visible" 
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    @click.self="handleClose"
  >
    <div class="bg-zinc-900 rounded-lg p-6 w-150 max-h-[80vh] overflow-y-auto">
      <h2 class="text-lg font-bold mb-4">编辑句子</h2>

      <div class="space-y-4">
        <!-- 句子类型 -->
        <div>
          <label class="block text-sm text-zinc-400 mb-2">句子类型</label>
          <div class="flex gap-4">
            <label class="flex items-center gap-2 cursor-pointer">
              <input 
                type="radio" 
                v-model="form.type" 
                value="text"
                class="w-4 h-4 accent-blue-500"
              />
              <span class="text-zinc-300">正文</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input 
                type="radio" 
                v-model="form.type" 
                value="title"
                class="w-4 h-4 accent-yellow-500"
              />
              <span class="text-zinc-300">标题</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input 
                type="radio" 
                v-model="form.type" 
                value="note"
                class="w-4 h-4 accent-green-500"
              />
              <span class="text-zinc-300">注释</span>
            </label>
          </div>
        </div>

        <!-- 正则原文 -->
        <div>
          <label class="block text-sm text-zinc-400 mb-1">正则原文 (Normalized)</label>
          <textarea
            v-model="form.normalized"
            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
            rows="3"
            placeholder="输入正则化后的原文..."
          />
        </div>

        <!-- 英文翻译 -->
        <div>
          <label class="block text-sm text-zinc-400 mb-1">英文翻译 (English)</label>
          <textarea
            v-model="form.en"
            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
            rows="3"
            placeholder="English translation..."
          />
        </div>

        <!-- 中文翻译 -->
        <div>
          <label class="block text-sm text-zinc-400 mb-1">中文翻译 (Chinese)</label>
          <textarea
            v-model="form.zh"
            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
            rows="3"
            placeholder="中文翻译..."
          />
        </div>
      </div>

      <div class="flex justify-end gap-2 mt-6">
        <button 
          class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-sm" 
          @click="handleClose"
        >
          取消
        </button>
        <button 
          class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm" 
          @click="handleSave"
        >
          保存
        </button>
      </div>
    </div>
  </div>
</template>