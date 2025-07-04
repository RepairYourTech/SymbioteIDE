-- PostgreSQL initialization script for SymbioteIDE

-- Create symbiote user and database
CREATE USER symbiote WITH PASSWORD 'symbiote123';
CREATE DATABASE symbiote OWNER symbiote;

-- Grant all privileges on symbiote database to symbiote user
GRANT ALL PRIVILEGES ON DATABASE symbiote TO symbiote;

-- Create langfuse database if it doesn't exist (for Langfuse service)
-- Note: The default postgres user creates the langfuse db, but we ensure it exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_database WHERE datname = 'langfuse') THEN
        CREATE DATABASE langfuse;
    END IF;
END
$$;

-- Connect to symbiote database to create schema
\c symbiote;

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create schema for better organization
CREATE SCHEMA IF NOT EXISTS symbiote AUTHORIZATION symbiote;

-- Set search path
ALTER DATABASE symbiote SET search_path TO symbiote, public;

-- Grant usage on schema
GRANT ALL ON SCHEMA symbiote TO symbiote;
GRANT ALL ON SCHEMA public TO symbiote;