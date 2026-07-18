-- P1 derived layer, part 2: materialized rollups.
--
-- Postgres cannot enforce RLS on a materialized view, and REFRESH runs the
-- defining query as the matview owner (plenum_admin, superuser) — so every
-- mv_* below holds EVERY territory's rows by design. The enforcement is the
-- grant boundary: plenum_app receives NO grant on any mv_* (0007's
-- GRANT ... ON ALL TABLES predates these objects and does not cover them,
-- and no ALTER DEFAULT PRIVILEGES exists). The only path from the API to
-- rollup data is the scoped read views in 0010, which carry the 0005
-- v_user_scope predicate. The adversarial matrix proves the denial.
--
-- Every matview is keyed (entity, territory_id, quarter_start) — territory_id
-- in every key so one uniform scope predicate covers all four. Money columns
-- are BIGINT cents; order_count is count(DISTINCT order_id), additive across
-- quarters and territories because an order has exactly one ordered_on and
-- one territory.
--
-- Created WITH NO DATA; populated by the seed's post-load refresh_rollups()
-- call and on demand via POST /api/admin/refresh-rollups.

CREATE MATERIALIZED VIEW mv_territory_period AS
SELECT
    territory_id,
    territory_code,
    territory_name,
    quota_year_cents,
    quarter_start,
    SUM(gross_cents)::bigint    AS gross_cents,
    SUM(net_cents)::bigint      AS net_cents,
    SUM(discount_cents)::bigint AS discount_cents,
    count(DISTINCT order_id)    AS order_count
FROM v_order_facts
GROUP BY territory_id, territory_code, territory_name, quota_year_cents, quarter_start
WITH NO DATA;

CREATE UNIQUE INDEX mv_territory_period_key
    ON mv_territory_period (territory_id, quarter_start);

CREATE MATERIALIZED VIEW mv_rep_period AS
SELECT
    rep_id,
    rep_name,
    territory_id,
    quarter_start,
    SUM(gross_cents)::bigint    AS gross_cents,
    SUM(net_cents)::bigint      AS net_cents,
    SUM(discount_cents)::bigint AS discount_cents,
    count(DISTINCT order_id)    AS order_count,
    COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint    AS capital_gross_cents,
    COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint    AS capital_net_cents,
    COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_gross_cents,
    COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_net_cents
FROM v_order_facts
GROUP BY rep_id, rep_name, territory_id, quarter_start
WITH NO DATA;

CREATE UNIQUE INDEX mv_rep_period_key
    ON mv_rep_period (rep_id, territory_id, quarter_start);

CREATE MATERIALIZED VIEW mv_product_period AS
SELECT
    product_id,
    product_sku,
    product_name,
    family,
    kind,
    territory_id,
    quarter_start,
    SUM(qty)::bigint            AS units,
    SUM(gross_cents)::bigint    AS gross_cents,
    SUM(net_cents)::bigint      AS net_cents,
    SUM(discount_cents)::bigint AS discount_cents,
    count(DISTINCT order_id)    AS order_count
FROM v_order_facts
GROUP BY product_id, product_sku, product_name, family, kind, territory_id, quarter_start
WITH NO DATA;

CREATE UNIQUE INDEX mv_product_period_key
    ON mv_product_period (product_id, territory_id, quarter_start);

CREATE MATERIALIZED VIEW mv_customer_period AS
SELECT
    account_id,
    account_name,
    territory_id,
    quarter_start,
    SUM(gross_cents)::bigint    AS gross_cents,
    SUM(net_cents)::bigint      AS net_cents,
    SUM(discount_cents)::bigint AS discount_cents,
    count(DISTINCT order_id)    AS order_count,
    COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint    AS capital_gross_cents,
    COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint    AS capital_net_cents,
    COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_gross_cents,
    COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_net_cents
FROM v_order_facts
GROUP BY account_id, account_name, territory_id, quarter_start
WITH NO DATA;

CREATE UNIQUE INDEX mv_customer_period_key
    ON mv_customer_period (account_id, territory_id, quarter_start);

-- Deliberately NO grants to plenum_app on any mv_* — that absence is the
-- security boundary, and the adversarial matrix demonstrates it failing
-- closed (permission denied).
