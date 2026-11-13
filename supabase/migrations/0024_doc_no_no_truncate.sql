-- 0024_doc_no_no_truncate.sql
-- next_doc_no padded with lpad(n, 3), but Postgres lpad also TRUNCATES text
-- longer than the length: lpad('1000', 3, '0') = '100'. The 1000th hamper
-- would have been H100 again, and every new hamper / quote / PI save after
-- that would fail on the unique code. Pad only up to p_pad, never cut.
-- Same signature, so the grants from 0020 still apply.

create or replace function next_doc_no(p_prefix text, p_pad int default 3)
returns text
language plpgsql
security definer
set search_path = public
as $$
declare n int;
begin
  insert into doc_counters (prefix, last_number)
  values (p_prefix, 0)
  on conflict (prefix) do nothing;

  update doc_counters
     set last_number = last_number + 1
   where prefix = p_prefix
  returning last_number into n;

  return p_prefix || lpad(n::text, greatest(p_pad, length(n::text)), '0');
end $$;
