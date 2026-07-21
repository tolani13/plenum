-- P4 Signals + AI, schema layer (constraint 6 — ADDITIVE ONLY).
--
-- Nothing here is destructive: two nullable columns + one unique index on
-- signals, one new config table (seeded here, like discount_policy), one
-- invoker-rights generator function, and ONE CREATE OR REPLACE VIEW that
-- preserves v_defection_risk's column list and — at the seeded default
-- multiplier 1.50 — its output byte-for-byte. No column dropped, no type
-- rewritten, no data touched. sqlx migrate runs this file in one transaction.

-- ── signals: deterministic identity + the anomaly line reference (R2/R4) ────
-- dedupe_key is the generator's stable identity for a signal:
--   reorder_due:<unit_id>:<due_date>       (cadence window; due_date =
--                                           last_filter_order_on
--                                           + round(ecm × 30.44) days)
--   reorder_due:<unit_id>:telemetry        (telemetry-triggered; one live/unit)
--   defection_risk:<unit_id>:<due_date>    (same window basis)
--   conquest:<unit_id>                     (no window — one per competitor unit)
--   discount_anomaly:<order_line_id>
-- Nullable (only generate_signals() writes it); the UNIQUE index is what the
-- upsert's ON CONFLICT targets, so a rerun can never duplicate a signal.
ALTER TABLE signals
    ADD COLUMN dedupe_key    text,
    ADD COLUMN order_line_id uuid REFERENCES order_lines (id);

CREATE UNIQUE INDEX signals_dedupe_key ON signals (dedupe_key);

-- ── signal_policy: generator thresholds become seed-config (R3) ─────────────
-- The discount_policy pattern exactly: boolean-true PK singleton, CHECKed,
-- seeded IN the migration (NOT in the seed truncate list — survives a reseed),
-- SELECT-only grant to plenum_app. Generators read these per run.
CREATE TABLE signal_policy (
    id                               boolean PRIMARY KEY DEFAULT true,
    -- The defection boundary multiplier (spec §5.7's 1.5×). Also drives the
    -- reorder lane's upper edge — the two lanes partition at this boundary.
    defection_multiplier             numeric(4, 2) NOT NULL CHECK (defection_multiplier > 0),
    -- Discount-anomaly threshold: lines > median + sigma × stddev (spec §5.5's 2σ).
    discount_sigma                   numeric(4, 2) NOT NULL CHECK (discount_sigma > 0),
    -- How far ahead of a unit's due date the reorder radar starts flagging.
    reorder_lookahead_days           int NOT NULL CHECK (reorder_lookahead_days >= 0),
    -- Anomaly feed recency window (PRE-5 laddered 90 → 180 → 365; 90 had rows).
    discount_window_days             int NOT NULL CHECK (discount_window_days > 0),
    -- Cadence fallback used ONLY where a competitor/telemetry unit lacks a
    -- stated expected_changeout_months.
    conquest_default_changeout_months int NOT NULL CHECK (conquest_default_changeout_months > 0),
    -- Telemetry trigger: filter_life_pct at or below this fires a reorder card.
    telemetry_low_pct                numeric(5, 2) NOT NULL CHECK (telemetry_low_pct >= 0 AND telemetry_low_pct <= 100),
    CONSTRAINT signal_policy_singleton CHECK (id)
);

INSERT INTO signal_policy
    (id, defection_multiplier, discount_sigma, reorder_lookahead_days,
     discount_window_days, conquest_default_changeout_months, telemetry_low_pct)
VALUES (true, 1.50, 2.00, 30, 90, 12, 20.00);

-- 0007's blanket grant predates this table. SELECT only — config is
-- seed/admin-owned; the app (and the security_invoker view below) reads it.
GRANT SELECT ON signal_policy TO plenum_app;

-- ── v_defection_risk re-created over the config row (R3) ────────────────────
-- IDENTICAL column list and semantics to 0010's original; the ONLY change is
-- the literal 1.5 becoming signal_policy.defection_multiplier (a single-row
-- CROSS JOIN). At the seeded default 1.50 the output is byte-identical, so
-- P1's metrics tests and GET /api/metrics/defection are untouched.
-- security_invoker restated explicitly (CREATE OR REPLACE sets the options
-- given here): callers are checked against the RLS tables underneath, exactly
-- as before — and against signal_policy, which plenum_app may SELECT (above).
CREATE OR REPLACE VIEW v_defection_risk WITH (security_invoker = true) AS
SELECT unit_id,
       serial,
       site_id,
       site_label,
       account_id,
       account_name,
       territory_id,
       territory_code,
       days_silent,
       expected_changeout_months,
       annual_consumable_value_cents,
       round(
           (days_silent::numeric / (expected_changeout_months * 30.44))
           * (annual_consumable_value_cents::numeric / 100),
           2
       ) AS score
FROM (
    SELECT unit_id, serial, site_id, site_label, account_id, account_name,
           territory_id, territory_code,
           (CURRENT_DATE - last_filter_order_on)  AS days_silent,
           expected_changeout_months,
           round(cartridge_count::numeric * cartridge_list_unit_cents * 12
                 / expected_changeout_months)::bigint
                                                  AS annual_consumable_value_cents
    FROM v_unit_facts
    CROSS JOIN signal_policy sp
    WHERE cartridge_product_id IS NOT NULL
      AND expected_changeout_months IS NOT NULL
      AND last_filter_order_on IS NOT NULL
      AND CURRENT_DATE - last_filter_order_on
          > (expected_changeout_months * sp.defection_multiplier) * 30.44
) at_risk;

-- ── generate_signals(): the idempotent, re-runnable derivation job (R2/R4) ──
--
-- The refresh_rollups() shape MINUS SECURITY DEFINER — invoker rights on
-- purpose: the admin endpoint runs it as plenum_app under an admin session
-- whose v_user_scope is every territory (RLS satisfied on both read and
-- write); the seed runs it as plenum_admin (superuser, RLS bypassed). Nothing
-- here needs definer rights, so it gets none.
--
-- All four generators derive ONLY from table data (R1): cadence math over
-- v_unit_facts, v_defection_risk verbatim, the filter_fits cross-reference,
-- and order-line statistics. No account name, no seed constant, no story-beat
-- knowledge appears anywhere below — the Ridgeline card must EMERGE.
--
-- Upsert semantics (R2): INSERT … ON CONFLICT (dedupe_key) DO UPDATE
--   SET score/reasons WHERE status = 'open' AND something actually changed.
-- A rerun therefore never duplicates, never touches assigned/actioned/
-- dismissed rows (no resurrection), and produces ZERO update rows — hence
-- zero audit noise from the 0006 trigger — when nothing changed (same-day
-- rerun). Signals whose predicate later stops holding are NOT auto-expired
-- (parked to P5 by ruling).
--
-- reasons[] weights are the raw numeric term each label describes —
-- days (last order / due / order age), months (cadence), dollars (values,
-- line gross), pct (discount, filter life), cycles or SKU-count where the
-- label says so. The UI renders label + detail; weight is the sortable
-- receipt behind the words.
CREATE FUNCTION generate_signals()
RETURNS TABLE (signal_type text, inserted bigint, updated bigint)
LANGUAGE plpgsql
AS $$
DECLARE
    ins bigint;
    upd bigint;
BEGIN
    -- ── 1 · reorder_due — the cadence window, plus the telemetry branch ─────
    -- Cadence: due or overdue inside the lookahead, but still under the
    -- defection boundary (the two lanes partition cleanly at
    -- ecm × defection_multiplier × 30.44 days of silence).
    -- Telemetry: filter_life_pct at/below the threshold, regardless of
    -- cadence fields — dead code until a writer exists (R13), by design.
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
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons
            WHERE signals.status = 'open'
              AND (signals.score IS DISTINCT FROM EXCLUDED.score
                   OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons)
        RETURNING (xmax = 0) AS is_insert
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert)
    INTO ins, upd FROM up;
    signal_type := 'reorder_due'; inserted := ins; updated := upd;
    RETURN NEXT;

    -- ── 2 · defection_risk — metric 7, verbatim from the view ───────────────
    -- The view IS the generator's predicate, boundary, exclusions, and score.
    -- due_date for the dedupe key is reconstructed from the view's own columns
    -- (last order = CURRENT_DATE − days_silent), so the key shares the reorder
    -- lane's window basis and stays stable across days.
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
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons
            WHERE signals.status = 'open'
              AND (signals.score IS DISTINCT FROM EXCLUDED.score
                   OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons)
        RETURNING (xmax = 0) AS is_insert
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert)
    INTO ins, upd FROM up;
    signal_type := 'defection_risk'; inserted := ins; updated := upd;
    RETURN NEXT;

    -- ── 3 · conquest — competitor units × the filter_fits cross-reference ───
    -- source <> 'ours' AND no filter-order history (0010's own partition: no
    -- relationship to defect from), with at least one consumable that fits the
    -- unit's family. Best fit = highest list price, tie-break sku ASC
    -- (deterministic). ecm falls back to the config default ONLY here (and in
    -- the telemetry branch above).
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
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons
            WHERE signals.status = 'open'
              AND (signals.score IS DISTINCT FROM EXCLUDED.score
                   OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons)
        RETURNING (xmax = 0) AS is_insert
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert)
    INTO ins, upd FROM up;
    signal_type := 'conquest'; inserted := ins; updated := upd;
    RETURN NEXT;

    -- ── 4 · discount_anomaly — order-line statistics (spec §5.5 feed) ───────
    -- Family stats over ALL order history (median via percentile_cont(0.5),
    -- σ via stddev_pop — the ruling's exact functions); candidates = lines in
    -- the trailing discount_window_days whose discount sits more than
    -- discount_sigma × σ above the family median. Score = the excess-leakage
    -- dollars on the line. Families with zero spread (σ = 0) produce nothing.
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
            SET score = EXCLUDED.score, reasons = EXCLUDED.reasons
            WHERE signals.status = 'open'
              AND (signals.score IS DISTINCT FROM EXCLUDED.score
                   OR signals.reasons IS DISTINCT FROM EXCLUDED.reasons)
        RETURNING (xmax = 0) AS is_insert
    )
    SELECT count(*) FILTER (WHERE is_insert),
           count(*) FILTER (WHERE NOT is_insert)
    INTO ins, upd FROM up;
    signal_type := 'discount_anomaly'; inserted := ins; updated := upd;
    RETURN NEXT;
END;
$$;

-- Functions are EXECUTE-able by PUBLIC by default. This one writes signals for
-- whatever the CALLER may see (invoker rights), so it is gated exactly like
-- refresh_rollups(): revoked from PUBLIC, granted to plenum_app; the HANDLER
-- gates on session role = admin before calling it.
REVOKE ALL ON FUNCTION generate_signals() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION generate_signals() TO plenum_app;
