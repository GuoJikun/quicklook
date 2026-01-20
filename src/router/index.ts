import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/home/index.vue'
import previewView from '../views/preview/index.vue'

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: '/',
            name: 'home',
            component: HomeView,
        },
        {
            path: '/preview',
            name: 'preview',
            component: previewView,
            children: [
                {
                    path: '/preview/not-support',
                    name: 'previewNotSupport',
                    component: () => import('@/views/preview/not-support.vue'),
                },
                {
                    path: '/preview/video',
                    name: 'previewVideo',
                    component: () => import('@/views/preview/video.vue'),
                },
                {
                    path: '/preview/image',
                    name: 'previewImage',
                    component: () => import('@/views/preview/image.vue'),
                },
                {
                    path: '/preview/audio',
                    name: 'previewAudio',
                    component: () => import('@/views/preview/audio.vue'),
                },
                {
                    path: '/preview/code',
                    name: 'previewCode',
                    component: () => import('@/views/preview/code.vue'),
                },
                {
                    path: '/preview/font',
                    name: 'previewFont',
                    component: () => import('@/views/preview/font.vue'),
                },
                {
                    path: '/preview/md',
                    name: 'previewMd',
                    component: () => import('@/views/preview/md.vue'),
                },
                {
                    path: '/preview/book',
                    name: 'previewBook',
                    component: () => import('@/views/preview/book.vue'),
                },
                {
                    path: '/preview/archive',
                    name: 'previewArchive',
                    component: () => import('@/views/preview/archive.vue'),
                },
                {
                    path: '/preview/document',
                    name: 'previewDocument',
                    component: () => import('@/views/preview/document.vue'),
                },
            ],
        },
        {
            path: '/settings',
            name: 'settings',
            component: () => import('@/views/settings.vue'),
        },
        {
            path: '/upgrade',
            name: 'upgrade',
            component: () => import('@/views/upgrade.vue'),
        },
    ],
})

export default router
