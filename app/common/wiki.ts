/** Story pages of prts.wiki, found through its `story` Cargo table. */
const API = "https://prts.wiki/api.php";
const PAGE = "https://prts.wiki/w/";
/** Keeps the URL short; Cargo itself caps a query at 500 rows. */
const BATCH = 50;

/** Script path → page URL, undefined when the wiki has no page for it. */
const known = new Map<string, string | undefined>();

interface CargoResponse {
  cargoquery?: { title: { page: string; textPath: string } }[];
}

function pageUrl(title: string) {
  const path = title.replaceAll(" ", "_").split("/");
  return PAGE + path.map((part) => encodeURIComponent(part)).join("/");
}

async function lookUp(scripts: string[]) {
  const quoted = scripts.map((s) => `'${s.replaceAll("'", "''")}'`);
  const query = new URLSearchParams({
    // Anonymous CORS; *.prts.wiki and everyone else alike.
    origin: "*",
    action: "cargoquery",
    format: "json",
    tables: "story",
    fields: "_pageName=page,textPath",
    where: `textPath IN (${quoted.join(",")})`,
    limit: String(BATCH),
  });
  const response = await fetch(`${API}?${query}`);
  const { cargoquery } = (await response.json()) as CargoResponse;
  if (!cargoquery) throw new Error("story page lookup failed");
  for (const s of scripts) known.set(s, undefined);
  for (const { title } of cargoquery)
    known.set(title.textPath, pageUrl(title.page));
}

/**
 * Wiki page of each story script (its path under `story/`, without `.txt`).
 * Scripts without a page are left out. So are those of a batch the wiki did
 * not answer: the link is optional, and they are looked up again next time.
 */
export async function loadStoryPages(
  scripts: string[],
): Promise<Map<string, string>> {
  const unknown = [...new Set(scripts)].filter((s) => !known.has(s));
  const batches: string[][] = [];
  for (let i = 0; i < unknown.length; i += BATCH)
    batches.push(unknown.slice(i, i + BATCH));
  await Promise.allSettled(batches.map((batch) => lookUp(batch)));

  const pages = new Map<string, string>();
  for (const s of scripts) {
    const page = known.get(s);
    if (page) pages.set(s, page);
  }
  return pages;
}

/** The script a gamedata file holds: `gamedata/<version>/story/<script>.txt` */
export function storyScript(path: string): string | undefined {
  return /^gamedata\/[^/]+\/story\/(.+)\.txt$/u.exec(path)?.[1];
}
