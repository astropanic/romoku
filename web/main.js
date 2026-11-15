// Browser frontend: canvas drawing, input and timers.
// All game rules, turns and the CPU live in Rust (romoku-core via WASM).

import init, { Game } from "./pkg/romoku_web.js";

const CPU_DELAY_MS = 300;
// Touchscreens get bigger cells so a fingertip hits the intended one
// (~44px is the usual minimum touch target), and can't zoom out below it.
const TOUCH = matchMedia("(pointer: coarse)").matches;
const DEFAULT_CELL = TOUCH ? 56 : 44;
const MIN_CELL = TOUCH ? 40 : 16;
const MAX_CELL = 120;
const SCROLL_MARGIN = 3; // cells kept between the cursor/new move and the edge
const DRAG_THRESHOLD = 5; // px of movement before a press becomes a pan

await init();

const canvas = document.getElementById("board");
const ctx = canvas.getContext("2d");
const statusEl = document.getElementById("status");
const modeEl = document.getElementById("mode");

// Optional starting mode from the URL, e.g. ?mode=3 for CPU vs CPU.
const game = new Game(Number(new URLSearchParams(location.search).get("mode")) || 0);

// Camera: the world point at the centre of the canvas, in cell units.
// Cell (i, j) covers the square [i, i+1) x [j, j+1). Unlike the TUI camera
// this is continuous (floats), so dragging and zooming are smooth.
const cam = { x: 0.5, y: 0.5, cell: DEFAULT_CELL };
const camTarget = { x: cam.x, y: cam.y }; // for smooth panning to new moves
let hover = null; // cell under the mouse
let keyboardMode = false; // show the keyboard cursor only once keys are used
let colors = readColors();

// ---- coordinates ---------------------------------------------------------

const size = () => ({ w: canvas.clientWidth, h: canvas.clientHeight });

function screenToWorld(sx, sy) {
  const { w, h } = size();
  return { x: cam.x + (sx - w / 2) / cam.cell, y: cam.y + (sy - h / 2) / cam.cell };
}

function worldToScreen(wx, wy) {
  const { w, h } = size();
  return { x: (wx - cam.x) * cam.cell + w / 2, y: (wy - cam.y) * cam.cell + h / 2 };
}

function cellAt(sx, sy) {
  const p = screenToWorld(sx, sy);
  return { x: Math.floor(p.x), y: Math.floor(p.y) };
}

/// Same idea as `Camera::follow` in the core: move the camera target only
/// as far as needed to keep the cell `SCROLL_MARGIN` cells from the edge.
function ensureVisible(cx, cy) {
  const { w, h } = size();
  const follow = (target, center, extent) => {
    const half = extent / cam.cell / 2;
    const margin = Math.min(SCROLL_MARGIN, Math.max(0, half - 1));
    const lo = center - half + margin;
    const hi = center + half - margin;
    if (target < lo) return center + (target - lo);
    if (target + 1 > hi) return center + (target + 1 - hi);
    return center;
  };
  camTarget.x = follow(cx, camTarget.x, w);
  camTarget.y = follow(cy, camTarget.y, h);
  requestRender();
}

function centerOn(cx, cy) {
  camTarget.x = cx + 0.5;
  camTarget.y = cy + 0.5;
  requestRender();
}

// ---- game flow -----------------------------------------------------------

/// Asks the core whether the CPU must move; if so, answers after a delay.
/// The ticket makes answers from before a restart harmless.
function pumpCpu() {
  const ticket = game.request_cpu();
  if (ticket === undefined) return;
  updateStatus();
  setTimeout(() => {
    if (game.resolve_cpu(ticket)) {
      ensureVisible(game.cursor_x(), game.cursor_y());
    }
    afterChange();
  }, CPU_DELAY_MS);
}

function afterChange() {
  updateStatus();
  requestRender();
  pumpCpu();
}

function playAt(x, y) {
  game.set_cursor(x, y);
  if (game.play(x, y)) afterChange();
  else requestRender();
}

function newGame(mode = game.mode()) {
  if (mode === game.mode()) game.restart();
  else game.set_mode(mode);
  modeEl.value = String(game.mode());
  camTarget.x = cam.x = 0.5;
  camTarget.y = cam.y = 0.5;
  afterChange();
}

// ---- status bar ----------------------------------------------------------

const mark = (p) => `<span class="${p === 1 ? "x" : "o"}">${p === 1 ? "X" : "O"}</span>`;

function updateStatus() {
  const winner = game.winner();
  let html;
  if (winner) {
    html = `${mark(winner)} wins in ${game.moves()} moves!`;
  } else {
    html = `${mark(game.turn())} to move`;
    if (game.is_waiting()) html += ` <span class="note">· CPU thinking…</span>`;
    else if (game.is_human_turn() && game.moves() === 0) html += ` <span class="note">· click anywhere</span>`;
  }
  statusEl.innerHTML = html;
}

for (let i = 0; i < Game.mode_count(); i++) {
  modeEl.add(new Option(Game.mode_label(i), String(i)));
}
modeEl.addEventListener("change", () => {
  newGame(Number(modeEl.value));
  canvas.focus();
});
document.getElementById("new-game").addEventListener("click", () => {
  newGame();
  canvas.focus();
});

// ---- drawing -------------------------------------------------------------

function readColors() {
  const css = getComputedStyle(document.documentElement);
  const get = (name) => css.getPropertyValue(name).trim();
  return Object.fromEntries(
    ["paper", "grid", "origin", "x", "o", "win", "last", "hover", "cursor"].map((n) => [n, get(`--${n}`)]),
  );
}
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  colors = readColors();
  requestRender();
});

let renderQueued = false;
function requestRender() {
  if (!renderQueued) {
    renderQueued = true;
    requestAnimationFrame(render);
  }
}

function render() {
  renderQueued = false;

  // Ease the camera toward its target; keep animating until close enough.
  const dx = camTarget.x - cam.x;
  const dy = camTarget.y - cam.y;
  if (Math.abs(dx) > 0.001 || Math.abs(dy) > 0.001) {
    cam.x += dx * 0.25;
    cam.y += dy * 0.25;
    requestRender();
  } else {
    cam.x = camTarget.x;
    cam.y = camTarget.y;
  }

  const dpr = window.devicePixelRatio || 1;
  const { w, h } = size();
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = colors.paper;
  ctx.fillRect(0, 0, w, h);

  const c = cam.cell;
  const tl = screenToWorld(0, 0);
  const br = screenToWorld(w, h);
  const x0 = Math.floor(tl.x), x1 = Math.ceil(br.x);
  const y0 = Math.floor(tl.y), y1 = Math.ceil(br.y);

  const fillCell = (cx, cy, style, inset = 0) => {
    const p = worldToScreen(cx, cy);
    ctx.fillStyle = style;
    ctx.fillRect(p.x + inset, p.y + inset, c - 2 * inset, c - 2 * inset);
  };

  // Grid: only the lines that are visible.
  ctx.strokeStyle = colors.grid;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let x = x0; x <= x1; x++) {
    const sx = Math.round(worldToScreen(x, 0).x) + 0.5;
    ctx.moveTo(sx, 0);
    ctx.lineTo(sx, h);
  }
  for (let y = y0; y <= y1; y++) {
    const sy = Math.round(worldToScreen(0, y).y) + 0.5;
    ctx.moveTo(0, sy);
    ctx.lineTo(w, sy);
  }
  ctx.stroke();

  // Origin marker so you can find your way home.
  {
    const p = worldToScreen(0.5, 0.5);
    ctx.fillStyle = colors.origin;
    ctx.beginPath();
    ctx.arc(p.x, p.y, Math.max(1.5, c * 0.06), 0, Math.PI * 2);
    ctx.fill();
  }

  const last = game.last_move();
  if (last.length) fillCell(last[0], last[1], colors.last);

  const win = game.winning_line();
  for (let i = 0; i < win.length; i += 2) fillCell(win[i], win[i + 1], colors.win);

  if (hover && !keyboardMode && game.is_human_turn() && !game.winner()) {
    fillCell(hover.x, hover.y, colors.hover);
  }

  // Stones (culled to the visible area).
  const stones = game.stones();
  ctx.lineCap = "round";
  ctx.lineWidth = Math.max(1.5, c * 0.1);
  for (let i = 0; i < stones.length; i += 3) {
    const sx = stones[i], sy = stones[i + 1], p = stones[i + 2];
    if (sx < x0 || sx > x1 || sy < y0 || sy > y1) continue;
    const center = worldToScreen(sx + 0.5, sy + 0.5);
    const r = c * 0.3;
    ctx.beginPath();
    if (p === 1) {
      ctx.strokeStyle = colors.x;
      ctx.moveTo(center.x - r, center.y - r);
      ctx.lineTo(center.x + r, center.y + r);
      ctx.moveTo(center.x + r, center.y - r);
      ctx.lineTo(center.x - r, center.y + r);
    } else {
      ctx.strokeStyle = colors.o;
      ctx.arc(center.x, center.y, r, 0, Math.PI * 2);
    }
    ctx.stroke();
  }

  // Line through the winning stones.
  if (win.length) {
    const pts = [];
    for (let i = 0; i < win.length; i += 2) pts.push([win[i], win[i + 1]]);
    pts.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    const a = worldToScreen(pts[0][0] + 0.5, pts[0][1] + 0.5);
    const b = worldToScreen(pts.at(-1)[0] + 0.5, pts.at(-1)[1] + 0.5);
    ctx.strokeStyle = game.winner() === 1 ? colors.x : colors.o;
    ctx.globalAlpha = 0.6;
    ctx.lineWidth = Math.max(2, c * 0.14);
    ctx.beginPath();
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
    ctx.stroke();
    ctx.globalAlpha = 1;
  }

  if (keyboardMode) {
    const p = worldToScreen(game.cursor_x(), game.cursor_y());
    ctx.strokeStyle = colors.cursor;
    ctx.lineWidth = 2;
    ctx.strokeRect(p.x + 2, p.y + 2, c - 4, c - 4);
  }
}

// ---- pointer input: tap/click to place, drag to pan, pinch/wheel to zoom --

const pointers = new Map(); // pointerId -> {x, y}
let press = null; // {x, y, dragging} for a single-pointer press
let pinchDist = 0;

function localPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: e.clientX - r.left, y: e.clientY - r.top };
}

/// Zooms by `factor`, keeping the world point under (sx, sy) fixed.
function zoomAt(sx, sy, factor) {
  const before = screenToWorld(sx, sy);
  cam.cell = Math.min(MAX_CELL, Math.max(MIN_CELL, cam.cell * factor));
  const after = screenToWorld(sx, sy);
  cam.x += before.x - after.x;
  cam.y += before.y - after.y;
  camTarget.x = cam.x;
  camTarget.y = cam.y;
  requestRender();
}

canvas.addEventListener("pointerdown", (e) => {
  canvas.focus();
  canvas.setPointerCapture(e.pointerId);
  const p = localPos(e);
  pointers.set(e.pointerId, p);
  if (pointers.size === 1) {
    press = { ...p, dragging: false };
  } else if (pointers.size === 2) {
    press = null; // a second finger cancels the tap and starts a pinch
    const [a, b] = [...pointers.values()];
    pinchDist = Math.hypot(a.x - b.x, a.y - b.y);
  }
});

canvas.addEventListener("pointermove", (e) => {
  const p = localPos(e);
  if (!pointers.has(e.pointerId)) {
    // Plain hover (mouse with no button pressed).
    const cell = cellAt(p.x, p.y);
    if (!hover || hover.x !== cell.x || hover.y !== cell.y) {
      hover = cell;
      keyboardMode = false;
      requestRender();
    }
    return;
  }
  const prev = pointers.get(e.pointerId);
  pointers.set(e.pointerId, p);

  if (pointers.size === 2) {
    const [a, b] = [...pointers.values()];
    const dist = Math.hypot(a.x - b.x, a.y - b.y);
    if (pinchDist > 0) zoomAt((a.x + b.x) / 2, (a.y + b.y) / 2, dist / pinchDist);
    pinchDist = dist;
    return;
  }
  if (press && !press.dragging && Math.hypot(p.x - press.x, p.y - press.y) > DRAG_THRESHOLD) {
    press.dragging = true;
    canvas.classList.add("dragging");
  }
  if (press?.dragging) {
    cam.x -= (p.x - prev.x) / cam.cell;
    cam.y -= (p.y - prev.y) / cam.cell;
    camTarget.x = cam.x;
    camTarget.y = cam.y;
    requestRender();
  }
});

function endPointer(e, cancelled) {
  if (!pointers.delete(e.pointerId)) return;
  if (press && !press.dragging && !cancelled && pointers.size === 0) {
    const p = localPos(e);
    const cell = cellAt(p.x, p.y);
    keyboardMode = false;
    playAt(cell.x, cell.y);
  }
  if (pointers.size === 0) {
    press = null;
    canvas.classList.remove("dragging");
  }
  if (pointers.size < 2) pinchDist = 0;
}
canvas.addEventListener("pointerup", (e) => endPointer(e, false));
canvas.addEventListener("pointercancel", (e) => endPointer(e, true));
canvas.addEventListener("pointerleave", () => {
  if (pointers.size === 0 && hover) {
    hover = null;
    requestRender();
  }
});

canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    const p = localPos(e);
    zoomAt(p.x, p.y, Math.exp(-e.deltaY * 0.0015));
  },
  { passive: false },
);

// ---- keyboard input: same keys as the terminal version -------------------

const MOVES = {
  ArrowLeft: [-1, 0], h: [-1, 0],
  ArrowRight: [1, 0], l: [1, 0],
  ArrowUp: [0, -1], k: [0, -1],
  ArrowDown: [0, 1], j: [0, 1],
};

document.addEventListener("keydown", (e) => {
  if (e.target === modeEl || e.metaKey || e.ctrlKey || e.altKey) return;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  if (MOVES[key]) {
    e.preventDefault();
    if (keyboardMode) game.move_cursor(...MOVES[key]);
    keyboardMode = true; // first press just reveals the cursor
    ensureVisible(game.cursor_x(), game.cursor_y());
  } else if (key === "Enter" || key === " ") {
    if (e.target instanceof HTMLButtonElement) return;
    e.preventDefault();
    keyboardMode = true;
    playAt(game.cursor_x(), game.cursor_y());
  } else if (key === "c") {
    centerOn(game.cursor_x(), game.cursor_y());
  } else if (key === "n") {
    newGame();
  } else if (key === "m") {
    newGame((game.mode() + 1) % Game.mode_count());
  }
});

new ResizeObserver(requestRender).observe(canvas);

modeEl.value = String(game.mode());
canvas.focus();
afterChange();
