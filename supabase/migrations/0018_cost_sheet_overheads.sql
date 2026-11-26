-- 0018_cost_sheet_overheads.sql
-- Overhead % per cost stage on a product's cost sheet, e.g. {"material": 10}.
-- Each stage's subtotal (and so material_total etc.) already includes it;
-- this keeps the percentages so the calculator can show and edit them.
alter table public.product_cost_sheets
  add column if not exists stage_overheads jsonb not null default '{}'::jsonb;
