// B-1: the bridge between tokens.css and three.js.
//
// The palette law (T1) says a colour VALUE lives in tokens.css and nowhere
// else. three.js materials take a THREE.Color, which cannot parse
// `var(--color-…)` the way an SVG fill can — so the values are declared as
// tokens and resolved here, once, at first use. Nothing in this folder holds
// a hex literal; change a collector colour in tokens.css and the 3D unit
// changes with it.
//
// Read lazily rather than at module scope: this module lives in a lazy chunk
// that only loads after the stylesheet is applied, but a lazy read costs
// nothing and removes the ordering question entirely.

const cache = new Map<string, string>();

/** The computed value of one design token, e.g. token("coll-steel"). */
export function token(name: string): string {
  const cached = cache.get(name);
  if (cached !== undefined) return cached;
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(`--color-${name}`)
    .trim();
  cache.set(name, value);
  return value;
}

/** Standard-material props for the unit's recurring surfaces. */
export const materials = {
  steel: () => ({ color: token("coll-steel"), metalness: 0.55, roughness: 0.45 }),
  steelDark: () => ({
    color: token("coll-steel-dark"),
    metalness: 0.6,
    roughness: 0.5,
  }),
  drum: () => ({ color: token("coll-drum"), metalness: 0.35, roughness: 0.55 }),
};
