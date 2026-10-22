export function BrandMark() {
  return (
    <div className="relative grid size-10 place-items-center overflow-hidden rounded-xl bg-violet-500 shadow-[0_0_30px_rgba(139,92,246,0.35)]">
      <span className="absolute h-5 w-1 rounded-full bg-white/90" />
      <span className="absolute h-3 w-1 -translate-x-2 rounded-full bg-white/70" />
      <span className="absolute h-7 w-1 translate-x-2 rounded-full bg-white" />
    </div>
  );
}
