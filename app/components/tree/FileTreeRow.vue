<template>
  <span
    class="caret"
    :class="{ 'is-open': row.open }"
    @click.stop="row.node.isDir && emit('toggle')"
  >
    <AsIcon v-if="row.node.isDir" name="chevron-right" :size="12" />
  </span>
  <ChangeMark v-if="row.node.mark" :kind="row.node.mark" />
  <AsIcon
    v-else-if="row.depth > 0 || !row.node.keep"
    :name="icon"
    :size="16"
    class="icon"
  />
  <span class="name">
    <span class="name__head"
      ><template v-for="(p, i) in parts.head" :key="i"
        ><mark v-if="p.mark">{{ p.text }}</mark
        ><template v-else>{{ p.text }}</template></template
      ></span
    >
    <span v-if="parts.tail.length" class="name__tail"
      ><template v-for="(p, i) in parts.tail" :key="i"
        ><mark v-if="p.mark">{{ p.text }}</mark
        ><template v-else>{{ p.text }}</template></template
      ></span
    >
  </span>
  <ChangeCounts
    v-if="row.node.counts"
    class="counts"
    :counts="row.node.counts"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
import AsIcon from "../AsIcon.vue";
import ChangeCounts from "../ChangeCounts.vue";
import ChangeMark from "../ChangeMark.vue";
import { nameParts } from "./model";
import type { Row } from "./model";

/** The content of one FileTree row: caret, icon or change mark, name, counts. */
const props = defineProps<{
  row: Row & { kind: "node" };
  highlight?: string;
}>();
const emit = defineEmits<{ toggle: [] }>();

const parts = computed(() =>
  nameParts(props.row.label, props.row.node.isDir, props.highlight),
);
const icon = computed(() => {
  const { node, open } = props.row;
  if (node.isDir) return open ? "folder-open" : (node.icon ?? "folder");
  return node.icon ?? "file";
});
</script>

<style scoped lang="scss">
.caret {
  flex: none;
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  color: var(--ak-fg-subtle);
  transition: transform var(--ak-dur-fast);

  &.is-open {
    transform: rotate(90deg);
  }
}

.icon {
  color: var(--ak-fg-muted);

  .is-selected > & {
    color: var(--ak-accent);
  }
}

.name {
  flex: 1;
  min-width: 0;
  display: flex;
  white-space: pre;

  &__head {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  &__tail {
    flex: none;
  }

  mark {
    padding: 0;
    background: var(--ak-mark-bg);
    color: inherit;
  }
}

.counts {
  flex: none;
  font-size: 12px;
}
</style>
