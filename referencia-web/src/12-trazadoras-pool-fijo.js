/* ---------- trazadoras: pool fijo ---------- */
const tracerMats = { 0xffc66a: new THREE.LineBasicMaterial({ color: 0xffc66a, transparent: true, opacity: .85 }), 0x5fd8ff: new THREE.LineBasicMaterial({ color: 0x5fd8ff }) };
const tracers = [];
for (let i = 0; i < 16; i++) {
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(6), 3));
  const l = new THREE.Line(g, tracerMats[0xffc66a]); l.visible = false; l.frustumCulled = false; scene.add(l);
  tracers.push({ l, life: 0 });
}
let tracerIdx = 0;
function tracer(a, b, color, life = .05) {
  const t = tracers[tracerIdx++ % tracers.length], arr = t.l.geometry.attributes.position;
  arr.setXYZ(0, a.x, a.y, a.z); arr.setXYZ(1, b.x, b.y, b.z); arr.needsUpdate = true;
  t.l.material = tracerMats[color]; t.l.visible = true; t.life = life;
}

