/**
 * Titleizes a string (e.g. "magic chalk" -> "Magic Chalk", "charm sigil" -> "Charm Sigil", "chisel-axe" -> "Chisel-Axe").
 * Preserves existing uppercase characters and treats spaces, hyphens, underscores, and slashes as word boundaries.
 * Does not capitalize following an apostrophe (e.g. "shaman's ring" -> "Shaman's Ring").
 */
export function titleize(str: string): string {
  if (!str) return str
  return str.replace(/(?:^|[\s\-_/])\S/g, match => match.toUpperCase())
}
