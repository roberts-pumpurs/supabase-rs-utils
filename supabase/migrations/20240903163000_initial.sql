-- Create a table for storing messages with timestamps
CREATE TABLE messages (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message TEXT NOT NULL
);

-- Create an index on created_at for faster timestamp-based queries
CREATE INDEX idx_messages_created_at ON messages(created_at);

-- Add a comment to the table
COMMENT ON TABLE messages IS 'Stores messages with their creation timestamps';

-- Enable the pg_cron extension
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Enable realtime functionality
CREATE EXTENSION IF NOT EXISTS pg_net;

-- Create a publication for realtime
DROP PUBLICATION IF EXISTS supabase_realtime;
CREATE PUBLICATION supabase_realtime FOR TABLE messages;

-- Create a function that will be called by the cron job
CREATE OR REPLACE FUNCTION insert_scheduled_message()
RETURNS void AS $$
BEGIN
    INSERT INTO messages (message)
    VALUES ('Scheduled message at ' || NOW());
END;
$$ LANGUAGE plpgsql;

-- Create a function to delete the oldest message
CREATE OR REPLACE FUNCTION delete_oldest_message()
RETURNS void AS $$
BEGIN
    DELETE FROM messages
    WHERE id = (
        SELECT id
        FROM messages
        ORDER BY created_at ASC
        LIMIT 1
    );
END;
$$ LANGUAGE plpgsql;

-- Schedule the job to run every 30 seconds
SELECT cron.schedule(
    'insert-message-every-1min',  -- job name
    '* * * * * *',           -- cron schedule (every 1 seconds)
    'SELECT insert_scheduled_message();'  -- the command to execute
);

-- Schedule the job to delete oldest message every minute
SELECT cron.schedule(
    'delete-oldest-message-every-1min',  -- job name
    '* * * * *',           -- cron schedule (every 1 minute)
    'SELECT delete_oldest_message();'  -- the command to execute
);
