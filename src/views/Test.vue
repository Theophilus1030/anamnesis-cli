<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()

const imagePath = ref('')
const modelPath = ref('')
const segModelPath = ref('')
const useDefaultRecModel = ref(true)   // 是否使用默认识别模型
const useDefaultSegModel = ref(true)   // 是否使用默认分割模型
const isProcessing = ref(false)
const resultXml = ref('')
const errorMsg = ref('')

// 默认模型状态
const defaultModelsStatus = ref({
  recognition_model: false,
  segmentation_model: false
})

onMounted(async () => {
  // 检查默认模型是否存在
  try {
    const status = await invoke('check_default_models')
    defaultModelsStatus.value = status as typeof defaultModelsStatus.value
  } catch (e) {
    console.error('检查默认模型失败:', e)
  }
})

async function selectImage() {
  const file = await open({
    filters: [{ name: 'Image', extensions: ['jpg', 'jpeg', 'png', 'tif', 'tiff'] }]
  })
  if (file) {
    imagePath.value = file as string
  }
}

async function selectModel() {
  const file = await open({
    filters: [{ name: 'Model', extensions: ['mlmodel'] }]
  })
  if (file) {
    modelPath.value = file as string
  }
}

async function selectSegModel() {
  const file = await open({
    filters: [{ name: 'Model', extensions: ['mlmodel'] }]
  })
  if (file) {
    segModelPath.value = file as string
  }
}

async function runOcr() {
  if (!imagePath.value) {
    alert('请先选择图片')
    return
  }

  // 检查模型选择
  if (!useDefaultRecModel.value && !modelPath.value) {
    alert('请选择识别模型或使用默认模型')
    return
  }
  if (!useDefaultSegModel.value && !segModelPath.value) {
    alert('请选择分割模型或使用默认模型')
    return
  }

  isProcessing.value = true
  resultXml.value = ''
  errorMsg.value = ''

  try {
    const result = await invoke('run_kraken_ocr', {
      imagePath: imagePath.value,
      modelPath: useDefaultRecModel.value ? null : modelPath.value,
      segModelPath: useDefaultSegModel.value ? null : segModelPath.value
    })
    resultXml.value = result as string
  } catch (e) {
    errorMsg.value = `执行失败: ${e}`
  } finally {
    isProcessing.value = false
  }
}

function goBack() {
  router.push('/')
}
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 p-8">
    <div class="max-w-4xl mx-auto">
      <div class="flex items-center gap-4 mb-8">
        <button
          class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded"
          @click="goBack"
        >
          ← 返回首页
        </button>
        <h1 class="text-2xl font-bold">Kraken OCR 测试</h1>
      </div>

      <!-- 文件选择 -->
      <div class="space-y-4 mb-8">
        <!-- 图片选择 -->
        <div class="flex items-center gap-4">
          <button
            class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded"
            @click="selectImage"
          >
            选择图片
          </button>
          <span class="text-zinc-400 text-sm truncate flex-1">
            {{ imagePath || '未选择' }}
          </span>
        </div>

        <!-- 识别模型选择 -->
        <div class="p-4 bg-zinc-900 rounded space-y-2">
          <div class="flex items-center gap-2">
            <input
              type="checkbox"
              id="useDefaultRecModel"
              v-model="useDefaultRecModel"
              class="w-4 h-4"
            />
            <label for="useDefaultRecModel" class="text-sm">
              使用默认识别模型 (catmus-medieval)
              <span v-if="defaultModelsStatus.recognition_model" class="text-green-400">✓ 已安装</span>
              <span v-else class="text-red-400">✗ 未找到</span>
            </label>
          </div>
          <div v-if="!useDefaultRecModel" class="flex items-center gap-4 ml-6">
            <button
              class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm"
              @click="selectModel"
            >
              选择识别模型
            </button>
            <span class="text-zinc-400 text-sm truncate flex-1">
              {{ modelPath || '未选择' }}
            </span>
          </div>
        </div>

        <!-- 分割模型选择 -->
        <div class="p-4 bg-zinc-900 rounded space-y-2">
          <div class="flex items-center gap-2">
            <input
              type="checkbox"
              id="useDefaultSegModel"
              v-model="useDefaultSegModel"
              class="w-4 h-4"
            />
            <label for="useDefaultSegModel" class="text-sm">
              使用默认分割模型 (blla)
              <span v-if="defaultModelsStatus.segmentation_model" class="text-green-400">✓ 已安装</span>
              <span v-else class="text-red-400">✗ 未找到</span>
            </label>
          </div>
          <div v-if="!useDefaultSegModel" class="flex items-center gap-4 ml-6">
            <button
              class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm"
              @click="selectSegModel"
            >
              选择分割模型
            </button>
            <span class="text-zinc-400 text-sm truncate flex-1">
              {{ segModelPath || '未选择' }}
            </span>
          </div>
        </div>
      </div>

      <!-- 执行按钮 -->
      <div class="mb-8">
        <button
          class="px-6 py-3 bg-green-600 hover:bg-green-500 rounded text-lg font-medium disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="isProcessing || !imagePath"
          @click="runOcr"
        >
          {{ isProcessing ? '处理中...' : '运行 OCR' }}
        </button>
      </div>

      <!-- 错误信息 -->
      <div v-if="errorMsg" class="mb-8 p-4 bg-red-900/50 border border-red-600 rounded">
        <h3 class="text-red-400 font-bold mb-2">错误</h3>
        <pre class="text-red-300 text-sm whitespace-pre-wrap">{{ errorMsg }}</pre>
      </div>

      <!-- 结果显示 -->
      <div v-if="resultXml" class="mb-8">
        <h3 class="text-lg font-bold mb-2">输出结果 (ALTO XML)</h3>
        <div class="bg-zinc-900 rounded p-4 max-h-96 overflow-auto">
          <pre class="text-zinc-300 text-sm whitespace-pre-wrap">{{ resultXml }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>
