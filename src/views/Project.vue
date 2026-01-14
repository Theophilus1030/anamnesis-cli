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

// --- 新增：重命名函数 ---
async function renamePage(page: Page) {
  // 使用 prompt 让用户输入新名字，默认填入旧名字
  const newName = prompt('请输入新的页面名称', page.name)

  // 如果用户点了确定(newName存在) 且 名字确实变了
  if (newName && newName.trim() !== '' && newName !== page.name) {
    try {
      // 调用后端命令
      await invoke('update_page_name', { id: page.id, name: newName })
      // 刷新列表
      await loadPages()
    } catch (e) {
      alert('重命名失败: ' + e)
    }
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
        <button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm" @click="createPage">
          创建页
        </button>
        <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-sm" @click="goBack">
          返回首页
        </button>
      </div>

      <div v-if="pages.length === 0" class="text-zinc-500 text-center py-16">
        暂无页面，点击上方按钮创建
      </div>

      <div class="grid grid-cols-3 gap-4">
        <div
          v-for="page in pages"
          :key="page.id"
          class="bg-zinc-900 rounded-lg overflow-hidden flex flex-col" 
        >
          <div
            class="p-4 hover:bg-zinc-800 cursor-pointer flex-1"
            @click="goToPage(page.id)"
          >
            <span class="text-lg font-medium">{{ page.name }}</span>
            <div class="text-zinc-500 text-xs mt-2">
              {{ new Date(page.created_at).toLocaleString() }}
            </div>
          </div>

          <div class="flex border-t border-zinc-800">
            <button
              class="flex-1 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 hover:text-white text-sm transition-colors border-r border-zinc-700"
              @click.stop="renamePage(page)"
            >
              重命名
            </button>
            
            <button
              class="flex-1 px-4 py-2 bg-zinc-800 hover:bg-red-900/50 text-red-400 hover:text-red-300 text-sm transition-colors"
              @click.stop="deletePage(page.id)"
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>