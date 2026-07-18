-- Full P0 schema per spec §4. Money is BIGINT cents. Timestamps timestamptz.
-- uuid PKs default gen_random_uuid() server-side (the seed generates its own
-- deterministic uuids; the default serves the app path).

CREATE TABLE users (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    name          text NOT NULL,
    role          user_role NOT NULL,
    manager_id    uuid REFERENCES users (id)
);

CREATE TABLE territories (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code             text NOT NULL UNIQUE,
    name             text NOT NULL,
    region           region NOT NULL,
    quota_year_cents bigint NOT NULL CHECK (quota_year_cents >= 0)
);

-- A rep can carry more than one territory.
CREATE TABLE territory_assignments (
    user_id      uuid NOT NULL REFERENCES users (id),
    territory_id uuid NOT NULL REFERENCES territories (id),
    PRIMARY KEY (user_id, territory_id)
);

CREATE TABLE accounts (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name              text NOT NULL,
    industry          text NOT NULL,
    status            account_status NOT NULL,
    territory_id      uuid NOT NULL REFERENCES territories (id),
    parent_account_id uuid REFERENCES accounts (id),
    created           timestamptz NOT NULL DEFAULT now()
);

-- sites <-> contacts is a deliberate circular reference (a site names its
-- primary contact; a contact belongs to a site). The FK on primary_contact_id
-- is added after contacts exists.
CREATE TABLE sites (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id uuid NOT NULL REFERENCES accounts (id),
    address    text NOT NULL,
    city       text NOT NULL,
    state      text NOT NULL
);

CREATE TABLE contacts (
    id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id uuid NOT NULL REFERENCES sites (id),
    name    text NOT NULL,
    title   text,
    email   text,
    phone   text,
    role    text -- buyer / EHS / plant engineer / maintenance (open set)
);

ALTER TABLE sites
    ADD COLUMN primary_contact_id uuid REFERENCES contacts (id);

CREATE TABLE products (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    sku              text NOT NULL UNIQUE,
    name             text NOT NULL,
    family           text NOT NULL,
    kind             product_kind NOT NULL,
    list_price_cents bigint NOT NULL CHECK (list_price_cents >= 0),
    -- Collector family codes a cartridge serves — including competitor
    -- families (the conquest cross-reference).
    filter_fits      text[] NOT NULL DEFAULT '{}'
);

CREATE TABLE installed_units (
    id                       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id                  uuid NOT NULL REFERENCES sites (id),
    product_id               uuid NOT NULL REFERENCES products (id),
    serial                   text NOT NULL UNIQUE,
    commissioned_on          date NOT NULL,
    -- 'ours' or a competitor brand string. Open set by design — NOT an enum.
    source                   text NOT NULL,
    cartridge_count          int NOT NULL CHECK (cartridge_count > 0),
    cartridge_product_id     uuid REFERENCES products (id),
    -- NULLABLE on purpose: the seed's data-quality story beat depends on it.
    expected_changeout_months int,
    last_filter_order_on     date,
    -- Telemetry stretch (Artifact 1 integration) — nullable until then.
    filter_life_pct          numeric(5, 2)
);

CREATE TABLE opportunities (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id     uuid NOT NULL REFERENCES accounts (id),
    territory_id   uuid NOT NULL REFERENCES territories (id),
    owner_id       uuid NOT NULL REFERENCES users (id),
    stage          opp_stage NOT NULL,
    kind           text NOT NULL, -- capital / retrofit / filter-program
    amount_cents   bigint NOT NULL CHECK (amount_cents >= 0), -- gross
    expected_close date,
    lost_reason    text
);

CREATE TABLE quotes (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    opportunity_id uuid NOT NULL REFERENCES opportunities (id),
    status         quote_status NOT NULL,
    approver_id    uuid REFERENCES users (id),
    created_by     uuid NOT NULL REFERENCES users (id),
    created_at     timestamptz NOT NULL DEFAULT now()
);

-- The dual-ledger price triplet. All three stored; the CHECK keeps them
-- consistent. round() in Postgres is half-away-from-zero; the Rust seed
-- computes net with integer round-half-up, which is identical for these
-- non-negative values.
CREATE TABLE quote_lines (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    quote_id        uuid NOT NULL REFERENCES quotes (id),
    product_id      uuid NOT NULL REFERENCES products (id),
    qty             int NOT NULL CHECK (qty > 0),
    list_unit_cents bigint NOT NULL CHECK (list_unit_cents >= 0),
    net_unit_cents  bigint NOT NULL,
    discount_pct    numeric(5, 2) NOT NULL CHECK (discount_pct >= 0 AND discount_pct <= 100),
    CHECK (net_unit_cents = round(list_unit_cents * (100 - discount_pct) / 100))
);

CREATE TABLE orders (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id   uuid NOT NULL REFERENCES accounts (id),
    site_id      uuid NOT NULL REFERENCES sites (id),
    territory_id uuid NOT NULL REFERENCES territories (id),
    rep_id       uuid NOT NULL REFERENCES users (id),
    ordered_on   date NOT NULL
);

CREATE TABLE order_lines (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id        uuid NOT NULL REFERENCES orders (id),
    product_id      uuid NOT NULL REFERENCES products (id),
    qty             int NOT NULL CHECK (qty > 0),
    list_unit_cents bigint NOT NULL CHECK (list_unit_cents >= 0),
    net_unit_cents  bigint NOT NULL,
    discount_pct    numeric(5, 2) NOT NULL CHECK (discount_pct >= 0 AND discount_pct <= 100),
    CHECK (net_unit_cents = round(list_unit_cents * (100 - discount_pct) / 100))
);

CREATE TABLE signals (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    type              signal_type NOT NULL,
    account_id        uuid NOT NULL REFERENCES accounts (id),
    site_id           uuid REFERENCES sites (id),
    installed_unit_id uuid REFERENCES installed_units (id),
    score             numeric(10, 2) NOT NULL DEFAULT 0,
    -- Array of {label, weight, detail} — the AI-with-receipts contract.
    reasons           jsonb NOT NULL DEFAULT '[]',
    status            signal_status NOT NULL DEFAULT 'open',
    assigned_to       uuid REFERENCES users (id),
    outcome           text,
    dismissed_reason  text,
    -- One timestamp per lifecycle transition.
    opened_at         timestamptz NOT NULL DEFAULT now(),
    assigned_at       timestamptz,
    actioned_at       timestamptz,
    dismissed_at      timestamptz
);

CREATE TABLE activities (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  uuid NOT NULL REFERENCES accounts (id),
    rep_id      uuid NOT NULL REFERENCES users (id),
    kind        activity_kind NOT NULL,
    occurred_at timestamptz NOT NULL,
    body        text NOT NULL
);

CREATE TABLE audit_log (
    id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    actor     uuid, -- NULL when the seed (admin connection, no GUC) writes
    action    text NOT NULL,
    entity    text NOT NULL,
    entity_id uuid NOT NULL,
    before    jsonb,
    after     jsonb,
    at        timestamptz NOT NULL
);
