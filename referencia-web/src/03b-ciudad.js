/* ---------- ciudad: Distrito Sur (mundo abierto) ---------- */
function rng(seed) { return () => { seed = seed + 0x6D2B79F5 | 0; let t = Math.imul(seed ^ seed >>> 15, 1 | seed); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
const asphaltTex = canvasTex(512, (g, s) => {
  g.fillStyle = '#3b3f42'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 16000; i++) { const v = 40 + Math.random()*40 | 0; g.fillStyle = `rgba(${v},${v+2},${v+4},.45)`; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  g.strokeStyle = 'rgba(20,22,24,.5)'; g.lineWidth = 1.5;
  for (let i = 0; i < 10; i++) { g.beginPath(); let x = Math.random()*s, y = Math.random()*s; g.moveTo(x, y); for (let k = 0; k < 6; k++) { x += (Math.random()-.5)*50; y += (Math.random()-.5)*50; g.lineTo(x, y); } g.stroke(); }
});
const tileTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#bdb6aa'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 1600; i++) { const v = 150 + Math.random()*60 | 0; g.fillStyle = `rgba(${v},${v-4},${v-12},.4)`; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  g.strokeStyle = 'rgba(90,84,74,.55)'; g.lineWidth = 2; g.strokeRect(1, 1, s/2 - 2, s/2 - 2); g.strokeRect(s/2 + 1, 1, s/2 - 2, s/2 - 2); g.strokeRect(1, s/2 + 1, s/2 - 2, s/2 - 2); g.strokeRect(s/2 + 1, s/2 + 1, s/2 - 2, s/2 - 2);
});
const plasterTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#e8e4dc'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 2600; i++) { const v = 200 + Math.random()*55 | 0; g.fillStyle = `rgba(${v},${v},${v-6},.5)`; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  for (let i = 0; i < 6; i++) { g.fillStyle = `rgba(120,110,95,${.05 + Math.random()*.08})`; g.fillRect(0, Math.random()*s, s, 2 + Math.random()*6); }
});
const glassTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#d7dadc'; g.fillRect(0, 0, s, s);
  const gr = g.createLinearGradient(0, 0, s, s); gr.addColorStop(0, '#5b7c96'); gr.addColorStop(.5, '#8fb2c8'); gr.addColorStop(1, '#46637a');
  g.fillStyle = gr; g.fillRect(8, 10, s - 16, s - 26);
  g.fillStyle = 'rgba(255,255,255,.18)'; g.fillRect(8, 10, (s - 16) / 2 - 2, s - 26);
  g.fillStyle = '#c8cccf'; g.fillRect(s/2 - 2, 10, 4, s - 26);
});
const citySkyTex = canvasTex(256, (g, s) => {
  const gr = g.createLinearGradient(0, 0, 0, s);
  gr.addColorStop(0, '#4a86c4'); gr.addColorStop(.4, '#9cc4e2'); gr.addColorStop(.5, '#e9dcc4'); gr.addColorStop(.56, '#b7aa98'); gr.addColorStop(1, '#6d675f');
  g.fillStyle = gr; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 16; i++) { const x = Math.random()*s, y = s*(.16 + Math.random()*.2), w = 30 + Math.random()*60; g.fillStyle = 'rgba(255,255,255,.4)'; g.beginPath(); g.ellipse(x, y, w, w*.16, 0, 0, Math.PI*2); g.fill(); }
});
for (const t of [tileTex, plasterTex, glassTex]) t.repeat.set(1, 1);
function wuvMat(params, wuv) { const m = new THREE.MeshStandardMaterial(params); m.userData.worldUV = wuv; return m; }
const matSidewalk = wuvMat({ map: tileTex, roughness: .92 }, 2.4);
const matInFloor = wuvMat({ map: tileTex, color: 0xa8a092, roughness: .8 }, 1.6);
const matRoof = wuvMat({ map: concreteTex, color: 0xb9b9b9, roughness: .95 }, 4);
const facadeMats = [0xd8c3a5, 0xb7c2c6, 0xe0d2ad, 0xa99a8e, 0xc9b294, 0xcfd6d1, 0xc78f6f, 0x9fb0a6].map(c => wuvMat({ map: plasterTex, color: c, roughness: .88 }, 3));
const towerMats = [0xc3d2dc, 0xd9cdb5, 0xaab6c2, 0xe0e4e6].map(c => wuvMat({ map: glassTex, color: c, roughness: .3, metalness: .45 }, 3.4));
const matSandbag = wuvMat({ map: plasterTex, color: 0x9a8660, roughness: 1 }, .9);
const matJersey = wuvMat({ map: concreteTex, color: 0xd6d2c8, roughness: .9 }, 2);
const matHedge = new THREE.MeshStandardMaterial({ color: 0x3f6a32, roughness: 1 });
const matTrunk = new THREE.MeshStandardMaterial({ color: 0x5a4130, roughness: 1 });
const matPole = new THREE.MeshStandardMaterial({ color: 0x2c3236, metalness: .6, roughness: .5 });
const matLamp = new THREE.MeshBasicMaterial({ color: 0xfff2d0 });
const matLine = new THREE.MeshBasicMaterial({ color: 0xe8e4d4 });
const matLineY = new THREE.MeshBasicMaterial({ color: 0xe0b23a });
const carMats = [0xb83a30, 0x2f5f8f, 0xd8d4c8, 0x2d2f31, 0x6f7f4a, 0xd9a43a].map(c => new THREE.MeshStandardMaterial({ color: c, metalness: .5, roughness: .4 }));
const matCarGlass = new THREE.MeshStandardMaterial({ color: 0x1d2a33, metalness: .8, roughness: .15 });
const matScreen = new THREE.MeshBasicMaterial({ color: 0x5fd8ff });
const matRed = new THREE.MeshBasicMaterial({ color: 0xff3d5a });
const foliageGeo = new THREE.IcosahedronGeometry(1, 1);
const matFoliage = new THREE.MeshStandardMaterial({ color: 0x4d7d3a, roughness: .9, flatShading: true });

const FH = 3.4, WT = .3, BLOCK = 32, BC = [-132, -88, -44, 0, 44, 88, 132];
let SAFE_POS = new V3(-135, 1, 134);
let cityTrees = [], cityRand = Math.random, OUTPOSTS = [], INTERACT = [], SUPPLY = [];

function freeSpans(half, spans) {
  const out = []; let cur = -half;
  for (const [a, b] of spans.slice().sort((p, q) => p[0] - q[0])) { if (a > cur) out.push([cur, a]); cur = Math.max(cur, b); }
  if (cur < half) out.push([cur, half]);
  return out;
}
// pared con aberturas: alféizar (0-1 m), ventanas (1-2.4 m) y dintel hasta el techo
function wallSide(axis, fixed, c0, len, y0, wallH, openings, mat) {
  const put = (s0, s1, ya, yb) => {
    if (s1 - s0 < .05 || yb - ya < .05) return;
    const mid = (s0 + s1) / 2 + c0, L = s1 - s0;
    if (axis === 'x') addBlock(mid, ya, fixed, L, yb - ya, WT, mat); else addBlock(fixed, ya, mid, WT, yb - ya, L, mat);
  };
  const half = len / 2;
  const doors = openings.filter(o => o.door).map(o => [o.s - o.w/2, o.s + o.w/2]);
  const all = openings.map(o => [o.s - o.w/2, o.s + o.w/2]);
  const top = y0 + wallH;
  for (const [a, b] of freeSpans(half, doors)) put(a, b, y0, y0 + 1.0);
  for (const [a, b] of freeSpans(half, all)) put(a, b, y0 + 1.0, Math.min(top, y0 + 2.4));
  if (top > y0 + 2.4) put(-half, half, y0 + 2.4, top);
}
function windowRow(len, spacing = 3.2, w = 1.4) {
  const n = Math.max(0, Math.floor((len - 1.6) / spacing)), o = [];
  for (let i = 0; i < n; i++) o.push({ s: -((n - 1) * spacing) / 2 + i * spacing, w });
  return o;
}
function slabWithHole(cx, cz, w, d, y, t, hx0, hx1, hz0, hz1, mat) {
  const X0 = cx - w/2, X1 = cx + w/2, Z0 = cz - d/2, Z1 = cz + d/2;
  const r = (x0, x1, z0, z1) => { if (x1 - x0 > .02 && z1 - z0 > .02) addBlock((x0 + x1)/2, y, (z0 + z1)/2, x1 - x0, t, z1 - z0, mat); };
  if (hx0 === null) { r(X0, X1, Z0, Z1); return; }
  r(X0, X1, Z0, hz0); r(X0, X1, hz1, Z1); r(X0, hx0, hz0, hz1); r(hx1, X1, hz0, hz1);
}
// edificio transitable: plantas con ventanas, puertas en planta baja, escaleras alternadas y azotea con parapeto
function addBuilding(cx, cz, w, d, floors, mat, o = {}) {
  const base = o.base ?? .16, stairW = 2, run = 9 * .6, doors = o.doors || ['s'];
  const stair = f => {
    const west = f % 2 === 0;
    const x = west ? cx - w/2 + WT + .1 + stairW/2 : cx + w/2 - WT - .1 - stairW/2;
    const z0 = west ? cz - d/2 + WT + .3 : cz + d/2 - WT - .3;
    return { west, x, z0, dir: west ? 1 : -1 };
  };
  for (let f = 0; f <= floors; f++) {
    const y0 = base + f * FH;
    if (f > 0) {
      const s = stair(f - 1), hx0 = s.x - stairW/2 - .1, hx1 = s.x + stairW/2 + .1;
      const za = s.z0, zb = s.z0 + s.dir * (run + .4);
      slabWithHole(cx, cz, w, d, y0 - .2, .2, hx0, hx1, Math.min(za, zb), Math.max(za, zb), f === floors ? matRoof : matInFloor);
    } else addVisual(cx, base + .006, cz, w - WT*2, .012, d - WT*2, matInFloor, false);
    if (f === floors) {
      for (const [x, z, ww, dd] of [[cx, cz - d/2 + .15, w, .3], [cx, cz + d/2 - .15, w, .3], [cx - w/2 + .15, cz, .3, d - .6], [cx + w/2 - .15, cz, .3, d - .6]]) addBlock(x, y0, z, ww, 1.0, dd, mat);
      if (o.fireEscape) addFireEscape(cx + w/2, cz, floors, base);
      if (o.roofProps !== false) { addBlock(cx + w*.2, y0, cz + d*.15, 2.2, 1.3, 1.6, matPillar); addBlock(cx - w*.15, y0, cz + d*.25, 1.2, .9, 1.2, matSteel); }
      break;
    }
    const wallH = FH - .2;
    const side = (key, len) => {
      const ops = windowRow(len);
      if (f === 0 && doors.includes(key)) {
        const ds = o.doorOffset || 0;
        for (let i = ops.length - 1; i >= 0; i--) if (Math.abs(ops[i].s - ds) < 2.2) ops.splice(i, 1);
        ops.push({ s: ds, w: 2.2, door: true });
      }
      return ops;
    };
    wallSide('x', cz - d/2 + WT/2, cx, w, y0, wallH, side('n', w), mat);
    wallSide('x', cz + d/2 - WT/2, cx, w, y0, wallH, side('s', w), mat);
    wallSide('z', cx - w/2 + WT/2, cz, d - WT*2, y0, wallH, side('w', d - WT*2), mat);
    wallSide('z', cx + w/2 - WT/2, cz, d - WT*2, y0, wallH, side('e', d - WT*2), mat);
    const s = stair(f);
    for (let k = 1; k <= 9; k++) addBlock(s.x, y0, s.z0 + s.dir * (k - .5) * .6, stairW, k * STEP, .6, matConcrete);
    // tabique interior con puerta: arma habitaciones y pasillos
    if (w >= 12) wallSide('z', cx + (f % 2 ? -1.5 : 1.5), cz + .8, d - WT*2 - 1.6, y0, wallH, [{ s: (f % 2 ? -1 : 1) * (d * .18), w: 1.3, door: true }], matInFloor);
    // balcones en la fachada sur
    if (o.balconies && f > 0) for (const bxo of [-w * .25, w * .25]) {
      const zf = cz + d/2;
      addBlock(cx + bxo, y0 - .2, zf + .65, 3.4, .2, 1.3, matRoof);
      addBlock(cx + bxo, y0, zf + 1.25, 3.4, .95, .08, matPole);
      addBlock(cx + bxo - 1.66, y0, zf + .65, .08, .95, 1.3, matPole); addBlock(cx + bxo + 1.66, y0, zf + .65, .08, .95, 1.3, matPole);
    }
    if (o.props !== false) {
      const r = cityRand;
      for (let i = 0; i < 2; i++) {
        const px = cx + (r() - .5) * (w - 6), pz = cz + (r() - .5) * (d - 5);
        if (Math.abs(px - s.x) < 2.5) continue;
        addBlock(px, y0, pz, 1.2 + r(), .9 + r()*.3, .8 + r()*.6, r() < .5 ? matCrate : matWood);
      }
    }
  }
}
// torre maciza con fachada vidriada; techo alcanzable con el gancho
function addTower(cx, cz, w, d, h, mat, gaps = []) {
  addBox(cx, cz, w, h, d, mat);
  for (const [key, x, z, ww, dd] of [['n', cx, cz - d/2 + .2, w, .4], ['s', cx, cz + d/2 - .2, w, .4], ['w', cx - w/2 + .2, cz, .4, d - .8], ['e', cx + w/2 - .2, cz, .4, d - .8]]) {
    if (!gaps.includes(key)) { addBlock(x, h, z, ww, 1.1, dd, matRoof); continue; }
    const alongX = ww > dd, L = alongX ? ww : dd, seg = (L - 3.4) / 2;
    for (const sg of [-1, 1]) { const off = sg * (1.7 + seg / 2); if (alongX) addBlock(x + off, h, z, seg, 1.1, dd, matRoof); else addBlock(x, h, z + off, ww, 1.1, seg, matRoof); }
  }
  addBlock(cx, h, cz, Math.min(5, w*.4), 2.2, Math.min(4, d*.4), matPillar);
  addStrip(cx, h + 2.25, cz, .2, .5, .2, matRed);
}
// galpón con portón y techo
function addHall(cx, cz, w, d, h, mat, doorSides = ['s']) {
  const base = .16;
  const ops = key => doorSides.includes(key) ? [{ s: 0, w: 5, door: true }] : [];
  wallSide('x', cz - d/2 + WT/2, cx, w, base, h, ops('n'), mat);
  wallSide('x', cz + d/2 - WT/2, cx, w, base, h, ops('s'), mat);
  wallSide('z', cx - w/2 + WT/2, cz, d - WT*2, base, h, ops('w'), mat);
  wallSide('z', cx + w/2 - WT/2, cz, d - WT*2, base, h, ops('e'), mat);
  addBlock(cx, base + h, cz, w, .3, d, matRoof);
  addVisual(cx, base + .006, cz, w - WT*2, .012, d - WT*2, matInFloor, false);
}
function addCar(x, z, alongX, mat, y = 0) {
  const L = 4.3, W = 1.9;
  const w = alongX ? L : W, d = alongX ? W : L;
  addBlock(x, y + .1, z, w, .9, d, mat);
  addVisual(x, y + 1.35, z, alongX ? 2.2 : 1.7, .6, alongX ? 1.7 : 2.2, matCarGlass);
  addCollider(x, z, alongX ? 2.2 : 1.7, .65, alongX ? 1.7 : 2.2, y + 1.0);
  addVisual(x, y + .3, z, w + .06, .25, d + .06, matTire, false);
}
// escalera de incendio exterior: tramos alternados en dos carriles y descansos por piso
function addFireEscape(wallX, cz, floors, base) {
  const inL = wallX + .55, outL = wallX + 1.45, zA = cz - 1.8, zB = cz + 1.8;
  for (let f = 0; f < floors; f++) {
    const y0 = base + f * FH, lane = f % 2 ? outL : inL, up = f % 2 === 0;
    for (let k = 1; k <= 9; k++) addBlock(lane, y0, up ? zA + (k - .5) * .4 : zB - (k - .5) * .4, .9, k * STEP, .4, matPole);
    const y1 = y0 + FH, lz = up ? zB + .6 : zA - .6;
    addBlock(wallX + 1, y1 - .14, lz, 1.9, .14, 1.2, matPole);
    addBlock(wallX + 1.97, y1, lz, .06, .95, 1.2, matPole);
  }
  for (let y = base + 1; y < base + floors * FH; y += 1) addStrip(wallX + 1.97, y, cz, .05, .05, 3.6, matPole);
}
// pasarela entre azoteas
function addSkybridge(x0, z0, x1, z1, y) {
  const alongX = Math.abs(x1 - x0) > Math.abs(z1 - z0), L = alongX ? Math.abs(x1 - x0) : Math.abs(z1 - z0), cx = (x0 + x1) / 2, cz = (z0 + z1) / 2;
  if (alongX) { addBlock(cx, y - .3, cz, L, .3, 3.2, matSteel); for (const s of [-1, 1]) addBlock(cx, y, cz + s * 1.55, L, 1.05, .1, matPole); for (let t = -L/2; t <= L/2; t += 3) addStrip(cx + t, y - .9, cz, .2, 1.2, 3.2, matPole); }
  else { addBlock(cx, y - .3, cz, 3.2, .3, L, matSteel); for (const s of [-1, 1]) addBlock(cx + s * 1.55, y, cz, .1, 1.05, L, matPole); for (let t = -L/2; t <= L/2; t += 3) addStrip(cx, y - .9, cz + t, 3.2, 1.2, .2, matPole); }
}
// autopista elevada con rampas de bajada en ambos extremos
function addHighway(z, x0, x1, deckY) {
  const len = x1 - x0, cx = (x0 + x1) / 2;
  addBlock(cx, deckY - .6, z, len, .6, 11, matConcrete);
  for (const s of [-1, 1]) addBlock(cx, deckY, z + s * 5.2, len, .95, .6, matJersey);
  for (let x = x0 + 6; x < x1 - 4; x += 14) { addBlock(x, 0, z - 3, 1.4, deckY - .6, 1.4, matConcrete); addBlock(x, 0, z + 3, 1.4, deckY - .6, 1.4, matConcrete); addVisual(x, deckY - .9, z, 1.6, .6, 9, matConcrete); }
  for (let x = x0 + 3; x < x1; x += 8) addStrip(x, deckY + .01, z, 3.5, .02, .18, matLine);
  const n = Math.round(deckY / STEP);
  for (const [edge, dir] of [[x0, -1], [x1, 1]]) {
    for (let k = 1; k < n; k++) addBlock(edge + dir * (k - .5) * .9, 0, z, .9, deckY - k * STEP, 10, matConcrete);
  }
  for (let x = x0 + 10; x < x1; x += 30) { addBlock(x, deckY, z - 5.6, .18, 6, .18, matPole); addVisual(x, deckY + 6.05, z - 5.1, .35, .14, .9, matLamp, false); }
}
function addTree(x, z, s = 1) {
  addBlock(x, 0, z, .35*s, 2.4*s, .35*s, matTrunk);
  cityTrees.push([x, 2.4*s + 1.2*s, z, 1.7*s]);
}
function addLamp(x, z) { addBlock(x, 0, z, .18, 6, .18, matPole); addVisual(x, 6.05, z, .8, .14, .35, matLamp, false); }
function sandbags(x, z, len, alongX) { addBox(x, z, alongX ? len : .9, 1.1, alongX ? .9 : len, matSandbag); }
function jersey(x, z, len, alongX) { addBox(x, z, alongX ? len : .6, .95, alongX ? .6 : len, matJersey); }

// estructura de obra: columnas, losas parciales y escaleras sin paredes
function addFrame(cx, cz, w, d, floors) {
  const base = .16, stairW = 2, run = 9 * .6;
  for (let x = -w/2; x <= w/2 + .01; x += w / 4) for (let z = -d/2; z <= d/2 + .01; z += d / 4) addBlock(cx + x, base, cz + z, .4, floors * FH, .4, matSteel);
  const stair = f => { const west = f % 2 === 0; return { x: west ? cx - w/2 + 1.6 : cx + w/2 - 1.6, z0: west ? cz - d/2 + .6 : cz + d/2 - .6, dir: west ? 1 : -1 }; };
  for (let f = 0; f <= floors; f++) {
    const y0 = base + f * FH;
    if (f > 0) {
      const s = stair(f - 1), za = s.z0, zb = s.z0 + s.dir * (run + .4);
      if (f === floors) { addBlock(cx - w/4, y0 - .2, cz, w/2, .2, d, matConcrete); }
      else slabWithHole(cx, cz, w, d, y0 - .2, .2, s.x - stairW/2 - .1, s.x + stairW/2 + .1, Math.min(za, zb), Math.max(za, zb), matConcrete);
      for (const [x, z, ww, dd] of [[cx, cz - d/2, w, .25], [cx, cz + d/2, w, .25], [cx - w/2, cz, .25, d], [cx + w/2, cz, .25, d]]) addVisual(x, y0 - .45, z, ww, .3, dd, matYellow);
    }
    if (f < floors) { const s = stair(f); for (let k = 1; k <= 9; k++) addBlock(s.x, y0, s.z0 + s.dir * (k - .5) * .6, stairW, k * STEP, .6, matSteel); }
  }
}
function addTankCar(x, z) {
  const t = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 1.5, 11, 18), matTank); t.rotation.z = Math.PI / 2; t.position.set(x, 2.1, z); t.castShadow = t.receiveShadow = true; track(t);
  addCollider(x, z, 11, 3, 3, .6); addVisual(x, .4, z, 11, .5, 2.4, matTire, false);
}
function addFuelTank(x, z, r, h) {
  const t = new THREE.Mesh(new THREE.CylinderGeometry(r, r, h, 24), matTank); t.position.set(x, .16 + h / 2, z); t.castShadow = t.receiveShadow = true; track(t);
  addCollider(x, z, r * 1.8, h, r * 1.8, .16);
  addStrip(x, .16 + h * .8, z, r * 2.02, .25, r * 2.02, matLineY);
}
function addStall(x, z, mat) {
  addBox(x, z, 2.6, 1.0, 1.2, matWood);
  for (const [dx, dz] of [[-1.3, -.9], [1.3, -.9], [-1.3, .9], [1.3, .9]]) addBlock(x + dx, 0, z + dz, .1, 2.6, .1, matPole);
  addVisual(x, 2.65, z, 3, .12, 2.2, mat, true);
}
const awningMats = [0xc0392b, 0x2e86ab, 0xe0a92a, 0x3f8f5a].map(c => new THREE.MeshStandardMaterial({ color: c, roughness: .8 }));
const matStatue = new THREE.MeshStandardMaterial({ color: 0x7a8a80, metalness: .6, roughness: .45 });

/* --- puestos enemigos: cada uno arma su bloque y registra patrullas, coberturas, defensas y objetivo --- */
function outpostWalls(cx, cz, gates) {
  const h = BLOCK/2 - .6;
  for (const [key, ax, fx] of [['n', true, cz - h], ['s', true, cz + h], ['w', false, cx - h], ['e', false, cx + h]]) {
    const segs = gates.includes(key) ? [[-h, -3], [3, h]] : [[-h, h]];
    for (const [a, b] of segs) {
      const c = (a + b) / 2, L = b - a;
      if (ax) sandbags(cx + c, fx, L, true); else sandbags(fx, cz + c, L, false);
    }
  }
}
function coverAround(op, x, z, w, d) { op.cover.push([x - w/2 - .9, z], [x + w/2 + .9, z], [x, z - d/2 - .9], [x, z + d/2 + .9]); }
function opCrate(op, x, z, w = 2, h = 1.2, d = 2) { addBox(x, z, w, h, d, matCrate); coverAround(op, x, z, w, d); }
function makeOutpost(def) {
  const op = Object.assign({ cover: [], patrol: [], soldiers: [], turrets: [], snipers: [], liberated: false, alarm: 0, search: 0, alertLevel: 0 }, def);
  OUTPOSTS.push(op);
  return op;
}
function buildOutposts() {
  // Antena de comunicaciones
  {
    const cx = -132, cz = -44, op = makeOutpost({ id: 'antena', name: 'Antena', full: 'Antena de comunicaciones', cx, cz,
      effect: 'Sin antena no llegan drones de refuerzo', task: 'Plantar carga en la antena', icon: 'A' });
    outpostWalls(cx, cz, ['e', 's']);
    const ag = new THREE.Group(); ag.position.set(cx, 0, cz); track(ag);
    const am = (w, h, d, x, y, z, m) => { const o = new THREE.Mesh(boxGeo, m); o.scale.set(w, h, d); o.position.set(x, y, z); o.castShadow = true; ag.add(o); };
    op.antennaCols = [];
    for (const [lx, lz] of [[-1.6, -1.6], [1.6, -1.6], [-1.6, 1.6], [1.6, 1.6]]) { am(.35, 24, .35, lx, 12, lz, matSteel); op.antennaCols.push(addCollider(cx + lx, cz + lz, .35, 24, .35)); }
    for (let y = 3; y < 24; y += 3) { am(3.2, .12, .12, 0, y, -1.6, matPole); am(3.2, .12, .12, 0, y, 1.6, matPole); am(.12, .12, 3.2, -1.6, y, 0, matPole); am(.12, .12, 3.2, 1.6, y, 0, matPole); }
    am(.6, .6, .6, 0, 24.3, 0, matRed); am(2.4, .2, .2, 0, 22, 0, matPole); am(.2, .2, 2.4, 0, 22, 0, matPole);
    op.antennaGroup = ag; op.antennaTop = new V3(cx, 24.3, cz);
    op.objPos = new V3(cx, 1, cz + 2.4);
    addBlock(cx, .16, cz + 2.6, 1.2, .9, .6, matPillar); addStrip(cx, .7, cz + 2.93, .8, .3, .02, matScreen);
    addContainer(cx - 9, cz - 9, true, 2, 1); addContainer(cx + 9, cz - 10, false, 1, 0); addContainer(cx - 10, cz + 8, false, 1, 2);
    coverAround(op, cx - 9, cz - 9, 6.2, 2.6); coverAround(op, cx + 9, cz - 10, 2.6, 6.2); coverAround(op, cx - 10, cz + 8, 2.6, 6.2);
    opCrate(op, cx + 6, cz + 6); opCrate(op, cx - 5, cz + 1, 1.4, 1.1, 3); opCrate(op, cx + 4, cz - 4, 3, 1.1, 1.2);
    op.patrol = [[cx - 6, cz + 10], [cx + 10, cz + 10], [cx + 10, cz - 2], [cx - 2, cz - 6], [cx - 12, cz]];
    op.soldiers = [{ x: cx + 8, z: cz + 9, r: 'patrol' }, { x: cx - 6, z: cz - 4, r: 'patrol' }, { x: cx + 2, z: cz + 4, r: 'guard' }, { x: cx - 12, z: cz + 12, r: 'guard' }];
    op.snipers = [{ x: cx - 9, z: cz - 9, y: 5.4 }];
    op.turrets = [{ x: cx + 12, z: cz - 12, yaw: Math.PI * .75 }];
  }
  // Subestación
  {
    const cx = 132, cz = 44, op = makeOutpost({ id: 'sub', name: 'Subestación', full: 'Subestación eléctrica', cx, cz,
      effect: 'Sin energía, todas las torretas se apagan', task: 'Sabotear el generador', icon: 'S' });
    outpostWalls(cx, cz, ['w', 'n']);
    for (const [tx, tz] of [[-8, -7], [-2, -7], [4, -7], [-8, 6], [-2, 6]]) {
      addBox(cx + tx, cz + tz, 3, 2.6, 2.4, matSteel); coverAround(op, cx + tx, cz + tz, 3, 2.4);
      for (let i = -1; i <= 1; i++) addStrip(cx + tx + i*.9, 3.0, cz + tz, .14, .8, .14, matPole);
    }
    addHall(cx + 8, cz + 6, 9, 8, 4, facadeMats[1], ['w']);
    op.objPos = new V3(cx + 10, 1, cz + 7);
    addBlock(cx + 10.6, .16, cz + 7, .7, 1.4, 1.4, matPillar); addStrip(cx + 10.24, 1.1, cz + 7, .02, .5, .9, matScreen);
    opCrate(op, cx + 2, cz + 1, 3, 1.1, 1.2); opCrate(op, cx - 5, cz, 1.2, 1.1, 3);
    op.patrol = [[cx - 11, cz - 1], [cx + 1, cz - 2], [cx + 2, cz + 11], [cx - 9, cz + 11]];
    op.soldiers = [{ x: cx - 10, z: cz, r: 'patrol' }, { x: cx + 1, z: cz + 10, r: 'patrol' }, { x: cx + 6, z: cz + 6, r: 'guard' }, { x: cx - 4, z: cz - 11, r: 'guard' }];
    op.turrets = [{ x: cx - 12, z: cz - 12, yaw: Math.PI * 1.25 }, { x: cx - 12, z: cz + 12, yaw: -Math.PI * .25 }];
  }
  // Oficinas
  {
    const cx = 88, cz = -132, op = makeOutpost({ id: 'datos', name: 'Oficinas', full: 'Torre de oficinas', cx, cz,
      effect: 'Los datos marcan a todos los enemigos', task: 'Descargar los datos (último piso)', icon: 'D' });
    outpostWalls(cx, cz, ['s', 'w']);
    addBuilding(cx + 2, cz - 3, 16, 13, 3, facadeMats[5], { doors: ['s', 'w'], roofProps: false, fireEscape: true });
    op.objPos = new V3(cx + 7, .16 + FH*2 + 1, cz - 6);
    addBlock(cx + 7.5, .16 + FH*2, cz - 6, 1.6, .8, .9, matPillar); addStrip(cx + 7.5, .16 + FH*2 + .95, cz - 6.2, 1.1, .5, .06, matScreen);
    opCrate(op, cx - 9, cz + 9); opCrate(op, cx + 8, cz + 10, 3, 1.1, 1.2); opCrate(op, cx - 10, cz - 6, 1.2, 1.1, 3);
    op.patrol = [[cx - 12, cz + 6], [cx - 4, cz + 12], [cx + 10, cz + 12], [cx + 12, cz + 6]];
    op.soldiers = [{ x: cx - 6, z: cz + 9, r: 'patrol' }, { x: cx + 9, z: cz + 7, r: 'guard' },
      { x: cx + 4, z: cz - 1, y: .16 + FH, r: 'guard', leash: 3 }, { x: cx - 2, z: cz - 6, y: .16 + FH*2, r: 'guard', leash: 3 }];
    op.snipers = [{ x: cx + 2, z: cz - 6, y: .16 + FH*3 }];
    op.turrets = [{ x: cx + 13, z: cz + 13, yaw: -Math.PI * .25 }];
  }
  // Arsenal
  {
    const cx = 0, cz = 132, op = makeOutpost({ id: 'arsenal', name: 'Arsenal', full: 'Arsenal', cx, cz,
      effect: 'Desbloquea el riel y el lanzagranadas', task: 'Forzar el arsenal', icon: 'R' });
    outpostWalls(cx, cz, ['n', 'w']);
    addHall(cx + 2, cz + 4, 16, 11, 5, facadeMats[3], ['n', 'w']);
    op.objPos = new V3(cx + 7, 1, cz + 7);
    addBlock(cx + 7, .16, cz + 7.5, 2.4, 1.2, 1.2, matContainers[3]);
    for (const [x, z] of [[-3, 6], [1, 2], [5, 2]]) { addBox(cx + x, cz + z, 2, 1.2, 1, matCrate); coverAround(op, cx + x, cz + z, 2, 1); }
    opCrate(op, cx - 9, cz - 8); opCrate(op, cx + 8, cz - 9, 3, 1.1, 1.2); opCrate(op, cx - 10, cz + 4, 1.2, 1.1, 3);
    op.patrol = [[cx - 11, cz - 10], [cx + 11, cz - 11], [cx + 11, cz - 4], [cx - 11, cz - 3]];
    op.soldiers = [{ x: cx - 8, z: cz - 11, r: 'patrol' }, { x: cx + 9, z: cz - 5, r: 'patrol' }, { x: cx + 4, z: cz + 6, r: 'guard', leash: 4 }, { x: cx - 3, z: cz + 2, r: 'guard', leash: 4 }];
    op.turrets = [{ x: cx + 12, z: cz - 13, yaw: Math.PI * .75 }];
  }
  // Puesto de mando
  {
    const cx = 0, cz = 0, op = makeOutpost({ id: 'mando', name: 'Mando', full: 'Puesto de mando', cx, cz,
      effect: 'Eliminar al comandante descabeza al ejército', task: 'Eliminar al comandante', icon: 'M' });
    outpostWalls(cx, cz, ['n', 's', 'e', 'w']);
    addHall(cx, cz - 6, 12, 8, 3.6, matJersey, ['s']);
    for (const [x, z, w, d] of [[-8, 6, 4, 1], [8, 6, 4, 1], [0, 10, 6, 1], [-10, -2, 1, 4], [10, -2, 1, 4]]) { addBox(cx + x, cz + z, w, 1.1, d, matSandbag); coverAround(op, cx + x, cz + z, w, d); }
    opCrate(op, cx - 5, cz + 2); opCrate(op, cx + 5, cz + 2);
    op.objPos = null;
    op.patrol = [[cx - 12, cz + 12], [cx + 12, cz + 12], [cx + 12, cz - 12], [cx - 12, cz - 12]];
    op.soldiers = [{ x: cx, z: cz - 6, r: 'commander', leash: 8 }, { x: cx - 10, z: cz + 10, r: 'patrol' }, { x: cx + 10, z: cz - 12, r: 'patrol' },
      { x: cx - 4, z: cz - 4, r: 'guard' }, { x: cx + 4, z: cz - 4, r: 'guard' }];
    op.snipers = [{ x: cx, z: cz - 6, y: .16 + 3.9 }];
    op.turrets = [{ x: cx - 13, z: cz - 13, yaw: Math.PI * 1.25 }, { x: cx + 13, z: cz + 13, yaw: -Math.PI * .25 }];
  }
  // Obra en construcción: la cabina de la grúa está a 40 m
  {
    const cx = -44, cz = -132, op = makeOutpost({ id: 'obra', name: 'Obra', full: 'Obra en construcción', cx, cz,
      effect: '+1 intento y francotiradores y torretas marcados', task: 'Hackear la grúa (cabina, a 40 m)', icon: 'O' });
    outpostWalls(cx, cz, ['s', 'e']);
    addFrame(cx - 4, cz - 3, 20, 16, 5);
    const mx = cx + 10, mz = cz + 9;
    addBox(mx, mz, 1.8, 40, 1.8, matYellow);
    for (let y = 2; y < 40; y += 2.5) addStrip(mx, y, mz, 1.9, .1, 1.9, matPole);
    addBlock(mx + 6, 40, mz, 34, 1.2, 1.6, matYellow);
    addBlock(mx - 9, 38.6, mz, 3, 1.4, 2.4, matConcrete);
    addBlock(mx + 2.4, 37.4, mz, 2.4, 2.4, 2.2, matYellow); addVisual(mx + 2.4, 38.7, mz + 1.12, 2, 1, .04, matCarGlass, false);
    for (let x = mx - 9; x <= mx + 22; x += 2) addStrip(x, 41.25, mz, .12, .1, 1.6, matPole);
    addStrip(mx + 18, 30, mz, .06, 20, .06, matPole); addBlock(mx + 18, 19.4, mz, .8, .8, .8, matSteel);
    op.objPos = new V3(mx + 2.4, 41.2 + 1, mz);
    op.extraLife = true;
    addContainer(cx + 10, cz - 12, true, 1, 0); addContainer(cx - 11, cz + 11, true, 2, 2);
    coverAround(op, cx + 10, cz - 12, 6.2, 2.6); coverAround(op, cx - 11, cz + 11, 6.2, 2.6);
    opCrate(op, cx + 4, cz + 9, 3, 1.2, 1.6); opCrate(op, cx - 4, cz + 11); opCrate(op, cx + 12, cz + 2, 1.4, 1.1, 3);
    for (let i = 0; i < 4; i++) addBox(cx + 9 + i * 1.2, cz - 2, 1, .6 + i * .3, 3, matWood);
    op.patrol = [[cx - 12, cz + 6], [cx + 4, cz + 12], [cx + 12, cz + 5], [cx + 12, cz - 12]];
    op.soldiers = [{ x: cx - 10, z: cz + 6, r: 'patrol' }, { x: cx + 6, z: cz + 12, r: 'patrol' }, { x: cx - 4, z: cz - 3, r: 'guard', leash: 5 },
      { x: cx - 8, z: cz - 6, y: .16 + FH * 2, r: 'guard', leash: 3 }];
    op.snipers = [{ x: cx - 10, z: cz - 3, y: .16 + FH * 5 }, { x: cx + 1, z: cz - 9, y: .16 + FH * 3 }];
    op.turrets = [{ x: cx + 13, z: cz + 13, yaw: -Math.PI * .25 }];
  }
  // Patio de trenes y depósito de combustible
  {
    const cx = 132, cz = 132, op = makeOutpost({ id: 'trenes', name: 'Trenes', full: 'Depósito de combustible', cx, cz,
      effect: 'Sin combustible, los drones no vuelan', task: 'Plantar carga en el depósito', icon: 'T' });
    outpostWalls(cx, cz, ['n', 'w']);
    for (const tz of [-9, -3]) { addStrip(cx, .19, tz + cz - .7, 31, .06, .12, matSteel); addStrip(cx, .19, tz + cz + .7, 31, .06, .12, matSteel); for (let x = -15; x <= 15; x += 1.2) addStrip(cx + x, .17, cz + tz, .25, .03, 2.2, matWood); }
    for (const [x, ci] of [[-9, 0], [5, 2]]) { addBlock(cx + x, .55, cz - 9, 12, 3.4, 3, matContainers[ci]); addVisual(cx + x, .3, cz - 9, 11, .5, 2.4, matTire, false); coverAround(op, cx + x, cz - 9, 12, 3); }
    addTankCar(cx - 4, cz - 3); coverAround(op, cx - 4, cz - 3, 11, 3);
    addFuelTank(cx + 9, cz + 9, 3, 7); addFuelTank(cx + 1, cz + 10, 2.6, 6);
    for (let x = -1; x <= 9; x += 2) addStrip(cx + x, 1.2, cz + 5.6, 2, .3, .3, matPole);
    op.objPos = new V3(cx + 5, 1, cz + 5);
    addBlock(cx + 5, .16, cz + 5.4, 1.4, 1.2, .7, matPillar); addStrip(cx + 5, .9, cz + 5.03, .9, .4, .04, matScreen);
    addBuilding(cx - 9.5, cz + 9, 10, 9, 2, facadeMats[6], { doors: ['n', 'e'], roofProps: false });
    opCrate(op, cx + 12, cz + 1); opCrate(op, cx - 2, cz + 4, 3, 1.1, 1.2);
    op.patrol = [[cx - 13, cz - 13], [cx + 13, cz - 13], [cx + 13, cz + 2], [cx - 2, cz + 1]];
    op.soldiers = [{ x: cx - 12, z: cz - 13, r: 'patrol' }, { x: cx + 12, z: cz + 2, r: 'patrol' }, { x: cx + 6, z: cz + 2, r: 'guard' }, { x: cx - 3, z: cz - 6, r: 'guard' }];
    op.snipers = [{ x: cx - 10, z: cz + 9, y: .16 + FH * 2 }];
    op.turrets = [{ x: cx + 13, z: cz + 13, yaw: -Math.PI * .25 }, { x: cx - 13, z: cz - 13, yaw: Math.PI * .75 }];
  }
}

/* --- manzanas civiles --- */
function blockApts(bx, bz, r, opts = {}) {
  addBuilding(bx - 6, bz - 5, 14, 12, 2 + (r()*2 | 0), facadeMats[r()*facadeMats.length | 0], Object.assign({ doors: ['s', 'e'] }, opts));
  addBuilding(bx + 8, bz + 7, 12, 11, 2 + (r()*2 | 0), facadeMats[r()*facadeMats.length | 0], { doors: ['w', 'n'], balconies: !!opts.balconies });
  addTree(bx - 11, bz + 11); addTree(bx + 12, bz - 11); addBox(bx + 6, bz - 8, 3, 1.1, 1, matHedge);
}
function blockCourtyard(bx, bz, r) {
  const m = facadeMats[r()*facadeMats.length | 0];
  addBuilding(bx, bz - 11, 30, 9, 3, m, { doors: ['s', 'n'], balconies: true });
  addBuilding(bx, bz + 11, 30, 9, 3, m, { doors: ['n', 's'] });
  addBuilding(bx - 11, bz, 8, 13, 2, m, { doors: ['e'], props: false });
  addBuilding(bx + 11, bz, 8, 13, 2, m, { doors: ['w'], props: false });
  addTree(bx, bz, 1.2); addBox(bx - 3, bz + 3, 2, .5, .6, matWood); addBox(bx + 3, bz - 3, 2, .5, .6, matWood);
}
function blockTerracePark(bx, bz) {
  for (let k = 1; k <= 6; k++) { const s = 30 - k * 4; addBox(bx, bz, s, k * STEP, s, k % 2 ? matHedge : matSidewalk); }
  for (const [dx, dz] of [[-1.5, -1.5], [1.5, -1.5], [-1.5, 1.5], [1.5, 1.5]]) addBlock(bx + dx, 6 * STEP, bz + dz, .2, 2.6, .2, matPole);
  addBlock(bx, 6 * STEP + 2.6, bz, 4, .2, 4, matRoof);
  for (const [dx, dz] of [[-14, -14], [14, -14], [-14, 14], [14, 14], [0, -14.5], [-14.5, 0]]) addTree(bx + dx, bz + dz, 1.1);
}
function blockMarket(bx, bz, r) {
  for (let i = -1; i <= 1; i++) for (let j = -1; j <= 1; j++) if (i || j) addStall(bx + i * 8, bz + j * 7, awningMats[r()*awningMats.length | 0]);
  addBox(bx, bz, 4, 3.2, 4, matWood); addVisual(bx, 3.4, bz, 5, .2, 5, awningMats[0]);
  for (const [dx, dz] of [[-13, -13], [13, 13]]) addTree(bx + dx, bz + dz);
}
function blockPlaza(bx, bz) {
  addBox(bx, bz, 9, .7, 9, matJersey); addVisual(bx, .72, bz, 8, .04, 8, matWater, false);
  addBox(bx, bz, 1.2, 2.4, 1.2, matJersey); addBlock(bx, 2.4, bz, .8, 1.8, .8, matStatue);
  for (const [dx, dz] of [[-10, 0], [10, 0], [0, -10], [0, 10]]) addBox(bx + dx, bz + dz, dx ? .6 : 3, .5, dx ? 3 : .6, matWood);
  for (const [dx, dz] of [[-12, -12], [12, -12], [-12, 12], [12, 12]]) addTree(bx + dx, bz + dz, 1.2);
}
function blockPark(bx, bz, r) {
  for (let i = 0; i < 8; i++) addTree(bx + (r()*2 - 1) * 12, bz + (r()*2 - 1) * 12, .9 + r()*.5);
  addBox(bx, bz, 6, .6, 6, matJersey); addVisual(bx, .62, bz, 5, .04, 5, matWater, false);
  for (const [x, z, w, d] of [[-8, 0, 1, 6], [8, 0, 1, 6], [0, -9, 6, 1], [0, 9, 6, 1]]) addBox(bx + x, bz + z, w, .8, d, matHedge);
}
function blockHall(bx, bz, r) {
  addHall(bx, bz - 2, 22, 16, 6, facadeMats[r()*facadeMats.length | 0], ['s', 'e']);
  for (let i = 0; i < 4; i++) addBox(bx - 6 + i*4, bz - 2 + (r() - .5) * 6, 2, 1.2 + r(), 2, matCrate);
  addContainer(bx + 10, bz + 12, true, 1, r()*4 | 0);
}
function blockContainers(bx, bz, r) {
  for (let i = 0; i < 7; i++) { const ax = r() < .5; addContainer(bx + (r()*2 - 1) * 10, bz + (r()*2 - 1) * 10, ax, 1 + (r() < .35 ? 1 : 0), r()*4 | 0); }
}
function blockIndustrial(bx, bz, r) {
  addHall(bx - 4, bz - 5, 18, 14, 7, facadeMats[3], ['s']);
  addFuelTank(bx + 10, bz + 8, 2.6, 9); addFuelTank(bx + 10, bz - 4, 2.6, 9);
  for (let y = 2; y < 9; y += 3) addStrip(bx + 10, y, bz + 2, .5, .5, 10, matPole);
  addContainer(bx - 6, bz + 11, true, 2, r()*4 | 0);
}
function blockLot(bx, bz, r) {
  for (let i = 0; i < 6; i++) addCar(bx - 9 + (i % 3) * 9, bz - 9 + (i / 3 | 0) * 9, false, carMats[r()*carMats.length | 0]);
  addBuilding(bx + 9, bz + 9, 10, 9, 1, facadeMats[r()*facadeMats.length | 0], { doors: ['w'] });
}
function blockTower(bx, bz, h, r, gaps = []) {
  addTower(bx, bz, 18, 18, h, towerMats[r()*towerMats.length | 0], gaps);
  for (const [x, z] of [[-13, 13], [13, -13]]) addTree(bx + x, bz + z);
  jersey(bx, bz + 14, 4, true);
}
function blockSkyscraper(bx, bz, r) {
  addTower(bx, bz, 20, 20, 58, towerMats[3]);
  addBuilding(bx, bz + 13, 26, 6, 1, facadeMats[5], { doors: ['s'], props: false, roofProps: false });
  for (const [x, z, w, d] of [[0, -4, 6, .4], [0, 4, 6, .4], [-3, 0, .4, 8.4], [3, 0, .4, 8.4]]) addStrip(bx + x, 59.2, bz + z, w, .04, d, matStripW);
  addBlock(bx + 7, 59.1, bz - 7, .5, 9, .5, matSteel); addStrip(bx + 7, 68.2, bz - 7, .5, .5, .5, matRed);
}

function buildCiudad() {
  cityRand = rng(20261016); cityTrees = []; OUTPOSTS = []; INTERACT = []; SUPPLY = [];
  const r = cityRand;
  const sky = new THREE.Mesh(new THREE.SphereGeometry(330, 32, 16), new THREE.MeshBasicMaterial({ map: citySkyTex, side: THREE.BackSide, fog: false })); track(sky);
  const sunDisc = new THREE.Mesh(new THREE.SphereGeometry(10, 16, 12), new THREE.MeshBasicMaterial({ color: 0xfff1d0, fog: false }));
  sunDisc.position.copy(sun.position).normalize().multiplyScalar(320); track(sunDisc);
  // perímetro y horizonte de edificios más allá del muro
  for (const [x, z, w, d] of [[0, -A, A*2+2, 2], [0, A, A*2+2, 2], [-A, 0, 2, A*2+2], [A, 0, 2, A*2+2]]) addBox(x, z, w, 12, d, matConcrete);
  for (let i = 0; i < 40; i++) {
    const t = (r() * 2 - 1) * A, side = i % 4, o = A + 14 + r() * 30, w = 14 + r() * 16, h = 18 + r() * 50;
    const [x, z] = side === 0 ? [t, -o] : side === 1 ? [t, o] : side === 2 ? [-o, t] : [o, t];
    addVisual(x, h / 2, z, w, h, w, towerMats[i % 4], false);
  }
  // veredas, líneas de calle, faroles y semáforos
  for (const bx of BC) for (const bz of BC) addBox(bx, bz, BLOCK, .16, BLOCK, matSidewalk);
  const streets = [-110, -66, -22, 22, 66, 110], avenue = 156;
  for (const s of [...streets, -avenue, avenue]) {
    const mat = Math.abs(s) > 150 ? matLineY : matLine;
    for (let t = -A + 4; t < A - 4; t += 7) { addStrip(s, .015, t, .18, .02, 3, mat); addStrip(t, .015, s, 3, .02, .18, mat); }
  }
  for (let i = 0; i < 70; i++) {
    const s = streets[r()*streets.length | 0], t = (r()*2 - 1) * (A - 14), ax = r() < .5;
    const x = ax ? t : s + (r() < .5 ? -4.2 : 4.2), z = ax ? s + (r() < .5 ? -4.2 : 4.2) : t;
    if (Math.abs(z + 22) < 7 && ax) continue;
    addCar(x, z, ax, carMats[r()*carMats.length | 0]);
  }
  // autopista elevada sobre la calle z = -22, con autos abandonados arriba
  addHighway(-22, -118, 118, 7.14);
  for (let i = 0; i < 9; i++) addCar(-100 + i * 24 + r() * 6, -22 + (r() < .5 ? -2.6 : 2.6), true, carMats[r()*carMats.length | 0], 7.14);
  buildOutposts();
  const used = new Set(OUTPOSTS.map(o => o.cx + ',' + o.cz));
  for (const bx of BC) for (const bz of BC) if (!used.has(bx + ',' + bz)) for (const [lx, lz] of [[-15, -15], [15, 15], [15, -15], [-15, 15]]) addLamp(bx + lx, bz + lz);
  // refugio
  {
    const cx = -132, cz = 132; used.add(cx + ',' + cz);
    addBuilding(cx, cz, 14, 11, 2, facadeMats[0], { doors: ['n', 'e'] });
    SAFE_POS = new V3(cx - 3, 1, cz + 2);
    addBlock(cx - 3, .16, cz + 2.6, 1.6, .9, .9, matContainers[1]); addStrip(cx - 3, 1.1, cz + 2.6, 1.2, .06, .7, new THREE.MeshBasicMaterial({ color: 0x5dffb0 }));
    for (const [x, z] of [[-12, -12], [12, 12], [-12, 12]]) addTree(cx + x, cz + z, 1.1);
  }
  // distrito por zonas: residencial al oeste, centro de torres con pasarelas, industrial al este
  const plan = {
    '-132,-132': 'aptsFE', '-88,-132': 'park', '0,-132': 'hall', '44,-132': 'apts', '132,-132': 'containers',
    '-132,-88': 'court', '-88,-88': 'sky', '-44,-88': 'aptsB', '0,-88': 'tower40', '44,-88': 'market', '88,-88': 'aptsFE', '132,-88': 'industrial',
    '-88,-44': 'apts', '-44,-44': 'tower30', '0,-44': 'twinA1', '44,-44': 'twinA2', '88,-44': 'court', '132,-44': 'hall',
    '-132,0': 'aptsFE', '-88,0': 'terraces', '-44,0': 'twinB2', '44,0': 'plaza', '88,0': 'market', '132,0': 'aptsB',
    '-132,44': 'hall', '-88,44': 'aptsB', '-44,44': 'twinB1', '0,44': 'tower24', '44,44': 'court', '88,44': 'lot',
    '-132,88': 'aptsFE', '-88,88': 'park', '-44,88': 'court', '0,88': 'apts', '44,88': 'tower2', '88,88': 'hall', '132,88': 'containers',
    '-88,132': 'lot', '-44,132': 'apts', '44,132': 'hall', '88,132': 'industrial'
  };
  for (const bx of BC) for (const bz of BC) {
    const key = bx + ',' + bz; if (used.has(key)) continue;
    switch (plan[key] || 'apts') {
      case 'apts': blockApts(bx, bz, r); break;
      case 'aptsB': blockApts(bx, bz, r, { balconies: true }); break;
      case 'aptsFE': blockApts(bx, bz, r, { fireEscape: true, balconies: true }); break;
      case 'court': blockCourtyard(bx, bz, r); break;
      case 'park': blockPark(bx, bz, r); break;
      case 'terraces': blockTerracePark(bx, bz); break;
      case 'market': blockMarket(bx, bz, r); break;
      case 'plaza': blockPlaza(bx, bz); break;
      case 'hall': blockHall(bx, bz, r); break;
      case 'containers': blockContainers(bx, bz, r); break;
      case 'industrial': blockIndustrial(bx, bz, r); break;
      case 'lot': blockLot(bx, bz, r); break;
      case 'sky': blockSkyscraper(bx, bz, r); break;
      case 'tower40': blockTower(bx, bz, 40, r); break;
      case 'tower30': blockTower(bx, bz, 30, r); break;
      case 'tower24': blockTower(bx, bz, 24, r); addTower(bx + 12.5, bz + 12.5, 5, 5, 12, towerMats[1]); break;
      case 'tower2': addTower(bx - 4, bz - 4, 14, 14, 26, towerMats[r()*4 | 0]); addTower(bx + 9, bz + 9, 8, 8, 14, towerMats[r()*4 | 0]); break;
      case 'twinA1': blockTower(bx, bz, 34, r, ['e']); break;
      case 'twinA2': blockTower(bx, bz, 34, r, ['w']); break;
      case 'twinB1': blockTower(bx, bz, 26, r, ['n']); break;
      case 'twinB2': blockTower(bx, bz, 26, r, ['s']); break;
    }
  }
  // pasarelas entre torres gemelas, por encima de las calles
  addSkybridge(9, -44, 35, -44, 34);
  addSkybridge(-44, 9, -44, 35, 26);
  // árboles: una sola malla instanciada para todo el follaje
  const im = new THREE.InstancedMesh(foliageGeo, matFoliage, cityTrees.length);
  const m4 = new THREE.Matrix4();
  cityTrees.forEach(([x, y, z, s], i) => { m4.makeScale(s, s * .9, s).setPosition(x, y, z); im.setMatrixAt(i, m4); });
  im.castShadow = im.receiveShadow = true; track(im);
}
MAPS.ciudad = { name: 'Distrito Sur', size: '330 m', desc: 'Siete puestos en un barrio enorme: autopista elevada, torres con pasarelas, obra con grúa, patio de trenes y edificios para recorrer piso por piso.', A: 164, start: [-132, 154],
  bg: 0xa9c6dc, fog: [120, 330], hemi: [0xe4f0ff, 0x8a7a66, .95], sun: [0xffe6c4, 1.55], floor: 0xffffff, floorTex: asphaltTex, build: buildCiudad, city: true };
