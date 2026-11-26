-- 0027_realtime.sql
-- Live refresh (src/components/live-refresh.tsx): changes to these tables are
-- pushed to every open page, which then re-fetches. Realtime checks each row
-- against the reader's RLS, so nobody is sent a row they can't already read.
--
-- Left out: product_cost_lines (deleted and re-inserted on every save; the
-- product and cost sheet rows change in the same save), product_history,
-- profiles, doc_counters.
--
-- Each of these tables also gets updated_by: who last wrote the row, stamped
-- by trigger so every save path is caught. Live refresh skips your own saves,
-- which the saving page has already shown. Cron and service-role writes stamp
-- null, so everyone refreshes for those. No FK: it's only compared, never
-- joined; product_history records who did what.

create or replace function stamp_updated_by()
returns trigger language plpgsql set search_path = public as $fn$
begin
  new.updated_by := auth.uid();
  return new;
end
$fn$;

do $$
declare
  t text;
begin
  foreach t in array array[
    'products', 'product_cost_sheets', 'product_images', 'categories',
    'hampers', 'hamper_items', 'quotes', 'quote_items', 'clients',
    'presentations', 'presentation_slides', 'app_settings',
    'cost_stages', 'cost_categories', 'cost_subcategories', 'cost_items', 'cost_varieties'
  ] loop
    execute format('alter table public.%I add column if not exists updated_by uuid', t);
    execute format('drop trigger if exists stamp_updated_by on public.%I', t);
    execute format('create trigger stamp_updated_by before insert or update on public.%I
                    for each row execute function stamp_updated_by()', t);
    if not exists (select 1 from pg_publication_tables
                   where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = t) then
      execute format('alter publication supabase_realtime add table public.%I', t);
    end if;
  end loop;
end
$$;
