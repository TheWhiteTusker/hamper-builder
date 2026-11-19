-- 0014_hamper_canvas.sql
-- A designed picture per hamper: the editable layout (canvas) and the PNG
-- rendered from it (image_url). save_hamper never touches either column, so
-- editing the line items keeps the design.

alter table hampers add column if not exists canvas    jsonb;
alter table hampers add column if not exists image_url text;

-- Same as 0004_rpc.sql, plus the design. The copy shares the PNG until it is
-- re-saved, which then writes its own file under hampers/{new id}/.
create or replace function duplicate_hamper(p_code text) returns text
language plpgsql security invoker set search_path = public as $fn$
declare
  v_src  hampers%rowtype;
  v_id   uuid;
  v_code text;
begin
  select * into v_src from hampers where code = p_code;
  if not found then
    raise exception 'Hamper % not found', p_code;
  end if;

  v_code := next_doc_no('H');

  insert into hampers (
    code, name, collection, status, target_sp, notes,
    discount_pct, final_catalogue_sp, created_by, canvas, image_url
  ) values (
    v_code,
    v_src.name || ' - Copy',
    v_src.collection,
    'Draft',                      -- a copy always starts as a draft
    v_src.target_sp,
    v_src.notes,
    v_src.discount_pct,
    v_src.final_catalogue_sp,
    auth.uid(),
    v_src.canvas,
    v_src.image_url
  )
  returning id into v_id;

  insert into hamper_items (
    hamper_id, line_no, product_id, product_code, product_name,
    category_name, source, qty, unit_cp, unit_sp, target_margin
  )
  select v_id, line_no, product_id, product_code, product_name,
         category_name, source, qty, unit_cp, unit_sp, target_margin
  from hamper_items
  where hamper_id = v_src.id
  order by line_no;

  return v_code;
end $fn$;
