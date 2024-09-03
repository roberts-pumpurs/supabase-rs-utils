
INSERT INTO users (username, email)
VALUES
('john_doe', 'john@example.com'),
('jane_doe', 'jane@example.com');


INSERT INTO posts (user_id, title, content)
VALUES
((SELECT id FROM users WHERE username = 'john_doe'), 'First Post', 'This is the content of the first post.'),
((SELECT id FROM users WHERE username = 'jane_doe'), 'Second Post', 'This is the content of the second post.');


INSERT INTO comments (post_id, user_id, content)
VALUES
((SELECT id FROM posts WHERE title = 'First Post'), (SELECT id FROM users WHERE username = 'jane_doe'), 'This is a comment on the first post.'),
((SELECT id FROM posts WHERE title = 'Second Post'), (SELECT id FROM users WHERE username = 'john_doe'), 'This is a comment on the second post.');
