-- 0027_realtime.sql
-- Live refresh (src/components/live-refresh.tsx): changes to these tables are
-- pushed to every open page, which then re-fetches. Realtime checks each row
-- against the reader's RLS, so nobody is sent a row they can't already read.
--
-- Left out: product_cost_lines (deleted and re-inserted on every save; the
-- product and cost sheet rows change in the same save), product_history,
-- profiles, doc_counters.

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
    if not exists (select 1 from pg_publication_tables
                   where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = t) then
      execute format('alter publication supabase_realtime add table public.%I', t);
    end if;
  end loop;
end
$$;
