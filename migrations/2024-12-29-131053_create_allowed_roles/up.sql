CREATE TABLE allowed_roles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    guild BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    operator_role BIGINT NOT NULL
);