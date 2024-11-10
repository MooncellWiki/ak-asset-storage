import { onBeforeMount, ref } from "vue";
import { client } from "./client";
import type { components } from "./schema";
import type { SelectGroupOption } from "naive-ui";

export function useVersionSelect() {
  const versionOpts = ref<SelectGroupOption[]>([]);
  const versions = ref<components["schemas"]["VersionModel"][]>([]);
  onBeforeMount(async () => {
    const { data } = await client.GET("/api/v1/version");
    const result: SelectGroupOption[] = [];
    versions.value = (data ?? []).reverse();
    let prev = "";
    let group: SelectGroupOption | undefined = void 0;
    for (const version of versions.value) {
      if (prev !== version.client) {
        prev = version.client;
        if (group) {
          result.push(group);
        }
        group = {
          type: "group",
          label: prev,
          key: prev,
          children: [
            {
              label: version.res,
              value: version.id,
            },
          ],
        };
      } else {
        group!.children!.push({ label: version.res, value: version.id });
      }
    }
    result.push(group!);
    versionOpts.value = result;
  });
  return {
    versionOpts,
    versions,
  };
}
