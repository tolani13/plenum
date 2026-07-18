-- P1 derived layer, part 1: the two facts views every metric reads from.
--
-- BOTH views are WITH (security_invoker = true). This is not optional:
-- plenum_admin (the migration runner and view owner) is the cluster
-- superuser, and a default (security-definer) view is queried with the
-- OWNER's privileges — superuser bypasses RLS, so a definer view over the
-- RLS tables would hand every territory's rows to any caller. With
-- security_invoker, the API's plenum_app connection is the one checked, and
-- the 0005 policies apply to every read through these views.
--
-- order_lines / sites / installed_units / products carry NO RLS (spec §4
-- lists exactly six RLS tables). Their visibility derives from the INNER
-- JOIN to an RLS table here — orders for order_lines, accounts for sites and
-- installed_units. No metrics handler reads those tables raw.

-- v_order_facts: one row per order line, denormalized to everything the
-- metrics dictionary needs. Money stays BIGINT cents (integer × integer,
-- cast back to bigint). quarter_start/year are calendar (Q1 = Jan–Mar).
CREATE VIEW v_order_facts WITH (security_invoker = true) AS
SELECT
    ol.id                                           AS order_line_id,
    o.id                                            AS order_id,
    o.account_id,
    a.name                                          AS account_name,
    o.site_id,
    o.territory_id,
    t.code                                          AS territory_code,
    t.name                                          AS territory_name,
    t.quota_year_cents,
    o.rep_id,
    u.name                                          AS rep_name,
    ol.product_id,
    p.sku                                           AS product_sku,
    p.name                                          AS product_name,
    p.family,
    p.kind,
    ol.qty,
    ol.list_unit_cents,
    ol.net_unit_cents,
    ol.discount_pct,
    (ol.list_unit_cents * ol.qty)::bigint           AS gross_cents,
    (ol.net_unit_cents * ol.qty)::bigint            AS net_cents,
    ((ol.list_unit_cents - ol.net_unit_cents) * ol.qty)::bigint AS discount_cents,
    o.ordered_on,
    (date_trunc('quarter', o.ordered_on))::date     AS quarter_start,
    (EXTRACT(year FROM o.ordered_on))::int          AS year
FROM order_lines ol
JOIN orders      o ON o.id = ol.order_id      -- RLS: orders
JOIN accounts    a ON a.id = o.account_id     -- RLS: accounts (same territory scope)
JOIN territories t ON t.id = o.territory_id
JOIN users       u ON u.id = o.rep_id
JOIN products    p ON p.id = ol.product_id;

-- v_unit_facts: installed units joined site -> account (RLS applies) ->
-- territory, with the cartridge product and its list/net unit price.
-- Coverage, defection, and attach-rate read here.
--
-- cartridge_net_unit_cents: the most recent net unit price this site
-- actually paid for the unit's cartridge (the lateral over the RLS-scoped
-- orders), falling back to list when there is no order history. That makes
-- the "net" projection an evidence-based number, not a guess.
CREATE VIEW v_unit_facts WITH (security_invoker = true) AS
SELECT
    iu.id                                           AS unit_id,
    iu.serial,
    s.id                                            AS site_id,
    (s.city || ', ' || s.state)                     AS site_label,
    a.id                                            AS account_id,
    a.name                                          AS account_name,
    a.territory_id,
    t.code                                          AS territory_code,
    t.name                                          AS territory_name,
    up.family                                       AS unit_family,
    iu.source,
    iu.commissioned_on,
    iu.cartridge_count,
    iu.cartridge_product_id,
    cp.sku                                          AS cartridge_sku,
    cp.name                                         AS cartridge_name,
    cp.list_price_cents                             AS cartridge_list_unit_cents,
    COALESCE(last_paid.net_unit_cents, cp.list_price_cents)
                                                    AS cartridge_net_unit_cents,
    iu.expected_changeout_months,
    iu.last_filter_order_on
FROM installed_units iu
JOIN sites       s  ON s.id = iu.site_id
JOIN accounts    a  ON a.id = s.account_id    -- RLS: accounts
JOIN territories t  ON t.id = a.territory_id
JOIN products    up ON up.id = iu.product_id
LEFT JOIN products cp ON cp.id = iu.cartridge_product_id
LEFT JOIN LATERAL (
    SELECT ol.net_unit_cents
    FROM order_lines ol
    JOIN orders o ON o.id = ol.order_id       -- RLS: orders
    WHERE o.site_id = iu.site_id
      AND ol.product_id = iu.cartridge_product_id
    ORDER BY o.ordered_on DESC, o.id DESC
    LIMIT 1
) last_paid ON true;

-- 0007's GRANT ... ON ALL TABLES predates these views and does not cover
-- new objects (no ALTER DEFAULT PRIVILEGES exists). Grant explicitly.
GRANT SELECT ON v_order_facts, v_unit_facts TO plenum_app;
