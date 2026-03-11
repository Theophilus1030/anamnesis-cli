<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

// --- 接口定义 (必须与 Page.vue 保持一致) ---
interface Glyph {
  id: string
  content: string
  hpos: number
  vpos: number
  width: number
  height: number
  lineId: string
  wordId: string // 核心字段：标记属于哪个单词
  isSpace?: boolean // 核心字段：标记是否是空格
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

// 前端计算用的单词结构
interface Word {
  id: string
  glyphs: Glyph[]
  content: string
  rect: { x: number, y: number, w: number, h: number }
  isSpace: boolean
  isFirst: boolean // 是否是本行第一个单词
  isLast: boolean  // 是否是本行最后一个单词
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
  (e: 'link-subs', lineId: string, wordId: string): void // 新增：跨行链接事件
  (e: 'update-sentence', data: { normalized: string; en: string; zh: string; type: 'text' | 'title' | 'note' }): void
}>()

// --- 状态定义 ---
const editingLine = ref<Line | null>(null)
const selectedGlyphIndex = ref<number | null>(null)
const selectedWordId = ref<string | null>(null) // 新增：当前选中的单词 ID

// 拖拽状态 (保留原有功能)
const draggingCorner = ref<'tl' | 'tr' | 'bl' | 'br' | null>(null)

// 编辑表单
const replaceChar = ref('')
const diacriticChar = ref('')
const sentenceForm = ref({
  normalized: '', en: '', zh: '', type: 'text' as 'text' | 'title' | 'note'
})

// Canvas 相关
const canvasRef = ref<HTMLCanvasElement | null>(null)
const imageRef = ref<HTMLImageElement | null>(null)
const imageLoaded = ref(false)

// --- 计算属性 ---

// 1. 全局位置计算 (保留)
const globalLineOrder = computed(() => {
  const order: string[] = []
  for (const sentence of props.sentences) {
    for (const lineId of sentence.line_ids) {
      order.push(lineId)
    }
  }
  return order
})

const globalPosition = computed(() => {
  if (!props.line) return { index: -1, total: 0, isFirst: true, isLast: true }
  const index = globalLineOrder.value.indexOf(props.line.id)
  const total = globalLineOrder.value.length
  return { index, total, isFirst: index === 0, isLast: index === total - 1 }
})

// --- 修正后的 words 计算属性 ---
const words = computed<Word[]>(() => {
  if (!editingLine.value || editingLine.value.glyphs.length === 0) return []
  
  const result: Word[] = []
  const glyphs = editingLine.value.glyphs // 获取所有 glyphs 以便查找下一个
  
  let currentWordId = glyphs[0].wordId
  let currentGroup: Glyph[] = []

  const flush = () => {
    if (currentGroup.length === 0) return
    const first = currentGroup[0]
    
    // 计算单词包围盒
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    let content = ''
    
    currentGroup.forEach(g => {
      minX = Math.min(minX, g.hpos)
      minY = Math.min(minY, g.vpos)
      
      // --- 关键修复开始 ---
      // 我们不能直接用 g.width，因为 CATMuS 输出为 0
      // 我们需要找到 g 在整个 glyphs 数组中的索引，看看后面有没有字符
      const idx = glyphs.indexOf(g)
      const nextGlyph = glyphs[idx + 1]
      
      // 如果有下一个字符，宽度 = 下一个字符hpos - 当前hpos
      // 如果没有下一个字符（行尾），用 g.width 兜底，如果还是0，给个 20px
      const effectiveWidth = nextGlyph 
        ? (nextGlyph.hpos - g.hpos) 
        : Math.max(g.width, 20)
        
      maxX = Math.max(maxX, g.hpos + effectiveWidth)
      // --- 关键修复结束 ---

      // 高度兜底：如果高度为0，给50，否则框是扁的
      const effectiveHeight = g.height || 50
      maxY = Math.max(maxY, g.vpos + effectiveHeight)
      
      content += g.content
    })

    result.push({
      id: currentWordId,
      glyphs: [...currentGroup],
      content,
      isSpace: !!first.isSpace,
      rect: { x: minX, y: minY, w: maxX - minX, h: maxY - minY },
      isFirst: false, 
      isLast: false
    })
  }

  glyphs.forEach(g => {
    if (g.wordId !== currentWordId) {
      flush()
      currentWordId = g.wordId
      currentGroup = []
    }
    currentGroup.push(g)
  })
  flush()

  if (result.length > 0) {
    result[0].isFirst = true
    result[result.length - 1].isLast = true
  }
  return result
})

// 当前选中的对象
const selectedWord = computed(() => {
  return words.value.find(w => w.id === selectedWordId.value) || null
})

const selectedGlyph = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return null
  return editingLine.value.glyphs[selectedGlyphIndex.value]
})

// 检查是否有下一行 (用于 SUBS 按钮状态)
const hasNextLine = computed(() => {
  if (!props.line) return false
  
  // 优先使用拓扑面板排序的全局阅读顺序
  const globalIdx = globalLineOrder.value.indexOf(props.line.id)
  if (globalIdx !== -1) {
    return globalIdx < globalLineOrder.value.length - 1
  }
  
  // 兜底：如果这行是未分配的游离行（不在句子中），退回使用数组顺序
  const rawIdx = props.allLines.findIndex(l => l.id === props.line!.id)
  return rawIdx !== -1 && rawIdx < props.allLines.length - 1
})

// 计算断开按钮状态 (保留原有逻辑)
const canSplitAfter = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return false
  if (globalPosition.value.isLast) return false
  if (selectedGlyphIndex.value === editingLine.value.glyphs.length - 1) return false
  return true
})

const canSplitBefore = computed(() => {
  if (selectedGlyphIndex.value === null || !editingLine.value) return false
  if (globalPosition.value.isFirst) return false
  if (selectedGlyphIndex.value === 0) return false
  return true
})

// 计算多边形边界 (用于 Canvas 视口)
const polygonBounds = computed(() => {
  if (!editingLine.value || editingLine.value.polygon.length === 0) {
    // 兜底：如果没有多边形，用 glyphs 算
    if (editingLine.value?.glyphs.length) {
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
        editingLine.value.glyphs.forEach(g => {
            minX = Math.min(minX, g.hpos)
            minY = Math.min(minY, g.vpos)
            maxX = Math.max(maxX, g.hpos + g.width)
            maxY = Math.max(maxY, g.vpos + g.height)
        })
        const padding = 20
        return { 
            minX: Math.max(0, minX - padding), 
            minY: Math.max(0, minY - padding), 
            maxX: maxX + padding, maxY: maxY + padding, 
            width: maxX - minX + padding * 2, height: maxY - minY + padding * 2 
        }
    }
    return { minX: 0, minY: 0, maxX: 200, maxY: 100, width: 200, height: 100 }
  }

  const points = editingLine.value.polygon
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const [x, y] of points) {
    minX = Math.min(minX, x); minY = Math.min(minY, y)
    maxX = Math.max(maxX, x); maxY = Math.max(maxY, y)
  }
  const padding = 20
  minX = Math.max(0, minX - padding)
  minY = Math.max(0, minY - padding)
  maxX += padding; maxY += padding
  return { minX, minY, maxX, maxY, width: maxX - minX, height: maxY - minY }
})


// --- 初始化与加载 ---
watch(() => props.visible, (newVal) => {
  if (newVal && props.line) {
    editingLine.value = JSON.parse(JSON.stringify(props.line))
    selectedGlyphIndex.value = null
    selectedWordId.value = null
    replaceChar.value = ''
    imageLoaded.value = false

    if (props.sentence) {
      sentenceForm.value = {
        normalized: props.sentence.normalized || '',
        en: props.sentence.en || '',
        zh: props.sentence.zh || '',
        type: props.sentence.type || 'text'
      }
    }
    nextTick(() => loadImage())
  }
}, { immediate: true })

function loadImage() {
  if (!props.imageUrl || !canvasRef.value) return
  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.onload = () => { imageRef.value = img; imageLoaded.value = true; drawCanvas() }
  img.src = props.imageUrl
}

// --- 核心绘制逻辑 ---
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

  // 1. 画背景图
  ctx.drawImage(img, bounds.minX, bounds.minY, bounds.width, bounds.height, 0, 0, bounds.width, bounds.height)
  
  // 遮罩层
  ctx.fillStyle = 'rgba(0, 0, 0, 0.5)'
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  // 剪裁区域 (只显示当前行)
  ctx.save()
  ctx.beginPath()
  const polygon = editingLine.value.polygon
  if (polygon.length > 0) {
    ctx.moveTo(polygon[0][0] - bounds.minX, polygon[0][1] - bounds.minY)
    for (let i = 1; i < polygon.length; i++) ctx.lineTo(polygon[i][0] - bounds.minX, polygon[i][1] - bounds.minY)
    ctx.closePath()
  }
  ctx.clip()
  ctx.drawImage(img, bounds.minX, bounds.minY, bounds.width, bounds.height, 0, 0, bounds.width, bounds.height)
  ctx.restore()

  // 2. 画基线
  const baseline = editingLine.value.baseline
  if (baseline.length > 1) {
    ctx.beginPath()
    ctx.moveTo(baseline[0][0] - bounds.minX, baseline[0][1] - bounds.minY)
    for (let i = 1; i < baseline.length; i++) ctx.lineTo(baseline[i][0] - bounds.minX, baseline[i][1] - bounds.minY)
    ctx.strokeStyle = 'rgba(0, 0, 255, 0.6)'
    ctx.lineWidth = 1
    ctx.stroke()
  }

  // 3. 画单词框 (Words)
  words.value.forEach(word => {
    if (word.isSpace) return // 空格不画框

    const x = word.rect.x - bounds.minX
    const y = word.rect.y - bounds.minY
    const w = word.rect.w
    const h = word.rect.h

    ctx.beginPath()
    const isSelected = word.id === selectedWordId.value
    
    // 样式逻辑
    // 1. 如果选中且是行尾：右边开口（示意 SUBS 可能）
    // 2. 如果选中且是行首：左边开口
    // 3. 普通情况：闭合矩形
    if (word.isLast && isSelected) {
       // 右开口
       ctx.moveTo(x, y); ctx.lineTo(x, y+h); ctx.lineTo(x+w, y+h); ctx.moveTo(x+w, y); ctx.lineTo(x, y)
    } else if (word.isFirst && isSelected) {
       // 左开口
       ctx.moveTo(x, y); ctx.lineTo(x+w, y); ctx.lineTo(x+w, y+h); ctx.lineTo(x, y+h)
    } else {
       ctx.rect(x, y, w, h)
    }

    ctx.strokeStyle = isSelected ? 'rgba(0, 255, 0, 1)' : 'rgba(0, 255, 255, 0.6)'
    ctx.lineWidth = isSelected ? 2 : 1
    ctx.stroke()

    // 显示单词内容 (调试辅助)
    if (isSelected) {
      ctx.font = '24px sans-serif'
      ctx.fillStyle = 'rgba(0, 255, 0, 1)'
      ctx.fillText(word.content, x, y - 8)
    }
  })

  // 4. 画当前选中的字符 (Glyph) - 红色高亮 + 拖拽点
  if (selectedGlyph.value && selectedGlyphIndex.value !== null) {
    const g = selectedGlyph.value
    const x = g.hpos - bounds.minX
    const y = g.vpos - bounds.minY
    
    // --- 修复开始 ---
    // 不要用 const w = g.width
    const w = getGlyphWidth(selectedGlyphIndex.value) 
    // --- 修复结束 ---
    
    const h = g.height

    ctx.strokeStyle = 'rgba(255, 0, 0, 0.8)'
    ctx.lineWidth = 1.5
    ctx.strokeRect(x, y, w, h)
    // 绘制拖拽点 (四个角)
    const corners = [
      { x: x, y: y }, { x: x + w, y: y }, { x: x, y: y + h }, { x: x + w, y: y + h }
    ]
    for (const corner of corners) {
      ctx.beginPath()
      
      // 修改前: ctx.arc(corner.x, corner.y, 4, 0, Math.PI * 2)
      // 修改后: 半径改为 8 (直径16px)，甚至更大
      ctx.arc(corner.x, corner.y, 8, 0, Math.PI * 2) 
      
      ctx.fillStyle = 'white'
      ctx.fill()
      ctx.strokeStyle = 'red'
      ctx.lineWidth = 1 // 边框线宽保持 1 或改为 2 均可
      ctx.stroke()
    }
  }
}

// --- 交互处理：点击、拖拽 ---

function selectGlyph(index: number) {
  selectedGlyphIndex.value = index
  if (editingLine.value) {
    selectedWordId.value = editingLine.value.glyphs[index].wordId
  }
  nextTick(() => drawCanvas())
}

function handleCanvasMouseDown(e: MouseEvent) {
  if (!editingLine.value || !canvasRef.value) return
  
  const canvas = canvasRef.value
  const rect = canvas.getBoundingClientRect()
  const scaleX = canvas.width / rect.width
  const scaleY = canvas.height / rect.height
  const mouseX = (e.clientX - rect.left) * scaleX 
  const mouseY = (e.clientY - rect.top) * scaleY 
  
  const bounds = polygonBounds.value
  const absX = mouseX + bounds.minX
  const absY = mouseY + bounds.minY

  // 1. 检查是否点中当前选中字符的拖拽点 (保留原有逻辑)
  if (selectedGlyph.value) {
    const g = selectedGlyph.value
    const gx = g.hpos - bounds.minX
    const gy = g.vpos - bounds.minY
    const corners = [
      { x: gx, y: gy, key: 'tl' as const },
      { x: gx + g.width, y: gy, key: 'tr' as const },
      { x: gx, y: gy + g.height, key: 'bl' as const },
      { x: gx + g.width, y: gy + g.height, key: 'br' as const }
    ]
    for (const corner of corners) {
      if (Math.hypot(mouseX - corner.x, mouseY - corner.y) <= 20) {
        draggingCorner.value = corner.key
        return // 开始拖拽
      }
    }
  }

  // 2. 检查点击了哪个 Glyph
  const index = editingLine.value.glyphs.findIndex((g, idx) => { // 注意这里加了 idx 参数
    // --- 修复开始 ---
    const effectiveWidth = getGlyphWidth(idx)
    return absX >= g.hpos && absX <= g.hpos + effectiveWidth && // 使用有效宽度
           absY >= g.vpos && absY <= g.vpos + g.height
    // --- 修复结束 ---
  })

  if (index !== -1) {
    selectGlyph(index)
  } else {
    selectedGlyphIndex.value = null
    selectedWordId.value = null
    drawCanvas()
  }
}

function handleCanvasMouseMove(e: MouseEvent) {
  if (!draggingCorner.value || selectedGlyphIndex.value === null || !editingLine.value || !canvasRef.value) return

  const canvas = canvasRef.value
  const rect = canvas.getBoundingClientRect()
  const scaleX = canvas.width / rect.width
  const scaleY = canvas.height / rect.height
  const mouseX = (e.clientX - rect.left) * scaleX
  const mouseY = (e.clientY - rect.top) * scaleY

  const bounds = polygonBounds.value
  const glyph = editingLine.value.glyphs[selectedGlyphIndex.value]
  const absX = mouseX + bounds.minX
  const absY = mouseY + bounds.minY

  // 拖拽逻辑 (保持原有逻辑)
  switch (draggingCorner.value) {
    case 'tl':
      const oldR = glyph.hpos + glyph.width
      const oldB = glyph.vpos + glyph.height
      glyph.hpos = Math.round(absX)
      glyph.vpos = Math.round(absY)
      glyph.width = Math.max(5, oldR - glyph.hpos)
      glyph.height = Math.max(5, oldB - glyph.vpos)
      break
    case 'tr':
      glyph.vpos = Math.round(absY)
      glyph.width = Math.max(5, Math.round(absX - glyph.hpos))
      glyph.height = Math.max(5, (glyph.vpos + glyph.height) - glyph.vpos) // 保持底边不变太复杂，简单调整高度
      // 修正：TR调整 width 和 vpos，height 应该基于新的 vpos 和旧的 bottom 计算
      const oldB_tr = glyph.vpos + glyph.height // 这里逻辑有点绕，简单处理：只调宽和y
      glyph.height = Math.max(5, oldB_tr - glyph.vpos) 
      break
    case 'bl':
      glyph.hpos = Math.round(absX)
      glyph.width = Math.max(5, (glyph.hpos + glyph.width) - glyph.hpos) // 保持右边不变
      // 修正：BL 调整 x 和 height
      const oldR_bl = glyph.hpos + glyph.width
      glyph.width = Math.max(5, oldR_bl - glyph.hpos)
      glyph.height = Math.max(5, Math.round(absY - glyph.vpos))
      break
    case 'br':
      glyph.width = Math.max(5, Math.round(absX - glyph.hpos))
      glyph.height = Math.max(5, Math.round(absY - glyph.vpos))
      break
  }
  drawCanvas()
}

function handleCanvasMouseUp() {
  draggingCorner.value = null
}

// --- 单词级操作逻辑 ---

// 1. 插入空格 (Split Word)
function splitWordAtGlyph() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return
  
  const currentGlyph = editingLine.value.glyphs[selectedGlyphIndex.value]
  const currentWord = words.value.find(w => w.id === currentGlyph.wordId)
  if (!currentWord) return

  const glyphs = editingLine.value.glyphs
  const newWordId = `word_${Date.now()}`
  const spaceId = `sp_${Date.now()}`
  
  // 找到切分点 (在当前选中字符之后)
  const splitIndex = selectedGlyphIndex.value + 1
  
  // 1. 插入空格 Glyph
  const spaceGlyph: Glyph = {
    id: spaceId, content: ' ', 
    hpos: currentGlyph.hpos + currentGlyph.width, 
    vpos: currentGlyph.vpos, width: 10, height: 0,
    lineId: editingLine.value.id,
    wordId: `word_${spaceId}`, // 空格独占 ID
    isSpace: true
  }
  
  glyphs.splice(splitIndex, 0, spaceGlyph)
  
  // 2. 修改空格之后的所有原单词字符的 wordId
  // 注意：需要判断字符是否属于原单词。简单做法是向后遍历直到遇到不同 wordId
  for (let i = splitIndex + 1; i < glyphs.length; i++) {
    const g = glyphs[i]
    if (g.wordId === currentWord.id) {
      g.wordId = newWordId
    } else {
      break // 遇到下一个单词了，停止
    }
  }
  
  drawCanvas()
}

// 2. 合并单词 (Merge with Next)
function mergeWithNextWord() {
  if (!selectedWordId.value || !editingLine.value) return
  
  const currentWordIdx = words.value.findIndex(w => w.id === selectedWordId.value)
  if (currentWordIdx === -1 || currentWordIdx >= words.value.length - 1) return
  
  const nextWord = words.value[currentWordIdx + 1]
  
  // 如果下一个是空格，先删掉空格
  if (nextWord.isSpace) {
    editingLine.value.glyphs = editingLine.value.glyphs.filter(g => g.wordId !== nextWord.id)
    // 删掉空格后，原来的 nextWord 没了，需要再次点击合并才能合并真正的下一个单词
    // 或者这里可以递归做，但为了稳妥，先只删空格
    nextTick(() => drawCanvas())
    return
  }
  
  // 合并：将 nextWord 的所有 glyph 的 wordId 改为 currentWordId
  editingLine.value.glyphs.forEach(g => {
    if (g.wordId === nextWord.id) {
      g.wordId = selectedWordId.value!
    }
  })
  drawCanvas()
}

// 3. 设为 SUBS (跨行链接)
function linkToNextLine() {
  if (!selectedWord.value || !editingLine.value) return
  // 只有行尾单词能连
  if (!selectedWord.value.isLast) return 
  if (!hasNextLine.value) {
    alert("没有下一行，无法链接")
    return
  }
  
  emit('link-subs', editingLine.value.id, selectedWord.value.id)
}

// --- 字符级操作逻辑 (保留原有) ---

function deleteCharacter() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return
  if (editingLine.value.glyphs.length <= 1) {
    alert('至少保留一个字符')
    return
  }
  editingLine.value.glyphs.splice(selectedGlyphIndex.value, 1)
  if (selectedGlyphIndex.value >= editingLine.value.glyphs.length) {
    selectedGlyphIndex.value = editingLine.value.glyphs.length - 1
  }
  nextTick(() => drawCanvas())
}


function insertCharacterBefore() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return
  const current = editingLine.value.glyphs[selectedGlyphIndex.value]
  
  // --- 恢复原版逻辑 ---
  const currentWidth = getGlyphWidth(selectedGlyphIndex.value) // 使用动态宽度
  const halfWidth = Math.round(currentWidth / 2)
  
  const newGlyph: Glyph = {
    id: `glyph_${Date.now()}`, 
    content: '?',
    hpos: current.hpos, 
    vpos: current.vpos, 
    width: halfWidth, 
    height: current.height, // 继承高度
    lineId: current.lineId,
    wordId: current.wordId,
    isSpace: false
  }
  
  // 原字符右移
  current.hpos += halfWidth
  // 注意：不需要修改 current.width，因为它是根据 hpos 动态算出来的
  
  editingLine.value.glyphs.splice(selectedGlyphIndex.value, 0, newGlyph)
  nextTick(() => drawCanvas())
}

function insertCharacterAfter() {
  if (selectedGlyphIndex.value === null || !editingLine.value) return
  const current = editingLine.value.glyphs[selectedGlyphIndex.value]
  
  // --- 恢复原版逻辑 ---
  const currentWidth = getGlyphWidth(selectedGlyphIndex.value)
  const halfWidth = Math.round(currentWidth / 2)
  
  // 新字符的位置是 当前位置 + 半宽
  const newHpos = current.hpos + halfWidth
  
  const newGlyph: Glyph = {
    id: `glyph_${Date.now()}`, 
    content: '?',
    hpos: newHpos, 
    vpos: current.vpos, 
    width: halfWidth, 
    height: current.height,
    lineId: current.lineId,
    wordId: current.wordId,
    isSpace: false
  }
  
  editingLine.value.glyphs.splice(selectedGlyphIndex.value + 1, 0, newGlyph)
  selectedGlyphIndex.value += 1
  nextTick(() => drawCanvas())
}

function createFirstCharacter() {
  if (!editingLine.value || editingLine.value.glyphs.length > 0) return
  const bounds = polygonBounds.value
  const newGlyph: Glyph = {
    id: `glyph_${Date.now()}`, content: '?',
    hpos: bounds.minX + 20, vpos: bounds.minY + 20, width: 30, height: 50,
    lineId: editingLine.value.id,
    wordId: `word_${Date.now()}`,
    isSpace: false
  }
  editingLine.value.glyphs.push(newGlyph)
  selectedGlyphIndex.value = 0
  nextTick(() => drawCanvas())
}

// 组合字符逻辑
const diacriticMap: { [key: string]: string } = {
  '~': '\u0303', '^': '\u0302', '`': '\u0300', "'": '\u0301', '"': '\u0308',
  'o': '\u030A', '-': '\u0304', 'u': '\u0306', 'v': '\u030C', '.': '\u0307', ',': '\u0327',
}
function addDiacritic() {
  if (selectedGlyphIndex.value === null || !editingLine.value || !diacriticChar.value) return
  const glyph = editingLine.value.glyphs[selectedGlyphIndex.value]
  let combining = diacriticMap[diacriticChar.value]
  if (!combining) {
    const code = diacriticChar.value.charCodeAt(0)
    if (code >= 0x0300 && code <= 0x036F) combining = diacriticChar.value
  }
  if (!combining) {
    alert(`无效的组合字符: "${diacriticChar.value}"`)
    return
  }
  const combined = (glyph.content + combining).normalize('NFC')
  glyph.content = combined.length > 1 && combined !== glyph.content ? combined.normalize('NFC') : combined
  diacriticChar.value = ''
  nextTick(() => drawCanvas())
}

function replaceCharacter() {
  if (selectedGlyphIndex.value === null || !editingLine.value || !replaceChar.value) return
  editingLine.value.glyphs[selectedGlyphIndex.value].content = replaceChar.value
  replaceChar.value = ''
  drawCanvas()
}

// 处理断开（移动）逻辑 (保留)
function handleSplitBefore() {
  if (!canSplitBefore.value || selectedGlyphIndex.value === null || !editingLine.value) return
  if (!confirm(`确定要将"${selectedGlyph.value?.content}"之前的字符移到上一个 TextLine 吗？`)) return
  emit('split-before', editingLine.value.id, selectedGlyphIndex.value)
}
function handleSplitAfter() {
  if (!canSplitAfter.value || selectedGlyphIndex.value === null || !editingLine.value) return
  if (!confirm(`确定要将"${selectedGlyph.value?.content}"之后的字符移到下一个 TextLine 吗？`)) return
  emit('split-after', editingLine.value.id, selectedGlyphIndex.value)
}

function handleSave() {
  if (editingLine.value) {
    emit('update-sentence', { ...sentenceForm.value })
    emit('save', JSON.parse(JSON.stringify(editingLine.value)))
  }
}
function handleClose() { emit('close') }

// 辅助函数：获取字符的有效宽度
function getGlyphWidth(index: number): number {
  if (!editingLine.value) return 20
  const glyphs = editingLine.value.glyphs
  const glyph = glyphs[index]
  const nextGlyph = glyphs[index + 1]
  
  // 同样的逻辑：如果有下一个，算间距；没有下一个，用自身宽度兜底
  return nextGlyph ? (nextGlyph.hpos - glyph.hpos) : Math.max(glyph.width, 20)
}

watch(selectedGlyphIndex, () => {
  if (imageLoaded.value) drawCanvas()
})
</script>

<template>
  <div v-if="visible && editingLine" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    @click.self="handleClose">
    <div class="bg-zinc-900 rounded-lg p-6 w-250 max-h-screen overflow-y-auto flex gap-6">
      
      <div class="flex-1">
        <h2 class="text-lg font-bold mb-4">编辑 TextLine</h2>

        <div class="mb-3 text-sm text-zinc-500">
          全局位置: 第 {{ globalPosition.index + 1 }} / {{ globalPosition.total }} 行
          <span v-if="globalPosition.isFirst" class="ml-2 text-yellow-500">(First)</span>
          <span v-if="globalPosition.isLast" class="ml-2 text-yellow-500">(Last)</span>
        </div>

        <div class="mb-4 bg-zinc-800 rounded-lg p-2 flex justify-center">
          <canvas ref="canvasRef" class="max-w-full cursor-crosshair" :style="{ maxHeight: '300px' }"
            @mousedown="handleCanvasMouseDown" @mousemove="handleCanvasMouseMove" @mouseup="handleCanvasMouseUp"
            @mouseleave="handleCanvasMouseUp" />
        </div>

        <div class="mb-4 text-center h-14">
          <div v-if="selectedWord">
            <div class="flex items-center justify-center gap-2">
                <span class="text-zinc-400 text-sm">选中单词:</span>
                <span :class="['text-xl font-bold px-2 rounded', selectedWord.isSpace ? 'bg-zinc-700 text-zinc-400' : 'bg-green-900 text-green-400']">
                {{ selectedWord.isSpace ? '[空格]' : selectedWord.content }}
                </span>
            </div>
            
            <div v-if="selectedGlyph" class="text-xs text-zinc-500 mt-1">
              选中字符: <span class="text-blue-400 font-bold text-base">{{ selectedGlyph.content }}</span>
              (Pos: {{ selectedGlyph.hpos }}, {{ selectedGlyph.vpos }})
            </div>
          </div>
          <div v-else class="text-zinc-500 leading-10">点击画面中的字符进行选择</div>
        </div>

        <div class="mb-4 p-3 bg-zinc-800/50 rounded border border-zinc-700">
          <div class="text-xs text-zinc-500 mb-2 font-bold uppercase">单词操作 (Word Ops)</div>
          <div class="flex flex-wrap gap-2">
            <button class="px-3 py-1 bg-zinc-700 hover:bg-zinc-600 rounded text-sm flex items-center gap-1 transition-colors"
              :disabled="!selectedGlyph" @click="splitWordAtGlyph" title="在选中字符后插入空格并切分单词">
              <span>✂️</span> 切分/插入空格
            </button>
            
            <button class="px-3 py-1 bg-zinc-700 hover:bg-zinc-600 rounded text-sm flex items-center gap-1 transition-colors"
              :disabled="!selectedWord" @click="mergeWithNextWord" title="与后一个单词合并 (自动删除中间的空格)">
              <span>🔗</span> 与后合并
            </button>

            <button class="px-3 py-1 bg-purple-700 hover:bg-purple-600 rounded text-sm flex items-center gap-1 disabled:opacity-50 transition-colors"
              :disabled="!selectedWord?.isLast || !hasNextLine" 
              @click="linkToNextLine"
              title="标记为跨行单词 (SUBS)，并连接到下一行">
              <span>⤵️</span> 设为跨行 (SUBS)
            </button>
          </div>
        </div>

        <div v-if="selectedGlyph" class="mb-4 space-y-3 p-3 bg-zinc-800 rounded">
          <div class="text-xs text-zinc-500 mb-2 font-bold uppercase">字符操作 (Glyph Ops)</div>
          
          <div class="flex items-center gap-2">
            <button class="px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-sm" @click="deleteCharacter">
              删除字符
            </button>
            <button class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-sm" @click="insertCharacterBefore">
              ← 前插
            </button>
            <button class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-sm" @click="insertCharacterAfter">
              后插 →
            </button>
          </div>

          <div class="flex items-center gap-2">
            <span class="text-zinc-400 text-sm">加帽子:</span>
            <input v-model="diacriticChar" type="text" maxlength="1"
              class="w-10 px-1 py-1 bg-zinc-900 border border-zinc-600 rounded text-zinc-100 text-center focus:outline-none focus:border-blue-500"
              placeholder="~" />
            <button
              class="px-3 py-1 bg-purple-600 hover:bg-purple-500 rounded text-sm disabled:opacity-50"
              :disabled="!diacriticChar" @click="addDiacritic">
              添加
            </button>
          </div>

          <div class="flex items-center gap-2">
            <span class="text-zinc-400 text-sm">替换为:</span>
            <input v-model="replaceChar" type="text" maxlength="1"
              class="w-10 px-1 py-1 bg-zinc-900 border border-zinc-600 rounded text-zinc-100 text-center focus:outline-none focus:border-blue-500"
              placeholder="?" />
            <button
              class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-sm disabled:opacity-50"
              :disabled="!replaceChar" @click="replaceCharacter">
              替换
            </button>
          </div>
        </div>

        <div class="mb-4 p-3 bg-zinc-800 rounded-lg max-h-32 overflow-y-auto">
          <div v-if="editingLine.glyphs.length > 0" class="flex flex-wrap gap-1">
            <button v-for="(glyph, index) in editingLine.glyphs" :key="glyph.id" :class="[
              'px-1.5 py-0.5 rounded text-lg transition-colors border',
              selectedGlyphIndex === index
                ? 'bg-blue-600 text-white border-blue-400'
                : (glyph.isSpace ? 'bg-zinc-700 border-zinc-600 text-zinc-500' : 'bg-zinc-700 text-zinc-300 border-transparent hover:bg-zinc-600')
            ]" @click="selectGlyph(index)">
              {{ glyph.isSpace ? '␣' : glyph.content }}
            </button>
          </div>
          <div v-else class="flex flex-col items-center gap-2 py-2">
            <span class="text-zinc-500 text-sm">空行</span>
            <button class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-sm" @click="createFirstCharacter">
              + 添加首字符
            </button>
          </div>
        </div>

        <div class="flex justify-between items-center mt-auto pt-4 border-t border-zinc-700">
           <div class="flex gap-2">
              <button class="px-3 py-1.5 bg-orange-700 hover:bg-orange-600 rounded text-sm disabled:opacity-30 disabled:cursor-not-allowed"
                :disabled="!canSplitBefore" @click="handleSplitBefore" title="将选中字符及之前的内容移到上一行">
                ← 移到上一行
              </button>
              <button class="px-3 py-1.5 bg-orange-700 hover:bg-orange-600 rounded text-sm disabled:opacity-30 disabled:cursor-not-allowed"
                :disabled="!canSplitAfter" @click="handleSplitAfter" title="将选中字符之后的内容移到下一行">
                移到下一行 →
              </button>
           </div>

           <div class="flex gap-2">
              <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-base" @click="handleClose">
                取消
              </button>
              <button class="px-4 py-2 bg-green-600 hover:bg-green-500 rounded text-base font-bold" @click="handleSave">
                保存修改
              </button>
           </div>
        </div>
      </div>

      <div class="w-75 border-l border-zinc-700 pl-6 flex flex-col">
        <h3 class="text-md font-bold mb-4">句子信息</h3>
        <div v-if="sentence" class="space-y-4 flex-1">
          <div>
            <label class="block text-base text-zinc-400 mb-2">类型</label>
            <div class="flex gap-4">
              <label class="flex items-center gap-2 cursor-pointer"><input type="radio" v-model="sentenceForm.type" value="text" class="accent-blue-500"> 正文</label>
              <label class="flex items-center gap-2 cursor-pointer"><input type="radio" v-model="sentenceForm.type" value="title" class="accent-yellow-500"> 标题</label>
              <label class="flex items-center gap-2 cursor-pointer"><input type="radio" v-model="sentenceForm.type" value="note" class="accent-green-500"> 注释</label>
            </div>
          </div>
          <div>
            <label class="block text-sm text-zinc-400 mb-1">原文 (Normalized)</label>
            <textarea v-model="sentenceForm.normalized" class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:border-blue-500 resize-none h-24" placeholder="Normalized..." />
          </div>
          <div>
            <label class="block text-sm text-zinc-400 mb-1">English</label>
            <textarea v-model="sentenceForm.en" class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:border-blue-500 resize-none h-24" placeholder="English..." />
          </div>
          <div>
            <label class="block text-sm text-zinc-400 mb-1">中文</label>
            <textarea v-model="sentenceForm.zh" class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:border-blue-500 resize-none h-24" placeholder="中文..." />
          </div>
        </div>
        <div v-else class="text-zinc-500">无关联句子</div>
      </div>

    </div>
  </div>
</template>