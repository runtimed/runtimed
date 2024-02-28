-- This stores all messages coming in from a connected jupyter kernel
--
CREATE TABLE disorganized_messages (
    id UUID PRIMARY KEY,
    msg_id VARCHAR(255),
    msg_type VARCHAR(255),
    content JSONB,
    metadata JSONB,

    runtime_id UUID,

    -- We don't know if we have also collected the parent ID
    -- so we just store it for later retrieval
    parent_id UUID,
    parent_msg_type VARCHAR(255),

    created_at TIMESTAMP
);
