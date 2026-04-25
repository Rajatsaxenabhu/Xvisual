"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useState } from "react";
import BgVisualizer from "@/components/BgVisualizer";

const INSTALL_CMD =
  "curl -sSL https://raw.githubusercontent.com/Rajatsaxenabhu/Xvisual/refs/heads/main/install.sh | bash";

const modes = [
  {
    key: "1",
    name: "Classic Orb",
    desc: "Pulsing circle with beat ripples and radial spikes",
  },
  {
    key: "2",
    name: "Car Dashboard",
    desc: "Tachometer arc driven by audio level",
  },
  {
    key: "3",
    name: "EQ Bars",
    desc: "Symmetric frequency bars mirrored up and down",
  },
  {
    key: "4",
    name: "Space Starfield",
    desc: "Stars burst outward from the center on every beat",
  },
  {
    key: "5",
    name: "Plasma Wave",
    desc: "Layered sine-wave plasma with hue-cycling waveforms",
  },
  {
    key: "6",
    name: "Hallucination",
    desc: "Neural network in deep space — neurons fire and propagate signals through synaptic connections, mapped to frequency bands",
  },
];

const deps = [
  { name: "crossterm", desc: "Terminal rendering" },
  { name: "pipewire-rs", desc: "Audio capture" },
  { name: "ringbuf", desc: "Lock-free audio ring buffer" },
  { name: "ctrlc", desc: "Signal handling" },
];

export default function Home() {
  const [copied, setCopied] = useState(false);

  function handleCopy() {
    navigator.clipboard.writeText(INSTALL_CMD);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div
      className="min-h-screen text-zinc-900 relative overflow-x-hidden"
      style={{ background: "#f5f5ff", fontFamily: "var(--font-inter), sans-serif" }}
    >
      <BgVisualizer />

      {/* Nav */}
      <nav className="relative z-10 border-b border-white/60 backdrop-blur-md bg-white/50 px-10 py-5 flex items-center justify-between sticky top-0">
        <span className="text-xl font-black tracking-widest text-violet-700 uppercase" style={{ fontFamily: "var(--font-geist-mono), monospace" }}>
          xvisual
        </span>
        <a
          href="https://github.com/Rajatsaxenabhu/Xvisual"
          target="_blank"
          rel="noopener noreferrer"
          className="text-base font-medium text-zinc-500 hover:text-zinc-900 transition-colors"
        >
          GitHub →
        </a>
      </nav>

      {/* Hero */}
      <section className="relative z-10 w-full px-10 py-16 text-center">
        <Badge className="mb-5 bg-white/70 text-violet-700 border-violet-300 uppercase tracking-widest text-xs font-semibold backdrop-blur-sm px-4 py-1">
          Linux · PipeWire · Terminal
        </Badge>
        <h1 className="text-9xl font-black tracking-tight mb-6 text-zinc-900" style={{ fontFamily: "var(--font-geist-mono), monospace" }}>
          x<span className="text-violet-600">visual</span>
        </h1>
        <p className="text-zinc-600 text-xl font-medium leading-relaxed mb-10 max-w-2xl mx-auto">
          A real-time terminal audio visualizer for Linux. Captures system audio
          via PipeWire and renders it live in the terminal across 6 visual modes.
        </p>

        {/* Install command */}
        <div className="flex items-center gap-3 bg-white/75 backdrop-blur-sm border border-white/80 rounded-2xl px-5 py-4 text-left text-base text-violet-700 max-w-3xl mx-auto shadow-lg shadow-violet-100/60"
          style={{ fontFamily: "var(--font-geist-mono), monospace" }}
        >
          <span className="flex-1 truncate font-medium">{INSTALL_CMD}</span>
          <Button
            size="sm"
            variant="outline"
            className="shrink-0 border-violet-200 text-violet-600 hover:text-violet-900 hover:border-violet-400 bg-white/70 font-semibold text-sm px-4"
            onClick={handleCopy}
          >
            {copied ? "Copied!" : "Copy"}
          </Button>
        </div>
        <p className="text-sm font-medium text-zinc-400 mt-3">
          Installs all dependencies and launches the visualizer.
        </p>
      </section>

      {/* Modes + Requirements + Dependencies — 2-column */}
      <section className="relative z-10 w-full px-10 py-10 pb-24">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-10 items-start">

          {/* Left: Modes */}
          <div>
            <h2 className="text-3xl font-bold mb-5 text-zinc-800 tracking-tight">Modes</h2>
            <div className="flex flex-col gap-4">
              {modes.map((m) => (
                <Card key={m.key} className="bg-white/65 backdrop-blur-sm border-white/80 text-zinc-900 shadow-md">
                  <CardHeader className="pb-2 pt-5 px-6">
                    <div className="flex items-center gap-4">
                      <span className="text-2xl font-black text-violet-600 w-10 h-10 flex items-center justify-center border-2 border-violet-300 rounded-lg bg-violet-50">
                        {m.key}
                      </span>
                      <CardTitle className="text-lg font-bold tracking-tight">{m.name}</CardTitle>
                    </div>
                  </CardHeader>
                  <CardContent className="px-6 pb-5">
                    <p className="text-base text-zinc-500 font-medium leading-relaxed">{m.desc}</p>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>

          {/* Right: Requirements + Dependencies */}
          <div className="flex flex-col gap-8">
            <div>
              <h2 className="text-3xl font-bold mb-5 text-zinc-800 tracking-tight">Requirements</h2>
              <div className="bg-white/65 backdrop-blur-sm border border-white/80 rounded-2xl p-6 shadow-md">
                <ul className="space-y-3 text-zinc-600">
                  <li className="flex items-start gap-3 text-base font-medium leading-snug">
                    <span className="text-violet-500 mt-0.5 font-black">—</span>
                    Linux (Ubuntu / Debian-based) with PipeWire
                  </li>
                  <li className="flex items-start gap-3 text-base font-medium leading-snug">
                    <span className="text-violet-500 mt-0.5 font-black">—</span>
                    Terminal with Unicode &amp; true-color support
                    <span className="text-zinc-400 font-normal"> (kitty, alacritty, wezterm)</span>
                  </li>
                </ul>
              </div>
            </div>

            <div>
              <h2 className="text-3xl font-bold mb-5 text-zinc-800 tracking-tight">Dependencies</h2>
              <div className="grid grid-cols-2 gap-4">
                {deps.map((d) => (
                  <Card key={d.name} className="bg-white/65 backdrop-blur-sm border-white/80 shadow-md">
                    <CardContent className="pt-5 pb-5 px-5">
                      <p className="font-bold text-violet-600 text-base mb-1">{d.name}</p>
                      <p className="text-zinc-500 text-sm font-medium leading-snug">{d.desc}</p>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          </div>

        </div>
      </section>

      {/* Footer */}
      <footer className="relative z-10 border-t border-white/60 backdrop-blur-md bg-white/30 py-7 text-center text-sm font-medium text-zinc-400">
        xvisual · MIT License ·{" "}
        <a
          href="https://github.com/Rajatsaxenabhu/Xvisual"
          target="_blank"
          rel="noopener noreferrer"
          className="hover:text-zinc-600 transition-colors"
        >
          github.com/Rajatsaxenabhu/Xvisual
        </a>
      </footer>
    </div>
  );
}
