<template>
  <ul class="res-list">
    <slot />
  </ul>
</template>

<script setup lang="ts">
/** A list of ResItem rows (story resources, siblings, search results). */
defineSlots<{ default?: () => unknown }>();
</script>

<style scoped lang="scss">
.res-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
</style>
