import { useToast } from "@mooncellwiki/prts-design-vue";

/** Copies text and confirms with a toast. Call from a component's setup. */
export function useCopy() {
  const toast = useToast();
  return async function copy(text: string, what = "已复制") {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(what, { duration: 3000 });
    } catch {
      toast.error("复制失败，浏览器没有给剪贴板权限");
    }
  };
}
