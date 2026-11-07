<template>
  <AkDialog v-model="open" title="重新解包">
    <form id="relaunch-form" @submit.prevent="submit">
      <AkField label="版本">
        <AkInput
          :model-value="`${version.clientVersion}  ${version.resVersion}`"
          readonly
        />
      </AkField>
      <AkField label="对比的上一版" required>
        <AkSelect v-model="prevId" :options="prevOptions" />
      </AkField>
      <div class="relaunch__tasks">
        <AkField label="只跑这些任务">
          <AkInput v-model="include" placeholder="CharSpine, EnemySpine" />
        </AkField>
        <AkField label="跳过这些任务">
          <AkInput v-model="exclude" placeholder="Video, Audio" />
        </AkField>
      </div>
      <AkField label="令牌" required class="relaunch__last">
        <AkInput v-model="token" type="password" autocomplete="off" />
      </AkField>
      <AkMessage v-if="error" variant="danger" class="relaunch__error">
        {{ error }}
      </AkMessage>
    </form>
    <template #footer>
      <AkButton @click="open = false">取消</AkButton>
      <AkButton
        variant="primary"
        type="submit"
        form="relaunch-form"
        :loading="submitting"
        :disabled="!canSubmit"
      >
        开始解包
      </AkButton>
    </template>
  </AkDialog>
</template>

<script setup lang="ts">
import {
  AkButton,
  AkDialog,
  AkField,
  AkInput,
  AkMessage,
  AkSelect,
  useToast,
} from "@mooncellwiki/prts-design-vue";
import { computed, ref, watch } from "vue";
import { client } from "~/common/client";
import { useVersions, versionOptions } from "~/common/versions";
import type { Version } from "~/common/versions";

const props = defineProps<{ version: Version }>();
const open = defineModel<boolean>({ default: false });

const { versions, previous, byId } = useVersions();
const toast = useToast();

const prevId = ref<number | string | null>(null);
const include = ref("");
const exclude = ref("");
const token = ref("");
const submitting = ref(false);
const error = ref("");

const prevOptions = computed(() =>
  versionOptions(versions.value.filter((v) => v.id < props.version.id)),
);

watch(open, (isOpen) => {
  if (!isOpen) return;
  prevId.value = previous(props.version)?.id ?? null;
  error.value = "";
});

const canSubmit = computed(
  () => prevId.value != null && token.value.length > 0,
);

async function submit() {
  const prev = byId(Number(prevId.value));
  if (!prev || !token.value) return;
  submitting.value = true;
  error.value = "";
  try {
    const { data, response } = await client.POST("/api/v1/docker/launch", {
      body: {
        client_version: props.version.clientVersion,
        res_version: props.version.resVersion,
        prev_client_version: prev.clientVersion,
        prev_res_version: prev.resVersion,
        include: include.value.trim() || null,
        exclude: exclude.value.trim() || null,
      },
      headers: { "torappu-auth": token.value },
    });
    if (data) {
      toast.success(`解包任务 ${data.container_name} 已启动`);
      open.value = false;
    } else if (response.status === 401) {
      error.value = "令牌不对";
    } else if (response.status === 400) {
      error.value = "参数不对，检查版本与任务名";
    } else {
      error.value = `没能启动解包（HTTP ${response.status}）`;
    }
  } catch {
    error.value = "连不上服务";
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.relaunch {
  &__tasks {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ak-space-4);

    @media (max-width: $bp-phone) {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  &__last {
    margin-bottom: 0;
  }

  &__error {
    margin: var(--ak-space-4) 0 0;
  }
}
</style>
