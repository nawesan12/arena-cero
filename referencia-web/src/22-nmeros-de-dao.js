/* ---------- números de daño ---------- */
const numsEl = $('nums'), numPool = [], numActive = [], pendingNums = [], _v = new V3();
for (let i = 0; i < 28; i++) { const el = document.createElement('span'); el.className = 'num'; el.hidden = true; numsEl.appendChild(el); numPool.push(el); }
function queueNum(e, v, crit) {
  if (!e.numP) { e.numP = { v: 0, crit: false }; pendingNums.push(e); }
  e.numP.v += v; e.numP.crit = e.numP.crit || crit;
}
function updateNums(dt) {
  for (const e of pendingNums) {
    const n = e.numP; e.numP = null;
    const el = numPool.pop() || (numActive.length ? numActive.shift().el : null); if (!el) continue;
    el.textContent = Math.max(1, Math.round(n.v * 10)); el.className = n.crit ? 'num crit' : 'num'; el.hidden = false;
    numActive.push({ el, x: e.g.position.x + (Math.random()-.5)*.8, y: e.g.position.y + e.radius + .3, z: e.g.position.z, t: 0 });
  }
  pendingNums.length = 0;
  const w = innerWidth, h = innerHeight;
  for (let i = numActive.length - 1; i >= 0; i--) {
    const n = numActive[i]; n.t += dt;
    if (n.t > .8) { n.el.hidden = true; numPool.push(n.el); numActive.splice(i, 1); continue; }
    _v.set(n.x, n.y, n.z).project(camera);
    if (_v.z > 1) { n.el.style.opacity = 0; continue; }
    n.el.style.transform = `translate(${((_v.x*.5 + .5)*w).toFixed(1)}px,${((-_v.y*.5 + .5)*h - n.t*46).toFixed(1)}px) translate(-50%,-50%)`;
    n.el.style.opacity = (1 - n.t / .8).toFixed(2);
  }
}

