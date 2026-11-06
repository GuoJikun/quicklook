<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'
import {
    ArrowClockwise16Regular,
    ArrowSync16Regular,
    ChevronDown16Regular,
    Cube16Regular,
    PaintBucket16Regular,
} from '@vicons/fluent'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js'
import { OBJLoader } from 'three/examples/jsm/loaders/OBJLoader.js'
import { PLYLoader } from 'three/examples/jsm/loaders/PLYLoader.js'
import { FBXLoader } from 'three/examples/jsm/loaders/FBXLoader.js'
import { ThreeMFLoader } from 'three/examples/jsm/loaders/3MFLoader.js'
import { ColladaLoader } from 'three/examples/jsm/loaders/ColladaLoader.js'
import { TDSLoader } from 'three/examples/jsm/loaders/TDSLoader.js'
import { AMFLoader } from 'three/examples/jsm/loaders/AMFLoader.js'
import { VRMLLoader } from 'three/examples/jsm/loaders/VRMLLoader.js'
import { LWOLoader } from 'three/examples/jsm/loaders/LWOLoader.js'
import LayoutPreview from '@/components/layout-preview.vue'
import type { FileInfo } from '@/utils/typescript'

defineOptions({ name: 'ModelSupport' })

/** 背景三态：网格（默认，带地面参考）/ 纯深 / 纯浅，循环切换 */
const BACKGROUNDS = ['grid', 'dark', 'light'] as const
type BackgroundMode = (typeof BACKGROUNDS)[number]

/** 模型本体统一用同一种冷调材质，避免导入的材质依赖贴图而显示成纯黑 */
const MODEL_COLOR = 0x8fbcd4
/** 坐标轴 gizmo 的视口边长与边距（CSS 像素） */
const GIZMO_SIZE = 86
const GIZMO_INSET = 14

interface ModelInfo {
    vertexCount: number
    faceCount: number
    format: string
}

const route = useRoute()
const { t } = useI18n()
const fileInfo = ref<FileInfo>()
const loading = ref(true)
const errorMsg = ref('')
const modelInfo = ref<ModelInfo>()
const wireframe = ref(false)
const autoRotate = ref(false)
const background = ref<BackgroundMode>('grid')

const canvasRef = ref<HTMLDivElement>()

// Three.js objects — shallowRef: 深响应会把整个场景树代理一遍，既慢又容易出错
const renderer = shallowRef<THREE.WebGLRenderer>()
const scene = shallowRef<THREE.Scene>()
const camera = shallowRef<THREE.PerspectiveCamera>()
const controls = shallowRef<OrbitControls>()
const modelGroup = shallowRef<THREE.Group>()
const grid = shallowRef<THREE.GridHelper>()
const gizmoCamera = shallowRef<THREE.OrthographicCamera>()

let animFrameId = 0
let stageW = 0
let stageH = 0
/** 模型包围盒，换背景重建网格时要按它重新对齐 */
let modelBox: THREE.Box3 | null = null

/** 坐标轴底板的定位：与 WebGL 里的小视口共用同一组常量，避免两处对不上 */
const gizmoStyle = {
    left: `${GIZMO_INSET}px`,
    bottom: `${GIZMO_INSET}px`,
    width: `${GIZMO_SIZE}px`,
    height: `${GIZMO_SIZE}px`,
}

const backgroundLabel = computed(() => {
    const key = { grid: 'model.bgGrid', dark: 'model.bgDark', light: 'model.bgLight' } as const
    return t(key[background.value])
})

// ── 场景 ──────────────────────────────────────────────────────────────────

/** 网格颜色随背景走：浅底上不能用深紫灰，否则网格比模型还抢眼 */
function createGrid(mode: BackgroundMode) {
    const [major, minor] = mode === 'light' ? [0xc9c9cf, 0xdedee4] : [0x44465c, 0x323444]
    const helper = new THREE.GridHelper(10, 20, major, minor)
    helper.visible = mode === 'grid'
    return helper
}

/** 网格贴到模型底面，尺寸按模型最大边放大三倍 */
function alignGrid() {
    const helper = grid.value
    if (!helper || !scene.value || !modelBox) return
    const center = modelBox.getCenter(new THREE.Vector3())
    const size = modelBox.getSize(new THREE.Vector3())
    const gridSize = Math.max(size.x, size.y, size.z) * 3 || 10
    helper.position.set(center.x, modelBox.min.y, center.z)
    helper.scale.setScalar(gridSize / 10)
}

function rebuildGrid() {
    const scn = scene.value
    if (!scn) return
    if (grid.value) {
        scn.remove(grid.value)
        grid.value.geometry.dispose()
        ;(grid.value.material as THREE.Material).dispose()
    }
    const helper = createGrid(background.value)
    scn.add(helper)
    grid.value = helper
    alignGrid()
}

function initScene(container: HTMLDivElement) {
    stageW = container.clientWidth
    stageH = container.clientHeight

    const scn = new THREE.Scene()
    scene.value = scn

    const cam = new THREE.PerspectiveCamera(45, stageW / stageH, 0.01, 10000)
    camera.value = cam

    // 背景交给 CSS 渐变，canvas 保持透明：这样「浅色」不用重新配光照，
    // 也不用给每套背景准备一张贴图
    const rdr = new THREE.WebGLRenderer({ antialias: true, alpha: true })
    rdr.setSize(stageW, stageH)
    rdr.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    rdr.setClearAlpha(0)
    container.appendChild(rdr.domElement)
    renderer.value = rdr

    const ctrl = new OrbitControls(cam, rdr.domElement)
    ctrl.enableDamping = true
    ctrl.dampingFactor = 0.05
    ctrl.autoRotateSpeed = 1.4
    controls.value = ctrl

    scn.add(new THREE.AmbientLight(0xffffff, 0.75))
    const key = new THREE.DirectionalLight(0xffffff, 1.6)
    key.position.set(5, 10, 7)
    scn.add(key)
    const fill = new THREE.DirectionalLight(0xffffff, 0.5)
    fill.position.set(-5, -5, -5)
    scn.add(fill)

    rebuildGrid()

    // 坐标轴指示器：独立场景 + 独立小视口，方向跟随主相机
    const gizmoScene = new THREE.Scene()
    const axes = new THREE.AxesHelper(1)
    ;(axes.material as THREE.Material).depthTest = false
    gizmoScene.add(axes)
    const gCam = new THREE.OrthographicCamera(-1.2, 1.2, 1.2, -1.2, 0.1, 20)
    gCam.position.set(0, 0, 6)
    gizmoCamera.value = gCam

    const animate = () => {
        animFrameId = requestAnimationFrame(animate)
        ctrl.update()
        rdr.setViewport(0, 0, stageW, stageH)
        rdr.render(scn, cam)

        // 清掉深度再画 gizmo，避免它被模型挡住；画完把视口还原给下一帧
        rdr.autoClear = false
        rdr.clearDepth()
        rdr.setViewport(GIZMO_INSET, GIZMO_INSET, GIZMO_SIZE, GIZMO_SIZE)
        gCam.quaternion.copy(cam.quaternion)
        rdr.render(gizmoScene, gCam)
        rdr.setViewport(0, 0, stageW, stageH)
        rdr.autoClear = true
    }
    animate()
}

function handleResize() {
    const container = canvasRef.value
    if (!container || !renderer.value || !camera.value) return
    stageW = container.clientWidth
    stageH = container.clientHeight
    camera.value.aspect = stageW / stageH
    camera.value.updateProjectionMatrix()
    renderer.value.setSize(stageW, stageH)
}

function fitCameraToObject(cam: THREE.PerspectiveCamera, ctrl: OrbitControls, group: THREE.Object3D) {
    const box = new THREE.Box3().setFromObject(group)
    const size = box.getSize(new THREE.Vector3())
    const center = box.getCenter(new THREE.Vector3())

    const maxDim = Math.max(size.x, size.y, size.z)
    const fov = cam.fov * (Math.PI / 180)
    const distance = Math.abs(maxDim / (2 * Math.tan(fov / 2))) * 1.8

    cam.position.set(center.x, center.y, center.z + distance)
    cam.near = distance / 100
    cam.far = distance * 100
    cam.updateProjectionMatrix()

    ctrl.target.copy(center)
    ctrl.update()
}

// ── 载入 ──────────────────────────────────────────────────────────────────

function getModelStats(object: THREE.Object3D, format: string): ModelInfo {
    let vertexCount = 0
    let faceCount = 0

    object.traverse(child => {
        const mesh = child as THREE.Mesh
        if (!mesh.isMesh) return
        const geometry = mesh.geometry
        vertexCount += geometry.attributes.position.count
        faceCount += geometry.index ? geometry.index.count / 3 : geometry.attributes.position.count / 3
    })

    return { vertexCount, faceCount, format: format.toUpperCase() }
}

/** 覆盖导入的材质：OBJ/FBX 等常常只带漫反射色，缺贴图时渲染出来几乎是黑的 */
function applyModelMaterial(root: THREE.Object3D) {
    root.traverse(child => {
        const mesh = child as THREE.Mesh
        if (mesh.isMesh) {
            mesh.material = new THREE.MeshStandardMaterial({
                color: MODEL_COLOR,
                side: THREE.DoubleSide,
                wireframe: wireframe.value,
            })
        }
    })
}

function applyWireframe(enabled: boolean) {
    modelGroup.value?.traverse(child => {
        const mesh = child as THREE.Mesh
        if (!mesh.isMesh) return
        const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material]
        materials.forEach(mat => ((mat as THREE.MeshStandardMaterial).wireframe = enabled))
    })
}

/** 部分格式（STL/PLY/3DS）以 Z 轴为朝上，统一转到 Y-up */
function wrapWithRotation(object: THREE.Object3D, rotationX: number, rotationY = 0) {
    object.rotation.x = rotationX
    object.rotation.y = rotationY
    const group = new THREE.Group()
    group.add(object)
    return group
}

type LoaderLike = {
    load: (url: string, onLoad: (result: never) => void, onProgress?: undefined, onError?: (e: unknown) => void) => void
}

function loadWith(loader: LoaderLike, url: string): Promise<unknown> {
    return new Promise((resolve, reject) => {
        loader.load(url, resolve as (result: never) => void, undefined, reject)
    })
}

/** 网格类格式没有材质信息，直接给一份标准材质 */
function createMesh(geometry: THREE.BufferGeometry) {
    return new THREE.Mesh(
        geometry,
        new THREE.MeshStandardMaterial({ color: MODEL_COLOR, side: THREE.DoubleSide, wireframe: wireframe.value }),
    )
}

async function loadModel(url: string, ext: string): Promise<THREE.Object3D> {
    switch (ext) {
        case 'gltf':
        case 'glb': {
            const gltf = (await loadWith(new GLTFLoader() as unknown as LoaderLike, url)) as { scene: THREE.Group }
            applyModelMaterial(gltf.scene)
            return gltf.scene
        }
        case 'stl': {
            const geometry = (await loadWith(new STLLoader() as unknown as LoaderLike, url)) as THREE.BufferGeometry
            geometry.computeVertexNormals()
            // STL 以 Z 轴朝上，转到 Y-up
            return wrapWithRotation(createMesh(geometry), -Math.PI / 2)
        }
        case 'ply': {
            const geometry = (await loadWith(new PLYLoader() as unknown as LoaderLike, url)) as THREE.BufferGeometry
            geometry.computeVertexNormals()
            return wrapWithRotation(createMesh(geometry), -Math.PI / 2)
        }
        case 'obj':
        case 'fbx':
        case '3mf':
        case 'amf': {
            const create = {
                obj: () => new OBJLoader(),
                fbx: () => new FBXLoader(),
                '3mf': () => new ThreeMFLoader(),
                amf: () => new AMFLoader(),
            }[ext]
            const root = (await loadWith(create() as unknown as LoaderLike, url)) as THREE.Object3D
            applyModelMaterial(root)
            return root
        }
        case 'dae': {
            const result = (await loadWith(new ColladaLoader() as unknown as LoaderLike, url)) as {
                scene?: THREE.Object3D
            }
            if (!result?.scene) throw new Error('Failed to load DAE file')
            applyModelMaterial(result.scene)
            return result.scene
        }
        case '3ds': {
            const root = (await loadWith(new TDSLoader() as unknown as LoaderLike, url)) as THREE.Object3D
            applyModelMaterial(root)
            return wrapWithRotation(root, -Math.PI / 2)
        }
        case 'wrl': {
            const root = (await loadWith(new VRMLLoader() as unknown as LoaderLike, url)) as THREE.Object3D
            applyModelMaterial(root)
            // WRL 的朝向是 -Z，绕 Y 轴转 180°
            return wrapWithRotation(root, 0, Math.PI)
        }
        case 'lwo':
        case 'lws': {
            const root = (await loadWith(new LWOLoader() as unknown as LoaderLike, url)) as THREE.Object3D
            applyModelMaterial(root)
            return root
        }
        default:
            throw new Error(t('model.unsupported', { ext }))
    }
}

// ── 交互 ──────────────────────────────────────────────────────────────────

function resetView() {
    if (!camera.value || !controls.value || !modelGroup.value) return
    fitCameraToObject(camera.value, controls.value, modelGroup.value)
}

function toggleWireframe() {
    wireframe.value = !wireframe.value
    applyWireframe(wireframe.value)
}

function cycleBackground() {
    const next = (BACKGROUNDS.indexOf(background.value) + 1) % BACKGROUNDS.length
    background.value = BACKGROUNDS[next]
}

function openByDefault() {
    const path = fileInfo.value?.path
    if (path) openPath(path)
}

watch(background, () => rebuildGrid())
watch(autoRotate, value => {
    if (controls.value) controls.value.autoRotate = value
})

// ── 生命周期 ──────────────────────────────────────────────────────────────

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const path = fileInfo.value.path as string
    const ext = (fileInfo.value.extension as string).toLowerCase()

    if (!canvasRef.value) return
    initScene(canvasRef.value)
    window.addEventListener('resize', handleResize)

    try {
        const group = await loadModel(convertFileSrc(path), ext)
        const root = group instanceof THREE.Group ? group : wrapWithRotation(group, 0)
        modelGroup.value = root
        scene.value!.add(root)
        scene.value!.updateMatrixWorld(true)

        // 空包围盒意味着文件里没有任何面（下错了格式、或被截断），
        // 继续往下走会算出 NaN 的相机距离，把整个舞台渲染成空白
        const box = new THREE.Box3().setFromObject(root)
        if (box.isEmpty()) throw new Error(t('model.empty'))

        modelInfo.value = getModelStats(root, ext)
        modelBox = box
        alignGrid()
        fitCameraToObject(camera.value!, controls.value!, root)
    } catch (e) {
        errorMsg.value = e instanceof Error ? e.message : String(e)
        cancelAnimationFrame(animFrameId)
    } finally {
        loading.value = false
        handleResize()
    }
})

onUnmounted(() => {
    cancelAnimationFrame(animFrameId)
    window.removeEventListener('resize', handleResize)
    grid.value?.geometry.dispose()
    renderer.value?.dispose()
    controls.value?.dispose()
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :hide-toolbar="!!errorMsg">
        <template #toolbar>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="wireframe"
                :title="t('model.wireframe')"
                :disabled="!modelInfo"
                @click="toggleWireframe"
            >
                <el-icon><Cube16Regular /></el-icon>
                {{ t('model.wireframe') }}
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('model.resetView')"
                :disabled="!modelInfo"
                @click="resetView"
            >
                <el-icon><ArrowSync16Regular /></el-icon>
                {{ t('model.resetView') }}
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="autoRotate"
                :title="t('model.autoRotate')"
                :disabled="!modelInfo"
                @click="autoRotate = !autoRotate"
            >
                <el-icon><ArrowClockwise16Regular /></el-icon>
                {{ t('model.autoRotate') }}
            </button>

            <i class="pv-tb-sep"></i>

            <button class="pv-tb-btn is-text" type="button" :title="t('model.backgroundHint')" @click="cycleBackground">
                <el-icon><PaintBucket16Regular /></el-icon>
                {{ t('model.background', { name: backgroundLabel }) }}
                <ChevronDown16Regular class="is-xs" />
            </button>

            <div class="pv-tb-grow"></div>

            <span v-if="modelInfo" class="pv-tb-info">
                {{ t('model.vertices', { count: modelInfo.vertexCount.toLocaleString() }) }} ·
                {{ t('model.faces', { count: modelInfo.faceCount.toLocaleString() }) }}
            </span>
        </template>

        <div class="md-wrap">
            <div class="md-stage" :class="[`bg-${background}`, { 'is-error': !!errorMsg }]">
                <!-- 坐标轴底板：canvas 透明，这块圆底从下面透出来，gizmo 才不会糊进场景里 -->
                <div class="md-gizmo" :style="gizmoStyle" aria-hidden="true"></div>
                <div ref="canvasRef" class="md-canvas" @dblclick="resetView"></div>

                <div v-if="loading" class="md-loading">
                    <i class="md-loading-bar"></i>
                    <span>{{ t('model.loading') }}</span>
                </div>

                <div v-if="errorMsg" class="md-error">
                    <svg class="md-error-icon" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M24 6 44 40H4z" />
                        <path d="M24 19v10M24 34.5v.5" />
                    </svg>
                    <h3>{{ t('model.loadFailed') }}</h3>
                    <p>{{ errorMsg }}</p>
                    <button class="md-primary" type="button" @click="openByDefault">
                        {{ t('header.openWithDefault') }}
                    </button>
                </div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.md-wrap {
    width: 100%;
    height: 100%;
    padding: 2px 12px 12px;
    box-sizing: border-box;
}

/* 画布内缩成圆角卡片：深色画面直接顶到浅色标题行会形成「上下两截」的硬边界，
   圆角 + 阴影把它收成一张“舞台”，与图片页的照片卡片一致。 */
.md-stage {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.1),
        0 10px 30px rgba(0, 0, 0, 0.14);
    transition: background 0.25s ease;
}

.bg-grid {
    background: radial-gradient(120% 120% at 50% 0%, #2a2f38 0%, #171a1f 58%, #0f1115 100%);
}

.bg-dark {
    background: #101114;
}

.bg-light {
    background: linear-gradient(180deg, #fbfbfd 0%, #e9e9ef 100%);
}

.md-gizmo {
    position: absolute;
    border-radius: 50%;
    /* 深色舞台上垫一层边缘渐隐的暗底，坐标轴才不会糊进场景；
       浅色舞台上这层暗底反而像一块污渍，直接去掉，红绿两线在白底上本来就够清楚 */
    background: radial-gradient(circle at 50% 50%, rgba(16, 18, 22, 0.42), rgba(16, 18, 22, 0.06));
    backdrop-filter: blur(2px);
    pointer-events: none;
}

.bg-light .md-gizmo {
    background: none;
    backdrop-filter: none;
}

.md-canvas {
    position: absolute;
    inset: 0;
    z-index: 1;

    .is-error & {
        visibility: hidden;
    }

    :deep(canvas) {
        display: block;
        width: 100% !important;
        height: 100% !important;
    }
}

/* ── 加载：只有一条不确定态进度条，说明「在动」而不假装知道百分比 ── */
.md-loading {
    position: absolute;
    z-index: 2;
    inset: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: 14px;
    font-size: var(--font-xs);
    color: rgba(255, 255, 255, 0.62);
    pointer-events: none;

    span {
        margin-top: 6px;
    }
}

.md-loading-bar {
    position: absolute;
    top: 0;
    left: 0;
    height: 2px;
    width: 32%;
    border-radius: var(--radius-xs);
    background: var(--color-accent);
    animation: md-indeterminate 1.15s ease-in-out infinite;
}

@keyframes md-indeterminate {
    0% {
        transform: translateX(-100%);
    }
    100% {
        transform: translateX(320%);
    }
}

/* ── 错态 ── */
.md-error {
    position: absolute;
    z-index: 3;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    background: var(--color-bg);
    color: var(--color-text-primary);

    &-icon {
        width: 44px;
        height: 44px;
        color: var(--color-text-tertiary);
    }

    h3 {
        margin: 2px 0 0;
        font-size: var(--font-lg);
        font-weight: 600;
    }

    p {
        margin: 0;
        max-width: 460px;
        font-size: var(--font-sm);
        line-height: var(--line-base);
        color: var(--color-text-secondary);
        word-break: break-word;
    }
}

.md-primary {
    margin-top: 8px;
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    font-family: inherit;
    font-size: var(--font-sm);
    cursor: pointer;

    &:hover {
        background: var(--btn-primary-hover-bg);
    }

    &:active {
        background: var(--btn-primary-active-bg);
    }
}
</style>
