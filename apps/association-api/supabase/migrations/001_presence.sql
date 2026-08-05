-- Kept for reference / manual Postgres (Neon or Supabase).
-- Runtime also auto-creates this table on first presence heartbeat.
create table if not exists public.presence (
  player_uuid uuid primary key,
  username text not null check (char_length(username) between 1 and 16),
  server_hash char(64) not null check (server_hash ~ '^[0-9a-f]{64}$'),
  client_version text not null check (char_length(client_version) between 1 and 32),
  last_seen timestamptz not null default timezone('utc', now())
);

create index if not exists presence_server_hash_last_seen_idx
  on public.presence (server_hash, last_seen desc);
