import type { ReactNode } from "react";

interface AppHeaderProps {
  title: string;
  description: string;
  eyebrow?: string;
  action?: ReactNode;
}

export function AppHeader({ title, description, eyebrow, action }: AppHeaderProps) {
  return (
    <header className="flex flex-col gap-4 border-b border-white/[0.07] pb-6 sm:flex-row sm:items-end sm:justify-between">
      <div className="min-w-0">
        {eyebrow ? (
          <p className="mb-2 text-[11px] font-semibold uppercase tracking-[0.16em] text-cyan-400">
            {eyebrow}
          </p>
        ) : null}
        <h1 className="text-2xl font-semibold tracking-[-0.035em] text-zinc-100 sm:text-[28px]">
          {title}
        </h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-zinc-500">{description}</p>
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </header>
  );
}
