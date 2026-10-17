/* ---------- colisiones ---------- */
const _cand = [];
function collideXZ(p, r, feet, height) {
  gridQuery(p.x - r, p.z - r, p.x + r, p.z + r, _cand);
  for (let i = 0; i < _cand.length; i++) {
    const b = _cand[i];
    if (feet >= b.max.y - .35 || feet + height <= b.min.y) continue;
    const cx = p.x < b.min.x ? b.min.x : p.x > b.max.x ? b.max.x : p.x;
    const cz = p.z < b.min.z ? b.min.z : p.z > b.max.z ? b.max.z : p.z;
    const dx = p.x - cx, dz = p.z - cz, d2 = dx*dx + dz*dz;
    if (d2 >= r*r) continue;
    if (d2 > 1e-8) { const d = Math.sqrt(d2), k = (r - d) / d; p.x += dx*k; p.z += dz*k; }
    else {
      const ox0 = b.min.x - r - p.x, ox1 = b.max.x + r - p.x, oz0 = b.min.z - r - p.z, oz1 = b.max.z + r - p.z;
      const mx = Math.abs(ox0) < Math.abs(ox1) ? ox0 : ox1, mz = Math.abs(oz0) < Math.abs(oz1) ? oz0 : oz1;
      if (Math.abs(mx) < Math.abs(mz)) p.x += mx; else p.z += mz;
    }
  }
}
function groundAt(x, z, feet) {
  let g = 0; const e = R*.5;
  gridQuery(x - e, z - e, x + e, z + e, _cand);
  for (let i = 0; i < _cand.length; i++) { const b = _cand[i];
    if (x > b.min.x - e && x < b.max.x + e && z > b.min.z - e && z < b.max.z + e && feet >= b.max.y - .36 && b.max.y > g) g = b.max.y; }
  return g;
}
// techo: la primera losa que la cabeza cruzaría al subir
function ceilingAt(x, z, headOld, headNew) {
  let c = null; const e = R*.7;
  gridQuery(x - e, z - e, x + e, z + e, _cand);
  for (let i = 0; i < _cand.length; i++) { const b = _cand[i];
    if (b.min.y > .05 && b.min.y >= headOld - .02 && b.min.y < headNew && x > b.min.x - e && x < b.max.x + e && z > b.min.z - e && z < b.max.z + e && (c === null || b.min.y < c)) c = b.min.y; }
  return c;
}
function boxAt(p) {
  gridQuery(p.x, p.z, p.x, p.z, _cand);
  for (let i = 0; i < _cand.length; i++) { const b = _cand[i]; if (p.x > b.min.x && p.x < b.max.x && p.z > b.min.z && p.z < b.max.z && p.y < b.max.y && p.y > b.min.y) return b; }
  return null;
}
// rayo contra el mundo (cajas por grilla + piso). Devuelve la distancia al primer impacto o `far`.
function rayAABB(o, d, b, tmax) {
  let t0 = 0, t1 = tmax;
  for (const a of ['x', 'y', 'z']) {
    if (Math.abs(d[a]) < 1e-9) { if (o[a] < b.min[a] || o[a] > b.max[a]) return -1; continue; }
    const inv = 1 / d[a]; let tn = (b.min[a] - o[a]) * inv, tf = (b.max[a] - o[a]) * inv;
    if (tn > tf) { const q = tn; tn = tf; tf = q; }
    if (tn > t0) t0 = tn; if (tf < t1) t1 = tf;
    if (t0 > t1) return -1;
  }
  return t0;
}
const _rc = [];
let rayNormalAxis = 'y';
function rayWorld(o, d, far) {
  let best = far; rayNormalAxis = 'y';
  if (d.y < -1e-6) { const t = -o.y / d.y; if (t >= 0 && t < best) best = t; }
  gstamp++;
  const step = CELL * .35;
  let lastKey = -1;
  for (let t = 0; t <= best + step; t += step) {
    const tt = Math.min(t, best), x = o.x + d.x*tt, z = o.z + d.z*tt;
    for (let ox = -1; ox <= 1; ox += 2) for (let oz = -1; oz <= 1; oz += 2) {
      const k = gkey(Math.floor((x + ox*.3) / CELL), Math.floor((z + oz*.3) / CELL));
      const c = grid.get(k); if (!c) continue;
      for (let i = 0; i < c.length; i++) {
        const b = c[i]; if (b.st === gstamp) continue; b.st = gstamp;
        const h = rayAABB(o, d, b, best);
        if (h >= 0 && h < best) best = h;
      }
    }
  }
  return best;
}
function hasLOS(from, to) {
  _d.copy(to).sub(from); const dist = _d.length(); if (dist < .01) return true; _d.divideScalar(dist);
  return rayWorld(from, _d, dist) >= dist - .05;
}

