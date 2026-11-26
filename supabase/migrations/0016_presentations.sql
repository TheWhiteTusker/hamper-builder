-- 0016_presentations.sql
-- Client presentations: an entry slide, a slide per hamper or product, and a
-- closing slide. Each slide is a designer canvas (see src/lib/hamper-canvas.ts)
-- edited in the same photo editor as hamper images.

create table if not exists presentations (
  id          uuid primary key default gen_random_uuid(),
  title       text not null,
  created_by  uuid references profiles(id) on delete set null default auth.uid(),
  created_at  timestamptz not null default now(),
  updated_at  timestamptz not null default now()
);

drop trigger if exists presentations_touch on presentations;
create trigger presentations_touch before update on presentations
  for each row execute function touch_updated_at();

-- No unique (presentation_id, position): reordering swaps two rows in two
-- statements, which a unique constraint would reject halfway through.
create table if not exists presentation_slides (
  id               uuid primary key default gen_random_uuid(),
  presentation_id  uuid not null references presentations(id) on delete cascade,
  position         int not null,
  kind             text not null check (kind in ('cover', 'hamper', 'product', 'closing', 'blank')),
  hamper_id        uuid references hampers(id) on delete set null,
  product_id       uuid references products(id) on delete set null,
  canvas           jsonb not null,
  created_at       timestamptz not null default now(),
  updated_at       timestamptz not null default now()
);

create index if not exists presentation_slides_order_idx on presentation_slides (presentation_id, position);
create index if not exists presentation_slides_hamper_idx on presentation_slides (hamper_id);
create index if not exists presentation_slides_product_idx on presentation_slides (product_id);

drop trigger if exists presentation_slides_touch on presentation_slides;
create trigger presentation_slides_touch before update on presentation_slides
  for each row execute function touch_updated_at();

-- Sales material: every signed-in user may build and edit presentations.
alter table presentations enable row level security;
alter table presentation_slides enable row level security;

drop policy if exists presentations_all on presentations;
create policy presentations_all on presentations
  for all to authenticated using (true) with check (true);

drop policy if exists presentation_slides_all on presentation_slides;
create policy presentation_slides_all on presentation_slides
  for all to authenticated using (true) with check (true);

grant select, insert, update, delete on presentations, presentation_slides to authenticated;
