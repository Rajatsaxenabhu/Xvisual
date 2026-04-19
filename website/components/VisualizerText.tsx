"use client";

import { useRef, useEffect, useCallback } from "react";

const BAR_COUNT = 48;
const COLORS = ["#7c3aed", "#0ea5e9", "#f43f5e", "#f59e0b", "#10b981"];

export default function VisualizerText() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseX = useRef(0.5);
  const mouseY = useRef(0.5);
  const heights = useRef(new Float32Array(BAR_COUNT).fill(0.05));
  const targets = useRef(new Float32Array(BAR_COUNT).fill(0.05));
  const rafRef = useRef<number>(0);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    const el = canvasRef.current?.parentElement;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    mouseX.current = (e.clientX - rect.left) / rect.width;
    mouseY.current = (e.clientY - rect.top) / rect.height;
  }, []);

  const handleMouseLeave = useCallback(() => {
    mouseX.current = 0.5;
    mouseY.current = 0.5;
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;

    const parent = canvas.parentElement!;
    parent.addEventListener("mousemove", handleMouseMove);
    parent.addEventListener("mouseleave", handleMouseLeave);

    function resize() {
      canvas!.width = parent.offsetWidth;
      canvas!.height = parent.offsetHeight;
    }
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(parent);

    function draw() {
      const W = canvas!.width;
      const H = canvas!.height;
      ctx.clearRect(0, 0, W, H);

      const cx = mouseX.current;
      const intensity = 0.25 + mouseY.current * 0.75;

      for (let i = 0; i < BAR_COUNT; i++) {
        const nx = i / (BAR_COUNT - 1);
        const dist = Math.abs(nx - cx);
        const peak = Math.exp(-dist * dist * 18) * intensity;
        const ripple = Math.sin(Date.now() / 180 + i * 0.5) * 0.04 * intensity;
        targets.current[i] = Math.max(0.03, peak + ripple);
        heights.current[i] += (targets.current[i] - heights.current[i]) * 0.14;
      }

      const barW = W / BAR_COUNT;
      const gap = barW * 0.25;

      for (let i = 0; i < BAR_COUNT; i++) {
        const h = heights.current[i] * H * 0.9;
        const x = i * barW + gap / 2;
        const y = (H - h) / 2;

        const color = COLORS[i % COLORS.length];
        const grad = ctx.createLinearGradient(x, y, x, y + h);
        grad.addColorStop(0, color + "cc");
        grad.addColorStop(1, color + "22");

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.roundRect(x, y, barW - gap, h, 3);
        ctx.fill();
      }

      rafRef.current = requestAnimationFrame(draw);
    }

    draw();

    return () => {
      cancelAnimationFrame(rafRef.current);
      ro.disconnect();
      parent.removeEventListener("mousemove", handleMouseMove);
      parent.removeEventListener("mouseleave", handleMouseLeave);
    };
  }, [handleMouseMove, handleMouseLeave]);

  return (
    <div className="relative max-w-2xl mx-auto mb-8 cursor-crosshair select-none">
      <canvas
        ref={canvasRef}
        className="absolute inset-0 w-full h-full rounded-xl"
        style={{ opacity: 0.7 }}
      />
      <p className="relative z-10 text-zinc-700 text-lg py-6 px-4 font-semibold">
        A real-time terminal audio visualizer for Linux. Captures system audio
        via PipeWire and renders it live in the terminal.
      </p>
    </div>
  );
}
