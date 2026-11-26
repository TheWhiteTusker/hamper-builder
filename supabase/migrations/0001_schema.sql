-- Lattice Lane Hamper Builder - core schema
-- Replaces the Google Sheet tabs: Product Master, Settings, Hamper Summary,
-- Hamper Details, Quotation Register, Quote Details.

create extension if not exists pgcrypto;

-- ---------------------------------------------------------------
-- ROLES / PROFILES
-- ---------------------------------------------------------------

create type user_role as enum ('admin', 'manager', 'sales');

create table profiles (
  id         uuid primary key references auth.users(id) on delete cascade,
  full_name  text,
  role       user_role not null default 'sales',
  created_at timestamptz not null default now()
);

-- First person to sign up owns the workspace; everyone after starts as sales.
create function handle_new_user() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  insert into public.profiles (id, full_name, role)
  values (
    new.id,
    coalesce(new.raw_user_meta_data ->> 'full_name', new.email),
    case when (select count(*) from public.profiles) = 0 then 'admin' else 'sales' end
  );
  return new;
end $fn$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function handle_new_user();

-- security definer so RLS policies can call these without recursing on profiles
create function app_role() returns user_role
language sql stable security definer set search_path = public as $fn$
  select role from public.profiles where id = auth.uid()
$fn$;

create function is_admin() returns boolean
language sql stable security definer set search_path = public as $fn$
  select coalesce(app_role() = 'admin', false)
$fn$;

create function is_manager() returns boolean
language sql stable security definer set search_path = public as $fn$
  select coalesce(app_role() in ('admin', 'manager'), false)
$fn$;

-- ---------------------------------------------------------------
-- SHARED
-- ---------------------------------------------------------------

create function touch_updated_at() returns trigger
language plpgsql as $fn$
begin
  new.updated_at = now();
  return new;
end $fn$;

-- ---------------------------------------------------------------
-- SETTINGS SHEET  ->  categories + app_settings
-- ---------------------------------------------------------------

-- Settings!A = category name, Settings!B = counts toward "No. of Items".
-- This is what categoryCountsAsItem() in the Apps Script read.
create table categories (
  id             uuid primary key default gen_random_uuid(),
  name           text not null unique,
  counts_as_item boolean not null default true,
  sort_order     int not null default 0,
  is_active      boolean not null default true,
  created_at     timestamptz not null default now()
);

-- Editable pick-lists and company config. Seeded in 0003 and topped up from
-- the spreadsheet during import, so the exact strings the sheet used
-- (statuses, detail modes, packaging treatments) survive the move.
create table app_settings (
  key        text primary key,
  value      jsonb not null,
  updated_at timestamptz not null default now()
);

create trigger app_settings_touch before update on app_settings
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- PRODUCT MASTER
-- ---------------------------------------------------------------

-- Sheet columns: A code, B category, C name, D source,
--                E cost price, F target margin, G default SP, H active
create table products (
  id            uuid primary key default gen_random_uuid(),
  code          text not null unique,
  category_id   uuid references categories(id) on delete restrict,
  name          text not null,
  source        text,
  cost_price    numeric not null default 0,
  target_margin numeric not null default 0,    -- fraction: 0.35 = 35%
  default_sp    numeric not null default 0,
  is_active     boolean not null default true,
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

create index products_category_idx on products (category_id);
create index products_active_idx   on products (is_active);
create index products_name_idx     on products (lower(name));

create trigger products_touch before update on products
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- HAMPERS
-- ---------------------------------------------------------------

-- Only the *input* columns live here. Everything the Hamper Summary sheet
-- stored as a computed value is derived by the hamper_summary view below.
-- final_catalogue_sp stays a manual override: refreshHamperPrices in the
-- Apps Script deliberately preserved it, and so do we.
create table hampers (
  id                 uuid primary key default gen_random_uuid(),
  code               text not null unique,
  name               text not null,
  collection         text,
  status             text not null default 'Draft',
  target_sp          numeric,
  notes              text,
  discount_pct       numeric not null default 0,   -- fraction
  final_catalogue_sp numeric,
  created_by         uuid references profiles(id) on delete set null,
  created_at         timestamptz not null default now(),
  updated_at         timestamptz not null default now()
);

create index hampers_status_idx     on hampers (status);
create index hampers_collection_idx on hampers (collection);

create trigger hampers_touch before update on hampers
  for each row execute function touch_updated_at();

-- Price snapshots, exactly like the Hamper Details sheet: saving copies the
-- product cost/margin/SP in, and a saved hamper does NOT move when Product
-- Master changes. Re-syncing is an explicit action (Refresh Prices).
create table hamper_items (
  id            uuid primary key default gen_random_uuid(),
  hamper_id     uuid not null references hampers(id) on delete cascade,
  line_no       int not null,
  product_id    uuid references products(id) on delete set null,
  product_code  text,
  product_name  text not null,
  category_name text,
  source        text,
  qty           numeric not null default 1,
  unit_cp       numeric not null default 0,
  unit_sp       numeric not null default 0,
  target_margin numeric not null default 0,
  total_cp      numeric generated always as (qty * unit_cp) stored,
  total_sp      numeric generated always as (qty * unit_sp) stored,
  unique (hamper_id, line_no)
);

create index hamper_items_hamper_idx  on hamper_items (hamper_id);
create index hamper_items_product_idx on hamper_items (product_id);

-- ---------------------------------------------------------------
-- QUOTES
-- ---------------------------------------------------------------

create type doc_type as enum ('quotation', 'proforma_invoice');

create table quotes (
  id              uuid primary key default gen_random_uuid(),
  doc_no          text not null unique,
  doc_type        doc_type not null default 'quotation',
  doc_date        date not null default current_date,
  client_name     text not null,
  contact_person  text,
  phone           text,
  email           text,
  billing_address text,
  gstin           text,
  occasion        text,
  quote_structure text not null default 'Combined Order',
  validity        text,
  status          text not null default 'Draft',
  order_discount  numeric not null default 0,    -- sheet H23
  adj1            numeric not null default 0,    -- sheet H24
  adj2            numeric not null default 0,    -- sheet H25
  gst_rate        numeric not null default 0.18, -- sheet H27
  notes           text,
  terms           text,
  follow_up_date  date,
  linked_doc_no   text,
  created_by      uuid references profiles(id) on delete set null,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now()
);

create index quotes_client_idx    on quotes (lower(client_name));
create index quotes_status_idx    on quotes (status);
create index quotes_follow_up_idx on quotes (follow_up_date);

create trigger quotes_touch before update on quotes
  for each row execute function touch_updated_at();

-- Mirrors the Quote Details sheet. final_rate and amount were spreadsheet
-- formulas written by addToQuote(); here they are generated columns, so they
-- can never drift out of sync with their inputs.
create table quote_items (
  id                  uuid primary key default gen_random_uuid(),
  quote_id            uuid not null references quotes(id) on delete cascade,
  line_no             int not null,
  option_label        text,
  hamper_id           uuid references hampers(id) on delete set null,
  hamper_code         text,
  hamper_name         text,
  qty                 numeric not null default 1,
  catalogue_price     numeric not null default 0,
  discount_pct        numeric not null default 0,
  detail_mode         text,
  packaging_treatment text,
  final_rate numeric generated always as
    (catalogue_price * (1 - coalesce(discount_pct, 0))) stored,
  amount     numeric generated always as
    (qty * catalogue_price * (1 - coalesce(discount_pct, 0))) stored,
  unique (quote_id, line_no)
);

create index quote_items_quote_idx  on quote_items (quote_id);
create index quote_items_hamper_idx on quote_items (hamper_id);

-- ---------------------------------------------------------------
-- DERIVED VIEWS  (these replace the Hamper Summary sheet and the
-- I26:I35 / H22:H29 formula blocks in the Builder tabs)
-- ---------------------------------------------------------------

create view hamper_totals with (security_invoker = on) as
select
  h.id as hamper_id,
  coalesce(sum(hi.total_cp), 0) as total_cp,
  coalesce(sum(hi.total_sp), 0) as base_sp,
  -- categoryCountsAsItem(): unknown categories default to counting
  coalesce(sum(hi.qty) filter (where coalesce(c.counts_as_item, true)), 0)
    as number_of_items,
  count(hi.id)::int as line_count
from hampers h
left join hamper_items hi on hi.hamper_id = h.id
left join categories c
  on lower(btrim(c.name)) = lower(btrim(hi.category_name))
group by h.id;

create view hamper_summary with (security_invoker = on) as
select
  h.id,
  h.code,
  h.name,
  h.collection,
  h.status,
  t.number_of_items,
  t.total_cp,
  t.base_sp,
  h.discount_pct,
  t.base_sp * (1 - h.discount_pct) as sp_after_discount,
  h.target_sp,
  case when h.target_sp is null then null
       else t.base_sp * (1 - h.discount_pct) - h.target_sp
  end as variance,
  h.final_catalogue_sp,
  case when h.final_catalogue_sp is null then null
       else h.final_catalogue_sp - t.total_cp
  end as gross_profit,
  case when coalesce(h.final_catalogue_sp, 0) = 0 then null
       else (h.final_catalogue_sp - t.total_cp) / h.final_catalogue_sp
  end as final_margin,
  h.notes,
  h.created_by,
  h.created_at,
  h.updated_at,
  t.line_count
from hampers h
join hamper_totals t on t.hamper_id = h.id;

-- Totals only apply to a Combined Order, matching the sheet formulas
-- =IF(B7="Combined Order", ...) in H22/H26/H28/H29. Option-style quotes
-- leave them null: the client picks one option, so there is no order total.
create view quote_summary with (security_invoker = on) as
select
  q.*,
  coalesce(a.subtotal, 0)    as subtotal,
  coalesce(a.total_qty, 0)   as total_qty,
  coalesce(a.line_count, 0)  as line_count,
  tx.taxable_value,
  tx.taxable_value * q.gst_rate       as gst_amount,
  tx.taxable_value * (1 + q.gst_rate) as grand_total
from quotes q
left join (
  select quote_id,
         sum(amount)   as subtotal,
         sum(qty)      as total_qty,
         count(*)::int as line_count
  from quote_items
  group by quote_id
) a on a.quote_id = q.id
left join lateral (
  select case when q.quote_structure = 'Combined Order'
              then coalesce(a.subtotal, 0) - q.order_discount + q.adj1 + q.adj2
         end as taxable_value
) tx on true;

-- ---------------------------------------------------------------
-- DOCUMENT NUMBERING
-- ---------------------------------------------------------------

-- The Apps Script did MAX(existing) + 1 with no lock, so two people saving at
-- the same moment produced the same code. UPDATE ... RETURNING takes a row
-- lock, which serialises concurrent callers.
create table doc_counters (
  prefix      text primary key,
  last_number int not null default 0
);

create function next_doc_no(p_prefix text, p_pad int default 3)
returns text language plpgsql security definer set search_path = public as $fn$
declare n int;
begin
  insert into doc_counters (prefix, last_number)
  values (p_prefix, 0)
  on conflict (prefix) do nothing;

  update doc_counters
     set last_number = last_number + 1
   where prefix = p_prefix
  returning last_number into n;

  return p_prefix || lpad(n::text, p_pad, '0');
end $fn$;

-- Run after importing the spreadsheet so numbering continues from the highest
-- existing H### / LLQT-### / LLPI-### instead of restarting at 001.
create function sync_doc_counters() returns void
language plpgsql security definer set search_path = public as $fn$
declare
  p text;
  hi int;
begin
  foreach p in array array['H', 'LLQT-', 'LLPI-'] loop
    select coalesce(max(nullif(regexp_replace(code, '^' || p || '(\d+)$', '\1'), code)::int), 0)
      into hi
      from (
        select code from hampers
        union all
        select doc_no from quotes
      ) all_codes(code)
     where code ~ ('^' || p || '\d+$');

    insert into doc_counters (prefix, last_number)
    values (p, hi)
    on conflict (prefix) do update set last_number = greatest(doc_counters.last_number, excluded.last_number);
  end loop;
end $fn$;
