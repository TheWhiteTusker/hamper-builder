-- Row level security.
--
-- Everyone signed in can READ everything - the sheet worked that way and the
-- team needs to see costs to quote. Writes follow the role matrix:
--
--                                  sales   manager   admin
--   quotes: create, edit own          x        x        x
--   quotes: edit anyone's, status              x        x
--   quotes: delete                             x        x
--   hampers + items                            x        x
--   products, categories, settings                      x
--   users and roles                                     x

alter table profiles     enable row level security;
alter table categories   enable row level security;
alter table app_settings enable row level security;
alter table products     enable row level security;
alter table hampers      enable row level security;
alter table hamper_items enable row level security;
alter table quotes       enable row level security;
alter table quote_items  enable row level security;
alter table doc_counters enable row level security;

-- doc_counters gets no policies at all: it is reachable only through
-- next_doc_no(), which is security definer.

-- ---------------------------------------------------------------
-- PROFILES
-- ---------------------------------------------------------------

create policy profiles_read on profiles
  for select to authenticated using (true);

create policy profiles_update on profiles
  for update to authenticated
  using (id = auth.uid() or is_admin())
  with check (id = auth.uid() or is_admin());

create policy profiles_delete on profiles
  for delete to authenticated using (is_admin());

-- Anyone may edit their own name, but only an admin may change a role -
-- otherwise a sales user could simply promote themselves.
create function enforce_role_change() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  if new.role is distinct from old.role and not is_admin() then
    raise exception 'Only an admin can change a user role';
  end if;
  return new;
end $fn$;

create trigger profiles_role_guard before update on profiles
  for each row execute function enforce_role_change();

-- ---------------------------------------------------------------
-- ADMIN-ONLY REFERENCE DATA
-- ---------------------------------------------------------------

create policy categories_read on categories
  for select to authenticated using (true);
create policy categories_write on categories
  for all to authenticated using (is_admin()) with check (is_admin());

create policy app_settings_read on app_settings
  for select to authenticated using (true);
create policy app_settings_write on app_settings
  for all to authenticated using (is_admin()) with check (is_admin());

create policy products_read on products
  for select to authenticated using (true);
create policy products_write on products
  for all to authenticated using (is_admin()) with check (is_admin());

-- ---------------------------------------------------------------
-- HAMPERS  (manager and admin)
-- ---------------------------------------------------------------

create policy hampers_read on hampers
  for select to authenticated using (true);
create policy hampers_write on hampers
  for all to authenticated using (is_manager()) with check (is_manager());

create policy hamper_items_read on hamper_items
  for select to authenticated using (true);
create policy hamper_items_write on hamper_items
  for all to authenticated using (is_manager()) with check (is_manager());

-- ---------------------------------------------------------------
-- QUOTES  (sales may create and edit their own)
-- ---------------------------------------------------------------

create policy quotes_read on quotes
  for select to authenticated using (true);

create policy quotes_insert on quotes
  for insert to authenticated
  with check (created_by = auth.uid());

create policy quotes_update on quotes
  for update to authenticated
  using (created_by = auth.uid() or is_manager())
  with check (created_by = auth.uid() or is_manager());

create policy quotes_delete on quotes
  for delete to authenticated using (is_manager());

-- Status drives the pipeline (Draft -> Sent -> Won/Lost), so only a manager
-- may move it. Sales can still edit every other field on their own quote.
create function enforce_quote_status_change() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  if new.status is distinct from old.status and not is_manager() then
    raise exception 'Only a manager can change quote status';
  end if;
  return new;
end $fn$;

create trigger quotes_status_guard before update on quotes
  for each row execute function enforce_quote_status_change();

-- Lines inherit their parent quote's permission.
create policy quote_items_read on quote_items
  for select to authenticated using (true);

create policy quote_items_write on quote_items
  for all to authenticated
  using (exists (
    select 1 from quotes q
    where q.id = quote_items.quote_id
      and (q.created_by = auth.uid() or is_manager())
  ))
  with check (exists (
    select 1 from quotes q
    where q.id = quote_items.quote_id
      and (q.created_by = auth.uid() or is_manager())
  ));

-- ---------------------------------------------------------------
-- GRANTS
-- ---------------------------------------------------------------

grant usage on schema public to authenticated;

grant select, insert, update, delete on
  profiles, categories, app_settings, products,
  hampers, hamper_items, quotes, quote_items
to authenticated;

grant select on hamper_totals, hamper_summary, quote_summary to authenticated;

grant execute on function next_doc_no(text, int) to authenticated;
grant execute on function app_role() to authenticated;
grant execute on function is_admin() to authenticated;
grant execute on function is_manager() to authenticated;
grant execute on function sync_doc_counters() to authenticated;

-- Nothing is readable without signing in.
revoke all on schema public from anon;
