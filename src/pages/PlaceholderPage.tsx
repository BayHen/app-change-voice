interface PlaceholderPageProps {
  eyebrow: string;
  title: string;
  description: string;
}

export function PlaceholderPage({ eyebrow, title, description }: PlaceholderPageProps) {
  return (
    <div className="mx-auto max-w-6xl">
      <p className="text-xs font-medium uppercase tracking-[0.22em] text-violet-400">{eyebrow}</p>
      <h1 className="mt-2 text-4xl font-semibold tracking-[-0.04em] text-white">{title}</h1>
      <div className="mt-9 min-h-72 rounded-3xl border border-dashed border-white/10 bg-white/[0.02] p-8">
        <p className="max-w-lg text-sm leading-6 text-zinc-500">{description}</p>
        <span className="mt-6 inline-flex rounded-full border border-white/8 px-3 py-1 text-xs text-zinc-600">
          Planned for a future task
        </span>
      </div>
    </div>
  );
}
