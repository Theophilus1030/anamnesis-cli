<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue'
import altoXml from '@/assets/output_alto.xml?raw'
import testImage from '@/assets/St_Gall_906_910.jpg'
const scale = ref(0.5)
const selectedLineId = ref<string | null>(null)

// 历史记录
interface HistoryState {
  lines: Line[]
  regions: Region[]
}
const history = ref<HistoryState[]>([])
const historyIndex = ref(-1)
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

const lines = ref<Line[]>([])
const regions = ref<Region[]>([])
const imageSize = ref({ width: 0, height: 0 })
const canvasRef = ref<HTMLCanvasElement | null>(null)
const imageRef = ref<HTMLImageElement | null>(null)
const selectedGlyphId = ref<string | null>(null)

// 显示控制
const showBaselines = ref(true)
const showPolygons = ref(false)

function parsePoints(pointsStr: string): number[][] {
  const nums = pointsStr.trim().split(/\s+/).map(Number)
  const points: number[][] = []
  for (let i = 0; i < nums.length; i += 2) {
    points.push([nums[i], nums[i + 1]])
  }
  return points
}

function parseAlto(xml: string) {
  const parser = new DOMParser()
  const doc = parser.parseFromString(xml, 'text/xml')

  const page = doc.querySelector('Page')
  if (page) {
    imageSize.value = {
      width: parseInt(page.getAttribute('WIDTH') || '0'),
      height: parseInt(page.getAttribute('HEIGHT') || '0')
    }
  }

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

    // 解析 baseline
    const baselineStr = line.getAttribute('BASELINE') || ''
    const baseline = parsePoints(baselineStr)

    // 解析 polygon
    const polygonEl = line.querySelector(':scope > Shape > Polygon')
    const polygon = polygonEl
      ? parsePoints(polygonEl.getAttribute('POINTS') || '')
      : []

    // 解析 glyphs
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

    lineList.push({
      id: lineId,
      baseline,
      polygon,
      glyphs
    })
  })

  return lineList
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

function handleGlyphClick(glyphId: string) {
  selectedGlyphId.value = glyphId
}

function saveHistory() {
  // 截断当前位置之后的历史
  history.value = history.value.slice(0, historyIndex.value + 1)
  // 深拷贝当前状态
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

function handleWheel(e: WheelEvent) {
  e.preventDefault()
  const delta = e.deltaY > 0 ? -0.1 : 0.1
  scale.value = Math.max(0.1, Math.min(3, scale.value + delta))
}

watch([selectedGlyphId, selectedLineId, showBaselines, showPolygons], () => {
  drawCanvas()
})

onMounted(() => {
  lines.value = parseAlto(altoXml)

  // 保存初始状态
  nextTick(() => {
    saveHistory()
  })

  nextTick(() => {
    if (imageRef.value) {
      imageRef.value.onload = () => {
        if (canvasRef.value && imageRef.value) {
          canvasRef.value.width = imageRef.value.naturalWidth
          canvasRef.value.height = imageRef.value.naturalHeight
          drawCanvas()
        }
      }
    }
  })
})
</script>

<template>
  <div class="h-screen w-screen flex flex-col">
    <!-- 顶部工具栏 -->
    <div class="h-12 bg-zinc-900 border-b border-zinc-700 flex items-center px-4 gap-2">
      <button class="px-3 py-1.5 rounded text-sm font-medium bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
        @click="$router.push('/')">
        ← 返回
      </button>
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
    </div>

    <!-- 主内容区 -->
    <div class="flex-1 flex overflow-hidden">
      <!-- 左侧：图像区域 -->
      <div ref="containerRef" class="flex-1 bg-zinc-900 relative overflow-auto" @wheel="handleWheel">
        <div class="relative inline-block origin-top-left" :style="{ transform: `scale(${scale})` }">
          <img ref="imageRef" :src="testImage" class="max-w-none" />
          <canvas ref="canvasRef" class="absolute top-0 left-0 cursor-crosshair" @click="handleCanvasClick" />
          <!-- 删除按钮 -->
          <button v-if="selectedLineId"
            class="absolute top-2 left-2 px-3 py-1.5 bg-red-600 hover:bg-red-500 text-white text-sm font-medium rounded shadow-lg z-10"
            @click="deleteLine">
            Delete
          </button>
        </div>
      </div>

      <!-- 右侧：文本区域 -->
      <div class="flex-1 border-l border-zinc-800 bg-zinc-950 p-4 overflow-y-auto">
        <div v-for="line in lines" :key="line.id" class="mb-2">
          <span v-for="glyph in line.glyphs" :key="glyph.id" :class="[
            'cursor-pointer hover:bg-blue-500/30 rounded px-px',
            selectedGlyphId === glyph.id ? 'bg-blue-500/50 text-white' : 'text-zinc-300'
          ]" @click="handleGlyphClick(glyph.id)">{{ glyph.content }}</span>
        </div>
      </div>
    </div>
  </div>
</template>