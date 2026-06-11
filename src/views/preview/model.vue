<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js'
import { OBJLoader } from 'three/examples/jsm/loaders/OBJLoader.js'
import LayoutPreview from '@/components/layout-preview.vue'
import type { FileInfo } from '@/utils/typescript'

defineOptions({ name: 'ModelSupport' })

interface ModelInfo {
    vertex_count: number
    face_count: number
    format: string
}

const route = useRoute()
const fileInfo = ref<FileInfo>()
const loading = ref(true)
const errorMsg = ref('')
const modelInfo = ref<ModelInfo>()
const wireframe = ref(false)

const canvasRef = ref<HTMLDivElement>()

// Three.js objects — shallowRef: no deep reactive proxy needed
const renderer = shallowRef<THREE.WebGLRenderer>()
const scene = shallowRef<THREE.Scene>()
const camera = shallowRef<THREE.PerspectiveCamera>()
const controls = shallowRef<OrbitControls>()
const modelGroup = shallowRef<THREE.Group>()
let animFrameId = 0

// ── helpers ──────────────────────────────────────────────────────────────────

function fitCameraToObject(cam: THREE.PerspectiveCamera, ctrl: OrbitControls, group: THREE.Group) {
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

function applyWireframe(group: THREE.Group, enabled: boolean) {
    group.traverse(child => {
        if ((child as THREE.Mesh).isMesh) {
            const mesh = child as THREE.Mesh
            const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material]
            materials.forEach(mat => {
                ;(mat as THREE.MeshStandardMaterial).wireframe = enabled
            })
        }
    })
}

// ── loaders ──────────────────────────────────────────────────────────────────

async function loadGltf(url: string): Promise<THREE.Group> {
    return new Promise((resolve, reject) => {
        new GLTFLoader().load(url, gltf => resolve(gltf.scene), undefined, reject)
    })
}

async function loadStl(url: string): Promise<THREE.Group> {
    return new Promise((resolve, reject) => {
        new STLLoader().load(
            url,
            geometry => {
                geometry.computeVertexNormals()
                const mat = new THREE.MeshStandardMaterial({ color: 0x8fbcd4, side: THREE.DoubleSide })
                const mesh = new THREE.Mesh(geometry, mat)
                const group = new THREE.Group()
                group.add(mesh)
                resolve(group)
            },
            undefined,
            reject,
        )
    })
}

async function loadObj(url: string): Promise<THREE.Group> {
    return new Promise((resolve, reject) => {
        new OBJLoader().load(
            url,
            group => {
                group.traverse(child => {
                    if ((child as THREE.Mesh).isMesh) {
                        ;(child as THREE.Mesh).material = new THREE.MeshStandardMaterial({
                            color: 0x8fbcd4,
                            side: THREE.DoubleSide,
                        })
                    }
                })
                resolve(group)
            },
            undefined,
            reject,
        )
    })
}

// ── scene setup ──────────────────────────────────────────────────────────────

function initScene(container: HTMLDivElement) {
    const w = container.clientWidth
    const h = container.clientHeight

    const scn = new THREE.Scene()
    scn.background = new THREE.Color(0x1e1e2e)
    scene.value = scn

    const cam = new THREE.PerspectiveCamera(45, w / h, 0.01, 10000)
    camera.value = cam

    const rdr = new THREE.WebGLRenderer({ antialias: true })
    rdr.setSize(w, h)
    rdr.setPixelRatio(window.devicePixelRatio)
    rdr.shadowMap.enabled = true
    container.appendChild(rdr.domElement)
    renderer.value = rdr

    const ctrl = new OrbitControls(cam, rdr.domElement)
    ctrl.enableDamping = true
    ctrl.dampingFactor = 0.05
    controls.value = ctrl

    // Lights
    const ambient = new THREE.AmbientLight(0xffffff, 0.6)
    scn.add(ambient)
    const dir1 = new THREE.DirectionalLight(0xffffff, 1.2)
    dir1.position.set(5, 10, 7)
    scn.add(dir1)
    const dir2 = new THREE.DirectionalLight(0xffffff, 0.4)
    dir2.position.set(-5, -5, -5)
    scn.add(dir2)

    // Grid
    const grid = new THREE.GridHelper(10, 20, 0x444466, 0x333355)
    scn.add(grid)

    // Render loop
    const animate = () => {
        animFrameId = requestAnimationFrame(animate)
        ctrl.update()
        rdr.render(scn, cam)
    }
    animate()
}

function handleResize() {
    const container = canvasRef.value
    if (!container || !renderer.value || !camera.value) return
    const w = container.clientWidth
    const h = container.clientHeight
    camera.value.aspect = w / h
    camera.value.updateProjectionMatrix()
    renderer.value.setSize(w, h)
}

// ── public actions ────────────────────────────────────────────────────────────

function resetView() {
    if (!camera.value || !controls.value || !modelGroup.value) return
    fitCameraToObject(camera.value, controls.value, modelGroup.value)
}

function toggleWireframe() {
    wireframe.value = !wireframe.value
    if (modelGroup.value) applyWireframe(modelGroup.value, wireframe.value)
}

// ── lifecycle ─────────────────────────────────────────────────────────────────

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const path = fileInfo.value.path as string
    const ext = (fileInfo.value.extension as string).toLowerCase()

    // Fetch model statistics from Rust backend
    try {
        modelInfo.value = await invoke<ModelInfo>('load_model', { path, extension: ext })
    } catch (e) {
        // Non-fatal: stats unavailable but we can still render
        console.warn('load_model error:', e)
    }

    // Init Three.js scene
    if (!canvasRef.value) return
    initScene(canvasRef.value)

    // Load 3D model via Tauri asset protocol
    const assetUrl = convertFileSrc(path)
    try {
        let group: THREE.Group
        if (ext === 'gltf' || ext === 'glb') {
            group = await loadGltf(assetUrl)
        } else if (ext === 'stl') {
            group = await loadStl(assetUrl)
        } else if (ext === 'obj') {
            group = await loadObj(assetUrl)
        } else {
            throw new Error(`不支持的格式: .${ext}`)
        }

        modelGroup.value = group
        scene.value!.add(group)
        fitCameraToObject(camera.value!, controls.value!, group)

        // Sync grid to model base
        const box = new THREE.Box3().setFromObject(group)
        const center = box.getCenter(new THREE.Vector3())
        const size = box.getSize(new THREE.Vector3())
        const gridSize = Math.max(size.x, size.z) * 3
        scene.value!.children
            .filter(c => c instanceof THREE.GridHelper)
            .forEach(g => {
                g.position.set(center.x, box.min.y, center.z)
                g.scale.setScalar(gridSize / 10)
            })
    } catch (e) {
        errorMsg.value = e instanceof Error ? e.message : String(e)
    } finally {
        loading.value = false
    }

    window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
    cancelAnimationFrame(animFrameId)
    renderer.value?.dispose()
    controls.value?.dispose()
    window.removeEventListener('resize', handleResize)
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <div class="model-support">
            <!-- 3D canvas -->
            <div ref="canvasRef" class="model-canvas" />

            <!-- Error overlay -->
            <div v-if="errorMsg" class="model-overlay model-error">
                <span class="error-icon">⚠</span>
                <p>{{ errorMsg }}</p>
            </div>

            <!-- Controls panel -->
            <div v-if="!loading && !errorMsg" class="model-controls">
                <button class="ctrl-btn" :class="{ active: wireframe }" title="线框模式" @click="toggleWireframe">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path
                            d="M1 1h6v6H1zm8 0h6v6h-6zM1 9h6v6H1zm8 0h6v6h-6z"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                </button>
                <button class="ctrl-btn" title="重置视角" @click="resetView">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path
                            d="M8 2a6 6 0 1 0 5.196 3H11.5a4.5 4.5 0 1 1-1.5-1.5V5l3-3-3-3v1.5A6 6 0 0 0 8 2z"
                        />
                    </svg>
                </button>
            </div>

            <!-- Model info panel -->
            <div v-if="modelInfo && !loading && !errorMsg" class="model-info">
                <span class="info-tag">{{ modelInfo.format.toUpperCase() }}</span>
                <span class="info-item">顶点 {{ modelInfo.vertex_count.toLocaleString() }}</span>
                <span class="info-sep">·</span>
                <span class="info-item">面 {{ modelInfo.face_count.toLocaleString() }}</span>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.model-support {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #1e1e2e;
}

.model-canvas {
    width: 100%;
    height: 100%;

    :deep(canvas) {
        display: block;
        width: 100% !important;
        height: 100% !important;
    }
}

.model-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
}

.model-error {
    background: rgba(30, 30, 46, 0.9);
    color: #f38ba8;
    font-size: 14px;

    .error-icon {
        font-size: 32px;
    }

    p {
        margin: 0;
    }
}

.model-controls {
    position: absolute;
    top: 12px;
    right: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    background: rgba(30, 30, 46, 0.8);
    color: rgba(255, 255, 255, 0.75);
    cursor: pointer;
    transition:
        background 0.15s,
        color 0.15s,
        border-color 0.15s;

    &:hover {
        background: rgba(99, 102, 241, 0.3);
        border-color: rgba(99, 102, 241, 0.6);
        color: #fff;
    }

    &.active {
        background: rgba(99, 102, 241, 0.5);
        border-color: #6366f1;
        color: #fff;
    }
}

.model-info {
    position: absolute;
    bottom: 12px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 6px;
    background: rgba(30, 30, 46, 0.85);
    backdrop-filter: blur(4px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
    pointer-events: none;
}

.info-tag {
    background: rgba(99, 102, 241, 0.4);
    color: #a5b4fc;
    border-radius: 4px;
    padding: 1px 6px;
    font-weight: 600;
    letter-spacing: 0.05em;
}

.info-sep {
    opacity: 0.4;
}
</style>
