<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import TopologyPanel from '@/components/TopologyPanel.vue'

const route = useRoute()
const router = useRouter()
const projectId = route.params.projectId as string
const pageId = route.params.pageId as string

const showTopology = ref(false)
interface PageMeta {
  page_id: string
  image_remote_url: string | null
  image_local_path: string | null
  description_en: string | null
  description_other: string | null
}

interface Sentence {
  id: string
  line_ids: string[]
  type: 'text' | 'title' | 'note'
  normalized: string
  en: string
  zh: string
}

interface SentenceData {
  sentences: Sentence[]
}


interface Glyph {
  id: string
  content: string
  hpos: number
  vpos: number
  width: number
  height: number
  lineId: string
}

interface Line {
  id: string
  baseline: number[][]
  polygon: number[][]
  glyphs: Glyph[]
}

interface Region {
  id: string
  polygon: number[][]
}

interface HistoryState {
  lines: Line[]
  regions: Region[]
}

const meta = ref<PageMeta | null>(null)
const altoXml = ref<string | null>(null)
const sentenceData = ref<SentenceData | null>(null)
const showMetaModal = ref(false)
const lines = ref<Line[]>([])
const regions = ref<Region[]>([])
const selectedLineId = ref<string | null>(null)
const selectedGlyphId = ref<string | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)
const imageRef = ref<HTMLImageElement | null>(null)
const scale = ref(0.5)
const showBaselines = ref(true)
const showPolygons = ref(false)
const history = ref<HistoryState[]>([])
const historyIndex = ref(-1)


// 表单数据
const form = ref({
  image_remote_url: '',
  image_local_path: '',
  description_en: '',
  description_other: '',
})

// 计算警告信息
const warning = computed(() => {
  if (!meta.value) return '当前页面还没有元信息'

  const { image_remote_url, image_local_path } = meta.value
  const hasImage = !!(image_remote_url || image_local_path)

  if (!hasImage) return '当前页面还没有图片'
  if (!altoXml.value) return '当前页面还没有导入 ALTO'

  return null
})

async function importAlto() {
  const file = await open({
    filters: [{ name: 'ALTO XML', extensions: ['xml'] }]
  })

  if (!file) return

  const content = await readTextFile(file as string)

  // 解析 XML 获取 TextLine ID 列表
  const parser = new DOMParser()
  const doc = parser.parseFromString(content, 'text/xml')
  const textLines = doc.querySelectorAll('TextLine')

  // 初始化语义数据，每个 TextLine 对应一个 Sentence
  const sentences: Sentence[] = []
  textLines.forEach(line => {
    const lineId = line.getAttribute('ID') || ''
    sentences.push({
      id: crypto.randomUUID(),
      line_ids: [lineId],
      type: 'text',
      normalized: '',
      en: '',
      zh: '',
    })
  })

  // 保存到数据库
  await invoke('save_alto', { pageId, xml: content })
  await invoke('save_sentence', { pageId, data: JSON.stringify({ sentences }) })

  // 重新加载数据
  await loadData()
}



// 获取图片URL
const imageUrl = computed(() => {
  if (!meta.value) return null
  return meta.value.image_remote_url || meta.value.image_local_path || null
})

async function loadMeta() {
  meta.value = await invoke('get_page_meta', { pageId })
}

function openMetaModal() {
  if (meta.value) {
    form.value = {
      image_remote_url: meta.value.image_remote_url || '',
      image_local_path: meta.value.image_local_path || '',
      description_en: meta.value.description_en || '',
      description_other: meta.value.description_other || '',
    }
  } else {
    form.value = {
      image_remote_url: '',
      image_local_path: '',
      description_en: '',
      description_other: '',
    }
  }
  showMetaModal.value = true
}

async function saveMeta() {
  const metaData: PageMeta = {
    page_id: pageId,
    image_remote_url: form.value.image_remote_url || null,
    image_local_path: form.value.image_local_path || null,
    description_en: form.value.description_en || null,
    description_other: form.value.description_other || null,
  }

  await invoke('save_page_meta', { meta: metaData })
  await loadMeta()
  showMetaModal.value = false
}

function parsePoints(pointsStr: string): number[][] {
  const nums = pointsStr.trim().split(/\s+/).map(Number)
  const points: number[][] = []
  for (let i = 0; i < nums.length; i += 2) {
    points.push([nums[i], nums[i + 1]])
  }
  return points
}

function parseAltoXml(xml: string) {
  const parser = new DOMParser()
  const doc = parser.parseFromString(xml, 'text/xml')

  // 解析 Regions
  const textBlocks = doc.querySelectorAll('TextBlock')
  const regionList: Region[] = []
  textBlocks.forEach(block => {
    const polygonEl = block.querySelector(':scope > Shape > Polygon')
    if (polygonEl) {
      regionList.push({
        id: block.getAttribute('ID') || '',
        polygon: parsePoints(polygonEl.getAttribute('POINTS') || '')
      })
    }
  })
  regions.value = regionList

  // 解析 Lines
  const textLines = doc.querySelectorAll('TextLine')
  const lineList: Line[] = []

  textLines.forEach(line => {
    const lineId = line.getAttribute('ID') || ''
    const baselineStr = line.getAttribute('BASELINE') || ''
    const baseline = parsePoints(baselineStr)

    const polygonEl = line.querySelector(':scope > Shape > Polygon')
    const polygon = polygonEl
      ? parsePoints(polygonEl.getAttribute('POINTS') || '')
      : []

    const glyphs: Glyph[] = []
    line.querySelectorAll('Glyph').forEach(g => {
      glyphs.push({
        id: g.getAttribute('ID') || '',
        content: g.getAttribute('CONTENT') || '',
        hpos: parseInt(g.getAttribute('HPOS') || '0'),
        vpos: parseInt(g.getAttribute('VPOS') || '0'),
        width: parseInt(g.getAttribute('WIDTH') || '0'),
        height: parseInt(g.getAttribute('HEIGHT') || '0'),
        lineId
      })
    })

    lineList.push({ id: lineId, baseline, polygon, glyphs })
  })

  lines.value = lineList
}

function getGlyphRect(glyph: Glyph, index: number, glyphs: Glyph[]) {
  const nextGlyph = glyphs[index + 1]
  const width = nextGlyph ? nextGlyph.hpos - glyph.hpos : 20
  return { x: glyph.hpos, y: glyph.vpos, width, height: glyph.height }
}

function drawCanvas() {
  const canvas = canvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  ctx.clearRect(0, 0, canvas.width, canvas.height)

  // 绘制 Regions（绿色）
  if (showPolygons.value) {
    for (const region of regions.value) {
      if (region.polygon.length > 0) {
        ctx.beginPath()
        ctx.moveTo(region.polygon[0][0], region.polygon[0][1])
        for (let i = 1; i < region.polygon.length; i++) {
          ctx.lineTo(region.polygon[i][0], region.polygon[i][1])
        }
        ctx.closePath()
        ctx.strokeStyle = 'rgba(0, 255, 128, 0.8)'
        ctx.lineWidth = 2
        ctx.stroke()
      }
    }
  }

  // 绘制行多边形（紫色）
  if (showPolygons.value) {
    for (const line of lines.value) {
      if (line.polygon.length > 0) {
        const isSelected = line.id === selectedLineId.value

        ctx.beginPath()
        ctx.moveTo(line.polygon[0][0], line.polygon[0][1])
        for (let i = 1; i < line.polygon.length; i++) {
          ctx.lineTo(line.polygon[i][0], line.polygon[i][1])
        }
        ctx.closePath()

        if (isSelected) {
          ctx.fillStyle = 'rgba(255, 0, 255, 0.3)'
          ctx.strokeStyle = 'rgba(255, 0, 255, 1)'
          ctx.lineWidth = 2
        } else {
          ctx.fillStyle = 'rgba(255, 0, 255, 0.15)'
          ctx.strokeStyle = 'rgba(255, 0, 255, 0.8)'
          ctx.lineWidth = 1
        }
        ctx.fill()
        ctx.stroke()

        // 选中时绘制顶点
        if (isSelected) {
          for (const point of line.polygon) {
            ctx.beginPath()
            ctx.arc(point[0], point[1], 4, 0, Math.PI * 2)
            ctx.fillStyle = 'white'
            ctx.fill()
            ctx.strokeStyle = 'rgba(255, 0, 255, 1)'
            ctx.lineWidth = 2
            ctx.stroke()
          }
        }
      }
    }
  }

  // 绘制 Baselines（蓝色）
  if (showBaselines.value) {
    for (const line of lines.value) {
      if (line.baseline.length > 1) {
        ctx.beginPath()
        ctx.moveTo(line.baseline[0][0], line.baseline[0][1])
        for (let i = 1; i < line.baseline.length; i++) {
          ctx.lineTo(line.baseline[i][0], line.baseline[i][1])
        }
        ctx.strokeStyle = 'rgba(0, 0, 255, 1)'
        ctx.lineWidth = 2
        ctx.stroke()
      }
    }
  }

  // 绘制字符框
  for (const line of lines.value) {
    const glyphs = line.glyphs
    for (let i = 0; i < glyphs.length; i++) {
      const glyph = glyphs[i]
      const rect = getGlyphRect(glyph, i, glyphs)
      const isSelected = glyph.id === selectedGlyphId.value

      if (isSelected) {
        ctx.fillStyle = 'rgba(59, 130, 246, 0.3)'
        ctx.fillRect(rect.x, rect.y, rect.width, rect.height)
        ctx.strokeStyle = 'rgba(59, 130, 246, 1)'
        ctx.lineWidth = 2
      } else {
        ctx.strokeStyle = 'rgba(59, 130, 246, 0.5)'
        ctx.lineWidth = 1
      }

      ctx.strokeRect(rect.x, rect.y, rect.width, rect.height)
    }
  }
}

function isPointInPolygon(x: number, y: number, polygon: number[][]): boolean {
  if (polygon.length < 3) return false

  let inside = false
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const xi = polygon[i][0], yi = polygon[i][1]
    const xj = polygon[j][0], yj = polygon[j][1]

    if (((yi > y) !== (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi)) {
      inside = !inside
    }
  }
  return inside
}

function handleCanvasClick(e: MouseEvent) {
  const canvas = canvasRef.value
  if (!canvas) return

  const rect = canvas.getBoundingClientRect()
  const scaleX = canvas.width / rect.width
  const scaleY = canvas.height / rect.height
  const x = (e.clientX - rect.left) * scaleX
  const y = (e.clientY - rect.top) * scaleY

  // 检查是否点击了某条线的多边形内部
  for (const line of lines.value) {
    if (isPointInPolygon(x, y, line.polygon)) {
      selectedLineId.value = line.id
      selectedGlyphId.value = null
      drawCanvas()
      return
    }
  }

  // 检查是否点击了某个字符
  for (const line of lines.value) {
    for (let i = 0; i < line.glyphs.length; i++) {
      const glyph = line.glyphs[i]
      const glyphRect = getGlyphRect(glyph, i, line.glyphs)

      if (
        x >= glyphRect.x &&
        x <= glyphRect.x + glyphRect.width &&
        y >= glyphRect.y &&
        y <= glyphRect.y + glyphRect.height
      ) {
        selectedGlyphId.value = glyph.id
        selectedLineId.value = null
        drawCanvas()
        return
      }
    }
  }

  selectedGlyphId.value = null
  selectedLineId.value = null
  drawCanvas()
}

function handleGlyphClick(glyphId: string) {
  selectedGlyphId.value = glyphId
  selectedLineId.value = null
  drawCanvas()
}

function handleWheel(e: WheelEvent) {
  e.preventDefault()
  const delta = e.deltaY > 0 ? -0.1 : 0.1
  scale.value = Math.max(0.1, Math.min(3, scale.value + delta))
}

function saveHistory() {
  history.value = history.value.slice(0, historyIndex.value + 1)
  history.value.push({
    lines: JSON.parse(JSON.stringify(lines.value)),
    regions: JSON.parse(JSON.stringify(regions.value))
  })
  historyIndex.value++
}

function undo() {
  if (historyIndex.value > 0) {
    historyIndex.value--
    const state = history.value[historyIndex.value]
    lines.value = JSON.parse(JSON.stringify(state.lines))
    regions.value = JSON.parse(JSON.stringify(state.regions))
    selectedLineId.value = null
    selectedGlyphId.value = null
    drawCanvas()
  }
}

function redo() {
  if (historyIndex.value < history.value.length - 1) {
    historyIndex.value++
    const state = history.value[historyIndex.value]
    lines.value = JSON.parse(JSON.stringify(state.lines))
    regions.value = JSON.parse(JSON.stringify(state.regions))
    selectedLineId.value = null
    selectedGlyphId.value = null
    drawCanvas()
  }
}

function deleteLine() {
  if (!selectedLineId.value) return
  saveHistory()
  lines.value = lines.value.filter(l => l.id !== selectedLineId.value)
  selectedLineId.value = null
  selectedGlyphId.value = null
  drawCanvas()
}

function goBack() {
  router.push(`/project/${projectId}`)
}

async function loadData() {
  altoXml.value = await invoke('get_alto', { pageId }) as string | null
  const sentenceStr = await invoke('get_sentence', { pageId }) as string | null
  sentenceData.value = sentenceStr ? JSON.parse(sentenceStr) : null

  // 解析 ALTO XML
  if (altoXml.value) {
    parseAltoXml(altoXml.value)
    nextTick(() => {
      saveHistory()
      if (imageRef.value && imageRef.value.complete) {
        initCanvas()
      }
    })
  }
}

function initCanvas() {
  if (canvasRef.value && imageRef.value) {
    canvasRef.value.width = imageRef.value.naturalWidth
    canvasRef.value.height = imageRef.value.naturalHeight
    drawCanvas()
  }
}

watch([selectedGlyphId, selectedLineId, showBaselines, showPolygons], () => {
  drawCanvas()
})

onMounted(() => {
  loadMeta()
  loadData()
})
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
    <!-- 工具栏 -->
    <div class="h-12 bg-zinc-900 border-b border-zinc-700 flex items-center px-4 gap-2">
      <button class="px-3 py-1.5 rounded text-sm font-medium bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
        @click="goBack">
        ← 返回
      </button>

      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button class="px-3 py-1.5 rounded text-sm font-medium bg-blue-600 text-white hover:bg-blue-500"
        @click="openMetaModal">
        元信息
      </button>

      <button class="px-3 py-1.5 rounded text-sm font-medium bg-green-600 text-white hover:bg-green-500"
        @click="importAlto">
        导入 ALTO
      </button>

      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button :class="[
        'px-3 py-1.5 rounded text-sm font-medium transition-colors',
        showBaselines
          ? 'bg-blue-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showBaselines = !showBaselines">
        Baselines
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-sm font-medium transition-colors',
        showPolygons
          ? 'bg-fuchsia-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showPolygons = !showPolygons">
        Polygons
      </button>

      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button :class="[
        'px-3 py-1.5 rounded text-sm font-medium transition-colors',
        historyIndex > 0
          ? 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
          : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
      ]" :disabled="historyIndex <= 0" @click="undo">
        Undo
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-sm font-medium transition-colors',
        historyIndex < history.length - 1
          ? 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
          : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
      ]" :disabled="historyIndex >= history.length - 1" @click="redo">
        Redo
      </button>

      <div v-if="warning" class="ml-4 px-3 py-1 bg-yellow-600/20 text-yellow-400 rounded text-sm">
        ⚠️ {{ warning }}
      </div>
      <button :class="[
        'px-3 py-1.5 rounded text-sm font-medium transition-colors',
        showTopology
          ? 'bg-purple-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showTopology = !showTopology">
        拓扑关系
      </button>
    </div>

    <!-- 内容区 -->
    <div class="flex-1 flex">
      <!-- 左侧图片区 -->
      <div :class="[
        'bg-zinc-900 relative overflow-auto transition-all duration-300',
        showTopology ? 'w-[36.4%]' : 'w-1/2'
      ]" @wheel="handleWheel">
        <!-- 删除按钮 -->
        <button v-if="selectedLineId"
          class="absolute top-2 left-2 px-3 py-1.5 bg-red-600 hover:bg-red-500 text-white text-sm font-medium rounded shadow-lg z-10"
          @click="deleteLine">
          Delete
        </button>

        <div v-if="imageUrl" class="relative inline-block origin-top-left" :style="{ transform: `scale(${scale})` }">
          <img ref="imageRef" :src="imageUrl" class="max-w-none" @load="initCanvas" />
          <canvas ref="canvasRef" class="absolute top-0 left-0 cursor-crosshair" @click="handleCanvasClick" />
        </div>
        <div v-else class="h-full flex items-center justify-center text-zinc-500">
          暂无图片
        </div>
      </div>

      <!-- 右侧文本区 -->
      <div :class="[
        'bg-zinc-950 border-l border-zinc-800 p-4 overflow-y-auto transition-all duration-300',
        showTopology ? 'w-[36.4%]' : 'w-1/2'
      ]">
        <div v-if="lines.length > 0" class="space-y-2">
          <div v-for="line in lines" :key="line.id" class="p-2 bg-zinc-900 rounded">
            <span v-for="glyph in line.glyphs" :key="glyph.id" :class="[
              'cursor-pointer hover:bg-blue-500/30 rounded px-px',
              selectedGlyphId === glyph.id ? 'bg-blue-500/50 text-white' : 'text-zinc-300'
            ]" @click="handleGlyphClick(glyph.id)">{{ glyph.content }}</span>
          </div>
        </div>
        <div v-else class="text-zinc-500">
          暂无数据，请导入 ALTO 文件
        </div>
      </div>

      <!-- 拓扑关系面板 -->
      <div v-if="showTopology" class="w-[27.2%] border-l border-zinc-800 transition-all duration-300">
        <TopologyPanel :sentences="sentenceData?.sentences || []" />
      </div>
    </div>

    <!-- 元信息弹窗 -->
    <div v-if="showMetaModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      @click.self="showMetaModal = false">
      <div class="bg-zinc-900 rounded-lg p-6 w-125 max-h-[80vh] overflow-y-auto">
        <h2 class="text-lg font-bold mb-4">编辑元信息</h2>

        <div class="space-y-4">
          <div>
            <label class="block text-sm text-zinc-400 mb-1">图片远程链接</label>
            <input v-model="form.image_remote_url" type="text"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500"
              placeholder="https://..." />
          </div>

          <div>
            <label class="block text-sm text-zinc-400 mb-1">图片本地路径</label>
            <input v-model="form.image_local_path" type="text"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500"
              placeholder="C:\..." />
          </div>
          <div>
            <label class="block text-sm text-zinc-400 mb-1">英文描述</label>
            <textarea v-model="form.description_en"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
              rows="3" placeholder="English description..." />
          </div>

          <div>
            <label class="block text-sm text-zinc-400 mb-1">其他语言描述</label>
            <textarea v-model="form.description_other"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
              rows="3" placeholder="其他语言描述..." />
          </div>
        </div>

        <div class="flex justify-end gap-2 mt-6">
          <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-sm" @click="showMetaModal = false">
            取消
          </button>
          <button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-sm" @click="saveMeta">
            保存
          </button>
        </div>
      </div>
    </div>
  </div>
</template>