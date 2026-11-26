-- 0013_clients_bank_and_hybrid.sql
-- Client master (picked on quotations / proforma invoices), bank details and
-- proforma terms for the PI format, and "Hybrid" as a product source.
-- Safe to run twice.

-- ---------------------------------------------------------------
-- CLIENTS
-- ---------------------------------------------------------------

-- Quotes copy these fields in when a client is picked, so a sent document
-- keeps the address it was sent with. No FK from quotes on purpose.
create table if not exists clients (
  id              uuid primary key default gen_random_uuid(),
  name            text not null,
  gstin           text unique,
  billing_address text,
  contact_person  text,
  phone           text,
  email           text,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now()
);

create index if not exists clients_name_idx on clients (lower(name));

drop trigger if exists clients_touch on clients;
create trigger clients_touch before update on clients
  for each row execute function touch_updated_at();

alter table clients enable row level security;

-- Sales create quotes, so everyone signed in may add and edit clients.
-- Deleting stays with managers, like quotes.
drop policy if exists clients_read on clients;
create policy clients_read on clients
  for select to authenticated using (true);

drop policy if exists clients_insert on clients;
create policy clients_insert on clients
  for insert to authenticated with check (true);

drop policy if exists clients_update on clients;
create policy clients_update on clients
  for update to authenticated using (true) with check (true);

drop policy if exists clients_delete on clients;
create policy clients_delete on clients
  for delete to authenticated using ((select is_manager()));

grant select, insert, update, delete on clients to authenticated;

-- ---------------------------------------------------------------
-- SETTINGS
-- ---------------------------------------------------------------

-- Legal entity + bank details for the proforma invoice. Existing keys win.
update app_settings
   set value = jsonb_build_object(
         'legal_name',        'Palm Length LLP',
         'bank_account_name', 'Palm Length LLP',
         'bank_name',         'HDFC Bank',
         'bank_account_no',   '50200082351809',
         'bank_ifsc',         'HDFC0001298'
       ) || value
 where key = 'company';

insert into app_settings (key, value) values
  ('pi_terms', to_jsonb($$1. Advance of 80% on confirmation, balance 20% on delivery.
2. Production shall commence after the receipt of Purchase Order with above mentioned Advance & finalization of design.
3. Delivery time will be 6 to 7 working days from the date of Advance Received & Design finalization, and 15 working days for Shipping.
4. QC by Client at 'Lattice Lane' before dispatchment.
5. Our responsibility ceases once the goods have been dispatched from Lattice Lane.$$::text))
on conflict (key) do nothing;

insert into app_settings (key, value) values
  ('sources', '["In-house", "Outsourced", "Hybrid"]'::jsonb)
on conflict (key) do nothing;

update app_settings
   set value = value || '["Hybrid"]'::jsonb
 where key = 'sources' and not value ? 'Hybrid';
