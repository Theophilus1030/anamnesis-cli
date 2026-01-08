<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

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

interface Sentence {
  id: string
  line_ids: string[]
  type: 'text' | 'title' | 'note'
  normalized: string
  en: string
  zh: string
}

const props = defineProps<{
  line: Line | null
  imageUrl: string | null
  visible: boolean
  allLines: Line[]
  sentences: Sentence[]
  sentence: Sentence | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', line: Line): void
  (e: 'split-after', lineId: string, glyphIndex: number): void
  (e: 'split-before', lineId: string, glyphIndex: number): void
  (e: 'update-sentence', data: { normalized: string; en: string; zh: string; type: 'text' | 'title' | 'note' }): void
}>()

// 本地编辑副本
const editingLine = ref<Line | null>(null)
const selectedGlyphIndex = ref<number | null>(null)
const replaceChar = ref('')
const diacriticChar = ref('')  // 用于加帽子的组合字符

// 句子编辑表单
const sentenceForm = ref({
  normalized: '',
  en: '',
  zh: '',
  type: 'text' as 'text' | 'title' | 'note'
})

// Canvas 相关
const canvasRef = ref<HTMLCanvasElement | null>(null)
const imageRef = ref<HTMLImageElement | null>(null)
const imageLoaded = ref(false)

// 拖拽状态
const draggingCorner = ref<'tl' | 'tr' | 'bl' | 'br' | null>(null)

// 计算全局 TextLine 顺序
const globalLineOrder = computed(() => {
  const order: string[] = []
  for (const sentence of props.sentences) {
    for (const lineId of sentence.line_ids) {
      order.push(lineId)
    }
  }
  return order
})

// 计算当前行在全局中的位置
const globalPosition = computed(() => {
  if (!props.line) return { index: -1, total: 0, isFirst: true, isLast: true }

  const index = globalLineOrder.value.indexOf(props.line.id)
  const total = globalLineOrder.value.length

  return {
    index,
    total,
    isFirst: index === 0,
    isLast: index === total - 1
  }
})

// 是否可以向后断开
const canSplitAfter = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return false
  if (globalPosition.value.isLast) return false
  // 最后一个字符不能向后断开（没有内容可移动）
  if (selectedGlyphIndex.value === editingLine.value.glyphs.length - 1) return false
  return true
})

// 是否可以向前断开
const canSplitBefore = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return false
  if (globalPosition.value.isFirst) return false
  // 第一个字符不能向前断开（没有内容可移动）
  if (selectedGlyphIndex.value === 0) return false
  return true
})

// 计算多边形边界框
const polygonBounds = computed(() => {
  if (!editingLine.value || editingLine.value.polygon.length === 0) {
    return { minX: 0, minY: 0, maxX: 100, maxY: 100, width: 100, height: 100 }
  }

  const points = editingLine.value.polygon
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity

  for (const [x, y] of points) {
    minX = Math.min(minX, x)
    minY = Math.min(minY, y)
    maxX = Math.max(maxX, x)
    maxY = Math.max(maxY, y)
  }

  const padding = 20
  minX = Math.max(0, minX - padding)
  minY = Math.max(0, minY - padding)
  maxX = maxX + padding
  maxY = maxY + padding

  return { minX, minY, maxX, maxY, width: maxX - minX, height: maxY - minY }
})

// 选中的字符
const selectedGlyph = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return null
  return editingLine.value.glyphs[selectedGlyphIndex.value]
})

// 计算字符的实际宽度
function getGlyphWidth(index: number): number {
  if (!editingLine.value) return 20
  const glyphs = editingLine.value.glyphs
  const glyph = glyphs[index]
  const nextGlyph = glyphs[index + 1]
  return nextGlyph ? nextGlyph.hpos - glyph.hpos : Math.max(glyph.width, 20)
}

// 监听 props 变化
watch(() => props.visible, (newVal) => {
  if (newVal && props.line) {
    editingLine.value = JSON.parse(JSON.stringify(props.line))
    selectedGlyphIndex.value = null
    replaceChar.value = ''
    imageLoaded.value = false

    // 初始化句子表单
    if (props.sentence) {
      sentenceForm.value = {
        normalized: props.sentence.normalized || '',
        en: props.sentence.en || '',
        zh: props.sentence.zh || '',
        type: props.sentence.type || 'text'
      }
    }

    nextTick(() => {
      loadImage()
    })
  }
}, { immediate: true })

function loadImage() {
  if (!props.imageUrl || !canvasRef.value) return

  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.onload = () => {
    imageRef.value = img
    imageLoaded.value = true
    drawCanvas()
  }
  img.src = props.imageUrl
}

function drawCanvas() {
  const canvas = canvasRef.value
  const img = imageRef.value
  if (!canvas || !img || !editingLine.value) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const bounds = polygonBounds.value

  canvas.width = bounds.width
  canvas.height = bounds.height

  ctx.clearRect(0, 0, canvas.width, canvas.height)

  ctx.drawImage(
    img,
    bounds.minX, bounds.minY, bounds.width, bounds.height,
    0, 0, bounds.width, bounds.height
  )

  ctx.fillStyle = 'rgba(0, 0, 0, 0.6)'
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  ctx.save()
  ctx.beginPath()
  const polygon = editingLine.value.polygon
  if (polygon.length > 0) {
    ctx.moveTo(polygon[0][0] - bounds.minX, polygon[0][1] - bounds.minY)
    for (let i = 1; i < polygon.length; i++) {
      ctx.lineTo(polygon[i][0] - bounds.minX, polygon[i][1] - bounds.minY)
    }
    ctx.closePath()
  }
  ctx.clip()

  ctx.drawImage(
    img,
    bounds.minX, bounds.minY, bounds.width, bounds.height,
    0, 0, bounds.width, bounds.height
  )
  ctx.restore()

  if (polygon.length > 0) {
    ctx.beginPath()
    ctx.moveTo(polygon[0][0] - bounds.minX, polygon[0][1] - bounds.minY)
    for (let i = 1; i < polygon.length; i++) {
      ctx.lineTo(polygon[i][0] - bounds.minX, polygon[i][1] - bounds.minY)
    }
    ctx.closePath()
    ctx.strokeStyle = 'rgba(255, 0, 255, 0.8)'
    ctx.lineWidth = 2
    ctx.stroke()
  }

  const baseline = editingLine.value.baseline
  if (baseline.length > 1) {
    ctx.beginPath()
    ctx.moveTo(baseline[0][0] - bounds.minX, baseline[0][1] - bounds.minY)
    for (let i = 1; i < baseline.length; i++) {
      ctx.lineTo(baseline[i][0] - bounds.minX, baseline[i][1] - bounds.minY)
    }
    ctx.strokeStyle = 'rgba(0, 0, 255, 0.8)'
    ctx.lineWidth = 2
    ctx.stroke()
  }

  if (selectedGlyph.value && selectedGlyphIndex.value !== null) {
    const glyph = selectedGlyph.value
    const width = getGlyphWidth(selectedGlyphIndex.value)

    const x = glyph.hpos - bounds.minX
    const y = glyph.vpos - bounds.minY
    const w = width
    const h = glyph.height

    ctx.strokeStyle = 'rgba(59, 130, 246, 1)'
    ctx.lineWidth = 2
    ctx.strokeRect(x, y, w, h)

    const corners = [
      { x: x, y: y },
      { x: x + w, y: y },
      { x: x, y: y + h },
      { x: x + w, y: y + h }
    ]

    for (const corner of corners) {
      ctx.beginPath()
      ctx.arc(corner.x, corner.y, 6, 0, Math.PI * 2)
      ctx.fillStyle = 'white'
      ctx.fill()
      ctx.strokeStyle = 'rgba(59, 130, 246, 1)'
      ctx.lineWidth = 2
      ctx.stroke()
    }
  }
}

function selectGlyph(index: number) {
  selectedGlyphIndex.value = index
  nextTick(() => drawCanvas())
}

function handleCanvasMouseDown(e: MouseEvent) {
  if (selectedGlyphIndex.value === null || !selectedGlyph.value) return

  const canvas = canvasRef.value
  if (!canvas) return

  const rect = canvas.getBoundingClientRect()
  const scaleX = canvas.width / rect.width
  const scaleY = canvas.height / rect.height
  const x = (e.clientX - rect.left) * scaleX
  const y = (e.clientY - rect.top) * scaleY

  const bounds = polygonBounds.value
  const glyph = selectedGlyph.value
  const width = getGlyphWidth(selectedGlyphIndex.value)

  const glyphX = glyph.hpos - bounds.minX
  const glyphY = glyph.vpos - bounds.minY
  const glyphW = width
  const glyphH = glyph.height

  const corners = [
    { x: glyphX, y: glyphY, key: 'tl' as const },
    { x: glyphX + glyphW, y: glyphY, key: 'tr' as const },
    { x: glyphX, y: glyphY + glyphH, key: 'bl' as const },
    { x: glyphX + glyphW, y: glyphY + glyphH, key: 'br' as const }
  ]

  for (const corner of corners) {
    if (Math.hypot(x - corner.x, y - corner.y) <= 10) {
      draggingCorner.value = corner.key
      return
    }
  }
}

function handleCanvasMouseMove(e: MouseEvent) {
  if (!draggingCorner.value || selectedGlyphIndex.value === null || !editingLine.value) return

  const canvas = canvasRef.value
  if (!canvas) return

  const rect = canvas.getBoundingClientRect()
  const scaleX = canvas.width / rect.width
  const scaleY = canvas.height / rect.height
  const x = (e.clientX - rect.left) * scaleX
  const y = (e.clientY - rect.top) * scaleY

  const bounds = polygonBounds.value
  const glyph = editingLine.value.glyphs[selectedGlyphIndex.value]

  const realX = x + bounds.minX
  const realY = y + bounds.minY

  switch (draggingCorner.value) {
    case 'tl':
      glyph.hpos = Math.round(realX)
      glyph.vpos = Math.round(realY)
      break
    case 'tr':
      glyph.vpos = Math.round(realY)
      break
    case 'bl':
      glyph.hpos = Math.round(realX)
      glyph.height = Math.round(realY - glyph.vpos)
      break
    case 'br':
      glyph.height = Math.round(realY - glyph.vpos)
      break
  }

  drawCanvas()
}

function handleCanvasMouseUp() {
  draggingCorner.value = null
}

function replaceCharacter() {
  if (selectedGlyphIndex.value === null || !editingLine.value || !replaceChar.value) return

  editingLine.value.glyphs[selectedGlyphIndex.value].content = replaceChar.value
  replaceChar.value = ''
}

function handleSplitAfter() {
  if (!canSplitAfter.value || selectedGlyphIndex.value === null || !editingLine.value) return

  if (!confirm(`确定要将"${selectedGlyph.value?.content}"之后的字符移到下一个 TextLine 吗？`)) return

  emit('split-after', editingLine.value.id, selectedGlyphIndex.value)
}

function handleSplitBefore() {
  if (!canSplitBefore.value || selectedGlyphIndex.value === null || !editingLine.value) return

  if (!confirm(`确定要将"${selectedGlyph.value?.content}"之前的字符移到上一个 TextLine 吗？`)) return

  emit('split-before', editingLine.value.id, selectedGlyphIndex.value)
}

function handleSave() {
  if (editingLine.value) {
    emit('update-sentence', { ...sentenceForm.value })
    emit('save', JSON.parse(JSON.stringify(editingLine.value)))
  }
}

function handleClose() {
  emit('close')
}

function deleteCharacter() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return
  if (editingLine.value.glyphs.length <= 1) {
    alert('至少保留一个字符')
    return
  }

  editingLine.value.glyphs.splice(selectedGlyphIndex.value, 1)

  // 调整选中索引
  if (selectedGlyphIndex.value >= editingLine.value.glyphs.length) {
    selectedGlyphIndex.value = editingLine.value.glyphs.length - 1
  }

  nextTick(() => drawCanvas())
}

function insertCharacterBefore() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return

  const currentGlyph = editingLine.value.glyphs[selectedGlyphIndex.value]

  // 计算新字符的位置（当前字符左移一半宽度）
  const currentWidth = getGlyphWidth(selectedGlyphIndex.value)
  const halfWidth = Math.round(currentWidth / 2)

  const newGlyph: Glyph = {
    id: `glyph_${Date.now()}`,
    content: '?',
    hpos: currentGlyph.hpos,
    vpos: currentGlyph.vpos,
    width: halfWidth,
    height: currentGlyph.height,
    lineId: currentGlyph.lineId
  }

  // 当前字符右移
  currentGlyph.hpos = currentGlyph.hpos + halfWidth

  // 插入新字符
  editingLine.value.glyphs.splice(selectedGlyphIndex.value, 0, newGlyph)

  // 选中新插入的字符
  nextTick(() => drawCanvas())
}

function insertCharacterAfter() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return

  const currentGlyph = editingLine.value.glyphs[selectedGlyphIndex.value]

  // 计算新字符的位置
  const currentWidth = getGlyphWidth(selectedGlyphIndex.value)
  const halfWidth = Math.round(currentWidth / 2)

  const newHpos = currentGlyph.hpos + halfWidth

  const newGlyph: Glyph = {
    id: `glyph_${Date.now()}`,
    content: '?',
    hpos: newHpos,
    vpos: currentGlyph.vpos,
    width: halfWidth,
    height: currentGlyph.height,
    lineId: currentGlyph.lineId
  }

  // 插入新字符
  editingLine.value.glyphs.splice(selectedGlyphIndex.value + 1, 0, newGlyph)

  // 选中新插入的字符
  selectedGlyphIndex.value = selectedGlyphIndex.value + 1

  nextTick(() => drawCanvas())
}

// 常用组合字符映射
const diacriticMap: { [key: string]: string } = {
  '~': '\u0303',  // 波浪号 combining tilde
  '^': '\u0302',  // 抑扬符 combining circumflex
  '`': '\u0300',  // 重音符 combining grave
  "'": '\u0301',  // 锐音符 combining acute
  '"': '\u0308',  // 分音符 combining diaeresis
  'o': '\u030A',  // 上圆圈 combining ring above
  '-': '\u0304',  // 长音符 combining macron
  'u': '\u0306',  // 短音符 combining breve
  'v': '\u030C',  // 抑扬符(倒) combining caron
  '.': '\u0307',  // 上点 combining dot above
  ',': '\u0327',  // 下尾 combining cedilla
}

function addDiacritic() {
  if (selectedGlyphIndex.value === null || !editingLine.value || !diacriticChar.value) return

  const glyph = editingLine.value.glyphs[selectedGlyphIndex.value]
  const baseChar = glyph.content

  // 查找组合字符
  let combiningChar = diacriticMap[diacriticChar.value]

  // 如果映射表里没有，尝试直接使用输入的字符（可能用户直接输入了组合字符）
  if (!combiningChar) {
    // 检查是否是组合字符范围 (U+0300 - U+036F)
    const code = diacriticChar.value.charCodeAt(0)
    if (code >= 0x0300 && code <= 0x036F) {
      combiningChar = diacriticChar.value
    }
  }

  if (!combiningChar) {
    alert(`找不到对应的组合字符: "${diacriticChar.value}"\n\n可用的符号: ~ ^ \` ' " o - u v . ,`)
    return
  }

  // 合并字符
  const combined = (baseChar + combiningChar).normalize('NFC')

  // 检查是否成功合并（如果 NFC 后长度还是 2，说明没有对应的组合形式）
  if (combined.length > 1 && combined !== baseChar) {
    // 虽然没有预组合形式，但仍然可以使用组合序列
    glyph.content = combined.normalize('NFC')
  } else {
    glyph.content = combined
  }

  diacriticChar.value = ''
  nextTick(() => drawCanvas())
}

watch(selectedGlyphIndex, () => {
  if (imageLoaded.value) {
    drawCanvas()
  }
})
</script>

<template>
  <div v-if="visible && editingLine" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    @click.self="handleClose">
    <div class="bg-zinc-900 rounded-lg p-6 w-250 max-h-[90vh] overflow-y-auto flex gap-6">
      <!-- 左侧：TextLine 编辑 -->
      <div class="flex-1">
        <h2 class="text-lg font-bold mb-4">编辑 TextLine</h2>

        <!-- 位置提示 -->
        <div class="mb-3 text-sm text-zinc-500">
          全局位置: 第 {{ globalPosition.index + 1 }} / {{ globalPosition.total }} 行
          <span v-if="globalPosition.isFirst" class="ml-2 text-yellow-500">(全局第一行)</span>
          <span v-if="globalPosition.isLast" class="ml-2 text-yellow-500">(全局最后一行)</span>
        </div>

        <!-- 图片预览区 -->
        <div class="mb-4 bg-zinc-800 rounded-lg p-2 flex justify-center">
          <canvas ref="canvasRef" class="max-w-full cursor-crosshair" :style="{ maxHeight: '250px' }"
            @mousedown="handleCanvasMouseDown" @mousemove="handleCanvasMouseMove" @mouseup="handleCanvasMouseUp"
            @mouseleave="handleCanvasMouseUp" />
        </div>

        <!-- 当前选中提示 -->
        <div class="mb-4 text-center">
          <span v-if="selectedGlyph" class="text-zinc-300">
            当前选中:
            <span class="text-2xl font-bold text-blue-400 mx-2">{{ selectedGlyph.content }}</span>
            <span class="text-zinc-500 text-base">
              (位置: {{ selectedGlyph.hpos }}, {{ selectedGlyph.vpos }})
            </span>
          </span>
          <span v-else class="text-zinc-500">点击下方字符进行选择</span>
        </div>
        <!-- 字符操作区 -->
        <div v-if="selectedGlyph" class="mb-4 space-y-3">
          <!-- 删除和插入 -->
          <div class="flex items-center justify-center gap-2">
            <button class="px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-base" @click="deleteCharacter">
              删除
            </button>
            <button class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-base" @click="insertCharacterBefore">
              ← 前插入
            </button>
            <button class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-base" @click="insertCharacterAfter">
              后插入 →
            </button>
          </div>


          <!-- 加帽子 -->
          <div class="flex items-center justify-center gap-2">
            <span class="text-zinc-400 text-base">加帽子:</span>
            <input v-model="diacriticChar" type="text" maxlength="1"
              class="w-12 px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 text-center focus:outline-none focus:border-blue-500"
              placeholder="~" title="可用: ~ ^ ` ' &quot; o - u v . ," />
            <button
              class="px-3 py-1 bg-purple-600 hover:bg-purple-500 rounded text-base disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!diacriticChar" @click="addDiacritic">
              添加
            </button>
            <span class="text-zinc-500 text-xs">~ ^ ` ' " o - u v . ,</span>
          </div>
        </div>
        <!-- 字符替换 -->
        <div v-if="selectedGlyph" class="mb-4 flex items-center justify-center gap-2">
          <span class="text-zinc-400 text-base">替换为:</span>
          <input v-model="replaceChar" type="text" maxlength="1"
            class="w-12 px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 text-center focus:outline-none focus:border-blue-500"
            placeholder="?" />
          <button
            class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-base disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="!replaceChar" @click="replaceCharacter">
            替换
          </button>
        </div>

        <!-- 断开操作 -->
        <div v-if="selectedGlyph" class="mb-4 flex items-center justify-center gap-2">
          <button
            class="px-3 py-1 bg-orange-600 hover:bg-orange-500 rounded text-base disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="!canSplitBefore"
            :title="globalPosition.isFirst ? '全局第一行不能向前断开' : (selectedGlyphIndex === 0 ? '第一个字符不能向前断开' : '将此字符之前的内容移到上一行')"
            @click="handleSplitBefore">
            ← 断开到前一行
          </button>
          <button
            class="px-3 py-1 bg-orange-600 hover:bg-orange-500 rounded text-base disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="!canSplitAfter"
            :title="globalPosition.isLast ? '全局最后一行不能向后断开' : (selectedGlyphIndex === editingLine.glyphs.length - 1 ? '最后一个字符不能向后断开' : '将此字符之后的内容移到下一行')"
            @click="handleSplitAfter">
            断开到后一行 →
          </button>
        </div>

        <!-- 字符列表 -->
        <div class="mb-4 p-3 bg-zinc-800 rounded-lg">
          <div class="text-zinc-500 text-sm mb-2">点击字符进行选择和编辑:</div>
          <div class="flex flex-wrap gap-1">
            <button v-for="(glyph, index) in editingLine.glyphs" :key="glyph.id" :class="[
              'px-2 py-1 rounded text-lg transition-colors',
              selectedGlyphIndex === index
                ? 'bg-blue-600 text-white'
                : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
            ]" @click="selectGlyph(index)">
              {{ glyph.content }}
            </button>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="flex justify-end gap-2">
          <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-base" @click="handleClose">
            取消
          </button>
          <button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-base" @click="handleSave">
            保存
          </button>
        </div>
      </div>

      <!-- 右侧：句子信息 -->
      <div class="w-75 border-l border-zinc-700 pl-6">
        <h3 class="text-md font-bold mb-4">句子信息</h3>

        <div v-if="sentence" class="space-y-4">
          <!-- 句子类型 -->
          <div>
            <label class="block text-base text-zinc-400 mb-2">句子类型</label>
            <div class="flex flex-col gap-2">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="sentenceForm.type" value="text" class="w-4 h-4 accent-blue-500" />
                <span class="text-zinc-300 text-base">正文</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="sentenceForm.type" value="title" class="w-4 h-4 accent-yellow-500" />
                <span class="text-zinc-300 text-base">标题</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="sentenceForm.type" value="note" class="w-4 h-4 accent-green-500" />
                <span class="text-zinc-300 text-base">注释</span>
              </label>
            </div>
          </div>

          <!-- 正则原文 -->
          <div>
            <label class="block text-base text-zinc-400 mb-1">正则原文</label>
            <textarea v-model="sentenceForm.normalized"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 text-base focus:outline-none focus:border-blue-500 resize-none"
              rows="2" placeholder="Normalized..." />
          </div>

          <!-- 英文翻译 -->
          <div>
            <label class="block text-base text-zinc-400 mb-1">英文翻译</label>
            <textarea v-model="sentenceForm.en"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 text-base focus:outline-none focus:border-blue-500 resize-none"
              rows="2" placeholder="English..." />
          </div>

          <!-- 中文翻译 -->
          <div>
            <label class="block text-base text-zinc-400 mb-1">中文翻译</label>
            <textarea v-model="sentenceForm.zh"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 text-base focus:outline-none focus:border-blue-500 resize-none"
              rows="2" placeholder="中文..." />
          </div>
        </div>

        <div v-else class="text-zinc-500 text-base">
          该 TextLine 未关联到任何句子
        </div>
      </div>
    </div>
  </div>
</template>