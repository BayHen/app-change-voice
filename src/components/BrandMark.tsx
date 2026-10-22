export function BrandMark() {
  return (
    <div className="relative grid size-9 place-items-center overflow-hidden rounded-xl border border-cyan-200/20 bg-cyan-300 text-[#071014]">
      <span className="absolute h-4 w-0.5 rounded-full bg-current" />
      <span className="absolute h-2.5 w-0.5 -translate-x-1.5 rounded-full bg-current opacity-60" />
      <span className="absolute h-5.5 w-0.5 translate-x-1.5 rounded-full bg-current" />
    </div>
  );
}
