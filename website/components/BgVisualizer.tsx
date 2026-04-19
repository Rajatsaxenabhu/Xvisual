"use client";

import { useRef, useEffect } from "react";

const BAR_COUNT = 48;
const COLORS = [
  "#c4b5fd", // violet-300
  "#7dd3fc", // sky-300
  "#fda4af", // rose-300
  "#fcd34d", // amber-300
  "#6ee7b7", // emerald-300
];

export default function BgVisualizer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseX = useRef(0.5);
  const mouseY = useRef(0.3);
  const heights = useRef(new Float32Array(BAR_COUNT).fill(0.04));
  const targets = useRef(new Float32Array(BAR_COUNT).fill(0.04));
  const rafRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current!;
    const ctx = canvas.getContext("2d")!;

    function resize() {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    }
    resize();
    window.addEventListener("resize", resize);
    window.addEventListener("mousemove", (e) => {
      mouseX.current = e.clientX / window.innerWidth;
      mouseY.current = e.clientY / window.innerHeight;
    });

    function draw() {
      const W = canvas.width;
      const H = canvas.height;
      ctx.clearRect(0, 0, W, H);

      const cx = mouseX.current;
      const intensity = 0.15 + mouseY.current * 0.55;
      const t = Date.now();

      for (let i = 0; i < BAR_COUNT; i++) {
        const nx = i / (BAR_COUNT - 1);
        const dist = Math.abs(nx - cx);
        const peak = Math.exp(-dist * dist * 14) * intensity;
        const ripple = Math.sin(t / 200 + i * 0.6) * 0.03 * (intensity + 0.3);
        targets.current[i] = Math.max(0.03, peak + ripple);
        heights.current[i] += (targets.current[i] - heights.current[i]) * 0.1;
      }

      const barW = W / BAR_COUNT;
      const gap = barW * 0.28;

      for (let i = 0; i < BAR_COUNT; i++) {
        const h = heights.current[i] * H;
        const x = i * barW + gap / 2;
        const y = H - h;
        const color = COLORS[i % COLORS.length];

        const grad = ctx.createLinearGradient(x, y, x, H);
        grad.addColorStop(0, color + "cc");
        grad.addColorStop(1, color + "18");

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.roundRect(x, y, barW - gap, h, 4);
        ctx.fill();
      }

      rafRef.current = requestAnimationFrame(draw);
    }

    draw();

    return () => {
      cancelAnimationFrame(rafRef.current);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="fixed inset-0 w-full h-full pointer-events-none"
      style={{ zIndex: 0 }}
    />
  );
}
