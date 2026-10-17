/* ---------- disparo del jugador ---------- */
const ray = new THREE.Raycaster(); ray.far = 120;
const _ndc = new THREE.Vector2();
function magOf(w) { return Math.round(WEAPONS[w].mag * P.magMul); }
function playerFire() {
  const W = WEAPONS[P.w];
  if (P.reload > 0 || P.fireCd > 0 || P.swapT > 0 || (semiLock && !W.auto)) return;
  if (P.ammo[P.w] <= 0) { sfx.empty(); P.fireCd = .25; semiLock = true; startReload(); return; }
  const od = P.overdrive > 0;
  P.ammo[P.w]--; P.fireCd = W.rate * (od ? .6 : 1); P.recoil = 1; P.slide = 0;
  if (!W.auto) semiLock = true;
  if (W.launcher) { fireLauncher(W, od); return; }
  const moving = Math.hypot(P.vel.x, P.vel.z) > 1 || !P.onGround;
  const spread = (moving ? W.spread[1] : W.spread[0]) * (1 - P.aimT * (P.w === 1 ? .35 : .85)) * (1 - P.crouchT * .4);
  ensureTargets();
  flash.getWorldPosition(_a).applyMatrix4(camera.matrixWorld);
  const dmgBase = W.dmg * P.dmgMul * (od ? 1.5 : 1);
  let anyHit = false, anyKill = false, anyCrit = false;
  for (let k = 0; k < W.pellets; k++) {
    const ang = Math.random() * Math.PI * 2, rad = Math.sqrt(Math.random()) * spread;
    _ndc.set(Math.cos(ang) * rad, Math.sin(ang) * rad * camera.aspect);
    ray.setFromCamera(_ndc, camera);
    const wt = rayWorld(ray.ray.origin, ray.ray.direction, 120);
    const hits = ray.intersectObjects(hitTargets, false);
    const seen = [];
    let endSet = false;
    for (const h of hits) {
      if (h.distance > wt) break;
      if (h.object.userData.shield) { _b.copy(h.point); endSet = true; burst(h.point, 0x9fe9ff, 6, 4, .25); sfx.ricochet(); break; }
      const e = h.object.userData.enemy;
      if (!e || e.hp <= 0 || seen.includes(e)) continue;
      seen.push(e);
      const crit = !!h.object.userData.crit;
      anyHit = true; if (crit) anyCrit = true;
      burst(h.point, crit ? 0xffb02e : W.pierce ? 0x5fd8ff : 0xfff0e0, crit ? 8 : W.pierce ? 14 : 4, 5, .25);
      if (damageEnemy(e, dmgBase * (crit ? P.critMul : 1), ray.ray.direction, crit)) anyKill = true;
      if (!W.pierce) { _b.copy(h.point); endSet = true; break; }
    }
    if (!endSet && wt < 120) {
      _b.copy(ray.ray.origin).addScaledVector(ray.ray.direction, wt); endSet = true;
      burst(_b, W.pierce ? 0x5fd8ff : 0xffb02e, W.pierce ? 10 : 3, 4, .3);
    }
    if (!endSet) _b.copy(ray.ray.origin).addScaledVector(ray.ray.direction, 80);
    if (W.pierce || k % 3 === 0) tracer(_a, _b, W.color, W.pierce ? .18 : .05);
  }
  if (anyKill) hitMark('kill'); else if (anyHit) { anyCrit ? sfx.crit() : sfx.hit(); hitMark(anyCrit ? 'crit' : ''); }
  flash.visible = true; flash.rotation.z = Math.random()*Math.PI; flashLight.intensity = W.pierce ? 4 : 3; worldFlash.intensity = W.pierce ? 3 : 2.5;
  const kick = (1 - P.aimT * .5) * (1 - P.crouchT * .35);
  P.pitch = Math.min(1.5, P.pitch + W.kick * kick); P.yaw += (Math.random()-.5) * W.kick * .45 * kick;
  if (P.w > 0) P.shake = Math.min(1, P.shake + .15);
  sfx.shot(P.w);
  if (isCity()) makeNoise(camera.position, [42, 55, 70, 50][P.w]);
  if (P.ammo[P.w] === 0) startReload();
}
function fireLauncher(W, od) {
  camera.getWorldDirection(_d);
  const sp = W.spread[Math.hypot(P.vel.x, P.vel.z) > 1 ? 1 : 0] * (1 - P.aimT * .8);
  _d.x += (Math.random()-.5)*sp; _d.y += (Math.random()-.5)*sp; _d.normalize();
  const m = new THREE.Mesh(nadeGeo, nadeMat); m.add(new THREE.Mesh(nadeLightGeo, nadeLightMat)); m.scale.setScalar(.8);
  m.position.copy(camera.position).addScaledVector(_d, .8); scene.add(m);
  nades.push({ m, v: _d.clone().multiplyScalar(34).addScaledVector(P.vel, .3), fuse: 3, beep: 99, impact: true, g: 9, dmg: 7 * P.dmgMul * (od ? 1.5 : 1) });
  flash.visible = true; flash.rotation.z = Math.random()*Math.PI; flashLight.intensity = 3; worldFlash.intensity = 2.5;
  P.pitch = Math.min(1.5, P.pitch + W.kick * (1 - P.aimT * .5)); P.shake = Math.min(1, P.shake + .25);
  models[3][1].drumTarget += Math.PI / 3;
  sfx.launch();
  if (P.ammo[P.w] === 0) startReload();
}
function startReload() {
  const W = WEAPONS[P.w];
  if (P.reload > 0 || P.ammo[P.w] === magOf(P.w) || P.swapT > 0) return;
  P.reload = P.reloadMax = W.reload * P.reloadMul; sfx.reload();
}
function switchWeapon(w, exact = false) {
  const nW = WEAPONS.length; w = ((w % nW) + nW) % nW;
  if (P.unlocked && !P.unlocked[w]) {
    if (exact) { banner('BLOQUEADA', 'Forzá el arsenal para usar esta arma'); return; }
    const dir = ((w - P.w + nW) % nW) <= nW / 2 ? 1 : -1;
    for (let k = 0; k < nW && !P.unlocked[w]; k++) w = ((w + dir) % nW + nW) % nW;
    if (!P.unlocked[w]) return;
  }
  if (w === P.w || state !== 'playing') return;
  P.w = w; P.reload = 0; P.swapT = .32; P.fireCd = Math.max(P.fireCd, .1); semiLock = false;
  sfx.swap();
  setTimeout(() => applyGunModel(P.w), 120);
}
let hitT = 0;
function hitMark(kind) { hud.hit.className = kind; hitT = kind === 'kill' ? .25 : .12; }

