<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()
const router = useRouter()
const projectId = route.params.id as string

interface Page {
  id: string
  project_id: string
  name: string
  created_at: number
}

const pages = ref<Page[]>([])

async function loadPages() {
  pages.value = await invoke('get_pages', { projectId })
}

async function createPage() {
  const name = prompt('请输入页面名称')
  if (name) {
    const id = crypto.randomUUID()
    await invoke('create_page', { id, projectId, name })
    await loadPages()
  }
}

async function deletePage(id: string) {
  if (confirm('确定要删除这个页面吗？')) {
    await invoke('delete_page', { id })
    await loadPages()
  }
}

function goToPage(pageId: string) {
  router.push(`/project/${projectId}/page/${pageId}`)
}

function goBack() {
  router.push('/')
}

onMounted(() => {
  loadPages()
})
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 p-8">
    <div class="max-w-4xl mx-auto">
      <p class="mb-4">这是项目页面，项目ID：{{ projectId }}</p>

      <div class="flex gap-2 mb-8">
        <button
          class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm"
          @click="createPage"
        >
          创建页
        </button>
        <button
          class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-sm"
          @click="goBack"
        >
          返回首页
        </button>
      </div>

      <div v-if="pages.length === 0" class="text-zinc-500 text-center py-16">
        暂无页面，点击上方按钮创建
      </div>

      <div v-else class="grid grid-cols-3 gap-4">
        <div
          v-for="page in pages"
          :key="page.id"
          class="bg-zinc-900 rounded-lg overflow-hidden"
        >
          <div
            class="p-4 hover:bg-zinc-800 cursor-pointer"
            @click="goToPage(page.id)"
          >
            <span>{{ page.name }}</span>
          </div>
          <button
            class="w-full px-4 py-2 bg-red-600/20 hover:bg-red-600 text-red-400 hover:text-white text-sm transition-colors"
            @click="deletePage(page.id)"
          >
            删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>