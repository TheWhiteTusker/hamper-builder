-- RLS performance pass, following Supabase's own guidance.
--
-- Two problems with 0002, both of which only bite once there is real data:
--
-- 1. A bare auth.uid() or is_manager() in a policy is evaluated ONCE PER ROW.
--    Wrapping it in a scalar subquery turns it into an InitPlan that runs once
--    per statement. Supabase measures this at 5-100x on large tables.
--
-- 2. quotes.created_by is read by every quotes policy and by the quote_items
--    subquery, but had no index, so each check was a sequential scan.
--
-- Written to be safe to run twice, and safe to run whether or not 0002 has
-- already been applied.

-- ---------------------------------------------------------------
-- INDEXES BEHIND THE POLICIES
-- ---------------------------------------------------------------

-- Postgres does not index foreign keys automatically. These two are read by
-- RLS on every quote and hamper query, and by ON DELETE SET NULL when a user
-- is removed.
create index if not exists quotes_created_by_idx  on quotes (created_by);
create index if not exists hampers_created_by_idx on hampers (created_by);

-- ---------------------------------------------------------------
-- PROFILES
-- ---------------------------------------------------------------

drop policy if exists profiles_update on profiles;
create policy profiles_update on profiles
  for update to authenticated
  using (id = (select auth.uid()) or (select is_admin()))
  with check (id = (select auth.uid()) or (select is_admin()));

drop policy if exists profiles_delete on profiles;
create policy profiles_delete on profiles
  for delete to authenticated
  using ((select is_admin()));

-- ---------------------------------------------------------------
-- ADMIN-ONLY REFERENCE DATA
-- ---------------------------------------------------------------

drop policy if exists categories_write on categories;
create policy categories_write on categories
  for all to authenticated
  using ((select is_admin()))
  with check ((select is_admin()));

drop policy if exists app_settings_write on app_settings;
create policy app_settings_write on app_settings
  for all to authenticated
  using ((select is_admin()))
  with check ((select is_admin()));

drop policy if exists products_write on products;
create policy products_write on products
  for all to authenticated
  using ((select is_admin()))
  with check ((select is_admin()));

-- ---------------------------------------------------------------
-- HAMPERS
-- ---------------------------------------------------------------

drop policy if exists hampers_write on hampers;
create policy hampers_write on hampers
  for all to authenticated
  using ((select is_manager()))
  with check ((select is_manager()));

drop policy if exists hamper_items_write on hamper_items;
create policy hamper_items_write on hamper_items
  for all to authenticated
  using ((select is_manager()))
  with check ((select is_manager()));

-- ---------------------------------------------------------------
-- QUOTES
-- ---------------------------------------------------------------

drop policy if exists quotes_insert on quotes;
create policy quotes_insert on quotes
  for insert to authenticated
  with check (created_by = (select auth.uid()));

drop policy if exists quotes_update on quotes;
create policy quotes_update on quotes
  for update to authenticated
  using (created_by = (select auth.uid()) or (select is_manager()))
  with check (created_by = (select auth.uid()) or (select is_manager()));

drop policy if exists quotes_delete on quotes;
create policy quotes_delete on quotes
  for delete to authenticated
  using ((select is_manager()));

drop policy if exists quote_items_write on quote_items;
create policy quote_items_write on quote_items
  for all to authenticated
  using (exists (
    select 1 from quotes q
    where q.id = quote_items.quote_id
      and (q.created_by = (select auth.uid()) or (select is_manager()))
  ))
  with check (exists (
    select 1 from quotes q
    where q.id = quote_items.quote_id
      and (q.created_by = (select auth.uid()) or (select is_manager()))
  ));
