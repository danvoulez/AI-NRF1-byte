-- ==========================================================================
-- LAB 512 — BASE Schema
--
-- This is the minimum schema the registry service needs.
-- Every table maps to a constitutional artifact.
--
-- Usage:
--   psql -d ubl_registry -f deploy/migrations/001_base_schema.sql
-- ==========================================================================

-- --- Apps and Tenants ---

CREATE TABLE IF NOT EXISTS app (
    id          UUID PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS tenant (
    id          UUID PRIMARY KEY,
    app_id      UUID NOT NULL REFERENCES app(id),
    slug        TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(app_id, slug)
);

-- --- Users and RBAC ---

CREATE TABLE IF NOT EXISTS membership (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    tenant_id   UUID NOT NULL REFERENCES tenant(id),
    role        TEXT NOT NULL,  -- signer | tenant_admin | app_owner
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, tenant_id, role)
);

-- --- Receipts (Article IV §4.1) ---

CREATE TABLE IF NOT EXISTS receipt (
    id          UUID PRIMARY KEY,
    app_id      UUID NOT NULL REFERENCES app(id),
    tenant_id   UUID NOT NULL REFERENCES tenant(id),
    cid         TEXT NOT NULL,      -- b3:<hex> receipt_cid
    did         TEXT NOT NULL,      -- issuer DID
    url         TEXT NOT NULL,      -- rich URL
    body        JSONB NOT NULL,     -- full signed receipt as JSON
    decision    TEXT,               -- ALLOW | DENY | REQUIRE | GHOST
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_receipt_cid ON receipt(cid);
CREATE INDEX IF NOT EXISTS idx_receipt_tenant ON receipt(tenant_id, created_at DESC);

-- --- Ghosts (Article IV §4.3 — WBE) ---

CREATE TABLE IF NOT EXISTS ghost (
    id          UUID PRIMARY KEY,
    app_id      UUID NOT NULL REFERENCES app(id),
    tenant_id   UUID NOT NULL REFERENCES tenant(id),
    cid         TEXT NOT NULL,      -- b3:<hex> ghost_cid
    did         TEXT NOT NULL,      -- actor DID
    url         TEXT NOT NULL,      -- rich URL
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending | promoted | expired
    receipt_cid TEXT,               -- set when promoted
    cause       TEXT,               -- set when expired (timeout | canceled | drift)
    wbe         JSONB NOT NULL,     -- full ghost as JSON
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ghost_cid ON ghost(cid);
CREATE INDEX IF NOT EXISTS idx_ghost_status ON ghost(tenant_id, status);

-- --- Seed data for development ---

INSERT INTO app (id, slug, name) VALUES
    ('00000000-0000-0000-0000-000000000001', 'lab512', 'LAB 512 Development')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO tenant (id, app_id, slug, name) VALUES
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001', 'dev', 'Development Tenant')
ON CONFLICT (app_id, slug) DO NOTHING;

-- Dev user with signer role (use this UUID as x-user-id header)
INSERT INTO membership (user_id, tenant_id, role) VALUES
    ('00000000-0000-0000-0000-000000000099', '00000000-0000-0000-0000-000000000002', 'signer')
ON CONFLICT (user_id, tenant_id, role) DO NOTHING;
