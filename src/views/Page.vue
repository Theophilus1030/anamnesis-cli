<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import TopologyPanel from '@/components/TopologyPanel.vue'
import SentenceEditor from '@/components/SentenceEditor.vue'
import TextLineEditor from '@/components/TextLineEditor.vue'
const route = useRoute()
const router = useRouter()
const projectId = route.params.projectId as string
const pageId = route.params.pageId as string
const editMode = ref<'none' | 'polygon' | 'baseline'>('none')
// 补足基线时的目标 Line ID
const targetMissingBaselineId = ref<string | null>(null)
// --- 新增状态 ---
const isCreating = ref(false) // 是否处于新建模式
const tempPoints = ref<number[][]>([]) // 新建模式下正在画的点
const draggingPointIndex = ref<number | null>(null) // 当前正在拖拽的顶点的索引
const editSnapshot = ref<string | null>(null) // 进入编辑模式前的数据快照（用于“放弃”操作）

// 样式配置
const STYLE = {
  lineWidth: 2,         
  selectedLineWidth: 3, 
  pointRadius: 6,       
  pointHitRadius: 10,   
  baselineColor: 'rgba(0, 0, 255, 1)',
  baselineSelectedColor: 'rgba(255, 165, 0, 1)', // 橙色高亮基线
}

// 辅助：检查是否正在进行任何未保存的操作
const isBusy = computed(() => {
  return isCreating.value ||
    (editMode.value !== 'none' && selectedLineId.value !== null) ||
    targetMissingBaselineId.value !== null
})

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
const hoveredLineId = ref<string | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)
const imageRef = ref<HTMLImageElement | null>(null)
const scale = ref(0.5)
const showBaselines = ref(true)
const showPolygons = ref(false)
const history = ref<HistoryState[]>([])
const historyIndex = ref(-1)

// 显示控制：正则原文、英文、中文
const showNormalized = ref(true)
const showEnglish = ref(true)
const showChinese = ref(true)
// 编辑 TextLine 弹窗
const editingTextLine = ref<Line | null>(null)
const showTextLineEditor = ref(false)
// 编辑句子弹窗
const editingSentence = ref<Sentence | null>(null)
const showSentenceEditor = ref(false)

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

// 计算点(px, py)到线段(x1, y1)-(x2, y2)的距离
function getDistanceToSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number) {
  const A = px - x1;
  const B = py - y1;
  const C = x2 - x1;
  const D = y2 - y1;

  const dot = A * C + B * D;
  const lenSq = C * C + D * D;
  let param = -1;
  if (lenSq !== 0) param = dot / lenSq;

  let xx, yy;

  if (param < 0) {
    xx = x1; yy = y1;
  } else if (param > 1) {
    xx = x2; yy = y2;
  } else {
    xx = x1 + param * C;
    yy = y1 + param * D;
  }

  const dx = px - xx;
  const dy = py - yy;
  return Math.sqrt(dx * dx + dy * dy);
}

// 检测线段 (p1-p2) 和 (p3-p4) 是否相交
function isLineIntersect(p1: number[], p2: number[], p3: number[], p4: number[]): boolean {
  const [x1, y1] = p1; const [x2, y2] = p2;
  const [x3, y3] = p3; const [x4, y4] = p4;

  // 快速排斥实验
  if (Math.max(x1, x2) < Math.min(x3, x4) || Math.max(y1, y2) < Math.min(y3, y4) ||
    Math.max(x3, x4) < Math.min(x1, x2) || Math.max(y3, y4) < Math.min(y1, y2)) return false;

  // 跨立实验
  const cross = (a: number[], b: number[], c: number[]) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);

  if (cross(p1, p2, p3) * cross(p1, p2, p4) > 0) return false;
  if (cross(p3, p4, p1) * cross(p3, p4, p2) > 0) return false;

  return true;
}

// 校验多边形是否自交 (True表示合法，False表示有重叠)
function validatePolygon(points: number[][]): boolean {
  if (points.length < 3) return false; // 少于3个点构不成多边形

  const len = points.length;
  // 遍历所有非相邻边，检查是否相交
  for (let i = 0; i < len; i++) {
    const p1 = points[i];
    const p2 = points[(i + 1) % len]; // 当前边

    // 从 i+2 开始检查，注意避开相邻边和首尾相接的情况
    for (let j = i + 2; j < len; j++) {
      if (i === 0 && j === len - 1) continue; // 忽略首尾边

      const p3 = points[j];
      const p4 = points[(j + 1) % len]; // 待查边

      if (isLineIntersect(p1, p2, p3, p4)) {
        return false; // 发现相交
      }
    }
  }
  return true;
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

  // 清空画布
  ctx.clearRect(0, 0, canvas.width, canvas.height)

  // ============================================================
  // Layer 0: 区域 Regions (最底层背景，绿色)
  // ============================================================
  if (showPolygons.value) {
    for (const region of regions.value) {
      if (region.polygon.length > 0) {
        ctx.beginPath()
        ctx.moveTo(region.polygon[0][0], region.polygon[0][1])
        for (let i = 1; i < region.polygon.length; i++) {
          ctx.lineTo(region.polygon[i][0], region.polygon[i][1])
        }
        ctx.closePath()
        ctx.strokeStyle = 'rgba(0, 255, 128, 0.5)'
        ctx.lineWidth = 2
        ctx.stroke()
      }
    }
  }

  // ============================================================
  // Layer 1: 基线 Baselines (位于多边形下方)
  // ============================================================
  if (showBaselines.value) {
    for (const line of lines.value) {
      if (!line.baseline || line.baseline.length < 2) continue

      // 只有在【基线编辑模式】且【选中该行】时，才高亮并显示点
      const isSelected = line.id === selectedLineId.value && editMode.value === 'baseline'

      ctx.beginPath()
      ctx.moveTo(line.baseline[0][0], line.baseline[0][1])
      for (let i = 1; i < line.baseline.length; i++) {
        ctx.lineTo(line.baseline[i][0], line.baseline[i][1])
      }

      // 样式区分：选中时为橙色，未选中为蓝色
      ctx.strokeStyle = isSelected ? STYLE.baselineSelectedColor : STYLE.baselineColor
      ctx.lineWidth = isSelected ? STYLE.selectedLineWidth : STYLE.lineWidth
      ctx.stroke()

      // 如果选中，绘制基线控制点 (实心白点)
      if (isSelected) {
        for (const point of line.baseline) {
          ctx.beginPath()
          ctx.arc(point[0], point[1], STYLE.pointRadius, 0, Math.PI * 2)
          ctx.fillStyle = 'white'
          ctx.fill()
          ctx.strokeStyle = STYLE.baselineSelectedColor
          ctx.lineWidth = 2
          ctx.stroke()
        }
      }
    }
  }

  // ============================================================
  // Layer 2: 多边形 Polygons (位于基线层上方，填充以实现遮挡)
  // ============================================================
  if (showPolygons.value) {
    for (const line of lines.value) {
      if (line.polygon.length > 0) {
        // 只有在【多边形编辑模式】且【选中该行】时，才高亮并显示点
        // 注意：如果在自由选择模式点击了多边形，editMode 会自动变成 'polygon'
        const isSelected = line.id === selectedLineId.value && editMode.value === 'polygon'

        ctx.beginPath()
        ctx.moveTo(line.polygon[0][0], line.polygon[0][1])
        for (let i = 1; i < line.polygon.length; i++) {
          ctx.lineTo(line.polygon[i][0], line.polygon[i][1])
        }
        ctx.closePath()

        if (isSelected) {
          ctx.fillStyle = 'rgba(255, 0, 255, 0.3)' // 较深的填充
          ctx.strokeStyle = 'rgba(255, 0, 255, 1)'
          ctx.lineWidth = STYLE.selectedLineWidth
        } else {
          ctx.fillStyle = 'rgba(255, 0, 255, 0.15)' // 浅填充，遮挡基线点击
          ctx.strokeStyle = 'rgba(255, 0, 255, 0.8)'
          ctx.lineWidth = STYLE.lineWidth
        }
        ctx.fill()
        ctx.stroke()

        // 如果选中，绘制多边形控制点 (空心/实心点)
        if (isSelected) {
          for (const point of line.polygon) {
            ctx.beginPath()
            ctx.arc(point[0], point[1], STYLE.pointRadius, 0, Math.PI * 2)
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

  // ============================================================
  // Layer 3: 字符框 Glyphs (辅助显示)
  // ============================================================
  // 仅在非新建、非补足模式下显示字符框，避免干扰
  if (!isCreating.value && !targetMissingBaselineId.value) {
    for (const line of lines.value) {
      const glyphs = line.glyphs
      for (let i = 0; i < glyphs.length; i++) {
        const glyph = glyphs[i]
        const isSelected = glyph.id === selectedGlyphId.value
        // 只有选中时才画，或者你想一直画也可以，这里保留选中高亮逻辑
        if (isSelected) {
          const rect = getGlyphRect(glyph, i, glyphs)
          ctx.fillStyle = 'rgba(59, 130, 246, 0.3)'
          ctx.fillRect(rect.x, rect.y, rect.width, rect.height)
          ctx.strokeStyle = 'rgba(59, 130, 246, 1)'
          ctx.lineWidth = 2
          ctx.strokeRect(rect.x, rect.y, rect.width, rect.height)
        }
      }
    }
  }

  // ============================================================
  // Layer 4: 临时编辑层 (新建/补足)
  // ============================================================

  // A. 补足基线模式 (Fix Missing Baseline)
  if (targetMissingBaselineId.value) {
    // 1. 绘制目标多边形的轮廓（作为底图参考），即使 showPolygons=false 也要画
    const targetLine = lines.value.find(l => l.id === targetMissingBaselineId.value)
    if (targetLine && targetLine.polygon.length > 2) {
      ctx.beginPath()
      ctx.moveTo(targetLine.polygon[0][0], targetLine.polygon[0][1])
      for (let i = 1; i < targetLine.polygon.length; i++) {
        ctx.lineTo(targetLine.polygon[i][0], targetLine.polygon[i][1])
      }
      ctx.closePath()
      ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)' // 红色虚线或淡线
      ctx.fillStyle = 'rgba(255, 0, 0, 0.05)'
      ctx.lineWidth = 1
      ctx.fill()
      ctx.stroke()
    }

    // 2. 绘制正在画的基线 (tempPoints)
    if (tempPoints.value.length > 0) {
      ctx.beginPath()
      ctx.moveTo(tempPoints.value[0][0], tempPoints.value[0][1])
      for (let i = 1; i < tempPoints.value.length; i++) {
        ctx.lineTo(tempPoints.value[i][0], tempPoints.value[i][1])
      }
      // 基线不闭合
      ctx.strokeStyle = STYLE.baselineSelectedColor // 使用基线高亮色
      ctx.lineWidth = STYLE.selectedLineWidth
      ctx.stroke()

      // 绘制点
      for (const point of tempPoints.value) {
        ctx.beginPath()
        ctx.arc(point[0], point[1], STYLE.pointRadius, 0, Math.PI * 2)
        ctx.fillStyle = 'white'
        ctx.fill()
        ctx.strokeStyle = STYLE.baselineSelectedColor
        ctx.lineWidth = 2
        ctx.stroke()
      }
    }
  }

  // B. 新建多边形模式 (Create Polygon)
  if (isCreating.value && tempPoints.value.length > 0) {
    // 连线
    ctx.beginPath()
    ctx.moveTo(tempPoints.value[0][0], tempPoints.value[0][1])
    for (let i = 1; i < tempPoints.value.length; i++) {
      ctx.lineTo(tempPoints.value[i][0], tempPoints.value[i][1])
    }
    // 如果点数 > 2，闭合预览
    if (tempPoints.value.length > 2) {
      ctx.closePath()
      ctx.fillStyle = 'rgba(0, 255, 0, 0.2)' // 绿色填充表示新建
      ctx.fill()
    }
    ctx.strokeStyle = '#00ff00'
    ctx.lineWidth = STYLE.selectedLineWidth
    ctx.stroke()

    // 绘制点
    for (const point of tempPoints.value) {
      ctx.beginPath()
      ctx.arc(point[0], point[1], STYLE.pointRadius, 0, Math.PI * 2)
      ctx.fillStyle = '#00ff00'
      ctx.fill()
      ctx.strokeStyle = 'white'
      ctx.lineWidth = 2
      ctx.stroke()
    }
  }

  // ============================================================
  // Layer 5: 悬浮高亮层 (Hover Highlight) - 优先级最高，强制显示
  // ============================================================
  if (hoveredLineId.value) {
    const line = lines.value.find(l => l.id === hoveredLineId.value)
    if (line && line.polygon.length > 0) {
      // 1. 绘制多边形
      ctx.beginPath()
      ctx.moveTo(line.polygon[0][0], line.polygon[0][1])
      for (let i = 1; i < line.polygon.length; i++) {
        ctx.lineTo(line.polygon[i][0], line.polygon[i][1])
      }
      ctx.closePath()

      // 样式：高亮青色 (Cyan)
      ctx.fillStyle = 'rgba(0, 255, 255, 0.2)' // 半透明填充
      ctx.strokeStyle = 'rgba(0, 255, 255, 1)' // 实线描边
      ctx.lineWidth = 3
      ctx.fill()
      ctx.stroke()

      // 2. 绘制基线 (如果存在) - 叠加显示
      if (line.baseline && line.baseline.length > 1) {
        ctx.beginPath()
        ctx.moveTo(line.baseline[0][0], line.baseline[0][1])
        for (let i = 1; i < line.baseline.length; i++) ctx.lineTo(line.baseline[i][0], line.baseline[i][1])
        ctx.strokeStyle = 'rgba(255, 255, 0, 1)' // 黄色基线
        ctx.lineWidth = 2
        ctx.stroke()
      }
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

// 开始补足基线
function startFixMissingBaseline() {
  // 1. 寻找第一个没有基线（或基线点数<2）的 Line
  const target = lines.value.find(l => !l.baseline || l.baseline.length < 2)

  if (!target) {
    alert("所有多边形都已有基线，无需补足！")
    return
  }

  // 2. 强制设置视图状态
  if (showPolygons.value) {
    // 自动关闭多边形显示，以便用户知道进入了基线模式
    showPolygons.value = false
  }
  showBaselines.value = true // 确保基线层开启

  // 3. 进入补足模式
  editMode.value = 'baseline'
  targetMissingBaselineId.value = target.id
  tempPoints.value = [] // 准备存新点
  selectedLineId.value = null // 清除其他选中

  drawCanvas()
}

// 补足模式下的保存
async function saveMissingBaseline() {
  if (tempPoints.value.length < 2) {
    alert("基线至少需要2个点")
    return
  }

  const line = lines.value.find(l => l.id === targetMissingBaselineId.value)
  if (line) {
    // 保存基线数据
    line.baseline = JSON.parse(JSON.stringify(tempPoints.value))

    // 如果基线只有1个点(实际上上面拦截了，但逻辑上要处理)，视同无效
    // 保存历史
    saveHistory()
    await saveToBackend()
  }

  // 退出模式
  quitBaselineMode()
}

// 补足模式下的放弃
function cancelMissingBaseline() {
  quitBaselineMode()
}

function quitBaselineMode() {
  targetMissingBaselineId.value = null
  tempPoints.value = []
  editMode.value = 'none'
  drawCanvas()
}

// --- 核心交互逻辑 ---

// 开始新建模式
function startCreatePolygon() {
  selectedLineId.value = null; // 清除选中
  selectedGlyphId.value = null;
  isCreating.value = true;
  tempPoints.value = [];
  drawCanvas();
}

// 放弃操作
function cancelEdit() {
  if (isCreating.value) {
    isCreating.value = false;
    tempPoints.value = [];
  } else if (selectedLineId.value && editSnapshot.value) {
    // 恢复编辑前的状态
    const originalLine = JSON.parse(editSnapshot.value);
    const targetIndex = lines.value.findIndex(l => l.id === selectedLineId.value);
    if (targetIndex !== -1) {
      lines.value[targetIndex] = originalLine;
    }
    editSnapshot.value = null;
    selectedLineId.value = null; // 退出编辑模式
  } else {
    // 只是选中但没改动，直接退出选中
    selectedLineId.value = null;
  }
  drawCanvas();
}

// 保存操作
async function saveEdit() {
  // ------------------------------------------------
  // 情况 1: 新建模式 (新建的一定是多边形，必须检查)
  // ------------------------------------------------
  if (isCreating.value) {
    if (tempPoints.value.length < 3) {
      alert("多边形至少需要3个点");
      return;
    }
    if (!validatePolygon(tempPoints.value)) {
      alert("多边形存在自交（重叠），无法保存");
      return;
    }

    // 创建新 Line
    const newLine: Line = {
      id: crypto.randomUUID(),
      baseline: [],
      polygon: JSON.parse(JSON.stringify(tempPoints.value)),
      glyphs: []
    };

    lines.value.push(newLine);

    // TODO: 调用后端更新

    isCreating.value = false;
    tempPoints.value = [];
    saveHistory();

  }
  // ------------------------------------------------
  // 情况 2: 编辑已有 Line
  // ------------------------------------------------
  else if (selectedLineId.value) {
    const line = lines.value.find(l => l.id === selectedLineId.value);
    if (!line) return;

    // --- 修复点开始：区分模式进行校验 ---

    // 如果当前是在编辑【多边形】，才检查多边形自交
    if (editMode.value === 'polygon') {
      if (!validatePolygon(line.polygon)) {
        alert("多边形存在自交（重叠），无法保存");
        // 恢复快照？通常用户希望继续改，所以不自动恢复，只是不让保存
        return;
      }
    }

    // 如果当前是在编辑【基线】，通常不需要检查自交（基线允许交叉，或者是开放路径）
    // 所以这里直接跳过 validatePolygon 检查

    // --- 修复点结束 ---

    editSnapshot.value = null; // 清除快照
    selectedLineId.value = null; // 退出选中/编辑状态
    editMode.value = 'none'; // 重置模式
    saveHistory();
    // TODO: 调用后端保存
  }
  await saveToBackend()
  drawCanvas();
}


// 鼠标按下：选择、开始拖拽、或新建点
function handleMouseDown(e: MouseEvent) {
  const { x, y } = getCanvasPos(e);

  // ----------------------------------------------------
  // 1. 补足基线模式 (Fix Missing Baseline)
  // ----------------------------------------------------
  if (targetMissingBaselineId.value) {
    tempPoints.value.push([x, y]);
    drawCanvas();
    return;
  }

  // ----------------------------------------------------
  // 2. 新建多边形模式 (Create Polygon)
  // ----------------------------------------------------
  if (isCreating.value) {
    tempPoints.value.push([x, y]);
    drawCanvas();
    return;
  }

  // ----------------------------------------------------
  // 3. 编辑模式：检查是否点中了控制点 (准备拖拽)
  // ----------------------------------------------------
  if (selectedLineId.value) {
    const line = lines.value.find(l => l.id === selectedLineId.value);

    if (line) {
      // A. 编辑基线模式：检查基线顶点
      if (editMode.value === 'baseline' && line.baseline) {
        for (let i = 0; i < line.baseline.length; i++) {
          const [px, py] = line.baseline[i];
          // 使用 Math.hypot 计算距离
          if (Math.hypot(x - px, y - py) <= STYLE.pointHitRadius) {
            draggingPointIndex.value = i; // 锁定被拖拽点的索引
            // 建立快照以便放弃
            if (!editSnapshot.value) {
              editSnapshot.value = JSON.stringify(line);
            }
            return;
          }
        }
      }

      // B. 编辑多边形模式：检查多边形顶点
      if (editMode.value === 'polygon') {
        for (let i = 0; i < line.polygon.length; i++) {
          const [px, py] = line.polygon[i];
          if (Math.hypot(x - px, y - py) <= STYLE.pointHitRadius) {
            draggingPointIndex.value = i; // 锁定被拖拽点的索引
            // 建立快照以便放弃
            if (!editSnapshot.value) {
              editSnapshot.value = JSON.stringify(line);
            }
            return;
          }
        }
      }
    }
  }

  // ----------------------------------------------------
  // 4. 选择模式：点击选中对象
  // ----------------------------------------------------

  // 优先级 A: 如果显示多边形，优先检测多边形 (多边形层在上方，遮挡基线)
  if (showPolygons.value) {
    let hitLineId: string | null = null;

    // 反向遍历，优先选中视觉上最上层的
    for (let i = lines.value.length - 1; i >= 0; i--) {
      if (isPointInPolygon(x, y, lines.value[i].polygon)) {
        hitLineId = lines.value[i].id;
        break;
      }
    }

    if (hitLineId) {
      // 如果点击了新的多边形，切换选中状态
      if (selectedLineId.value !== hitLineId) {
        selectedLineId.value = hitLineId;
        selectedGlyphId.value = null;
        editMode.value = 'polygon'; // 自动进入多边形编辑模式

        // 自动建立快照，防止直接开始操作没有退路
        const line = lines.value.find(l => l.id === hitLineId);
        if (line) editSnapshot.value = JSON.stringify(line);
      }
    }
    // 注意：如果 showPolygons 为 true 但没点中多边形，
    // 我们直接结束，不透过去检测基线 (因为物理遮挡)
    drawCanvas();
    return;
  }

  // 优先级 B: 如果多边形关闭 且 显示基线，则检测基线
  if (showBaselines.value) {
    let hitLineId: string | null = null;
    let minDist = Infinity;

    for (const line of lines.value) {
      if (!line.baseline || line.baseline.length < 2) continue;

      // 检测鼠标是否在基线折线段附近
      for (let i = 0; i < line.baseline.length - 1; i++) {
        const p1 = line.baseline[i];
        const p2 = line.baseline[i + 1];
        // 计算点到线段距离
        const d = getDistanceToSegment(x, y, p1[0], p1[1], p2[0], p2[1]);

        // 找到最近的一条基线
        if (d <= STYLE.pointHitRadius && d < minDist) {
          minDist = d;
          hitLineId = line.id;
        }
      }
    }

    if (hitLineId) {
      if (selectedLineId.value !== hitLineId) {
        selectedLineId.value = hitLineId;
        selectedGlyphId.value = null;
        editMode.value = 'baseline'; // 自动进入基线编辑模式

        const line = lines.value.find(l => l.id === hitLineId);
        if (line) editSnapshot.value = JSON.stringify(line);
      }
    }

    drawCanvas();
    return;
  }

  drawCanvas();
}

// MouseMove: 通用拖拽
function handleMouseMove(e: MouseEvent) {
  if (draggingPointIndex.value === null || !selectedLineId.value) return
  const { x, y } = getCanvasPos(e)
  const line = lines.value.find(l => l.id === selectedLineId.value)
  if (!line) return

  if (editMode.value === 'polygon') {
    line.polygon[draggingPointIndex.value] = [x, y]
  } else if (editMode.value === 'baseline') {
    line.baseline[draggingPointIndex.value] = [x, y]
  }
  drawCanvas()
}

// 鼠标松开：结束拖拽
function handleMouseUp() {
  draggingPointIndex.value = null;
}

// 双击：在线段上添加点
function handleDblClick(e: MouseEvent) {
  if (isCreating.value || !selectedLineId.value) return;

  const { x, y } = getCanvasPos(e);
  const line = lines.value.find(l => l.id === selectedLineId.value);
  if (!line) return;


  if (editMode.value === 'baseline') {
    // 基线加点逻辑
    for (let i = 0; i < line.baseline.length - 1; i++) {
      const p1 = line.baseline[i]; const p2 = line.baseline[i + 1];
      if (getDistanceToSegment(x, y, p1[0], p1[1], p2[0], p2[1]) <= STYLE.pointHitRadius) {
        if (!editSnapshot.value) editSnapshot.value = JSON.stringify(line);
        line.baseline.splice(i + 1, 0, [x, y]); // 插入点
        drawCanvas();
        return;
      }
    }
  } else if (editMode.value === 'polygon') {
    // 遍历所有边，看点击位置是否在边附近
    for (let i = 0; i < line.polygon.length; i++) {
      const p1 = line.polygon[i];
      const p2 = line.polygon[(i + 1) % line.polygon.length];

      const dist = getDistanceToSegment(x, y, p1[0], p1[1], p2[0], p2[1]);

      if (dist <= STYLE.pointHitRadius) {
        // 插入新点
        // 为了防止插入快照丢失，确保存快照
        if (!editSnapshot.value) editSnapshot.value = JSON.stringify(line);

        // 在 i 和 i+1 之间插入
        line.polygon.splice(i + 1, 0, [x, y]);
        drawCanvas();
        return;
      }
    }
  }
}


// 简化多边形：移除距离过近的冗余点
// threshold: 阈值，单位像素。默认 5 像素内的点会被合并
function simplifyPoints(points: number[][], threshold: number = 5): number[][] {
  if (points.length <= 3) return points; // 点太少不处理

  const newPoints: number[][] = [points[0]]; // 保留第一个点
  let lastPoint = points[0];

  for (let i = 1; i < points.length; i++) {
    const current = points[i];
    // 计算欧几里得距离
    const dist = Math.hypot(current[0] - lastPoint[0], current[1] - lastPoint[1]);

    // 只有距离大于阈值才保留，或者如果是最后一个点，强制保留（防止形状未闭合）
    // 但为了闭合逻辑，通常只看距离即可。为了保持轮廓，我们优先保留"远"点
    if (dist >= threshold) {
      newPoints.push(current);
      lastPoint = current;
    }
  }

  // 兜底保护：如果简化后点数少于3个，说明是个微小的噪点团，强行保留原始点或至少保留首尾
  if (newPoints.length < 3) {
    // 如果原点数够多但全被删了，至少返回原始形状的一个三角形近似，或者直接不简化返回原值
    return points.length >= 3 ? [points[0], points[Math.floor(points.length / 2)], points[points.length - 1]] : points;
  }

  return newPoints;
}

function handleClearNearbyPoints() {
  const THRESHOLD = 8; // 8像素内的点会被清除，你可以根据实际手感调整这个值

  if (isCreating.value) {
    // 1. 处理新建中的点
    if (tempPoints.value.length <= 3) return;
    const simplified = simplifyPoints(tempPoints.value, THRESHOLD);
    // 更新
    tempPoints.value = simplified;

  } else if (selectedLineId.value) {
    // 2. 处理选中的已有 Line
    const line = lines.value.find(l => l.id === selectedLineId.value);
    if (!line) return;

    if (line.polygon.length <= 3) return;

    // 重要：先存快照！以便用户点“放弃”时能恢复一堆乱点的状态
    if (!editSnapshot.value) {
      editSnapshot.value = JSON.stringify(line);
    }

    const simplified = simplifyPoints(line.polygon, THRESHOLD);

    // 如果简化没产生变化，给个提示可选
    if (simplified.length === line.polygon.length) {
      // console.log("没有发现冗余点");
    } else {
      line.polygon = simplified;
    }
  }

  drawCanvas();
}
// 右键：删除点
// 右键：删除点
function handleContextMenu(e: MouseEvent) {
  e.preventDefault(); // 阻止浏览器默认菜单

  // 必须处于：补足基线模式 OR 新建模式 OR 选中编辑模式 之一
  if (!isCreating.value && !selectedLineId.value && !targetMissingBaselineId.value) return;

  const { x, y } = getCanvasPos(e);

  // --------------------------------------------------------
  // 1. 处理【临时点】删除 (新建多边形 OR 补足基线)
  // --------------------------------------------------------
  if (isCreating.value || targetMissingBaselineId.value) {
    // 检查点击了哪个临时点
    for (let i = 0; i < tempPoints.value.length; i++) {
      const [px, py] = tempPoints.value[i];
      // 使用 Math.hypot 计算距离
      if (Math.hypot(x - px, y - py) <= STYLE.pointHitRadius) {
        tempPoints.value.splice(i, 1);
        drawCanvas();
        return;
      }
    }
    return;
  }

  // --------------------------------------------------------
  // 2. 处理【已有 Line】删除 (编辑模式)
  // --------------------------------------------------------
  const line = lines.value.find(l => l.id === selectedLineId.value);
  if (!line) return;

  // A. 编辑基线模式 (Edit Baseline)
  if (editMode.value === 'baseline') {
    if (!line.baseline) return;

    for (let i = 0; i < line.baseline.length; i++) {
      const [px, py] = line.baseline[i];

      if (Math.hypot(x - px, y - py) <= STYLE.pointHitRadius) {
        // 操作前存快照
        if (!editSnapshot.value) editSnapshot.value = JSON.stringify(line);

        // 删除该点
        line.baseline.splice(i, 1);

        // 校验：如果基线点数小于 2，视为删除整条基线
        if (line.baseline.length < 2) {
          line.baseline = []; // 清空数组
          // 这里不直接退出编辑模式，而是让用户看到基线消失了
          // 用户随后点击“保存”时，空的 baseline 应该被处理为删除 ALTO 标签
        }

        drawCanvas();
        return;
      }
    }
  }
  // B. 编辑多边形模式 (Edit Polygon)
  else if (editMode.value === 'polygon') {
    for (let i = 0; i < line.polygon.length; i++) {
      const [px, py] = line.polygon[i];

      if (Math.hypot(x - px, y - py) <= STYLE.pointHitRadius) {
        // 校验：多边形至少需要3个点
        if (line.polygon.length <= 3) {
          alert("多边形至少需要保留3个点");
          return;
        }

        // 操作前存快照
        if (!editSnapshot.value) editSnapshot.value = JSON.stringify(line);

        // 删除该点
        line.polygon.splice(i, 1);
        drawCanvas();
        return;
      }
    }
  }
}

// 辅助：获取 Canvas 坐标
function getCanvasPos(e: MouseEvent) {
  const canvas = canvasRef.value!;
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: (e.clientY - rect.top) * scaleY
  };
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

async function deleteLine() {
  if (!selectedLineId.value) return

  // 弹窗确认（可选）
  if (!confirm("确定要删除选中的行吗？")) return

  saveHistory()
  lines.value = lines.value.filter(l => l.id !== selectedLineId.value)

  // 清理选中状态
  selectedLineId.value = null
  selectedGlyphId.value = null

  // 保存到后端
  await saveToBackend() // <--- 新增这行

  drawCanvas()
}

function goBack() {
  router.push(`/project/${projectId}`)
}

// 将坐标数组转换为 ALTO 格式的字符串 "x1 y1 x2 y2 ..."
function pointsToString(points: number[][]): string {
  return points.map(p => `${Math.round(p[0])} ${Math.round(p[1])}`).join(' ')
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

async function saveToBackend() {
  if (!altoXml.value) return

  const parser = new DOMParser()
  const doc = parser.parseFromString(altoXml.value, "text/xml")
  
  const ns = doc.documentElement.getAttribute('xmlns')

  const findLineNode = (id: string) => {
    let node = doc.getElementById(id)
    if (!node) {
      const allLines = doc.getElementsByTagName('TextLine')
      for (let i = 0; i < allLines.length; i++) {
        if (allLines[i].getAttribute('ID') === id) return allLines[i]
      }
    }
    return node
  }

  let textBlock = doc.getElementsByTagName('TextBlock')[0]
  if (!textBlock) {
    console.error("XML 中未找到 TextBlock，无法保存")
    return
  }

  // 遍历前端的数据，更新 XML
  lines.value.forEach(lineData => {
    let lineNode = findLineNode(lineData.id)

    if (lineNode) {
      // --- 更新已有 Line ---
      
      // 更新 Baseline
      if (lineData.baseline && lineData.baseline.length > 0) {
        lineNode.setAttribute('BASELINE', pointsToString(lineData.baseline))
      } else {
        lineNode.removeAttribute('BASELINE')
      }
      
      // 更新 Polygon
      let shapeNode = lineNode.querySelector(':scope > Shape')
      if (!shapeNode) {
        shapeNode = doc.createElementNS(ns, 'Shape')
        lineNode.insertBefore(shapeNode, lineNode.firstChild)
      }
      
      let polyNode = shapeNode.querySelector(':scope > Polygon')
      if (!polyNode) {
        polyNode = doc.createElementNS(ns, 'Polygon')
        shapeNode.appendChild(polyNode)
      }
      
      polyNode.setAttribute('POINTS', pointsToString(lineData.polygon))
      
      // --- 重建 String 和 Glyph 结构 ---
      // 删除所有现有的 String 节点
      const existingStrings = Array.from(lineNode.getElementsByTagName('String'))
      existingStrings.forEach(s => s.parentNode?.removeChild(s))
      
      // 删除所有现有的 SP 节点（空格）
      const existingSPs = Array.from(lineNode.getElementsByTagName('SP'))
      existingSPs.forEach(s => s.parentNode?.removeChild(s))
      
      // 根据前端数据重建 String 和 Glyph
      // 简化处理：将所有 Glyph 放在一个 String 中
      if (lineData.glyphs.length > 0) {
        const newString = doc.createElementNS(ns, 'String')
        newString.setAttribute('ID', `string_${lineData.id}`)
        
        // 计算 String 的边界
        let minHpos = Infinity, minVpos = Infinity
        let maxHpos = -Infinity, maxVpos = -Infinity
        
        lineData.glyphs.forEach(g => {
          minHpos = Math.min(minHpos, g.hpos)
          minVpos = Math.min(minVpos, g.vpos)
          maxHpos = Math.max(maxHpos, g.hpos + (g.width || 20))
          maxVpos = Math.max(maxVpos, g.vpos + g.height)
        })
        
        newString.setAttribute('HPOS', String(Math.round(minHpos)))
        newString.setAttribute('VPOS', String(Math.round(minVpos)))
        newString.setAttribute('WIDTH', String(Math.round(maxHpos - minHpos)))
        newString.setAttribute('HEIGHT', String(Math.round(maxVpos - minVpos)))
        
        // 拼接内容
        const content = lineData.glyphs.map(g => g.content).join('')
        newString.setAttribute('CONTENT', content)
        newString.setAttribute('WC', '1.0')
        
        // 创建 Glyph 节点
        lineData.glyphs.forEach((glyphData, idx) => {
          const newGlyph = doc.createElementNS(ns, 'Glyph')
          newGlyph.setAttribute('ID', glyphData.id || `glyph_${lineData.id}_${idx}`)
          newGlyph.setAttribute('CONTENT', glyphData.content)
          newGlyph.setAttribute('HPOS', String(Math.round(glyphData.hpos)))
          newGlyph.setAttribute('VPOS', String(Math.round(glyphData.vpos)))
          newGlyph.setAttribute('WIDTH', String(Math.round(glyphData.width || 0)))
          newGlyph.setAttribute('HEIGHT', String(Math.round(glyphData.height)))
          newGlyph.setAttribute('GC', '1.0')
          
          // 创建 Glyph 的 Shape
          const glyphShape = doc.createElementNS(ns, 'Shape')
          const glyphPoly = doc.createElementNS(ns, 'Polygon')
          // 简单矩形
          const x = glyphData.hpos
          const y = glyphData.vpos
          const w = glyphData.width || 0
          const h = glyphData.height
          glyphPoly.setAttribute('POINTS', `${x} ${y} ${x} ${y + h} ${x + w} ${y + h} ${x + w} ${y}`)
          glyphShape.appendChild(glyphPoly)
          newGlyph.appendChild(glyphShape)
          
          newString.appendChild(newGlyph)
        })
        
        lineNode.appendChild(newString)
      }
      
    } else {
      // --- 新增 Line ---
      const newLine = doc.createElementNS(ns, 'TextLine')
      newLine.setAttribute('ID', lineData.id)
      
      if (lineData.baseline.length > 0) {
        newLine.setAttribute('BASELINE', pointsToString(lineData.baseline))
      }
      
      const newShape = doc.createElementNS(ns, 'Shape')
      const newPoly = doc.createElementNS(ns, 'Polygon')
      newPoly.setAttribute('POINTS', pointsToString(lineData.polygon))
      newShape.appendChild(newPoly)
      newLine.appendChild(newShape)
      
      // 添加 Glyphs
      if (lineData.glyphs.length > 0) {
        const newString = doc.createElementNS(ns, 'String')
        newString.setAttribute('ID', `string_${lineData.id}`)
        
        const content = lineData.glyphs.map(g => g.content).join('')
        newString.setAttribute('CONTENT', content)
        
        lineData.glyphs.forEach((glyphData, idx) => {
          const newGlyph = doc.createElementNS(ns, 'Glyph')
          newGlyph.setAttribute('ID', glyphData.id || `glyph_${lineData.id}_${idx}`)
          newGlyph.setAttribute('CONTENT', glyphData.content)
          newGlyph.setAttribute('HPOS', String(Math.round(glyphData.hpos)))
          newGlyph.setAttribute('VPOS', String(Math.round(glyphData.vpos)))
          newGlyph.setAttribute('WIDTH', String(Math.round(glyphData.width || 0)))
          newGlyph.setAttribute('HEIGHT', String(Math.round(glyphData.height)))
          newGlyph.setAttribute('GC', '1.0')
          newString.appendChild(newGlyph)
        })
        
        newLine.appendChild(newString)
      }
      
      textBlock.appendChild(newLine)
    }
  })

  // 处理删除
  const currentIds = new Set(lines.value.map(l => l.id))
  const xmlLines = Array.from(doc.getElementsByTagName('TextLine'))
  
  xmlLines.forEach(node => {
    const id = node.getAttribute('ID')
    if (id && !currentIds.has(id)) {
      node.parentNode?.removeChild(node)
    }
  })

  // 序列化回字符串
  const serializer = new XMLSerializer()
  const newXmlStr = serializer.serializeToString(doc)

  altoXml.value = newXmlStr
  
  try {
    await invoke('save_alto', { pageId, xml: newXmlStr })
    console.log("保存成功！")
  } catch (e) {
    console.error("保存失败:", e)
    alert("保存到数据库失败: " + e)
  }
}

// 按句子分组的 TextLine
const groupedLines = computed(() => {
  if (!sentenceData.value) return { sentences: [], unassigned: lines.value }

  const assignedLineIds = new Set<string>()

  const sentences = sentenceData.value.sentences.map(sentence => {
    const sentenceLines: Line[] = []
    for (const lineId of sentence.line_ids) {
      const line = lines.value.find(l => l.id === lineId)
      if (line) {
        sentenceLines.push(line)
        assignedLineIds.add(lineId)
      }
    }
    return {
      ...sentence,
      lines: sentenceLines
    }
  })

  // 找出未分配的 TextLine
  const unassigned = lines.value.filter(l => !assignedLineIds.has(l.id))

  return { sentences, unassigned }
})

// 获取句子类型的显示标签
function getSentenceTypeLabel(type: 'text' | 'title' | 'note'): string {
  switch (type) {
    case 'title': return '标题'
    case 'note': return '注释'
    default: return '正文'
  }
}

// 获取句子类型的颜色
function getSentenceTypeColor(type: 'text' | 'title' | 'note'): string {
  switch (type) {
    case 'title': return 'border-yellow-500/50'
    case 'note': return 'border-green-500/50'
    default: return 'border-blue-500/50'
  }
}

function openSentenceEditor(sentence: Sentence) {
  editingSentence.value = sentence
  showSentenceEditor.value = true
}

async function handleSaveSentence(data: { normalized: string; en: string; zh: string; type: 'text' | 'title' | 'note' }) {
  if (!editingSentence.value || !sentenceData.value) return

  // 找到并更新句子
  const index = sentenceData.value.sentences.findIndex(s => s.id === editingSentence.value!.id)
  if (index !== -1) {
    sentenceData.value.sentences[index] = {
      ...sentenceData.value.sentences[index],
      normalized: data.normalized,
      en: data.en,
      zh: data.zh,
      type: data.type
    }

    // 保存到数据库
    await invoke('save_sentence', {
      pageId,
      data: JSON.stringify(sentenceData.value)
    })
  }

  showSentenceEditor.value = false
  editingSentence.value = null
}

function closeSentenceEditor() {
  showSentenceEditor.value = false
  editingSentence.value = null
}

function openTextLineEditor(line: Line) {
  editingTextLine.value = line
  showTextLineEditor.value = true
}

async function handleSaveTextLine(updatedLine: Line) {
  // 更新 lines 数组中对应的行
  const index = lines.value.findIndex(l => l.id === updatedLine.id)
  if (index !== -1) {
    lines.value[index] = updatedLine
    saveHistory()
    await saveToBackend()
  }

  showTextLineEditor.value = false
  editingTextLine.value = null
  drawCanvas()
}

function closeTextLineEditor() {
  showTextLineEditor.value = false
  editingTextLine.value = null
}

// 根据 line id 找到所属的 sentence
function findSentenceByLineId(lineId: string): Sentence | null {
  if (!sentenceData.value) return null
  return sentenceData.value.sentences.find(s => s.line_ids.includes(lineId)) || null
}

// 获取全局 TextLine 顺序
function getGlobalLineOrder(): string[] {
  if (!sentenceData.value) return []
  const order: string[] = []
  for (const sentence of sentenceData.value.sentences) {
    for (const lineId of sentence.line_ids) {
      order.push(lineId)
    }
  }
  return order
}

async function handleSplitAfter(lineId: string, glyphIndex: number) {
  if (!sentenceData.value) return

  const globalOrder = getGlobalLineOrder()
  const globalIndex = globalOrder.indexOf(lineId)

  if (globalIndex === -1 || globalIndex === globalOrder.length - 1) return

  const currentLine = lines.value.find(l => l.id === lineId)
  if (!currentLine) return

  // 获取下一个 TextLine 的 ID
  const nextLineId = globalOrder[globalIndex + 1]
  const nextLine = lines.value.find(l => l.id === nextLineId)
  if (!nextLine) return

  // 获取要移动的 glyphs（glyphIndex 之后的所有字符）
  const glyphsToMove = currentLine.glyphs.slice(glyphIndex + 1)

  if (glyphsToMove.length === 0) return

  // 更新 lineId
  glyphsToMove.forEach(g => {
    g.lineId = nextLineId
  })

  // 从当前行移除
  currentLine.glyphs = currentLine.glyphs.slice(0, glyphIndex + 1)

  // 添加到下一行的开头
  nextLine.glyphs = [...glyphsToMove, ...nextLine.glyphs]

  saveHistory()
  await saveToBackend()

  // 关闭编辑器
  showTextLineEditor.value = false
  editingTextLine.value = null

  drawCanvas()
}

async function handleSplitBefore(lineId: string, glyphIndex: number) {
  if (!sentenceData.value) return

  const globalOrder = getGlobalLineOrder()
  const globalIndex = globalOrder.indexOf(lineId)

  if (globalIndex === -1 || globalIndex === 0) return

  const currentLine = lines.value.find(l => l.id === lineId)
  if (!currentLine) return

  // 获取上一个 TextLine 的 ID
  const prevLineId = globalOrder[globalIndex - 1]
  const prevLine = lines.value.find(l => l.id === prevLineId)
  if (!prevLine) return

  // 获取要移动的 glyphs（glyphIndex 之前的所有字符，不包括 glyphIndex）
  const glyphsToMove = currentLine.glyphs.slice(0, glyphIndex)

  if (glyphsToMove.length === 0) return

  // 更新 lineId
  glyphsToMove.forEach(g => {
    g.lineId = prevLineId
  })

  // 从当前行移除
  currentLine.glyphs = currentLine.glyphs.slice(glyphIndex)

  // 添加到上一行的末尾
  prevLine.glyphs = [...prevLine.glyphs, ...glyphsToMove]

  saveHistory()
  await saveToBackend()

  // 关闭编辑器
  showTextLineEditor.value = false
  editingTextLine.value = null

  drawCanvas()
}

async function handleUpdateSentenceFromEditor(data: { normalized: string; en: string; zh: string; type: 'text' | 'title' | 'note' }) {
  if (!editingTextLine.value || !sentenceData.value) return

  const sentence = findSentenceByLineId(editingTextLine.value.id)
  if (!sentence) return

  const index = sentenceData.value.sentences.findIndex(s => s.id === sentence.id)
  if (index !== -1) {
    sentenceData.value.sentences[index] = {
      ...sentenceData.value.sentences[index],
      ...data
    }

    await invoke('save_sentence', {
      pageId,
      data: JSON.stringify(sentenceData.value)
    })
  }
}

// 处理拓扑图的更新（拖拽排序、移动、新建等）
async function handleTopologyUpdate(newSentences: Sentence[]) {
  if (!sentenceData.value) return

  // 更新本地数据
  sentenceData.value.sentences = newSentences

  // 保存到数据库
  await invoke('save_sentence', {
    pageId,
    data: JSON.stringify(sentenceData.value)
  })

  // 强制重绘 Canvas (因为选中状态可能变了)
  drawCanvas()
}

function handleLineHover(id: string) {
  hoveredLineId.value = id
  drawCanvas()
}

function handleLineLeave() {
  hoveredLineId.value = null
  drawCanvas()
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
      <button class="px-3 py-1.5 rounded text-base font-medium bg-zinc-700 text-zinc-300 hover:bg-zinc-600"
        @click="goBack">
        ← 返回
      </button>

      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button class="px-3 py-1.5 rounded text-base font-medium bg-blue-600 text-white hover:bg-blue-500"
        @click="openMetaModal">
        元信息
      </button>

      <button class="px-3 py-1.5 rounded text-base font-medium bg-green-600 text-white hover:bg-green-500"
        @click="importAlto">
        导入 ALTO
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        isCreating
          ? 'bg-yellow-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="startCreatePolygon" :disabled="isBusy">
        + 新建 Line
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        targetMissingBaselineId
          ? 'bg-yellow-600 text-white'
          : 'bg-indigo-700 text-zinc-300 hover:bg-indigo-600'
      ]" @click="startFixMissingBaseline" :disabled="isBusy">
        Fix Baselines
      </button>
      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        showBaselines
          ? 'bg-blue-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showBaselines = !showBaselines">
        Baselines
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        showPolygons
          ? 'bg-fuchsia-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showPolygons = !showPolygons">
        Polygons
      </button>

      <div class="w-px h-6 bg-zinc-700 mx-2"></div>

      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        historyIndex > 0
          ? 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
          : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
      ]" :disabled="historyIndex <= 0" @click="undo">
        Undo
      </button>
      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        historyIndex < history.length - 1
          ? 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
          : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
      ]" :disabled="historyIndex >= history.length - 1" @click="redo">
        Redo
      </button>

      <div v-if="warning" class="ml-4 px-3 py-1 bg-yellow-600/20 text-yellow-400 rounded text-base">
        ⚠️ {{ warning }}
      </div>
      <button :class="[
        'px-3 py-1.5 rounded text-base font-medium transition-colors',
        showTopology
          ? 'bg-purple-600 text-white'
          : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'
      ]" @click="showTopology = !showTopology">
        拓扑关系
      </button>
      <div class="flex items-center gap-3 text-base">
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="showNormalized" class="w-3.5 h-3.5 accent-blue-500" />
          <span class="text-zinc-400">正则</span>
        </label>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="showEnglish" class="w-3.5 h-3.5 accent-blue-500" />
          <span class="text-zinc-400">英文</span>
        </label>
        <label class="flex items-center gap-1.5 cursor-pointer">
          <input type="checkbox" v-model="showChinese" class="w-3.5 h-3.5 accent-blue-500" />
          <span class="text-zinc-400">中文</span>
        </label>
      </div>
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
          class="absolute top-2 left-2 px-3 py-1.5 bg-red-600 hover:bg-red-500 text-white text-base font-medium rounded shadow-lg z-10"
          @click="deleteLine">
          Delete
        </button>
        <div v-if="isCreating || selectedLineId || targetMissingBaselineId"
          class="absolute top-12 left-2 flex flex-col gap-2 z-10">

          <div v-if="targetMissingBaselineId"
            class="mb-2 text-sm text-yellow-400 bg-black/80 p-2 rounded shadow border border-yellow-600/50">
            <strong>模式: 补足基线</strong><br>
            请点击至少2个点绘制基线
          </div>
          <div v-else-if="editMode === 'baseline'"
            class="mb-2 text-sm text-orange-400 bg-black/80 p-2 rounded shadow border border-orange-600/50">
            <strong>模式: 编辑基线</strong><br>
            拖动调整，双击加点，右键删点
          </div>
          <div v-else-if="isCreating"
            class="mb-2 text-sm text-green-400 bg-black/80 p-2 rounded shadow border border-green-600/50">
            <strong>模式: 新建多边形</strong><br>
            点击绘制，右键撤销点
          </div>

          <button v-if="editMode === 'polygon' || isCreating"
            class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-base font-medium rounded shadow-lg flex items-center justify-center gap-1"
            @click="handleClearNearbyPoints" title="自动删除堆叠在一起的冗余顶点">
            <span>🧹</span> 清除冗余点
          </button>

          <button class="px-3 py-1.5 bg-green-600 hover:bg-green-500 text-white text-base font-medium rounded shadow-lg"
            @click="targetMissingBaselineId ? saveMissingBaseline() : saveEdit()">
            保存 (Save)
          </button>

          <button class="px-3 py-1.5 bg-zinc-600 hover:bg-zinc-500 text-white text-base font-medium rounded shadow-lg"
            @click="targetMissingBaselineId ? cancelMissingBaseline() : cancelEdit()">
            放弃 (Cancel)
          </button>
        </div>
        <div v-if="imageUrl" class="relative inline-block origin-top-left" :style="{ transform: `scale(${scale})` }">
          <img ref="imageRef" :src="imageUrl" class="max-w-none" @load="initCanvas" />
          <canvas ref="canvasRef" class="absolute top-0 left-0 cursor-crosshair" @mousedown="handleMouseDown"
            @mousemove="handleMouseMove" @mouseup="handleMouseUp" @dblclick="handleDblClick"
            @contextmenu="handleContextMenu" />
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
        <div v-if="lines.length > 0" class="space-y-4">
          <!-- 按句子分组显示 -->
          <div v-for="(sentence, sIndex) in groupedLines.sentences" :key="sentence.id" :class="[
            'rounded-lg border-l-4 bg-zinc-900/50',
            getSentenceTypeColor(sentence.type)
          ]">
            <!-- 句子头部 -->
            <div class="px-3 py-2 border-b border-zinc-800 flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="text-zinc-500 text-sm font-mono">{{ sIndex + 1 }}</span>
                <button class="text-sm px-1.5 py-0.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-300"
                  @click="openSentenceEditor(sentence)">
                  编辑
                </button>
                <span class="text-zinc-400 text-sm px-1.5 py-0.5 rounded bg-zinc-800">
                  {{ getSentenceTypeLabel(sentence.type) }}
                </span>
              </div>
              <span class="text-zinc-600 text-sm">
                {{ sentence.lines.length }} 行
              </span>
            </div>

            <!-- 句子内的 TextLine 列表 -->
            <div class="p-2 space-y-1">
              <div v-for="(line, lIndex) in sentence.lines" :key="line.id"
                class="p-2 bg-zinc-900 rounded flex items-start gap-2" :class="[
                  'p-2 rounded flex items-start gap-2 transition-colors duration-200',
                  hoveredLineId === line.id ? 'bg-zinc-800 ring-1 ring-cyan-500' : 'bg-zinc-900'
                ]">
                <span class="text-zinc-600 text-sm font-mono shrink-0 mt-0.5">
                  {{ lIndex + 1 }}
                </span>
                <div class="flex-1">
                  <span v-for="glyph in line.glyphs" :key="glyph.id" :class="[
                    'cursor-pointer hover:bg-blue-500/30 rounded px-px [font-variant-alternates:historical-forms]',
                    selectedGlyphId === glyph.id ? 'bg-blue-500/50 text-white' : 'text-zinc-300'
                  ]" @click="handleGlyphClick(glyph.id)">{{ glyph.content }}</span>
                  <span v-if="line.glyphs.length === 0" class="text-zinc-600 italic text-base">
                    (空行)
                  </span>
                </div>
                <button class="shrink-0 text-sm px-1.5 py-0.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-400"
                  @click="openTextLineEditor(line)">
                  编辑
                </button>
              </div>

              <!-- 空句子提示 -->
              <div v-if="sentence.lines.length === 0" class="text-zinc-600 text-base italic p-2">
                该句子没有关联的 TextLine
              </div>

              <!-- 翻译文本显示 -->
              <div v-if="showNormalized || showEnglish || showChinese"
                class="mt-2 pt-2 border-t border-zinc-800 space-y-1">
                <div v-if="showNormalized && sentence.normalized" class="text-base">
                  <span class="text-zinc-500 text-sm mr-2">正则:</span>
                  <span class="text-zinc-300">{{ sentence.normalized }}</span>
                </div>
                <div v-if="showEnglish && sentence.en" class="text-base">
                  <span class="text-zinc-500 text-sm mr-2">EN:</span>
                  <span class="text-zinc-300">{{ sentence.en }}</span>
                </div>
                <div v-if="showChinese && sentence.zh" class="text-base">
                  <span class="text-zinc-500 text-sm mr-2">中:</span>
                  <span class="text-zinc-300">{{ sentence.zh }}</span>
                </div>
                <div
                  v-if="(showNormalized || showEnglish || showChinese) && !sentence.normalized && !sentence.en && !sentence.zh"
                  class="text-zinc-600 text-sm italic">
                  暂无翻译
                </div>
              </div>
            </div>
          </div>

          <!-- 未分配的 TextLine -->
          <div v-if="groupedLines.unassigned.length > 0" class="rounded-lg border-l-4 border-red-500/50 bg-zinc-900/50">
            <div class="px-3 py-2 border-b border-zinc-800 flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="text-red-400 text-sm">⚠️ 待分配 TextLine</span>
              </div>
              <span class="text-zinc-600 text-sm">
                {{ groupedLines.unassigned.length }} 行
              </span>
            </div>

            <div class="p-2 space-y-1">
              <div v-for="line in groupedLines.unassigned" :key="line.id"
                class="p-2 bg-zinc-900 rounded flex items-start gap-2" :class="[
                  'p-2 rounded flex items-start gap-2 transition-colors duration-200',
                  hoveredLineId === line.id ? 'bg-zinc-800 ring-1 ring-cyan-500' : 'bg-zinc-900'
                ]">
                <div class="flex-1">
                  <span v-for="glyph in line.glyphs" :key="glyph.id" :class="[
                    'cursor-pointer hover:bg-blue-500/30 rounded px-px',
                    selectedGlyphId === glyph.id ? 'bg-blue-500/50 text-white' : 'text-zinc-300'
                  ]" @click="handleGlyphClick(glyph.id)">{{ glyph.content }}</span>
                  <span v-if="line.glyphs.length === 0" class="text-zinc-600 italic text-base">
                    (空行)
                  </span>
                </div>
                <button class="shrink-0 text-sm px-1.5 py-0.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-400"
                  @click="openTextLineEditor(line)">
                  编辑
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="text-zinc-500">
          暂无数据，请导入 ALTO 文件
        </div>
      </div>

      <!-- 拓扑关系面板 -->
      <div v-if="showTopology" class="w-[27.2%] border-l border-zinc-800 transition-all duration-300">
        <TopologyPanel :sentences="sentenceData?.sentences || []" :lines="lines" @hover-line="handleLineHover"
          @leave-line="handleLineLeave" @update="handleTopologyUpdate" />
      </div>
    </div>

    <!-- 元信息弹窗 -->
    <div v-if="showMetaModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      @click.self="showMetaModal = false">
      <div class="bg-zinc-900 rounded-lg p-6 w-125 max-h-[80vh] overflow-y-auto">
        <h2 class="text-xl font-bold mb-4">编辑元信息</h2>

        <div class="space-y-4">
          <div>
            <label class="block text-base text-zinc-400 mb-1">图片远程链接</label>
            <input v-model="form.image_remote_url" type="text"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500"
              placeholder="https://..." />
          </div>

          <div>
            <label class="block text-base text-zinc-400 mb-1">图片本地路径</label>
            <input v-model="form.image_local_path" type="text"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500"
              placeholder="C:\..." />
          </div>
          <div>
            <label class="block text-base text-zinc-400 mb-1">英文描述</label>
            <textarea v-model="form.description_en"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
              rows="3" placeholder="English description..." />
          </div>

          <div>
            <label class="block text-base text-zinc-400 mb-1">其他语言描述</label>
            <textarea v-model="form.description_other"
              class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-500 resize-none"
              rows="3" placeholder="其他语言描述..." />
          </div>
        </div>

        <div class="flex justify-end gap-2 mt-6">
          <button class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-base" @click="showMetaModal = false">
            取消
          </button>
          <button class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-base" @click="saveMeta">
            保存
          </button>
        </div>
      </div>
    </div>

    <!-- 编辑句子弹窗 -->
    <SentenceEditor :sentence="editingSentence" :visible="showSentenceEditor" @close="closeSentenceEditor"
      @save="handleSaveSentence" />
    <!-- 编辑 TextLine 弹窗 -->
    <!-- 编辑 TextLine 弹窗 -->
    <!-- 编辑 TextLine 弹窗 -->
    <TextLineEditor :line="editingTextLine" :image-url="imageUrl" :visible="showTextLineEditor" :all-lines="lines"
      :sentences="sentenceData?.sentences || []"
      :sentence="editingTextLine ? findSentenceByLineId(editingTextLine.id) : null" @close="closeTextLineEditor"
      @save="handleSaveTextLine" @split-after="handleSplitAfter" @split-before="handleSplitBefore"
      @update-sentence="handleUpdateSentenceFromEditor" />
  </div>
</template>