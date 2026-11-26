-- 0026_product_history.sql
-- Who created, edited, binned, restored or purged each product, and what
-- changed. Written only by triggers, so every save path is caught: the
-- calculator, bulk Excel edits, recolour, bin. Admins read it.
--
-- A calculator save writes the product and its cost sheet in separate
-- requests, so edits by one person to one product within 2 minutes fold into
-- one entry (oldest "from", newest "to").
--
-- ponytail: cost lines are deleted and re-inserted on every save, so a line
-- swap that leaves every stage total unchanged isn't recorded. Add a lines
-- fingerprint to product_cost_sheets if that ever matters.

create table if not exists product_history (
  id           bigint generated always as identity primary key,
  product_id   uuid not null,          -- no FK: history outlives a purge
  product_code text not null,          -- as it was then
  product_name text not null,
  action       text not null check (action in ('created', 'edited', 'deleted', 'restored', 'purged')),
  changes      jsonb not null default '{}',  -- {"field": [old, new]}
  changed_by   uuid references profiles(id) on delete set null default auth.uid(),
  changed_at   timestamptz not null default now(),
  backfilled   boolean not null default false  -- rebuilt from before tracking began (below)
);

create index if not exists product_history_product_idx on product_history (product_id, changed_at desc);
create index if not exists product_history_changed_at_idx on product_history (changed_at desc);
create index if not exists product_history_changed_by_idx on product_history (changed_by);

alter table product_history enable row level security;

drop policy if exists product_history_admin_read on product_history;
create policy product_history_admin_read on product_history
  for select to authenticated using ((select is_admin()));

-- Before tracking: what the existing rows still tell. Created: products.created_at,
-- by whoever saved its cost sheet within 5 minutes (the calculator saves both).
-- Last edited and binned: the time only; who isn't recorded anywhere. An edit
-- within a minute of binning is the bin itself (the touch trigger), so skipped.
insert into product_history (product_id, product_code, product_name, action, changed_by, changed_at, backfilled)
select p.id, p.code, p.name, e.action, e.changed_by, e.changed_at, true
from products p
left join product_cost_sheets s on s.product_id = p.id
cross join lateral (
  select 'created' action, p.created_at changed_at,
         case when abs(extract(epoch from s.created_at - p.created_at)) < 300 then s.created_by end changed_by
  union all
  select 'edited', greatest(p.updated_at, s.updated_at), null
  where greatest(p.updated_at, s.updated_at) > p.created_at + interval '2 minutes'
    and (p.deleted_at is null or greatest(p.updated_at, s.updated_at) < p.deleted_at - interval '1 minute')
  union all
  select 'deleted', p.deleted_at, null
  where p.deleted_at is not null
) e
where not exists (select 1 from product_history where backfilled);

-- {"field": [old, new]} for each listed field that differs. JSON null and a
-- missing key count as the same, so a new row's empty fields aren't changes.
create or replace function product_history_diff(old_row jsonb, new_row jsonb, fields text[])
returns jsonb language sql immutable set search_path = public as $fn$
  select coalesce(jsonb_object_agg(f, jsonb_build_array(o, n)), '{}')
  from unnest(fields) f,
       lateral (select nullif(old_row -> f, 'null') o, nullif(new_row -> f, 'null') n) v
  where o is distinct from n
$fn$;

create or replace function log_product_history(
  p_id uuid, p_code text, p_name text, p_action text, p_changes jsonb
) returns void language plpgsql security definer set search_path = public as $fn$
declare
  recent product_history;
begin
  if p_action = 'edited' then
    if p_changes = '{}' then return; end if;
    select * into recent from product_history
     where product_id = p_id
       and changed_by is not distinct from auth.uid()
       and changed_at > now() - interval '2 minutes'
     order by changed_at desc
     limit 1;
    if found and recent.action = 'created' then
      return;  -- the rest of creating it
    end if;
    if found and recent.action = 'edited' then
      update product_history h set
        changes = (
          select coalesce(jsonb_object_agg(k, jsonb_build_array(o, n)), '{}')
          from (select jsonb_object_keys(recent.changes) k
                union select jsonb_object_keys(p_changes)) keys,
               lateral (select coalesce(recent.changes -> k -> 0, p_changes -> k -> 0) o,
                               coalesce(p_changes -> k -> 1, recent.changes -> k -> 1) n) v
          where o is distinct from n
        ),
        product_code = p_code,
        product_name = p_name,
        changed_at = now()
      where h.id = recent.id;
      return;
    end if;
  end if;

  insert into product_history (product_id, product_code, product_name, action, changes)
  values (p_id, p_code, p_name, p_action, coalesce(p_changes, '{}'));
end
$fn$;

create or replace function products_history_trigger()
returns trigger language plpgsql security definer set search_path = public as $fn$
declare
  fields constant text[] := array['code', 'name', 'category_id', 'source', 'cost_price',
                                  'markup_pct', 'default_sp', 'is_active', 'colors', 'image_url'];
begin
  if tg_op = 'INSERT' then
    perform log_product_history(new.id, new.code, new.name, 'created', '{}');
  elsif tg_op = 'DELETE' then
    perform log_product_history(old.id, old.code, old.name, 'purged', '{}');
  elsif old.deleted_at is null and new.deleted_at is not null then
    perform log_product_history(new.id, new.code, new.name, 'deleted', '{}');
  else
    if old.deleted_at is not null and new.deleted_at is null then
      perform log_product_history(new.id, new.code, new.name, 'restored', '{}');
    end if;
    perform log_product_history(new.id, new.code, new.name, 'edited',
      product_history_diff(to_jsonb(old), to_jsonb(new), fields));
  end if;
  return null;
end
$fn$;

drop trigger if exists products_history on products;
create trigger products_history
  after insert or update or delete on products
  for each row execute function products_history_trigger();

-- Costing changes, filed under the product.
create or replace function cost_sheets_history_trigger()
returns trigger language plpgsql security definer set search_path = public as $fn$
declare
  fields constant text[] := array['material_total', 'hardware_total', 'finishing_total',
                                  'machine_total', 'stage_overheads', 'notes'];
  p products;
begin
  select * into p from products where id = new.product_id;
  if not found then return null; end if;
  perform log_product_history(p.id, p.code, p.name, 'edited',
    product_history_diff(case when tg_op = 'INSERT' then '{}'::jsonb else to_jsonb(old) end,
                         to_jsonb(new), fields));
  return null;
end
$fn$;

drop trigger if exists cost_sheets_history on product_cost_sheets;
create trigger cost_sheets_history
  after insert or update on product_cost_sheets
  for each row execute function cost_sheets_history_trigger();

revoke execute on function log_product_history(uuid, text, text, text, jsonb) from public, anon, authenticated;
