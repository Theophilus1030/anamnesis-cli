<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()

interface Project {
  id: string
  name: string
  created_at: number
}

const projects = ref<Project[]>([])

async function loadProjects() {
  projects.value = await invoke('get_projects')
}

async function createProject() {
  const name = prompt('请输入项目名称')
  if (name) {
    const id = crypto.randomUUID()
    await invoke('create_project', { id, name })
    await loadProjects()
  }
}

async function deleteProject(id: string) {
  await invoke('delete_project', { id })
  await loadProjects()
}

// --- 新增：重命名函数 ---
async function renameProject(project: Project) {
  const newName = prompt('请输入新的项目名称', project.name)

  if (newName && newName.trim() !== '' && newName !== project.name) {
    try {
      await invoke('update_project_name', { id: project.id, name: newName })
      await loadProjects()
    } catch (e) {
      alert('重命名失败: ' + e)
    }
  }
}


function goToProject(id: string) {
  router.push(`/project/${id}`)
}

function goToTest() {
  router.push('/test')
}

onMounted(() => {
  loadProjects()
})
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 p-8">
    <div class="max-w-4xl mx-auto">
      <div class="flex items-center justify-between mb-8">
        <h1 class="text-2xl font-bold">项目列表</h1>
        <div class="flex gap-2">
          <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-sm" @click="goToTest">
            测试页面
          </button>
          <button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm" @click="createProject">
            创建项目
          </button>
        </div>
      </div>

      <div v-if="projects.length === 0" class="text-zinc-500 text-center py-16">
        暂无项目，点击上方按钮创建
      </div>

      <div v-else class="grid gap-4">
        <div v-for="project in projects" :key="project.id"
          class="group relative p-4 bg-zinc-900 rounded-lg hover:bg-zinc-800 cursor-pointer transition-colors"
          @click="goToProject(project.id)">
          <div class="flex items-center justify-between">
            <span class="font-medium text-lg">{{ project.name }}</span>
            <span class="text-zinc-500 text-sm">
              {{ new Date(project.created_at).toLocaleString() }}
            </span>
          </div>

          <div
            class="absolute right-4 top-1/2 -translate-y-1/2 flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity bg-zinc-800/80 backdrop-blur-sm p-1 rounded">
            <button class="px-3 py-1 bg-zinc-700 hover:bg-zinc-600 rounded text-sm text-zinc-200"
              @click.stop="renameProject(project)">
              重命名
            </button>
            <button class="px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-sm text-white"
              @click.stop="deleteProject(project.id)">
              删除
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>