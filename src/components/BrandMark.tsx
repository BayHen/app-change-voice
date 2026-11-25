export function BrandMark() {
  return (
    <div className="relative grid size-9 place-items-center overflow-hidden rounded-xl border border-violet-400/30 bg-gradient-to-br from-violet-600 to-indigo-900 text-violet-100">
      <span className="absolute h-4 w-0.5 rounded-full bg-current" />
      <span className="absolute h-2.5 w-0.5 -translate-x-1.5 rounded-full bg-current opacity-60" />
      <span className="absolute h-5.5 w-0.5 translate-x-1.5 rounded-full bg-current" />
    </div>
  );
}
