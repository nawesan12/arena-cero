/* ---------- gancho (equipo de maniobras) ---------- */
const hookRay = new THREE.Raycaster(); hookRay.far = 55;
const cableGeo = new THREE.BufferGeometry(); cableGeo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(6), 3));
const cable = new THREE.Line(cableGeo, new THREE.LineBasicMaterial({ color: 0xc9d4d0 })); cable.frustumCulled = false; cable.visible = false; scene.add(cable);
const spike = new THREE.Mesh(new THREE.ConeGeometry(.07, .26, 6), new THREE.MeshBasicMaterial({ color: 0xffb02e })); spike.geometry.rotateX(Math.PI/2); spike.visible = false; scene.add(spike);
const _hk = new V3(), _hs = new V3();
let hookScanT = 0;
function ensureTargets() { if (targetsDirty) { hitTargets = [].concat(...enemies.map(e => e.hitMeshes || [])); targetsDirty = false; } }
// gancho: devuelve {pt, e} contra drones o el mundo
function hookCast(far) {
  ensureTargets();
  hookRay.setFromCamera(_ndc.set(0, 0), camera);
  const o = hookRay.ray.origin, d = hookRay.ray.direction, wt = rayWorld(o, d, far);
  const eh = hookRay.intersectObjects(hitTargets, false)[0];
  if (eh && eh.distance < wt) return { pt: eh.point.clone(), e: eh.object.userData.enemy };
  if (wt < far) return { pt: o.clone().addScaledVector(d, wt), e: null };
  return null;
}
function fireHook() {
  if (state !== 'playing') return;
  if (P.hook) { releaseHook(false); return; }
  if (P.hookCd > 0) return;
  const h = hookCast(55);
  P.hookCd = .3;
  if (!h) { sfx.hookMiss(); return; }
  const e = h.e && !h.e.soldier && !h.e.turret ? h.e : null;
  P.hook = { pt: h.pt, e, off: e ? h.pt.clone().sub(e.g.position) : null, t: 0 };
  P.jumps = Math.min(P.jumps, 1);
  spike.visible = cable.visible = true; sfx.hook();
}
function releaseHook(boost) {
  if (!P.hook) return;
  P.hook = null; spike.visible = cable.visible = false; P.hookCd = .2;
  if (boost) { P.vel.y = Math.max(P.vel.y, 0) + 7.5; sfx.djump(); _a.set(P.pos.x, P.pos.y - 1, P.pos.z); burst(_a, 0x5fd8ff, 14, 4, .35); }
}
function updateHook(dt) {
  P.hookCd -= dt;
  const hk = P.hook; if (!hk) return;
  hk.t += dt;
  if (hk.e) { if (hk.e.hp <= 0 || !enemies.includes(hk.e)) { releaseHook(false); return; } hk.pt.copy(hk.e.g.position).add(hk.off); }
  if (hk.t < .08) return; // el cable todavía viaja
  const dx = hk.pt.x - P.pos.x, dy = hk.pt.y - (P.pos.y - .5), dz = hk.pt.z - P.pos.z, d = Math.hypot(dx, dy, dz) || .001;
  if (d < 1.8 || hk.t > 3.5) {
    if (hk.e && d < 1.8) { // llegar a un dron lo golpea
      _d.set(dx, dy, dz).normalize();
      const killed = damageEnemy(hk.e, 3 * P.dmgMul, _d, false); hitMark(killed ? 'kill' : ''); if (!killed) sfx.hit();
      P.vel.multiplyScalar(-.25); P.vel.y = 6;
    }
    releaseHook(false); return;
  }
  const pull = 58;
  P.vel.x += dx/d*pull*dt; P.vel.y += dy/d*pull*dt + 13*dt; P.vel.z += dz/d*pull*dt;
  const vs = P.vel.length(); if (vs > 34) P.vel.multiplyScalar(34 / vs);
  P.onGround = false; P.slide = 0;
}
function drawHook(dt) {
  const hk = P.hook;
  if (!hk) {
    if ((hookScanT -= dt) <= 0 && state === 'playing') {
      hookScanT = .08;
      const can = !!hookCast(55);
      if (last.hk !== can) { last.hk = can; hud.cross.classList.toggle('hk', can); }
    }
    if (last.hooked) { last.hooked = false; hud.cross.classList.remove('hooked'); }
    return;
  }
  if (!last.hooked) { last.hooked = true; hud.cross.classList.add('hooked'); }
  _hs.set(-.22, -.28, -.35).applyMatrix4(camera.matrixWorld);
  _hk.copy(_hs).lerp(hk.pt, Math.min(1, hk.t / .08));
  const a = cableGeo.attributes.position; a.setXYZ(0, _hs.x, _hs.y, _hs.z); a.setXYZ(1, _hk.x, _hk.y, _hk.z); a.needsUpdate = true;
  spike.position.copy(_hk); spike.lookAt(_hk.x*2 - _hs.x, _hk.y*2 - _hs.y, _hk.z*2 - _hs.z);
}

