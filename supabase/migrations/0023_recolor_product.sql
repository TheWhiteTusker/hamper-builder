-- 0023_recolor_product.sql
-- Changes one product's colour, and with it the colour suffix of its code
-- (ED/0006/WL -> ED/0006/NE). Every product is a single colour, so the code
-- has to follow. Everything that stores the code is updated in the same
-- transaction: the cost sheet and the hamper lines that snapshot it. Photos
-- tagged with the old colour move to the new one. Quotes only reference hamper
-- codes, so they are unaffected.

create or replace function recolor_product(p_id uuid, p_color text, p_color_code text)
returns text
language plpgsql
security invoker            -- RLS still applies: products are admin-only
set search_path = public
as $$
declare
  v_old  products%rowtype;
  v_code text;
  v_old_color text;
begin
  -- auth.uid() is null only for the database owner (scripts); the app must be an admin.
  if auth.uid() is not null and not is_admin() then
    raise exception 'Only admins can change a product''s colour';
  end if;

  select * into v_old from products where id = p_id for update;
  if not found then raise exception 'Product not found'; end if;

  p_color_code := upper(trim(p_color_code));
  if p_color_code !~ '^[A-Z0-9]{2}$' then raise exception 'Colour code must be 2 letters'; end if;

  -- CAT/SERIAL/XX -> CAT/SERIAL/NEW; a code without a suffix gets one.
  v_code := case
    when v_old.code ~ '^[^/]+/[^/]+/[^/]+$' then regexp_replace(v_old.code, '/[^/]+$', '/' || p_color_code)
    else v_old.code || '/' || p_color_code
  end;
  v_old_color := coalesce(v_old.colors[1], '');

  if v_code <> v_old.code and exists (select 1 from products where code = v_code) then
    raise exception 'Product % already exists (it may be in the Recycle Bin)', v_code;
  end if;

  update products set code = v_code, colors = array[p_color] where id = p_id;
  update product_cost_sheets set product_code = v_code where product_id = p_id;
  update hamper_items set product_code = v_code where product_id = p_id;
  update product_images
     set color = p_color, color_code = p_color_code
   where product_id = p_id
     and (color_code = split_part(v_old.code, '/', 3) or color = v_old_color);

  return v_code;
end $$;

revoke all on function recolor_product(uuid, text, text) from public, anon;
grant execute on function recolor_product(uuid, text, text) to authenticated;
