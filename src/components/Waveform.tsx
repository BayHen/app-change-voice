import { useEffect, useRef } from "react";

// Envelope history only: the backend exposes peak/RMS summaries, not PCM samples.
export function Waveform({
  level,
  active,
  label = "Microphone level history",
}: {
  level: number;
  active: boolean;
  label?: string;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const history = useRef<number[]>([]);
  const signal = useRef({ level, active });
  useEffect(() => {
    signal.current = { level, active };
  }, [level, active]);
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const paint = () => {
      if (document.visibilityState === "hidden") return;
      history.current = [
        ...history.current.slice(-119),
        signal.current.active ? Math.min(1, Math.max(0, signal.current.level / 100)) : 0,
      ];
      const width = el.clientWidth || 600,
        height = el.clientHeight || 130;
      const dpr = window.devicePixelRatio || 1;
      el.width = width * dpr;
      el.height = height * dpr;
      const ctx = el.getContext("2d");
      if (!ctx) return;
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, width, height);
      ctx.strokeStyle = "#30334b";
      ctx.lineWidth = 1;
      for (let y = 0; y < 5; y++) {
        ctx.beginPath();
        ctx.moveTo(0, (height * y) / 4);
        ctx.lineTo(width, (height * y) / 4);
        ctx.stroke();
      }
      const gradient = ctx.createLinearGradient(0, 0, width, 0);
      gradient.addColorStop(0, "#7653ed");
      gradient.addColorStop(0.65, "#b582ff");
      gradient.addColorStop(1, "#40bce6");
      ctx.fillStyle = gradient;
      const samples = [
        ...Array(Math.max(0, 120 - history.current.length)).fill(0),
        ...history.current,
      ];
      samples.forEach((v, i) => {
        const h = Math.max(2, v * (height - 12));
        ctx.fillRect((i * width) / 120, (height - h) / 2, Math.max(1, width / 120 - 2), h);
      });
    };
    paint();
    const timer = window.setInterval(paint, 100);
    return () => window.clearInterval(timer);
  }, []);
  return (
    <canvas
      ref={canvas}
      className="vm-waveform"
      role="img"
      aria-label={`${label}. ${active ? "Live peak envelope" : "No live signal"}.`}
    />
  );
}
