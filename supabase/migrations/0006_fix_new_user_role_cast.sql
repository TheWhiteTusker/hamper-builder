-- Creating any user failed with "Database error creating new user".
--
-- handle_new_user picked the role with:
--
--   case when (select count(*) from public.profiles) = 0
--        then 'admin' else 'sales' end
--
-- A bare literal like 'admin' is type `unknown` and coerces to an enum on
-- insert, but CASE resolves its branches to `text`, and Postgres has no
-- implicit text -> enum cast. So the insert failed, the trigger aborted, and
-- the auth.users insert rolled back with it.
--
-- The cast is the whole fix; the logic is unchanged. Still: first account in
-- owns the workspace, everyone after starts as sales.

create or replace function handle_new_user() returns trigger
language plpgsql security definer set search_path = public as $fn$
begin
  insert into public.profiles (id, full_name, role)
  values (
    new.id,
    coalesce(new.raw_user_meta_data ->> 'full_name', new.email),
    (case when (select count(*) from public.profiles) = 0 then 'admin' else 'sales' end)::user_role
  );
  return new;
end $fn$;
