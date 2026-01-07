import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: '/',
            name: 'home',
            component: () => import('../views/Home.vue')
        },
        {
            path: '/test',
            name: 'test',
            component: () => import('../views/Test.vue')
        },
        {
            path: '/project/:id',
            name: 'project',
            component: () => import('../views/Project.vue')
        },
        {
            path: '/project/:projectId/page/:pageId',
            name: 'page',
            component: () => import('../views/Page.vue')
        }
    ]
})

export default router