-- 0021_bin_feature.sql
-- Soft deletion and 30-day Recycle Bin for hampers, products, and product_images.

-- 1. ADD DELETED_AT COLUMNS
alter table hampers add column if not exists deleted_at timestamptz;
alter table products add column if not exists deleted_at timestamptz;
alter table product_images add column if not exists deleted_at timestamptz;

-- 2. INDEXES
create index if not exists hampers_deleted_at_idx on hampers (deleted_at) where deleted_at is not null;
create index if not exists products_deleted_at_idx on products (deleted_at) where deleted_at is not null;
create index if not exists product_images_deleted_at_idx on product_images (deleted_at) where deleted_at is not null;

create index if not exists hampers_active_idx on hampers (id) where deleted_at is null;
create index if not exists products_active_idx on products (id) where deleted_at is null;
create index if not exists product_images_active_idx on product_images (id) where deleted_at is null;

-- 3. PARTIAL UNIQUE INDEXES FOR CODE (Allows soft-deleted codes not to conflict with new items)
alter table hampers drop constraint if exists hampers_code_key;
drop index if exists hampers_code_active_key;
create unique index hampers_code_active_key on hampers (code) where deleted_at is null;

alter table products drop constraint if exists products_code_key;
drop index if exists products_code_active_key;
create unique index products_code_active_key on products (code) where deleted_at is null;

-- 4. UPDATE HAMPER_SUMMARY VIEW TO ONLY INCLUDE ACTIVE HAMPERS
drop view if exists hamper_summary;
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
  t.line_count,
  h.image_url,
  h.deleted_at
from hampers h
join hamper_totals t on t.hamper_id = h.id
where h.deleted_at is null;

-- 5. FUNCTION TO PURGE EXPIRED BIN RECORDS (> 30 DAYS)
create or replace function purge_expired_bin_records()
returns table (
  purged_hampers int,
  purged_products int,
  purged_images int
)
language plpgsql
security definer
set search_path = public
as $$
declare
  v_hampers int := 0;
  v_products int := 0;
  v_images int := 0;
begin
  -- Delete expired product images (> 30 days)
  delete from product_images
  where deleted_at is not null and deleted_at < now() - interval '30 days';
  get diagnostics v_images = row_count;

  -- Delete expired products (> 30 days)
  delete from products
  where deleted_at is not null and deleted_at < now() - interval '30 days';
  get diagnostics v_products = row_count;

  -- Delete expired hampers (> 30 days)
  delete from hampers
  where deleted_at is not null and deleted_at < now() - interval '30 days';
  get diagnostics v_hampers = row_count;

  return query select v_hampers, v_products, v_images;
end;
$$;
