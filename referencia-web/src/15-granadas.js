/* ---------- granadas ---------- */
const nadeGeo = new THREE.SphereGeometry(.13, 10, 8);
const nadeMat = new THREE.MeshStandardMaterial({ color: 0x1c2125, metalness: .6, roughness: .4 });
const nadeLightMat = new THREE.MeshBasicMaterial({ color: 0xffb02e });
const nadeLightGeo = new THREE.TorusGeometry(.14, .025, 6, 16);
function throwNade() {
  if (P.nades <= 0 || P.nadeCd > 0 || P.swapT > 0) return;
  P.nades--; P.nadeCd = .6; sfx.toss();
  const m = new THREE.Mesh(nadeGeo, nadeMat); m.add(new THREE.Mesh(nadeLightGeo, nadeLightMat));
  camera.getWorldDirection(_d);
  m.position.copy(camera.position).addScaledVector(_d, .6);
  scene.add(m);
  const v = _d.clone().multiplyScalar(17).add(_c.set(0, 3.5, 0)).addScaledVector(P.vel, .6);
  nades.push({ m, v, fuse: 1.7, beep: .4 });
}
function updateNades(dt) {
  for (let i = nades.length - 1; i >= 0; i--) {
    const n = nades[i], p = n.m.position;
    n.v.y -= (n.g || 22) * dt;
    const px = p.x, py = p.y, pz = p.z;
    p.addScaledVector(n.v, dt);
    n.m.rotation.x += n.v.z * dt * 3; n.m.rotation.z -= n.v.x * dt * 3;
    if (p.y < .13) { p.y = .13; n.v.y = Math.abs(n.v.y) * .4; n.v.x *= .7; n.v.z *= .7; }
    const b = boxAt(p);
    if (b) {
      if (py >= b.max.y - .05) { p.y = b.max.y + .13; n.v.y = Math.abs(n.v.y) * .4; n.v.x *= .7; n.v.z *= .7; }
      else {
        const wasInX = px > b.min.x && px < b.max.x;
        p.x = px; p.z = pz;
        if (wasInX) n.v.z *= -.5; else n.v.x *= -.5;
      }
    }
    p.x = Math.max(-LIM, Math.min(LIM, p.x)); p.z = Math.max(-LIM, Math.min(LIM, p.z));
    if (n.impact) {
      if (Math.random() < .7) burst(p, 0x5a5f62, 1, .6, .45);
      let hitE = p.y <= .14 || !!b;
      if (!hitE) for (const e of enemies) if (e.g.position.distanceTo(p) < e.radius + .35) { hitE = true; break; }
      if (hitE || n.fuse <= 0) { scene.remove(n.m); nades.splice(i, 1); explode(p, 4.5, n.dmg, 16, true); if (state !== 'playing') return; continue; }
    }
    n.fuse -= dt; n.beep -= dt;
    if (!n.impact && n.beep <= 0) { sfx.beep(); n.beep = n.fuse < .6 ? .1 : .3; }
    n.m.children[0].visible = n.beep > .05;
    if (n.fuse <= 0) { scene.remove(n.m); nades.splice(i, 1); explode(p, 6.5, 9, 30, true); if (state !== 'playing') return; }
  }
}

