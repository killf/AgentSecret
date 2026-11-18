-- MySQL official image creates MYSQL_DATABASE and MYSQL_USER before this script runs.
-- This script keeps permissions explicit and safe for recreating the application account.
ALTER DATABASE agentsecret CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE USER IF NOT EXISTS 'agentsecret'@'%' IDENTIFIED BY 'agentsecret';
GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, ALTER, INDEX, DROP, REFERENCES ON agentsecret.* TO 'agentsecret'@'%';
FLUSH PRIVILEGES;
