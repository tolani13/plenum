-- P5 Polish + Territory Map + signal auto-expiry, schema layer
-- (constraint 6 — ADDITIVE ONLY).
--
-- Nothing here is destructive: one new config table (seeded here, the
-- discount_policy pattern), one new enum VALUE, one nullable column, one
-- measured index, and generate_signals() re-created with the R4 expiry step
-- (DROP + CREATE because the RETURNS TABLE shape gains the per-type expired
-- count — CREATE OR REPLACE cannot change a return type; the function holds
-- no data, so this is a code swap, not a rewrite). No column dropped, no
-- type rewritten, no data touched.

-- ── territory_states: territory geography becomes seed-config (R3) ──────────
-- Keyed by TERRITORY CODE, not id, deliberately: the seed wipe is
-- `TRUNCATE territories … CASCADE`, and CASCADE truncates every table with
-- an FK into the truncated set — an FK here would silently wipe this config
-- on every reseed, defeating the "survives reseed" requirement. Codes are
-- unique and stable across reseeds (the P4 tile-match precedent: summary
-- rows match Territory Board tiles by CODE), so the join
-- `territories t ON t.code = ts.territory_code` is equivalent to an id FK
-- for every read path, and the config row set is truncate-proof.
--
-- Mapping: all 50 states + DC across the six US territories along US Census
-- division lines (each seed city-pool state maps to its own territory — the
-- PRE-2 alignment check proved 100% of every territory's dollars land in its
-- mapped set):
--   NE-1  Northeast + Mid-Atlantic: ME NH VT MA RI CT NY NJ PA MD DE DC
--   SE-1  Southeast:                VA WV NC SC GA FL AL TN KY MS
--   MW-1  Midwest:                  OH MI IN IL WI MN IA MO ND SD NE KS
--   SC-1  West South Central:       TX OK AR LA
--   MT-1  Mountain:                 MT ID WY NV UT CO AZ NM
--   W-1   Pacific:                  WA OR CA AK HI
-- Canada: CA-E / CA-W are the map's two schematic block codes (the SVG's
-- data-state values); the provinces/territories map alongside them so
-- site-state joins (ON QC … / AB BC MB SK …) attribute Canadian dollars to
-- the right block. Province-level map detail stays out of scope (R3).
CREATE TABLE territory_states (
    state_code     text PRIMARY KEY,
    territory_code text NOT NULL
);

INSERT INTO territory_states (state_code, territory_code) VALUES
    -- NE-1 — Northeast + Mid-Atlantic
    ('ME','NE-1'), ('NH','NE-1'), ('VT','NE-1'), ('MA','NE-1'), ('RI','NE-1'),
    ('CT','NE-1'), ('NY','NE-1'), ('NJ','NE-1'), ('PA','NE-1'), ('MD','NE-1'),
    ('DE','NE-1'), ('DC','NE-1'),
    -- SE-1 — Southeast
    ('VA','SE-1'), ('WV','SE-1'), ('NC','SE-1'), ('SC','SE-1'), ('GA','SE-1'),
    ('FL','SE-1'), ('AL','SE-1'), ('TN','SE-1'), ('KY','SE-1'), ('MS','SE-1'),
    -- MW-1 — Midwest
    ('OH','MW-1'), ('MI','MW-1'), ('IN','MW-1'), ('IL','MW-1'), ('WI','MW-1'),
    ('MN','MW-1'), ('IA','MW-1'), ('MO','MW-1'), ('ND','MW-1'), ('SD','MW-1'),
    ('NE','MW-1'), ('KS','MW-1'),
    -- SC-1 — West South Central
    ('TX','SC-1'), ('OK','SC-1'), ('AR','SC-1'), ('LA','SC-1'),
    -- MT-1 — Mountain
    ('MT','MT-1'), ('ID','MT-1'), ('WY','MT-1'), ('NV','MT-1'), ('UT','MT-1'),
    ('CO','MT-1'), ('AZ','MT-1'), ('NM','MT-1'),
    -- W-1 — Pacific
    ('WA','W-1'), ('OR','W-1'), ('CA','W-1'), ('AK','W-1'), ('HI','W-1'),
    -- Canada blocks (map shapes) + provinces/territories (site-state joins)
    ('CA-E','CE-1'), ('ON','CE-1'), ('QC','CE-1'), ('NB','CE-1'), ('NS','CE-1'),
    ('PE','CE-1'), ('NL','CE-1'),
    ('CA-W','CW-1'), ('BC','CW-1'), ('AB','CW-1'), ('SK','CW-1'), ('MB','CW-1'),
    ('YT','CW-1'), ('NT','CW-1'), ('NU','CW-1');

-- 0007's blanket grant predates this table. SELECT only — geography is
-- seed/admin-owned config; the app reads it and never writes it.
GRANT SELECT ON territory_states TO plenum_app;

-- ── signal_status gains 'expired' (R4) ──────────────────────────────────────
-- Additive enum value. Postgres 12+ allows ADD VALUE inside a transaction as
-- long as the NEW value is not evaluated in the same transaction — nothing
-- below evaluates it: the function body is stored text (parsed at first
-- execution), and the index predicate uses only pre-existing values.
ALTER TYPE signal_status ADD VALUE 'expired';

-- When the machine expired the card (NULL on every other status; cleared if
-- the generator reopens the card because its predicate holds again).
ALTER TABLE signals
    ADD COLUMN expired_at timestamptz;

-- ── the measured R5 index ────────────────────────────────────────────────────
-- PRE-4 EXPLAIN ANALYZE: v_unit_facts' last_paid lateral seq-scanned orders
-- per unit (Filter: site_id = iu.site_id, ~17k rows removed, ×232 units,
-- est. cost ≈ 8.6k/loop → plan cost ≈ 517k). That cost pushed every consumer
-- (the enriched signals list, all four generators, coverage) over the JIT
-- thresholds — the enriched active list spent 1,021 ms of its 1,187 ms in
-- JIT compilation alone. This index turns the lateral into an ordered index
-- scan, which collapses the plan cost below the JIT trigger and speeds the
-- lateral itself. (The other unit-prompt candidates were measured moot:
-- signals(status, type) has existed since 0003, and the signals table is
-- ~250 rows — no plan touches it expensively.)
CREATE INDEX idx_orders_site_ordered ON orders (site_id, ordered_on DESC, id DESC);

-- ── generate_signals(): the R4 auto-expiry step ─────────────────────────────
-- DROP + CREATE: the return table gains the per-type `expired` count (R4);
-- a return-type change cannot ride CREATE OR REPLACE. Grants are re-issued
-- below (they die with the DROP).
--
-- New semantics, per generator type, all in the same statement as the upsert
-- (same snapshot; the key sets are disjoint by construction):
--   · expiry — open rows of this type whose dedupe_key is NOT in this run's
--     emitted key-set flip to status='expired', expired_at=now(). Predicate
--     died ⇒ the machine takes the card back. assigned/actioned/dismissed
--     rows are NEVER touched (human-touched cards are human property), and
--     rows without a dedupe_key are never touched (machine-keyed rows only).
--   · reopen — if a previously-expired key is emitted again (the predicate
--     returned: e.g. telemetry dips, recovers, dips again), the upsert
--     revives it: status back to 'open', expired_at cleared, fresh
--     score/reasons. Machine gave, machine took, machine may give again;
--     reopens count in `updated`. dismissed/actioned still never resurrect.
--   · idempotency preserved — an immediate second run emits the same key
--     sets, so the expiry UPDATE matches zero rows and the upsert changes
--     zero rows: 0/0/0 per type, zero audit delta.
-- Every expiry/reopen is a real state change, so the 0006 trigger writes an
-- audit row for each — expected and correct.
DROP FUNCTION generate_signals();

CREATE FUNCTION generate_signals()
RETURNS TABLE (signal_type text, inserted bigint, updated bigint, expired bigint)
LANGUAGE plpgsql
AS $$
DECLARE
    ins  bigint;
    upd  bigint;
    expd bigint;
BEGIN
    -- ── 1 · reorder_due — the cadence window, plus the telemetry branch ─────
    WITH sp AS (SELECT * FROM signal_policy),
    cadence AS (
        SELECT u.unit_id, u.account_id, u.site_id,
               u.last_filter_order_on,
               u.expected_changeout_months            AS ecm,
               (u.last_filter_order_on
                + round(u.expected_changeout_months * 30.44)::int) AS due_date,
               (CURRENT_DATE - u.last_filter_order_on) AS days_silent,
               round(u.cartridge_count::numeric * u.cartridge_list_unit_cents * 12
                     / u.expected_changeout_months)::bigint AS annual_value_cents
        FROM v_unit_facts u
        WHERE u.cartridge_product_id IS NOT NULL
          AND u.expected_changeout_months IS NOT NULL
          AND u.last_filter_order_on IS NOT NULL
    ),
    src AS (
        SELECT c.account_id, c.site_id, c.unit_id,
               'reorder_due:' || c.unit_id || ':' || c.due_date AS dedupe_key,
               round((c.annual_value_cents / 100.0)
                     * (1 + GREATEST(0, CURRENT_DATE - c.due_date)
                            / (c.ecm * 30.44)), 2) AS score,
               jsonb_build_array(
                   jsonb_build_object('label', 'last order',
                       'weight', c.days_silent,
                       'detail', 'last cartridge order '
                                 || to_char(c.last_filter_order_on, 'YYYY-MM-DD')),
                   jsonb_build_object('label', 'cadence',
                       'weight', c.ecm,
                       'detail', 'every ' || c.ecm || ' months'),
                   jsonb_build_object('label', 'due',
                       'weight', (c.due_date - CURRENT_DATE),
                       'detail', CASE WHEN c.due_date >= CURRENT_DATE
                                 THEN 'due ' || to_char(c.due_date, 'YYYY-MM-DD')
                                 ELSE 'overdue ' || (CURRENT_DATE - c.due_date)
                                      || ' days (was due '
                                      || to_char(c.due_date, 'YYYY-MM-DD') || ')'
                                 END),
                   jsonb_build_object('label', 'annual value',
                       'weight', round(c.annual_value_cents / 100.0, 2),
                       'detail', '$' || to_char(round(c.annual_value_cents / 100.0),
                                                'FM999,999,999')
                                 || '/yr cartridge value')
               ) AS reasons
        FROM cadence c CROSS JOIN sp
        WHERE (c.due_date - CURRENT_DATE) <= sp.reorder_lookahead_days
          AND c.days_silent <= c.ecm * sp.defection_multiplier * 30.44
        UNION ALL
        -- v_unit_facts (0008) does not carry filter_life_pct; the join to
        -- installed_units below DECORATES the view's already-scoped row set
        -- with that one column (scope-by-parent-join, the 0008 doctrine —
        -- the row only exists if the view's RLS'd account join produced it).
        SELECT u.account_id, u.site_id, u.unit_id,
               'reorder_due:' || u.unit_id || ':telemetry' AS dedupe_key,
               round((round(u.cartridge_count::numeric
                            * COALESCE(u.cartridge_list_unit_cents, 0) * 12
                            / COALESCE(u.expected_changeout_months,
                                       sp.conquest_default_changeout_months))
                      / 100.0)
                     * (1 + (100 - iu.filter_life_pct) / 100.0), 2) AS score,
               jsonb_build_array(
                   jsonb_build_object('label', 'last order',
                       'weight', (CURRENT_DATE - u.last_filter_order_on),
                       'detail', CASE WHEN u.last_filter_order_on IS NULL
                                 THEN 'no cartridge order on file'
                                 ELSE 'last cartridge order '
                                      || to_char(u.last_filter_order_on, 'YYYY-MM-DD')
                                 END),
                   jsonb_build_object('label', 'cadence',
                       'weight', u.expected_changeout_months,
                       'detail', CASE WHEN u.expected_changeout_months IS NULL
                                 THEN 'cadence unknown (assumes '
                                      || sp.conquest_default_changeout_months
                                      || '-month change-out)'
                                 ELSE 'every ' || u.expected_changeout_months
                                      || ' months'
                                 END),
                   jsonb_build_object('label', 'filter life',
                       'weight', iu.filter_life_pct,
                       'detail', 'filter life ' || (iu.filter_life_pct::float8)
                                 || '% — telemetry'),
                   jsonb_build_object('label', 'annual value',
                       'weight', round(round(u.cartridge_count::numeric
                                       * COALESCE(u.cartridge_list_unit_cents, 0) * 12
                                       / COALESCE(u.expected_changeout_months,
                                                  sp.conquest_default_changeout_months))
                                       / 100.0, 2),
                       'detail', '$' || to_char(round(round(u.cartridge_count::numeric
                                          * COALESCE(u.cartridge_list_unit_cents, 0) * 12
                                          / COALESCE(u.expected_changeout_months,
                                                     sp.conquest_default_changeout_months))
                                          / 100.0), 'FM999,999,999')
                                 || '/yr cartridge value')
               ) AS reasons
        FROM v_unit_facts u
        JOIN installed_units iu ON iu.id = u.unit_id
        CROSS JOIN sp
        WHERE iu.filter_life_pct IS NOT NULL
          AND iu.filter_life_pct <= sp.telemetry_low_pct
    ),
    up AS (
        INSERT INTO signals (type, account_id, site_id, installed_unit_id,
                             score, reasons, dedupe_key)
        SELECT 'reorder_due'::signal_type, s.account_id, s.site_id, s.unit_id,
               s.score, s.reasons, s.dedupe_key
        FROM src s
        ON CONFLICT (dedupe_key) DO UPDATE
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons,
                status = 'open', expired_at = NULL
            WHERE (signals.status = 'open'
                   AND (signals.score IS DISTINCT FROM EXCLUDED.score
                        OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons))
               OR signals.status = 'expired'
        RETURNING (xmax = 0) AS is_insert
    ),
    exp AS (
        UPDATE signals s SET status = 'expired', expired_at = now()
        WHERE s.status = 'open'
          AND s.type = 'reorder_due'
          AND s.dedupe_key IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM src WHERE src.dedupe_key = s.dedupe_key)
        RETURNING 1
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert),
           (SELECT count(*) FROM exp)
    INTO ins, upd, expd FROM up;
    signal_type := 'reorder_due'; inserted := ins; updated := upd; expired := expd;
    RETURN NEXT;

    -- ── 2 · defection_risk — metric 7, verbatim from the view ───────────────
    WITH src AS (
        SELECT d.account_id, d.site_id, d.unit_id,
               'defection_risk:' || d.unit_id || ':'
                   || ((CURRENT_DATE - d.days_silent)
                       + round(d.expected_changeout_months * 30.44)::int) AS dedupe_key,
               d.score,
               jsonb_build_array(
                   jsonb_build_object('label', 'last order',
                       'weight', d.days_silent,
                       'detail', 'last cartridge order '
                                 || to_char(CURRENT_DATE - d.days_silent, 'YYYY-MM-DD')),
                   jsonb_build_object('label', 'silence',
                       'weight', round(d.days_silent
                                       / (d.expected_changeout_months * 30.44), 1),
                       'detail', d.days_silent || ' days silent ≈ '
                                 || round(d.days_silent
                                          / (d.expected_changeout_months * 30.44), 1)
                                 || ' change-out cycles missed'),
                   jsonb_build_object('label', 'cadence',
                       'weight', d.expected_changeout_months,
                       'detail', 'expected every ' || d.expected_changeout_months
                                 || ' months'),
                   jsonb_build_object('label', 'annual value',
                       'weight', round(d.annual_consumable_value_cents / 100.0, 2),
                       'detail', '$' || to_char(round(d.annual_consumable_value_cents / 100.0),
                                                'FM999,999,999')
                                 || '/yr at stake')
               ) AS reasons
        FROM v_defection_risk d
    ),
    up AS (
        INSERT INTO signals (type, account_id, site_id, installed_unit_id,
                             score, reasons, dedupe_key)
        SELECT 'defection_risk'::signal_type, s.account_id, s.site_id, s.unit_id,
               s.score, s.reasons, s.dedupe_key
        FROM src s
        ON CONFLICT (dedupe_key) DO UPDATE
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons,
                status = 'open', expired_at = NULL
            WHERE (signals.status = 'open'
                   AND (signals.score IS DISTINCT FROM EXCLUDED.score
                        OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons))
               OR signals.status = 'expired'
        RETURNING (xmax = 0) AS is_insert
    ),
    exp AS (
        UPDATE signals s SET status = 'expired', expired_at = now()
        WHERE s.status = 'open'
          AND s.type = 'defection_risk'
          AND s.dedupe_key IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM src WHERE src.dedupe_key = s.dedupe_key)
        RETURNING 1
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert),
           (SELECT count(*) FROM exp)
    INTO ins, upd, expd FROM up;
    signal_type := 'defection_risk'; inserted := ins; updated := upd; expired := expd;
    RETURN NEXT;

    -- ── 3 · conquest — competitor units × the filter_fits cross-reference ───
    WITH sp AS (SELECT * FROM signal_policy),
    src AS (
        SELECT u.account_id, u.site_id, u.unit_id,
               'conquest:' || u.unit_id AS dedupe_key,
               round(round(u.cartridge_count::numeric * bf.list_price_cents * 12
                           / COALESCE(u.expected_changeout_months,
                                      sp.conquest_default_changeout_months))
                     / 100.0, 2) AS score,
               jsonb_build_array(
                   jsonb_build_object('label', 'competitor unit',
                       'weight', u.cartridge_count,
                       'detail', u.source || ' ' || u.unit_family || ' · '
                                 || u.cartridge_count || ' cartridges'),
                   jsonb_build_object('label', 'our fit',
                       'weight', bf.n_fits,
                       'detail', bf.sku || ' fits (' || bf.n_fits
                                 || ' compatible SKU'
                                 || CASE WHEN bf.n_fits = 1 THEN '' ELSE 's' END
                                 || ')'),
                   jsonb_build_object('label', 'annual value',
                       'weight', round(round(u.cartridge_count::numeric
                                             * bf.list_price_cents * 12
                                             / COALESCE(u.expected_changeout_months,
                                                        sp.conquest_default_changeout_months))
                                       / 100.0, 2),
                       'detail', 'est. $'
                                 || to_char(round(round(u.cartridge_count::numeric
                                               * bf.list_price_cents * 12
                                               / COALESCE(u.expected_changeout_months,
                                                          sp.conquest_default_changeout_months))
                                               / 100.0), 'FM999,999,999')
                                 || '/yr filter value'
                                 || CASE WHEN u.expected_changeout_months IS NULL
                                    THEN ' (assumes '
                                         || sp.conquest_default_changeout_months
                                         || '-month change-out)'
                                    ELSE '' END)
               ) AS reasons
        FROM v_unit_facts u
        CROSS JOIN sp
        JOIN LATERAL (
            SELECT p.sku, p.list_price_cents,
                   (SELECT count(*) FROM products p2
                    WHERE p2.kind = 'consumable'
                      AND p2.filter_fits @> ARRAY[u.unit_family]) AS n_fits
            FROM products p
            WHERE p.kind = 'consumable'
              AND p.filter_fits @> ARRAY[u.unit_family]
            ORDER BY p.list_price_cents DESC, p.sku ASC
            LIMIT 1
        ) bf ON true
        WHERE u.source <> 'ours'
          AND u.last_filter_order_on IS NULL
    ),
    up AS (
        INSERT INTO signals (type, account_id, site_id, installed_unit_id,
                             score, reasons, dedupe_key)
        SELECT 'conquest'::signal_type, s.account_id, s.site_id, s.unit_id,
               s.score, s.reasons, s.dedupe_key
        FROM src s
        ON CONFLICT (dedupe_key) DO UPDATE
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons,
                status = 'open', expired_at = NULL
            WHERE (signals.status = 'open'
                   AND (signals.score IS DISTINCT FROM EXCLUDED.score
                        OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons))
               OR signals.status = 'expired'
        RETURNING (xmax = 0) AS is_insert
    ),
    exp AS (
        UPDATE signals s SET status = 'expired', expired_at = now()
        WHERE s.status = 'open'
          AND s.type = 'conquest'
          AND s.dedupe_key IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM src WHERE src.dedupe_key = s.dedupe_key)
        RETURNING 1
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert),
           (SELECT count(*) FROM exp)
    INTO ins, upd, expd FROM up;
    signal_type := 'conquest'; inserted := ins; updated := upd; expired := expd;
    RETURN NEXT;

    -- ── 4 · discount_anomaly — order-line statistics (spec §5.5 feed) ───────
    WITH sp AS (SELECT * FROM signal_policy),
    fam_stats AS (
        SELECT family,
               percentile_cont(0.5) WITHIN GROUP
                   (ORDER BY discount_pct::float8) AS median_pct,
               stddev_pop(discount_pct::float8)    AS sd
        FROM v_order_facts
        GROUP BY family
    ),
    src AS (
        SELECT f.account_id, f.site_id, f.order_line_id,
               'discount_anomaly:' || f.order_line_id AS dedupe_key,
               round((f.gross_cents::numeric
                      * (f.discount_pct::numeric - s.median_pct::numeric)
                      / 100 / 100), 2) AS score,
               jsonb_build_array(
                   jsonb_build_object('label', 'discount vs median',
                       'weight', f.discount_pct,
                       'detail', (f.discount_pct::float8) || '% vs '
                                 || round(s.median_pct::numeric, 1)
                                 || '% family median (+'
                                 || round(((f.discount_pct::float8 - s.median_pct)
                                           / s.sd)::numeric, 1) || 'σ)'),
                   jsonb_build_object('label', 'line',
                       'weight', round(f.gross_cents / 100.0, 2),
                       'detail', f.product_sku || ' × ' || f.qty || ' — $'
                                 || to_char(round(f.gross_cents / 100.0),
                                            'FM999,999,999') || ' gross'),
                   jsonb_build_object('label', 'order',
                       'weight', (CURRENT_DATE - f.ordered_on),
                       'detail', to_char(f.ordered_on, 'YYYY-MM-DD') || ' · '
                                 || f.rep_name)
               ) AS reasons
        FROM v_order_facts f
        JOIN fam_stats s ON s.family = f.family
        CROSS JOIN sp
        WHERE f.ordered_on >= CURRENT_DATE - sp.discount_window_days
          AND s.sd > 0
          AND f.discount_pct::float8 > s.median_pct + sp.discount_sigma * s.sd
    ),
    up AS (
        INSERT INTO signals (type, account_id, site_id, order_line_id,
                             score, reasons, dedupe_key)
        SELECT 'discount_anomaly'::signal_type, s.account_id, s.site_id,
               s.order_line_id, s.score, s.reasons, s.dedupe_key
        FROM src s
        ON CONFLICT (dedupe_key) DO UPDATE
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons,
                status = 'open', expired_at = NULL
            WHERE (signals.status = 'open'
                   AND (signals.score IS DISTINCT FROM EXCLUDED.score
                        OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons))
               OR signals.status = 'expired'
        RETURNING (xmax = 0) AS is_insert
    ),
    exp AS (
        UPDATE signals s SET status = 'expired', expired_at = now()
        WHERE s.status = 'open'
          AND s.type = 'discount_anomaly'
          AND s.dedupe_key IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM src WHERE src.dedupe_key = s.dedupe_key)
        RETURNING 1
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert),
           (SELECT count(*) FROM exp)
    INTO ins, upd, expd FROM up;
    signal_type := 'discount_anomaly'; inserted := ins; updated := upd; expired := expd;
    RETURN NEXT;
END;
$$;

-- Functions are EXECUTE-able by PUBLIC by default; the DROP above also
-- discarded the 0012 grants. Same posture as before: revoked from PUBLIC,
-- granted to plenum_app; the HANDLER gates on session role = admin.
REVOKE ALL ON FUNCTION generate_signals() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION generate_signals() TO plenum_app;
