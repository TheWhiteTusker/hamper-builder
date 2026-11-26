-- 0015_hamper_summary_image.sql
-- Expose the designed image on the summary view so the hampers list can show
-- a thumbnail. Same definition as 0001_schema.sql; the new column goes last,
-- which is the only change `create or replace view` allows.

create or replace view hamper_summary with (security_invoker = on) as
select
  h.id,
  h.code,
  h.name,
  h.collection,
  h.status,
  t.number_of_items,
  t.total_cp,
  t.base_sp,
  h.discount_pct,
  t.base_sp * (1 - h.discount_pct) as sp_after_discount,
  h.target_sp,
  case when h.target_sp is null then null
       else t.base_sp * (1 - h.discount_pct) - h.target_sp
  end as variance,
  h.final_catalogue_sp,
  case when h.final_catalogue_sp is null then null
       else h.final_catalogue_sp - t.total_cp
  end as gross_profit,
  case when coalesce(h.final_catalogue_sp, 0) = 0 then null
       else (h.final_catalogue_sp - t.total_cp) / h.final_catalogue_sp
  end as final_margin,
  h.notes,
  h.created_by,
  h.created_at,
  h.updated_at,
  t.line_count,
  h.image_url
from hampers h
join hamper_totals t on t.hamper_id = h.id;
