/* ---------- radar ---------- */
const radarEl = $('radar'), rctx = radarEl.getContext('2d'), RR = 132, RANGE = 42;
function drawRadar() {
  const range = isCity() ? 70 : RANGE;
  const S = RR, c = S/2, k = (c - 8) / range, sy = Math.sin(P.yaw), cy = Math.cos(P.yaw);
  rctx.clearRect(0, 0, S*2, S*2); rctx.save(); rctx.scale(2, 2);
  rctx.fillStyle = 'rgba(7,11,13,.62)'; rctx.beginPath(); rctx.arc(c, c, c - 2, 0, Math.PI*2); rctx.fill();
  rctx.strokeStyle = 'rgba(230,237,232,.14)'; rctx.lineWidth = 1;
  for (const rr of [.33, .66, 1]) { rctx.beginPath(); rctx.arc(c, c, (c - 8)*rr, 0, Math.PI*2); rctx.stroke(); }
  rctx.beginPath(); rctx.arc(c, c, c - 2, 0, Math.PI*2); rctx.clip();
  const proj = (x, z) => { const dx = x - P.pos.x, dz = z - P.pos.z; return [c + (dx*cy - dz*sy)*k, c + (dx*sy + dz*cy)*k]; };
  // perímetro y estructuras grandes
  rctx.strokeStyle = 'rgba(255,176,46,.45)'; rctx.beginPath();
  [[-A,-A],[A,-A],[A,A],[-A,A]].forEach(([x, z], i) => { const [px, py] = proj(x, z); i ? rctx.lineTo(px, py) : rctx.moveTo(px, py); }); rctx.closePath(); rctx.stroke();
  rctx.fillStyle = 'rgba(230,237,232,.16)';
  for (const b of boxes) {
    if (Math.abs(b.min.x - P.pos.x) > range + 20 && Math.abs(b.max.x - P.pos.x) > range + 20) continue;
    if (Math.abs(b.min.z - P.pos.z) > range + 20 && Math.abs(b.max.z - P.pos.z) > range + 20) continue;
    if (b.max.y < 2.5 || (b.max.x - b.min.x) > 50 || (b.max.z - b.min.z) > 50) continue;
    const p = [proj(b.min.x, b.min.z), proj(b.max.x, b.min.z), proj(b.max.x, b.max.z), proj(b.min.x, b.max.z)];
    rctx.beginPath(); p.forEach(([x, y], i) => i ? rctx.lineTo(x, y) : rctx.moveTo(x, y)); rctx.fill();
  }
  if (isCity()) for (const s of SUPPLY) { if (!s.active) continue; const [x, y] = proj(s.pos.x, s.pos.z); rctx.fillStyle = '#5dffb0'; rctx.fillRect(x - 3, y - 1, 6, 2); rctx.fillRect(x - 1, y - 3, 2, 6); }
  if (isCity()) for (const op of OUTPOSTS) { if (op.done) continue; const [x, y] = proj(op.cx, op.cz); rctx.strokeStyle = op.alarm > 0 ? '#ff3d5a' : '#ffb02e'; rctx.lineWidth = 1.5; rctx.strokeRect(x - 5, y - 5, 10, 10); }
  for (const o of orbs) { const [x, y] = proj(o.m.position.x, o.m.position.z); rctx.fillStyle = '#5dffb0'; rctx.fillRect(x - 2, y - 2, 4, 4); }
  for (const e of enemies) {
    if (isCity() && !(e.turret || e.surv || e.hunting || e.tagged || cityState.intel || (e.soldier && e.st === 'combat' && e.seeT < 2.5))) continue;
    let [x, y] = proj(e.g.position.x, e.g.position.z);
    const dx = x - c, dy = y - c, d = Math.hypot(dx, dy), max = c - 6;
    const out = d > max; if (out) { x = c + dx/d*max; y = c + dy/d*max; }
    rctx.fillStyle = e.turret ? (cityState.powered ? '#ffb02e' : '#55606a') : e.surv || e.kami ? '#ff6a2e' : e.heavy ? '#ffb02e' : '#ff3d5a';
    rctx.globalAlpha = out ? .5 : 1;
    rctx.beginPath(); rctx.arc(x, y, e.heavy ? 4 : 2.6, 0, Math.PI*2); rctx.fill();
  }
  rctx.globalAlpha = 1;
  rctx.fillStyle = '#e6ede8'; rctx.beginPath(); rctx.moveTo(c, c - 6); rctx.lineTo(c + 4.5, c + 5); rctx.lineTo(c, c + 2.5); rctx.lineTo(c - 4.5, c + 5); rctx.closePath(); rctx.fill();
  rctx.restore();
}

