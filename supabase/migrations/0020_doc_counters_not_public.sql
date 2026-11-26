-- 0020_doc_counters_not_public.sql
-- 0002 granted these to authenticated, but Postgres also grants EXECUTE on
-- every new function to PUBLIC, so logged-out visitors could call them and
-- burn quotation / hamper numbers. Signed-in users keep their grant.

revoke execute on function public.next_doc_no(text, int) from public, anon;
revoke execute on function public.sync_doc_counters() from public, anon;
grant execute on function public.next_doc_no(text, int) to authenticated;
grant execute on function public.sync_doc_counters() to authenticated;
