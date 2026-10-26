import { bundledLanguages, bundledLanguagesAlias, codeToTokens } from "shiki";
import { createCssVariablesTheme } from "shiki/core";
import type { BundledLanguage } from "shiki";

/**
 * Shiki colours every token with a CSS variable from this theme; the
 * variables are mapped back to the mockup's `.as-code` token classes below,
 * so highlighting reads PRTS Design tokens and follows the light / dark theme.
 */
const theme = createCssVariablesTheme({
  name: "prts-design",
  variablePrefix: "--as-code-",
});

const CLASSES: Record<string, string> = {
  "token-keyword": "k",
  "token-string": "s",
  "token-string-expression": "s",
  "token-constant": "n",
  "token-punctuation": "p",
  "token-comment": "c",
  "token-function": "f",
  "token-parameter": "v",
  "token-link": "l",
  "token-inserted": "i",
  "token-deleted": "d",
};
// Shiki FontStyle bit flags
const ITALIC = 1;
const BOLD = 2;

export interface CodeToken {
  text: string;
  /** Space-separated .as-code token classes */
  cls?: string;
}

/** Shiki language id for a file name, if Shiki knows the extension. */
export function languageOf(name: string): BundledLanguage | undefined {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return undefined;
  const ext = name.slice(dot + 1).toLowerCase();
  return ext in bundledLanguages || ext in bundledLanguagesAlias
    ? (ext as BundledLanguage)
    : undefined;
}

/** Tokenises `code` line by line; the grammar is loaded on first use. */
export async function highlightLines(
  code: string,
  lang: BundledLanguage,
): Promise<CodeToken[][]> {
  const { tokens } = await codeToTokens(code, { lang, theme });
  return tokens.map((line) =>
    line.map((t) => {
      const variable = /--as-code-([a-z-]+)/u.exec(t.color ?? "")?.[1];
      const classes = [variable && CLASSES[variable]];
      const style = t.fontStyle ?? 0;
      if (style & ITALIC) classes.push("it");
      if (style & BOLD) classes.push("bd");
      const cls = classes.filter(Boolean).join(" ");
      return cls ? { text: t.content, cls } : { text: t.content };
    }),
  );
}
