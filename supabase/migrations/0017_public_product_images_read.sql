-- 0017_public_product_images_read.sql
-- Allow unauthenticated (anon) visitors to view active product images on the landing page

drop policy if exists "products_public_read" on products;
create policy "products_public_read" on products
  for select
  to anon
  using (is_active = true and image_url is not null);

drop policy if exists "product_images_public_read" on product_images;
create policy "product_images_public_read" on product_images
  for select
  to anon
  using (true);
