-- Enums per spec §4. installed_units.source is deliberately NOT an enum:
-- it is plain text ('ours' or a competitor brand string — open set by design).

CREATE TYPE user_role AS ENUM ('rep', 'regional_manager', 'vp', 'admin');

CREATE TYPE account_status AS ENUM ('customer', 'prospect', 'at_risk', 'dormant');

CREATE TYPE opp_stage AS ENUM ('lead', 'qualified', 'quoted', 'negotiation', 'won', 'lost');

CREATE TYPE quote_status AS ENUM ('draft', 'pending_approval', 'approved', 'sent', 'accepted', 'rejected');

CREATE TYPE signal_type AS ENUM ('reorder_due', 'defection_risk', 'conquest', 'discount_anomaly');

CREATE TYPE signal_status AS ENUM ('open', 'assigned', 'actioned', 'dismissed');

CREATE TYPE product_kind AS ENUM ('capital', 'consumable', 'part', 'service');

CREATE TYPE activity_kind AS ENUM ('call', 'visit', 'email', 'note');

CREATE TYPE region AS ENUM ('northeast', 'southeast', 'midwest', 'south_central', 'mountain', 'west', 'canada_e', 'canada_w');
