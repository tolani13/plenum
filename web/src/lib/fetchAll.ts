// P5 (R9c): the fetch-all idiom, unified. Every list surface fetches the
// whole scoped result at the API's pagination law ceiling (limit=200 ≥ every
// seeded total), which is what makes client ranking, user sorting, and CSV
// honest against one complete result set. The three hook modules used to
// carry their own `const LIMIT = 200` + `const q = encodeURIComponent`
// copies — this is now the one home for both. Behavior-identical.

/** The API's MAX_LIMIT — one page holds the whole scoped set. */
export const FETCH_LIMIT = 200;

/** URL-component encoder, aliased where query strings are assembled. */
export const q = encodeURIComponent;
