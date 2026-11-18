<template>
  <PaneBox tag="aside" label="详情">
    <template #head>
      <h2 class="detail__title">
        <ChangeMark v-if="kind" :kind="kind" />
        <span class="detail__path">{{ path }}</span>
      </h2>
    </template>
    <template #head-end>
      <CopyButton :text="path" label="复制路径" size="sm" />
      <button
        type="button"
        class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
        title="关闭"
        aria-label="关闭"
        @click="emit('close')"
      >
        <AsIcon name="x" :size="16" />
      </button>
    </template>

    <div class="compare">
      <div
        v-for="(side, index) in [before, after]"
        :key="index"
        class="compare__col"
        :class="{ 'is-new': index === 1 }"
      >
        <div class="compare__ver">{{ side.version }}</div>
        <template v-if="side.size != null">
          <div class="compare__size">{{ formatSize(side.size) }}</div>
          <span
            v-if="index === 1 && delta != null"
            class="compare__delta"
            :class="{ 'is-up': delta > 0, 'is-down': delta < 0 }"
          >
            {{ kind === "same" ? "未变化" : formatDelta(delta) }}
          </span>
          <div v-if="side.hash" class="compare__hash" :title="side.hash">
            {{ side.hash.slice(0, 12) }}…
          </div>
          <slot :name="index === 0 ? 'before-action' : 'after-action'" />
        </template>
        <div v-else class="compare__none">
          {{ side.missing ?? "这一版没有" }}
        </div>
      </div>
    </div>

    <slot />
    <template v-if="$slots.foot" #foot><slot name="foot" /></template>
  </PaneBox>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatDelta, formatSize } from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import ChangeMark from "../ChangeMark.vue";
import CopyButton from "../CopyButton.vue";
import PaneBox from "../layout/PaneBox.vue";
import type { ChangeKind } from "~/common/bundles";

interface Side {
  version: string;
  size?: number | null;
  hash?: string | null;
  missing?: string;
}

const props = defineProps<{
  path: string;
  kind?: ChangeKind;
  before: Side;
  after: Side;
}>();
const emit = defineEmits<{ close: [] }>();
defineSlots<{
  "before-action"?: () => unknown;
  "after-action"?: () => unknown;
  default?: () => unknown;
  foot?: () => unknown;
}>();

const delta = computed(() => {
  const { before, after } = props;
  return before.size != null && after.size != null
    ? after.size - before.size
    : undefined;
});
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.detail {
  &__title {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    font-size: var(--ak-fs-body);
    font-weight: 700;
  }

  &__path {
    @include mono;

    overflow-wrap: anywhere;
  }
}

// Old / new columns; the new side is tinted.
.compare {
  display: grid;
  grid-template-columns: 1fr 1fr;
  border-bottom: 1px solid var(--ak-border);

  &__col {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
    padding: var(--ak-space-4);

    & + & {
      border-left: 1px solid var(--ak-border);
    }

    &.is-new {
      background: var(--ak-accent-subtle);
    }
  }

  &__ver {
    font: 600 12px/1.3 var(--ak-font-mono);
    color: var(--ak-fg-muted);
    overflow-wrap: anywhere;
  }

  &__size {
    font: 700 var(--ak-fs-h3) / 1.2 var(--ak-font-label);
    font-variant-numeric: tabular-nums;
  }

  &__delta {
    display: block;
    font-size: var(--ak-fs-sm);
    font-weight: 600;
    color: var(--ak-fg-muted);

    &.is-up {
      color: var(--ak-rt-vup, var(--ak-accent));
    }

    &.is-down {
      color: var(--ak-rt-vdown, var(--ak-warning));
    }
  }

  &__hash {
    margin-bottom: 8px;
    font: 12px/1.3 var(--ak-font-mono);
    color: var(--ak-fg-muted);
  }

  &__none {
    font-size: var(--ak-fs-sm);
    color: var(--ak-fg-subtle);
  }

  @media (max-width: $bp-phone) {
    grid-template-columns: 1fr;

    &__col + &__col {
      border-left: 0;
    }
  }
}
</style>
