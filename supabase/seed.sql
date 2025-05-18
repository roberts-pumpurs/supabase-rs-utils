-- Insert 10 initial seed entries
INSERT INTO messages (created_at, message)
VALUES (
        NOW() - INTERVAL '9 minutes',
        'First seed message'
    ),
    (
        NOW() - INTERVAL '8 minutes',
        'Welcome to the database!'
    ),
    (
        NOW() - INTERVAL '7 minutes',
        'This is a sample message'
    ),
    (
        NOW() - INTERVAL '6 minutes',
        'Database initialization complete'
    ),
    (
        NOW() - INTERVAL '5 minutes',
        'Ready for production use'
    ),
    (
        NOW() - INTERVAL '4 minutes',
        'System is operational'
    ),
    (
        NOW() - INTERVAL '3 minutes',
        'All services running'
    ),
    (
        NOW() - INTERVAL '2 minutes',
        'Monitoring active'
    ),
    (NOW() - INTERVAL '1 minute', 'Last seed entry'),
    (NOW(), 'Current timestamp entry');
-- Create a new user in auth.users
INSERT INTO auth.users (
        instance_id,
        id,
        aud,
        role,
        email,
        encrypted_password,
        email_confirmed_at,
        recovery_sent_at,
        last_sign_in_at,
        raw_app_meta_data,
        raw_user_meta_data,
        created_at,
        updated_at,
        confirmation_token,
        email_change,
        email_change_token_new,
        recovery_token
    )
VALUES (
        '00000000-0000-0000-0000-000000000000',
        gen_random_uuid(),
        'authenticated',
        'authenticated',
        'username@username.com',
        crypt('password', gen_salt('bf')),
        NOW(),
        NOW(),
        NOW(),
        '{"provider":"email","providers":["email"]}',
        '{}',
        NOW(),
        NOW(),
        '',
        '',
        '',
        ''
    );
