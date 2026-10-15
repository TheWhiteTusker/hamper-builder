import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";
import { supabaseEnv, SupabaseConfigError } from "@/lib/supabase/env";

/**
 * Refreshes the auth session on every request and keeps signed-out users out
 * of the app. Pages still call requireUser(); this just avoids rendering them
 * at all, and stops the session cookie from expiring mid-session.
 */
export async function proxy(request: NextRequest) {
  let response = NextResponse.next({ request });

  // This runs on every request, so a missing build-time variable takes down
  // the whole site rather than one route. Answer with something that says so:
  // the detail goes to the logs, not to whoever is looking at the page.
  let url: string;
  let key: string;

  try {
    ({ url, key } = supabaseEnv());
  } catch (error) {
    if (error instanceof SupabaseConfigError) {
      console.error(error.message);
      return new NextResponse("This deployment is missing its configuration.", {
        status: 503,
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }
    throw error;
  }

  const supabase = createServerClient(url, key, {
    cookies: {
        getAll: () => request.cookies.getAll(),
        setAll(cookiesToSet) {
          for (const { name, value } of cookiesToSet) {
            request.cookies.set(name, value);
          }
          response = NextResponse.next({ request });
          for (const { name, value, options } of cookiesToSet) {
            response.cookies.set(name, value, options);
          }
        },
    },
  });

  // getClaims() still refreshes an expired session (and writes the cookies),
  // but verifies the JWT locally instead of calling Supabase Auth each request.
  const { data } = await supabase.auth.getClaims();
  const user = data?.claims;

  const path = request.nextUrl.pathname;
  const isPublic =
    path === "/" ||
    path.startsWith("/login") ||
    path.startsWith("/auth") ||
    path.startsWith("/updates");

  if (!user && !isPublic) {
    const url = request.nextUrl.clone();
    url.pathname = "/login";
    url.searchParams.set("next", path);
    return NextResponse.redirect(url);
  }

  if (user && path.startsWith("/login")) {
    const url = request.nextUrl.clone();
    url.pathname = "/dashboard";
    url.search = "";
    return NextResponse.redirect(url);
  }

  return response;
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|models/|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)"],
};
