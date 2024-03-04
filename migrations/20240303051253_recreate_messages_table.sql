-- Sadly sqlite doesn't allow us to change constraints on a column!
DROP TABLE disorganized_messages;

CREATE TABLE disorganized_messages (
    id UUID PRIMARY KEY NOT NULL,
    msg_id VARCHAR(255),
    msg_type VARCHAR(255),
    content JSONB,
    metadata JSONB,

    runtime_id UUID NOT NULL,

    -- We don't know if we have also collected the parent ID
    -- so we just store it for later retrieval
    parent_msg_id UUID,
    parent_msg_type VARCHAR(255),

    created_at TIMESTAMPZ NOT NULL
);
