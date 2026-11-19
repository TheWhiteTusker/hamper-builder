-- 0011_product_images_and_storage.sql
-- Add image_url to products, create product_images table, and setup storage bucket

-- ---------------------------------------------------------------
-- 1. ADD IMAGE_URL TO PRODUCTS
-- ---------------------------------------------------------------

alter table products add column if not exists image_url text;

-- ---------------------------------------------------------------
-- 2. PRODUCT IMAGES TABLE
-- ---------------------------------------------------------------

create table if not exists product_images (
  id           uuid primary key default gen_random_uuid(),
  product_id   uuid not null references products(id) on delete cascade,
  url          text not null,
  storage_path text,
  color        text, -- 'Walnut', 'Natural', 'Black', or null for All/General
  color_code   text, -- 'WL', 'NT', 'BL', or null
  is_primary   boolean not null default false,
  sort_order   int not null default 0,
  caption      text,
  created_at   timestamptz not null default now(),
  updated_at   timestamptz not null default now()
);

create index if not exists product_images_product_idx on product_images (product_id);
create index if not exists product_images_color_idx on product_images (product_id, color);
create index if not exists product_images_primary_idx on product_images (product_id, is_primary);

drop trigger if exists product_images_touch on product_images;
create trigger product_images_touch before update on product_images
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 3. RLS POLICIES FOR PRODUCT IMAGES
-- ---------------------------------------------------------------

alter table product_images enable row level security;

drop policy if exists "product_images_read" on product_images;
create policy "product_images_read" on product_images
  for select
  to authenticated
  using (true);

drop policy if exists "product_images_write" on product_images;
create policy "product_images_write" on product_images
  for all
  to authenticated
  using (true)
  with check (true);

-- ---------------------------------------------------------------
-- 4. STORAGE BUCKET: product-images
-- ---------------------------------------------------------------

insert into storage.buckets (id, name, public)
values ('product-images', 'product-images', true)
on conflict (id) do update set public = true;

-- Storage object policies for product-images bucket
drop policy if exists "product_images_storage_read" on storage.objects;
create policy "product_images_storage_read" on storage.objects
  for select
  to public
  using (bucket_id = 'product-images');

drop policy if exists "product_images_storage_insert" on storage.objects;
create policy "product_images_storage_insert" on storage.objects
  for insert
  to authenticated
  with check (bucket_id = 'product-images');

drop policy if exists "product_images_storage_update" on storage.objects;
create policy "product_images_storage_update" on storage.objects
  for update
  to authenticated
  using (bucket_id = 'product-images');

drop policy if exists "product_images_storage_delete" on storage.objects;
create policy "product_images_storage_delete" on storage.objects
  for delete
  to authenticated
  using (bucket_id = 'product-images');
