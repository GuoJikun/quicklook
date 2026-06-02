<script setup lang="ts">
import { ref } from 'vue'

const tmp = ref()
const init = async () => {
    console.log('init', tmp)
}

init()
</script>

<template>
    <div class="preview">Home</div>
</template>

<style scoped lang="scss">
.preview {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    position: relative;
    &-header {
        position: absolute;
        left: 0;
        top: 0;
        width: 100%;
    }
    &-footer {
        position: absolute;
        left: 0;
        bottom: 0;
        width: 100%;
        font-size: var(--font-xs);
    }
    &-body {
        padding: 28px 0 20px;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100%;
    }
}
</style>
