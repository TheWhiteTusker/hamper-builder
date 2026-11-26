-- 0019_landing_photos_function.sql
-- 0017 let logged-out visitors (anon) select from products and product_images
-- directly. RLS filters rows, not columns, so that also exposed cost prices,
-- markups and margins. Replace both policies with one function that returns
-- only what the landing page shows: photo, name and a label.

drop policy if exists "products_public_read" on public.products;
drop policy if exists "product_images_public_read" on public.product_images;

create or replace function public.landing_photos()
returns table (id uuid, url text, name text, category text)
language sql
stable
security definer
set search_path = ''
as $$
  (select p.id, p.image_url, p.name, coalesce(p.code, 'Catalog Product')
     from public.products p
    where p.is_active and p.image_url is not null
    order by p.code
    limit 40)
  union all
  (select i.id, i.url, coalesce(i.caption, 'Curated Item'), 'Product Gallery'
     from public.product_images i
     join public.products p on p.id = i.product_id
    where p.is_active and i.url is not null
    order by i.created_at
    limit 40)
$$;

revoke all on function public.landing_photos() from public;
grant execute on function public.landing_photos() to anon, authenticated;
