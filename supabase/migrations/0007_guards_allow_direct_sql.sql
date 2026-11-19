-- The two guard triggers locked out database administration.
--
-- Both called is_admin() / is_manager(), which resolve through auth.uid().
-- Over the API that is exactly right: a sales user must not promote themselves
-- or move a quotation's status. But auth.uid() is NULL for anything that is
-- not an API request - the SQL editor, a service-role script, a migration -
-- so those contexts were refused too:
--
--   ERROR: Only an admin can change a user role
--
-- That is a lockout risk rather than a safeguard. If the only admin leaves,
-- nobody can promote a replacement through the app, and the trigger blocked
-- fixing it directly as well. A direct SQL connection is already fully
-- privileged, so the trigger was protecting nothing there.
--
-- Both guards now apply only when there IS an authenticated caller. The
-- security property over the API is unchanged: auth.uid() is always set for a
-- PostgREST request, so an API caller must still be admin (or manager).

create or replace function enforce_role_change() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  if new.role is distinct from old.role
     and auth.uid() is not null
     and not is_admin() then
    raise exception 'Only an admin can change a user role';
  end if;
  return new;
end $fn$;

create or replace function enforce_quote_status_change() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  if new.status is distinct from old.status
     and auth.uid() is not null
     and not is_manager() then
    raise exception 'Only a manager can change quote status';
  end if;
  return new;
end $fn$;
