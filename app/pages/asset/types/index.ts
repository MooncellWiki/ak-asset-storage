export interface TreeNode extends Record<string, unknown> {
  key: string;
  label: string;
  path: string;
  is_dir: boolean;
  size: number;
  isLeaf?: boolean;
  children?: TreeNode[];
}
