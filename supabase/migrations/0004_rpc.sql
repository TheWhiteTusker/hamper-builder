-- Saving a hamper or a quote replaces all of its lines. The Apps Script did
-- that as delete-then-insert across separate calls, so a failure halfway
-- through left a record with no lines. These run inside one transaction.
--
-- All of them are SECURITY INVOKER, so the RLS policies in 0002 still decide
-- who is allowed to write.

-- ---------------------------------------------------------------
-- SAVE HAMPER
-- ---------------------------------------------------------------

create function save_hamper(p jsonb) returns text
language plpgsql security invoker set search_path = public as $fn$
declare
  v_id   uuid := nullif(p ->> 'id', '')::uuid;
  v_code text;
  line   jsonb;
  i      int := 0;
begin
  if v_id is null then
    -- New hamper: take the next code under a row lock (see next_doc_no).
    v_code := coalesce(nullif(p ->> 'code', ''), next_doc_no('H'));

    insert into hampers (
      code, name, collection, status, target_sp, notes,
      discount_pct, final_catalogue_sp, created_by
    ) values (
      v_code,
      p ->> 'name',
      nullif(p ->> 'collection', ''),
      coalesce(nullif(p ->> 'status', ''), 'Draft'),
      nullif(p ->> 'target_sp', '')::numeric,
      nullif(p ->> 'notes', ''),
      coalesce(nullif(p ->> 'discount_pct', '')::numeric, 0),
      nullif(p ->> 'final_catalogue_sp', '')::numeric,
      auth.uid()
    )
    returning id, code into v_id, v_code;
  else
    update hampers set
      name               = p ->> 'name',
      collection         = nullif(p ->> 'collection', ''),
      status             = coalesce(nullif(p ->> 'status', ''), status),
      target_sp          = nullif(p ->> 'target_sp', '')::numeric,
      notes              = nullif(p ->> 'notes', ''),
      discount_pct       = coalesce(nullif(p ->> 'discount_pct', '')::numeric, 0),
      final_catalogue_sp = nullif(p ->> 'final_catalogue_sp', '')::numeric
    where id = v_id
    returning code into v_code;

    if v_code is null then
      raise exception 'Hamper not found, or you do not have permission to edit it';
    end if;

    delete from hamper_items where hamper_id = v_id;
  end if;

  for line in select * from jsonb_array_elements(coalesce(p -> 'lines', '[]'::jsonb)) loop
    i := i + 1;

    insert into hamper_items (
      hamper_id, line_no, product_id, product_code, product_name,
      category_name, source, qty, unit_cp, unit_sp, target_margin
    ) values (
      v_id,
      i,
      nullif(line ->> 'product_id', '')::uuid,
      nullif(line ->> 'product_code', ''),
      line ->> 'product_name',
      nullif(line ->> 'category_name', ''),
      nullif(line ->> 'source', ''),
      coalesce(nullif(line ->> 'qty', '')::numeric, 0),
      coalesce(nullif(line ->> 'unit_cp', '')::numeric, 0),
      coalesce(nullif(line ->> 'unit_sp', '')::numeric, 0),
      coalesce(nullif(line ->> 'target_margin', '')::numeric, 0)
    );
  end loop;

  return v_code;
end $fn$;

-- ---------------------------------------------------------------
-- DUPLICATE HAMPER
-- ---------------------------------------------------------------

-- duplicateHamper() in the Apps Script only renamed the builder and waited for
-- a manual save. Here the copy is a real record straight away.
create function duplicate_hamper(p_code text) returns text
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
    discount_pct, final_catalogue_sp, created_by
  ) values (
    v_code,
    v_src.name || ' - Copy',
    v_src.collection,
    'Draft',                      -- a copy always starts as a draft
    v_src.target_sp,
    v_src.notes,
    v_src.discount_pct,
    v_src.final_catalogue_sp,
    auth.uid()
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

-- ---------------------------------------------------------------
-- SAVE QUOTE
-- ---------------------------------------------------------------

create function save_quote(p jsonb) returns text
language plpgsql security invoker set search_path = public as $fn$
declare
  v_id     uuid := nullif(p ->> 'id', '')::uuid;
  v_type   doc_type := coalesce(nullif(p ->> 'doc_type', ''), 'quotation')::doc_type;
  v_no     text;
  v_prefix text;
  line     jsonb;
  i        int := 0;
begin
  if v_id is null then
    -- Prefix comes from settings so LLQT-/LLPI- stay configurable.
    select coalesce(value ->> v_type::text, case when v_type = 'proforma_invoice'
                                                 then 'LLPI-' else 'LLQT-' end)
      into v_prefix
    from app_settings where key = 'doc_prefixes';

    v_no := coalesce(nullif(p ->> 'doc_no', ''), next_doc_no(coalesce(v_prefix, 'LLQT-')));

    insert into quotes (
      doc_no, doc_type, doc_date, client_name, contact_person, phone, email,
      billing_address, gstin, occasion, quote_structure, validity, status,
      order_discount, adj1, adj2, gst_rate, notes, terms, follow_up_date,
      linked_doc_no, created_by
    ) values (
      v_no, v_type,
      coalesce(nullif(p ->> 'doc_date', '')::date, current_date),
      p ->> 'client_name',
      nullif(p ->> 'contact_person', ''),
      nullif(p ->> 'phone', ''),
      nullif(p ->> 'email', ''),
      nullif(p ->> 'billing_address', ''),
      nullif(p ->> 'gstin', ''),
      nullif(p ->> 'occasion', ''),
      coalesce(nullif(p ->> 'quote_structure', ''), 'Combined Order'),
      nullif(p ->> 'validity', ''),
      coalesce(nullif(p ->> 'status', ''), 'Draft'),
      coalesce(nullif(p ->> 'order_discount', '')::numeric, 0),
      coalesce(nullif(p ->> 'adj1', '')::numeric, 0),
      coalesce(nullif(p ->> 'adj2', '')::numeric, 0),
      coalesce(nullif(p ->> 'gst_rate', '')::numeric, 0.18),
      nullif(p ->> 'notes', ''),
      nullif(p ->> 'terms', ''),
      nullif(p ->> 'follow_up_date', '')::date,
      nullif(p ->> 'linked_doc_no', ''),
      auth.uid()
    )
    returning id, doc_no into v_id, v_no;
  else
    update quotes set
      doc_type        = v_type,
      doc_date        = coalesce(nullif(p ->> 'doc_date', '')::date, doc_date),
      client_name     = p ->> 'client_name',
      contact_person  = nullif(p ->> 'contact_person', ''),
      phone           = nullif(p ->> 'phone', ''),
      email           = nullif(p ->> 'email', ''),
      billing_address = nullif(p ->> 'billing_address', ''),
      gstin           = nullif(p ->> 'gstin', ''),
      occasion        = nullif(p ->> 'occasion', ''),
      quote_structure = coalesce(nullif(p ->> 'quote_structure', ''), quote_structure),
      validity        = nullif(p ->> 'validity', ''),
      status          = coalesce(nullif(p ->> 'status', ''), status),
      order_discount  = coalesce(nullif(p ->> 'order_discount', '')::numeric, 0),
      adj1            = coalesce(nullif(p ->> 'adj1', '')::numeric, 0),
      adj2            = coalesce(nullif(p ->> 'adj2', '')::numeric, 0),
      gst_rate        = coalesce(nullif(p ->> 'gst_rate', '')::numeric, gst_rate),
      notes           = nullif(p ->> 'notes', ''),
      terms           = nullif(p ->> 'terms', ''),
      follow_up_date  = nullif(p ->> 'follow_up_date', '')::date,
      linked_doc_no   = nullif(p ->> 'linked_doc_no', '')
    where id = v_id
    returning doc_no into v_no;

    if v_no is null then
      raise exception 'Quotation not found, or you do not have permission to edit it';
    end if;

    delete from quote_items where quote_id = v_id;
  end if;

  for line in select * from jsonb_array_elements(coalesce(p -> 'lines', '[]'::jsonb)) loop
    i := i + 1;

    insert into quote_items (
      quote_id, line_no, option_label, hamper_id, hamper_code, hamper_name,
      qty, catalogue_price, discount_pct, detail_mode, packaging_treatment
    ) values (
      v_id,
      i,
      nullif(line ->> 'option_label', ''),
      nullif(line ->> 'hamper_id', '')::uuid,
      nullif(line ->> 'hamper_code', ''),
      nullif(line ->> 'hamper_name', ''),
      coalesce(nullif(line ->> 'qty', '')::numeric, 0),
      coalesce(nullif(line ->> 'catalogue_price', '')::numeric, 0),
      coalesce(nullif(line ->> 'discount_pct', '')::numeric, 0),
      nullif(line ->> 'detail_mode', ''),
      nullif(line ->> 'packaging_treatment', '')
    );
  end loop;

  return v_no;
end $fn$;

-- ---------------------------------------------------------------
-- REFRESH HAMPER PRICES
-- ---------------------------------------------------------------

-- The surviving half of refreshHamperPrices(): re-snapshot hamper lines from
-- the current Product Master. Summary totals need no recalculation, because
-- hamper_summary derives them. Quotations are deliberately left alone, exactly
-- as the original function promised.
--
-- Call with p_apply => false for a preview of what would change.
create function refresh_hamper_prices(p_apply boolean default false)
returns table (
  hamper_code  text,
  line_no      int,
  product_code text,
  product_name text,
  old_cp       numeric,
  new_cp       numeric,
  old_sp       numeric,
  new_sp       numeric,
  note         text
)
language plpgsql security invoker set search_path = public as $fn$
begin
  drop table if exists _refresh_diff;

  create temp table _refresh_diff on commit drop as
  select
    h.code            as hamper_code,
    hi.id             as item_id,
    hi.line_no        as line_no,
    hi.product_code   as product_code,
    hi.product_name   as product_name,
    hi.unit_cp        as old_cp,
    hi.unit_sp        as old_sp,
    p.id              as new_product_id,
    p.name            as new_name,
    c.name            as new_category,
    p.source          as new_source,
    p.cost_price      as new_cp,
    p.default_sp      as new_sp,
    p.target_margin   as new_margin,
    case when p.id is null then 'missing' else 'changed' end as note
  from hamper_items hi
  join hampers h on h.id = hi.hamper_id
  left join products p on p.code = hi.product_code
  left join categories c on c.id = p.category_id
  where p.id is null
     or hi.unit_cp       is distinct from p.cost_price
     or hi.unit_sp       is distinct from p.default_sp
     or hi.target_margin is distinct from p.target_margin
     or hi.source        is distinct from p.source
     or hi.product_name  is distinct from p.name
     or hi.category_name is distinct from c.name;

  if p_apply then
    update hamper_items hi set
      product_id    = d.new_product_id,
      product_name  = d.new_name,
      category_name = d.new_category,
      source        = d.new_source,
      unit_cp       = d.new_cp,
      unit_sp       = d.new_sp,
      target_margin = d.new_margin
    from _refresh_diff d
    where hi.id = d.item_id
      and d.note = 'changed';
  end if;

  return query
    select d.hamper_code, d.line_no, d.product_code, d.product_name,
           d.old_cp, d.new_cp, d.old_sp, d.new_sp, d.note
    from _refresh_diff d
    order by d.hamper_code, d.line_no;
end $fn$;

grant execute on function save_hamper(jsonb)              to authenticated;
grant execute on function duplicate_hamper(text)          to authenticated;
grant execute on function save_quote(jsonb)               to authenticated;
grant execute on function refresh_hamper_prices(boolean)  to authenticated;
