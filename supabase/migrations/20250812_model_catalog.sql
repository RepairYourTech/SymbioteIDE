-- Migration: Model Catalog schema (source of truth)
-- This mirrors newplan/specs/migrations/2025-08-12_model_catalog.sql

create extension if not exists pgcrypto;

create or replace function public.set_updated_at()
returns trigger language plpgsql as $$
begin
  new.updated_at = now();
  return new;
end; $$;

create table if not exists public.models (
  id uuid primary key default gen_random_uuid(),
  slug text unique not null,
  provider text not null,
  name text,
  model_version text,
  is_local boolean default false,
  uses text[] default '{}',
  input_modalities text[] default '{}',
  output_modalities text[] default '{}',
  supported_features text[] default '{}',
  supported_parameters text[] default '{}',
  context_window integer,
  max_output_tokens integer,
  pricing jsonb,
  device_caps jsonb,
  quantization jsonb,
  local_runner jsonb,
  datacenters jsonb,
  availability text,
  reliability_score numeric(3,2),
  latency_ms integer,
  created_at timestamptz default now(),
  updated_at timestamptz default now()
);
create trigger models_set_updated_at
  before update on public.models
  for each row execute function public.set_updated_at();

create index if not exists idx_models_provider on public.models(provider);
create index if not exists idx_models_uses on public.models using gin(uses);
create index if not exists idx_models_supported_features on public.models using gin(supported_features);
create index if not exists idx_models_supported_parameters on public.models using gin(supported_parameters);
create index if not exists idx_models_quantization on public.models using gin(quantization);
create index if not exists idx_models_device_caps on public.models using gin(device_caps);
create index if not exists idx_models_pricing on public.models using gin(pricing);
create index if not exists idx_models_context_window on public.models(context_window);
create index if not exists idx_models_latency_ms on public.models(latency_ms);

create table if not exists public.task_categories (
  id uuid primary key default gen_random_uuid(),
  category_name text unique not null,
  domain text not null,
  description text,
  complexity_level integer,
  created_at timestamptz default now()
);

create table if not exists public.model_ratings (
  id uuid primary key default gen_random_uuid(),
  model_id uuid references public.models(id) on delete cascade,
  task_category_id uuid references public.task_categories(id) on delete set null,
  user_id uuid,
  quality_score numeric(3,2),
  speed_score numeric(3,2),
  cost_effectiveness numeric(3,2),
  reliability_score numeric(3,2),
  pros text[],
  cons text[],
  use_case_notes text,
  task_complexity integer,
  dataset_size text,
  created_at timestamptz default now(),
  updated_at timestamptz default now()
);
create trigger model_ratings_set_updated_at
  before update on public.model_ratings
  for each row execute function public.set_updated_at();
create index if not exists idx_model_ratings_model on public.model_ratings(model_id);
create index if not exists idx_model_ratings_scores on public.model_ratings(quality_score, speed_score, cost_effectiveness, reliability_score);

create table if not exists public.model_combinations (
  id uuid primary key default gen_random_uuid(),
  combination_name text not null,
  task_category_id uuid references public.task_categories(id) on delete set null,
  models jsonb,
  overall_rating numeric(3,2),
  cost_per_task numeric(10,6),
  avg_completion_time integer,
  upvotes integer default 0,
  downvotes integer default 0,
  user_notes text[],
  created_at timestamptz default now(),
  updated_at timestamptz default now()
);
create trigger model_combinations_set_updated_at
  before update on public.model_combinations
  for each row execute function public.set_updated_at();

create table if not exists public.user_model_preferences (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null,
  domain text not null,
  preferred_models jsonb,
  budget_preference text,
  privacy_preference text,
  created_at timestamptz default now(),
  updated_at timestamptz default now()
);
create trigger user_model_preferences_set_updated_at
  before update on public.user_model_preferences
  for each row execute function public.set_updated_at();

alter table public.models enable row level security;
alter table public.task_categories enable row level security;
alter table public.model_ratings enable row level security;
alter table public.model_combinations enable row level security;
alter table public.user_model_preferences enable row level security;

create policy models_select_anon on public.models for select using (true);
create policy task_categories_select_anon on public.task_categories for select using (true);
create policy model_ratings_select_anon on public.model_ratings for select using (true);
create policy model_combinations_select_anon on public.model_combinations for select using (true);
create policy user_model_preferences_select_auth on public.user_model_preferences for select to authenticated using (true);

