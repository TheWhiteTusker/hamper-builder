import Link from "@/components/link";
import { requireUser } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";

type Row = {
  id: string;
  title: string;
  updated_at: string;
  presentation_slides: { count: number }[];
};

export default async function PresentationsPage() {
  const { supabase } = await requireUser();
  const { data } = await supabase
    .from("presentations")
    .select("id, title, updated_at, presentation_slides(count)")
    .order("updated_at", { ascending: false })
    .returns<Row[]>();
  const rows = data ?? [];

  return (
    <>
      <PageHeader title="Presentations" subtitle="Client decks: a cover, a slide per hamper or product, and a closing slide">
        <Link href="/presentations/new" className="btn-primary">
          New presentation
        </Link>
      </PageHeader>

      <div className="card overflow-x-auto">
        <table className="table">
          <thead>
            <tr>
              <th>Title</th>
              <th className="num">Slides</th>
              <th>Last edited</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={3} className="py-10 text-center text-sm text-[var(--color-muted)]">
                  No presentations yet.
                  <Link href="/presentations/new" className="ml-2 underline">
                    Make the first one
                  </Link>
                </td>
              </tr>
            ) : (
              rows.map((p) => (
                <tr key={p.id}>
                  <td>
                    <Link href={`/presentations/${p.id}`} className="font-medium hover:underline">
                      {p.title}
                    </Link>
                  </td>
                  <td className="num">{p.presentation_slides[0]?.count ?? 0}</td>
                  <td className="text-[var(--color-muted)]">
                    {new Date(p.updated_at).toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </>
  );
}
