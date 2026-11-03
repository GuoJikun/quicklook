<script setup lang="ts">
import { RouterView } from 'vue-router'
import { elementLocale } from '@/locales'
</script>

<template>
    <el-config-provider :locale="elementLocale">
        <RouterView />
    </el-config-provider>
</template>
