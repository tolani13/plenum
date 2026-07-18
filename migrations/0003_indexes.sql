-- Spec-named indexes plus FK-support indexes.

-- Spec §4 named indexes on installed_units.
CREATE INDEX idx_installed_units_site ON installed_units (site_id);
CREATE INDEX idx_installed_units_cartridge_last ON installed_units (cartridge_product_id, last_filter_order_on);

-- FK-support indexes.
CREATE INDEX idx_accounts_territory ON accounts (territory_id);
CREATE INDEX idx_sites_account ON sites (account_id);
CREATE INDEX idx_orders_territory_ordered ON orders (territory_id, ordered_on);
CREATE INDEX idx_orders_account ON orders (account_id);
CREATE INDEX idx_order_lines_order ON order_lines (order_id);
CREATE INDEX idx_order_lines_product ON order_lines (product_id);
CREATE INDEX idx_opportunities_territory ON opportunities (territory_id);
CREATE INDEX idx_signals_status_type ON signals (status, type);
CREATE INDEX idx_activities_account_occurred ON activities (account_id, occurred_at);
CREATE INDEX idx_audit_log_entity ON audit_log (entity, entity_id, at);
