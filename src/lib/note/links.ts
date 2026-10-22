/** Bare web addresses typed into a note; ASCII-only so Korean text right after a URL is not swallowed. */
export const URL_PATTERN = /\b(?:https?:\/\/|www\.)[\w\-.~:/?#[\]@!$&'()*+,;=%]+/gi;

const TRAILING = new Set(".,;:!?'*");

/** Drops sentence punctuation and unmatched closing brackets, e.g. "(see https://a.com)." → "https://a.com". */
export function trimUrl(text: string): string {
  let parentheses = 0, brackets = 0;
  for (const char of text) {
    if (char === "(") parentheses++;
    else if (char === ")") parentheses--;
    else if (char === "[") brackets++;
    else if (char === "]") brackets--;
  }
  let end = text.length;
  while (end > 0) {
    const char = text[end - 1];
    if (char === ")" && parentheses < 0) parentheses++;
    else if (char === "]" && brackets < 0) brackets++;
    else if (!TRAILING.has(char)) break;
    end--;
  }
  return text.slice(0, end);
}

/** The address handed to the browser, or null when the text is not an openable web URL. */
export function linkTarget(url: string): string | null {
  const href = /^www\./i.test(url) ? `https://${url}` : url;
  try {
    const parsed = new URL(href);
    const web = parsed.protocol === "http:" || parsed.protocol === "https:";
    return web && (parsed.hostname.includes(".") || parsed.hostname === "localhost") ? href : null;
  } catch {
    return null;
  }
}

export interface NoteLink { from: number; to: number; href: string }

export function findLinks(text: string): NoteLink[] {
  const links: NoteLink[] = [];
  for (const match of text.matchAll(URL_PATTERN)) {
    const url = trimUrl(match[0]), href = linkTarget(url);
    if (href) links.push({ from: match.index, to: match.index + url.length, href });
  }
  return links;
}
