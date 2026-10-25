<template>
  <button
    type="button"
    class="ak-btn ak-btn--ghost ak-btn--icon"
    :class="[`ak-btn--${size}`]"
    :title="label"
    :aria-label="label"
    @click="copy(props.text, message)"
  >
    <AsIcon :name="icon" :size="size === 'xs' ? 14 : 16" />
  </button>
</template>

<script setup lang="ts">
import { useCopy } from "~/common/clipboard";
import AsIcon from "./AsIcon.vue";

const props = withDefaults(
  defineProps<{
    /** Text put on the clipboard */
    text: string;
    /** Accessible name and tooltip */
    label?: string;
    size?: "xs" | "sm";
    icon?: "copy" | "link";
    /** Toast text after copying */
    message?: string;
  }>(),
  { label: "复制", size: "xs", icon: "copy", message: "已复制" },
);

const copy = useCopy();
</script>
