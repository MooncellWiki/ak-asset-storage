import type { TreeOption } from "naive-ui";

export interface TreeNode extends TreeOption {
  path: string;
  is_dir: boolean;
  size: number;
  isLeaf?: boolean;
  children?: TreeNode[];
}
