-- P1 derived layer, part 3: scoped read views over the rollups, the
-- defection-risk view, and the refresh function.
--
-- ── Why the scoped views are DEFINER views (no security_invoker) ──────────
-- plenum_app has NO grant on any raw mv_* (0009) — that is the enforcement
-- boundary for matview data. A security_invoker view would check plenum_app
-- against the matview and fail. These four views therefore run with their
-- owner's (plenum_admin's) privileges, and the scoping is the EXPLICIT
-- predicate below — the 0005 fail-closed predicate verbatim:
--     territory_id IN (SELECT territory_id FROM v_user_scope
--                      WHERE user_id = NULLIF(current_setting('app.user_id',
--                                             true), '')::uuid)
-- current_setting reads the transaction-local GUC set by the API's rls_tx
-- helper; it resolves identically inside a definer view. No GUC -> NULL
-- user_id -> no v_user_scope row -> ZERO rows from BOTH branches: fail closed.
--
-- The predicate appears on BOTH branches deliberately. Inside a definer view
-- the live branch's read of v_order_facts (security_invoker) is checked as
-- plenum_admin — superuser, so RLS does NOT constrain it. Without the
-- explicit predicate the live branch would leak every territory's current
-- quarter. With it, both branches are scoped by the same territory set the
-- 0005 policies use.
--
-- ── The boundary pair ─────────────────────────────────────────────────────
-- matview branch:  quarter_start <  date_trunc('quarter', now())::date
-- live branch:     quarter_start >= date_trunc('quarter', now())::date
-- Half-open on the same instant: no gap, no double-count, even if the
-- matview was refreshed mid-quarter. "Today's order moves today's number"
-- (spec §4) — the current quarter is ALWAYS computed live from v_order_facts.

CREATE VIEW v_territory_period AS
SELECT territory_id, territory_code, territory_name, quota_year_cents,
       quarter_start, gross_cents, net_cents, discount_cents, order_count
FROM mv_territory_period
WHERE quarter_start < date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
UNION ALL
SELECT territory_id, territory_code, territory_name, quota_year_cents,
       quarter_start,
       SUM(gross_cents)::bigint, SUM(net_cents)::bigint,
       SUM(discount_cents)::bigint, count(DISTINCT order_id)
FROM v_order_facts
WHERE quarter_start >= date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
GROUP BY territory_id, territory_code, territory_name, quota_year_cents,
         quarter_start;

CREATE VIEW v_rep_period AS
SELECT rep_id, rep_name, territory_id, quarter_start,
       gross_cents, net_cents, discount_cents, order_count,
       capital_gross_cents, capital_net_cents,
       consumable_gross_cents, consumable_net_cents
FROM mv_rep_period
WHERE quarter_start < date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
UNION ALL
SELECT rep_id, rep_name, territory_id, quarter_start,
       SUM(gross_cents)::bigint, SUM(net_cents)::bigint,
       SUM(discount_cents)::bigint, count(DISTINCT order_id),
       COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint,
       COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint,
       COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint,
       COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint
FROM v_order_facts
WHERE quarter_start >= date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
GROUP BY rep_id, rep_name, territory_id, quarter_start;

CREATE VIEW v_product_period AS
SELECT product_id, product_sku, product_name, family, kind, territory_id,
       quarter_start, units, gross_cents, net_cents, discount_cents,
       order_count
FROM mv_product_period
WHERE quarter_start < date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
UNION ALL
SELECT product_id, product_sku, product_name, family, kind, territory_id,
       quarter_start,
       SUM(qty)::bigint, SUM(gross_cents)::bigint, SUM(net_cents)::bigint,
       SUM(discount_cents)::bigint, count(DISTINCT order_id)
FROM v_order_facts
WHERE quarter_start >= date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
GROUP BY product_id, product_sku, product_name, family, kind, territory_id,
         quarter_start;

CREATE VIEW v_customer_period AS
SELECT account_id, account_name, territory_id, quarter_start,
       gross_cents, net_cents, discount_cents, order_count,
       capital_gross_cents, capital_net_cents,
       consumable_gross_cents, consumable_net_cents
FROM mv_customer_period
WHERE quarter_start < date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
UNION ALL
SELECT account_id, account_name, territory_id, quarter_start,
       SUM(gross_cents)::bigint, SUM(net_cents)::bigint,
       SUM(discount_cents)::bigint, count(DISTINCT order_id),
       COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint,
       COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint,
       COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint,
       COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint
FROM v_order_facts
WHERE quarter_start >= date_trunc('quarter', now())::date
  AND territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
GROUP BY account_id, account_name, territory_id, quarter_start;

-- ── Defection risk (metric 7; P4's signal generator reuses this view) ─────
-- security_invoker: this chains to v_unit_facts and must be read with the
-- caller's privileges so the 0005 policies scope it (definer would bypass
-- RLS — plenum_admin is superuser).
--
-- Spec §5.7 verbatim: at risk when
--     today − last_filter_order_on > expected_changeout_months × 1.5
-- The left side is days; months convert at 30.44 days/month (the mean
-- Gregorian month), the same factor the score's gap term uses — one
-- conversion, used consistently.
--
-- Excluded on purpose:
--   · the two seeded NULL expected_changeout_months units — they are
--     data-quality material for the P5 panel, not defection rows (cadence
--     math cannot run without a cadence);
--   · units with no cartridge product or no filter-order history (e.g. the
--     Alpenglow competitor units) — those are conquest fuel (P4), not
--     defection: you cannot defect from a relationship that never existed.
--
-- score = gap_size × annual consumable value, where gap_size is the silence
-- measured in expected change-out cycles (days_silent / (ecm × 30.44 days))
-- and the value term is annual_consumable_value_cents / 100 (dollars) to
-- keep the score in a rankable range. Deterministic given the data.
CREATE VIEW v_defection_risk WITH (security_invoker = true) AS
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
    WHERE cartridge_product_id IS NOT NULL
      AND expected_changeout_months IS NOT NULL
      AND last_filter_order_on IS NOT NULL
      AND CURRENT_DATE - last_filter_order_on
          > (expected_changeout_months * 1.5) * 30.44
) at_risk;

-- ── refresh_rollups() ─────────────────────────────────────────────────────
-- SECURITY DEFINER so the API can refresh WITHOUT an admin connection (the
-- P0 doctrine stands: the API connects only as plenum_app). The function
-- runs as its owner (plenum_admin), refreshes all four matviews
-- non-concurrently, and returns per-matview row counts — plenum_app cannot
-- count the matviews itself (no grant), so the counts come back through the
-- definer boundary. search_path is pinned (SECURITY DEFINER hygiene).
-- The HANDLER gates on session role = admin BEFORE calling this; the grant
-- to plenum_app is what makes the call possible at all.
CREATE FUNCTION refresh_rollups()
RETURNS TABLE (matview text, row_count bigint)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    REFRESH MATERIALIZED VIEW mv_territory_period;
    REFRESH MATERIALIZED VIEW mv_rep_period;
    REFRESH MATERIALIZED VIEW mv_product_period;
    REFRESH MATERIALIZED VIEW mv_customer_period;
    RETURN QUERY
        SELECT 'mv_territory_period'::text, count(*) FROM mv_territory_period
        UNION ALL
        SELECT 'mv_rep_period'::text,       count(*) FROM mv_rep_period
        UNION ALL
        SELECT 'mv_product_period'::text,   count(*) FROM mv_product_period
        UNION ALL
        SELECT 'mv_customer_period'::text,  count(*) FROM mv_customer_period;
END;
$$;

-- Functions are EXECUTE-able by PUBLIC by default; a definer function that
-- refreshes as superuser must not be. Revoke, then grant to plenum_app only.
REVOKE ALL ON FUNCTION refresh_rollups() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION refresh_rollups() TO plenum_app;

-- Read grants for the scoped views and the defection view (0007's blanket
-- grant does not cover post-0007 objects).
GRANT SELECT ON v_territory_period, v_rep_period, v_product_period,
               v_customer_period, v_defection_risk TO plenum_app;
