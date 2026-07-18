-- Row-Level Security on exactly the six spec §4 tables:
-- accounts, orders, opportunities, quotes, signals, activities.
--
-- Scope key is the row's territory:
--   accounts / orders / opportunities — territory_id directly;
--   quotes — via opportunity_id -> opportunities.territory_id;
--   signals / activities — via account_id -> accounts.territory_id.
--
-- The predicate reads the transaction-local GUC app.user_id that the API's
-- RLS middleware sets via set_config(..., true). current_setting(..., true)
-- returns NULL when the GUC was never set; NULLIF folds empty-string to NULL
-- too. NULL user_id matches no v_user_scope row -> zero rows. A connection
-- that skipped the middleware sees NOTHING rather than everything: fail closed.
--
-- ENABLE + FORCE on every table. FORCE is belt-and-braces: even the table
-- owner is subject to RLS (superusers still bypass — the plenum_admin
-- seed/migration path — but plenum_app has no bypass, period).
--
-- The subselect-per-row policies on quotes/signals/activities are O(row) —
-- acceptable at P0 scale; scoping stays in Postgres, never in app code.

-- accounts ------------------------------------------------------------------
ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE accounts FORCE ROW LEVEL SECURITY;

CREATE POLICY accounts_territory_scope ON accounts
FOR ALL
USING (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
))
WITH CHECK (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
));

-- orders --------------------------------------------------------------------
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE orders FORCE ROW LEVEL SECURITY;

CREATE POLICY orders_territory_scope ON orders
FOR ALL
USING (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
))
WITH CHECK (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
));

-- opportunities -------------------------------------------------------------
ALTER TABLE opportunities ENABLE ROW LEVEL SECURITY;
ALTER TABLE opportunities FORCE ROW LEVEL SECURITY;

CREATE POLICY opportunities_territory_scope ON opportunities
FOR ALL
USING (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
))
WITH CHECK (territory_id IN (
  SELECT territory_id FROM v_user_scope
  WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
));

-- quotes (scope via opportunity's territory) --------------------------------
ALTER TABLE quotes ENABLE ROW LEVEL SECURITY;
ALTER TABLE quotes FORCE ROW LEVEL SECURITY;

CREATE POLICY quotes_territory_scope ON quotes
FOR ALL
USING (opportunity_id IN (
  SELECT o.id FROM opportunities o
  WHERE o.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
))
WITH CHECK (opportunity_id IN (
  SELECT o.id FROM opportunities o
  WHERE o.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
));

-- signals (scope via account's territory) -----------------------------------
ALTER TABLE signals ENABLE ROW LEVEL SECURITY;
ALTER TABLE signals FORCE ROW LEVEL SECURITY;

CREATE POLICY signals_territory_scope ON signals
FOR ALL
USING (account_id IN (
  SELECT a.id FROM accounts a
  WHERE a.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
))
WITH CHECK (account_id IN (
  SELECT a.id FROM accounts a
  WHERE a.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
));

-- activities (scope via account's territory) --------------------------------
ALTER TABLE activities ENABLE ROW LEVEL SECURITY;
ALTER TABLE activities FORCE ROW LEVEL SECURITY;

CREATE POLICY activities_territory_scope ON activities
FOR ALL
USING (account_id IN (
  SELECT a.id FROM accounts a
  WHERE a.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
))
WITH CHECK (account_id IN (
  SELECT a.id FROM accounts a
  WHERE a.territory_id IN (
    SELECT territory_id FROM v_user_scope
    WHERE user_id = NULLIF(current_setting('app.user_id', true), '')::uuid
  )
));
